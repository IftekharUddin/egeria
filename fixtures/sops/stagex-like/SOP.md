# stagex-like

A release-engineering pipeline of the kind that actually runs unattended: an
upstream version announcement arrives on AMQP, and eight deterministic steps
turn it into a reviewable pull request. Because `deterministic = true`, each
step's output is piped straight into the next with no model in the loop, so
every step declares what it consumes and what it produces.

The fan-out is real: the patch refresh depends only on the bump, the digest
depends only on the build, and the commit waits for both.

## Steps

1. **Resolve** - Ask the release feed for the newest upstream version of the package named in the trigger payload.
   - tools: http_request
   - output: {"type": "object", "required": ["package", "version", "tarball_url"]}
   - on_failure: retry: 3

2. **Bump** - Rewrite the package Makefile and lockfile with the resolved version.
   - allow-tools: file_read, file_write
   - deny-tools: shell, git_operations
   - input: {"type": "object", "required": ["package", "version"]}
   - output: {"type": "object", "required": ["package", "version", "changed_files"]}
   - depends_on: 1

3. **Build** - Build the package reproducibly in its container and keep the log.
   - allow-tools: shell
   - output: {"type": "object", "required": ["artifact_path", "build_log"]}
   - depends_on: 2
   - on_failure: retry: 1

4. **Patch** - Refresh the patch series against the new source tree and drop any patch that upstream has taken.
   - tools: file_read, file_write, shell
   - output: {"type": "object", "required": ["applied", "dropped"]}
   - depends_on: 2

5. **Digest** - Compute the sha256 of the built artifact and check it against the reproducibility baseline.
   - tools: shell, calculator
   - input: {"type": "object", "required": ["artifact_path"]}
   - output: {"type": "object", "required": ["sha256", "reproducible"]}
   - depends_on: 3

6. **Commit** - Stage the bump, the refreshed patches, and the recorded digest on a release branch.
   - allow-tools: git_operations
   - deny-tools: http_request
   - requires_confirmation: true
   - depends_on: 4, 5

7. **Open PR** - Push the release branch and open the pull request against main.
   - tools: git_operations, http_request
   - output: {"type": "object", "required": ["pr_number", "pr_url"]}
   - depends_on: 6
   - on_failure: retry: 2

8. **Announce** - Post the release note to the announcements room.
   - kind: capability
   - capability: notify.channel
   - with: {"channel": "matrix.announce", "message": "package bumped; pull request open for review"}
   - depends_on: 7
   - terminal: true
