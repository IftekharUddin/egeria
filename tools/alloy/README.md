# tools/alloy

Download target for the Alloy distribution JAR. **Everything in this directory
except this README is `.gitignore`d** — the JAR is never committed (ADR-0006,
and see `THIRD-PARTY.md` for why).

```bash
cargo xtask fetch-alloy
```

That downloads `org.alloytools.alloy.dist-6.2.0.jar` (~21 MB) from Maven Central
and verifies it against `xtask/pins/alloy-6.2.0.sha256`. A mismatch deletes the
file and fails.

Running Alloy also needs a JVM (Java 17 or newer). Without one, Egeria's
Alloy-backed tests skip with a message and the rest of the workspace is
unaffected — that is the intended experience, not a degraded one. CI sets
`EGERIA_REQUIRE_ALLOY=1` so those tests fail loudly there instead of skipping
silently.

To point Egeria at a JAR you manage yourself, set `EGERIA_ALLOY_JAR` to its
path.
