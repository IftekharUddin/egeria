# Research

Source material behind the project's design, kept verbatim so that a later reader
can check a decision against the reasoning that produced it.

## `alloy-zeroclaw-workflow-research.md`

The founding deep-research assessment (August 2026). It evaluated the original
idea — a ZeroClaw fork with a Blueprint-style canvas and Alloy underneath — and
recommended against it, arguing for a separate harness-independent workflow
compiler and verifier with ZeroClaw as the first backend. Most of the ADRs in
`docs/adr/` trace directly to it.

It is a snapshot, not a maintained document. Where it and the implementation
disagree, the implementation and the ADRs win; where it and a later document
disagree on something not yet built, say so explicitly rather than silently
picking one.

Two things in it are worth reading even if you skip the rest: the table of which
verification technique fits which property, and the section on why a formal
result must always state its modeling boundary.
