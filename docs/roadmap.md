# Roadmap

Milestones exist as GitHub milestones too. The V1 ones carry issues; the future
ones carry descriptions and entry criteria, and get issues when their entry
criteria are met.

The ordering is deliberate and is the project's main defense against becoming
five unfinished things at once (ADR-0002). Each phase produces something
independently useful, and each later phase depends on the earlier ones having
settled a contract.

## V1 — the compiler and the verifier

### V1-M0 — ZeroClaw round-trip

Prove Egeria can read and canonically re-emit real ZeroClaw SOPs before any IR
exists. Egeria-owned source model, `SOP.toml` and `SOP.md` parser and printer, a
25-fixture corpus covering every documented construct, and a parse-print-parse
identity harness.

**Exit:** every corpus fixture round-trips at source level with committed
snapshots; the parser matches the documented grammar including guard, switch, and
failure precedence.

### V1-M1 — IR and static verifier

The semantic core and the value proposition. Typed Workflow IR with effects,
trust, capabilities and gates; SOP-to-IR-to-SOP conformance; graph analyses; and
the rule set — `EGR-STRUCT-001/002/003`, `EGR-RETRY-001`, `EGR-DATA-001/002`,
`EGR-SEC-001/002/003`, `EGR-TERM-001`, `EGR-CAP-001/002` — each with a
machine-readable witness. Plus the backend contract and the ZeroClaw capability
manifest.

**Exit:** corpus SOP-to-IR-to-SOP is semantically equal; the mutation suite maps
every mutation to its expected diagnostic and covers every shipped rule; the
corpus compiles at `exact` fidelity.

### V1-M2 — Alloy differential backend

The optional formal layer, kept honest to "static by default". Checksum-pinned
JAR fetch, headless `exec --type json` runner with environment-gated tests, trusted
`.als` generation for approval domination, Alloy-instance-to-witness mapping, and
a seeded differential fuzz harness comparing the dominator checker against Alloy.

**Exit:** the Alloy CI job is green under `EGERIA_REQUIRE_ALLOY=1`; a differential
run at N >= 200 locally has zero unexplained divergences, with any divergence
minimized and committed as a regression fixture.

### V1-M3 — CLI and evidence UX

Make the evidence usable. `import`, `check`, `compile`, `explain`, `capabilities`;
graph export as Mermaid, DOT, JSON, and ZeroClaw's Blueprint wire shape, with
witness-path highlighting; SARIF for code scanning; the GitHub bug-fix demo with a
workflow that dogfoods Egeria on itself; and the normative documentation written
from implemented reality.

**Exit:** the README quickstart works end to end; removing an approval from the
demo produces an `EGR-SEC-001` witness in terminal, Mermaid, and SARIF form.

## Future milestones

Each has entry criteria — the thing that must exist before starting is usually
more important than the feature itself.

### F1 — TaskSpec and natural-language planning

A typed `TaskSpec` capturing required outcomes, operations, hard constraints,
forbidden effects, preferences, and explicit assumptions and ambiguities; an
extractor from natural language; and a generator producing a single workflow that
must pass the V1 verifier. Inferred assumptions are user-visible, because every
proof depends on them being right.

The evaluation question is not "does the diagram look plausible" but: how
accurately did the TaskSpec capture the user's constraints, how often does
verification catch generation errors, and how often does verification create
false confidence?

**Entry:** V1 complete; IR schema stable; the rule set settled enough to generate
against.

### F2 — Pattern registry, candidate search, and Pareto ranking

Structural operators — human gate, validator before effect, executor-reviewer,
fan-out/gather, fallback, sandbox-then-promote — applied as deterministic
transforms, pruned statically and formally, and presented as a genuine
nondominated frontier rather than four prompt adjectives. Metrics are labeled by
provenance: measured on a named benchmark, analytically derived, or an
uncalibrated prior. They are never blended.

**Entry:** F1's TaskSpec exists to hold the task contract constant; analytical
metrics (model calls, privileged nodes, human gates, critical path) are computable
from the IR.

### F3 — SMT and optimization layer

An `egeria-smt` crate wrapping Z3 for what neither graph analysis nor Alloy does
well: budgets, call caps, reviewer cardinality, weighted soft constraints, and
Pareto objective combinations. Environment-gated the same way Alloy is.

**Entry:** F2 produces enough structured variants that arithmetic selection
matters. Un-defers part of ADR-0009.

### F4 — Benchmark harness and run corpus

Execute compiled SOPs against a real ZeroClaw runtime, capturing success, cost,
latency, retries and interventions with full provenance: workflow digest, task set
digest, model and version, backend and adapter version, tool versions, run count,
seed policy, evaluation version.

