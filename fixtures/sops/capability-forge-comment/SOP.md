# capability-forge-comment

The headless review shape: draft, gate, post. `kind:` and `capability:` are
sub-bullets on every capability step. Written on the title line — the form the
upstream docs show — they are swallowed into the step body, the step stays an
execute step, and the comment is posted without ever having been drafted.

`forge.comment` fails closed unless a checkpoint ran before it, so step 2 is
load-bearing rather than decorative.

Because step 3 authors a `with:` object, the piped value from step 2 arrives
nested under its `input` key, and `forge.comment` reads `number` and `body` from
there. `repo` and `channel` are pinned at the top level so the bot can only ever
post into its own repository; `number` rides through on step 1's `echo` list.

## Steps

1. **Draft the triage comment** - Read the issue and write the comment body a maintainer would post.
   - kind: capability
   - capability: llm.generate
   - with: {"instruction": "Summarize the issue, name the component it belongs to, and propose a single triage label.", "output_key": "body", "echo": ["repo", "number"]}
   - on_failure: retry: 2

2. **Approve the comment** - A maintainer reads the drafted comment and amends the wording before it is posted under the bot account.
   - kind: checkpoint
   - policy: triage
   - edit: body
   - depends_on: 1

3. **Post the comment** - Post the approved body to the issue thread.
   - kind: capability
   - capability: forge.comment
   - with: {"repo": "zeroclaw-labs/zeroclaw", "channel": "git.main"}
   - depends_on: 2
   - on_failure: fail
   - terminal: true
