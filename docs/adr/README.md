# Architecture decision records

Short records of decisions that shape the project, kept so that a later reader —
human or agent — can tell the difference between "nobody got to it yet" and "this
was deliberately excluded, and here is why."

Adding a new ADR is ordinary work. **Changing an existing one is a human
decision**, because several of these exist specifically to stop a well-meaning
contributor from doing the obvious thing.

| # | Decision |
|---|---|
| [0001](0001-separate-repository.md) | Egeria is a separate repository; ZeroClaw is the first backend, not a fork target |
| [0002](0002-v1-is-a-rust-cli.md) | V1 is a Rust CLI — no web UI, no NL generation, no probability, no registry |
| [0003](0003-ir-is-the-semantic-core.md) | The Workflow IR is the semantic core; view and layout data are not semantic |
| [0004](0004-verification-portfolio.md) | Verification is a portfolio: static by default, Alloy optional, proof boundaries explicit |
| [0005](0005-zeroclaw-integration-boundary.md) | Egeria owns its SOP parser; `zeroclaw-sop-graph` is the only linked ZeroClaw crate |
| [0006](0006-alloy-is-fetched-never-vendored.md) | Alloy is fetched by checksum, never vendored or redistributed |
| [0007](0007-stable-rule-ids.md) | Verifier rules have stable `EGR-<AREA>-NNN` identifiers |
| [0008](0008-finding-is-the-universal-output.md) | The machine-readable `Finding` is the universal verifier output |
| [0009](0009-deferred-scope.md) | Deferred by decision: SMT/Z3, structured loops, TaskSpec/NL |
