# Working on Egeria

This is the operating manual for anyone — agent or human — developing this
repository. Read it before picking up an issue.

Egeria is a workflow compiler and verifier for agentic systems: ZeroClaw SOP in,
typed Workflow IR, static verification with machine-readable counterexamples,
capability-aware compilation back out.

## Invariants

These come from the ADRs in `docs/adr/`. They are not style preferences; a change
that breaks one needs the ADR changed first, by a human.

- **Layout is never semantic.** View data (canvas positions) is carried through
  the IR but excluded from equality, from the semantic hash, and from every
  analysis. Moving a node must not invalidate a proof. (ADR-0003)
- **Static analysis is the default; Alloy is optional.** Reach for dominators,
  SCCs, dataflow, and taint first. Alloy is a cross-check and a tool for bounded
  relational questions — not the primary verifier. (ADR-0004)
- **Compiled artifacts run without a JVM, a solver, or anything from the design
  environment.** If a change makes a compiled workflow depend on Alloy, the
  change is wrong. (ADR-0004)
- **Rule IDs are stable forever.** `EGR-<AREA>-NNN`, never renumbered, never
  reused, even if a rule is removed. (ADR-0007)
- **`Finding` is the universal verifier output.** Terminal text, SARIF, graph
  highlighting, and Alloy results are all projections of the same structure.
  Never invent a parallel diagnostic type. (ADR-0008)
- **Never link `zeroclaw-runtime`.** `zeroclaw-sop-graph` is the only ZeroClaw
  crate in the build graph. The SOP format is parsed against its documented
  grammar by code Egeria owns. (ADR-0005)
- **Never modify anything under `external/`.** Those submodules are read-only
  reference material.
- **Never bump a pin on your own.** The ZeroClaw tag, the Alloy version, and the
  Alloy checksum are human-approved decisions accompanied by an ADR update.
- **Do not build deferred work.** `egeria-smt`/Z3, structured loop nodes, and
  TaskSpec/NL generation are explicitly out of V1 (ADR-0009). If an issue seems
  to need one, say so in the issue rather than building it.

## Commands

```bash
cargo build --workspace
cargo test --workspace
cargo test -p egeria-analysis                  # one crate
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
cargo deny check                                # licenses, advisories, sources
cargo insta review                              # review snapshot changes
```

Alloy-backed work needs a JVM (Java 17+) and the distribution JAR:

```bash
cargo xtask fetch-alloy                         # downloads + verifies the JAR
EGERIA_REQUIRE_ALLOY=1 cargo test -p egeria-alloy
```

Without Java or the JAR, Alloy-backed tests **skip with a message** — that is the
designed local experience. `EGERIA_REQUIRE_ALLOY=1` turns skips into failures, and
CI sets it so those tests can never silently pass by not running.
`EGERIA_ALLOY_JAR` points at a JAR you manage yourself.

Reference source (never a build input, never fetched by CI):

```bash
git submodule update --init --depth 1 external/zeroclaw
git submodule update --init --depth 1 external/alloy
```

## Architecture

```
egeria-ir  <-  egeria-analysis  <-  egeria-compiler  <-  egeria-adapter-zeroclaw  <-  egeria-cli
   ^                  ^
   +---- egeria-alloy +
```

| Crate | Owns |
|---|---|
| `egeria-ir` | Workflow, Node, Edge, Port, Capability, Effect, RetryPolicy, ApprovalGate, Finding, CheckReport, validation, semantic hash, generated schemas |
| `egeria-analysis` | Graph indexes and algorithms (petgraph), the rule trait and registry, every `EGR-*` rule |
| `egeria-compiler` | `Backend` trait, `BackendCapabilities`, `CompileResult` fidelity ladder |
| `egeria-adapter-zeroclaw` | SOP model, parser, printer, import, lowering, ZeroClaw capability manifest, SopGraph export |
| `egeria-alloy` | JAR location and execution, `.als` generation, Alloy-instance-to-witness mapping, differential fuzzing |
| `egeria-cli` | Argument parsing, output rendering (human, JSON, SARIF, Mermaid, DOT), exit codes |

Nothing depends on `egeria-cli`. Rendering belongs in the CLI; semantics belong
below it.

## Where things live

- **Fixtures** — `fixtures/`, reached via `env!("CARGO_MANIFEST_DIR")`. A new SOP
  fixture is a directory `fixtures/sops/<name>/` with `SOP.toml` and `SOP.md`,
  **and** a row in `fixtures/sops/INDEX.md`; a test asserts the index covers every
  fixture directory. Test harnesses auto-discover fixtures — never hand-register
  them.
- **Rules** — a new rule is five things, not one: the implementation, its tests
  (including a corpus-clean zero-false-positive test), an entry in the mutation
  suite, a page at `docs/rules/EGR-<AREA>-NNN.md`, and a row in
  `docs/rules/README.md`. A test asserts the registry and the pages are in
  bijection, so a missing page fails the build.
- **Schemas** — `schemas/*.json` are generated from Rust types and committed as
  golden files. Change the type, run the test, commit the regenerated schema.
  Never hand-edit them.
- **ADRs** — `docs/adr/`. Adding a decision is fine. Changing one is a human's
  call.

## Picking up work

1. An issue is workable when **every issue in its "Blocked by" list is closed**.
   The body text is authoritative; the `status/*` labels are a convenience and
   may lag.
2. Prefer the lowest-numbered workable issue in the earliest open milestone.
   Milestones are ordered `V1-M0` -> `V1-M1` -> `V1-M2` -> `V1-M3`; issues in
   different lanes (adapter, IR, infra) can proceed in parallel.
3. Comment on the issue to claim it before starting.
4. Branch `issue-<N>-<short-slug>`. One issue, one pull request.
5. Open the PR with `Closes #N` and the issue's acceptance criteria as a checked
   list, each item pointing at the test that demonstrates it.
6. If an issue is ambiguous or its acceptance criteria are unachievable as
   written, say so in a comment and apply `needs/human-decision` rather than
   guessing. Deciding a semantic question quietly is the expensive failure mode
   here.

## Pull requests

- CI must be green: `fmt`, `clippy -D warnings`, `test`, and `deny`.
- Stay in scope. No drive-by refactors of code the issue does not touch.
- New external dependencies need a justification in the PR body and must pass
  `cargo deny`. Prefer the standard library and what is already in the lock file.
- Snapshot changes must be explained in the PR body — say what changed and why the
  new output is correct. An unexplained snapshot update is how a regression gets
  blessed.
- Never hand-edit `Cargo.lock`. Never commit a JAR or any other fetched artifact.
- Never touch `external/`.

## Definition of done

Every acceptance criterion is demonstrably met by a named test that runs in CI.
Docs and the rule registry are updated in the same change. `cargo test
--workspace` is green locally, and the change does nothing the issue did not ask
for.
