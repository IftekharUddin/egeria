# trigger-conditions

Every trigger here carries a condition, which is the whole point: the broker
delivers far more traffic than this procedure should act on. Note that the
peripheral guard is the bare-comparison form, evaluated against the signal
value rather than a JSON path.

## Steps

1. **Sample** - Pull the last five pressure readings for the pump that raised the alarm.
   - tools: http_request
   - output: {"type": "object", "required": ["pump", "readings"]}
   - on_failure: retry: 2

2. **Classify** - Decide whether this is a momentary spike or a sustained overpressure.
   - tools: calculator, memory_recall
   - input: {"type": "object", "required": ["pump", "readings"]}
   - output: {"type": "object", "required": ["classification", "peak_psi"]}
   - depends_on: 1
   - switch: sustained>$.classification == "sustained">3; spike>>4

3. **Escalate** - Page the on-call engineer and open the isolation valve.
   - allow-tools: pushover, http_request
   - deny-tools: shell
   - requires_confirmation: true
   - terminal: true

4. **Log** - Record the spike for the weekly trend review and leave the pump alone.
   - tools: memory_store
   - terminal: true
