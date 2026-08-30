# ADR-0007: Verifier rules have stable identifiers

**Status:** accepted

## Decision

Every verifier rule has an identifier of the form `EGR-<AREA>-NNN`, where `AREA`
is one of `STRUCT`, `RETRY`, `DATA`, `SEC`, `CAP`, or `TERM`, and `NNN` is a
zero-padded sequence number within that area. Identifiers are assigned once and
never renumbered or reused — not even when a rule is removed.

Each rule has a page at `docs/rules/EGR-<AREA>-NNN.md` and a row in
`docs/rules/README.md`. A test asserts the registry and the pages are in
bijection.

## Context

Rule identifiers end up in places Egeria does not control: suppression comments,
CI configuration, SARIF uploads and code-scanning history, dashboards, saved
queries, and other people's documentation. Renumbering silently changes what a
suppression suppresses, which is a security-relevant failure for rules like
approval domination.

Areas rather than one flat sequence make identifiers self-describing at a glance,
and let a reader guess what `EGR-SEC-003` is about before looking it up.

## Consequences

- A removed rule's identifier is retired, with its page kept and marked as such.
- `RuleId` validates its format at construction, so a malformed identifier cannot
  reach a report.
- Adding a rule means adding its page in the same change; the bijection test fails
  otherwise.
- Every page carries a proof-boundary section, because the identifier is what
  people will cite when they claim something was verified.
