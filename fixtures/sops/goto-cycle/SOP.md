# goto-cycle

**This fixture is deliberately broken. Do not "fix" it.**

Each recovery step was written to hand off to the next specialist handler, and
the last one hands back to the first: step 2 fails into 3, step 3 fails into 4,
and step 4 fails into 2. The three form a closed cycle in the failure graph with
no `terminal: true`, no `on_failure: fail`, and no bounded retry anywhere on it,
so a persistently failing shipper never leaves the loop and never reaches the
bookkeeping in step 5. Each of the three does still have a success exit — that
is the point: the failure edges alone form a strongly connected component with
no escape, and a run only leaves it by a handler eventually succeeding, which is
exactly what a wedged ingest endpoint will not do.

The defect is structural, not syntactic — the file parses cleanly and every
`goto` target exists. Finding it is a verifier's job.

## Steps

1. **Refresh the shipper credentials** - Mint a short-lived token for the log ingest endpoint before touching the buffer.
   - tools: http_request
   - output: {"type":"object","required":["token","expires_in"]}

2. **Drain the buffer** - Read the on-disk spool and hand the batch to the uploader.
   - allow-tools: file_read, shell
   - input: {"type":"object","required":["token"]}
   - output: {"type":"object","required":["batch_id","records"]}
   - on_failure: goto:3

3. **Retry the upload** - Re-send the batch that the drain could not place, one chunk at a time.
   - tools: http_request
   - input: {"type":"object","required":["batch_id"]}
   - on_failure: goto:4

4. **Reset the connection** - Tear down the ingest session and rebuild it, then go back to draining.
   - tools: shell, http_request
   - on_failure: goto:2

5. **Record the batch** - Store the shipped batch id so the next run resumes after it.
   - tools: memory_store
   - depends_on: 2
   - terminal: true
