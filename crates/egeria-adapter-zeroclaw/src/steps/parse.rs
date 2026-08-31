//! Parsing the step list out of `SOP.md`.
//!
//! The grammar is line-oriented and has no lookahead, no nesting, and no
//! recovery: upstream's parser never fails, it only degrades. Egeria reproduces
//! the accepted set exactly — being bug-compatible is the point, since a
//! workflow must be analyzed as it will actually run — and adds diagnostics
//! where upstream degrades silently.
//!
//! Citations are into `external/zeroclaw/crates/zeroclaw-runtime/src/sop/`,
//! abbreviated `MOD` for `mod.rs`. The authority is that file's `parse_steps`
//! (MOD:504-657) and its helpers (MOD:659-837), which is the only SOP.md parser
//! in the workspace.
//!
//! Three behaviors are worth knowing before reading the code, because each looks
//! like a bug and is not:
//!
//! * **Authored step numbers are discarded.** Steps are renumbered by position,
//!   so `1.`, `5.`, `7.` become 1, 2, 3, and every cross-reference (`next:`,
//!   `depends_on:`, `goto:`) is resolved against the *new* numbers.
//! * **Indentation is stripped before anything else**, so nesting carries no
//!   meaning. A "nested" numbered item starts a new step; a nested bullet is an
//!   ordinary bullet.
//! * **There is no code-fence awareness.** Inside a fence, `- foo:` is still a
//!   bullet and `## Other` still ends the section.

use crate::diagnostic::{Diagnostic, DiagnosticKind, Location};
use crate::model::{
    PlannedToolCall, SopExecutionMode, SopStep, SopStepKind, StepFailure, StepSchema,
    StepToolScope, SwitchRule,
};

/// Parse the `## Steps` section of a `SOP.md` document.
///
/// Never fails. Steps are returned renumbered from 1 in file order, alongside
/// diagnostics for the places upstream would have degraded without saying so.
pub fn parse_steps(markdown: &str) -> (Vec<SopStep>, Vec<Diagnostic>) {
    let mut parser = Parser::default();
    parser.run(markdown);
    parser.finish()
}

#[derive(Default)]
struct Parser {
    steps: Vec<SopStep>,
    diagnostics: Vec<Diagnostic>,
    current: Option<Pending>,
    in_steps_section: bool,
    /// Step numbers as the author wrote them, in file order.
    ///
    /// Upstream throws these away. Egeria keeps them only to notice when they
    /// disagree with the positional numbering that actually governs, which is a
    /// warning upstream's own `syntax.md` advertises but which can never fire
    /// there (MOD:993-1001 compares a value against the formula that produced
    /// it).
    written_numbers: Vec<u32>,
}

/// A step under construction.
struct Pending {
    number: u32,
    step: SopStep,
}

impl Parser {
    fn run(&mut self, markdown: &str) {
        for (index, line) in markdown.lines().enumerate() {
            let line_no = index + 1;
            let trimmed = line.trim();

            if trimmed.starts_with("## ") {
                if trimmed.eq_ignore_ascii_case("## steps") {
                    // Deliberately does not flush: a second `## Steps` while a
                    // step is open leaves it open, and the step keeps absorbing
                    // content (MOD:514-518 continues before the flush at :522).
                    self.in_steps_section = true;
                } else if self.in_steps_section {
                    self.flush();
                    self.in_steps_section = false;
                }
                continue;
            }

            if !self.in_steps_section {
                continue;
            }

            if let Some((written, rest)) = parse_numbered_item(trimmed) {
                self.flush();
                self.written_numbers.push(written);
                let number = u32::try_from(self.steps.len())
                    .unwrap_or(u32::MAX)
                    .saturating_add(1);
                let mut step = SopStep {
                    number,
                    ..Default::default()
                };
                match extract_bold_title(rest) {
                    Some((title, body)) => {
                        step.title = title;
                        step.body = body;
                    }
                    None => step.title = rest.to_string(),
                }
                self.current = Some(Pending { number, step });
                continue;
            }

            if self.current.is_some() && trimmed.starts_with("- ") {
                let bullet = trimmed.trim_start_matches("- ").trim();
                if self.apply_bullet(bullet, line_no) {
                    continue;
                }
                // Unrecognized: appended to the body *with* its marker, which
                // is what upstream does (it pushes the untrimmed line, not the
                // stripped bullet).
                self.note(
                    DiagnosticKind::UnrecognizedBullet {
                        text: bullet.to_string(),
                    },
                    line_no,
                );
                self.push_body(trimmed);
                continue;
            }

            if self.current.is_some() && !trimmed.is_empty() {
                self.push_body(trimmed);
            }
        }
        self.flush();
    }

    fn finish(mut self) -> (Vec<SopStep>, Vec<Diagnostic>) {
        let irregular = self
            .written_numbers
            .iter()
            .enumerate()
            .any(|(i, n)| *n as usize != i + 1);
        if irregular {
            let written = std::mem::take(&mut self.written_numbers);
            self.diagnostics.push(Diagnostic::new(
                DiagnosticKind::NumberingIrregularity { written },
                Location::Document { line: None },
            ));
        }
        (self.steps, self.diagnostics)
    }

    fn flush(&mut self) {
        if let Some(pending) = self.current.take() {
            let mut step = pending.step;
            // The body is trimmed as a whole on flush; the title is not
            // (MOD:695-696).
            step.body = step.body.trim().to_string();
            self.steps.push(step);
        }
    }

    fn push_body(&mut self, text: &str) {
        if let Some(pending) = self.current.as_mut() {
            if !pending.step.body.is_empty() {
                pending.step.body.push('\n');
            }
            pending.step.body.push_str(text);
        }
    }

    fn note(&mut self, kind: DiagnosticKind, line: usize) {
        let number = self.current.as_ref().map_or(0, |p| p.number);
        self.diagnostics.push(Diagnostic::new(
            kind,
            Location::Step {
                number,
                line: Some(line),
            },
        ));
    }

