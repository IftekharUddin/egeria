# explicit-next

Steps 1, 3 and 4 each name their successor with `next:`, so file order and
execution order are not the same thing. The run goes 1 -> 3 -> 4 -> 2 and stops
there: step 2 declares no `next:` and is marked `terminal: true`. Step 2 sits
second in the file purely because that is where a reader looking for the
announcement expects it.

Every `next:` and `depends_on:` target here is a *position*, not the digit an
author typed. The digits happen to agree with the positions in this file, but
nothing in the format guarantees that — the parser renumbers from 1 in file
order and resolves every cross-reference against the new numbers.

## Steps

1. **Fetch the build manifest** - Pull the manifest for the tag being released.
   - tools: http_request
   - output: {"type":"object","required":["digest"],"properties":{"digest":{"type":"string"}}}
   - next: 3

2. **Announce the release** - Post the release note once the artifact is live.
   - tools: git_operations
   - depends_on: 4
   - terminal: true

3. **Verify the checksum** - Compare the downloaded digest against the manifest.
   - tools: shell
   - allow-tools: shell, file_read
   - on_failure: fail
   - next: 4

4. **Upload the artifact** - Push the verified artifact to the release bucket.
   - tools: shell
   - deny-tools: git_operations
   - requires_confirmation: true
   - on_failure: retry: 2
   - next: 2
