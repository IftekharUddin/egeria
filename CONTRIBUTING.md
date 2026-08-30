# Contributing

Egeria is built from its issue backlog, largely by coding agents. Humans are very
welcome — the workflow is the same either way, and it is written down in
[CLAUDE.md](CLAUDE.md): how the crates fit together, how to run the build, how to
pick up an issue, and what a finished change looks like.

Short version:

1. Find an issue whose "Blocked by" list is fully closed. Comment to claim it.
2. Branch `issue-<N>-<slug>`, one issue per pull request.
3. Open the PR with `Closes #N` and the acceptance criteria as a checked list,
   each item pointing at the test that proves it.
4. Keep CI green: `cargo fmt --all --check`, `cargo clippy --workspace
   --all-targets -- -D warnings`, `cargo test --workspace`, `cargo deny check`.

If something in an issue is ambiguous, ask in the issue instead of guessing. This
project is about being precise regarding what is and is not proven, and a
quietly-made semantic decision is expensive to undo later.

Before proposing a large change, read the relevant ADR in [docs/adr/](docs/adr/).
Several things that look like obvious improvements — adding a solver, making
layout semantic, linking the ZeroClaw runtime — are deliberate exclusions with
reasons recorded there. Changing one starts with changing the ADR.

## License

Contributions are dual-licensed under `MIT OR Apache-2.0`, matching the project.
By submitting a contribution you agree it may be distributed under those terms.
