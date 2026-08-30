# ADR-0002: V1 is a Rust CLI

**Status:** accepted

## Decision

V1 is a Rust workspace producing a command-line tool. It contains no web UI, no
TypeScript, no natural-language workflow generation, no probabilistic prediction,
and no registry.

## Context

The full vision spans a typed IR, a verifier, a formal backend, natural-language
planning, topology search, calibrated prediction, a public registry, and a
Blueprint-style visual IDE. Attempting them together produces several unfinished
projects rather than one working one — the single most likely way for this
project to fail.

The ordering follows from what depends on what. Natural-language generation
without a stable IR produces another loosely specified workflow language.
Probabilistic ranking without an execution corpus produces confident numbers with
nothing behind them. A visual editor without settled semantics becomes a second
source of truth competing with the IR. The verifier and the compiler, by
contrast, depend on nothing but the IR, and are independently useful the day they
work.

## Consequences

- The V1 backlog is finishable, and every issue in it has a testable acceptance
  criterion.
- Graph visualization in V1 is export-based — Mermaid, DOT, JSON, and ZeroClaw's
  own Blueprint wire format — which renders in tools people already have.
- The deferred work is not cancelled. It is scheduled, with entry criteria, in
  `docs/roadmap.md`, and the UI design is recorded in
  `docs/vision/verified-workflow-studio.md` so that V1's data structures are
  built to serve it.
- Anything that would pull a JavaScript toolchain or an LLM dependency into the
  repository during V1 is out of scope by definition.
