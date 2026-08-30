# Verified Workflow Studio

Blueprint-style UI for designing, verifying, analyzing, and compiling agent
workflows.

> **Source:** `verified_workflow_studio_ui.docx`, 2026-08-30, converted to
> Markdown. This is a **vision document**, not a specification of anything that
> exists. It describes the eventual interface, scheduled as milestone **F8** in
> [the roadmap](../roadmap.md). V1 is a command-line tool (ADR-0002) and ships no
> UI at all.
>
> Where this document and the founding research disagree on the canvas library —
> Rete.js v2 here, React Flow there — this document is the working default, and
> the decision is re-made against requirements when F8 actually starts.
>
> Its central architectural claim is already binding on V1: **the UI graph is not
> the workflow.** That is ADR-0003, and several V1 decisions exist to make this
> interface buildable later without redesigning the core. See "How V1 serves the
> Studio" in the roadmap.

The interface is intentionally modeled after Unreal Engine Blueprints: an
infinite node canvas, typed pins, direct manipulation, contextual node creation,
and deep inspection. The difference is that the graph represents a
harness-agnostic Workflow IR rather than a runtime-specific workflow.

## 1. Product goal

The UI should make sophisticated agent workflow design approachable without
reducing the system to a black box. A user should be able to describe an outcome
in natural language, inspect the generated architecture, modify it visually,
verify important properties, compare alternatives, and compile the final workflow
into a supported agent harness.

The user is not merely drawing automation steps. They are editing an explicit
model of agent behavior.

### Design principles

| Principle | Meaning |
|---|---|
| Intent first | Users may begin from a prompt, a template, an imported workflow, or an empty canvas. |
| Graph is inspectable | Every meaningful execution dependency is visible as a node, connection, property, or constraint. |
| Verification is interactive | Formal verification is surfaced as understandable paths and violated rules, not theorem-prover output. |
| Runtime is replaceable | The visual model is independent from ZeroClaw, LangGraph, or any future harness. |
| Analysis is actionable | Cost, latency, reliability, and risk estimates should lead directly to alternative workflow designs. |

## 2. Main workspace

The primary screen should feel closer to an IDE than to a SaaS form builder. Most
of the screen belongs to the canvas, while secondary panes remain available for
node discovery, detailed configuration, verification, execution state, and
analysis.

- **Top command bar** — workflow name, verification state, optimization actions,
  run controls, target harness, save and version state.
- **Node palette** — searchable library of agents, tools, control-flow nodes,
  human gates, data transformations, resources, and verification helpers.
- **Infinite canvas** — pan, zoom, box select, multi-drag, grouping, reroute
  points, comments, subgraphs, and visual execution traces.
- **Inspector** — configuration for the selected node or connection: model,
  prompt, permissions, retries, schemas, timeouts, resources, and constraints.
- **Bottom analysis pane** — problems, formal verification, runtime trace, cost,
  latency, probabilities, logs, and generated alternatives.

## 3. Blueprint-style interaction model

### Typed pins and explicit flow

Connections should carry meaning. Control flow, structured data, errors,
approvals, events, resources, and state should not all look or behave the same.
Typed ports prevent obviously invalid wiring before formal verification is
needed.

Example node interface:

```text
+------------- Research Agent -------------+
|                                          |
* execution                      execution *
* query: string                     result * Report
* credentials: GitHub               errors * Error
+------------------------------------------+
```

### Context-aware node creation

Dragging a wire into empty space should open a search menu filtered to compatible
nodes. If a user drags from a `Report` output and searches "verify," the editor
can prioritize Fact Check, LLM Judge, Citation Validator, or Schema Validator
nodes that accept that type. This keeps complex graphs fast to author.

### Canvas ergonomics

- Right-click searchable node menu
- Reroute points for long connections
- Frames and comments for explaining subgraphs
- Collapsible groups and reusable subflows
- Keyboard-first duplication, deletion, navigation, and search
- Auto-layout for generated workflows, followed by free manual arrangement
- Undo/redo that treats multi-node graph edits as coherent operations

## 4. Node model

The node library should expose concepts that are portable across harnesses.
Runtime-specific nodes may exist, but they should be clearly marked as
target-specific extensions.

| Node family | Purpose |
|---|---|
| Agent | LLM-backed reasoning or generation step. |
| Tool | External capability: browser, filesystem, GitHub, database, API, shell. |
| Control Flow | Branch, merge, loop, retry, parallelize, wait, timeout. |
| Human | Approval, review, clarification, escalation. |
| Data | Transform, validate, parse, map, aggregate, schema conversion. |
| Resource | Credential, workspace, memory/state handle, rate-limit budget. |
| Verification | Invariant, assertion, policy boundary, independent validator. |
| Harness Extension | A capability available only when targeting a specific runtime. |

## 5. Formal verification UX

Formal methods should remain invisible until they provide a useful answer. The
user should not need to understand Alloy, relations, SAT solving, or bounded model
checking to benefit from them.

### Verification states

