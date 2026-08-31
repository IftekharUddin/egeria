//! ZeroClaw SOP adapter: parse, import to IR, lower back, and compile.
//!
//! Egeria owns its own model of the SOP file format, parsed against the
//! documented grammar rather than by linking the ZeroClaw runtime (ADR-0005).
//! The only ZeroClaw crate this workspace ever links is `zeroclaw-sop-graph`,
//! pinned to tag `v0.8.4`, and only for the Blueprint-graph wire shape.
//!
//! Reference source lives under `external/zeroclaw/` as a read-only submodule.
//! Fetch it with `git submodule update --init --depth 1 external/zeroclaw`.
//!
//! Modules land milestone by milestone; see the open issues in `V1-M0` and
//! `V1-M1`.

pub mod diagnostic;
pub mod error;
pub mod manifest;
pub mod model;
pub mod sop;
pub mod steps;

pub use diagnostic::{Diagnostic, DiagnosticKind, Location, Severity};
pub use error::{ReadError, WriteError};
pub use manifest::{parse_manifest, write_manifest};
pub use model::{
    FilesystemEventKind, PlannedToolCall, Sop, SopAdmissionPolicy, SopExecutionMode, SopManifest,
    SopMeta, SopPriority, SopStep, SopStepKind, SopTrigger, StepFailure, StepPos, StepPosition,
    StepRouting, StepSchema, StepToolScope, SwitchRule,
};
pub use sop::{read_sop, read_sop_str, render_sop, write_sop};
pub use steps::{parse_steps, print_steps};
