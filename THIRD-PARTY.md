# Third-party software

Egeria itself is licensed under `MIT OR Apache-2.0`. This file records what
Egeria depends on, what it links, and — importantly — what it deliberately does
**not** redistribute.

## ZeroClaw

- Upstream: <https://github.com/zeroclaw-labs/zeroclaw>
- License: `MIT OR Apache-2.0` (the repository ships both `LICENSE-MIT` and
  `LICENSE-APACHE`; its `Cargo.toml` declares the dual license)
- Pinned reference: tag `v0.8.4` (commit `a56c345d51dd8ab562e9351e0d4ab83f6a741db9`)

Two distinct uses, deliberately kept separate:

1. **`external/zeroclaw`** is a read-only git submodule. It exists so agents and
   humans can read the upstream SOP grammar and runtime source while working on
   the adapter. It is never a build input, and CI never fetches it.
2. **`zeroclaw-sop-graph`** is the single ZeroClaw crate Egeria links, as a
   Cargo git dependency pinned to tag `v0.8.4` (ADR-0005). It is `publish =
   false` upstream, so crates.io is not an option; the crates.io entries for the
   ZeroClaw workspace are `0.0.0` name reservations, not usable releases.
   Egeria never links `zeroclaw-runtime`.

Bumping either pin is a deliberate, human-approved change — see `CLAUDE.md`.

## Alloy

- Upstream: <https://github.com/AlloyTools/org.alloytools.alloy>
- Pinned reference: tag `v6.2.0` (commit `59ba2033993449d483d54acad0e11a7bbf20354f`)
- Distribution artifact: `org.alloytools:org.alloytools.alloy.dist:6.2.0` on
  Maven Central

**Egeria does not vendor, bundle, or redistribute Alloy source or binaries**
(ADR-0006). The reason is a genuine licensing ambiguity upstream: the repository
`LICENSE` file contains the full Apache-2.0 text, but is prefixed with the line

> `# THIS IS NOT VALID YET! CURRENTLY CODE IS UNDER MIT LICENSE`

GitHub's license detection therefore reports `NOASSERTION` for the repository.
Both readings are permissive, but the file does not settle which one governs a
given release artifact, so Egeria takes the conservative route:

- `external/alloy` is a submodule *pointer* — a commit reference, not a copy.
- The distribution JAR is downloaded at the developer's or CI's request by
  `cargo xtask fetch-alloy` and verified against `xtask/pins/alloy-6.2.0.sha256`.
  It is `.gitignore`d and never committed.
- Nothing Egeria compiles or ships contains Alloy code. A compiled workflow
  artifact has no JVM, solver, or Alloy dependency of any kind.

Anyone redistributing Alloy alongside Egeria should resolve the upstream
licensing question with the Alloy maintainers first.

## Rust dependencies

Ordinary crates.io dependencies are governed by `deny.toml`, which restricts the
allowed license set and forbids unknown registries and unknown git sources. Run
`cargo deny check` to audit; CI runs it on every pull request.
