# when-guard-jump

The alert-triage shape from the routing docs. Step 1 carries both a `when:`
guard and a `next:` jump: the guard decides whether the jump is taken at all.
When `$.steps.1.severity == "critical"` holds the run jumps to step 3 and
remediates; when it does not, the guard is false and the run falls through to
the linear successor, step 2, which logs the alert and ends the run.

A step guard is an out-edge guard on the step that already ran, and it is
evaluated against the run payload `{"steps": {"<n>": <output of step n>}}` —
never against the raw trigger event. That is why every `when:` here is rooted at
`$.steps.<n>`: a bare `$.severity` would fail to resolve and the guard would be
false on every run. The trigger `condition` in `SOP.toml` is the opposite case —
it sees the event payload itself, so `$.value > 85` is correct there.

Step 6 shows the other half of the false-guard rule. Its guard sits on a
`terminal: true` step, so a false guard completes the run instead of falling
through to a successor.

## Steps

1. **Classify the alert** - Inspect the incoming payload and label its severity.
   - tools: http_request
   - output: {"type":"object","required":["severity"],"properties":{"severity":{"type":"string"}}}
   - when: $.steps.1.severity == "critical"
   - next: 3

2. **Record and close** - File a non-critical reading and stop without paging anyone.
   - tools: memory_store
   - terminal: true

3. **Prepare the remediation plan** - Build the operator-facing plan for the valve change.
   - tools: file_read, calculator
   - depends_on: 1
   - on_failure: retry: 2
   - next: 4

4. **Apply the remediation** - Run the approved command against the pump controller.
   - allow-tools: shell
   - deny-tools: file_write
   - requires_confirmation: true
   - on_failure: goto: 5
   - next: 6

5. **Page the operator** - Send a failure notice so a human picks the alert up.
   - tools: pushover
   - terminal: true

6. **Confirm the pressure dropped** - Re-read the sensor and close the incident.
   - tools: http_request
   - output: {"type":"object","required":["pressure_ok"],"properties":{"pressure_ok":{"type":"boolean"}}}
   - when: $.steps.6.pressure_ok == "true"
   - terminal: true
