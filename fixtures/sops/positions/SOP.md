# positions

Coordinates attach by step number, and markdown steps are renumbered
positionally from 1 — so the steps below are numbered 1 through 4 and every
`[[positions]]` entry finds its node. Two of the four sit at negative
coordinates, which the canvas allows and the manifest stores verbatim. Moving
any of these nodes must not change what the procedure means.

## Steps

1. **Gather** - Pull last night's error lines out of the service logs.
   - tools: file_read, shell
   - output: {"type": "object", "required": ["lines"]}

2. **Summarize** - Group the errors by service and count them.
   - tools: calculator
   - depends_on: 1

3. **Search** - Look up known issues for whichever service regressed the most.
   - tools: web_search_tool, memory_recall
   - depends_on: 2
   - on_failure: retry: 2

4. **File** - Store the digest and page the on-call engineer if anything regressed.
   - tools: memory_store, pushover
   - depends_on: 3
   - terminal: true
