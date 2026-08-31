# admission-hold

The settings in the manifest only make sense together. `max_concurrent = 1`
keeps a second migration from running while the first is mid-apply;
`admission_policy = "hold"` queues those triggers instead of dropping them; and
because a run parked at the approval gate in step 3 releases its execution slot,
`max_pending_approvals = 3` is what actually bounds the queue. The
fifteen-minute cooldown stops a retried Slack message from replaying the whole
migration.

## Steps

1. **Snapshot** - Take a logical backup of the target database before touching the schema.
   - allow-tools: shell
   - deny-tools: file_write
   - requires_confirmation: true
   - on_failure: retry: 1

2. **Plan** - Render the pending migration as the exact statements that will run.
   - tools: file_read, shell
   - output: {"type": "object", "required": ["statements"]}
   - depends_on: 1

3. **Approve** - Hold here until a release approver signs off on the statements.
   - kind: checkpoint
   - policy: prod
   - prompt: Apply this migration to production?
   - edit: statements
   - depends_on: 2

4. **Apply** - Run the approved statements against production.
   - allow-tools: shell
   - deny-tools: file_write, git_operations
   - on_failure: goto: 5
   - depends_on: 3
   - terminal: true

5. **Rollback** - Restore the snapshot and page the on-call engineer.
   - tools: shell, pushover
   - terminal: true
