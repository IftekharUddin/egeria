# goto-recovery

Every step on the happy path routes its failure to step 5, the escalation step,
with `on_failure: goto:5`. Step 5 pages the on-call rotation and is marked
`terminal: true`, so the recovery arm ends there instead of falling through into
the success bookkeeping in step 4. The two arms are disjoint: 4 ends the run
clean, 5 ends it escalated.

Nothing here routes on success — no `next:`, no `switch:`, no `when:` — so the
happy path is exactly file order, 1 to 2 to 3 to 4, and every edge that is not
file order is a failure edge. Step 5 is reachable only by `goto`, since step 4
is terminal and never falls through into it.

## Steps

1. **Read the current certificate** - Parse the installed chain and compute the days remaining before expiry.
   - tools: file_read, shell
   - output: {"type":"object","required":["serial","days_remaining"]}
   - on_failure: goto:5

2. **Request the renewal** - Ask the ACME endpoint for a fresh certificate for the edge hostnames.
   - allow-tools: http_request
   - deny-tools: file_write
   - input: {"type":"object","required":["serial"]}
   - output: {"type":"object","required":["chain","fingerprint"]}
   - on_failure: goto:5

3. **Install and reload** - Write the new chain into the edge config and reload the proxy without dropping connections.
   - allow-tools: file_write, shell
   - requires_confirmation: true
   - input: {"type":"object","required":["chain","fingerprint"]}
   - on_failure: goto:5

4. **Record the rotation** - Store the new fingerprint and expiry so the next run can skip a fresh certificate.
   - tools: memory_store
   - depends_on: 3
   - terminal: true

5. **Escalate to the on-call** - Page the rotation with the failing stage and the current expiry window, then stop.
   - tools: pushover
   - terminal: true
