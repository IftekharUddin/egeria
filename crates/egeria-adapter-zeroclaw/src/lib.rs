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
//! This crate is a stub. See the open issues in milestones `V1-M0` and `V1-M1`.
