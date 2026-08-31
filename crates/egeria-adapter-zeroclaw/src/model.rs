//! Egeria's model of the ZeroClaw SOP file format.
//!
//! These types mirror the shapes ZeroClaw serializes, but Egeria owns them
//! (ADR-0005): the adapter parses the documented format itself rather than
//! linking `zeroclaw-runtime`. Every type below records the upstream location
//! it mirrors so the correspondence can be rechecked when the pin moves.
//!
//! Upstream paths in citations are relative to `external/zeroclaw/`, and
//! abbreviate as:
//!
//! * `TYPES` = `crates/zeroclaw-runtime/src/sop/types.rs`
//! * `CONTRACT` = `crates/zeroclaw-runtime/src/sop/step_contract.rs`
//! * `SCOPE` = `crates/zeroclaw-runtime/src/sop/scope/mod.rs`
//! * `MOD` = `crates/zeroclaw-runtime/src/sop/mod.rs`
//!
//! Two upstream conventions shape everything here. Nothing in the SOP tree uses
//! `deny_unknown_fields`, so unknown keys are accepted and discarded — Egeria
//! keeps that tolerance but reports what it dropped. And no field has a serde
//! `alias`, so each TOML key has exactly one accepted spelling; the
//! hyphen/underscore pairs authors write live in the `SOP.md` bullet grammar,
//! not here.

use serde::{Deserialize, Serialize};

// ── Enums ───────────────────────────────────────────────────────

/// Scheduling priority. Mirrors `SopPriority` (TYPES:11-21).
///
/// Also gates steps under [`SopExecutionMode::PriorityBased`], so this is not
/// purely a scheduling hint.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SopPriority {
    Low,
    #[default]
    Normal,
    High,
    Critical,
}

/// How a procedure's steps are driven. Mirrors `SopExecutionMode`
/// (TYPES:36-54).
///
/// The upstream enum's own `Default` is `Supervised`, but that is *not* the
/// manifest default: [`SopMeta::execution_mode`] is an `Option` defaulting to
/// `None`, and an absent value falls back to a daemon-config setting Egeria
/// cannot see. Resolution is left to the caller rather than guessed here.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SopExecutionMode {
    Auto,
    #[default]
    Supervised,
    StepByStep,
    PriorityBased,
    Deterministic,
}

/// What to do when a trigger fires while the procedure is already running.
/// Mirrors `SopAdmissionPolicy` (TYPES:529-548).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SopAdmissionPolicy {
    #[default]
    Parallel,
    Hold,
    Coalesce,
    Drop,
}

/// Filesystem events a watch trigger reacts to. Mirrors
/// `FilesystemEventKind` (TYPES:71-91).
///
/// Upstream's `FromStr` lowercases before matching, but serde does not, so
/// `events = ["Created"]` is a parse error. That asymmetry is deliberate here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FilesystemEventKind {
    Created,
    Modified,
    Deleted,
    Renamed,
}

/// What a step is. Mirrors `SopStepKind` (TYPES:238-251).
///
/// Whether a `Checkpoint` actually gates depends on the resolved execution
/// mode, which is a semantic question for import (issue #9) rather than a
/// parsing one.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SopStepKind {
    #[default]
    Execute,
    Checkpoint,
    Capability,
}

impl std::fmt::Display for SopStepKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Execute => "execute",
            Self::Checkpoint => "checkpoint",
            Self::Capability => "capability",
        })
    }
}

// ── Step components ─────────────────────────────────────────────

/// One named output port on a switch step. Mirrors `SwitchRule`
/// (CONTRACT:9-18).
///
/// Ports are evaluated top to bottom and the first whose guard passes wins. A
/// port with no `when` is the catch-all and belongs last.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwitchRule {
    /// Port label, shown on the node's output pin.
    pub name: String,
    /// Guard expression. `None` makes this the catch-all.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    /// Step this port routes to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goto: Option<u32>,
}

