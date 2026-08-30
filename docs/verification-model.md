# Verification model

> **Stub.** Filled by issue #35, using evidence produced by the mutation suite
> (#19) and the differential harness (#27).

Planned sections:

## The engine portfolio

Which technique answers which question, and why the cheapest sound one wins
(ADR-0004).

## What a finding claims

The `Finding` structure, and why `model` — engine, engine version, scope — is not
optional decoration.

## Proof boundaries

The canonical VERIFIED / NOT VERIFIED text. This is a shared constant: the CLI's
`--verbose` footer prints the same string this document quotes, so the two cannot
drift.

## Scope semantics

What a bounded scope means, and why the absence of a counterexample within a
finite scope is not a theorem about arbitrary workflows.

## Mutation coverage

The mutation-operator matrix: each operator, the rule it should trigger, and the
fixture it was applied to.

## Differential validation

Results of comparing the static dominator analysis against Alloy on randomly
generated graphs: sample size, divergences found, and what each divergence turned
out to be.
