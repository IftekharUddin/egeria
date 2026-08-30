//! Backend contract and capability-aware compilation for Egeria.
//!
//! A backend declares what it can actually represent, and compilation reports
//! fidelity honestly: `exact`, `emulated` (with residual risks), `lossy`
//! (requiring explicit acceptance), or `rejected` (with the missing
//! capabilities named). A capability that a target cannot enforce is never
//! silently emulated — the compiler fails closed.
//!
//! Compiled artifacts must run without a JVM, a solver, or any part of the
//! design-time environment (ADR-0004).
//!
//! This crate is a stub. See the open issues in milestone `V1-M1`.
