//! Reading and writing `SOP.toml`.
//!
//! Upstream uses no `deny_unknown_fields` anywhere in the SOP tree, so an
//! unrecognized key is accepted and silently discarded. Egeria keeps the
//! tolerance — refusing a file because it carries a field from a newer ZeroClaw
//! would make the adapter useless the first time upstream adds one — but not
//! the silence. Serde alone cannot tell us what it ignored, so parsing walks
//! the raw TOML alongside the typed result and reports the difference.

use toml::Value;

use crate::diagnostic::{Diagnostic, DiagnosticKind, Location};
use crate::error::{ReadError, WriteError};
use crate::model::SopManifest;

/// The keys `[sop]` accepts. Mirrors `SopMeta` (TYPES:605-633).
const SOP_KEYS: &[&str] = &[
    "name",
    "description",
    "version",
    "priority",
    "execution_mode",
    "cooldown_secs",
    "max_concurrent",
    "deterministic",
    "admission_policy",
    "max_pending_approvals",
    "agent",
];

/// The keys a `[[positions]]` entry accepts.
const POSITION_KEYS: &[&str] = &["step", "x", "y"];

/// The keys a `[[steps]]` entry accepts. Mirrors `SopStep` (TYPES:308-392).
///
/// `with` rather than `capability_input`: the Rust field is renamed, and the
/// TOML spelling is what a manifest carries.
const STEP_KEYS: &[&str] = &[
    "number",
    "title",
    "body",
    "suggested_tools",
    "requires_confirmation",
    "kind",
    "schema",
    "scope",
    "routing",
    "on_failure",
    "mode",
    "calls",
    "pos",
    "agent",
    "capability",
    "with",
    "policy",
    "gate_prompt",
    "edit",
];

/// The top-level keys the manifest accepts.
const TOP_KEYS: &[&str] = &["sop", "triggers", "positions", "steps"];

/// The keys each trigger variant accepts, keyed by its `type` tag.
///
/// `webhook` and `cron` deliberately have no `condition`: upstream drops one
/// written there, which is worth saying out loud rather than reproducing
/// silently.
fn trigger_keys(tag: &str) -> Option<&'static [&'static str]> {
    Some(match tag {
        "mqtt" => &["type", "topic", "condition"],
        "webhook" => &["type", "path"],
        "cron" => &["type", "expression"],
        "peripheral" => &["type", "board", "signal", "condition"],
        "filesystem" => &["type", "path", "events", "condition"],
        "calendar" => &["type", "calendar_source", "calendar_ids", "condition"],
        "channel" => &["type", "channel", "alias", "condition"],
        "manual" => &["type"],
        "amqp" => &["type", "routing_key", "condition"],
        _ => return None,
    })
}

/// Parse `SOP.toml`.
///
/// Returns the manifest and everything noteworthy that did not prevent
/// producing it. A malformed document or a missing required field is an error;
/// an unrecognized key is a diagnostic.
pub fn parse_manifest(text: &str) -> Result<(SopManifest, Vec<Diagnostic>), ReadError> {
    // Parse once as a generic document to see the keys as written, and once
    // into the typed shape. The typed parse is the source of truth for values;
    // the raw parse is the only way to learn what serde discarded.
    let raw: Value = toml::from_str(text).map_err(|source| ReadError::ManifestSyntax { source })?;
    let manifest: SopManifest =
        toml::from_str(text).map_err(|source| ReadError::ManifestSyntax { source })?;

    let mut diagnostics = Vec::new();
    collect_unknown_keys(&raw, &mut diagnostics);
    Ok((manifest, diagnostics))
}

/// Serialize a manifest to canonical `SOP.toml` text.
pub fn write_manifest(manifest: &SopManifest) -> Result<String, WriteError> {
    toml::to_string_pretty(manifest).map_err(|source| WriteError::ManifestSerialize { source })
}

fn unknown(key_path: String, key: String) -> Diagnostic {
    Diagnostic::new(
        DiagnosticKind::UnknownManifestKey { key },
        Location::Manifest { key_path },
    )
}