    fn malformed(&mut self, key: &str, reason: impl Into<String>, line: usize) {
        self.note(
            DiagnosticKind::MalformedBullet {
                key: key.to_string(),
                reason: reason.into(),
            },
            line,
        );
    }

    /// Apply a recognized bullet. Returns false if the key is unknown.
    ///
    /// Keys are matched case-sensitively against a literal that includes the
    /// colon, so `- Tools: x` and `- tools : x` are both unrecognized. No key is
    /// a prefix of another once the colon is included, so match order carries no
    /// meaning (MOD:555-633).
    fn apply_bullet(&mut self, bullet: &str, line: usize) -> bool {
        let Some((key, value)) = split_bullet(bullet) else {
            return false;
        };
        let val = value.trim();

        // Borrowed separately from the diagnostic helpers, which need &mut self.
        macro_rules! step {
            () => {
                match self.current.as_mut() {
                    Some(p) => &mut p.step,
                    None => return false,
                }
            };
        }

        match key {
            "tools" => step!().suggested_tools = parse_csv_list(val),
            "allow-tools" | "allow_tools" => {
                let list = parse_csv_list(val);
                // `Some([])` is meaningfully different from absent: an empty
                // allow-list permits nothing, while no allow-list permits
                // everything not denied.
                ensure_scope(step!()).allow = Some(list);
            }
            "deny-tools" | "deny_tools" => {
                let list = parse_csv_list(val);
                ensure_scope(step!()).deny = list;
            }
            "requires_confirmation" => {
                step!().requires_confirmation = val.eq_ignore_ascii_case("true");
            }
            "kind" => {
                let kind = parse_step_kind(val);
                if kind == SopStepKind::Execute && !is_execute_spelling(val) {
                    // Upstream maps every unrecognized value to Execute, so a
                    // typo silently produces an ordinary step where the author
                    // wanted a gate.
                    self.malformed(
                        "kind",
                        format!("unknown kind `{val}`; treated as execute"),
                        line,
                    );
                }
                step!().kind = kind;
            }
            "capability" => step!().capability = Some(val.to_string()),
            "with" => step!().capability_input = Some(parse_value_fragment(val)),
            "input" => ensure_schema(step!()).input = Some(parse_value_fragment(val)),
            "output" => ensure_schema(step!()).output = Some(parse_value_fragment(val)),
            "when" => {
                // An empty value is a no-op rather than a clear, unlike `agent`
                // and `policy` below. The asymmetry is upstream's (MOD:585).
                if !val.is_empty() {
                    step!().routing.when = Some(val.to_string());
                }
            }
            "next" => {
                let parsed = val.parse::<u32>().ok();
                if parsed.is_none() && !val.is_empty() {
                    self.malformed("next", format!("`{val}` is not a step number"), line);
                }
                step!().routing.next = parsed;
            }
            "terminal" => step!().routing.terminal = val.eq_ignore_ascii_case("true"),
            "depends_on" | "depends-on" => {
                let (list, dropped) = parse_u32_list(val);
                if !dropped.is_empty() {
                    self.malformed(
                        "depends_on",
                        format!("dropped non-numeric {}", quoted(&dropped)),
                        line,
                    );
                }
                step!().routing.depends_on = list;
            }
            "switch" => {
                let (rules, problems) = parse_switch_rules(val);
                for problem in problems {
                    self.malformed("switch", problem, line);
                }
                step!().routing.switch = rules;
            }
            "on_failure" | "on-failure" => {
                let failure = parse_step_failure(val);
                if failure.is_fail() && !val.eq_ignore_ascii_case("fail") {
                    self.malformed(
                        "on_failure",
                        format!("unrecognized policy `{val}`; treated as fail"),
                        line,
                    );
                }
                step!().on_failure = failure;
            }
            "mode" => {
                if !is_known_mode(val) {
                    self.malformed(
                        "mode",
                        format!("unknown mode `{val}`; treated as supervised"),
                        line,
                    );
                }
                step!().mode = Some(parse_execution_mode(val));
            }
            "agent" => step!().agent = (!val.is_empty()).then(|| val.to_string()),
            "call" => match serde_json::from_str::<PlannedToolCall>(val) {
                Ok(call) => step!().calls.push(call),
                Err(err) => {
                    // Upstream drops the bullet entirely with no fallback to
                    // body text, so the authored call vanishes without trace.
                    self.malformed("call", format!("{err}; the call is dropped"), line);
                }
            },
            "prompt" => {
                if !val.is_empty() {
                    step!().gate_prompt = Some(val.to_string());
                }
            }
            "policy" => step!().policy = (!val.is_empty()).then(|| val.to_string()),
            "edit" => step!().edit = (!val.is_empty()).then(|| val.to_string()),
            _ => return false,
        }
        true
    }
}

/// Split `key: value` at the first colon, returning the key without it.
///
/// The colon must be attached to the key: upstream matches literals like
/// `"tools:"`, so `tools : x` does not match.
fn split_bullet(bullet: &str) -> Option<(&str, &str)> {
    let colon = bullet.find(':')?;
    let key = &bullet[..colon];
    if key.is_empty() || key.contains(char::is_whitespace) {
        return None;
    }
    Some((key, &bullet[colon + 1..]))
}

/// Recognize `N. rest`, returning the written number and the remainder.
///
/// Mirrors `parse_numbered_item` (MOD:808-817). The literal `". "` is required —
/// `1)` and a bare `1.` are not step items — and the digits must be ASCII.
fn parse_numbered_item(line: &str) -> Option<(u32, &str)> {
    let dot = line.find(". ")?;
    let prefix = &line[..dot];
    if prefix.is_empty() || !prefix.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    // Saturating: upstream discards the value entirely, so an absurd number
    // must not change what parses.
    let written = prefix.parse::<u32>().unwrap_or(u32::MAX);
    Some((written, line[dot + 2..].trim()))
}

