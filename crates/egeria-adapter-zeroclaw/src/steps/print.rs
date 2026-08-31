//! Writing the step list back out as `SOP.md`.
//!
//! The contract is `parse(print(m)) == m` for every model the parser can
//! produce. That is a stronger guarantee than upstream offers: its own
//! `render_steps` (MOD:932-948) carries a doc comment claiming to be the
//! inverse of `parse_steps`, and is not. It never emits `capability:`, `with:`,
//! `policy:`, `prompt:`, or `edit:`, and it only emits `kind:` for a checkpoint
//! — so a capability step round-trips into a plain execute step with no
//! capability and no arguments, which is the exact shape `syntax.md` promotes.
//!
//! Egeria emits all 21 keys. Where upstream does emit a key, its spelling is
//! matched exactly so that files written by either side diff cleanly: hyphens
//! for `allow-tools` and `deny-tools`, underscores for `requires_confirmation`,
//! `depends_on` and `on_failure`, and the space-after-colon form for failure
//! policies (`retry: 2`).
//!
//! Bullet order follows the parser's own match chain (MOD:555-633). No key is a
//! prefix of another once the colon is attached, so the order is a readability
//! choice rather than a semantic one.

use crate::diagnostic::{Diagnostic, DiagnosticKind, Location};
use crate::model::{SopStep, SopStepKind, StepFailure};

/// Render steps as the `## Steps` section of a `SOP.md`.
///
/// Returns the markdown and any diagnostics for constructs that cannot survive
/// the trip. The diagnostics are empty for every model the parser can produce;
/// they exist for models built by hand, which can hold values the format has no
/// way to express.
pub fn print_steps(steps: &[SopStep]) -> (String, Vec<Diagnostic>) {
    let mut out = String::from("## Steps\n");
    let mut diagnostics = Vec::new();

    for step in steps {
        out.push('\n');
        check_round_trip(step, &mut diagnostics);

        let body = step.body.trim();
        // A body's first line rides on the title line after the separator;
        // upstream omits the separator entirely when the body is empty.
        match body.lines().next() {
            Some(first) => {
                out.push_str(&format!(
                    "{}. **{}** - {}\n",
                    step.number, step.title, first
                ));
            }
            None => out.push_str(&format!("{}. **{}**\n", step.number, step.title)),
        }
        for line in body.lines().skip(1) {
            out.push_str(&format!("   {line}\n"));
        }

        for bullet in step_bullets(step) {
            out.push_str(&format!("   - {bullet}\n"));
        }
    }
    (out, diagnostics)
}

/// Flag values the format cannot represent.
///
/// None of these are reachable from parsed input — the parser's own splitting
/// makes a `>` in a guard or a `;` in a port name impossible — but a
/// hand-constructed step can hold them, and silently emitting something that
/// reads back differently is the one outcome worth refusing to do quietly.
fn check_round_trip(step: &SopStep, out: &mut Vec<Diagnostic>) {
    let mut lossy = |construct: &str, detail: String| {
        out.push(Diagnostic::new(
            DiagnosticKind::LossyConstruct {
                construct: construct.to_string(),
                detail,
            },
            Location::Step {
                number: step.number,
                line: None,
            },
        ));
    };

    for rule in &step.routing.switch {
        if rule.name.is_empty() {
            lossy(
                "switch",
                "a port with an empty name is dropped when re-read".into(),
            );
        }
        if rule.name.contains('>') || rule.name.contains(';') {
            lossy(
                "switch",
                format!("port name `{}` contains a separator", rule.name),
            );
        }
        if let Some(when) = &rule.when
            && (when.contains('>') || when.contains(';'))
        {
            lossy(
                "switch",
                format!("guard `{when}` contains a separator; the port would be truncated"),
            );
        }
    }

    if step.title.contains("**") {
        lossy(
            "title",
            "a title containing `**` re-reads as a shorter title".into(),
        );
    }

    // A continuation line that looks like a numbered item opens a new step on
    // the next read. The first body line is safe: it rides on the title line.
    for line in step.body.trim().lines().skip(1) {
        let line = line.trim();
        if let Some(dot) = line.find(". ")
            && !line[..dot].is_empty()
            && line[..dot].chars().all(|c| c.is_ascii_digit())
        {
            lossy(
                "body",
                format!("the line `{line}` would re-read as a new step"),
            );
        }
    }

    for name in step.suggested_tools.iter().chain(
        step.scope
            .iter()
            .flat_map(|s| s.allow.iter().flatten().chain(s.deny.iter())),
    ) {
        if name.contains(',') {
            lossy("tools", format!("tool name `{name}` contains a comma"));
        }
    }
}

