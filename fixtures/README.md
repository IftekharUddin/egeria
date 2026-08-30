# Fixtures

Shared test corpora. Crates reach these paths via `env!("CARGO_MANIFEST_DIR")`
plus `../../fixtures`, so the corpus is owned by the repository rather than by
any one crate.

| Directory | Contents | Introduced by |
|---|---|---|
| `sops/` | ZeroClaw SOP source fixtures, one directory per fixture (`SOP.toml` + `SOP.md`), plus `INDEX.md` — a construct-coverage matrix that a test asserts is complete | issue #4 |
| `ir/` | Hand-written Workflow IR JSON documents, used where a SOP round-trip would obscure what is under test | issue #6 |
| `demo/` | The GitHub bug-fix demo, in `with-approval/` and `approval-removed/` variants | issue #34 |
| `regression/` | Minimized reproducers discovered by the differential fuzz harness; committed as they are found | issue #27 |

Fixtures use realistic content — real tool names, real schemas, real conditions.
A fixture whose only purpose is to trip a rule says so in `sops/INDEX.md`.