/// Where a step goes next. Mirrors `StepRouting` (CONTRACT:20-41).
///
/// The interaction between these fields is not obvious and is not what the
/// prose documentation describes; see `import` (issue #9) for the resolved
/// precedence. Parsing keeps them exactly as authored.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepRouting {
    /// Guard on this step running at all.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    /// Explicit successor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<u32>,
    /// Ends this branch: no implicit fallthrough to the following step.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub terminal: bool,
    /// Steps that must have completed before this one runs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<u32>,
    /// Ordered switch ports. Non-empty makes this a multi-branch node.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub switch: Vec<SwitchRule>,
}

impl StepRouting {
    /// Whether this is entirely default, i.e. plain linear fallthrough.
    ///
    /// Upstream uses the equivalent predicate to skip the field on
    /// serialization (CONTRACT:43-47); the printer relies on the same test.
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }
}

/// What happens when a step fails. Mirrors `StepFailure` (CONTRACT:50-62).
///
/// Externally tagged in `snake_case`, so the TOML forms are `"fail"`,
/// `{ retry = { max = 3 } }`, and `{ goto = { step = 4 } }`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepFailure {
    /// Fail the run.
    #[default]
    Fail,
    /// Retry this step up to `max` times.
    ///
    /// The authored bound is clamped against a daemon-config cap at run time,
    /// not at parse time, so a value here may exceed what actually happens.
    Retry { max: u32 },
    /// Jump to another step.
    ///
    /// On a denied approval this is what continues the run, which is why it
    /// matters to the approval-domination rule (issue #16).
    Goto { step: u32 },
}

impl StepFailure {
    /// Whether this is the default fail-the-run policy.
    pub fn is_fail(&self) -> bool {
        matches!(self, Self::Fail)
    }
}

/// Typed input and output contracts for a step. Mirrors `StepSchema`
/// (TYPES:265-277).
///
/// Held as opaque JSON: Egeria's own compact type system is an IR concern
/// (issue #6), and the adapter must not lose anything by interpreting early.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StepSchema {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<serde_json::Value>,
}

/// Per-step tool allow/deny scope. Mirrors `StepToolScope` (SCOPE:9-19).
///
/// `allow` being `Option` is meaningful: absent means "no allow-list", which
/// differs from present-but-empty.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepToolScope {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deny: Vec<String>,
}

/// A tool invocation planned for a step. Mirrors `PlannedToolCall`
/// (TYPES:287-297).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlannedToolCall {
    pub tool: String,
    /// Argument template; string leaves may carry `{{steps.N}}` bindings.
    #[serde(default)]
    pub args: serde_json::Value,
    /// Pinned sample output, for authoring without re-running the tool.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned: Option<serde_json::Value>,
}

/// A step's canvas coordinate. Mirrors `StepPos` (TYPES:299-306).
///
/// View data, never semantic (ADR-0003).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StepPos {
    pub x: f64,
    pub y: f64,
}

// ── Step ────────────────────────────────────────────────────────

