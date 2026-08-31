# linear-minimal

The smallest useful shape: one manual trigger and a straight line of steps.
Nothing here routes — no `next:`, no `when:`, no `switch:`, no `depends_on:` —
so execution is exactly file order and the run completes after the last step.

## Steps

1. **Read the staged changelog** - Load the release notes collected for this cycle.
   - tools: file_read

2. **Summarize the entries** - Condense the changelog into one operator-facing paragraph.
   - tools: llm_task

3. **Store the summary** - Keep the paragraph so later runs can recall it.
   - tools: memory_store
