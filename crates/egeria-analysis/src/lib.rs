//! Static analysis and the EGR rule engine over the Egeria Workflow IR.
//!
//! Egeria verifies with the cheapest sound technique for each property
//! (ADR-0004): graph traversal, dominators, strongly connected components,
//! typed dataflow, and taint propagation come first; the optional Alloy
//! backend in `egeria-alloy` is a cross-check, not the primary verifier.
//!
//! Every rule has a stable identifier of the form `EGR-<AREA>-NNN` (ADR-0007),
//! a page under `docs/rules/`, and emits `egeria_ir::Finding` values carrying a
//! machine-readable witness.
//!
//! This crate is a stub. See the open issues in milestone `V1-M1`.
