# typed-contracts

Every step declares both the shape it accepts and the shape it produces, and
each step's `output:` is exactly the next step's `input:`. The chain is
therefore checkable without running anything: a producer whose output schema
stops matching its consumer's input schema is a static defect, not a runtime
surprise.

This fixture is the well-typed case. Its sibling `schema-mismatch` is the same
pipeline with two links deliberately broken.

## Steps

1. **Read the lock file** — Load the workspace lock file and list every resolved crate.
   - tools: file_read
   - input: {"type":"object","required":["workspace"],"properties":{"workspace":{"type":"string"}}}
   - output: {"type":"object","required":["packages"],"properties":{"packages":{"type":"array","items":{"type":"object","required":["name","version"],"properties":{"name":{"type":"string"},"version":{"type":"string"}}}}}}
   - next: 2

2. **Query the advisory database** — Ask the advisory service about each resolved crate.
   - tools: http_request
   - input: {"type":"object","required":["packages"],"properties":{"packages":{"type":"array","items":{"type":"object","required":["name","version"],"properties":{"name":{"type":"string"},"version":{"type":"string"}}}}}}
   - output: {"type":"object","required":["advisories"],"properties":{"advisories":{"type":"array","items":{"type":"object","required":["id","package","severity"],"properties":{"id":{"type":"string"},"package":{"type":"string"},"severity":{"type":"string"}}}}}}
   - depends_on: 1
   - on_failure: retry:2
   - next: 3

3. **Rank by severity** — Count the critical hits and render a plain-text report.
   - tools: calculator
   - input: {"type":"object","required":["advisories"],"properties":{"advisories":{"type":"array","items":{"type":"object","required":["id","package","severity"],"properties":{"id":{"type":"string"},"package":{"type":"string"},"severity":{"type":"string"}}}}}}
   - output: {"type":"object","required":["critical_count","report"],"properties":{"critical_count":{"type":"integer"},"report":{"type":"string"}}}
   - depends_on: 2
   - next: 4

4. **Store the report** — Keep the rendered report so tomorrow's run can diff against it.
   - tools: memory_store
   - input: {"type":"object","required":["critical_count","report"],"properties":{"critical_count":{"type":"integer"},"report":{"type":"string"}}}
   - output: {"type":"object","required":["key"],"properties":{"key":{"type":"string"}}}
   - depends_on: 3
   - next: 5

5. **Page on critical findings** — Notify the on-call rotation, but only when something critical landed.
   - tools: pushover
   - input: {"type":"object","required":["key"],"properties":{"key":{"type":"string"}}}
   - output: {"type":"object","required":["delivered"],"properties":{"delivered":{"type":"boolean"}}}
   - when: $.steps.3.critical_count > 0
   - depends_on: 4
   - terminal: true
