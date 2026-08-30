# ADR-0005: The ZeroClaw integration boundary

**Status:** accepted

## Decision

Egeria owns its own model and parser for the ZeroClaw SOP format, written against
the documented grammar. The only ZeroClaw crate in Egeria's build graph is
`zeroclaw-sop-graph`, consumed as a Cargo git dependency pinned to tag `v0.8.4`,
and only for the Blueprint graph wire shape. `zeroclaw-runtime` is never a
dependency. The `external/zeroclaw` submodule is read-only reference material and
never a build input.

## Context

The alternative — linking ZeroClaw's runtime types directly — looks like it would
save work. It would instead couple Egeria's semantics to another project's
internal representation, make every upstream refactor a breaking change here, and
quietly import a large dependency tree into a tool whose value proposition
includes being lightweight.

Parsing the documented format ourselves also produces something valuable: it
forces the adapter to state its understanding of the grammar explicitly, in code
with tests, which is exactly the artifact needed to detect drift between what the
documentation says and what the runtime does.

`zeroclaw-sop-graph` is the one exception, and a cheap one. It is a pure-serde
projection crate with a tiny dependency footprint (serde, strum, optional
schemars), deliberately shared upstream across the runtime, the gateway schema
and the TUI. Consuming it lets Egeria emit the exact wire shape ZeroClaw's canvas
already renders. It is `publish = false` upstream and the ZeroClaw crates.io
entries are `0.0.0` name reservations, so a pinned git dependency is the only
available route.

## Consequences

- The adapter's understanding of SOP syntax is testable and reviewable in one
  place, against a fixture corpus that covers every documented construct.
- Upstream changes cannot silently change Egeria's semantics; they show up as
  fixture failures when the pin is deliberately moved.
- The capability manifest must be derived from documented and tested behavior, not
  inferred from field names. A serialization field resembling a concept does not
  imply the runtime implements that concept — `depends_on` expressing fan-in
  ordering does not mean the runtime executes those steps concurrently.
- Moving the pin is a human-approved change accompanied by a fixture review.