/// Split a leading `**title**` off a step's first line.
///
/// Mirrors `extract_bold_title` (MOD:819-837). Two behaviors are surprising and
/// deliberate: the opening `**` is found *anywhere* in the line, so anything
/// before it is discarded; and the title is not trimmed.
fn extract_bold_title(text: &str) -> Option<(String, String)> {
    let start = text.find("**")?;
    let after = start + 2;
    let end = text[after..].find("**")?;
    let title = text[after..after + end].to_string();

    let rest = text[after + end + 2..].trim();
    // Exactly one separator, tried in this order: em dash, en dash, hyphen.
    let rest = rest
        .strip_prefix('\u{2014}')
        .or_else(|| rest.strip_prefix('\u{2013}'))
        .or_else(|| rest.strip_prefix('-'))
        .unwrap_or(rest)
        .trim();
    Some((title, rest.to_string()))
}

fn ensure_scope(step: &mut SopStep) -> &mut StepToolScope {
    step.scope.get_or_insert_with(StepToolScope::default)
}

fn ensure_schema(step: &mut SopStep) -> &mut StepSchema {
    step.schema.get_or_insert(StepSchema {
        input: None,
        output: None,
    })
}

/// Comma-separated list. Mirrors `parse_csv_list` (MOD:718-724).
///
/// Comma is the only separator, so `read_file shell` is one entry, not two.
fn parse_csv_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect()
}

/// Comma-separated step numbers. Mirrors `parse_u32_list` (MOD:726-731).
///
/// Also returns what it dropped, which upstream discards silently.
fn parse_u32_list(value: &str) -> (Vec<u32>, Vec<String>) {
    let mut kept = Vec::new();
    let mut dropped = Vec::new();
    for item in value.split(',') {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        match item.parse::<u32>() {
            Ok(n) => kept.push(n),
            Err(_) => dropped.push(item.to_string()),
        }
    }
    (kept, dropped)
}

/// Switch ports: `name>when>goto`, `;`-separated.
///
/// Mirrors `parse_switch_rules` (MOD:733-751). Also reports two silent traps:
/// a segment with an empty name is dropped entirely, and because the split is
/// `splitn(3, '>')` the third field absorbs every remaining `>` — so a guard
/// containing a `>` comparison is truncated and its target lost.
fn parse_switch_rules(value: &str) -> (Vec<SwitchRule>, Vec<String>) {
    let mut rules = Vec::new();
    let mut problems = Vec::new();
    for segment in value.split(';') {
        if segment.trim().is_empty() {
            continue;
        }
        let mut parts = segment.splitn(3, '>');
        let name = parts.next().unwrap_or("").trim().to_string();
        if name.is_empty() {
            problems.push(format!("dropped port with an empty name in `{segment}`"));
            continue;
        }
        let when = parts.next().unwrap_or("").trim();
        let goto_raw = parts.next().unwrap_or("").trim();
        let goto = goto_raw.parse::<u32>().ok();
        if goto.is_none() && !goto_raw.is_empty() {
            problems.push(format!(
                "port `{name}` has target `{goto_raw}`, which is not a step number; \
                 a `>` inside a guard truncates the port"
            ));
        }
        rules.push(SwitchRule {
            name,
            when: (!when.is_empty()).then(|| when.to_string()),
            goto,
        });
    }
    (rules, problems)
}

/// Failure policy. Mirrors `parse_step_failure` (MOD:775-795).
///
/// `fail` is case-insensitive; the `retry`/`goto` prefixes are not. Both a colon
/// and a space form are accepted. Anything unrecognized becomes `Fail`.
fn parse_step_failure(value: &str) -> StepFailure {
    let value = value.trim();
    if value.eq_ignore_ascii_case("fail") {
        return StepFailure::Fail;
    }
    if let Some(max) = value
        .strip_prefix("retry:")
        .or_else(|| value.strip_prefix("retry "))
        .and_then(|raw| raw.trim().parse::<u32>().ok())
    {
        return StepFailure::Retry { max };
    }
    if let Some(step) = value
        .strip_prefix("goto:")
        .or_else(|| value.strip_prefix("goto "))
        .and_then(|raw| raw.trim().parse::<u32>().ok())
    {
        return StepFailure::Goto { step };
    }
    StepFailure::Fail
}

/// Step kind. Mirrors `parse_step_kind` (MOD:753-759).
///
/// Note `approval` as an accepted spelling of `checkpoint`, and that everything
/// unrecognized falls to `Execute`.
fn parse_step_kind(value: &str) -> SopStepKind {
    match value.trim().to_ascii_lowercase().as_str() {
        "checkpoint" | "approval" => SopStepKind::Checkpoint,
        "capability" => SopStepKind::Capability,
        _ => SopStepKind::Execute,
    }
}

fn is_execute_spelling(value: &str) -> bool {
    value.trim().eq_ignore_ascii_case("execute")
}

/// Execution mode. Mirrors `parse_execution_mode` (MOD:170-179).
///
/// Unlike the manifest's strict serde path, this is lenient and lowercases
/// first, falling back to `Supervised`.
fn parse_execution_mode(value: &str) -> SopExecutionMode {
    match value.trim().to_lowercase().as_str() {
        "auto" => SopExecutionMode::Auto,
        "step_by_step" => SopExecutionMode::StepByStep,
        "priority_based" => SopExecutionMode::PriorityBased,
        "deterministic" => SopExecutionMode::Deterministic,
        _ => SopExecutionMode::Supervised,
    }
}

fn is_known_mode(value: &str) -> bool {
    matches!(
        value.trim().to_lowercase().as_str(),
        "auto" | "step_by_step" | "priority_based" | "deterministic" | "supervised"
    )
}

/// A `with:`, `input:` or `output:` value.
///
/// Mirrors `parse_value_fragment` (MOD:761-773): JSON first, then the fragment
/// wrapped as a TOML right-hand side, then the raw string. Nothing is ever
/// invalid, so this cannot fail.
///
/// Two consequences of the TOML stage are worth knowing: `1 # note` parses as
/// `1` with the comment discarded, and `'abc'` loses its quotes.
fn parse_value_fragment(value: &str) -> serde_json::Value {
    if let Ok(json) = serde_json::from_str(value) {
        return json;
    }
    if let Ok(table) = toml::from_str::<toml::Value>(&format!("value = {value}"))
        && let Some(inner) = table.get("value")
        && let Ok(json) = serde_json::to_value(inner)
    {
        return json;
    }
    serde_json::Value::String(value.to_string())
}

