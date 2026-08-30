Closes #

## What changed

<!-- One or two sentences. What does this do that the repository could not do before? -->

## Acceptance criteria

<!-- Copy the checklist from the issue. Check each item and name the test that
     demonstrates it — "verified locally" is not a demonstration. -->

- [ ]

## Checklist

- [ ] `cargo test --workspace` passes locally
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` is clean
- [ ] `cargo fmt --all --check` is clean
- [ ] Docs updated where behavior changed; a new rule also has its
      `docs/rules/EGR-*.md` page and registry row
- [ ] No changes under `external/`, no committed JARs, no hand-edited `Cargo.lock`
- [ ] New dependencies (if any) are justified below and pass `cargo deny check`
- [ ] Snapshot changes (if any) are explained below

## Notes

<!-- Explain any snapshot diffs and why the new output is correct, justify new
     dependencies, and flag anything you had to decide that the issue did not
     settle. -->
