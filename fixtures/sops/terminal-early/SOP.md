# terminal-early

Step 2 carries `terminal: true` and sits second in a five-step list. The
duplicate arm is the common case, so ending there keeps the cheap path cheap —
but it also means nothing ever falls through from 2 into 3. Steps 3, 4 and 5 are
reachable only through the switch ports on step 1 and the explicit `next: 5` on
step 3. Linear order stops carrying the control flow after step 2.

Both port guards are rooted at `$.steps.1` and use `==` only. Guards see the run
payload `{"steps": {"<n>": <output of step n>}}` rather than the trigger event, so
a bare `$.classification` would never resolve and only the catch-all would ever
fire. `>` and `<` are unusable inside a port guard at all: `switch:` splits each
segment on `>` with `splitn(3, '>')`, so a comparison operator would be swallowed
and the port's target silently lost. The `condition` on the AMQP trigger in
`SOP.toml` is the other case — it does see the event payload.

## Steps

1. **Classify the alert** - Match the incoming alert against the open incidents and the recent-duplicate window.
   - tools: memory_recall
   - output: {"type":"object","required":["classification","fingerprint"]}
   - switch: duplicate>$.steps.1.classification == "duplicate">2; incident>$.steps.1.classification == "incident">3; defer>>4

2. **Close as a duplicate** - Bump the occurrence count on the existing incident and stop; there is nothing to page about.
   - tools: memory_store
   - input: {"type":"object","required":["fingerprint"]}
   - terminal: true

3. **Page the on-call** - Send the page with the alert body and the incident link.
   - tools: pushover
   - input: {"type":"object","required":["classification","fingerprint"]}
   - output: {"type":"object","required":["paged_at"]}
   - next: 5
   - on_failure: retry:2

4. **File for morning triage** - Park the alert on the triage queue for the next working day and stop.
   - tools: memory_store
   - terminal: true

5. **Record the page** - Store who was paged and when, so a repeat inside the window classifies as a duplicate.
   - tools: memory_store
   - depends_on: 3
   - terminal: true