/// A single step. Mirrors `SopStep` (TYPES:308-392).
///
/// Every field carries `#[serde(default)]` upstream, so a `[[steps]]` table may
/// omit any of them. `PartialEq` but not `Eq`, because the JSON-valued fields
/// can hold floats.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SopStep {
    /// Position within the procedure, 1-based.
    ///
    /// Authored numbers are advisory: steps parsed from `SOP.md` are renumbered
    /// positionally, so what an author wrote is not necessarily what runs.
    #[serde(default)]
    pub number: u32,
    #[serde(default)]
    pub title: String,
    /// The instruction body, in Markdown.
    #[serde(default)]
    pub body: String,
    /// Advisory tool names. A legacy alias for `scope.allow`, kept separate
    /// because collapsing the two would lose which spelling was authored.
    #[serde(default)]
    pub suggested_tools: Vec<String>,
    /// Pause for confirmation before running.
    #[serde(default)]
    pub requires_confirmation: bool,
    #[serde(default)]
    pub kind: SopStepKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<StepSchema>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<StepToolScope>,
    #[serde(default, skip_serializing_if = "StepRouting::is_default")]
    pub routing: StepRouting,
    #[serde(default, skip_serializing_if = "StepFailure::is_fail")]
    pub on_failure: StepFailure,
    /// Per-step execution mode override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<SopExecutionMode>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub calls: Vec<PlannedToolCall>,
    /// Canvas coordinate. View data only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pos: Option<StepPos>,
    /// Agent alias running this step; unset inherits the procedure's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    /// Capability identifier, used when `kind = "capability"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability: Option<String>,
    /// Capability arguments. Serialized as `with` — the single renamed key in
    /// the whole manifest tree (TYPES:373).
    #[serde(default, rename = "with", skip_serializing_if = "Option::is_none")]
    pub capability_input: Option<serde_json::Value>,
    /// Approval policy name, naming a group and quorum the broker enforces.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy: Option<String>,
    /// Gate-notice template for a human approval step.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate_prompt: Option<String>,
    /// Field an approver may amend before the run resumes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edit: Option<String>,
}

// ── Triggers ────────────────────────────────────────────────────

/// What starts a procedure. Mirrors `SopTrigger` (TYPES:115-229).
///
/// Internally tagged on `type` with lowercase variant names. Note that
/// `webhook` and `cron` are the only variants with no `condition` field —
/// writing one is silently ignored upstream, so the adapter reports it.
///
/// Variant order matches upstream's declaration order, which is the order the
/// generated documentation tables use.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SopTrigger {
    /// MQTT message arrival. `+` matches one topic level, `#` the rest.
    Mqtt {
        topic: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        condition: Option<String>,
    },
    /// Inbound HTTP request on a gateway path.
    Webhook { path: String },
    /// Schedule.
    Cron { expression: String },
    /// Hardware signal from a board.
    Peripheral {
        board: String,
        signal: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        condition: Option<String>,
    },
    /// Filesystem change.
    ///
    /// An empty `events` list skips the event-kind check entirely rather than
    /// matching nothing.
    Filesystem {
        path: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        events: Vec<FilesystemEventKind>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        condition: Option<String>,
    },
    /// Calendar event.
    Calendar {
        calendar_source: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        calendar_ids: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        condition: Option<String>,
    },
    /// Message on a configured channel.
    ///
    /// The channel name matches case-insensitively; the alias does not.
    Channel {
        channel: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        alias: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        condition: Option<String>,
    },
    /// Started by hand.
    Manual,
    /// AMQP message. `*` matches one word, `#` zero or more.
    Amqp {
        routing_key: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        condition: Option<String>,
    },
}

impl SopTrigger {
    /// The `type` tag as it appears in TOML.
    pub fn tag(&self) -> &'static str {
        match self {
            Self::Mqtt { .. } => "mqtt",
            Self::Webhook { .. } => "webhook",
            Self::Cron { .. } => "cron",
            Self::Peripheral { .. } => "peripheral",
            Self::Filesystem { .. } => "filesystem",
            Self::Calendar { .. } => "calendar",
            Self::Channel { .. } => "channel",
            Self::Manual => "manual",
            Self::Amqp { .. } => "amqp",
        }
    }

    /// Whether this variant has a `condition` field at all.
    ///
    /// `webhook` and `cron` do not, and a `condition` written on one is
    /// silently dropped upstream — worth a diagnostic rather than silence.
    pub fn supports_condition(&self) -> bool {
        !matches!(self, Self::Webhook { .. } | Self::Cron { .. })
    }
}

// ── Whole procedure ─────────────────────────────────────────────

/// A procedure's canvas coordinate entry. Mirrors `StepPosition`
/// (TYPES:597-603).
///
/// `step` is a step *number*, not an index. All three fields are required.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StepPosition {
    pub step: u32,
    pub x: f64,
    pub y: f64,
}

