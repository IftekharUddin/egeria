# schema-mismatch

DELIBERATELY BROKEN FIXTURE. This file parses cleanly and is meant to; the
defects are semantic and belong to a later verifier milestone, not to the
parser.

Two links in the chain are wrong, and neither is detectable by reading a single
step in isolation:

- Container mismatch. The collector emits a bare JSON array of alerts. Its
  consumer declares an object with an `alerts` property, so the value it
  actually receives is not the value it says it accepts.
- Scalar mismatch. The rollup emits `window_minutes` as an integer. The
  formatter downstream declares `window_minutes` as a string.

Nothing upstream compares a producer's `output:` against a consumer's `input:`,
so both survive load, both survive a save round trip, and both only surface as a
runtime type error against real data.

The well-typed twin of this fixture is `typed-contracts`, where every producer's
`output:` is byte-identical to its consumer's `input:`.

## Steps

1. **Collect firing alerts** — Pull the current firing set from the monitor.
   Emits a bare array. This is the producer half of the container mismatch.
   - tools: http_request
   - input: {"type":"object","required":["monitor_url"],"properties":{"monitor_url":{"type":"string"}}}
   - output: {"type":"array","items":{"type":"object","required":["alert_id","service","severity"],"properties":{"alert_id":{"type":"string"},"service":{"type":"string"},"severity":{"type":"string"}}}}
   - next: 2

2. **Group alerts by service** — Fold the firing set into one entry per service.
   Consumer half of the container mismatch: this input declares an object whose
   `alerts` property holds the array, but the previous step hands over the array
   itself with no wrapper object around it.
   - tools: calculator
   - input: {"type":"object","required":["alerts"],"properties":{"alerts":{"type":"array","items":{"type":"object","required":["alert_id","service","severity"],"properties":{"alert_id":{"type":"string"},"service":{"type":"string"},"severity":{"type":"string"}}}}}}
   - output: {"type":"object","required":["groups","window_minutes"],"properties":{"groups":{"type":"array","items":{"type":"object","required":["service","count"],"properties":{"service":{"type":"string"},"count":{"type":"integer"}}}},"window_minutes":{"type":"integer"}}}
   - depends_on: 1
   - on_failure: retry:2
   - next: 3

3. **Format the digest** — Render the grouped counts as a message body.
   Consumer half of the scalar mismatch: `window_minutes` arrives as an integer
   and is declared here as a string.
   - tools: file_write
   - input: {"type":"object","required":["groups","window_minutes"],"properties":{"groups":{"type":"array","items":{"type":"object","required":["service","count"],"properties":{"service":{"type":"string"},"count":{"type":"integer"}}}},"window_minutes":{"type":"string"}}}
   - output: {"type":"object","required":["digest"],"properties":{"digest":{"type":"string"}}}
   - depends_on: 2
   - next: 4

4. **Send the digest** — Deliver the rendered digest to the on-call rotation.
   This link is well typed; only steps one through three carry the defects.
   - tools: pushover
   - input: {"type":"object","required":["digest"],"properties":{"digest":{"type":"string"}}}
   - output: {"type":"object","required":["delivered"],"properties":{"delivered":{"type":"boolean"}}}
   - depends_on: 3
   - terminal: true
