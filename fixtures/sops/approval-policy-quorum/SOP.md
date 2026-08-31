# approval-policy-quorum

The gate in step 3 names the `prod` policy, which the daemon config defines as a
quorum of two drawn from the `release` group. Nothing in this directory can
grant that quorum; the SOP only names the policy.

The `when:` guard sits on the last step rather than on the gate. A false guard
takes the linear successor, so guarding the checkpoint would route a failed
release straight into the publish step; guarding a `terminal: true` step ends
the run instead.

## Steps

1. **Collect the release diff** - Summarize what changed between the last published tag and the tag being released.
   - tools: git_operations, shell
   - allow-tools: git_operations, shell
   - output: {"type": "object", "required": ["tag", "commit_count", "changelog"]}

2. **Check the release gates** - Confirm the tag builds clean and that no advisory is open against a shipped crate.
   - tools: http_request, shell
   - depends_on: 1
   - output: {"type": "object", "required": ["ci_green", "advisories_open"]}
   - on_failure: fail

3. **Release sign-off** - Two members of the release group must approve before anything is published.
   - kind: checkpoint
   - policy: prod
   - depends_on: 1, 2

4. **Publish the release** - Push the artifacts and mark the tag published.
   - tools: shell, git_operations
   - agent: release-publisher
   - output: {"type": "object", "required": ["published", "artifact_count"]}
   - on_failure: retry: 1

5. **Announce the release** - Post the changelog to the release room, and stay quiet if the publish step shipped nothing.
   - tools: pushover, http_request
   - depends_on: 4
   - when: $.published == "true"
   - terminal: true
