# tool-scopes

Three keys govern what a step may reach, and they are not interchangeable.

`tools:` is the legacy spelling. It fills the allow-list only when no explicit
allow-list is present, so a step carrying both keeps them as separate fields and
the explicit one wins. `allow-tools:` sets the allow-list outright, and a
present-but-empty allow-list permits nothing at all, which is not the same as
having no allow-list. `deny-tools:` subtracts, and it materializes a scope even
on a step whose allow-list is still absent.

## Steps

1. **Check the working tree** — Refuse to cut from a dirty tree.
   Legacy spelling only, so the allow-list stays absent and is filled from the
   suggested list. The deny still materializes a scope around it.
   - tools: git_operations, file_read
   - deny-tools: shell
   - output: {"type":"object","required":["clean"],"properties":{"clean":{"type":"boolean"}}}
   - next: 2

2. **Read the changelog draft** — Load the pending release notes.
   Both spellings on one step. They stay distinct: the explicit allow-list is
   the one that binds, and the legacy list stays advisory because the allow-list
   it would have filled is already present.
   - tools: file_read, file_write, shell
   - allow-tools: file_read
   - deny-tools: shell, git_operations
   - depends_on: 1
   - next: 3

3. **Approve the cut** — Human sign-off before the branch exists.
   A present-but-empty allow-list: this gate runs with no tool surface at all.
   - kind: checkpoint
   - allow-tools:
   - requires_confirmation: true
   - policy: prod
   - prompt: Cut the release branch from the current head?
   - depends_on: 2
   - next: 4

4. **Create the release branch** — Branch and push.
   Written with the underscore aliases, which the parser accepts and the printer
   normalizes back to the hyphen spellings upstream emits. A push that loses the
   race jumps straight to the announcement, which reports either outcome.
   - allow_tools: git_operations, shell
   - deny_tools: file_write, http_request
   - on_failure: goto: 5
   - depends_on: 3
   - next: 5

5. **Announce the outcome** — Tell the release channel where the cut landed.
   The redundant case: the legacy list and the allow-list name the same tool.
   - tools: pushover
   - allow-tools: pushover
   - deny-tools: shell
   - depends_on: 4
   - terminal: true