/// The `[sop]` table. Mirrors `SopMeta` (TYPES:605-633).
///
/// Exactly eleven fields; no others exist. Only `name` and `description` are
/// required.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SopMeta {
    /// Unique procedure name, which also keys its directory on disk.
    pub name: String,
    /// Free text; never executed.
    pub description: String,
    /// Opaque version string. Never parsed as semver upstream.
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default)]
    pub priority: SopPriority,
    /// Absent means "fall back to the daemon default", which Egeria cannot
    /// see — so this stays `None` rather than being resolved at parse time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_mode: Option<SopExecutionMode>,
    /// Minimum seconds between runs; `0` disables.
    #[serde(default)]
    pub cooldown_secs: u64,
    /// Maximum simultaneously executing runs.
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent: u32,
    /// Opt-in deterministic execution. A hard override of `execution_mode`.
    #[serde(default)]
    pub deterministic: bool,
    #[serde(default)]
    pub admission_policy: SopAdmissionPolicy,
    /// Cap on runs parked at an approval; `0` is unlimited.
    #[serde(default)]
    pub max_pending_approvals: u32,
    /// Parent agent alias. Steps inherit it unless they override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
}

fn default_version() -> String {
    "0.1.0".to_string()
}

fn default_max_concurrent() -> u32 {
    1
}

impl SopMeta {
    /// A minimal valid manifest table.
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            version: default_version(),
            priority: SopPriority::default(),
            execution_mode: None,
            cooldown_secs: 0,
            max_concurrent: default_max_concurrent(),
            deterministic: false,
            admission_policy: SopAdmissionPolicy::default(),
            max_pending_approvals: 0,
            agent: None,
        }
    }

    /// The execution mode that actually applies, given the daemon default.
    ///
    /// `deterministic = true` wins unconditionally, ahead of any authored
    /// `execution_mode` (MOD:456-461) — the one ordering worth encoding here,
    /// because reversing it silently disables approval gating.
    pub fn effective_execution_mode(&self, daemon_default: SopExecutionMode) -> SopExecutionMode {
        if self.deterministic {
            SopExecutionMode::Deterministic
        } else {
            self.execution_mode.unwrap_or(daemon_default)
        }
    }
}

/// The whole of `SOP.toml`. Mirrors `SopManifest` (TYPES:582-595).
///
/// `steps` is a fallback used only when `SOP.md` is absent — see
/// [`crate::manifest`] for the precedence, which is decided by the file
/// existing rather than by its content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SopManifest {
    pub sop: SopMeta,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub triggers: Vec<SopTrigger>,
    /// Persisted canvas coordinates, merged onto steps by number at load.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub positions: Vec<StepPosition>,
    /// Steps, used only when `SOP.md` does not exist.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<SopStep>,
}

impl SopManifest {
    /// A manifest with metadata and nothing else.
    pub fn new(meta: SopMeta) -> Self {
        Self {
            sop: meta,
            triggers: Vec::new(),
            positions: Vec::new(),
            steps: Vec::new(),
        }
    }
}

/// A procedure as Egeria holds it: metadata, triggers, and resolved steps.
///
/// This is what a reader produces and a writer consumes. It differs from
/// [`SopManifest`] in that steps have been resolved from whichever source won,
/// and positions have been merged onto them.
#[derive(Debug, Clone, PartialEq)]
pub struct Sop {
    /// The `[sop]` table.
    pub meta: SopMeta,
    /// Triggers, in authored order. Order is preserved and duplicates are
    /// kept, since nothing upstream deduplicates them.
    pub triggers: Vec<SopTrigger>,
    /// Steps, renumbered 1..=N when they came from `SOP.md`.
    pub steps: Vec<SopStep>,
}

impl Sop {
    /// A procedure with metadata and no triggers or steps.
    pub fn new(meta: SopMeta) -> Self {
        Self {
            meta,
            triggers: Vec::new(),
            steps: Vec::new(),
        }
    }

