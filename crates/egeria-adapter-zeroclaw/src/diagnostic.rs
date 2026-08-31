//! Diagnostics emitted while reading SOP sources.
//!
//! These are deliberately *not* [`egeria_ir::Finding`] values. A `Finding`
//! describes something a verification rule concluded about a workflow: it
//! carries an `EGR-<AREA>-NNN` rule identifier and a mandatory record of which
//! engine produced it at what scope (ADR-0008). A parse warning has neither —
//! nothing verified anything, and there is no rule to cite. Reusing `Finding`
//! here would mean inventing rule identifiers for parser events, which would
//! put them in the rule registry and, eventually, in someone's SARIF baseline.
//!
//! So the adapter has its own small vocabulary. Import (issue #9) is where SOP
//! sources become a workflow that rules can then say things about.

use std::fmt;

use serde::{Deserialize, Serialize};

/// How much a diagnostic should worry the reader.
///
/// There is no `Error` variant. Anything that prevents producing a value is
/// returned as an `Err`, not collected as a diagnostic — a caller must not be
/// able to ignore it by not looking at the diagnostic list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Something was read successfully, but information was lost or a
    /// construct was not understood.
    Warning,
    /// Worth surfacing, but nothing was lost.
    Note,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Warning => f.write_str("warning"),
            Self::Note => f.write_str("note"),
        }
    }
}

/// Where in a SOP source a diagnostic came from.
///
/// A SOP is a directory of two files, so a location has to say which one, and
/// step-scoped problems need the step number to be actionable — "unrecognized
/// bullet" without a step number sends the reader hunting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Location {
    /// Somewhere in `SOP.toml`.
    Manifest {
        /// Dotted path to the offending key, e.g. `meta.admission_policy` or
        /// `triggers[1].topic`.
        key_path: String,
    },
    /// Somewhere in the prose of `SOP.md`, outside any step.
    Document {
        /// 1-based line number, when known.
        line: Option<usize>,
    },
    /// Within a specific step of `SOP.md`.
    Step {
        /// The step number as written in the source, before any renumbering.
        number: u32,
        /// 1-based line number, when known.
        line: Option<usize>,
    },
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Manifest { key_path } => write!(f, "SOP.toml: {key_path}"),
            Self::Document { line: Some(line) } => write!(f, "SOP.md:{line}"),
            Self::Document { line: None } => f.write_str("SOP.md"),
            Self::Step {
                number,
                line: Some(line),
            } => write!(f, "SOP.md:{line}: step {number}"),
            Self::Step { number, line: None } => write!(f, "SOP.md: step {number}"),
        }
    }
}

/// What kind of thing happened, as a machine-readable discriminant.
///
/// Callers switch on this rather than matching on message text. The variants
/// are open-ended by design — adding one is not a breaking change for anyone
/// who handles the default case.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum DiagnosticKind {
    /// A manifest key Egeria does not model. Carried through untouched where
    /// possible, but not interpreted.
    UnknownManifestKey {
        /// The key exactly as it appeared.
        key: String,
    },
    /// A step sub-bullet whose prefix matched nothing known. Its text is
    /// appended to the step body rather than dropped.
    UnrecognizedBullet {
        /// The bullet text, trimmed.
        text: String,
    },
    /// A recognized bullet whose value did not parse.
    MalformedBullet {
        /// The bullet key, normalized.
        key: String,
        /// Why the value was rejected.
        reason: String,
    },
    /// Step numbers in the source are not a gapless ascending run.
    ///
    /// Not an error: upstream renumbers positionally, so the written numbers
    /// are advisory. Worth saying because a reader who wrote `next: 7` while
    /// looking at a mis-numbered list may not have meant what the parser sees.
    NumberingIrregularity {
        /// Step numbers in the order they appeared.
        written: Vec<u32>,
    },
    /// A construct was understood but cannot survive a round trip unchanged.
    LossyConstruct {
        /// What was affected.
        construct: String,
        /// What specifically is lost.
        detail: String,
    },
}

impl DiagnosticKind {
    /// A short, stable identifier for this kind, for grouping and filtering.
    pub fn code(&self) -> &'static str {
        match self {
            Self::UnknownManifestKey { .. } => "unknown_manifest_key",
            Self::UnrecognizedBullet { .. } => "unrecognized_bullet",
            Self::MalformedBullet { .. } => "malformed_bullet",
            Self::NumberingIrregularity { .. } => "numbering_irregularity",
            Self::LossyConstruct { .. } => "lossy_construct",
        }
    }

    /// The severity this kind carries by default.
    pub fn default_severity(&self) -> Severity {
        match self {
            Self::UnknownManifestKey { .. }
            | Self::UnrecognizedBullet { .. }
            | Self::MalformedBullet { .. }
            | Self::LossyConstruct { .. } => Severity::Warning,
            Self::NumberingIrregularity { .. } => Severity::Note,
        }
    }
}

