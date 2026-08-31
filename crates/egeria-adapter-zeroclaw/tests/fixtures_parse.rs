//! Every fixture in the corpus parses.
//!
//! Fixtures are discovered from the filesystem rather than listed here, so
//! adding a directory under `fixtures/sops/` is enough to bring it under test.
//! That is deliberate: a corpus you have to register in two places is a corpus
//! that quietly grows holes.

use std::path::{Path, PathBuf};

use egeria_adapter_zeroclaw::read_sop;

/// The corpus root, resolved relative to this crate rather than the working
/// directory, so the tests run the same from anywhere.
fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/sops")
        .canonicalize()
        .expect("fixtures/sops must exist")
}

/// Every fixture directory, sorted, so failures report in a stable order.
fn fixtures() -> Vec<(String, PathBuf)> {
    let mut found: Vec<(String, PathBuf)> = std::fs::read_dir(corpus_root())
        .expect("corpus is readable")
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .map(|entry| {
            (
                entry.file_name().to_string_lossy().into_owned(),
                entry.path(),
            )
        })
        .collect();
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found
}

/// Fixtures that exist to trip a verifier rule rather than to be clean.
///
/// They still have to *parse* — a fixture the parser cannot read tests nothing.
/// What makes them special is that later milestones expect findings from them,
/// so `INDEX.md` documents which rule each is aimed at.
const INTENTIONALLY_DIAGNOSTIC: &[&str] = &["switch-shadowed", "goto-cycle", "schema-mismatch"];

#[test]
fn the_corpus_is_not_empty() {
    let found = fixtures();
    assert!(
        found.len() >= 25,
        "expected at least 25 fixtures, found {}: {:?}",
        found.len(),
        found.iter().map(|(n, _)| n).collect::<Vec<_>>()
    );
}

#[test]
fn every_fixture_parses() {
    let mut failures = Vec::new();
    for (name, path) in fixtures() {
        match read_sop(&path) {
            Ok((sop, diagnostics)) => {
                if sop.steps.is_empty() && !name.starts_with("no-steps") {
                    failures.push(format!("{name}: parsed to zero steps"));
                }
                // Diagnostics are allowed — several fixtures exist to produce
                // them — but a parse must never fail.
                let _ = diagnostics;
            }
            Err(err) => failures.push(format!("{name}: {err}")),
        }
    }
    assert!(
        failures.is_empty(),
        "fixtures failed to parse:\n{}",
        failures.join("\n")
    );
}

#[test]
fn every_fixture_has_both_files() {
    for (name, path) in fixtures() {
        assert!(
            path.join("SOP.toml").is_file(),
            "{name} is missing SOP.toml"
        );
        assert!(path.join("SOP.md").is_file(), "{name} is missing SOP.md");
    }
}

#[test]
fn the_index_lists_every_fixture() {
    let index_path = corpus_root().join("INDEX.md");
    let index = std::fs::read_to_string(&index_path).expect("fixtures/sops/INDEX.md must exist");
    let mut missing = Vec::new();
    for (name, _) in fixtures() {
        if !index.contains(&format!("`{name}`")) {
            missing.push(name);
        }
    }
    assert!(
        missing.is_empty(),
        "these fixtures are not listed in INDEX.md: {missing:?}"
    );
}

#[test]
fn intentionally_diagnostic_fixtures_are_documented_as_such() {
    let index = std::fs::read_to_string(corpus_root().join("INDEX.md")).expect("INDEX.md");
    for name in INTENTIONALLY_DIAGNOSTIC {
        let (_, path) = fixtures()
            .into_iter()
            .find(|(n, _)| n == name)
            .unwrap_or_else(|| panic!("{name} must exist in the corpus"));
        assert!(path.is_dir());
        assert!(
            index.contains(&format!("`{name}`")),
            "{name} must be listed in INDEX.md"
        );
    }
    // The index has to say somewhere that these are deliberate, or a later
    // reader will "fix" them.
    assert!(
        index.to_lowercase().contains("deliberate") || index.to_lowercase().contains("intentional"),
        "INDEX.md must explain that some fixtures are deliberately broken"
    );
}

/// Every accepted bullet spelling, including the four asymmetric aliases.
const BULLET_KEYS: &[&str] = &[
    "tools",
    "allow-tools",
    "allow_tools",
    "deny-tools",
    "deny_tools",
    "requires_confirmation",
    "kind",
    "capability",
    "with",
    "input",
    "output",
    "when",
    "next",
    "terminal",
    "depends_on",
    "depends-on",
    "switch",
    "on_failure",
    "on-failure",
    "mode",
    "agent",
    "call",
    "prompt",
    "policy",
    "edit",
];

