# checkpoint-basic

Restarting the ingest workers drops in-flight batches, so the rendered config is
shown to an operator before the restart rather than after it.

## Steps

1. **Render the worker config** - Read the template and write the rendered config to the staging path so it can be diffed before it goes live.
   - tools: file_read, file_write
   - output: {"type": "object", "required": ["config_path", "diff_lines"]}

2. **Approve the restart** - Hold here until an operator has read the rendered config and confirmed the rollout should proceed.
   - kind: checkpoint
   - requires_confirmation: true
   - depends_on: 1

3. **Roll the worker pool** - Restart the ingest workers one host at a time, waiting for each to report healthy before moving on.
   - tools: shell, http_request
   - on_failure: retry: 2

4. **Record the rollout** - Store the config hash and the restart timestamp so the next run can tell what changed.
   - tools: memory_store
   - terminal: true