/// Something worth telling the caller about a SOP source that was nevertheless
/// read successfully.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    /// How much this should worry the reader.
    pub severity: Severity,
    /// What happened, machine-readable.
    pub kind: DiagnosticKind,
    /// Where it happened.
    pub location: Location,
    /// A human-readable sentence. Derived from `kind`; never the sole carrier
    /// of information a caller might want to act on.
    pub message: String,
}

impl Diagnostic {
    /// Build a diagnostic, taking severity and message from the kind.
    pub fn new(kind: DiagnosticKind, location: Location) -> Self {
        let severity = kind.default_severity();
        let message = render_message(&kind);
        Self {
            severity,
            kind,
            location,
            message,
        }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}[{}]: {} ({})",
            self.severity,
            self.kind.code(),
            self.message,
            self.location
        )
    }
}

fn render_message(kind: &DiagnosticKind) -> String {
    match kind {
        DiagnosticKind::UnknownManifestKey { key } => {
            format!("unknown manifest key `{key}`; carried through but not interpreted")
        }
        DiagnosticKind::UnrecognizedBullet { text } => {
            let preview = truncate(text, 60);
            format!("unrecognized bullet `{preview}`; appended to the step body")
        }
        DiagnosticKind::MalformedBullet { key, reason } => {
            format!("could not parse `{key}` bullet: {reason}")
        }
        DiagnosticKind::NumberingIrregularity { written } => {
            let list = written
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "step numbers are not a gapless ascending run ({list}); steps are renumbered by position"
            )
        }
        DiagnosticKind::LossyConstruct { construct, detail } => {
            format!("`{construct}` will not survive a round trip unchanged: {detail}")
        }
    }
}

/// Truncate on a character boundary, appending an ellipsis when shortened.
fn truncate(s: &str, max_chars: usize) -> String {
    let mut out: String = s.chars().take(max_chars).collect();
    if s.chars().count() > max_chars {
        out.push('…');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_is_derived_from_kind() {
        let d = Diagnostic::new(
            DiagnosticKind::UnknownManifestKey {
                key: "meta.wobble".into(),
            },
            Location::Manifest {
                key_path: "meta.wobble".into(),
            },
        );
        assert_eq!(d.severity, Severity::Warning);

        let d = Diagnostic::new(
            DiagnosticKind::NumberingIrregularity {
                written: vec![1, 3],
            },
            Location::Document { line: None },
        );
        assert_eq!(d.severity, Severity::Note);
    }

    #[test]
    fn codes_are_stable_and_distinct() {
        let kinds = [
            DiagnosticKind::UnknownManifestKey { key: "k".into() },
            DiagnosticKind::UnrecognizedBullet { text: "t".into() },
            DiagnosticKind::MalformedBullet {
                key: "k".into(),
                reason: "r".into(),
            },
            DiagnosticKind::NumberingIrregularity { written: vec![1] },
            DiagnosticKind::LossyConstruct {
                construct: "c".into(),
                detail: "d".into(),
            },
        ];
        let codes: Vec<_> = kinds.iter().map(DiagnosticKind::code).collect();
        let unique: std::collections::BTreeSet<_> = codes.iter().collect();
        assert_eq!(codes.len(), unique.len(), "codes must be distinct");
    }

    #[test]
    fn location_renders_readably() {
        assert_eq!(
            Location::Step {
                number: 4,
                line: Some(31)
            }
            .to_string(),
            "SOP.md:31: step 4"
        );
        assert_eq!(
            Location::Manifest {
                key_path: "triggers[0].topic".into()
            }
            .to_string(),
            "SOP.toml: triggers[0].topic"
        );
        assert_eq!(Location::Document { line: None }.to_string(), "SOP.md");
    }

    #[test]
    fn display_includes_code_and_location() {
        let d = Diagnostic::new(
            DiagnosticKind::MalformedBullet {
                key: "switch".into(),
                reason: "expected `name>condition>step`".into(),
            },
            Location::Step {
                number: 2,
                line: None,
            },
        );
        let s = d.to_string();
        assert!(s.contains("malformed_bullet"), "{s}");
        assert!(s.contains("step 2"), "{s}");
        assert!(s.contains("expected `name>condition>step`"), "{s}");
    }

    #[test]
    fn truncation_respects_char_boundaries() {
        // Multi-byte characters must not be split; naive byte slicing panics here.
        let s = "é".repeat(80);
        let out = truncate(&s, 60);
        assert_eq!(out.chars().count(), 61, "60 chars plus the ellipsis");
        assert!(out.ends_with('…'));
    }

    #[test]
    fn diagnostics_round_trip_through_serde() {
        let d = Diagnostic::new(
            DiagnosticKind::LossyConstruct {
                construct: "meta.extra".into(),
                detail: "not modeled".into(),
            },
            Location::Manifest {
                key_path: "meta.extra".into(),
            },
        );
        let json = serde_json::to_string(&d).expect("serialize");
        let back: Diagnostic = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(d, back);
    }
}
