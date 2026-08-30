# Schemas

JSON Schemas for Egeria's public data formats. These files are **generated** from
the Rust types via `schemars` and committed as golden files; a test regenerates
them and fails if the committed copy has drifted.

Do not hand-edit anything here. Change the Rust type, run the test, review the
regenerated schema in the diff, and commit it with the code change.

| File | Source of truth | Introduced by |
|---|---|---|
| `workflow-ir-v1alpha1.schema.json` | `egeria_ir::Workflow` | issue #6 |
| `finding-v1alpha1.schema.json` | `egeria_ir::CheckReport` | issue #7 |

These schemas are also the intended contract for non-Rust consumers — the future
Verified Workflow Studio generates its TypeScript types from them rather than
restating the shapes by hand.
