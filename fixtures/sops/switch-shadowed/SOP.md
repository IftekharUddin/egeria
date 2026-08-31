# switch-shadowed

DELIBERATELY BROKEN. This fixture parses cleanly and is semantically wrong; do
not "fix" it. It exists so a switch-reachability rule has something to find.

Step 1's switch has four ports, and two of them can never be selected:

* `full` (port 2) carries a guard byte-identical to `canary` (port 1). Ports are
  evaluated top to bottom and the first match wins, so any payload that would
  satisfy `full` has already been claimed by `canary`. Port 2 is dead.
* `rollback` (port 4) sits after `any` (port 3), whose guard is empty. An empty
  guard is the catch-all and matches unconditionally, so evaluation never gets
  past port 3. Port 4 is dead, and the catch-all is not ordered last, which is
  where the format says it belongs.

Steps 3 and 5 are therefore unreachable through the switch. The rule that should
catch this is a switch-port reachability check: report a port whose guard is
subsumed by an earlier port's guard, and report any port ordered after a
catch-all.

Both defects are about port order, not about path resolution. Every port guard
is rooted at `$.steps.1`, which is the form a switch actually resolves — a
switch is evaluated against the run payload `{"steps": {"<n>": <output of step
n>}}` — so the guards themselves are well-formed and a reachability rule has to
reason about the ordering rather than dismiss every port as a dead path.

## Steps

1. **Select the rollout lane** - Read the requested stage off the deploy request.
   - tools: file_read
   - output: {"type":"object","required":["stage"],"properties":{"stage":{"type":"string"}}}
   - switch: canary>$.steps.1.stage == "canary">2; full>$.steps.1.stage == "canary">3; any>>4; rollback>$.steps.1.stage == "rollback">5

2. **Ship to the canary fleet** - Deploy to the ten percent canary slice and watch it.
   - tools: shell, http_request
   - allow-tools: shell
   - requires_confirmation: true
   - on_failure: goto: 5
   - terminal: true

3. **Ship to the full fleet** - Roll the release out to every remaining host.
   - tools: shell
   - requires_confirmation: true
   - on_failure: goto: 5
   - terminal: true

4. **Hold for a human** - Nothing matched a named lane, so ask before touching prod.
   - kind: checkpoint
   - policy: prod
   - prompt: No rollout lane matched. Approve a manual deploy?
   - edit: stage
   - terminal: true

5. **Roll back** - Restore the previously released build and page the on-call.
   - tools: shell, pushover
   - deny-tools: file_write
   - on_failure: fail
   - terminal: true
