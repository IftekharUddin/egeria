# Backend contract

> **Stub.** Filled by issues #21 and #22, and extended with the full ZeroClaw
> mapping table by #35.

Planned sections:

## The `Backend` trait

What a backend must provide: name, version, a capability manifest, and lowering.

## Capability manifests

The shape of `BackendCapabilities` — control, state, human, effects, data — and
the rule that every claim in a manifest is grounded in documented, tested
behavior rather than inferred from a field name. A serialization field that
resembles a concept does not mean the runtime implements it.

## The fidelity ladder

`exact`, `emulated`, `lossy`, `rejected`. What each means, when each is
permitted, and why compensation is not rollback: sending a second request that
reverses a refund, deployment or mutation can itself fail, may not restore
observational equivalence, and leaves intermediate states visible. An IR that
requires atomic rollback is never silently lowered onto a backend that offers
only compensation.

## Failing closed

A backend that cannot enforce a capability boundary cannot honestly compile a
workflow whose invariant depends on that boundary.

## ZeroClaw mapping

The normative table from IR construct to SOP construct, with a citation for every
capability claim.