fn collect_unknown_keys(raw: &Value, out: &mut Vec<Diagnostic>) {
    let Some(table) = raw.as_table() else {
        return;
    };

    for (key, value) in table {
        if !TOP_KEYS.contains(&key.as_str()) {
            out.push(unknown(key.clone(), key.clone()));
            continue;
        }
        match key.as_str() {
            "sop" => check_table(value, SOP_KEYS, "sop", out),
            "positions" => check_array(value, POSITION_KEYS, "positions", out),
            "steps" => check_array(value, STEP_KEYS, "steps", out),
            "triggers" => check_triggers(value, out),
            _ => unreachable!("key was matched against TOP_KEYS above"),
        }
    }
}

fn check_table(value: &Value, known: &[&str], prefix: &str, out: &mut Vec<Diagnostic>) {
    let Some(table) = value.as_table() else {
        return;
    };
    for key in table.keys() {
        if !known.contains(&key.as_str()) {
            out.push(unknown(format!("{prefix}.{key}"), key.clone()));
        }
    }
}

fn check_array(value: &Value, known: &[&str], prefix: &str, out: &mut Vec<Diagnostic>) {
    let Some(items) = value.as_array() else {
        return;
    };
    for (index, item) in items.iter().enumerate() {
        check_table(item, known, &format!("{prefix}[{index}]"), out);
    }
}

