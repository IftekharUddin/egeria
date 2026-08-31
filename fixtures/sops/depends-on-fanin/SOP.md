# depends-on-fanin

The audit pins a commit, then runs two evidence gatherers that do not need each
other: an offline lockfile scan and an online advisory fetch. Neither reads the
other's output; both declare `depends_on: 1`, and the report step joins them
with `depends_on: 2, 3`. The dependency graph is therefore a diamond rather than
a chain.

Step 3 deliberately spells its bullets `depends-on:` and `on-failure:` while step 2 uses `depends_on:` and `on_failure:`. Both spellings are accepted and neither is documented upstream, so the mixture is intentional — do not normalize it.

The diamond is a constraint, not a schedule. A run walks one cursor in file
order, so step 2 does run before step 3 — what `depends_on` adds is the rule
that step 4 parks unless *both* arms recorded a Completed output, and that
nothing but the join orders 2 against 3.

## Steps

1. **Pin the revision** - Record the commit the audit ran against so the report is reproducible.
   - tools: git_operations
   - output: {"type":"object","required":["commit","branch"]}

2. **Scan the lockfile** - Run the offline advisory scan over Cargo.lock and emit the matched advisories.
   - allow-tools: shell
   - deny-tools: http_request
   - input: {"type":"object","required":["commit"]}
   - output: {"type":"object","required":["advisories"]}
   - depends_on: 1

3. **Fetch the advisory feed** - Pull the published advisory index for the same day from the RustSec mirror.
   - tools: http_request
   - input: {"type":"object","required":["commit"]}
   - output: {"type":"object","required":["feed"]}
   - depends-on: 1
   - on-failure: retry:1

4. **Reconcile both sources** - Join the local scan against the published feed and flag anything present in one but not the other.
   - tools: calculator, memory_recall
   - input: {"type":"object","required":["advisories","feed"]}
   - output: {"type":"object","required":["unmatched","total"]}
   - depends_on: 2, 3

5. **File the report** - Store the reconciled audit under the nightly key and notify the release channel.
   - tools: memory_store, pushover
   - depends_on: 4
   - terminal: true
