# ADR-0001: Egeria is a separate repository

**Status:** accepted

## Decision

Egeria lives in its own repository. ZeroClaw is its first backend. Egeria is not
a ZeroClaw fork and does not aim to become part of ZeroClaw.

## Context

The obvious way to build a verified workflow tool for ZeroClaw would be to fork
ZeroClaw and add a verification layer inside it. Investigation showed that to be
the worst available option.

ZeroClaw already has a substantial SOP Blueprint graph layer: a dedicated
`zeroclaw-sop-graph` crate defining flow and data pins, typed nodes, wires,
diagnostics, persisted canvas positions and run states; a runtime that projects
SOPs into that graph; and an interactive canvas in its web application. That code
is under active development. A fork would duplicate work already being done
upstream and would inherit permanent merge pressure against a fast-moving
codebase.

More importantly, the interesting problem is not ZeroClaw-shaped. Semantic
portability, verifiable graph contracts, counterexample-driven repair, and
capability negotiation across runtimes are all questions about workflows in
general. Answering them inside one runtime's repository would bake that runtime's
assumptions into the answer — which is precisely the failure the project exists
to avoid.

## Consequences

- Egeria owns its schema, its release cadence, and its dependency surface.
- ZeroClaw is treated as a real, independent system: pinned, read-only, and
  integrated through a documented adapter rather than shared internals.
- Cooperation with ZeroClaw is expected and welcome — small upstream changes that
  make the adapter contract cleaner, and eventually rendering Egeria output in
  ZeroClaw's own canvas. That is a hybrid strategy, not an incompatible parallel
  universe.
- The project must prove its own value. Without a host runtime carrying it, the
  IR and the verifier have to be independently useful, which is the right bar.
