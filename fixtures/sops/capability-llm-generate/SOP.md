# capability-llm-generate

`llm.generate` is the one builtin capability that requires an authored `with:`,
and the object is validated against the capability's input schema when the SOP
loads. `instruction` is required; `system`, `output_key` and `echo` are not.
Step 1's output is piped in under the `input` key of that same object.

## Steps

1. **Collect the failing runs** - Query the forge for workflow runs that failed since the last digest.
   - tools: http_request, git_operations
   - output: {"type": "object", "required": ["repo", "run_ids", "logs"]}

2. **Summarize the failures** - Turn the raw run logs into one paragraph a human can act on.
   - kind: capability
   - capability: llm.generate
   - with: { instruction = "Summarize the failing CI runs and name the single most likely shared cause.", system = "You are a release engineer. Be terse and factual, and never invent a commit hash.", output_key = "digest", echo = ["repo", "run_ids"] }
   - depends_on: 1
   - on_failure: retry: 2

3. **File the digest** - Store the digest under the repository key so the morning triage run can recall it.
   - tools: memory_store, file_write
   - depends_on: 2
   - terminal: true
