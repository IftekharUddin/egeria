# Demo

> **Stub.** Written by issue #34, with captured real output rather than
> illustrative text.

The demo is a GitHub bug-fix workflow in two variants — `with-approval` and
`approval-removed` — that shows the whole pipeline on one realistic example:

1. `egeria import zeroclaw` the SOP.
2. `egeria check` it: the first variant is clean, the second produces
   `EGR-SEC-001` with a witness path from the trigger to the pull-request
   creation.
3. `egeria graph --highlight EGR-SEC-001` renders the same path as Mermaid.
4. `egeria check --format sarif` uploads to code scanning, so the violation
   appears as an annotation.

The same commands run in `.github/workflows/demo.yml`, which is Egeria dogfooding
its own workflow-CI use case: a change that introduces an approval bypass fails
the repository's own checks.
