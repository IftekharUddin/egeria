# retry-bounded

The status-page API drops roughly one request in twenty under load, and a
dropped publish is not worth paging anyone over. Step 3 is the only step that
talks to it, and it carries `on_failure: retry:2` — three attempts total, then
the run fails. Nothing here escalates on its own; the bound is the whole point.

Step 2 skips the publish entirely when the computed level matches what the last
run stored. A step guard is an out-edge guard evaluated after the step ran, so
it only does work alongside a `next:` or a `switch:` — here `when:` plus
`next: 4` jumps past the flaky call, and a false guard falls through to step 3
and publishes. The guard is rooted at `$.steps.2` because guards see the run
payload `{"steps": {"<n>": <output of step n>}}`, never the trigger event; the
`condition` on the webhook trigger would be the other case, which is why there
is none on it — webhook and cron are the two variants with no `condition` field.

## Steps

1. **Collect the health snapshot** - Read the rollout metrics the deploy job left on disk.
   - tools: file_read
   - output: {"type":"object","required":["version","error_rate","p99_ms"]}

2. **Decide what to publish** - Map the raw metrics onto one of the three public status levels.
   - tools: calculator
   - input: {"type":"object","required":["error_rate","p99_ms"]}
   - output: {"type":"object","required":["level","summary","unchanged"]}
   - when: $.steps.2.unchanged == "true"
   - next: 4

3. **Push to the status page** - POST the component update to the status API, which is intermittently unavailable.
   - allow-tools: http_request
   - deny-tools: shell, file_write
   - input: {"type":"object","required":["level","summary"]}
   - output: {"type":"object","required":["incident_id"]}
   - on_failure: retry:2

4. **Record the decided level** - Keep the level the run settled on so the next run can suppress a duplicate update.
   - tools: memory_store
   - input: {"type":"object","required":["level"]}
   - depends_on: 2
   - terminal: true