fn quoted(items: &[String]) -> String {
    items
        .iter()
        .map(|i| format!("`{i}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn steps(md: &str) -> Vec<SopStep> {
        parse_steps(md).0
    }

    fn one(md: &str) -> SopStep {
        let s = steps(md);
        assert_eq!(s.len(), 1, "expected exactly one step, got {}", s.len());
        s.into_iter().next().unwrap()
    }

    fn codes(md: &str) -> Vec<&'static str> {
        parse_steps(md).1.iter().map(|d| d.kind.code()).collect()
    }

    // ── Section location ────────────────────────────────────────

    #[test]
    fn steps_heading_is_ascii_case_insensitive() {
        for heading in ["## Steps", "## steps", "## STEPS"] {
            let md = format!("{heading}\n\n1. **A** - body\n");
            assert_eq!(steps(&md).len(), 1, "failed for {heading}");
        }
    }

    #[test]
    fn near_miss_headings_do_not_open_the_section() {
        // Equality, not prefix: exactly `##`, one space, `steps`.
        for heading in [
            "##  Steps",
            "## Steps:",
            "## Steps ##",
            "### Steps",
            "# Steps",
        ] {
            let md = format!("{heading}\n\n1. **A** - body\n");
            assert!(steps(&md).is_empty(), "{heading} should not open a section");
        }
    }

    #[test]
    fn content_before_the_section_is_discarded() {
        let md = "# Title\n\n1. **Not a step** - x\n\n## Steps\n\n1. **Real** - y\n";
        let s = steps(md);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].title, "Real");
    }

    #[test]
    fn another_h2_terminates_the_section_and_flushes() {
        let md = "## Steps\n\n1. **A** - body\n\n## Notes\n\nignored\n\n2. **B** - x\n";
        let s = steps(md);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].title, "A");
        assert_eq!(s[0].body, "body");
    }

    #[test]
    fn h1_and_h3_do_not_terminate_and_become_body() {
        let md = "## Steps\n\n1. **A** - body\n\n### Sub\n\nmore\n";
        let s = one(md);
        assert_eq!(s.body, "body\n### Sub\nmore");
    }

    #[test]
    fn a_second_steps_heading_does_not_flush_the_open_step() {
        // The `## Steps` branch continues before the flush, so the open step
        // stays open and keeps absorbing.
        let md = "## Steps\n\n1. **A** - body\n\n## Steps\n\ncontinued\n";
        let s = one(md);
        assert_eq!(s.body, "body\ncontinued");
    }

    #[test]
    fn a_bare_double_hash_is_body_not_a_heading() {
        // Trimming removes the trailing space, so "## " never matches.
        let md = "## Steps\n\n1. **A** - body\n\n## \n\nmore\n";
        let s = one(md);
        assert!(s.body.contains("##"), "{}", s.body);
    }

    #[test]
    fn a_section_left_open_at_eof_still_yields_its_step() {
        let s = one("## Steps\n\n1. **A** - body");
        assert_eq!(s.title, "A");
    }

    // ── Step items ──────────────────────────────────────────────

    #[test]
    fn only_number_dot_space_starts_a_step() {
        assert_eq!(steps("## Steps\n1. **A** - x\n").len(), 1);
        // No paren form, no bare `1.`, no space-less form, no bullet marker.
        assert!(steps("## Steps\n1) **A** - x\n").is_empty());
        assert!(steps("## Steps\n1.\n").is_empty());
        assert!(steps("## Steps\n1.**A** - x\n").is_empty());
        assert!(steps("## Steps\n- 1. **A** - x\n").is_empty());
    }

    #[test]
    fn authored_numbers_are_discarded_and_steps_renumbered_positionally() {
        let md = "## Steps\n\n3. **First** - a\n\n1. **Second** - b\n\n9. **Third** - c\n";
        let s = steps(md);
        // File order wins; the written digits contribute nothing, not even sort.
        assert_eq!(
            s.iter().map(|x| x.number).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        assert_eq!(
            s.iter().map(|x| x.title.as_str()).collect::<Vec<_>>(),
            vec!["First", "Second", "Third"]
        );
    }

    #[test]
    fn irregular_numbering_is_reported_even_though_upstream_cannot() {
        assert!(
            codes("## Steps\n1. **A** - x\n5. **B** - y\n").contains(&"numbering_irregularity")
        );
        // Gapless ascending is quiet.
        assert!(
            !codes("## Steps\n1. **A** - x\n2. **B** - y\n").contains(&"numbering_irregularity")
        );
    }

    #[test]
    fn leading_zeros_are_accepted() {
        assert_eq!(steps("## Steps\n007. **A** - x\n").len(), 1);
    }

    #[test]
    fn a_numeric_continuation_line_starts_a_new_step() {
        // A known hazard: prose beginning with a year parses as a step item.
        let md = "## Steps\n\n1. **A** - body\n2024. was a good year\n";
        assert_eq!(steps(md).len(), 2);
    }

    #[test]
    fn indentation_is_meaningless_so_nested_items_are_steps() {
        let md = "## Steps\n\n1. **A** - body\n    1. **Nested** - x\n";
        assert_eq!(steps(md).len(), 2);
    }

    // ── Titles ──────────────────────────────────────────────────

    #[test]
    fn title_and_body_split_on_bold_and_separator() {
        for sep in ["-", "\u{2014}", "\u{2013}"] {
            let md = format!("## Steps\n1. **Deploy** {sep} Ship it.\n");
            let s = one(&md);
            assert_eq!(s.title, "Deploy");
            assert_eq!(s.body, "Ship it.", "separator {sep:?}");
        }
    }

    #[test]
    fn separator_is_optional() {
        let s = one("## Steps\n1. **Resolve** Do the first step\n");
        assert_eq!(s.title, "Resolve");
        assert_eq!(s.body, "Do the first step");
    }

    #[test]
    fn absent_bold_makes_the_whole_line_the_title() {
        let s = one("## Steps\n1. Just prose here\n");
        assert_eq!(s.title, "Just prose here");
        assert_eq!(s.body, "");
    }

    #[test]
    fn unterminated_bold_keeps_the_asterisks() {
        let s = one("## Steps\n1. **Deploy\n");
        assert_eq!(s.title, "**Deploy");
    }

    #[test]
    fn bold_is_found_anywhere_and_discards_the_prefix() {
        // The most surprising title behavior, and it is upstream's.
        let s = one("## Steps\n1. Do the **thing** now\n");
        assert_eq!(s.title, "thing");
        assert_eq!(s.body, "now");
    }

    #[test]
    fn title_is_not_trimmed_but_body_is() {
        let s = one("## Steps\n1. ** Spaced ** -   body  \n");
        assert_eq!(s.title, " Spaced ");
        assert_eq!(s.body, "body");
    }

    #[test]
    fn only_the_first_bold_pair_is_consumed() {
        let s = one("## Steps\n1. **A** and **B** - x\n");
        assert_eq!(s.title, "A");
        assert_eq!(s.body, "and **B** - x");
    }

    #[test]
    fn multibyte_titles_do_not_panic() {
        let s = one("## Steps\n1. **Déployer — étape** - café ☕\n");
        assert_eq!(s.title, "Déployer — étape");
        assert_eq!(s.body, "café ☕");
    }

    // ── Bullets: every key ──────────────────────────────────────

    #[test]
    fn tools_is_comma_separated_only() {
        let s = one("## Steps\n1. **A** - x\n   - tools: file_read, shell\n");
        assert_eq!(s.suggested_tools, vec!["file_read", "shell"]);
        // Whitespace is not a separator.
        let s = one("## Steps\n1. **A** - x\n   - tools: file_read shell\n");
        assert_eq!(s.suggested_tools, vec!["file_read shell"]);
        // Empty entries dropped; a trailing comma is harmless.
        let s = one("## Steps\n1. **A** - x\n   - tools: a,,b,\n");
        assert_eq!(s.suggested_tools, vec!["a", "b"]);
    }

    #[test]
    fn allow_and_deny_tools_accept_both_spellings() {
        for key in ["allow-tools", "allow_tools"] {
            let md = format!("## Steps\n1. **A** - x\n   - {key}: shell\n");
            let s = one(&md);
            assert_eq!(s.scope.unwrap().allow, Some(vec!["shell".to_string()]));
        }
        for key in ["deny-tools", "deny_tools"] {
            let md = format!("## Steps\n1. **A** - x\n   - {key}: shell\n");
            let s = one(&md);
            assert_eq!(s.scope.unwrap().deny, vec!["shell"]);
        }
    }

    #[test]
    fn an_empty_allow_list_is_present_not_absent() {
        // Some([]) permits nothing; None permits anything not denied.
        let s = one("## Steps\n1. **A** - x\n   - allow-tools:\n");
        assert_eq!(s.scope.unwrap().allow, Some(vec![]));
    }

    #[test]
    fn deny_tools_materializes_the_scope_even_when_empty() {
        let s = one("## Steps\n1. **A** - x\n   - deny-tools:\n");
        assert!(s.scope.is_some());
    }

    #[test]
    fn requires_confirmation_has_no_hyphen_alias() {
        let s = one("## Steps\n1. **A** - x\n   - requires_confirmation: true\n");
        assert!(s.requires_confirmation);
        // The asymmetry is upstream's: there is no `requires-confirmation`.
        let s = one("## Steps\n1. **A** - x\n   - requires-confirmation: true\n");
        assert!(!s.requires_confirmation);
        assert!(s.body.contains("requires-confirmation"));
    }

    #[test]
    fn requires_confirmation_only_accepts_true() {
        for (value, expected) in [("true", true), ("TRUE", true), ("yes", false), ("1", false)] {
            let md = format!("## Steps\n1. **A** - x\n   - requires_confirmation: {value}\n");
            assert_eq!(one(&md).requires_confirmation, expected, "value {value}");
        }
    }

    #[test]
    fn kind_accepts_approval_as_a_checkpoint_spelling() {
        for value in ["checkpoint", "approval", "CHECKPOINT"] {
            let md = format!("## Steps\n1. **A** - x\n   - kind: {value}\n");
            assert_eq!(one(&md).kind, SopStepKind::Checkpoint, "value {value}");
        }
        let s = one("## Steps\n1. **A** - x\n   - kind: capability\n");
        assert_eq!(s.kind, SopStepKind::Capability);
    }

    #[test]
    fn an_unknown_kind_degrades_to_execute_but_is_reported() {
        let md = "## Steps\n1. **A** - x\n   - kind: chekpoint\n";
        assert_eq!(one(md).kind, SopStepKind::Execute);
        // Upstream is silent here, which turns a typo into a missing gate.
        assert!(codes(md).contains(&"malformed_bullet"));
        // The correct spelling is quiet.
        assert!(
            !codes("## Steps\n1. **A** - x\n   - kind: execute\n").contains(&"malformed_bullet")
        );
    }

    #[test]
    fn capability_keeps_an_empty_value_as_some() {
        let s = one("## Steps\n1. **A** - x\n   - capability:\n");
        assert_eq!(s.capability, Some(String::new()));
    }

    #[test]
    fn when_and_prompt_treat_empty_as_a_no_op() {
        let md = "## Steps\n1. **A** - x\n   - when: $.a == 1\n   - when:\n   - prompt: hi\n   - prompt:\n";
        let s = one(md);
        assert_eq!(s.routing.when.as_deref(), Some("$.a == 1"));
        assert_eq!(s.gate_prompt.as_deref(), Some("hi"));
    }

    #[test]
    fn agent_policy_and_edit_treat_empty_as_a_clear() {
        let md = "## Steps\n1. **A** - x\n   - agent: bot\n   - agent:\n   - policy: p\n   - policy:\n   - edit: body\n   - edit:\n";
        let s = one(md);
        assert_eq!(s.agent, None);
        assert_eq!(s.policy, None);
        assert_eq!(s.edit, None);
    }

    #[test]
    fn next_parses_and_a_bad_value_clears_it() {
        let s = one("## Steps\n1. **A** - x\n   - next: 3\n");
        assert_eq!(s.routing.next, Some(3));
        let md = "## Steps\n1. **A** - x\n   - next: 3\n   - next: abc\n";
        assert_eq!(one(md).routing.next, None);
        assert!(codes(md).contains(&"malformed_bullet"));
    }

    #[test]
    fn terminal_is_true_or_false() {
        assert!(
            one("## Steps\n1. **A** - x\n   - terminal: true\n")
                .routing
                .terminal
        );
        assert!(
            !one("## Steps\n1. **A** - x\n   - terminal: yes\n")
                .routing
                .terminal
        );
    }

    #[test]
    fn depends_on_accepts_both_spellings_and_reports_dropped_items() {
        for key in ["depends_on", "depends-on"] {
            let md = format!("## Steps\n1. **A** - x\n   - {key}: 1, 2\n");
            assert_eq!(one(&md).routing.depends_on, vec![1, 2]);
        }
        let md = "## Steps\n1. **A** - x\n   - depends_on: 1, two, 3\n";
        assert_eq!(one(md).routing.depends_on, vec![1, 3]);
        assert!(codes(md).contains(&"malformed_bullet"));
    }

    #[test]
    fn switch_parses_ports_in_order_with_a_catch_all() {
        let md = "## Steps\n1. **A** - x\n   - switch: pull_request>$.event>3; catch_all>>2\n";
        let s = one(md);
        assert_eq!(s.routing.switch.len(), 2);
        assert_eq!(s.routing.switch[0].name, "pull_request");
        assert_eq!(s.routing.switch[0].when.as_deref(), Some("$.event"));
        assert_eq!(s.routing.switch[0].goto, Some(3));
        assert_eq!(s.routing.switch[1].name, "catch_all");
        assert_eq!(s.routing.switch[1].when, None);
        assert_eq!(s.routing.switch[1].goto, Some(2));
    }

    #[test]
    fn a_greater_than_inside_a_switch_guard_truncates_and_is_reported() {
        // splitn(3, '>') means a comparison operator in a guard silently eats
        // the target. Upstream gives no parse-time signal at all.
        let md = "## Steps\n1. **A** - x\n   - switch: hot>$.n > 5>3\n";
        let s = one(md);
        assert_eq!(s.routing.switch[0].when.as_deref(), Some("$.n"));
        assert_eq!(s.routing.switch[0].goto, None);
        assert!(codes(md).contains(&"malformed_bullet"));
    }

    #[test]
    fn a_switch_port_with_an_empty_name_is_dropped_and_reported() {
        let md = "## Steps\n1. **A** - x\n   - switch: >$.x>2; ok>>3\n";
        let s = one(md);
        assert_eq!(s.routing.switch.len(), 1);
        assert_eq!(s.routing.switch[0].name, "ok");
        assert!(codes(md).contains(&"malformed_bullet"));
    }

    #[test]
    fn on_failure_accepts_both_spellings_and_both_forms() {
        for key in ["on_failure", "on-failure"] {
            for (value, expected) in [
                ("fail", StepFailure::Fail),
                ("FAIL", StepFailure::Fail),
                ("retry:2", StepFailure::Retry { max: 2 }),
                ("retry: 2", StepFailure::Retry { max: 2 }),
                ("retry 2", StepFailure::Retry { max: 2 }),
                ("goto:4", StepFailure::Goto { step: 4 }),
                ("goto 4", StepFailure::Goto { step: 4 }),
            ] {
                let md = format!("## Steps\n1. **A** - x\n   - {key}: {value}\n");
                assert_eq!(one(&md).on_failure, expected, "{key}: {value}");
            }
        }
    }

    #[test]
    fn retry_and_goto_prefixes_are_case_sensitive() {
        // `fail` folds case; the prefixes do not, so RETRY:2 degrades to Fail.
        let md = "## Steps\n1. **A** - x\n   - on_failure: RETRY:2\n";
        assert_eq!(one(md).on_failure, StepFailure::Fail);
        assert!(codes(md).contains(&"malformed_bullet"));
    }

    #[test]
    fn mode_always_assigns_some_and_reports_unknown_values() {
        let s = one("## Steps\n1. **A** - x\n   - mode: deterministic\n");
        assert_eq!(s.mode, Some(SopExecutionMode::Deterministic));
        // Unknown is Some(Supervised), never None.
        let md = "## Steps\n1. **A** - x\n   - mode: wobble\n";
        assert_eq!(one(md).mode, Some(SopExecutionMode::Supervised));
        assert!(codes(md).contains(&"malformed_bullet"));
    }

    #[test]
    fn call_accumulates_and_a_bad_call_is_dropped_with_a_diagnostic() {
        let md = "## Steps\n1. **A** - x\n   - call: {\"tool\":\"shell\",\"args\":{\"cmd\":\"ls\"}}\n   - call: {\"tool\":\"file_read\"}\n";
        let s = one(md);
        assert_eq!(s.calls.len(), 2, "call is the only appending key");
        assert_eq!(s.calls[0].tool, "shell");

        let md = "## Steps\n1. **A** - x\n   - call: {not json}\n";
        assert!(one(md).calls.is_empty());
        assert!(codes(md).contains(&"malformed_bullet"));
    }

    #[test]
    fn with_input_and_output_try_json_then_toml_then_string() {
        let s = one("## Steps\n1. **A** - x\n   - with: {\"a\": 1}\n");
        assert_eq!(s.capability_input, Some(serde_json::json!({"a": 1})));

        // The TOML stage is what makes the inline-table form work.
        let s = one("## Steps\n1. **A** - x\n   - with: { require_clean = true }\n");
        assert_eq!(
            s.capability_input,
            Some(serde_json::json!({"require_clean": true}))
        );

        let s = one("## Steps\n1. **A** - x\n   - input: not json at all\n");
        assert_eq!(
            s.schema.unwrap().input,
            Some(serde_json::Value::String("not json at all".into()))
        );

        let s = one("## Steps\n1. **A** - x\n   - output: 42\n");
        assert_eq!(s.schema.unwrap().output, Some(serde_json::json!(42)));
    }

    #[test]
    fn a_toml_comment_in_a_value_swallows_the_tail() {
        // Surprising, and upstream's: `value = 1 # note` is valid TOML.
        let s = one("## Steps\n1. **A** - x\n   - input: 1 # note\n");
        assert_eq!(s.schema.unwrap().input, Some(serde_json::json!(1)));
    }

    // ── Bullet recognition edges ────────────────────────────────

    #[test]
    fn keys_are_case_sensitive_and_the_colon_must_be_attached() {
        for bad in ["- Tools: shell", "- WHEN: x", "- tools : shell"] {
            let md = format!("## Steps\n1. **A** - x\n   {bad}\n");
            let s = one(&md);
            assert!(s.suggested_tools.is_empty(), "{bad} should not parse");
            assert!(s.body.contains(bad.trim_start_matches("- ")) || s.body.contains(bad));
        }
    }

    #[test]
    fn extra_spaces_after_the_bullet_hyphen_are_tolerated() {
        for prefix in ["- ", "-  ", "- - "] {
            let md = format!("## Steps\n1. **A** - x\n   {prefix}tools: shell\n");
            assert_eq!(one(&md).suggested_tools, vec!["shell"], "prefix {prefix:?}");
        }
    }

    #[test]
    fn non_hyphen_markers_are_not_bullets() {
        for marker in ["* tools: shell", "+ tools: shell", "-tools: shell"] {
            let md = format!("## Steps\n1. **A** - x\n   {marker}\n");
            assert!(one(&md).suggested_tools.is_empty(), "{marker}");
        }
    }

    #[test]
    fn bullets_before_the_first_step_are_discarded() {
        let md = "## Steps\n\n- tools: shell\n\n1. **A** - x\n";
        let s = one(md);
        assert!(s.suggested_tools.is_empty());
    }

    #[test]
    fn an_unrecognized_bullet_keeps_its_marker_in_the_body() {
        let md = "## Steps\n1. **A** - x\n   - notakey: value\n";
        let s = one(md);
        assert_eq!(s.body, "x\n- notakey: value");
        assert!(codes(md).contains(&"unrecognized_bullet"));
    }

    // ── Body accumulation ───────────────────────────────────────

    #[test]
    fn body_drops_blank_lines_and_trims_each_line() {
        let md = "## Steps\n1. **A** - first\n\n   second\n\n\n     third   \n";
        let s = one(md);
        // Paragraph breaks are destroyed; there is no way to keep one.
        assert_eq!(s.body, "first\nsecond\nthird");
    }

    #[test]
    fn repeated_scalar_bullets_take_the_last_value() {
        let md = "## Steps\n1. **A** - x\n   - next: 2\n   - next: 5\n";
        assert_eq!(one(md).routing.next, Some(5));
    }

    #[test]
    fn repeated_list_bullets_replace_rather_than_merge() {
        let md = "## Steps\n1. **A** - x\n   - tools: a\n   - tools: b\n";
        assert_eq!(one(md).suggested_tools, vec!["b"]);
    }

    // ── Whole documents ─────────────────────────────────────────

    #[test]
    fn the_capability_form_from_the_upstream_test_parses() {
        let md = "## Steps\n\n1. **Status** - Check the repository.\n   - kind: capability\n   - capability: git.status\n   - with: { require_clean = true }\n";
        let s = one(md);
        assert_eq!(s.kind, SopStepKind::Capability);
        assert_eq!(s.capability.as_deref(), Some("git.status"));
        assert_eq!(
            s.capability_input,
            Some(serde_json::json!({"require_clean": true}))
        );
    }

    #[test]
    fn the_capability_form_from_the_docs_does_not_parse() {
        // syntax.md:271-276 writes these on the title line, where they are
        // swallowed into the body. Reproducing the failure is the point: this
        // is a silent authoring trap upstream ships in its own documentation.
        let md = "## Steps\n\n1. **Draft** - kind: capability / capability: llm.generate\n";
        let s = one(md);
        assert_eq!(s.kind, SopStepKind::Execute);
        assert_eq!(s.capability, None);
        assert!(s.body.contains("kind: capability"));
    }

    #[test]
    fn a_realistic_multi_step_document_parses() {
        let md = r#"
# Deploy

Some prose that is discarded.

## Steps

1. **Triage** — Classify the incoming issue.
   - tools: file_read
   - output: {"type": "object", "properties": {"simple": {"type": "boolean"}}}
   - when: $.action == "opened"

2. **Fix** - Attempt a patch in the sandbox.
   - allow-tools: file_write, shell
   - deny-tools: git_operations
   - on_failure: retry:2

3. **Approve** - Human review.
   - kind: checkpoint
   - policy: prod
   - edit: body
   - prompt: Approve the patch for {{repo}}?

4. **Open PR** - Publish.
   - kind: capability
   - capability: forge.comment
   - depends_on: 2, 3
   - terminal: true
"#;
        let (s, diags) = parse_steps(md);
        assert_eq!(s.len(), 4);
        assert!(diags.is_empty(), "{diags:?}");

        assert_eq!(s[0].title, "Triage");
        assert_eq!(
            s[0].routing.when.as_deref(),
            Some(r#"$.action == "opened""#)
        );
        assert!(s[0].schema.as_ref().unwrap().output.is_some());

        assert_eq!(s[1].on_failure, StepFailure::Retry { max: 2 });
        let scope = s[1].scope.as_ref().unwrap();
        assert_eq!(
            scope.allow,
            Some(vec!["file_write".to_string(), "shell".to_string()])
        );
        assert_eq!(scope.deny, vec!["git_operations"]);

        assert_eq!(s[2].kind, SopStepKind::Checkpoint);
        assert_eq!(s[2].policy.as_deref(), Some("prod"));
        assert_eq!(s[2].edit.as_deref(), Some("body"));
        assert!(s[2].gate_prompt.is_some());

        assert_eq!(s[3].kind, SopStepKind::Capability);
        assert_eq!(s[3].routing.depends_on, vec![2, 3]);
        assert!(s[3].routing.terminal);
    }

    #[test]
    fn every_accepted_spelling_is_recognized() {
        // 21 keys, 25 spellings. A spelling that stops being recognized silently
        // becomes body text, so this enumerates them rather than trusting the
        // per-key tests to stay exhaustive.
        let spellings: &[(&str, &str)] = &[
            ("tools", "shell"),
            ("allow-tools", "shell"),
            ("allow_tools", "shell"),
            ("deny-tools", "shell"),
            ("deny_tools", "shell"),
            ("requires_confirmation", "true"),
            ("kind", "checkpoint"),
            ("capability", "git.status"),
            ("with", "{}"),
            ("input", "{}"),
            ("output", "{}"),
            ("when", "$.a == 1"),
            ("next", "2"),
            ("terminal", "true"),
            ("depends_on", "1"),
            ("depends-on", "1"),
            ("switch", "ok>>2"),
            ("on_failure", "fail"),
            ("on-failure", "fail"),
            ("mode", "auto"),
            ("agent", "bot"),
            ("call", r#"{"tool":"shell"}"#),
            ("prompt", "ok?"),
            ("policy", "prod"),
            ("edit", "body"),
        ];
        assert_eq!(spellings.len(), 25, "upstream accepts 25 spellings");

        for (key, value) in spellings {
            let md = format!("## Steps\n1. **A** - x\n   - {key}: {value}\n");
            let step = one(&md);
            assert_eq!(
                step.body, "x",
                "`{key}` was not recognized and leaked into the body"
            );
        }
    }

    #[test]
    fn the_deploy_example_from_syntax_md_parses() {
        // syntax.md:111-124, verbatim.
        let md = r#"## Steps

1. **Preflight** — Check service health and release window.
   - tools: http_request

2. **Deploy** — Run deployment command.
   - tools: shell
   - requires_confirmation: true
   - policy: prod
   - input: {"type":"object","required":["version"],"properties":{"version":{"type":"string"}}}
   - output: {"type":"object","required":["digest"],"properties":{"digest":{"type":"string"}}}
   - next: 3
"#;
        let (s, diags) = parse_steps(md);
        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(s.len(), 2);

        assert_eq!(s[0].title, "Preflight");
        assert_eq!(s[0].body, "Check service health and release window.");
        assert_eq!(s[0].suggested_tools, vec!["http_request"]);

        assert_eq!(s[1].title, "Deploy");
        assert!(s[1].requires_confirmation);
        assert_eq!(s[1].policy.as_deref(), Some("prod"));
        assert_eq!(s[1].routing.next, Some(3));
        let schema = s[1].schema.as_ref().unwrap();
        assert_eq!(
            schema.input.as_ref().unwrap()["required"],
            serde_json::json!(["version"])
        );
        assert_eq!(
            schema.output.as_ref().unwrap()["required"],
            serde_json::json!(["digest"])
        );
        // `next: 3` against a two-step list — upstream resolves cross-references
        // against the renumbered list, so this dangles. Detecting that is a
        // verifier rule, not a parser concern.
        assert!(s.len() < 3);
    }

    #[test]
    fn the_combined_routing_example_from_syntax_md_parses() {
        // syntax.md:128-152, verbatim.
        let md = r#"## Steps

1. **Classify event** — Inspect the incoming payload.
   - output: {"type":"object","required":["severity"],"properties":{"severity":{"type":"string"}}}
   - when: $.steps.1.severity == "critical"
   - next: 2

2. **Prepare summary** — Build the operator-facing remediation plan.
   - depends_on: 1
   - on_failure: retry:2
   - next: 3

3. **Approval gate** — Require explicit approval before changing state.
   - kind: checkpoint
   - requires_confirmation: true
   - next: 4

4. **Apply remediation** — Execute the approved action.
   - tools: shell
   - allow-tools: shell
   - on_failure: goto:5

5. **Notify operator** — Send a failure notice for follow-up.
   - tools: http_request
"#;
        let (s, diags) = parse_steps(md);
        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(s.len(), 5);
        assert_eq!(
            s.iter().map(|x| x.number).collect::<Vec<_>>(),
            vec![1, 2, 3, 4, 5]
        );

        assert_eq!(
            s[0].routing.when.as_deref(),
            Some(r#"$.steps.1.severity == "critical""#)
        );
        assert_eq!(s[1].routing.depends_on, vec![1]);
        assert_eq!(s[1].on_failure, StepFailure::Retry { max: 2 });
        assert_eq!(s[2].kind, SopStepKind::Checkpoint);
        assert!(s[2].requires_confirmation);
        assert_eq!(s[3].on_failure, StepFailure::Goto { step: 5 });
        // `tools:` and `allow-tools:` are both present and stay distinct: the
        // legacy alias only fills an absent allow-list, so collapsing them here
        // would lose which one the author wrote.
        assert_eq!(s[3].suggested_tools, vec!["shell"]);
        assert_eq!(
            s[3].scope.as_ref().unwrap().allow,
            Some(vec!["shell".to_string()])
        );
        assert_eq!(s[4].title, "Notify operator");
    }

    #[test]
    fn no_input_panics() {
        // The parser is total: every one of these must return, not unwind.
        let hostile = [
            "",
            "## Steps",
            "## Steps\n1. ",
            "## Steps\n1. **",
            "## Steps\n1. ****",
            "## Steps\n1. **A**\n   - switch: >>>>>>\n",
            "## Steps\n1. **A**\n   - call: \n",
            "## Steps\n1. **A**\n   - depends_on: ,,,\n",
            "## Steps\n999999999999999999999. **A** - x\n",
            "## Steps\n1. **é** - ☕\n   - with: {broken\n",
            "## Steps\n1. **A**\n   - :\n",
            "## Steps\n1. **A**\n   - : x\n",
        ];
        for md in hostile {
            let _ = parse_steps(md);
        }
    }
}
