# planned-calls

A planned call names the tool, the argument template, and optionally a pinned
sample of what the tool returned the last time it ran. The pinned sample is what
lets the rest of the procedure be authored and checked without paying for a real
invocation.

Argument templates carry `{{steps.N}}` bindings, which resolve against the
output of the numbered step. Numbering is positional, so a binding names the
step's position in this list rather than whatever digit an author typed.

`call:` is the only bullet key that accumulates. Every other key is
last-write-wins, so a second `tools:` replaces the first; a second `call:`
appends. Step two below carries two of them, in order.

## Steps

1. **Resolve the release tag** — Find the tag this release points at.
   - tools: git_operations
   - call: {"tool":"git_operations","args":{"operation":"describe","match":"v*"},"pinned":{"tag":"v0.7.3","commit":"9f2c1ab"}}
   - output: {"type":"object","required":["tag"],"properties":{"tag":{"type":"string"}}}
   - next: 2

2. **Gather commits and issues** — Two calls, both bound to the resolved tag.
   - tools: shell, http_request
   - call: {"tool":"shell","args":{"command":"git log --oneline {{steps.1}}..HEAD"}}
   - call: {"tool":"http_request","args":{"method":"GET","url":"https://forge.internal/api/repos/zeroclaw-labs/zeroclaw/issues","query":{"milestone":"{{steps.1}}","state":"closed"}},"pinned":{"status":200,"count":31}}
   - depends_on: 1
   - on_failure: retry:2
   - next: 3

3. **Draft the notes** — Turn the commit list into prose.
   The planned call caches the raw commit list next to the draft, so a reviewer
   who disputes the wording can regenerate it without re-running the log.
   - kind: capability
   - capability: llm.generate
   - with: {"instruction":"Write release notes grouped by area from the commit list and closed issues.","output_key":"body","echo":["tag"]}
   - call: {"tool":"file_write","args":{"path":".release/commits.md","content":"{{steps.2}}"},"pinned":{"bytes_written":4096}}
   - depends_on: 2
   - next: 4

4. **Approve the notes** — Read the draft before anything is published.
   - kind: checkpoint
   - requires_confirmation: true
   - policy: prod
   - prompt: Publish these release notes?
   - edit: body
   - depends_on: 3
   - next: 5

5. **Publish the notes** — Post the approved body and announce it.
   - kind: capability
   - capability: forge.comment
   - with: {"repo":"zeroclaw-labs/zeroclaw","number":812,"body":"{{steps.4}}","channel":"git.main"}
   - call: {"tool":"pushover","args":{"title":"Release notes posted","message":"{{steps.1}} notes are live"}}
   - depends_on: 4
   - terminal: true
