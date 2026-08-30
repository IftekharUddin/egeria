# external/

Read-only reference source. **Never modify anything here**, and never make it a
build input.

These submodules exist so that whoever is working on the adapter or the Alloy
backend can read the upstream source and documentation directly — the SOP grammar,
the runtime's step contract, Alloy's CLI options — rather than guessing or
working from a summary. CI does not fetch them.

They are not checked out by default. Fetch what you need:

```bash
git submodule update --init --depth 1 external/zeroclaw
git submodule update --init --depth 1 external/alloy
```

| Path | Upstream | Pinned to | Read it for |
|---|---|---|---|
| `zeroclaw/` | `zeroclaw-labs/zeroclaw` | `v0.8.4` (`a56c345`) | `docs/book/src/sop/` — the SOP grammar and worked examples; `crates/zeroclaw-runtime/src/sop/` — step and routing shapes; `crates/zeroclaw-sop-graph/src/lib.rs` — the Blueprint wire format |
| `alloy/` | `AlloyTools/org.alloytools.alloy` | `v6.2.0` (`59ba203`) | `org.alloytools.alloy.cli/` — the headless `exec` command's real options and output format |

Both pins are deliberate and are changed only by a human, alongside an ADR update
and a fixture review (ADR-0005, ADR-0006). A pin that moves on its own turns
"our tests pass" into a statement about a different piece of software than the one
we documented.

The relationship between these pointers and what Egeria actually builds against
is worth being precise about, because they are not the same thing:

- Egeria links exactly one upstream crate, `zeroclaw-sop-graph`, as a Cargo git
  dependency declared in the workspace manifest. That dependency is what the
  compiler sees; this submodule is what you read.
- Egeria links no Alloy code at all. The Alloy JAR is fetched separately by
  `cargo xtask fetch-alloy` and is never committed. See `THIRD-PARTY.md` for the
  licensing reason.