const TRIGGER_VARIANTS: &[&str] = &[
    "mqtt",
    "webhook",
    "cron",
    "peripheral",
    "filesystem",
    "calendar",
    "channel",
    "manual",
    "amqp",
];

/// Read every fixture's `SOP.md` and `SOP.toml` as raw text.
fn corpus_text() -> (String, String) {
    let mut markdown = String::new();
    let mut manifests = String::new();
    for (_, path) in fixtures() {
        markdown.push_str(&std::fs::read_to_string(path.join("SOP.md")).unwrap_or_default());
        markdown.push('\n');
        manifests.push_str(&std::fs::read_to_string(path.join("SOP.toml")).unwrap_or_default());
        manifests.push('\n');
    }
    (markdown, manifests)
}

#[test]
fn the_corpus_exercises_every_bullet_spelling() {
    // A spelling nothing uses is a spelling nothing tests — and the four
    // aliases are asymmetric upstream, so they cannot be assumed to follow from
    // their canonical forms.
    let (markdown, _) = corpus_text();
    let mut missing = Vec::new();
    for key in BULLET_KEYS {
        let used = markdown.lines().any(|line| {
            let trimmed = line.trim();
            trimmed
                .strip_prefix("- ")
                .map(|b| {
                    b.trim_start_matches("- ")
                        .trim()
                        .starts_with(&format!("{key}:"))
                })
                .unwrap_or(false)
        });
        if !used {
            missing.push(*key);
        }
    }
    assert!(
        missing.is_empty(),
        "no fixture exercises these bullet spellings: {missing:?}"
    );
}

#[test]
fn the_corpus_exercises_every_trigger_variant() {
    let (_, manifests) = corpus_text();
    let missing: Vec<_> = TRIGGER_VARIANTS
        .iter()
        .filter(|t| !manifests.contains(&format!("type = \"{t}\"")))
        .collect();
    assert!(
        missing.is_empty(),
        "no fixture uses these trigger variants: {missing:?}"
    );
}

#[test]
fn the_corpus_exercises_every_step_kind_and_failure_policy() {
    let (markdown, _) = corpus_text();
    for kind in ["checkpoint", "capability"] {
        assert!(
            markdown.contains(&format!("- kind: {kind}")),
            "no fixture uses `kind: {kind}`"
        );
    }
    for policy in ["fail", "retry", "goto"] {
        assert!(
            markdown.contains(&format!("failure: {policy}")),
            "no fixture uses `on_failure: {policy}`"
        );
    }
}

#[test]
fn the_index_documents_every_covered_construct() {
    // The index is maintained by hand; this keeps its coverage tables honest.
    let index = std::fs::read_to_string(corpus_root().join("INDEX.md")).expect("INDEX.md");
    let mut missing = Vec::new();
    for key in BULLET_KEYS {
        if !index.contains(&format!("`{key}:`")) {
            missing.push(format!("bullet `{key}:`"));
        }
    }
    for trigger in TRIGGER_VARIANTS {
        if !index.contains(&format!("`{trigger}`")) {
            missing.push(format!("trigger `{trigger}`"));
        }
    }
    assert!(
        missing.is_empty(),
        "INDEX.md does not document: {missing:?}"
    );
}

#[test]
fn every_fixture_round_trips() {
    // The corpus is the input to the round-trip conformance suite in #5, so a
    // fixture that cannot survive a write/read cycle would poison it.
    let mut failures = Vec::new();
    for (name, path) in fixtures() {
        let Ok((sop, _)) = read_sop(&path) else {
            continue; // already reported by every_fixture_parses
        };
        let dir = tempfile::tempdir().expect("tempdir");
        match egeria_adapter_zeroclaw::write_sop(dir.path(), &sop) {
            Ok(lossy) if !lossy.is_empty() => {
                failures.push(format!("{name}: lossy write: {lossy:?}"));
            }
            Err(err) => failures.push(format!("{name}: write failed: {err}")),
            Ok(_) => match read_sop(dir.path()) {
                Ok((back, _)) if back != sop => {
                    failures.push(format!("{name}: round trip changed the SOP"));
                }
                Err(err) => failures.push(format!("{name}: reread failed: {err}")),
                Ok(_) => {}
            },
        }
    }
    assert!(
        failures.is_empty(),
        "fixtures failed to round trip:\n{}",
        failures.join("\n")
    );
}
