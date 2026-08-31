# SOP fixture corpus

Twenty-five ZeroClaw SOP directories covering every construct the format has.
Everything downstream tests against this corpus: round-trip conformance, the
import mapping, every verifier rule's zero-false-positive check, the mutation
suite, and compile fidelity. **A construct absent from this corpus is a
construct nothing is verifying.**

Generated claims in the tables below were extracted from the fixture files
themselves, not transcribed — but the tables are maintained by hand, and
`crates/egeria-adapter-zeroclaw/tests/fixtures_parse.rs` asserts that every
directory here appears in this file.

## Adding a fixture

Create `fixtures/sops/<name>/` with `SOP.toml` and `SOP.md`, then add a row to
the first table below. The harness discovers directories from the filesystem, so
that is all — but a directory with no row here fails the build.

Two rules worth stating because they are easy to get wrong:

* **Use realistic content.** Real tool names, real capability ids, conditions in
  the real grammar. A fixture full of `foo` and `bar` passes "it parses" while
  testing nothing.
* **Root routing guards at `$.steps.<n>.<field>`.** A step-level `when:` or a
  switch port guard is evaluated against `{"steps": {...}}`, so a bare
  `$.severity` never resolves and the guard fails closed to false on every run.
  A fixture whose guards can never be taken is dead weight that looks alive.

## Fixtures

| Fixture | Steps | What it demonstrates |
|---|---:|---|
| `admission-hold` | 5 | Admission control made coherent by the procedure: one migration at a time, concurrent triggers queued rather than dropped, the queue bounded by runs parked at a real approval gate. |
| `approval-edit-gate` | 5 | A checkpoint carrying both `prompt:` and `edit:`, so an approver can amend a field before the run resumes. |
| `approval-policy-quorum` | 5 | A checkpoint naming an approval policy. Policies themselves live in daemon config; the SOP only references one. |
| `capability-forge-comment` | 3 | The headless-review shape: generate a comment, gate it on human approval, then post it. `kind:`/`capability:` are sub-bullets, which is where they actually work. |
| `capability-llm-generate` | 3 | A deterministic `llm.generate` capability step with an authored `with:` object. |
| `checkpoint-basic` | 4 | A plain human approval standing between a rendered config and the restart that applies it. |
| `depends-on-fanin` | 5 | A diamond: two independent gatherers joined by a later step. Mixes the underscore and hyphen spellings of `depends_on`/`on_failure` deliberately. |
| `explicit-next` | 4 | File order and execution order diverge — the run goes 1 → 3 → 4 → 2. |
| `goto-cycle` | 5 | **Deliberately broken.** Failure routes form a cycle with no exit. |
| `goto-recovery` | 5 | `on_failure: goto:` routing into an escalation step that ends the run. |
| `linear-minimal` | 3 | The floor of the format: one trigger, three steps, no routing at all. |
| `mode-overrides` | 5 | `deterministic = true` hard-overriding an authored `execution_mode`, plus per-step `mode:` and `agent:` overrides. |
| `planned-calls` | 5 | `call:` bullets with `{{steps.N}}` bindings and pinned sample outputs. One step carries two, since `call:` is the only accumulating key. |
| `positions` | 4 | Canvas coordinates in `[[positions]]`, including negative and fractional values. Layout is carried but never semantic. |
| `retry-bounded` | 4 | A flaky external call with a bounded `retry:` policy. |
| `schema-mismatch` | 4 | **Deliberately broken.** A producer's `output:` type conflicts with its consumer's `input:` type. |
| `stagex-like` | 8 | The corpus's substantial fixture: an eight-step deterministic release pipeline with a genuine dependency diamond. |
| `switch-multiway` | 5 | A multi-port switch with a catch-all ordered last. The clean counterexample for port-reachability analysis. |
| `switch-shadowed` | 5 | **Deliberately broken.** Two switch ports are unreachable: one repeats an earlier guard, one sits after the catch-all. |
| `terminal-early` | 5 | `terminal: true` partway down the list, so later steps are reachable only by an explicit jump. |
| `tool-scopes` | 5 | `tools:`, `allow-tools:` and `deny-tools:` together, including a present-but-empty allow-list and both alias spellings. |
| `trigger-conditions` | 4 | Conditions on the variants that have the field: an MQTT JSON-path guard, an AMQP numeric guard, and a peripheral bare comparison. |
| `triggers-many` | 4 | Five trigger variants on one procedure, proving `webhook` and `cron` carry no condition while the others do. |
| `typed-contracts` | 5 | A chain where each step's `output:` schema matches the next step's `input:`, so the contract is statically checkable. |
| `when-guard-jump` | 6 | A `when:` guard paired with `next:`. A false guard falls through to the linear successor — unless the step is `terminal:`, where it completes the run. |

## Deliberately broken fixtures

These three are **intentionally defective**. They parse cleanly — a fixture the
parser cannot read tests nothing — but they carry a semantic defect that a
later verifier rule is expected to find. Each one says so in its own prose too.

**Do not "fix" them.** They are the positive cases for rules that do not exist
yet; repairing them would silently delete the only evidence those rules work.