    /// Find a step by its number.
    pub fn step(&self, number: u32) -> Option<&SopStep> {
        self.steps.iter().find(|s| s.number == number)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Enum spellings ──────────────────────────────────────────
    //
    // These pin the exact serialized strings. Upstream has no `alias` on any of
    // them, so a wrong spelling here is a parse failure against real files
    // rather than a graceful degradation.

    #[test]
    fn priority_spellings() {
        for (value, text) in [
            (SopPriority::Low, "low"),
            (SopPriority::Normal, "normal"),
            (SopPriority::High, "high"),
            (SopPriority::Critical, "critical"),
        ] {
            let json = serde_json::to_string(&value).unwrap();
            assert_eq!(json, format!("\"{text}\""));
            let back: SopPriority = serde_json::from_str(&json).unwrap();
            assert_eq!(back, value);
        }
        assert_eq!(SopPriority::default(), SopPriority::Normal);
    }

    #[test]
    fn execution_mode_spellings_are_snake_case() {
        for (value, text) in [
            (SopExecutionMode::Auto, "auto"),
            (SopExecutionMode::Supervised, "supervised"),
            (SopExecutionMode::StepByStep, "step_by_step"),
            (SopExecutionMode::PriorityBased, "priority_based"),
            (SopExecutionMode::Deterministic, "deterministic"),
        ] {
            assert_eq!(
                serde_json::to_string(&value).unwrap(),
                format!("\"{text}\"")
            );
        }
    }

    #[test]
    fn execution_mode_is_case_sensitive() {
        // Upstream's config path lowercases and falls back; the manifest path
        // does neither. Egeria parses manifests, so this must be strict.
        assert!(serde_json::from_str::<SopExecutionMode>("\"Auto\"").is_err());
        assert!(serde_json::from_str::<SopExecutionMode>("\"stepByStep\"").is_err());
        assert!(serde_json::from_str::<SopExecutionMode>("\"auto\"").is_ok());
    }

    #[test]
    fn admission_policy_spellings() {
        for (value, text) in [
            (SopAdmissionPolicy::Parallel, "parallel"),
            (SopAdmissionPolicy::Hold, "hold"),
            (SopAdmissionPolicy::Coalesce, "coalesce"),
            (SopAdmissionPolicy::Drop, "drop"),
        ] {
            assert_eq!(
                serde_json::to_string(&value).unwrap(),
                format!("\"{text}\"")
            );
        }
        assert_eq!(SopAdmissionPolicy::default(), SopAdmissionPolicy::Parallel);
    }

    #[test]
    fn filesystem_event_kinds_are_lowercase_only() {
        assert_eq!(
            serde_json::to_string(&FilesystemEventKind::Created).unwrap(),
            "\"created\""
        );
        assert!(serde_json::from_str::<FilesystemEventKind>("\"created\"").is_ok());
        // FromStr upstream lowercases; serde does not.
        assert!(serde_json::from_str::<FilesystemEventKind>("\"Created\"").is_err());
    }

    #[test]
    fn step_kind_spellings_and_display() {
        assert_eq!(SopStepKind::default(), SopStepKind::Execute);
        assert_eq!(
            serde_json::to_string(&SopStepKind::Capability).unwrap(),
            "\"capability\""
        );
        assert_eq!(SopStepKind::Checkpoint.to_string(), "checkpoint");
    }

    // ── Failure policy ──────────────────────────────────────────

    #[test]
    fn failure_policy_round_trips_all_three_forms() {
        let cases = [
            (StepFailure::Fail, r#""fail""#),
            (StepFailure::Retry { max: 3 }, r#"{"retry":{"max":3}}"#),
            (StepFailure::Goto { step: 4 }, r#"{"goto":{"step":4}}"#),
        ];
        for (value, json) in cases {
            assert_eq!(serde_json::to_string(&value).unwrap(), json);
            let back: StepFailure = serde_json::from_str(json).unwrap();
            assert_eq!(back, value);
        }
        assert!(StepFailure::Fail.is_fail());
        assert!(!StepFailure::Retry { max: 1 }.is_fail());
    }

    #[test]
    fn failure_policy_in_toml() {
        #[derive(Deserialize)]
        struct Holder {
            on_failure: StepFailure,
        }
        let h: Holder = toml::from_str("on_failure = \"fail\"").unwrap();
        assert_eq!(h.on_failure, StepFailure::Fail);

        let h: Holder = toml::from_str("[on_failure.retry]\nmax = 2\n").unwrap();
        assert_eq!(h.on_failure, StepFailure::Retry { max: 2 });

        let h: Holder = toml::from_str("[on_failure.goto]\nstep = 7\n").unwrap();
        assert_eq!(h.on_failure, StepFailure::Goto { step: 7 });
    }

    // ── Routing ─────────────────────────────────────────────────

    #[test]
    fn default_routing_is_recognized_as_default() {
        assert!(StepRouting::default().is_default());
        assert!(
            !StepRouting {
                next: Some(3),
                ..Default::default()
            }
            .is_default()
        );
        assert!(
            !StepRouting {
                terminal: true,
                ..Default::default()
            }
            .is_default()
        );
    }

    #[test]
    fn switch_rules_preserve_order_and_catch_all() {
        let toml_src = r#"
[[switch]]
name = "urgent"
when = "$.sev == \"high\""
goto = 5

[[switch]]
name = "default"
goto = 6
"#;
        #[derive(Deserialize)]
        struct Holder {
            switch: Vec<SwitchRule>,
        }
        let h: Holder = toml::from_str(toml_src).unwrap();
        assert_eq!(h.switch.len(), 2);
        assert_eq!(h.switch[0].name, "urgent");
        assert!(h.switch[0].when.is_some());
        // A port with no guard is the catch-all, and order is significant.
        assert_eq!(h.switch[1].name, "default");
        assert!(h.switch[1].when.is_none());
    }

    // ── Triggers ────────────────────────────────────────────────

    #[test]
    fn every_trigger_variant_round_trips() {
        let variants = vec![
            SopTrigger::Mqtt {
                topic: "facility/+/pressure/#".into(),
                condition: Some("$.value > 85".into()),
            },
            SopTrigger::Webhook {
                path: "/sop/deploy".into(),
            },
            SopTrigger::Cron {
                expression: "0 */5 * * *".into(),
            },
            SopTrigger::Peripheral {
                board: "nucleo-f401re-0".into(),
                signal: "pin_3".into(),
                condition: Some("> 0".into()),
            },
            SopTrigger::Filesystem {
                path: "/var/inbox/**/*.json".into(),
                events: vec![FilesystemEventKind::Created, FilesystemEventKind::Modified],
                condition: None,
            },
            SopTrigger::Calendar {
                calendar_source: "microsoft365".into(),
                calendar_ids: vec!["primary".into(), "team".into()],
                condition: None,
            },
            SopTrigger::Channel {
                channel: "git".into(),
                alias: Some("main".into()),
                condition: Some("$.event_type == \"pull_request.opened\"".into()),
            },
            SopTrigger::Manual,
            SopTrigger::Amqp {
                routing_key: "org.release.*.version.#".into(),
                condition: None,
            },
        ];
        assert_eq!(variants.len(), 9, "upstream defines exactly nine variants");

        for t in &variants {
            let json = serde_json::to_string(t).unwrap();
            let back: SopTrigger = serde_json::from_str(&json).unwrap();
            assert_eq!(&back, t, "round trip failed for {}", t.tag());
            // The discriminator is `type`, lowercase.
            assert!(
                json.contains(&format!(r#""type":"{}""#, t.tag())),
                "missing type tag in {json}"
            );
        }
    }

    #[test]
    fn manual_trigger_is_a_bare_tag() {
        let t: SopTrigger = toml::from_str("type = \"manual\"").unwrap();
        assert_eq!(t, SopTrigger::Manual);
    }

    #[test]
    fn webhook_and_cron_are_the_only_variants_without_conditions() {
        let without: Vec<&str> = [
            SopTrigger::Mqtt {
                topic: "t".into(),
                condition: None,
            },
            SopTrigger::Webhook { path: "/p".into() },
            SopTrigger::Cron {
                expression: "* * * * *".into(),
            },
            SopTrigger::Peripheral {
                board: "b".into(),
                signal: "s".into(),
                condition: None,
            },
            SopTrigger::Filesystem {
                path: "/p".into(),
                events: vec![],
                condition: None,
            },
            SopTrigger::Calendar {
                calendar_source: "c".into(),
                calendar_ids: vec![],
                condition: None,
            },
            SopTrigger::Channel {
                channel: "c".into(),
                alias: None,
                condition: None,
            },
            SopTrigger::Manual,
            SopTrigger::Amqp {
                routing_key: "r".into(),
                condition: None,
            },
        ]
        .iter()
        .filter(|t| !t.supports_condition())
        .map(|t| t.tag())
        .collect();
        assert_eq!(without, vec!["webhook", "cron"]);
    }

    #[test]
    fn filesystem_events_default_to_empty() {
        let t: SopTrigger = toml::from_str("type = \"filesystem\"\npath = \"/tmp\"\n").unwrap();
        match t {
            SopTrigger::Filesystem { events, path, .. } => {
                assert_eq!(path, "/tmp");
                // Empty means "any event kind", not "no event kind".
                assert!(events.is_empty());
            }
            other => panic!("wrong variant: {other:?}"),
        }
    }

    // ── Meta ────────────────────────────────────────────────────

    #[test]
    fn meta_defaults_match_upstream() {
        let m: SopMeta = toml::from_str("name = \"s\"\ndescription = \"d\"\n").unwrap();
        assert_eq!(m.version, "0.1.0");
        assert_eq!(m.priority, SopPriority::Normal);
        // Not Supervised: the manifest field is an Option defaulting to None,
        // and resolution needs a daemon default Egeria cannot see.
        assert_eq!(m.execution_mode, None);
        assert_eq!(m.cooldown_secs, 0);
        assert_eq!(m.max_concurrent, 1);
        assert!(!m.deterministic);
        assert_eq!(m.admission_policy, SopAdmissionPolicy::Parallel);
        assert_eq!(m.max_pending_approvals, 0);
        assert_eq!(m.agent, None);
    }

    #[test]
    fn meta_requires_name_and_description() {
        assert!(toml::from_str::<SopMeta>("description = \"d\"").is_err());
        assert!(toml::from_str::<SopMeta>("name = \"n\"").is_err());
        assert!(toml::from_str::<SopMeta>("name = \"n\"\ndescription = \"\"\n").is_ok());
    }

    #[test]
    fn deterministic_overrides_authored_execution_mode() {
        let mut m = SopMeta::new("s", "d");
        m.execution_mode = Some(SopExecutionMode::Auto);
        m.deterministic = true;
        // The override is unconditional and beats the authored value; getting
        // this backwards silently changes which steps gate.
        assert_eq!(
            m.effective_execution_mode(SopExecutionMode::Supervised),
            SopExecutionMode::Deterministic
        );

        m.deterministic = false;
        assert_eq!(
            m.effective_execution_mode(SopExecutionMode::Supervised),
            SopExecutionMode::Auto
        );

        m.execution_mode = None;
        assert_eq!(
            m.effective_execution_mode(SopExecutionMode::StepByStep),
            SopExecutionMode::StepByStep
        );
    }

    // ── Steps and manifest ──────────────────────────────────────

    #[test]
    fn step_fields_all_default() {
        // Every field is optional upstream, so an empty table must parse.
        let s: SopStep = toml::from_str("").unwrap();
        assert_eq!(s.number, 0);
        assert_eq!(s.kind, SopStepKind::Execute);
        assert!(s.routing.is_default());
        assert!(s.on_failure.is_fail());
        assert!(s.calls.is_empty());
        assert_eq!(s.capability_input, None);
    }

    #[test]
    fn capability_input_is_spelled_with() {
        let s: SopStep = toml::from_str("with = { instruction = \"go\" }").unwrap();
        assert!(s.capability_input.is_some());
        let out = toml::to_string(&s).unwrap();
        assert!(out.contains("with"), "{out}");
        assert!(
            !out.contains("capability_input"),
            "the Rust field name must not leak: {out}"
        );
    }

    #[test]
    fn manifest_parses_with_only_a_sop_table() {
        let m: SopManifest = toml::from_str("[sop]\nname = \"s\"\ndescription = \"d\"\n").unwrap();
        assert_eq!(m.sop.name, "s");
        assert!(m.triggers.is_empty());
        assert!(m.positions.is_empty());
        assert!(m.steps.is_empty());
    }

    #[test]
    fn manifest_requires_the_sop_table() {
        let err = toml::from_str::<SopManifest>("[[triggers]]\ntype = \"manual\"\n").unwrap_err();
        assert!(err.to_string().contains("sop"), "{err}");
    }

    #[test]
    fn positions_require_all_three_fields() {
        #[derive(Deserialize)]
        struct Holder {
            positions: Vec<StepPosition>,
        }
        let h: Holder = toml::from_str("[[positions]]\nstep = 1\nx = 320.5\ny = -48.0\n").unwrap();
        assert_eq!(h.positions[0].step, 1);
        assert!((h.positions[0].y + 48.0).abs() < f64::EPSILON);

        assert!(toml::from_str::<Holder>("[[positions]]\nstep = 1\nx = 0.0\n").is_err());
    }

    #[test]
    fn unknown_keys_are_tolerated() {
        // Upstream has no deny_unknown_fields anywhere in the SOP tree, so a
        // manifest carrying a future field must still load. The adapter reports
        // what it dropped rather than refusing the file.
        let m: SopManifest =
            toml::from_str("[sop]\nname = \"s\"\ndescription = \"d\"\nsome_future_key = 42\n")
                .unwrap();
        assert_eq!(m.sop.name, "s");
    }

    #[test]
    fn whole_manifest_round_trips_through_toml() {
        let mut meta = SopMeta::new("deploy-prod", "Production deploy with approval");
        meta.priority = SopPriority::Critical;
        meta.execution_mode = Some(SopExecutionMode::StepByStep);
        meta.max_concurrent = 2;
        meta.admission_policy = SopAdmissionPolicy::Hold;
        meta.agent = Some("release-bot".into());

        let manifest = SopManifest {
            sop: meta,
            triggers: vec![
                SopTrigger::Manual,
                SopTrigger::Channel {
                    channel: "git".into(),
                    alias: Some("main".into()),
                    condition: Some("$.event_type == \"pull_request.opened\"".into()),
                },
            ],
            positions: vec![StepPosition {
                step: 1,
                x: 320.5,
                y: -48.0,
            }],
            steps: Vec::new(),
        };

        let text = toml::to_string_pretty(&manifest).unwrap();
        let back: SopManifest = toml::from_str(&text).unwrap();
        assert_eq!(back, manifest, "round trip changed the manifest:\n{text}");
    }

    #[test]
    fn sop_finds_steps_by_number() {
        let mut sop = Sop::new(SopMeta::new("s", "d"));
        sop.steps = vec![
            SopStep {
                number: 1,
                title: "First".into(),
                ..Default::default()
            },
            SopStep {
                number: 2,
                title: "Second".into(),
                ..Default::default()
            },
        ];
        assert_eq!(sop.step(2).unwrap().title, "Second");
        assert!(sop.step(3).is_none());
    }
}