fn check_triggers(value: &Value, out: &mut Vec<Diagnostic>) {
    let Some(items) = value.as_array() else {
        return;
    };
    for (index, item) in items.iter().enumerate() {
        let prefix = format!("triggers[{index}]");
        let Some(table) = item.as_table() else {
            continue;
        };
        // An absent or unknown tag is a typed-parse error, which has already
        // been raised by the time we get here.
        let Some(tag) = table.get("type").and_then(Value::as_str) else {
            continue;
        };
        let Some(known) = trigger_keys(tag) else {
            continue;
        };
        for key in table.keys() {
            if !known.contains(&key.as_str()) {
                out.push(unknown(format!("{prefix}.{key}"), key.clone()));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        SopAdmissionPolicy, SopExecutionMode, SopMeta, SopPriority, SopTrigger, StepPosition,
    };

    const MINIMAL: &str = "[sop]\nname = \"s\"\ndescription = \"d\"\n";

    #[test]
    fn minimal_manifest_parses_without_diagnostics() {
        let (m, diags) = parse_manifest(MINIMAL).unwrap();
        assert_eq!(m.sop.name, "s");
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn maximal_manifest_parses_every_field() {
        let text = r#"
[sop]
name = "deploy-prod"
description = "Production deploy with approval"
version = "1.4.2"
priority = "critical"
execution_mode = "step_by_step"
cooldown_secs = 300
max_concurrent = 2
deterministic = false
admission_policy = "hold"
max_pending_approvals = 8
agent = "release-bot"
"#;
        let (m, diags) = parse_manifest(text).unwrap();
        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(m.sop.version, "1.4.2");
        assert_eq!(m.sop.priority, SopPriority::Critical);
        assert_eq!(m.sop.execution_mode, Some(SopExecutionMode::StepByStep));
        assert_eq!(m.sop.cooldown_secs, 300);
        assert_eq!(m.sop.max_concurrent, 2);
        assert_eq!(m.sop.admission_policy, SopAdmissionPolicy::Hold);
        assert_eq!(m.sop.max_pending_approvals, 8);
        assert_eq!(m.sop.agent.as_deref(), Some("release-bot"));
    }

    #[test]
    fn all_nine_trigger_variants_parse_from_toml() {
        let text = r#"
[sop]
name = "everything"
description = "every trigger"

[[triggers]]
type = "mqtt"
topic = "facility/+/pressure/#"
condition = "$.value > 85"

[[triggers]]
type = "webhook"
path = "/sop/everything"

[[triggers]]
type = "cron"
expression = "0 */5 * * *"

[[triggers]]
type = "peripheral"
board = "nucleo-f401re-0"
signal = "pin_3"
condition = "> 0"

[[triggers]]
type = "filesystem"
path = "/var/inbox/**/*.json"
events = ["created", "modified"]

[[triggers]]
type = "calendar"
calendar_source = "microsoft365"
calendar_ids = ["primary", "team"]

[[triggers]]
type = "channel"
channel = "git"
alias = "main"
condition = "$.event_type == \"pull_request.opened\""

[[triggers]]
type = "manual"

[[triggers]]
type = "amqp"
routing_key = "org.release.*.version.#"
"#;
        let (m, diags) = parse_manifest(text).unwrap();
        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(m.triggers.len(), 9);
        let tags: Vec<_> = m.triggers.iter().map(SopTrigger::tag).collect();
        assert_eq!(
            tags,
            vec![
                "mqtt",
                "webhook",
                "cron",
                "peripheral",
                "filesystem",
                "calendar",
                "channel",
                "manual",
                "amqp"
            ]
        );
    }

    #[test]
    fn trigger_order_is_preserved_and_duplicates_kept() {
        let text = r#"
[sop]
name = "s"
description = "d"

[[triggers]]
type = "manual"

[[triggers]]
type = "manual"
"#;
        let (m, _) = parse_manifest(text).unwrap();
        // Nothing upstream deduplicates triggers, so neither does this.
        assert_eq!(m.triggers.len(), 2);
    }

    #[test]
    fn unknown_top_level_key_is_a_diagnostic_not_an_error() {
        let text = "[sop]\nname = \"s\"\ndescription = \"d\"\n\n[wobble]\nx = 1\n";
        let (m, diags) = parse_manifest(text).unwrap();
        assert_eq!(m.sop.name, "s");
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].kind.code(), "unknown_manifest_key");
        assert!(diags[0].message.contains("wobble"), "{:?}", diags[0]);
    }

    #[test]
    fn unknown_sop_key_is_reported_with_its_path() {
        let text = "[sop]\nname = \"s\"\ndescription = \"d\"\nsome_future_key = 42\n";
        let (_, diags) = parse_manifest(text).unwrap();
        assert_eq!(diags.len(), 1);
        match &diags[0].location {
            Location::Manifest { key_path } => assert_eq!(key_path, "sop.some_future_key"),
            other => panic!("wrong location: {other:?}"),
        }
    }

    #[test]
    fn condition_on_webhook_is_reported() {
        // Upstream drops this silently — webhook has no condition field — so a
        // reader who wrote one has a false expectation worth surfacing.
        let text = r#"
[sop]
name = "s"
description = "d"

[[triggers]]
type = "webhook"
path = "/p"
condition = "$.x > 1"
"#;
        let (_, diags) = parse_manifest(text).unwrap();
        assert_eq!(diags.len(), 1, "{diags:?}");
        match &diags[0].location {
            Location::Manifest { key_path } => assert_eq!(key_path, "triggers[0].condition"),
            other => panic!("wrong location: {other:?}"),
        }
    }

    #[test]
    fn condition_on_cron_is_reported() {
        let text = "[sop]\nname = \"s\"\ndescription = \"d\"\n\n[[triggers]]\ntype = \"cron\"\nexpression = \"* * * * *\"\ncondition = \"$.x\"\n";
        let (_, diags) = parse_manifest(text).unwrap();
        assert_eq!(diags.len(), 1, "{diags:?}");
    }

    #[test]
    fn condition_on_mqtt_is_not_reported() {
        let text = "[sop]\nname = \"s\"\ndescription = \"d\"\n\n[[triggers]]\ntype = \"mqtt\"\ntopic = \"t\"\ncondition = \"$.x > 1\"\n";
        let (_, diags) = parse_manifest(text).unwrap();
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn unknown_keys_in_positions_and_steps_are_reported() {
        let text = r#"
[sop]
name = "s"
description = "d"

[[positions]]
step = 1
x = 0.0
y = 0.0
z = 3.0

[[steps]]
number = 1
title = "t"
unexpected = true
"#;
        let (_, diags) = parse_manifest(text).unwrap();
        let paths: Vec<String> = diags
            .iter()
            .map(|d| match &d.location {
                Location::Manifest { key_path } => key_path.clone(),
                other => panic!("wrong location: {other:?}"),
            })
            .collect();
        assert!(paths.contains(&"positions[0].z".to_string()), "{paths:?}");
        assert!(
            paths.contains(&"steps[0].unexpected".to_string()),
            "{paths:?}"
        );
    }

    #[test]
    fn missing_required_field_is_an_error() {
        let err = parse_manifest("[sop]\ndescription = \"d\"\n").unwrap_err();
        assert!(matches!(err, ReadError::ManifestSyntax { .. }), "{err:?}");
        assert!(err.to_string().contains("name"), "{err}");
    }

    #[test]
    fn missing_sop_table_is_an_error() {
        let err = parse_manifest("[[triggers]]\ntype = \"manual\"\n").unwrap_err();
        assert!(err.to_string().contains("sop"), "{err}");
    }

    #[test]
    fn malformed_toml_is_a_syntax_error() {
        let err = parse_manifest("[sop\nname =").unwrap_err();
        assert!(matches!(err, ReadError::ManifestSyntax { .. }), "{err:?}");
    }

    #[test]
    fn bad_enum_value_is_an_error_not_a_fallback() {
        // The config path lowercases and falls back to Supervised; the manifest
        // path must not. A typo has to be loud.
        let err =
            parse_manifest("[sop]\nname = \"s\"\ndescription = \"d\"\npriority = \"urgent\"\n")
                .unwrap_err();
        assert!(matches!(err, ReadError::ManifestSyntax { .. }), "{err:?}");

        let err =
            parse_manifest("[sop]\nname = \"s\"\ndescription = \"d\"\nexecution_mode = \"Auto\"\n")
                .unwrap_err();
        assert!(matches!(err, ReadError::ManifestSyntax { .. }), "{err:?}");
    }

    #[test]
    fn writing_then_parsing_returns_an_equal_manifest() {
        let mut meta = SopMeta::new("deploy-prod", "Production deploy");
        meta.priority = SopPriority::High;
        meta.admission_policy = SopAdmissionPolicy::Coalesce;
        meta.agent = Some("release-bot".into());

        let manifest = SopManifest {
            sop: meta,
            triggers: vec![
                SopTrigger::Manual,
                SopTrigger::Mqtt {
                    topic: "sensors/#".into(),
                    condition: Some("$.value > 85".into()),
                },
            ],
            positions: vec![StepPosition {
                step: 1,
                x: 320.5,
                y: -48.0,
            }],
            steps: Vec::new(),
        };

        let text = write_manifest(&manifest).unwrap();
        let (back, diags) = parse_manifest(&text).unwrap();
        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(back, manifest, "round trip changed the manifest:\n{text}");
    }

    #[test]
    fn writing_is_canonical() {
        let manifest = SopManifest::new(SopMeta::new("s", "d"));
        let a = write_manifest(&manifest).unwrap();
        let b = write_manifest(&manifest).unwrap();
        assert_eq!(a, b);
        // Empty collections are omitted rather than written as empty arrays.
        assert!(!a.contains("triggers"), "{a}");
        assert!(!a.contains("positions"), "{a}");
        assert!(!a.contains("steps"), "{a}");
    }

    #[test]
    fn every_sop_key_constant_is_actually_accepted() {
        // Guards against the key list drifting from the struct: each listed key
        // must parse, so a rename upstream shows up here rather than as a
        // spurious "unknown key" diagnostic for a field we do model.
        let values = [
            ("name", "\"s\""),
            ("description", "\"d\""),
            ("version", "\"1.0\""),
            ("priority", "\"high\""),
            ("execution_mode", "\"auto\""),
            ("cooldown_secs", "5"),
            ("max_concurrent", "3"),
            ("deterministic", "true"),
            ("admission_policy", "\"drop\""),
            ("max_pending_approvals", "2"),
            ("agent", "\"a\""),
        ];
        assert_eq!(values.len(), SOP_KEYS.len(), "key list and test disagree");
        for (key, _) in &values {
            assert!(SOP_KEYS.contains(key), "{key} missing from SOP_KEYS");
        }
        let body: String = values.iter().map(|(k, v)| format!("{k} = {v}\n")).collect();
        let (_, diags) = parse_manifest(&format!("[sop]\n{body}")).unwrap();
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn every_trigger_variant_has_a_key_list() {
        for tag in [
            "mqtt",
            "webhook",
            "cron",
            "peripheral",
            "filesystem",
            "calendar",
            "channel",
            "manual",
            "amqp",
        ] {
            let keys = trigger_keys(tag).unwrap_or_else(|| panic!("no key list for {tag}"));
            assert!(keys.contains(&"type"), "{tag} must accept its own tag");
        }
        assert!(trigger_keys("telepathy").is_none());
    }
}
