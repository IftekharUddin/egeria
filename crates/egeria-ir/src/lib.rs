//! Workflow IR: the harness-independent semantic core of Egeria.
//!
//! Everything else in the workspace is defined in terms of the types in this
//! crate. The IR describes *semantics* — typed ports, control and data
//! dependencies, effects, trust labels, capabilities, approval gates, retry
//! policies — never the syntax of any particular runtime.
//!
//! Two invariants are load-bearing and must survive every change here:
//!
//! * View and layout data is carried but is **not** semantic (ADR-0003). It is
//!   excluded from equality, from the semantic hash, and from every analysis.
//! * The [`Finding`] vocabulary is the universal verifier output (ADR-0008).
//!   Terminal rendering, SARIF, graph highlighting, and the Alloy backend are
//!   all projections of the same structure.
//!
//! This crate is a stub. See the open issues in milestone `V1-M1`.
