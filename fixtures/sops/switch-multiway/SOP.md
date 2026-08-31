# switch-multiway

A four-port dispatch: three guarded ports and a catch-all whose guard is empty,
ordered last as the format requires. Ports are evaluated top to bottom and the
first match wins, so the catch-all only fires for an event type none of the
named ports claimed.

Worth knowing while reading step 1: a non-empty `switch:` suppresses `next:` and
the linear successor entirely, and if no port matches the run simply completes.
That is why the catch-all is not optional here — without it an unrecognized
event type would end the run silently instead of reaching the fallback.

Every port guard is rooted at `$.steps.1`, because a switch is evaluated against
the run payload `{"steps": {"<n>": <output of step n>}}` rather than against the
raw trigger event. A bare `$.event_type` would fail to resolve, every port would
be false, and the catch-all would be the only reachable arm. The `condition` on
the trigger in `SOP.toml` is the other case — it does see the event payload, so
`$.event_type != "push"` is correct there.

Guards stay on `==`. A `>` or `<` inside a port guard would be eaten by the
`name>when>goto` split, so numeric comparisons have to be done in a step rather
than in a port.

## Steps

1. **Read the forge event** - Classify the inbound webhook payload by event type.
   - tools: http_request
   - output: {"type":"object","required":["event_type"],"properties":{"event_type":{"type":"string"}}}
   - switch: opened>$.steps.1.event_type == "pull_request.opened">2; merged>$.steps.1.event_type == "pull_request.merged">3; review>$.steps.1.event_type == "pull_request_review_comment.created">4; other>>5

2. **Greet the contributor** - Post the review checklist on a newly opened pull request.
   - kind: capability
   - capability: forge.comment
   - with: {"repo":"zeroclaw-labs/zeroclaw","number":41,"body":"Thanks for the pull request. A reviewer will pick this up shortly.","channel":"git.main"}
   - terminal: true

3. **Cut the changelog entry** - Append the merged pull request to the release notes.
   - tools: file_read, file_write, git_operations
   - allow-tools: file_write
   - on_failure: retry: 3
   - terminal: true

4. **Route the review comment** - Notify the author that a reviewer left a note.
   - kind: capability
   - capability: notify.channel
   - with: {"channel":"discord.ops","message":"New review comment on the release branch."}
   - terminal: true

5. **Record the unhandled event** - Keep the payload so the dispatch table can be widened later.
   - tools: memory_store
   - agent: triage-bot
   - terminal: true
