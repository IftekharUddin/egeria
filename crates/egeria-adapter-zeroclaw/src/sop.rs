//! Reading and writing a SOP directory.
//!
//! A SOP is a directory holding `SOP.toml` and `SOP.md`. The split is not by
//! topic but by authority: the manifest carries metadata, triggers, and canvas
//! positions, while the markdown carries the steps — and when both claim to
//! carry steps, the markdown wins.
//!
//! Two upstream behaviors govern that precedence and are reproduced here:
//!
//! * `SOP.md` wins whenever the **file exists**, tested by existence rather
//!   than content (MOD:428-435). A `SOP.md` that parses to zero steps still
//!   beats a populated `[[steps]]` table.
//! * Positions are merged onto steps **by step number** (MOD:437-441), after
//!   parsing. Since markdown steps are renumbered positionally, positions
//!   written against author-chosen numbers attach to the wrong step or to
//!   none — silently, upstream.
//!
//! Egeria never *writes* `[[steps]]`. Upstream does, which puts the step list
//! in both files with the markdown authoritative on reload, so the two can
//! drift apart with nothing to notice. Since the printer here is genuinely
//! lossless, the markdown alone is sufficient.

use std::path::{Path, PathBuf};

use crate::diagnostic::{Diagnostic, DiagnosticKind, Location};
use crate::error::{ReadError, WriteError};
use crate::manifest::{parse_manifest, write_manifest};
use crate::model::{Sop, SopManifest, SopStep, StepPos, StepPosition};
use crate::steps::{parse_steps, print_steps};

/// The manifest file inside a SOP directory.
pub const MANIFEST_FILE: &str = "SOP.toml";
/// The step-list file inside a SOP directory.
pub const STEPS_FILE: &str = "SOP.md";

/// Read a SOP directory.
pub fn read_sop(dir: impl AsRef<Path>) -> Result<(Sop, Vec<Diagnostic>), ReadError> {
    let dir = dir.as_ref();
    let manifest_path = dir.join(MANIFEST_FILE);
    let steps_path = dir.join(STEPS_FILE);

    if !manifest_path.exists() {
        return Err(ReadError::NotASopDirectory {
            path: dir.to_path_buf(),
            missing: MANIFEST_FILE,
        });
    }
    let manifest_text = std::fs::read_to_string(&manifest_path)
        .map_err(|source| ReadError::io(manifest_path.clone(), source))?;
    let markdown = if steps_path.exists() {
        Some(
            std::fs::read_to_string(&steps_path)
                .map_err(|source| ReadError::io(steps_path.clone(), source))?,
        )
    } else {
        None
    };

    let (sop, diagnostics) = read_sop_str(&manifest_text, markdown.as_deref())?;
    Ok((sop, diagnostics))
}

/// Read a SOP from the text of its two files.
///
/// `markdown` is `None` when `SOP.md` does not exist, which is the only case
/// where the manifest's `[[steps]]` table is consulted.
pub fn read_sop_str(
    manifest_text: &str,
    markdown: Option<&str>,
) -> Result<(Sop, Vec<Diagnostic>), ReadError> {
    let (manifest, mut diagnostics) = parse_manifest(manifest_text)?;

    let mut steps = match markdown {
        Some(md) => {
            let (steps, step_diagnostics) = parse_steps(md);
            diagnostics.extend(step_diagnostics);
            if !manifest.steps.is_empty() {
                // The two can disagree with nothing upstream to notice, since
                // the markdown silently wins on every load.
                diagnostics.push(Diagnostic::new(
                    DiagnosticKind::LossyConstruct {
                        construct: "steps".into(),
                        detail: format!(
                            "SOP.toml declares {} step(s) and SOP.md declares {}; \
                             SOP.md wins and the manifest steps are discarded",
                            manifest.steps.len(),
                            steps.len()
                        ),
                    },
                    Location::Manifest {
                        key_path: "steps".into(),
                    },
                ));
            }
            steps
        }
        None => normalize_manifest_steps(manifest.steps.clone()),
    };

    merge_positions(&mut steps, &manifest.positions, &mut diagnostics);

    Ok((
        Sop {
            meta: manifest.sop,
            triggers: manifest.triggers,
            steps,
        },
        diagnostics,
    ))
}

/// Fill in numbers and titles for steps that came from `[[steps]]`.
///
/// Deliberately unlike the markdown path: a number is filled only when it is
/// zero, so authored numbers survive and may be duplicated or non-contiguous
/// (MOD:483-496). An empty title falls back to the capability name, then to the
/// step kind.
fn normalize_manifest_steps(mut steps: Vec<SopStep>) -> Vec<SopStep> {
    for (index, step) in steps.iter_mut().enumerate() {
        if step.number == 0 {
            step.number = u32::try_from(index).unwrap_or(u32::MAX).saturating_add(1);
        }
        if step.title.is_empty() {
            step.title = step
                .capability
                .clone()
                .unwrap_or_else(|| step.kind.to_string());
        }
    }
    steps
}

