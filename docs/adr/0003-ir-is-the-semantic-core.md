# ADR-0003: The Workflow IR is the semantic core

**Status:** accepted

## Decision

The Workflow IR is the canonical representation of a workflow. Every other
surface — the SOP adapter, the analyses, the Alloy models, the CLI renderings,
and eventually the visual editor — consumes or produces the IR rather than owning
its own notion of what a workflow is.

View and layout data (canvas positions, geometry) is carried in the IR but is
explicitly **not semantic**: it is excluded from equality, from the semantic
hash, and from every analysis.

## Context

The IR is a compiler IR, not a JSON export format. That distinction drives
several concrete choices:

- **Stable string node identifiers**, not ordinal step numbers. Reordering steps
  must not change node identity. The ZeroClaw adapter lowers stable IDs into SOP
  step numbers at the boundary.
- **Control edges and data edges are distinct.** ZeroClaw reached the same
  conclusion independently with its flow and data pin classes.
- **Effects, permissions, trust labels, and data classifications are
  first-class.** A `github.create_pr` call is not "a tool node with a prompt"; it
  is an external write requiring a capability. Without that in the type, useful
  static verification is impossible.
- **Prompts and model configuration live in the node implementation**, not in
  control semantics, so swapping models does not change the graph.

The layout rule matters more than it looks. If dragging a node five pixels
changes the semantic hash, then every proof, signature, and cached analysis is
invalidated by a cosmetic edit — and the eventual visual editor becomes unusable
for exactly the workflows it is meant to help with.

## Consequences

- `semantic_hash()` covers semantics only. A test asserts that mutating view data
  leaves it unchanged and that mutating an edge changes it.
- The future visual editor is a projection of the IR, never a second source of
  truth. It may edit the IR; it may not mean something the IR does not say.
- Adding a field to the IR is a semantic act requiring thought about what it means
  for equality, hashing, analysis, and lowering.