fn step_bullets(step: &SopStep) -> Vec<String> {
    let mut bullets = Vec::new();

    if !step.suggested_tools.is_empty() {
        bullets.push(format!("tools: {}", step.suggested_tools.join(", ")));
    }
    if let Some(scope) = &step.scope {
        // `Some([])` is emitted as an empty allow-list rather than skipped:
        // present-but-empty permits nothing, absent permits anything.
        if let Some(allow) = &scope.allow {
            bullets.push(format!("allow-tools: {}", allow.join(", ")));
        }
        if !scope.deny.is_empty() {
            bullets.push(format!("deny-tools: {}", scope.deny.join(", ")));
        }
    }
    if step.requires_confirmation {
        bullets.push("requires_confirmation: true".into());
    }
    // Upstream emits this only for a checkpoint, which is why its capability
    // steps decay into execute steps.
    if step.kind != SopStepKind::Execute {
        bullets.push(format!("kind: {}", step.kind));
    }
    if let Some(capability) = &step.capability {
        bullets.push(format!("capability: {capability}"));
    }
    if let Some(with) = &step.capability_input {
        bullets.push(format!("with: {}", compact(with)));
    }
    if let Some(schema) = &step.schema {
        if let Some(input) = &schema.input {
            bullets.push(format!("input: {}", compact(input)));
        }
        if let Some(output) = &schema.output {
            bullets.push(format!("output: {}", compact(output)));
        }
    }
    if let Some(when) = &step.routing.when {
        bullets.push(format!("when: {when}"));
    }
    if let Some(next) = step.routing.next {
        bullets.push(format!("next: {next}"));
    }
    if step.routing.terminal {
        bullets.push("terminal: true".into());
    }
    if !step.routing.depends_on.is_empty() {
        let list = step
            .routing
            .depends_on
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        bullets.push(format!("depends_on: {list}"));
    }
    if !step.routing.switch.is_empty() {
        let ports = step
            .routing
            .switch
            .iter()
            .map(|rule| {
                format!(
                    "{}>{}>{}",
                    rule.name,
                    rule.when.as_deref().unwrap_or(""),
                    rule.goto.map(|g| g.to_string()).unwrap_or_default()
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        bullets.push(format!("switch: {ports}"));
    }
    match &step.on_failure {
        StepFailure::Fail => {}
        StepFailure::Retry { max } => bullets.push(format!("on_failure: retry: {max}")),
        StepFailure::Goto { step } => bullets.push(format!("on_failure: goto: {step}")),
    }
    if let Some(mode) = step.mode {
        bullets.push(format!("mode: {}", mode_str(mode)));
    }
    if let Some(agent) = &step.agent {
        bullets.push(format!("agent: {agent}"));
    }
    for call in &step.calls {
        bullets.push(format!(
            "call: {}",
            serde_json::to_string(call).unwrap_or_else(|_| "{}".into())
        ));
    }
    if let Some(prompt) = &step.gate_prompt {
        bullets.push(format!("prompt: {prompt}"));
    }
    if let Some(policy) = &step.policy {
        bullets.push(format!("policy: {policy}"));
    }
    if let Some(edit) = &step.edit {
        bullets.push(format!("edit: {edit}"));
    }

    bullets
}

fn compact(value: &serde_json::Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "null".into())
}

fn mode_str(mode: crate::model::SopExecutionMode) -> &'static str {
    use crate::model::SopExecutionMode as M;
    match mode {
        M::Auto => "auto",
        M::Supervised => "supervised",
        M::StepByStep => "step_by_step",
        M::PriorityBased => "priority_based",
        M::Deterministic => "deterministic",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{PlannedToolCall, SopExecutionMode, StepSchema, StepToolScope, SwitchRule};
    use crate::steps::parse::parse_steps;

    /// The contract: printing a parsed document and reparsing yields the same
    /// steps.
    fn assert_round_trips(md: &str) -> Vec<SopStep> {
        let (steps, _) = parse_steps(md);
        let (printed, lossy) = print_steps(&steps);
        assert!(lossy.is_empty(), "unexpected loss: {lossy:?}\n{printed}");
        let (again, _) = parse_steps(&printed);
        assert_eq!(
            steps, again,
            "round trip changed the steps.\n--- printed ---\n{printed}"
        );
        steps
    }

    #[test]
    fn a_minimal_step_round_trips() {
        assert_round_trips("## Steps\n1. **A** - body\n");
    }

    #[test]
    fn an_empty_body_omits_the_separator() {
        let (printed, _) = print_steps(&[SopStep {
            number: 1,
            title: "A".into(),
            ..Default::default()
        }]);
        assert!(printed.contains("1. **A**\n"), "{printed}");
        assert!(!printed.contains("**A** -"), "{printed}");
        assert_round_trips(&printed);
    }

    #[test]
    fn every_bullet_key_survives_a_round_trip() {
        // The point of the exercise: upstream loses five of these outright.
        let md = r#"## Steps

1. **Everything** - the body
   - tools: file_read, shell
   - allow-tools: shell
   - deny-tools: git_operations
   - requires_confirmation: true
   - kind: capability
   - capability: forge.comment
   - with: {"repo":"o/r"}
   - input: {"type":"object"}
   - output: {"type":"string"}
   - when: $.a == 1
   - next: 2
   - terminal: true
   - depends_on: 1, 2
   - switch: hot>$.sev>3; other>>4
   - on_failure: retry:2
   - mode: deterministic
   - agent: bot
   - call: {"tool":"shell","args":{"cmd":"ls"}}
   - prompt: approve?
   - policy: prod
   - edit: body
"#;
        let steps = assert_round_trips(md);
        let s = &steps[0];
        // Spot-check the five upstream drops.
        assert_eq!(s.kind, SopStepKind::Capability);
        assert_eq!(s.capability.as_deref(), Some("forge.comment"));
        assert!(s.capability_input.is_some());
        assert_eq!(s.policy.as_deref(), Some("prod"));
        assert_eq!(s.gate_prompt.as_deref(), Some("approve?"));
        assert_eq!(s.edit.as_deref(), Some("body"));
    }

    #[test]
    fn a_capability_step_does_not_decay_into_an_execute_step() {
        // This is the exact regression upstream's renderer has.
        let md = "## Steps\n1. **Status** - Check.\n   - kind: capability\n   - capability: git.status\n   - with: { require_clean = true }\n";
        let (steps, _) = parse_steps(md);
        let (printed, _) = print_steps(&steps);
        assert!(printed.contains("kind: capability"), "{printed}");
        assert!(printed.contains("capability: git.status"), "{printed}");
        assert!(printed.contains("with:"), "{printed}");
        let (again, _) = parse_steps(&printed);
        assert_eq!(again[0].kind, SopStepKind::Capability);
        assert_eq!(again[0].capability.as_deref(), Some("git.status"));
    }

    #[test]
    fn canonical_spellings_match_upstream_where_it_emits_them() {
        let md = "## Steps\n1. **A** - x\n   - allow_tools: shell\n   - deny_tools: git\n   - depends-on: 1\n   - on-failure: retry 3\n";
        let (steps, _) = parse_steps(md);
        let (printed, _) = print_steps(&steps);
        // Hyphens for the tool scopes, underscores for the rest, and the
        // space-after-colon failure form — matching what ZeroClaw writes so the
        // two do not fight over diffs.
        assert!(printed.contains("- allow-tools: shell"), "{printed}");
        assert!(printed.contains("- deny-tools: git"), "{printed}");
        assert!(printed.contains("- depends_on: 1"), "{printed}");
        assert!(printed.contains("- on_failure: retry: 3"), "{printed}");
    }

    #[test]
    fn both_failure_forms_round_trip() {
        for value in ["retry:0", "retry:2", "goto:5", "fail"] {
            let md = format!("## Steps\n1. **A** - x\n   - on_failure: {value}\n");
            assert_round_trips(&md);
        }
    }

    #[test]
    fn an_empty_allow_list_stays_present() {
        let md = "## Steps\n1. **A** - x\n   - allow-tools:\n";
        let steps = assert_round_trips(md);
        assert_eq!(steps[0].scope.as_ref().unwrap().allow, Some(vec![]));
    }

    #[test]
    fn switch_ports_round_trip_including_the_catch_all() {
        let md = "## Steps\n1. **A** - x\n   - switch: hot>$.sev>3; catch>>4; bare>>\n";
        let steps = assert_round_trips(md);
        let switch = &steps[0].routing.switch;
        assert_eq!(switch.len(), 3);
        assert_eq!(switch[1].when, None);
        assert_eq!(switch[2].goto, None);
    }

    #[test]
    fn multiline_bodies_round_trip() {
        let md = "## Steps\n1. **A** - first line\n   second line\n   third line\n";
        let steps = assert_round_trips(md);
        assert_eq!(steps[0].body, "first line\nsecond line\nthird line");
    }

    #[test]
    fn unrecognized_bullets_in_a_body_round_trip() {
        // They keep their marker, so on reread they are unrecognized again and
        // land back in the body unchanged.
        let md = "## Steps\n1. **A** - x\n   - notakey: value\n";
        let steps = assert_round_trips(md);
        assert_eq!(steps[0].body, "x\n- notakey: value");
    }

    #[test]
    fn value_fragments_round_trip_through_json() {
        for value in [
            r#"{"a":1}"#,
            "42",
            "true",
            "null",
            r#""a string""#,
            "[1,2,3]",
        ] {
            let md = format!("## Steps\n1. **A** - x\n   - input: {value}\n");
            assert_round_trips(&md);
        }
    }

    #[test]
    fn a_bare_string_fragment_round_trips() {
        // Parsed as String("not json"), printed back as JSON `"not json"`,
        // which reparses to the same string.
        let md = "## Steps\n1. **A** - x\n   - input: not json at all\n";
        let steps = assert_round_trips(md);
        assert_eq!(
            steps[0].schema.as_ref().unwrap().input,
            Some(serde_json::Value::String("not json at all".into()))
        );
    }

    #[test]
    fn multiple_calls_round_trip_in_order() {
        let md = "## Steps\n1. **A** - x\n   - call: {\"tool\":\"shell\"}\n   - call: {\"tool\":\"file_read\"}\n";
        let steps = assert_round_trips(md);
        assert_eq!(steps[0].calls.len(), 2);
        assert_eq!(steps[0].calls[0].tool, "shell");
        assert_eq!(steps[0].calls[1].tool, "file_read");
    }

    #[test]
    fn the_syntax_md_examples_round_trip() {
        assert_round_trips(
            r#"## Steps

1. **Preflight** — Check service health and release window.
   - tools: http_request

2. **Deploy** — Run deployment command.
   - tools: shell
   - requires_confirmation: true
   - policy: prod
   - next: 3
"#,
        );
        assert_round_trips(
            r#"## Steps

1. **Classify event** — Inspect the incoming payload.
   - when: $.steps.1.severity == "critical"
   - next: 2

2. **Prepare summary** — Build the plan.
   - depends_on: 1
   - on_failure: retry:2

3. **Approval gate** — Require approval.
   - kind: checkpoint
   - requires_confirmation: true

4. **Apply remediation** — Execute.
   - tools: shell
   - allow-tools: shell
   - on_failure: goto:5

5. **Notify operator** — Send a notice.
   - tools: http_request
"#,
        );
    }

    #[test]
    fn printing_is_stable() {
        let md = "## Steps\n1. **A** - x\n   - tools: shell\n   - next: 2\n";
        let (steps, _) = parse_steps(md);
        let (a, _) = print_steps(&steps);
        let (b, _) = print_steps(&steps);
        assert_eq!(a, b);
    }

    #[test]
    fn a_body_line_that_looks_like_a_step_is_reported() {
        // Reachable only by hand: the parser turns such a line into a step.
        let step = SopStep {
            number: 1,
            title: "A".into(),
            body: "first\n2024. was a good year".into(),
            ..Default::default()
        };
        let (_, lossy) = print_steps(&[step]);
        assert_eq!(lossy.len(), 1, "{lossy:?}");
        assert_eq!(lossy[0].kind.code(), "lossy_construct");
    }

    #[test]
    fn a_switch_guard_containing_a_separator_is_reported() {
        let step = SopStep {
            number: 1,
            title: "A".into(),
            routing: crate::model::StepRouting {
                switch: vec![SwitchRule {
                    name: "hot".into(),
                    when: Some("$.n > 5".into()),
                    goto: Some(3),
                }],
                ..Default::default()
            },
            ..Default::default()
        };
        let (_, lossy) = print_steps(&[step]);
        assert_eq!(lossy.len(), 1, "{lossy:?}");
        assert!(lossy[0].message.contains("separator"), "{:?}", lossy[0]);
    }

    #[test]
    fn a_tool_name_containing_a_comma_is_reported() {
        let step = SopStep {
            number: 1,
            title: "A".into(),
            suggested_tools: vec!["a,b".into()],
            ..Default::default()
        };
        let (_, lossy) = print_steps(&[step]);
        assert_eq!(lossy.len(), 1, "{lossy:?}");
    }

    #[test]
    fn a_hand_built_step_with_every_field_round_trips() {
        let step = SopStep {
            number: 1,
            title: "Everything".into(),
            body: "line one\nline two".into(),
            suggested_tools: vec!["file_read".into()],
            requires_confirmation: true,
            kind: SopStepKind::Capability,
            schema: Some(StepSchema {
                input: Some(serde_json::json!({"type":"object"})),
                output: Some(serde_json::json!({"type":"string"})),
            }),
            scope: Some(StepToolScope {
                allow: Some(vec!["shell".into()]),
                deny: vec!["git".into()],
            }),
            routing: crate::model::StepRouting {
                when: Some("$.a == 1".into()),
                next: Some(2),
                terminal: true,
                depends_on: vec![1, 2],
                switch: vec![SwitchRule {
                    name: "hot".into(),
                    when: Some("$.sev".into()),
                    goto: Some(3),
                }],
            },
            on_failure: StepFailure::Retry { max: 2 },
            mode: Some(SopExecutionMode::Deterministic),
            calls: vec![PlannedToolCall {
                tool: "shell".into(),
                args: serde_json::json!({"cmd":"ls"}),
                pinned: None,
            }],
            pos: None,
            agent: Some("bot".into()),
            capability: Some("forge.comment".into()),
            capability_input: Some(serde_json::json!({"repo":"o/r"})),
            policy: Some("prod".into()),
            gate_prompt: Some("approve?".into()),
            edit: Some("body".into()),
        };
        let (printed, lossy) = print_steps(std::slice::from_ref(&step));
        assert!(lossy.is_empty(), "{lossy:?}");
        let (again, _) = parse_steps(&printed);
        assert_eq!(again.len(), 1);
        assert_eq!(again[0], step, "--- printed ---\n{printed}");
    }
}
