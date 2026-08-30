# ADR-0008: The `Finding` is the universal verifier output

**Status:** accepted

## Decision

Every verification engine emits the same `Finding` structure: rule identifier,
severity, status, message, a witness (node ids, edge ids, and values), the model
that produced it (engine, engine version, scope), and suggested remediations.
Terminal output, JSON, SARIF, graph highlighting, and Alloy results are all
projections of that one structure.

## Context

The alternative — engines returning strings, and each renderer parsing or
formatting its own — makes several desirable things impossible at once. A witness
that is a string cannot be highlighted on a canvas, cannot become a SARIF
`codeFlow`, cannot be compared across engines, and cannot be fed to an automatic
repair pass.

Making the witness structured makes all of those fall out of the same data:
graph highlighting is the node and edge sets, a SARIF code flow is the path in
order, an IDE jump-to-witness is the first node, automatic repair reads the
remediation, and a research dataset is the findings themselves.

The engine field is what makes differential validation possible. When the
dominator analysis and the Alloy model both report on the same property, the
findings must be shape-identical and differ only in `model` — which is precisely
the assertion the differential harness makes.

## Consequences

- A new engine implements a mapping to `Finding`; it does not get its own
  diagnostic type.
- The `model` field is not optional decoration. A finding that does not say what
  produced it, at what version, and within what scope, is not usable as evidence.
- The eventual visual editor consumes findings directly for its verification
  states and counterexample overlays, with no intermediate format to keep in sync.