fn merge_positions(
    steps: &mut [SopStep],
    positions: &[StepPosition],
    diagnostics: &mut Vec<Diagnostic>,
) {
    for position in positions {
        match steps.iter_mut().find(|s| s.number == position.step) {
            Some(step) => {
                step.pos = Some(StepPos {
                    x: position.x,
                    y: position.y,
                });
            }
            None => {
                // Upstream drops these without a word. Since markdown steps are
                // renumbered positionally, a position written against an
                // author-chosen number lands here rather than on its node.
                diagnostics.push(Diagnostic::new(
                    DiagnosticKind::LossyConstruct {
                        construct: "positions".into(),
                        detail: format!("no step numbered {} to attach to", position.step),
                    },
                    Location::Manifest {
                        key_path: "positions".into(),
                    },
                ));
            }
        }
    }
}

/// Write a SOP to a directory, creating it if needed.
pub fn write_sop(dir: impl AsRef<Path>, sop: &Sop) -> Result<Vec<Diagnostic>, WriteError> {
    let dir = dir.as_ref();
    std::fs::create_dir_all(dir).map_err(|source| WriteError::io(dir, source))?;

    let (manifest_text, markdown, diagnostics) = render_sop(sop)?;

    let manifest_path: PathBuf = dir.join(MANIFEST_FILE);
    std::fs::write(&manifest_path, manifest_text)
        .map_err(|source| WriteError::io(manifest_path, source))?;
    let steps_path: PathBuf = dir.join(STEPS_FILE);
    std::fs::write(&steps_path, markdown).map_err(|source| WriteError::io(steps_path, source))?;

    Ok(diagnostics)
}

