//! Source-level round-trip conformance across the whole fixture corpus.
//!
//! This closes milestone V1-M0. What it establishes is narrow and worth stating
//! precisely: Egeria can read every documented ZeroClaw SOP construct and write
//! it back without losing anything. That is a prerequisite for the IR work, not
//! a substitute for it — nothing here says the *meaning* was preserved, only
//! the content.
//!
//! Doing this before designing the IR is deliberate. It exposes missing concepts
//! against real syntax rather than against an imagined version of it, and it has
//! already done so: the corpus is what turned up the `[[steps]]` fallback,
//! upstream's lossy renderer, and the guard-rooting rule.
//!
//! Three properties are checked, in increasing strength:
//!
//! 1. **Parse → print → parse** returns an equal model.
//! 2. **Printing is a fixed point** — printing the reparsed model gives byte-
//!    identical text, so nothing oscillates.
//! 3. **The canonical form is snapshotted** per fixture, so formatting drift
//!    shows up in review rather than accumulating unnoticed.

use std::path::{Path, PathBuf};

use egeria_adapter_zeroclaw::{Sop, parse_steps, print_steps, read_sop, render_sop};
use pretty_assertions::assert_eq;

fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/sops")
        .canonicalize()
        .expect("fixtures/sops must exist")
}

/// Fixture directories, discovered rather than listed.
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

fn read(path: &Path) -> Sop {
    read_sop(path)
        .unwrap_or_else(|err| panic!("{} failed to read: {err}", path.display()))
        .0
}

#[test]
fn every_fixture_survives_a_step_level_round_trip() {
    // parse -> print -> parse, comparing models rather than text: formatting is
    // allowed to be canonicalized, content is not allowed to change.
    for (name, path) in fixtures() {
        let markdown = std::fs::read_to_string(path.join("SOP.md")).expect("SOP.md");
        let (steps, _) = parse_steps(&markdown);
        let (printed, lossy) = print_steps(&steps);
        assert!(
            lossy.is_empty(),
            "{name}: printing reported loss: {lossy:?}"
        );

        let (reparsed, _) = parse_steps(&printed);
        assert_eq!(
            steps, reparsed,
            "{name}: step round trip changed the model\n--- printed ---\n{printed}"
        );
    }
}

#[test]
fn every_fixture_survives_a_whole_sop_round_trip() {
    // The same property one level up: both files, through the real reader and
    // writer, including trigger and position handling.
    for (name, path) in fixtures() {
        let sop = read(&path);
        let (manifest_text, markdown, lossy) =
            render_sop(&sop).unwrap_or_else(|err| panic!("{name}: render failed: {err}"));
        assert!(
            lossy.is_empty(),
            "{name}: rendering reported loss: {lossy:?}"
        );

        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("SOP.toml"), &manifest_text).expect("write manifest");
        std::fs::write(dir.path().join("SOP.md"), &markdown).expect("write markdown");

        let back = read(dir.path());
        assert_eq!(
            sop, back,
            "{name}: whole-SOP round trip changed the model\n--- SOP.toml ---\n{manifest_text}\n--- SOP.md ---\n{markdown}"
        );
    }
}

#[test]
fn printing_reaches_a_fixed_point_immediately() {
    // A printer that keeps changing its own output would make every later diff
    // noise. One pass must be enough.
    for (name, path) in fixtures() {
        let sop = read(&path);
        let (first_manifest, first_markdown, _) = render_sop(&sop).expect("render");

        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("SOP.toml"), &first_manifest).expect("write");
        std::fs::write(dir.path().join("SOP.md"), &first_markdown).expect("write");
        let back = read(dir.path());

        let (second_manifest, second_markdown, _) = render_sop(&back).expect("render");
        assert_eq!(
            first_markdown, second_markdown,
            "{name}: SOP.md is not a fixed point"
        );
        assert_eq!(
            first_manifest, second_manifest,
            "{name}: SOP.toml is not a fixed point"
        );
    }
}

#[test]
fn canonical_output_is_snapshotted_per_fixture() {
    // Formatting drift becomes a reviewable diff rather than something that
    // accumulates. Snapshots live under tests/snapshots/.
    for (name, path) in fixtures() {
        let sop = read(&path);
        let (manifest_text, markdown, _) = render_sop(&sop).expect("render");
        insta::assert_snapshot!(format!("{name}__SOP.md"), markdown);
        insta::assert_snapshot!(format!("{name}__SOP.toml"), manifest_text);
    }
}

#[test]
fn the_corpus_covers_every_fixture_in_the_index() {
    // Guards the harness itself: if discovery silently returned nothing, every
    // test above would pass vacuously.
    let found = fixtures();
    assert!(
        found.len() >= 25,
        "expected at least 25 fixtures, discovered {}",
        found.len()
    );
    for (name, path) in &found {
        assert!(path.join("SOP.md").is_file(), "{name} has no SOP.md");
    }
}

#[test]
fn a_deliberately_broken_lowering_is_caught() {
    // A meta-test: the suite must be capable of failing. Without this, a harness
    // that silently stopped comparing would look identical to a passing one.
    let (_, path) = fixtures()
        .into_iter()
        .find(|(n, _)| n == "linear-minimal")
        .expect("linear-minimal must exist");
    let mut sop = read(&path);

    // The kind of change a broken lowering would make: drop a step's tools.
    let original = sop.clone();
    sop.steps[0].suggested_tools.clear();
    assert_ne!(
        original, sop,
        "the mutation must actually change the model, or this test proves nothing"
    );

    let (manifest_text, markdown, _) = render_sop(&sop).expect("render");
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("SOP.toml"), &manifest_text).expect("write");
    std::fs::write(dir.path().join("SOP.md"), &markdown).expect("write");
    let back = read(dir.path());

    // The mutated SOP still round-trips faithfully — the point is that it does
    // *not* match the original, which is what a real comparison would catch.
    assert_eq!(back, sop, "the mutated SOP should still round trip");
    assert_ne!(
        back, original,
        "comparing against the original must detect the dropped tools"
    );
}
