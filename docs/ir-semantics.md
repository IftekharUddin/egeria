# Workflow IR semantics

> **Stub.** Written from implemented reality by issue #35, once the IR, the
> importer, the lowerer and the round-trip suite exist. Until then the normative
> source is the types in `egeria-ir` and the tests around them.

Planned sections:

## Document structure

`apiVersion`, metadata, interface, triggers, nodes, edges, policies, invariants,
objectives, assumptions, view.

## Node kinds

What `agent`, `capability`, `approval` and `end` mean, and what each carries:
typed ports, effects, trust, permissions, gates, execution policy, and the
backend-opaque implementation payload.

## Edge kinds

`control`, `data`, and `failure`; conditions; switch arm ordering and port names.

## Guard, switch and terminal precedence

The exact encoding of ZeroClaw's `when`, `switch`, `next` and `terminal`
semantics as IR edges, including what a false guard bypasses.

## End semantics

Why there is exactly one end node, and what routes to it.

## Retry semantics

Bounded retry as node state rather than as a cycle, and why arbitrary cycles are
findings instead of loops (ADR-0009).

## View separation and the semantic hash

What the hash covers, what it deliberately does not, and the test that holds the
line (ADR-0003).

## Effects, trust and capabilities

The effect ladder from `pure` to `destructive`, trust classes, data
classifications, and how capability references are named.