/// Render a SOP to the text of its two files, without touching the filesystem.
pub fn render_sop(sop: &Sop) -> Result<(String, String, Vec<Diagnostic>), WriteError> {
    let positions = sop
        .steps
        .iter()
        .filter_map(|step| {
            step.pos.map(|pos| StepPosition {
                step: step.number,
                x: pos.x,
                y: pos.y,
            })
        })
        .collect();

    let manifest = SopManifest {
        sop: sop.meta.clone(),
        triggers: sop.triggers.clone(),
        positions,
        // Never written: the markdown is authoritative on every read, so
        // emitting the list twice only creates something to drift.
        steps: Vec::new(),
    };

    let manifest_text = write_manifest(&manifest)?;
    let (markdown, diagnostics) = print_steps(&sop.steps);
    Ok((manifest_text, markdown, diagnostics))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{SopMeta, SopStepKind, SopTrigger, StepFailure};

    fn rich_sop() -> Sop {
        let (sop, diagnostics) = read_sop_str(
            r#"
[sop]
name = "deploy-prod"
description = "Production deploy with approval"
version = "1.4.2"
priority = "critical"
admission_policy = "hold"
agent = "release-bot"

[[triggers]]
type = "channel"
channel = "git"
alias = "main"
condition = "$.event_type == \"pull_request.opened\""

[[triggers]]
type = "manual"

[[positions]]
step = 1
x = 320.5
y = -48.0
"#,
            Some(
                r#"## Steps

1. **Preflight** — Check service health.
   - tools: http_request
   - output: {"type":"object"}

2. **Deploy** — Run the deployment.
   - allow-tools: shell
   - deny-tools: git_operations
   - requires_confirmation: true
   - on_failure: retry:2
   - next: 3

3. **Approve** — Human review before publishing.
   - kind: checkpoint
   - policy: prod
   - edit: body
   - prompt: Approve the deploy?

4. **Publish** — Post the result.
   - kind: capability
   - capability: forge.comment
   - with: {"repo":"o/r"}
   - depends_on: 2, 3
   - terminal: true
"#,
            ),
        )
        .expect("fixture parses");
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        sop
    }

    #[test]
    fn a_rich_sop_reads_as_expected() {
        let sop = rich_sop();
        assert_eq!(sop.meta.name, "deploy-prod");
        assert_eq!(sop.triggers.len(), 2);
        assert_eq!(sop.steps.len(), 4);
        assert_eq!(sop.steps[2].kind, SopStepKind::Checkpoint);
        assert_eq!(sop.steps[3].kind, SopStepKind::Capability);
        assert_eq!(sop.steps[1].on_failure, StepFailure::Retry { max: 2 });
        // Position merged onto step 1 by number.
        assert!(sop.steps[0].pos.is_some());
        assert!(sop.steps[1].pos.is_none());
    }

    #[test]
    fn canonical_output_is_locked() {
        let sop = rich_sop();
        let (manifest, markdown, lossy) = render_sop(&sop).unwrap();
        assert!(lossy.is_empty(), "{lossy:?}");
        insta::assert_snapshot!("rich_sop_manifest", manifest);
        insta::assert_snapshot!("rich_sop_markdown", markdown);
    }

    #[test]
    fn a_written_directory_reads_back_to_an_equal_sop() {
        let sop = rich_sop();
        let dir = tempfile::tempdir().unwrap();
        let lossy = write_sop(dir.path(), &sop).unwrap();
        assert!(lossy.is_empty(), "{lossy:?}");

        let (back, diagnostics) = read_sop(dir.path()).unwrap();
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(back, sop);
    }

    #[test]
    fn writing_is_idempotent() {
        let sop = rich_sop();
        let dir = tempfile::tempdir().unwrap();
        write_sop(dir.path(), &sop).unwrap();
        let first = std::fs::read_to_string(dir.path().join(STEPS_FILE)).unwrap();
        let (back, _) = read_sop(dir.path()).unwrap();
        write_sop(dir.path(), &back).unwrap();
        let second = std::fs::read_to_string(dir.path().join(STEPS_FILE)).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn positions_never_leak_into_the_markdown() {
        let sop = rich_sop();
        let (manifest, markdown, _) = render_sop(&sop).unwrap();
        assert!(manifest.contains("[[positions]]"), "{manifest}");
        assert!(!markdown.contains("320.5"), "{markdown}");
    }

    #[test]
    fn steps_are_never_written_to_the_manifest() {
        let sop = rich_sop();
        let (manifest, _, _) = render_sop(&sop).unwrap();
        assert!(!manifest.contains("[[steps]]"), "{manifest}");
    }

    #[test]
    fn markdown_wins_over_manifest_steps_and_the_conflict_is_reported() {
        let (sop, diagnostics) = read_sop_str(
            "[sop]\nname = \"s\"\ndescription = \"d\"\n\n[[steps]]\nnumber = 1\ntitle = \"From TOML\"\n",
            Some("## Steps\n1. **From markdown** - x\n"),
        )
        .unwrap();
        assert_eq!(sop.steps.len(), 1);
        assert_eq!(sop.steps[0].title, "From markdown");
        assert!(
            diagnostics
                .iter()
                .any(|d| d.kind.code() == "lossy_construct"),
            "{diagnostics:?}"
        );
    }

    #[test]
    fn an_empty_markdown_file_still_beats_manifest_steps() {
        // Existence, not content: this is upstream's rule and it is easy to
        // trip over.
        let (sop, _) = read_sop_str(
            "[sop]\nname = \"s\"\ndescription = \"d\"\n\n[[steps]]\nnumber = 1\ntitle = \"From TOML\"\n",
            Some(""),
        )
        .unwrap();
        assert!(sop.steps.is_empty());
    }

    #[test]
    fn manifest_steps_are_used_when_there_is_no_markdown() {
        let (sop, _) = read_sop_str(
            "[sop]\nname = \"s\"\ndescription = \"d\"\n\n[[steps]]\ntitle = \"First\"\n\n[[steps]]\nkind = \"capability\"\ncapability = \"git.status\"\n",
            None,
        )
        .unwrap();
        assert_eq!(sop.steps.len(), 2);
        // Numbers filled positionally only because they were zero.
        assert_eq!(sop.steps[0].number, 1);
        assert_eq!(sop.steps[1].number, 2);
        // An empty title falls back to the capability, then the kind.
        assert_eq!(sop.steps[1].title, "git.status");
    }

    #[test]
    fn manifest_steps_keep_their_authored_numbers() {
        let (sop, _) = read_sop_str(
            "[sop]\nname = \"s\"\ndescription = \"d\"\n\n[[steps]]\nnumber = 5\ntitle = \"Five\"\n",
            None,
        )
        .unwrap();
        // Unlike the markdown path, a non-zero number is preserved — so this is
        // the only route by which gaps and duplicates reach a loaded SOP.
        assert_eq!(sop.steps[0].number, 5);
    }

    #[test]
    fn an_unattachable_position_is_reported() {
        let (_, diagnostics) = read_sop_str(
            "[sop]\nname = \"s\"\ndescription = \"d\"\n\n[[positions]]\nstep = 9\nx = 1.0\ny = 2.0\n",
            Some("## Steps\n1. **A** - x\n"),
        )
        .unwrap();
        assert!(
            diagnostics
                .iter()
                .any(|d| d.message.contains("no step numbered 9")),
            "{diagnostics:?}"
        );
    }

    #[test]
    fn a_missing_manifest_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let err = read_sop(dir.path()).unwrap_err();
        assert!(matches!(err, ReadError::NotASopDirectory { .. }), "{err:?}");
    }

    #[test]
    fn a_sop_with_no_markdown_and_no_steps_is_fine() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(MANIFEST_FILE),
            "[sop]\nname = \"s\"\ndescription = \"d\"\n",
        )
        .unwrap();
        let (sop, _) = read_sop(dir.path()).unwrap();
        assert!(sop.steps.is_empty());
        assert!(sop.triggers.is_empty());
    }

    #[test]
    fn a_minimal_sop_round_trips_through_a_directory() {
        let sop = Sop {
            meta: SopMeta::new("minimal", "nothing much"),
            triggers: vec![SopTrigger::Manual],
            steps: vec![SopStep {
                number: 1,
                title: "Only".into(),
                ..Default::default()
            }],
        };
        let dir = tempfile::tempdir().unwrap();
        write_sop(dir.path(), &sop).unwrap();
        let (back, _) = read_sop(dir.path()).unwrap();
        assert_eq!(back, sop);
    }
}
