# triggers-many

Four fan-in sources feed the same intake procedure. The webhook and the
filesystem watcher carry the artifact manifest directly; the cron sweep and the
forge release event have to go fetch it.

## Steps

1. **Collect** - Read the artifact manifest that announced this run.
   - tools: file_read, http_request
   - output: {"type": "object", "required": ["artifact", "sha256"]}

2. **Verify** - Recompute the checksum of the downloaded artifact and compare it against the manifest.
   - allow-tools: shell
   - deny-tools: git_operations
   - on_failure: retry: 2
   - depends_on: 1

3. **Record** - Store the verified digest so later releases can diff against it.
   - tools: memory_store
   - depends_on: 2

4. **Announce** - Tell the release channel that the artifact landed and verified.
   - tools: pushover
   - depends_on: 3
   - terminal: true
