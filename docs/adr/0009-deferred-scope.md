# ADR-0009: Deferred scope

**Status:** accepted

## Decision

The following are deliberately **not** built in V1. Do not add them, and do not
add stubs or placeholder crates for them.

- **`egeria-smt` / Z3.** Numeric and resource reasoning — budgets, call caps,
  reviewer cardinality, weighted soft constraints, Pareto objectives — is where
  an SMT solver beats both graph analysis and Alloy. It becomes useful once there
  are alternative topologies to choose between, which V1 does not generate. Z3 is
  already available on the development machine; adding the crate later is a
  one-commit change.
- **Structured loop and subflow nodes.** V1 represents bounded retry explicitly
  and treats other cycles as findings. General loops need decisions about
  bounds, carried state, and lowering to backends that have no loop construct.
  Guessing now would bake the wrong answer into the IR.
- **TaskSpec and natural-language generation.** Generating workflows before the
  IR and verifier are stable produces an LLM emitting another loosely specified
  workflow language. Generation should target a settled contract and be measured
  by whether its output passes verification.

## Context

Every one of these is a genuinely good idea with a designed future milestone.
They are listed here because each is the kind of thing a capable contributor
naturally reaches for mid-task — "this rule would be easier with a solver", "this
issue really wants a loop node" — and doing so quietly expands V1 past what can
be finished and verified.

## Consequences

- An issue that appears to need one of these is a signal to comment on the issue,
  not to build the missing piece. Apply `needs/human-decision`.
- Un-deferring is a roadmap decision recorded by updating this ADR, and each has
  explicit entry criteria in `docs/roadmap.md`.
- Empty placeholder crates are worse than nothing: they add build and lint surface
  and imply work is underway when it is not.
