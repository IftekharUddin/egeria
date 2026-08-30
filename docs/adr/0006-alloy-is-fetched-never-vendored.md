# ADR-0006: Alloy is fetched by checksum, never vendored

**Status:** accepted

## Decision

Egeria never vendors, bundles, or redistributes Alloy source or binaries. The
distribution JAR is downloaded on demand by `cargo xtask fetch-alloy` from Maven
Central and verified against a checksum committed in `xtask/pins/`. The
`external/alloy` submodule is a commit pointer for reading source, not a copy.

## Context

Alloy's upstream `LICENSE` file contains the full Apache-2.0 text prefixed with
the line `# THIS IS NOT VALID YET! CURRENTLY CODE IS UNDER MIT LICENSE`. GitHub's
license detection reports `NOASSERTION` as a result. Both readings are
permissive, and neither is likely to cause anyone real trouble — but the file
does not settle which license governs a given release artifact, and a downstream
project that redistributes the code is the one that has to answer for that.

Fetching rather than vendoring costs almost nothing and removes the question
entirely. It also avoids building Alloy from source, which needs Gradle, bnd, and
a recursive clone.

Maven Central publishes a `.sha1` for the artifact but no `.sha256`. The recorded
pin was produced by verifying the download against the published SHA-1 and then
computing SHA-256 from that verified file.

## Consequences

- Nothing Egeria ships contains Alloy code.
- A checksum mismatch during fetch is a hard failure: the file is deleted and the
  task exits nonzero.
- Changing the pinned version changes the scope semantics of every Alloy-backed
  finding, so it is a human-approved change alongside an update to this ADR.
- Anyone wanting to distribute Alloy alongside Egeria should resolve the licensing
  question with the Alloy maintainers first.
