# Rule registry

Every verification rule Egeria ships is listed here and has a page in this
directory. A test asserts that the registry in `egeria-analysis` and the pages
here are in bijection — a rule without a page, or a page without a rule, fails the
build.

## Identifiers

```
EGR-<AREA>-NNN
```

| Area | Covers |
|---|---|
| `STRUCT` | Reachability, dead branches, illegal cycles — the shape of the graph |
| `RETRY` | Retry bounds and structurally nonterminating failure paths |
| `DATA` | Typed dataflow: producers, availability, port compatibility |
| `SEC` | Approval domination, validator bypass, untrusted-data flow |
| `CAP` | Capability and tool-scope requirements against a target backend |
| `TERM` | Terminal states and required outcomes |

Identifiers are assigned once and **never renumbered or reused**, including for
removed rules (ADR-0007). They end up in suppression comments, CI configuration
and code-scanning history, where a silently reassigned number would change what a
suppression suppresses.

## Registry

Empty — rules arrive with milestone `V1-M1`.

| Rule | Title | Default severity | Engine | Since |
|---|---|---|---|---|
| — | — | — | — | — |

## Page format

Each page begins with YAML front matter that the CLI parses (`egeria explain`
embeds these pages at compile time), followed by prose:

```markdown
---
id: EGR-SEC-001
summary: A privileged effect is reachable without a required approval.
engine: graph
witness: path
---

## What it checks

## Why it matters

## Example witness

## Proof boundary

## Suggested repairs
```

The **proof boundary** section is required, not optional. It states what the rule
does *not* establish. A rule proving that every modeled path to a pull-request
creation passes an approval gate proves nothing about whether the code is
correct, whether the human read it, or whether the service behaves as modeled —
and the page has to say so, because the rule identifier is what people will cite
when they claim something was verified.
