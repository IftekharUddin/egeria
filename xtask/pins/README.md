# Pinned artifact checksums

`cargo xtask fetch-alloy` verifies every download against the checksum recorded
here. A mismatch is a hard failure: the downloaded file is deleted and the task
exits nonzero.

## `alloy-6.2.0.sha256`

Artifact: `org.alloytools:org.alloytools.alloy.dist:6.2.0`
Source: <https://repo1.maven.org/maven2/org/alloytools/org.alloytools.alloy.dist/6.2.0/org.alloytools.alloy.dist-6.2.0.jar>
Size: 21,064,917 bytes

Maven Central publishes a `.sha1` for this artifact but no `.sha256`. The SHA-1
was checked against the published value at the time this pin was recorded —

```
f399311928e4e9f5cc8a6c09facc36c6dd4f4b9c  (matches the published .sha1)
```

— and the SHA-256 in `alloy-6.2.0.sha256` was then computed from that same
verified file. SHA-256 is what the fetch task enforces.

Changing a pin means changing the Alloy version Egeria verifies against, which
is a decision with consequences for every recorded proof scope. It requires a
human-approved change alongside an ADR update, never a drive-by bump.
