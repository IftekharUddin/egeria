# ADR-0004: Verification is a portfolio

**Status:** accepted

## Decision

Egeria verifies with the cheapest sound technique for each property. Ordinary
graph and dataflow analysis is the default and the largest part of the verifier.
Alloy is an optional design-time backend for bounded relational and temporal
questions and for differential validation. Every finding states the boundary of
what it proves.

Compiled artifacts run without a JVM, a solver, or any part of the design-time
environment.

## Context

It is tempting to encode every question as a relational model and let a solver
answer it. That would be slower, harder to package, harder to explain, and harder
to map back onto a specific path in a specific workflow.

Most of what matters has a classical answer. Unreachable nodes are graph
traversal. Unstructured cycles are strongly connected components. "Every publish
path passes approval" is dominance on a control-flow graph. Missing producers are
definite-assignment analysis. Port mismatches are type checking. Sensitive data
reaching an untrusted sink is taint propagation. Each of these is linear or near
enough, and each produces a witness a person can read.

Alloy earns its place where bounded relational reasoning genuinely adds
something: structural synthesis under constraints, alternative approval policies,
temporal properties over bounded execution states, and — the use V1 actually
ships — an independent check that the hand-written dominator analysis is right.
Its bounded scopes are a feature for finding small counterexamples fast, but they
mean the absence of a counterexample within a scope is not a theorem about
arbitrary workflows, and the tool must say so.

The packaging consequence is decisive. Alloy is a JVM artifact. Making it
mandatory would put a JVM between a user and a compiled workflow, which
contradicts the entire point of compiling to a lightweight runtime.

## Consequences

- Every finding carries its engine, engine version, and scope, and every rule page
  has a proof-boundary section stating what is *not* proven.
- A rule implemented in Alloy that could have been a graph algorithm is a design
  error, not a clever solution.
- Alloy-backed tests skip when no JVM is present and fail hard under
  `EGERIA_REQUIRE_ALLOY=1`, so they never silently pass by not running.
- A differential fuzz harness compares the static checker against Alloy on random
  small graphs. A divergence is a bug in one of them and becomes a committed
  regression fixture.
- If it turns out that nearly all product value comes from the classical
  analyses, that is a legitimate and publishable result, not a failure.