- **Verified** — all configured properties hold within the selected verification
  scope.
- **Warning** — the workflow is executable, but an intended property is absent,
  weak, or not proven.
- **Violation** — a concrete counterexample exists.
- **Unverified** — the graph has changed since the last analysis, or contains
  unsupported semantics.

### Counterexamples become graph overlays

When verification finds a violating execution path, the editor should highlight
that path directly on the graph and explain the violated property in plain
language.

```text
PROPERTY VIOLATED

  MergePR requires HumanApproval == Granted

Counterexample:
  Issue -> Patch -> Test -> MergePR

Suggested repair:
  Insert approval gate between Test and MergePR.
```

The repair may be offered as a one-click graph patch, but the user should remain
able to inspect the change before accepting it.

## 6. Predictive and optimization UX

The same graph should become the surface for probabilistic analysis. Historical
benchmark data, model and tool reliability estimates, current observations, and
simulation can be projected back onto nodes and paths.

- **Workflow reliability** — estimated probability that the workflow satisfies its
  task-level success criteria.
- **Expected cost** — model and tool cost for one execution.
- **Latency** — expected and tail runtime, such as P50 and P95.
- **Human intervention** — probability that a run reaches a manual gate or
  escalation.
- **Retry and failure hotspots** — nodes contributing disproportionately to
  retries or unsuccessful runs.

Optimization should generate alternatives rather than silently mutate the
workflow. A user may request "optimize for reliability," "reduce expected cost,"
"minimize latency," or "reduce human intervention," then compare formally valid
candidates side by side.

## 7. Architecture boundary

The most important architectural rule is that **the UI graph is not the
workflow**. The canonical workflow exists as a harness-independent intermediate
representation. The UI, the Alloy model, the predictive analyzer, and the runtime
adapters all consume or update that representation.

The Workflow IR is the shared contract between authoring, verification, analysis,
and execution targets.

### Why this separation matters

- Changing UI frameworks does not redefine workflow semantics.
- Formal tools can reason about workflows without depending on rendering details.
- ZeroClaw becomes the first execution target rather than the permanent owner of
  the format.
- New harnesses integrate by implementing capability discovery and IR
  compilation.
- Benchmarks and the public workflow registry remain portable across runtimes.

### Recommended implementation stack

| Layer | Technology | Role |
|---|---|---|
| Desktop shell | Tauri 2 | Cross-platform desktop application and Rust integration. |
| Frontend | React + TypeScript | Application UI and state projection. |
| Graph editor | Rete.js v2 | Sockets, typed connections, context menus, reroutes, grouping, custom nodes. |
| Layout | ELK | Automatic layout for generated or imported graphs. |
| Core | Rust | Workflow IR, validation, compiler, adapters, orchestration. |
| Formal methods | Alloy sidecar/adapter | Constraint checking, model finding, counterexamples. |
| Runtime target | ZeroClaw first | Compile Workflow IR into ZeroClaw SOP representation. |

## 8. Key user flows

**Prompt to workflow.** User describes an outcome; the planner creates a typed
Workflow IR; the graph is auto-laid out; verification runs; the user reviews the
candidate; compile or run.

**Manual authoring.** User starts empty; drags nodes and connects typed pins; the
inspector configures semantics; verification runs continuously or on demand;
export or run.

**Import and inspect.** User imports a ZeroClaw SOP or other supported workflow;
the adapter converts it to IR; the editor visualizes it; analysis identifies risks
and optimization opportunities.

**Repair.** Verification finds a counterexample; the violating path is
highlighted; the system proposes a graph repair; the user accepts or edits
manually; re-verify.

**Optimize.** User chooses an objective; the system produces multiple valid
candidates; compare cost, reliability, latency, and security; select a candidate;
compile for the chosen harness.

## 9. MVP scope

The first useful version should prove the interaction and the architecture
without implementing every long-term feature.

- Tauri and React application shell.
- Infinite canvas with typed pins and five to ten portable node types.
- Workflow IR serialization and deterministic graph-to-IR editing.
- Inspector panel for node semantics, schemas, permissions, retry, timeout, and
  model/tool configuration.
- ZeroClaw SOP import and export.
- Verification for a small set of high-value invariants.
- Counterexample path highlighting in the graph.
- Auto-layout for generated and imported graphs.
- Basic execution trace overlay for ZeroClaw runs.

### Deferred after the Studio MVP

Prompt-to-workflow synthesis, probabilistic scoring, multi-objective workflow
optimization, public registry integration, cross-harness compilation,
collaborative editing, and advanced debugging layer on top once the Workflow IR
and editor semantics are stable.

## 10. Definition of success

A successful UI should let a user understand a nontrivial agent workflow at a
glance, discover invalid or dangerous execution paths before runtime, modify the
system without editing configuration files, and retain ownership of the workflow
independent of the execution harness.

The intended product experience is not "draw boxes and connect arrows." It is an
IDE for agent architecture: design, inspect, verify, predict, optimize, and
execute from the same graph.