| Fixture | Defect | Expected to trip |
|---|---|---|
| `switch-shadowed` | unreachable switch arms (a shadowed guard, and a port after the catch-all) | a switch-port reachability rule (`EGR-STRUCT-002`) |
| `goto-cycle` | a failure-route cycle with no exit | a cycle / non-termination rule (`EGR-STRUCT-003`, `EGR-RETRY-001`) |
| `schema-mismatch` | producer/consumer type conflicts: array vs object, integer vs string | a typed-dataflow rule (`EGR-DATA-002`) |

## Coverage: bullet keys

All 21 keys and all 25 accepted spellings. The four aliases are asymmetric
upstream — `allow_tools`/`deny_tools` are underscore forms of hyphen-canonical
keys, while `depends-on`/`on-failure` are hyphen forms of underscore-canonical
ones — and there is no `requires-confirmation`, so each alias needs its own
fixture coverage.

| Bullet key | Fixtures |
|---|---|
| `tools:` | `admission-hold`, `approval-edit-gate`, `approval-policy-quorum`, `capability-llm-generate` … |
| `allow-tools:` | `admission-hold`, `approval-policy-quorum`, `depends-on-fanin`, `explicit-next` … |
| `allow_tools:` | `tool-scopes` |
| `deny-tools:` | `admission-hold`, `depends-on-fanin`, `explicit-next`, `goto-recovery` … |
| `deny_tools:` | `tool-scopes` |
| `requires_confirmation:` | `admission-hold`, `checkpoint-basic`, `explicit-next`, `goto-recovery` … |
| `kind:` | `admission-hold`, `approval-edit-gate`, `approval-policy-quorum`, `capability-forge-comment` … |
| `capability:` | `capability-forge-comment`, `capability-llm-generate`, `planned-calls`, `stagex-like` … |
| `with:` | `capability-forge-comment`, `capability-llm-generate`, `planned-calls`, `stagex-like` … |
| `input:` | `depends-on-fanin`, `goto-cycle`, `goto-recovery`, `retry-bounded` … |
| `output:` | `admission-hold`, `approval-edit-gate`, `approval-policy-quorum`, `capability-llm-generate` … |
| `when:` | `approval-policy-quorum`, `retry-bounded`, `typed-contracts`, `when-guard-jump` |
| `next:` | `explicit-next`, `mode-overrides`, `planned-calls`, `retry-bounded` … |
| `terminal:` | `admission-hold`, `approval-edit-gate`, `approval-policy-quorum`, `capability-forge-comment` … |
| `depends_on:` | `admission-hold`, `approval-edit-gate`, `approval-policy-quorum`, `capability-forge-comment` … |
| `depends-on:` | `depends-on-fanin` |
| `switch:` | `switch-multiway`, `switch-shadowed`, `terminal-early`, `trigger-conditions` |
| `on_failure:` | `admission-hold`, `approval-edit-gate`, `approval-policy-quorum`, `capability-forge-comment` … |
| `on-failure:` | `depends-on-fanin` |
| `mode:` | `mode-overrides` |
| `agent:` | `approval-policy-quorum`, `mode-overrides`, `switch-multiway` |
| `call:` | `planned-calls` |
| `prompt:` | `admission-hold`, `approval-edit-gate`, `mode-overrides`, `planned-calls` … |
| `policy:` | `admission-hold`, `approval-edit-gate`, `approval-policy-quorum`, `capability-forge-comment` … |
| `edit:` | `admission-hold`, `approval-edit-gate`, `capability-forge-comment`, `planned-calls` … |

## Coverage: trigger variants

All nine. `webhook` and `cron` are the only two with no `condition` field, so no
fixture puts one there.

| Trigger | Fixtures |
|---|---|
| `mqtt` | `approval-edit-gate`, `goto-cycle`, `schema-mismatch`, `trigger-conditions` … |
| `webhook` | `approval-edit-gate`, `explicit-next`, `retry-bounded`, `triggers-many` |
| `cron` | `capability-llm-generate`, `depends-on-fanin`, `goto-recovery`, `mode-overrides` … |
| `peripheral` | `trigger-conditions` |
| `filesystem` | `checkpoint-basic`, `goto-recovery`, `mode-overrides`, `triggers-many` |
| `calendar` | `triggers-many` |
| `channel` | `admission-hold`, `approval-policy-quorum`, `capability-forge-comment`, `planned-calls` … |
| `manual` | `admission-hold`, `approval-policy-quorum`, `capability-forge-comment`, `capability-llm-generate` … |
| `amqp` | `stagex-like`, `terminal-early`, `trigger-conditions` |

## Coverage: step kinds and failure policies

| Construct | Fixtures |
|---|---|
| `kind: execute` | `admission-hold`, `approval-edit-gate`, `approval-policy-quorum`, `capability-forge-comment` … |
| `kind: checkpoint` | `admission-hold`, `approval-edit-gate`, `approval-policy-quorum`, `capability-forge-comment` … |
| `kind: capability` | `capability-forge-comment`, `capability-llm-generate`, `planned-calls`, `stagex-like` … |
| `on_failure: fail` | `approval-policy-quorum`, `capability-forge-comment`, `explicit-next`, `switch-shadowed` |
| `on_failure: retry` | `admission-hold`, `approval-edit-gate`, `approval-policy-quorum`, `capability-forge-comment` … |
| `on_failure: goto` | `admission-hold`, `goto-cycle`, `goto-recovery`, `mode-overrides` … |