**Entry:** V1 compiles the demo workflows at exact fidelity; a runnable ZeroClaw
environment and an explicit experiment budget exist.

### F5 — Calibrated probabilistic prediction

Hierarchical models with uncertainty intervals, calibration tracking (Brier and
log scores, calibration curves, discrimination reported separately), and an
explicit out-of-distribution abstention state. Never a point estimate without an
interval. Naive independent-node multiplication is a labeled baseline, not the
model — agent node outcomes share strong latent dependencies.

**Entry:** F4's corpus is large enough to estimate uncertainty honestly. Fitting a
sophisticated model to synthetic guesses is the failure this ordering prevents.

### F6 — Open registry and provenance

A public workflow and benchmark registry structured like a benchmark project
rather than a template marketplace: signed result metadata, versioned holdout
suites, repeated trials, and CI that runs declarative submissions in strong
isolation. Contributor-provided evaluation code never runs on privileged runners.

A workflow is not deprecated for scoring lower if it occupies a different region
of the cost, security, and latency frontier.

**Entry:** F4's result formats are stable; an anti-gaming protocol is designed.

### F7 — Second backend adapter

A genuinely different runtime — durable-execution systems in the Temporal mold are
the natural stress test — to find out whether the IR is portable or merely
"ZeroClaw with renamed fields". Capability negotiation must report exact,
emulated, lossy, or rejected honestly, and must fail closed when a security
property cannot be preserved.

**Entry:** the backend contract is stable through V1 use; portability fixtures
selected.

### F8 — Verified Workflow Studio

The visual IDE described in
[docs/vision/verified-workflow-studio.md](vision/verified-workflow-studio.md): a
Blueprint-style infinite canvas with typed pins, context-aware node creation, an
inspector, live verification states, counterexample paths drawn on the graph, and
compile-fidelity feedback when the target harness changes.

The architectural rule is the one from ADR-0003: the UI graph is a projection of
the IR, never a second source of truth. Its killer feature is not bezier wires —
it is that editing the graph immediately changes provable properties and compile
feasibility.

**Entry:** V1 complete, with the IR and `Finding` schemas stable enough to
generate TypeScript types from `schemas/`. May proceed in parallel with F1 and
later phases. The canvas library is re-evaluated at entry: the vision document
specifies Rete.js v2, earlier research suggested React Flow, and the decision
should be made against the requirements as they stand then.

### F9 — Research: counterexample-guided repair

The cleanest first study the project enables. Generate candidate workflows from a
TaskSpec under three conditions — no structural verification; static verification
with deterministic repair; static plus Alloy counterexamples with model-driven
repair — and measure invariant violation rate, task completion, repair success,
iterations, cost, and false-positive and false-negative diagnostics.

The contribution is not "LLMs can generate graphs" or "formal methods can check a
plan", both of which have prior art. It is applying counterexample-guided repair
to a typed, portable agent-workflow architecture with backend semantics attached.

**Entry:** F1 generation exists; V1 witnesses are machine-consumable, which
ADR-0008 already ensures.

### F10 — Upstream ZeroClaw integration

The cooperative half of the hybrid strategy in ADR-0001: render Egeria output in
ZeroClaw's own canvas through the SopGraph wire shape, and contribute small
upstream changes that make the adapter contract cleaner.

**Entry:** the SopGraph export is proven against a live ZeroClaw web UI.

## How V1 serves the Studio

The Studio is a future milestone, but several V1 decisions exist specifically so
that it can be built without redesigning the core:

- **Verification states.** The Studio shows Verified, Warning, Violation, and
  Unverified. The last one — "the graph changed since the last analysis" — is
  exactly `CheckReport.workflow_digest` compared against the current semantic
  hash. Because layout is excluded from that hash (ADR-0003), panning the canvas
  does not mark a workflow stale.
- **Counterexample overlays.** The Studio highlights the violating path on the
  graph. That is `Finding.witness.nodes` and `Finding.witness.edges`, which V1's
  CLI already uses for `--highlight`.
- **Repairs.** `Finding.remediation` starts as human-readable strings in V1 and
  becomes machine-applicable graph patches when there is a UI to apply them to.
- **Node families.** V1's node kinds are the ZeroClaw-expressible core. The
  richer families in the vision document — Tool, Data, Resource, Verification,
  Harness Extension — arrive with F1, F2, and F7, as generation and a second
  backend create the need for them.
- **Types across the boundary.** `schemas/` is generated from the Rust types and
  committed, so the Studio generates its TypeScript from the same source of truth
  instead of restating the shapes by hand.
