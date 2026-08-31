# mode-overrides

The manifest carries both `deterministic = true` and `execution_mode =
"step_by_step"`. Those are not equal partners: `deterministic` is a hard
override, so the effective mode is deterministic and the authored
`execution_mode` is discarded with no diagnostic. Both values survive on disk,
which is exactly why the conflict is easy to miss — reading the manifest suggests
a step-by-step run that will never happen.

Per-step `mode:` overrides the resolved procedure mode for that step alone. An
unrecognized value does not clear the override, it silently becomes
`supervised`, so every value below is spelled deliberately.

`agent:` works the same way one level down: the manifest names the parent alias
that owns the procedure, and a step's own `agent:` replaces it for that step
only. Steps without one inherit `mirror-bot`.

## Steps

1. **Inventory the incoming directory** — List the batches waiting to be mirrored.
   No agent override, so this runs as the procedure's own alias.
   - tools: file_read
   - mode: deterministic
   - output: {"type":"object","required":["batches"],"properties":{"batches":{"type":"array","items":{"type":"string"}}}}
   - next: 2

2. **Verify checksums** — Check every archive against its signed manifest.
   - tools: shell
   - allow-tools: shell
   - mode: step_by_step
   - agent: verify-bot
   - depends_on: 1
   - on_failure: retry:3
   - next: 3

3. **Approve the publish** — Hold the batch until a human releases it.
   A checkpoint under a supervised override, run by the release alias rather
   than the mirror alias that owns the rest of the procedure.
   - kind: checkpoint
   - mode: supervised
   - agent: release-bot
   - requires_confirmation: true
   - policy: prod
   - prompt: Publish the verified mirror batch?
   - depends_on: 2
   - next: 4

4. **Sync to the public mirror** — Push the approved batch outward.
   A push that fails still has to be recorded, so the failure route lands on the
   same bookkeeping step the success route does.
   - tools: shell
   - allow-tools: shell
   - deny-tools: file_write
   - mode: auto
   - agent: mirror-bot
   - depends_on: 3
   - on_failure: goto: 5
   - next: 5

5. **Record the outcome** — Remember what shipped, or what stalled, so the next
   run picks up in the right place.
   - tools: memory_store
   - mode: priority_based
   - depends_on: 4
   - terminal: true
