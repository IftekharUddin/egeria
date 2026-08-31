# approval-edit-gate

The status page is customer-facing, so the gate does more than say yes or no:
`edit: body` lets the incident commander rewrite the drafted text in place, and
the run resumes with the amended value rather than the drafted one.

## Steps

1. **Gather the incident facts** - Pull the current alert state and the affected component list from the incident record.
   - tools: http_request, memory_recall
   - output: {"type": "object", "required": ["incident_id", "component", "started_at"]}

2. **Draft the status update** - Compose a short customer-facing update from the incident facts.
   - tools: file_read, file_write
   - depends_on: 1
   - output: {"type": "object", "required": ["body"]}

3. **Commander sign-off** - The incident commander approves the wording, amending it in place if the draft overstates or understates impact.
   - kind: checkpoint
   - prompt: Read the drafted update as a customer would. Correct the wording if it overstates or understates impact, then approve.
   - edit: body
   - policy: incident
   - depends_on: 2

4. **Publish to the status page** - Post the approved body to the status page and pin it to the open incident.
   - tools: http_request
   - on_failure: retry: 3

5. **Notify the on-call channel** - Tell the on-call room what was published so nobody posts a second update.
   - tools: pushover
   - terminal: true
