# ZeroClaw SOP format: implementation reference

**What this is.** A derived reference for the SOP file format, assembled by
reading ZeroClaw's source at the pinned tag `v0.8.4` (`a56c345`). It exists
because the prose documentation upstream is incomplete in ways that matter:
`docs/book/src/sop/syntax.md` never mentions `switch:` or `terminal:` at all,
and it contradicts itself about what a false `when:` guard does. Building the
adapter against the docs alone would have produced a parser that is wrong about
control flow, which is the one thing Egeria's security rules depend on.

**How to read it.** *The Rust source is authoritative; this document is a
convenience.* Every claim carries a `file:line` citation into
`external/zeroclaw/` — follow it rather than trusting the summary when
something matters. Where the docs and the source disagree, the source wins and
the disagreement is called out explicitly; §14 of the routing section carries a
consolidated register of those conflicts.

Citation shorthand used throughout, relative to `external/zeroclaw/`:

| Short | Path |
|---|---|
| `TYPES` | `crates/zeroclaw-runtime/src/sop/types.rs` |
| `MOD` | `crates/zeroclaw-runtime/src/sop/mod.rs` |
| `CONTRACT` / `step_contract.rs` | `crates/zeroclaw-runtime/src/sop/step_contract.rs` |
| `ROUTE` | `crates/zeroclaw-runtime/src/sop/route/mod.rs` |
| `SYNTAX` | `docs/book/src/sop/syntax.md` |

**Provenance and staleness.** Derived by a multi-agent read of the pinned
source, with each section independently cross-checked against that source by a
second reader. It describes ZeroClaw at `v0.8.4` and nothing else. Moving the
submodule pin without re-deriving this document makes it silently wrong, which
is one of the reasons moving a pin is a human decision (ADR-0005).

**Open questions.** Each section ends with points genuinely undecidable from the
source, each carrying an interim instruction. Those are the places where an
implementer should escalate rather than guess.

---

# Part: Manifest — `SOP.toml`

## SOP.toml — Manifest Specification (corrected & completed)

**Authority:** `crates/zeroclaw-runtime/src/sop/types.rs` and `crates/zeroclaw-runtime/src/sop/mod.rs`. All paths are relative to `/Users/hollow/Documents/Workflow Generator/egeria/external/zeroclaw/`.

Shorthand:
- **TYPES** = `crates/zeroclaw-runtime/src/sop/types.rs`
- **MOD** = `crates/zeroclaw-runtime/src/sop/mod.rs`
- **TRIGSRC** = `crates/zeroclaw-runtime/src/sop/trigger_source.rs`
- **ENGINE** = `crates/zeroclaw-runtime/src/sop/engine.rs`
- **COND** = `crates/zeroclaw-runtime/src/sop/condition.rs`
- **PROCMEM** = `crates/zeroclaw-runtime/src/sop/procedural_memory.rs`
- **CFG** = `crates/zeroclaw-config/src/schema.rs`
- **CLI** = `src/sop/mod.rs` (binary crate; the cited hits are all `#[cfg(test)]`)
- **SYNTAX** = `docs/book/src/sop/syntax.md`

Corrections against the prior draft are marked **[CORRECTION]**; net-new material is marked **[ADDED]**.

---

### 0. Doc-vs-source status for this area (read first)

SYNTAX:16-20:

> ## 2. Authoring Boundary
> The file-backed representation still contains a manifest file plus `SOP.md`.
> This page intentionally does not enumerate manifest fields or provide
> hand-authored manifest examples.

**[CORRECTION] The draft's "syntax.md deliberately does not document the manifest at all" is too strong on two counts:**

1. SYNTAX:27 *does* name the identity triple: "`SOP.toml` carries the SOP's identity (`name`, `description`, `version`), its `triggers`, and its execution knobs." SYNTAX:31-35 tabulates `max_concurrent`, `admission_policy`, `max_pending_approvals` with defaults; SYNTAX:37-49 enumerates the `admission_policy` values; SYNTAX:64-75 gives a worked `[sop]` + `[[triggers]]` block.
2. **Trigger fields are documented**, machine-generated, in syntax.md itself — SYNTAX:342-344 is `## 4. Trigger Types` followed by `{{#sop-trigger-index}}`. That table is a projection of `SopTrigger` (see §4.1), so it is accurate by construction.

**The actual doc void, verified by grep over `docs/book/src/sop/syntax.md`:**

| Manifest key | occurrences in syntax.md |
|---|---|
| `priority` | **0** |
| `execution_mode` | **0** |
| `cooldown_secs` | **0** |
| `positions` | **0** |
| `[[steps]]` | **0** |
| `deterministic` | 5 — all prose about deterministic *mode* (SYNTAX:161, 218, 315, 334), never as a `[sop]` key |
| `agent` | 7 — all about approver identities (SYNTAX:83, 93) or the step-level agent, never as a `[sop]` key |

So five `[sop]` keys (`priority`, `execution_mode`, `cooldown_secs`, `deterministic`, `agent`) plus both array tables (`positions`, `steps`) have no prose reference anywhere. This spec is their only reference.

**[ADDED] The manifest types are excluded from schema export.** `SopTrigger` (TYPES:125), `StepPos` (TYPES:302), `SopStep` (TYPES:310), `SopPriority` (TYPES:13), `SopExecutionMode` (TYPES:38), `SopAdmissionPolicy` (TYPES:529) and `FilesystemEventKind` (TYPES:83) all carry `#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]`. `SopManifest` (TYPES:583), `SopMeta` (TYPES:606) and `StepPosition` (TYPES:598) **do not**. There is therefore no upstream JSON Schema for the manifest envelope — Egeria cannot generate its model from an exported schema and must transcribe the Rust structs.

**Egeria consequence:** the parser must be derived from `SopManifest`/`SopMeta`/`SopTrigger`/`StepPosition` as specified below.

---

### 1. Parse pipeline — the exact sequence Egeria must reproduce

There is exactly **one** deserialization site for `SOP.toml`:

```
MOD:425:    let manifest: SopManifest = toml::from_str(&toml_content)?;
```

Exhaustive `grep -rn "SopManifest" --include="*.rs" .` returns only: TYPES:584 (definition), TYPES:635 (`impl`), TYPES:1371 (test), MOD:66 (import), MOD:425 (the parse), MOD:966 (the writer), CLI:245 (test import), CLI:555 (test).

Load sequence:

0. **[ADDED] Root resolution** — `resolve_sops_dir` (MOD:194-202): a non-empty `sop.sops_dir` is tilde-expanded then `workspace_dir.join(...)`; an absolute value replaces the base. Empty/absent ⇒ `<workspace>/sops` (MOD:183-186).
1. **[ADDED] Early bail-outs** — `load_sops_from_directory` returns an empty `Vec` if the directory does not exist (MOD:382-384) or `read_dir` fails (MOD:388-390). `entries.flatten()` (MOD:392) silently skips unreadable entries.
2. **Discovery** (MOD:392-401): skip non-directories (MOD:394-396); skip any directory whose `SOP.toml` does not exist (MOD:398-401). `SOP.toml` is the *marker file* — a directory without one is invisible, not an error.
   **[ADDED] Discovery is exactly one level deep and non-recursive.** Consequence: `sops/.rollback/<proposal_id>/{SOP.toml,SOP.md}`, written by the procedural-memory rollback path (PROCMEM:346-355), is *not* loaded, because `sops/.rollback/` itself contains no `SOP.toml`. Egeria's discovery must match: one level, `SOP.toml` present, no recursion.
   `path.is_dir()` (MOD:394) follows symlinks — a symlinked directory is discovered.
3. **Read + parse** (MOD:423-425).
4. **Steps source selection** (MOD:427-435) — see §8:
   ```rust
   let md_path = sop_dir.join("SOP.md");
   let mut steps = if md_path.exists() {
       let md_content = std::fs::read_to_string(&md_path)?;
       parse_steps(&md_content)
   } else if !manifest.steps.is_empty() {
       normalize_manifest_steps(manifest.steps)
   } else {
       Vec::new()
   };
   ```
5. **Position merge** (MOD:437-441) — see §6.
6. **Destructure `[sop]`** (MOD:442-454).
7. **`deterministic` override** (MOD:456-461) — see §7.2.
8. **Construct runtime `Sop`** (MOD:463-478).
9. **Capability validation** (MOD:479): `capability::SopCapabilityRegistry::with_builtins().validate_sop(&sop)?` — the only *semantic* hard error at load time. **[CORRECTION] cite `capability/registry.rs:39-68`** (the draft said 40-70): it loops `sop.steps`, skips every step whose `kind != Capability` (registry.rs:40-43), then requires a `capability` id (registry.rs:44-49), a registered capability (registry.rs:50-55), and — when `requires_authored_input()` — a schema-valid authored `with` (registry.rs:56-66). It never inspects the manifest envelope.
10. **Failure handling** (MOD:403-414): any `Err` from `load_sop` — TOML syntax error, missing required field, unknown enum value, unknown capability — is **logged at WARN and the SOP is silently skipped**. Loading is never fatal.
11. **Sort** (MOD:417): `sops.sort_by(|a, b| a.name.cmp(&b.name))` — by manifest `name`, not directory name. Byte-wise `String` ordering, not locale-aware.

`load_sop_by_name` (MOD:235-241) is the strict single-SOP path and *does* propagate the error.

---

### 2. Top-level manifest shape

TYPES:582-595, verbatim:

```rust
/// Top-level SOP.toml structure.
##[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SopManifest {
    pub sop: SopMeta,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub triggers: Vec<SopTrigger>,
    /// Persisted canvas coordinates per step. Written by the Blueprint editor,
    /// kept out of SOP.md so step prose stays position-free. Merged back onto
    /// `SopStep::pos` at load time.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub positions: Vec<StepPosition>,
    #[serde(default)]
    pub steps: Vec<SopStep>,
}
```

| TOML key | Rust type | Required | Default | Notes |
|---|---|---|---|---|
| `[sop]` | `SopMeta` | **yes** | — | No `#[serde(default)]` (TYPES:585). Absent ⇒ `missing field \`sop\``. |
| `[[triggers]]` | `Vec<SopTrigger>` | no | `[]` | TYPES:586-587 |
| `[[positions]]` | `Vec<StepPosition>` | no | `[]` | TYPES:591-592 |
| `[[steps]]` | `Vec<SopStep>` | no | `[]` | TYPES:593-594. **Fallback only** — §8. |

**No `deny_unknown_fields`, no `rename`, no `flatten`, no `alias` anywhere in the manifest types.** Verified:
- `grep -rn "deny_unknown_fields" crates/zeroclaw-runtime/src/sop/` → **0 matches** (repo-wide there are 11, none under `sop/`).
- `grep -rn "serde(alias" crates/zeroclaw-runtime/src/sop/` → **0 matches**. No key has an accepted synonym.
- The only `serde(rename = ...)` under `sop/` is TYPES:373, `capability_input` → `with`, which is a **step** field and therefore reachable only through `[[steps]]` (§8).

---

### 3. The `[sop]` table — every field

TYPES:605-633, verbatim:

```rust
/// The `[sop]` table in SOP.toml.
##[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SopMeta {
    pub name: String,
    pub description: String,
    #[serde(default = "default_sop_version")]
    pub version: String,
    #[serde(default)]
    pub priority: SopPriority,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_mode: Option<SopExecutionMode>,
    #[serde(default = "default_cooldown_secs")]
    pub cooldown_secs: u64,
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent: u32,
    /// Opt-in deterministic execution (no LLM round-trips between steps).
    #[serde(default)]
    pub deterministic: bool,
    /// Concurrent-trigger admission policy (`parallel` | `hold` | `coalesce` | `drop`).
    #[serde(default)]
    pub admission_policy: SopAdmissionPolicy,
    /// Max runs parked at a HITL approval at once (`0` = unlimited).
    #[serde(default = "default_max_pending_approvals")]
    pub max_pending_approvals: u32,
    /// Parent agent alias that owns the procedure. Steps run as this agent
    /// unless a step overrides it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
}
```

**Exactly 11 fields. No others exist.**

| # | TOML key | Rust type | Required | Default | Default defined at | Semantics |
|---|---|---|---|---|---|---|
| 1 | `name` | `String` | **yes** | — | — | Unique procedure name; doubles as the on-disk directory key (TYPES:459-461). Empty parses fine; warning at MOD:980-982; **blocking** in `validate_sop_strict` (MOD:1091-1093), which runs on save only. |
| 2 | `description` | `String` | **yes** | — | — | Free text, never executed (TYPES:462-464). Empty ⇒ warning only (MOD:983-985). |
| 3 | `version` | `String` | no | `"0.1.0"` | TYPES:668-670 | Opaque; never parsed as semver anywhere. |
| 4 | `priority` | `SopPriority` | no | `Normal` | `#[derive(Default)]` + `#[default]`, TYPES:12/17-18 | Scheduling priority (TYPES:468-470). **Also gates steps** under `execution_mode = "priority_based"` — §10.3. |
| 5 | `execution_mode` | `Option<SopExecutionMode>` | no | `None` | serde `Option` | `None` ⇒ fall back to the caller-supplied `default_execution_mode` (MOD:460). §7.2. |
| 6 | `cooldown_secs` | `u64` | no | `0` | TYPES:513-515 | Minimum seconds between runs; `0` disables (TYPES:481-483). |
| 7 | `max_concurrent` | `u32` | no | `1` | TYPES:517-519 | Max simultaneously *executing* runs. SYNTAX:33: a run parked at a HITL approval or deterministic checkpoint releases its slot. |
| 8 | `deterministic` | `bool` | no | `false` | `#[serde(default)]`, TYPES:621 | **Hard override** of `execution_mode` — §7.2. |
| 9 | `admission_policy` | `SopAdmissionPolicy` | no | `Parallel` | `#[derive(Default)]` + `#[default]`, TYPES:530/536-537 | §7.3. |
| 10 | `max_pending_approvals` | `u32` | no | `0` (= unlimited) | TYPES:521-523 | Cap on runs parked at a HITL gate (TYPES:500-503). |
| 11 | `agent` | `Option<String>` | no | `None` | serde `Option` | Parent agent alias. §10.4. |

**Serde attribute audit on `SopMeta`:** no `deny_unknown_fields`; no `rename_all` (so **every TOML key is exactly the Rust identifier**, e.g. `max_pending_approvals`, `cooldown_secs`); no `rename`; no `alias`; no `flatten`. `skip_serializing_if` on `execution_mode` (TYPES:614) and `agent` (TYPES:631) is **serialization-only** and irrelevant to a parser.

**Round-trip note:** `SopManifest::from_sop` (TYPES:636-665) always writes `execution_mode: Some(sop.execution_mode)` (TYPES:643). A machine-written `SOP.toml` therefore always carries an explicit `execution_mode`, and the config fallback no longer applies on reload. Only hand-written manifests exercise the `None` path.

#### Minimal valid `[sop]`

```toml
[sop]
name = "s"
description = "d"
```

**[CORRECTION] Proof citation.** The draft cited MOD:1665-1668 as writing "exactly" this; MOD:1665-1668 actually writes
`"[sop]\nname = \"s\"\ndescription = \"d\"\nadmission_policy = \"drop\"\nmax_pending_approvals = 1\n"` — two extra keys. The clean two-key proof is **TYPES:1358-1376** (`manifest_parse`), which parses a `[sop]` with only `name` + `description` and asserts `priority == Normal` (TYPES:1374) and `execution_mode == None` (TYPES:1375). MOD:1658-1676 remains the correct citation for the *load-path* proof of `admission_policy`/`max_pending_approvals`.

#### Maximal `[sop]`

```toml
[sop]
name = "deploy-prod"
description = "Production deploy with approval"
version = "1.4.2"
priority = "critical"
execution_mode = "step_by_step"
cooldown_secs = 300
max_concurrent = 2
deterministic = false
admission_policy = "hold"
max_pending_approvals = 8
agent = "release-bot"
```

---

### 4. Trigger representation in TOML

`triggers` is a TOML array of tables — repeated `[[triggers]]` blocks (TYPES:586-587). TYPES:115-142:

```rust
##[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize,
    strum_macros::EnumDiscriminants,
    zeroclaw_macros::TriggerFields,
)]
##[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
##[serde(tag = "type", rename_all = "lowercase")]
##[strum_discriminants(name(SopTriggerSource), …, serde(rename_all = "lowercase"),
                      strum(serialize_all = "lowercase"), …)]
pub enum SopTrigger {
```

- **Internally tagged**; the discriminator key is literally `type` (TYPES:126).
- Tag values are `rename_all = "lowercase"` of the variant name. All variant names are single words, so lowercase == snake_case here.
- **Field names are not renamed** — no field-level `rename_all`, no `rename`, no `alias`. `routing_key`, `calendar_source`, `calendar_ids` appear verbatim. Proven by TOML round-trip tests TYPES:1149-1162 and TYPES:1185-1208.
- **No `deny_unknown_fields`.** Serde internally-tagged enums buffer content and silently discard undeclared keys — see §9.2.
- `zeroclaw_macros::TriggerFields` (TYPES:123) is metadata-only; it generates `field_specs()` for the authoring registry (documented at `crates/zeroclaw-macros/src/lib.rs:2244-2259` — **[CORRECTION]**, the draft said 2241-2258). It emits no serde impls. `#[trigger(display = …)]` / `#[trigger(config_derived, …)]` are inert for deserialization.

Multi-trigger syntax, verbatim from CLI:528-566 (`parse_all_trigger_types`):

```toml
[sop]
name = "multi-trigger"
description = "SOP with all trigger types"

[[triggers]]
type = "mqtt"
topic = "sensors/temp"
condition = "$.value > 90"

[[triggers]]
type = "webhook"
path = "/sop/test"

[[triggers]]
type = "cron"
expression = "0 */5 * * *"

[[triggers]]
type = "peripheral"
board = "nucleo-f401re-0"
signal = "pin_3"
condition = "> 0"

[[triggers]]
type = "manual"
```

A manifest with **no** `[[triggers]]` loads (TYPES:586); `validate_sop` emits `"SOP has no triggers defined"` (MOD:986-988).

#### 4.1 The fan-in doc pages are generated from this enum

**[CORRECTION] — every line number in the draft's §4.1 was wrong, and one factual claim was wrong.**

The placeholder is on **line 9**, not 11, of each page: `docs/book/src/sop/fan-in/mqtt.md:9`, `amqp.md:9`, `channel.md:9`, `calendar.md:9`, `cron.md:9`, `filesystem.md:9`, `peripheral.md:9`, `webhook.md:9`, `git.md:9`. `fan-in/overview.md:18` (not 20) carries `{{#sop-trigger-index}}`, as does **SYNTAX:344**.

- **`fan-in/manual.md` contains no `{{#sop-trigger …}}` placeholder at all** — it is hand-written prose. The draft's "Every `fan-in/*.md` page contains only a placeholder" is false.
- **`fan-in/git.md:9` is `{{#sop-trigger channel}}`** — a second page projecting the *same* `Channel` variant. There are 10 fan-in pages for 9 variants; there is no `git` trigger type.

Expansion: `xtask/src/cmd/mdbook/peer_groups.rs:119-120` registers the two placeholders, dispatched at peer_groups.rs:147-148 to `render_sop_trigger_index` (peer_groups.rs:279-292) and `render_sop_trigger` (peer_groups.rs:294-33x). Both read `schemars::schema_for!(zeroclaw_runtime::sop::types::SopTrigger)` (peer_groups.rs:218) via `sop_trigger_variants` (peer_groups.rs:217-240). Required-vs-optional in the rendered tables is exactly schemars' `required` set (peer_groups.rs:252-266), i.e. *"has no `#[serde(default)]` and is not `Option`"*.

**Consequence for Egeria: the fan-in pages carry no independent information.** They are a pure projection of TYPES:142-229.

---

### 5. Every trigger variant

Source: TYPES:142-229. Matching: TRIGSRC:20-183.

#### 5.1 Variant reference table

| `type` | Required fields | Optional fields | Field types |
|---|---|---|---|
| `mqtt` | `topic` | `condition` | `String`, `Option<String>` |
| `webhook` | `path` | *(none)* | `String` |
| `cron` | `expression` | *(none)* | `String` |
| `peripheral` | `board`, `signal` | `condition` | `String`, `String`, `Option<String>` |
| `filesystem` | `path` | `events`, `condition` | `String`, `Vec<FilesystemEventKind>`, `Option<String>` |
| `calendar` | `calendar_source` | `calendar_ids`, `condition` | `String`, `Vec<String>`, `Option<String>` |
| `channel` | `channel` | `alias`, `condition` | `String`, `Option<String>`, `Option<String>` |
| `manual` | *(none)* | *(none)* | — |
| `amqp` | `routing_key` | `condition` | `String`, `Option<String>` |

`webhook` and `cron` are the **only two variants with no `condition` field** (TYPES:155-158, TYPES:161-164). Their matchers (TRIGSRC:58-62, TRIGSRC:67-71) never call `condition_holds`.

Variant declaration order in the enum (this is the order schemars and the doc tables use): Mqtt, Webhook, Cron, Peripheral, Filesystem, Calendar, Channel, Manual, Amqp.

**[ADDED] Shared helper:** `condition_holds` (TRIGSRC:20-25) — `None` ⇒ `true`; `Some(c)` ⇒ `evaluate_condition(c, event.payload.as_deref())`. Every trigger except `webhook`/`cron`/`manual` ends in `&& condition_holds(...)`, so the condition is always the **last** gate.

#### 5.2 Per-variant detail, matching semantics, and TOML examples

##### `mqtt` — TYPES:143-152

```rust
/// MQTT message arrival. Live: delivered by the MQTT listener.
##[trigger(display = "topic")]
Mqtt {
    /// Topic filter. `+` matches one level, `#` matches the remaining levels.
    topic: String,
    #[serde(default)]
    condition: Option<String>,
},
```

Matching, TRIGSRC:31-38: `event.topic` must be `Some` and satisfy `mqtt_topic_matches(pattern, topic)`, then the condition. Wildcards at **ENGINE:5370-5397** (**[CORRECTION]**, the draft said 5369-5395): `/`-delimited segments; `#` returns `true` immediately for all remaining segments (ENGINE:5378); `+` consumes exactly one segment (ENGINE:5379-5383); otherwise segments must be equal; the loop exits when either side is exhausted and the final check is `pi == pat_parts.len() && ti == top_parts.len()` (ENGINE:5396).

**[ADDED] Trap the draft did not state: MQTT `#` does not absorb zero levels.** Trace ENGINE:5376-5396 with pattern `a/#` and topic `a`: the loop consumes `a`, `ti` reaches 1 == `top_parts.len()`, the loop exits before `#` is ever inspected, and `pi == 1 != 2` ⇒ **false**. Likewise `a/+` vs `a` ⇒ false, and `a` vs `a/b` ⇒ false. This is the exact opposite of AMQP `#` (below) and is the single reason the two matchers cannot be shared.

Minimal / maximal:
```toml
[[triggers]]
type = "mqtt"
topic = "facility/pump/pressure"
```
```toml
[[triggers]]
type = "mqtt"
topic = "facility/+/pressure/#"
condition = "$.value > 85"
```
(TYPES:1164-1175 proves the maximal form parses.)

##### `webhook` — TYPES:153-158

Matching, TRIGSRC:58-62: `event.topic.as_deref() == Some(path)` — byte-exact equality, no globbing, no condition support. A `None` topic fails.

```toml
[[triggers]]
type = "webhook"
path = "/sop/test"
```

##### `cron` — TYPES:159-164

Matching, TRIGSRC:67-71: `event.topic.as_deref() == Some(expression)` — the dispatcher pre-resolves schedules and puts the expression itself into the topic. No condition support.

```toml
[[triggers]]
type = "cron"
expression = "0 */5 * * *"
```

Cron-expression validity is **not** checked by the manifest parser — `load_sop` (MOD:422-481) contains no cron parse. `docs/book/src/sop/fan-in/cron.md:3` ("Schedules are parsed once at startup") and `cron.md:5` ("Invalid expressions fail closed during parsing and cache build") describe the dispatch/maintenance layer. **An invalid cron string loads successfully.**

##### `peripheral` — TYPES:165-175

Matching, TRIGSRC:78-86: `event.topic == format!("{board}/{signal}")` exactly, then the condition.

```toml
[[triggers]]
type = "peripheral"
board = "nucleo-f401re-0"
signal = "pin_3"
condition = "> 0"
```
(CLI:546-550 uses exactly this — note the *direct numeric* condition form, §12.)

##### `filesystem` — TYPES:176-187

Matching, TRIGSRC:93-109, in order:
1. `filesystem_path_matches(path, event.topic)` (**ENGINE:5422-5430**): try `glob::Pattern::new(pattern).matches(path)`; **if the glob does not match — whether it failed to compile or simply did not match — fall through** to the bare-directory test `path == pattern.trim_end_matches('/') || path.starts_with(&format!("{prefix}/"))` (ENGINE:5428-5429). **[CORRECTION]** the draft said the fallback applies only "if that fails [to compile]"; ENGINE:5423-5427 shows the `if let Ok(...) && compiled.matches(path)` guard, so a *compiling but non-matching* glob also reaches the prefix test.
2. If `events` is non-empty, `filesystem_event_listed` (ENGINE:5433-5447) requires a JSON payload with a string field `event` equal to one listed kind's `Display` string (ENGINE:5446 compares `e.to_string() == kind`). An **empty `events` list skips the check entirely**; a non-empty list with a missing payload (ENGINE:5437-5439), non-JSON payload (ENGINE:5440-5442), or missing/non-string `event` key (ENGINE:5443-5445) fails closed.
3. Condition.

`events` values (`FilesystemEventKind`, TYPES:71-91, `#[serde(rename_all = "lowercase")]`): `"created"`, `"modified"`, `"deleted"`, `"renamed"`. Display strings match (TYPES:93-102).

```toml
[[triggers]]
type = "filesystem"
path = "/var/inbox"
```
(TYPES:1210-1220 proves `events` defaults to `[]` and `condition` to `None`.)
```toml
[[triggers]]
type = "filesystem"
path = "/var/inbox/**/*.json"
events = ["created", "modified"]
condition = "$.extension == \"json\""
```
(TYPES:1184-1208, verbatim.)

##### `calendar` — TYPES:188-199

Matching, `calendar_trigger_matches` (**ENGINE:5342-5367**, **[CORRECTION]** — the draft said 5343-5367) via TRIGSRC:116-121, is unusually narrow: `event.topic` must equal the constant `CALENDAR_NO_SHOW_TOPIC` (ENGINE:5347, imported at ENGINE:24), the payload must deserialize as `CalendarNoShowEvent` (ENGINE:5354-5357), `payload.calendar_source` must equal the trigger's (ENGINE:5358-5360), and — if `calendar_ids` is non-empty — `payload.calendar_id` must be listed (ENGINE:5362-5366). Empty `calendar_ids` matches every calendar of the source.

```toml
[[triggers]]
type = "calendar"
calendar_source = "microsoft365"
calendar_ids = ["primary", "team"]
condition = "$.organizer == \"ops\""
```
(TYPES:1148-1162 proves the `calendar_source` + `calendar_ids` form.)

##### `channel` — TYPES:200-216

Matching, `channel_trigger_topic_matches` (ENGINE:5323-5340). The topic is parsed by `ChannelSopTopic::parse` (`crates/zeroclaw-api/src/channel.rs:47-57`):

```rust
let (head, event_type) = match topic.split_once(':') { … };
let (channel, alias) = head
    .split_once('.')
    .or_else(|| head.split_once('/'))
    .map_or((head, None), |(c, a)| (c, Some(a)));
```

**[ADDED] precision the draft lacked:**
- `:` splits off `event_type` first; then `.` is tried for the alias, and only if absent is `/` tried (`channel.rs:52-55`). So `git.main:pull_request.opened` ⇒ channel `git`, alias `main`, event `pull_request.opened`; `telegram/prod` ⇒ channel `telegram`, alias `prod`.
- **Channel type compares case-insensitively** (`eq_ignore_ascii_case`, ENGINE:5333) but the **alias compares case-sensitively** (`ta == a`, ENGINE:5337). The draft stated the first and not the second.
- An aliased trigger requires an exact alias; an alias-less trigger matches any instance (ENGINE:5336-5339). A `None` topic fails closed (ENGINE:5329-5331).
- Per TYPES:200-205 the forge producer puts `event_type` in the payload, so forge-event filtering uses `condition`, not a separate shape.

`channel` is a `ChannelKind` snake_case value (`telegram`, `discord`, `slack`, `git`, …) — TYPES:207-208. **The manifest parser does not validate it against the configured channel set**; `#[trigger(config_derived)]` (TYPES:206) only means the *authoring registry* supplies options from live config (documented at `crates/zeroclaw-macros/src/lib.rs:2254-2256`).

```toml
[[triggers]]
type = "channel"
channel = "git"
alias = "main"
condition = "$.event_type == \"pull_request.opened\""
```
(TYPES:1222-1236, verbatim.)

##### `manual` — TYPES:217-218

```rust
/// Agent-initiated run via the `sop_execute` tool. Not an external fan-in.
Manual,
```

Unit variant under an internally-tagged enum ⇒ a table containing only `type`. Matching (TRIGSRC:136-140) is unconditionally `true` — it does not even look at the topic.

```toml
[[triggers]]
type = "manual"
```
(TYPES:1177-1182: `let toml_str = r#"type = "manual""#;` parses to `SopTrigger::Manual`.)

##### `amqp` — TYPES:219-228

Matching, TRIGSRC:45-53 → `amqp_routing_key_matches` (ENGINE:5402-5406) → `amqp_match_from` (ENGINE:5408-5417):

```rust
match pat.first() {
    None => words.is_empty(),
    Some(&"#") => (0..=words.len()).any(|skip| amqp_match_from(&pat[1..], &words[skip..])),
    Some(&"*") => !words.is_empty() && amqp_match_from(&pat[1..], &words[1..]),
    Some(seg) => !words.is_empty() && *seg == words[0] && amqp_match_from(&pat[1..], &words[1..]),
}
```

`.`-delimited words; `*` matches exactly one word; `#` matches **zero or more** words, backtracking over every split. ENGINE:5400-5401 states it outright: *"A `#` that can absorb zero segments is what distinguishes this from MQTT matching."* **Egeria must not share one wildcard matcher between `mqtt` and `amqp`.**

```toml
[[triggers]]
type = "amqp"
routing_key = "org.release.*.version.#"
condition = "$.project.name == \"bzip2\""
```

#### 5.3 Trigger discriminant enum (`SopTriggerSource`)

TYPES:127-141 derives `SopTriggerSource` from `SopTrigger` via `strum_discriminants`, with `serde(rename_all = "lowercase")` (TYPES:137) and `strum(serialize_all = "lowercase")` (TYPES:138). Serialized spellings are identical to the `type` tag values (proven TYPES:1141-1145, TYPES:1379-1383).

It is **not** a manifest field. **[CORRECTION]** the draft said it "appears in `SopEvent.source` (TYPES:677) and `SopRunSummary.trigger_source`." The first is right; the second is wrong — `SopRunSummary.trigger_source` is a **`String`** (TYPES:842), populated by `run.trigger_event.source.to_string()` (TYPES:858), i.e. the strum `Display`, not the enum. Egeria needs `SopTriggerSource` only if it models events.

---

### 6. The `positions` structure

TYPES:597-603:

```rust
/// One step's persisted canvas coordinate in SOP.toml.
##[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct StepPosition {
    pub step: u32,
    pub x: f64,
    pub y: f64,
}
```

- **Shape:** TOML array of tables, `[[positions]]`.
- **Key type:** `step: u32` — a **step number**, matched against `SopStep::number`. Not an index, not a string key. A negative value is a `u32` parse error.
- **Coordinates:** `f64`. Negative values legal (MOD:1309 uses `y: -48.0`).
- **All three fields required** — no `#[serde(default)]` on any (TYPES:600-602).
- No `deny_unknown_fields`, no renames, **no `JsonSchema` derive** (contrast TYPES:302 on `StepPos`).

Merge, MOD:437-441:

```rust
for pos in &manifest.positions {
    if let Some(step) = steps.iter_mut().find(|s| s.number == pos.step) {
        step.pos = Some(types::StepPos { x: pos.x, y: pos.y });
    }
}
```

Behavioral consequences:
- A `[[positions]]` entry whose `step` matches no parsed step is **silently ignored** — no error, no warning.
- **[CORRECTION] Two distinct duplicate cases, which the draft conflated:**
  - **Duplicate `[[positions]]` entries for the same `step`:** each iteration re-finds the same step and overwrites ⇒ the **last `[[positions]]` entry wins**.
  - **Duplicate step *numbers* among the steps** (possible only via the `[[steps]]` fallback, which preserves authored numbers — §8): `find` returns the **first** such step, so the later duplicates never receive a position.
- `step = 0` never matches anything: `parse_steps` numbers positionally from 1 (MOD:537-540) and `normalize_manifest_steps` replaces `0` with `idx+1` (MOD:485-487).
- Merging happens **after** step parsing and **before** the `[sop]` destructure, so positions attach to steps from either source.

The runtime target type is `StepPos { x: f64, y: f64 }` (TYPES:301-306) — a **different type name** with no `step` field, and (unlike `StepPosition`) it derives `PartialEq` and `JsonSchema`.

Round-trip proof, MOD:1304-1332 (`step_pos_roundtrips_via_toml_and_stays_out_of_markdown`):
```rust
assert!(toml.contains("[[positions]]"), "positions block in TOML: {toml}");   // MOD:1315-1318
assert!(!md.contains("320.5"), "coordinate must not leak into SOP.md: {md}"); // MOD:1320-1323
assert_eq!(loaded.steps[1].pos, None);                                        // MOD:1331
```

```toml
[[positions]]
step = 1
x = 320.5
y = -48.0

[[positions]]
step = 2
x = 640.0
y = 120.0
```

**Egeria/ADR-0003 note:** this is exactly the "layout is never semantic" data. It maps to Egeria's view data and must be excluded from equality and the semantic hash.

---

### 7. Enums — exact serialized spellings

All are **case-sensitive** serde enums. An unrecognized value is a `toml::from_str` **error**, which under directory loading means warn-and-skip (MOD:405-413).

#### 7.1 `SopPriority` — TYPES:11-21

```rust
##[serde(rename_all = "lowercase")]
pub enum SopPriority { Low, #[default] Normal, High, Critical }
```
Values `"low"` / `"normal"` / `"high"` / `"critical"`; default `normal`. Round-trip TYPES:1113-1119; manifest default TYPES:1374.

#### 7.2 `SopExecutionMode` and the two-stage resolution — TYPES:36-54

```rust
##[serde(rename_all = "snake_case")]
pub enum SopExecutionMode { Auto, #[default] Supervised, StepByStep, PriorityBased, Deterministic }
```
Values `"auto"` / `"supervised"` / `"step_by_step"` / `"priority_based"` / `"deterministic"`. Round-trip TYPES:1121-1127 and TYPES:1286-1292.

**The enum's `#[default]` (`Supervised`, TYPES:44) is NOT the manifest default.** In `SopMeta` the field is `Option<SopExecutionMode>` defaulting to `None` (TYPES:614-615); TYPES:1375 asserts `manifest.sop.execution_mode == None`.

Resolution, MOD:456-461:
```rust
// When deterministic=true, override execution_mode to Deterministic
let effective_mode = if deterministic {
    SopExecutionMode::Deterministic
} else {
    execution_mode.unwrap_or(default_execution_mode)
};
```
Precedence, in order: `deterministic = true` ⇒ `Deterministic`, unconditionally; else authored `execution_mode`; else `default_execution_mode`.

`default_execution_mode` is a **caller-supplied parameter** (MOD:422, MOD:380, MOD:227), sourced from the daemon config's `[sop] default_execution_mode` string (CFG:22564-22568; default `"supervised"` at CFG:22687-22689) and converted by `parse_execution_mode` (MOD:170-179):

```rust
pub fn parse_execution_mode(s: &str) -> SopExecutionMode {
    match s.trim().to_lowercase().as_str() {
        "auto" => SopExecutionMode::Auto,
        "step_by_step" => SopExecutionMode::StepByStep,
        "priority_based" => SopExecutionMode::PriorityBased,
        "deterministic" => SopExecutionMode::Deterministic,
        // "supervised" and any unknown value
        _ => SopExecutionMode::Supervised,
    }
}
```

**Asymmetry the implementer must not smooth over:** the *config* string is trimmed, lowercased, and falls back to `Supervised` on an unknown value; the *manifest* `execution_mode` goes through strict serde and **errors** on an unknown value or wrong case. `execution_mode = "Auto"` in `SOP.toml` kills the whole SOP; `default_execution_mode = "Auto"` in `config.toml` quietly becomes `Auto`. Egeria parses only `SOP.toml` ⇒ implement the **strict** behavior.

#### 7.3 `SopAdmissionPolicy` — TYPES:529-548

```rust
##[serde(rename_all = "snake_case")]
pub enum SopAdmissionPolicy { #[default] Parallel, Hold, Coalesce, Drop }
```
Values `"parallel"` (default) / `"hold"` / `"coalesce"` / `"drop"`. Load-path proof MOD:1658-1676. Documented at SYNTAX:37-49, which matches the source exactly (TYPES:533-547 vs SYNTAX:39-49). No doc/source conflict here.

#### 7.4 `FilesystemEventKind` — TYPES:71-91

```rust
##[serde(rename_all = "lowercase")]
##[strum(serialize_all = "lowercase")]
pub enum FilesystemEventKind { Created, Modified, Deleted, Renamed }
```
Values `"created"` / `"modified"` / `"deleted"` / `"renamed"`. Only ever appears inside `[[triggers]] events = [...]`. Round-trip TYPES:1238-1246. `FromStr` (TYPES:104-110) lowercases first, but that is a runtime helper — **serde deserialization does not**, so `events = ["Created"]` is a parse error.

#### 7.5 Enums reachable only through `[[steps]]`

`SopStepKind` (TYPES:240-251, `snake_case`: `execute` / `checkpoint` / `capability`) and `StepFailure` (`step_contract.rs:50-62`, `snake_case`, externally tagged: `"fail"`, `{ retry = { max = 3 } }`, `{ goto = { step = 4 } }`). `SopRunStatus` (TYPES:691-703) and `SopStepStatus` (TYPES:719-726) belong to runs and never appear in `SOP.toml`.

---

### 8. `[[steps]]` in SOP.toml — the audit's claim, corrected

The prior audit recorded: *"There is no `[[steps]]` table in SOP.toml — steps live only in SOP.md."*

**That is true of hand-authored docs and checked-in fixtures, but false of the code — and, [CORRECTION], also false of the writer's actual output, which the draft got wrong.**

1. `SopManifest` **has** `steps: Vec<SopStep>` with `#[serde(default)]` (TYPES:593-594). A `SOP.toml` containing `[[steps]]` parses.
2. `load_sop` uses it **only as a fallback** (MOD:428-435): if `SOP.md` **exists**, `parse_steps(&md_content)` wins and `manifest.steps` is **discarded entirely** — even if `SOP.md` parses to zero steps. The existence test is `md_path.exists()`, not a content test.
3. The fallback normalizes (MOD:483-496):
   ```rust
   fn normalize_manifest_steps(mut steps: Vec<SopStep>) -> Vec<SopStep> {
       for (idx, step) in steps.iter_mut().enumerate() {
           if step.number == 0 {
               step.number = u32::try_from(idx).unwrap_or(u32::MAX).saturating_add(1);
           }
           if step.title.is_empty() {
               step.title = step.capability.clone().unwrap_or_else(|| step.kind.to_string());
           }
       }
       steps
   }
   ```
   This is **different** from the Markdown renumbering at MOD:537-540, which is positional and unconditional (`u32::try_from(steps.len()) + 1`). `normalize_manifest_steps` fills `number` only when it is `0` and otherwise preserves the authored number — so `[[steps]]` can legally produce duplicate or non-contiguous step numbers, which `SOP.md` cannot.
4. **The writer emits it, and this is exercised by upstream tests.** `SopManifest.steps` has `#[serde(default)]` and **no `skip_serializing_if`** (TYPES:593-594) — unlike `triggers` and `positions` (TYPES:586, 591). `save_sop` (MOD:966-969) does `toml::to_string_pretty(&manifest)` and writes both files.
   **[CORRECTION] The draft's claim that "no test, no doc, no fixture exercises it" is wrong.** The grep for the literal string `[[steps]]` returns zero matches only because the blocks are *generated at run time*. Every `save_sop` test with a non-empty step list writes them: MOD:1291 and MOD:1312 both call `save_sop(dir.path(), &sop)` on an `authoring_sop(vec![titled_step(...)])` (helpers at MOD:1155-1180) and both `unwrap()` successfully. So `toml::to_string_pretty` on a `SopManifest` with non-empty `steps` **demonstrably succeeds** for plain `SopStep`s, and a round-tripped SOP has its steps in *both* files with `SOP.md` authoritative on reload.

**`[[steps]]` field shape** (TYPES:311-392), for an implementer who must accept it — all have `#[serde(default)]`, so every one is optional:

| TOML key | Rust type | Line |
|---|---|---|
| `number` | `u32` (default `0` ⇒ filled positionally) | TYPES:315-316 |
| `title` | `String` (default `""` ⇒ filled from `capability` or `kind`) | TYPES:319-320 |
| `body` | `String` | TYPES:324-325 |
| `suggested_tools` | `Vec<String>` (legacy alias for `scope.allow`) | TYPES:329-330 |
| `requires_confirmation` | `bool` | TYPES:333-334 |
| `kind` | `SopStepKind` | TYPES:336-337 |
| `schema` | `Option<StepSchema>` (`input`/`output`, `serde_json::Value`) | TYPES:339-340, TYPES:268-277 |
| `scope` | `Option<StepToolScope>` (`allow: Option<Vec<String>>`, `deny: Vec<String>`) | TYPES:342-343, `scope/mod.rs:9-19` |
| `routing` | `StepRouting` (`when`, `next`, `terminal`, `depends_on`, `switch[]`) | TYPES:345-346, `step_contract.rs:20-41` |
| `on_failure` | `StepFailure` | TYPES:348-349, `step_contract.rs:50-62` |
| `mode` | `Option<SopExecutionMode>` | TYPES:351-352 |
| `calls` | `Vec<PlannedToolCall>` (`tool`, `args`, `pinned`) | TYPES:355-356, TYPES:289-297 |
| `pos` | `Option<StepPos>` | TYPES:359-360 |
| `agent` | `Option<String>` | TYPES:367-368 |
| `capability` | `Option<String>` | TYPES:370-371 |
| **`with`** | `Option<serde_json::Value>` — **the only renamed key in the manifest tree**, Rust field `capability_input` | TYPES:373-374 |
| `policy` | `Option<String>` | TYPES:378-379 |
| `gate_prompt` | `Option<String>` | TYPES:385-386 |
| `edit` | `Option<String>` | TYPES:390-391 |

#### Recommendation for Egeria

- **Accept** `[[steps]]`; do not reject it as unknown (upstream does not).
- **Implement the precedence exactly**: `SOP.md` present ⇒ `[[steps]]` ignored, regardless of content.
- Emit an Egeria `Finding` when both `SOP.md` and a non-empty `[[steps]]` are present — the two can silently diverge, and upstream has no diagnostic for it.
- Note that `[[steps]]` is the only path by which duplicate step numbers can reach a loaded `Sop`, and therefore the only path that can suppress a `[[positions]]` merge (§6).

---

### 9. Unknown keys and missing required fields

#### 9.1 Unknown keys — accepted and discarded

`grep -rn "deny_unknown_fields" crates/zeroclaw-runtime/src/sop/` → **0 matches** (repo-wide: 11, all outside `sop/`). Specifically absent from `SopManifest` (TYPES:583-595), `SopMeta` (TYPES:606-633), `StepPosition` (TYPES:598-603), `SopTrigger` (TYPES:115-142), `SopStep` (TYPES:309-392).

Therefore, with serde defaults:
- Unknown **top-level** key (`[metadata]`, `notes = "..."`) — silently ignored.
- Unknown key **inside `[sop]`** (`max_step_retries = 5`, `owner = "ops"`) — silently ignored.
- Unknown key **inside a `[[triggers]]` table** — silently ignored (internally-tagged enums buffer content and drop undeclared keys).
- Unknown key inside `[[positions]]` or `[[steps]]` — silently ignored.

Nothing anywhere logs or warns about a dropped key.

#### 9.2 The highest-value silent-drop traps

1. **`condition` on `webhook` or `cron`.** Those variants declare no `condition` (TYPES:155-158, TYPES:161-164). `type = "webhook"`, `path = "/x"`, `condition = "$.a == 1"` parses cleanly and the condition **vanishes**; the trigger then matches every event whose topic equals `/x` (TRIGSRC:58-62). Egeria should surface a Finding rather than reproduce the silence.
2. **`max_step_retries` / `max_step_visits` / `step_scope_enforce` / `step_schema_enforce` inside `[sop]` of `SOP.toml`.** These are *daemon config* keys (CFG:22637-22655), not manifest keys. Writing them into `SOP.toml` is silently ignored. See §10.5.
3. **[ADDED] `sop.approval` written into `SOP.toml`.** SYNTAX:77-79 shows `[sop.approval.groups.*]` / `[sop.approval.policies.*]` blocks; those live in the daemon config's `[sop]` table (CFG:22626-22635), **not** in `SOP.toml`, whose `[sop]` table is `SopMeta`. A `[sop.approval]` block in `SOP.toml` is an unknown key inside `[sop]` and is silently dropped. The visual similarity of the two `[sop]` tables makes this an easy authoring mistake and a strong Egeria Finding.
4. **[ADDED] Step-level keys written into `[sop]`** (`policy`, `gate_prompt`, `edit`, `kind`) — silently dropped.

#### 9.3 Missing required fields

**[CORRECTION] The draft's sentence "Only three fields have no default" sat above a five-row table.** The correct statement: **two `[sop]` scalars** (`name`, `description`) and **the `[sop]` table itself** have no default; additionally, per-variant trigger fields and all three `StepPosition` fields have no default.

| Missing | Result |
|---|---|
| the `[sop]` table | `toml::from_str` error `missing field \`sop\`` (TYPES:585, no `default`) |
| `sop.name` | error `missing field \`name\`` (TYPES:608) |
| `sop.description` | error `missing field \`description\`` (TYPES:609) |
| a required trigger field (`topic`, `path`, `expression`, `board`, `signal`, `calendar_source`, `channel`, `routing_key`) | error for that variant |
| `type` on a `[[triggers]]` table | error — the internal tag is not optional (TYPES:126) |
| `positions.step` / `.x` / `.y` | error (TYPES:600-602) |

Every such error propagates out of `load_sop` (MOD:425 uses `?`) and, under directory loading, produces warn-and-skip (MOD:403-414). `load_sop_by_name` (MOD:235-241) returns the `Err`.

`Option<T>` fields (`execution_mode`, `agent`, every `condition`, `alias`) deserialize to `None` when absent: serde's derive routes a missing field without `default` through `serde::__private::de::missing_field`, whose deserializer answers `deserialize_option` with `visit_none`. The `#[serde(default)]` on `condition`/`alias` (TYPES:150, 173, 185-186, 197, 211, 214, 226) is therefore explicit-but-redundant. `Vec<T>` fields genuinely need `#[serde(default)]`, and every one has it (TYPES:182, 194, 586, 591, 593).

#### 9.4 Emptiness is not an error

`name = ""` and `description = ""` parse. They are warnings in `validate_sop` (MOD:980-985) and blocking only in `validate_sop_strict` (MOD:1091-1093, name only), which runs on **save** (MOD:958-961), not on load. **A SOP with an empty name loads successfully.** Note the asymmetry: `validate_sop` tests `is_empty()` (MOD:980), `validate_sop_strict` tests `trim().is_empty()` (MOD:1091), so `name = "  "` warns nowhere on load but blocks on save.

---

### 10. Manifest → runtime mapping, and manifest fields that affect step semantics

#### 10.1 Full mapping (MOD:442-478)

| `SopMeta` field | `Sop` field | Transformation |
|---|---|---|
| `name` | `name` | verbatim |
| `description` | `description` | verbatim |
| `version` | `version` | verbatim |
| `priority` | `priority` | verbatim |
| `execution_mode` | `execution_mode` | **`effective_mode`** (§7.2) |
| `cooldown_secs` | `cooldown_secs` | verbatim |
| `max_concurrent` | `max_concurrent` | verbatim |
| `deterministic` | `deterministic` | verbatim (kept as well as applied) |
| `admission_policy` | `admission_policy` | verbatim |
| `max_pending_approvals` | `max_pending_approvals` | verbatim |
| `agent` | `agent` | verbatim |
| `manifest.triggers` | `triggers` | verbatim, order preserved |
| *(steps, §1 step 4)* | `steps` | `SOP.md` else `[[steps]]` else `[]` |
| *(none)* | `location` | `Some(sop_dir.to_path_buf())` (MOD:473); `#[serde(skip)]` on the type (TYPES:489-491) |
| *(dropped)* | — | `manifest.positions` is consumed by the merge (MOD:437-441) and has no `Sop` field |

#### 10.2 `deterministic` — the hard override

MOD:456-461. `deterministic = true` sets `execution_mode = Deterministic` **unconditionally**, discarding any authored `execution_mode`. TYPES:471-474 states it:

> How steps are driven: `auto`, `supervised` (default), `step_by_step`,
> `priority_based`, or `deterministic`. `deterministic = true` forces
> the last regardless of this field.

Effect (TYPES:50-53): steps execute sequentially with **no LLM round-trips**, each step's output piped as the next step's input, and `kind = "checkpoint"` steps pause for human approval. End-to-end proof CLI:568-611 (`deterministic_flag_overrides_execution_mode`), which loads a manifest with only `deterministic = true` and asserts `sop.execution_mode == Deterministic` (CLI:610).

**Egeria must model `deterministic` as a mode selector, not an independent boolean.** `deterministic = true` + `execution_mode = "auto"` is not a conflict upstream — `auto` is silently discarded with no diagnostic. Good Egeria Finding candidate.

#### 10.3 `priority` affects gating, not just scheduling

Under `execution_mode = "priority_based"`, `priority` decides whether each step gates. **[CORRECTION] cite ENGINE:5461-5469** (the match arm opens at 5461, not 5462):

```rust
SopExecutionMode::PriorityBased => match sop.priority {
    // [SEC-FLIP] Critical/High are the MOST dangerous runs, so they MUST
    // gate (was `=> false`, an inversion that auto-ran the riskiest SOPs).
    SopPriority::Critical | SopPriority::High => true,
    SopPriority::Normal | SopPriority::Low => {
        // Supervised behavior for normal/low
        step.number == 1
    }
},
```

This **contradicts the enum's own doc comment** at TYPES:48: `/// Critical/High → Auto, Normal/Low → Supervised.` The comment describes the pre-`[SEC-FLIP]` behavior. **Source wins:** Critical/High gate every step; Normal/Low gate only step 1.

Full mode→gating table, `execution_mode_needs_approval` (ENGINE:5451-5471): `auto` and `deterministic` ⇒ never gate on this path (ENGINE:5455); `supervised` ⇒ gate iff `step.number == 1` (ENGINE:5456-5459); `step_by_step` ⇒ always (ENGINE:5460); `priority_based` ⇒ as above.

**[ADDED] the draft's table was incomplete — the real predicate is `step_requires_approval_gate` (ENGINE:5473-5481):**

```rust
fn step_requires_approval_gate(sop: &Sop, step: &SopStep) -> bool {
    if step.requires_confirmation {
        return true;
    }
    let effective_mode = step.mode.unwrap_or(sop.execution_mode);
    execution_mode_needs_approval(sop.execution_mode, sop, step)
        || execution_mode_needs_approval(effective_mode, sop, step)
}
```

So: `requires_confirmation: true` short-circuits to gating; otherwise the result is the **OR** of the SOP-level mode and the step's `mode` override — a step-level `mode` can only **add** gates, never remove one the SOP-level mode imposes. A `checkpoint` step blocks direct advance regardless (ENGINE:5483-5485).

#### 10.4 `agent` — parent agent for every step

`SopMeta.agent` → `Sop.agent`. Resolution at TYPES:446-450 (**[CORRECTION]**, the draft said 448-450; the doc comment begins at 446):

```rust
/// The agent alias that runs this step: the step's own override when set,
/// otherwise the SOP's parent agent.
pub fn effective_agent<'a>(&'a self, parent: Option<&'a str>) -> Option<&'a str> {
    self.agent.as_deref().or(parent)
}
```

Applied at ENGINE:5489-5493, where the resolved alias is stamped onto the cloned step before the action is built. Step-level `- agent:` (parsed at MOD:606-608) overrides; unset inherits. `Sop.agent`'s doc comment (TYPES:505-508):

> Required for headless triggers (mqtt, webhook, cron, amqp), which have no
> ambient agent loop to borrow.

**This "required" is not enforced anywhere.** No check in `load_sop` (MOD:422-481), `validate_sop` (MOD:977-1008), `validate_sop_strict` (MOD:1088-1123), or the graph diagnostics (`grep -n "agent" graph.rs` yields only graph.rs:37, 762, 1067 — a doc comment and two test literals). A manifest with an `mqtt` trigger and no `agent` loads clean. The consequence surfaces only at dispatch, as a WARN log — dispatch.rs:982-986: `"SOP headless dispatch: run {run_id} ('{sop_name}') ready for step {} '{}' but no agent loop available to execute"`.

**Egeria opportunity:** a genuine unenforced upstream invariant and a natural `EGR-*` rule. Do not claim upstream rejects it.

#### 10.5 What is **NOT** in `SOP.toml`

| Knob | Where it actually lives | Default | Default defined at |
|---|---|---|---|
| `max_step_retries` | daemon config `[sop]` (CFG:22653-22655) | `2` | CFG:22843-22845 |
| `max_step_visits` | daemon config `[sop]` (CFG:22649-22651) | `256` | CFG:22839-22841 |
| `default_execution_mode` | daemon config `[sop]` (CFG:22564-22568) | `"supervised"` | CFG:22687-22689 |
| `max_concurrent_total` | daemon config `[sop]` (CFG:22570-22572) | `4` | CFG:22812-22814 |
| `step_scope_enforce` | daemon config `[sop]` (CFG:22637-22639) | `false` | `#[serde(default)]`, CFG:22638 |
| `step_schema_enforce` | daemon config `[sop]` (CFG:22645-22647) | `true` | CFG:22835-22837 |
| `step_mandatory_tools` **[ADDED]** | daemon config `[sop]` (CFG:22641-22643) | `["sop_advance","sop_approve","sop_status"]` | CFG:22828-22833 |
| `approval_timeout_secs` **[ADDED]** | daemon config `[sop]` (CFG:22574-22579) | `300` | CFG:22816-22818 |
| `sops_dir` | daemon config `[sop]` (CFG:22557-22562) | `<workspace>/sops` | MOD:183-186, MOD:194-202 |
| approval groups/policies | daemon config `[sop.approval]` (CFG:22626-22635) | empty | — |

Consumption sites proving these are engine-config: ENGINE:2108, 2134, 2224 (`self.config.max_step_visits`); ENGINE:3182, 3393 (`self.config.max_step_retries`). SYNTAX:77-78 confirms the approval side: *"Approval broker groups and policies live in the main ZeroClaw config, not in per-SOP `SOP.toml` files."*

**Egeria consequence:** `max_step_retries` and `max_step_visits` are **not** parseable from a SOP directory. Model them as an **analysis parameter with the upstream defaults 2 and 256**, not as manifest fields. A per-step `- on_failure: retry:<count>` bullet in `SOP.md` is clamped against the config value at run time, not at parse time.

---

### 11. Name ↔ directory relationship

`name` doubles as the on-disk directory key (TYPES:459-461). `resolve_sop_dir` (MOD:206-219):

```rust
fn resolve_sop_dir(sops_dir: &Path, name: &str) -> Result<PathBuf> {
    let mut components = Path::new(name).components();
    let single_normal = matches!(
        (components.next(), components.next()),
        (Some(std::path::Component::Normal(_)), None)
    );
    if single_normal && !name.contains(['/', '\\', '\0']) {
        Ok(sops_dir.join(name))
    } else {
        anyhow::bail!(
            "invalid SOP name '{name}': must be a single path component (no separators, '.', '..', or absolute paths)"
        )
    }
}
```

Rejected inputs, tested exhaustively at MOD:1334-1369: `"../escape"`, `".."`, `"."`, `"/etc/shadow"`, `"a/b"`, `"a\\b"`, `"../../etc/cron.d/evil"`, `""`.

**Critical asymmetry:** the check applies to `load_sop_by_name` (MOD:240), `delete_sop` (MOD:246), `create_sop` (MOD:257), `create_sop_typed` (MOD:285), `delete_sop_typed` (MOD:293) and `save_sop` (MOD:963) — **not** to `load_sop` from directory scanning. `load_sops_from_directory` (MOD:392-415) takes the name from the manifest **without checking it against the directory name and without running `resolve_sop_dir`**. So `sops/foo/` whose `SOP.toml` says `name = "bar"` loads as `"bar"`, and `zeroclaw sop show bar` (which resolves `sops/bar/`) then fails to find it.

**[CORRECTION] The draft's "Upstream emits no diagnostic for this mismatch" is wrong in one path.** `validate_candidate` (PROCMEM:233-254) writes a candidate into a temp dir named `slugify(sop_name)`, loads it, and explicitly rejects the mismatch (PROCMEM:243-249):

```rust
if sops[0].name != sop_name {
    bail!(
        "candidate manifest name '{}' does not match proposal target '{}'",
        sops[0].name,
        sop_name
    );
}
```

It also requires exactly one loadable SOP (PROCMEM:240-242) and non-empty steps (PROCMEM:250-252). This is the *procedural-memory apply* gate only; the normal daemon load path still has no such check. Accurate statement: **upstream enforces name↔target agreement only for procedural-memory proposals, and nowhere on the ordinary load path.** Still a good Egeria Finding.

---

### 12. Trigger `condition` grammar (manifest-embedded expressions)

`condition` values are plain TOML strings, **not parsed or validated at manifest load time** — there is no call site in `load_sop` (MOD:422-481). They are interpreted at dispatch by `evaluate_condition` (COND:3-21), shared with step `when:` guards (SYNTAX:350-351).

Grammar (docs at SYNTAX:348-424 match the source):

- **Empty or whitespace-only condition ⇒ `true`**, unconditional match (COND:4-7). The empty check runs **before** the payload check, so `condition = ""` matches even with no payload. (**[CORRECTION]** the draft raised and then answered this as a rhetorical aside; state it as fact: COND:4 trims, COND:5-7 returns `true`, COND:9-12 is reached only for a non-empty condition.)
- **Missing or empty payload ⇒ `false`**, fail-closed (COND:9-12): `Some(p) if !p.is_empty()` — an empty-string payload is treated as no payload.
- **JSON-path form**: condition starts with `$` (COND:14-16), evaluated by `evaluate_json_path_condition` (COND:24-42). `$.path.to.field <op> <value>`.
  - Path segments are `.`-separated with empty segments filtered (**COND:94**, not 96); an all-empty path returns `None` ⇒ `false` (COND:96-98). Array elements are numeric segments (`resolve_json_path`, COND:122-140, array branch at COND:130-135). **No bracket syntax, no wildcards, no recursive descent, no filters** (SYNTAX:389-393).
  - Invalid JSON (COND:25-28), missing key or out-of-range index (COND:36-39, COND:136) ⇒ `false`.
- **Direct form**: no leading `$` (COND:17-19), `evaluate_direct_condition` (COND:46-63). `<op> <value>`; the whole payload is parsed as `f64`. Non-numeric payload (COND:52-55) or comparand (COND:57-60) ⇒ `false`. `parse_op_value` (COND:107-119) uses `strip_prefix`, so the operator **must** be at the start.
- **Operators** — `ConditionOp` (COND:149-157) with tokens at **COND:161-170** (**[CORRECTION]**, the draft said 158-167). Scan order `parse_order()` (COND:70-80): `>=`, `<=`, `!=`, `==`, `>`, `<`. Exactly one comparison per condition — **no `AND`/`OR`/`NOT`** (SYNTAX:423-424).
- **[ADDED] Real trap the draft's "longest-token-first so `>=` never mis-parses as `>`" understates.** `parse_path_op_value` (COND:83-105) does `input.find(op.token())` — a **find-anywhere** scan in `parse_order` sequence, not a leftmost-token scan. The first operator in *priority* order that occurs *anywhere* in the string wins, including inside the comparand. So `$.msg == ">=x"` splits on the `>=` inside the quoted comparand, yielding path segments `["msg", "== \""]`, which resolve to nothing ⇒ `false`. Egeria should flag conditions whose comparand contains an operator token.
- **Comparison** (`compare_values`, COND:293-317): numeric first — `value_as_f64` (COND:319-325) accepts JSON numbers **and numeric strings**, so `{"value":"90"}` compares numerically against `90`. Falls back to string comparison with surrounding double quotes stripped from the comparand (COND:304-307). `value_as_string` (COND:327-333) renders booleans as `"true"`/`"false"`, `null` as `""`, and everything else via `Value::to_string()`. Ordering operators on strings are lexicographic (COND:312-315).

**Egeria consequence:** a syntactically invalid condition is *not* a parse error — it is a silently always-false trigger. Egeria may validate condition strings at parse time and report a Finding where upstream fails closed at run time; that is a strict improvement, provided the Egeria parser still *accepts* the file.

---

### 13. Complete worked examples

#### Absolute minimum loadable manifest

```toml
[sop]
name = "s"
description = "d"
```
Parse proof TYPES:1358-1376; load proof (with two extra keys) MOD:1658-1676. Loads; `validate_sop` warns "SOP has no triggers defined" (MOD:987) and "SOP has no steps (missing or empty SOP.md)" (MOD:990).

#### Documented example — SYNTAX:64-75, verbatim

```toml
[sop]
name = "deploy-prod"
description = "Production deploy with approval"
version = "1.0.0"
max_concurrent = 1
admission_policy = "hold"
max_pending_approvals = 8

[[triggers]]
type = "manual"
```

#### The one machine generator — PROCMEM:309-315

```rust
fn default_manifest_toml(name: &str, description: &str) -> String {
    format!(
        "[sop]\nname = \"{}\"\ndescription = \"{}\"\nversion = \"0.1.0\"\n\n[[triggers]]\ntype = \"manual\"\n",
        toml_escape(name), toml_escape(description)
    )
}
```

**[CORRECTION]** The draft listed `crates/zeroclaw-runtime/src/tools/mod.rs:2500` and `sop/engine.rs:11978` as "same shape" generators. They are **`#[cfg(test)]` fixtures**, not generators — tools/mod.rs:2492-2502 is inside `registered_sop_tools_persist_audit_trail` and writes a literal `canary` manifest. `default_manifest_toml` at PROCMEM:309 is the only generator; it is reached via `read_or_default_manifest` (PROCMEM:268-276).

#### Maximal manifest exercising every field and every trigger variant

```toml
[sop]
name = "everything"
description = "Exercises every manifest field"
version = "2.1.0"
priority = "high"
execution_mode = "step_by_step"
cooldown_secs = 300
max_concurrent = 3
deterministic = false
admission_policy = "coalesce"
max_pending_approvals = 4
agent = "release-bot"

[[triggers]]
type = "mqtt"
topic = "facility/+/pressure/#"
condition = "$.value > 85"

[[triggers]]
type = "amqp"
routing_key = "org.release.*.version.#"
condition = "$.project.name == \"bzip2\""

[[triggers]]
type = "filesystem"
path = "/var/inbox/**/*.json"
events = ["created", "modified"]
condition = "$.extension == \"json\""

[[triggers]]
type = "channel"
channel = "git"
alias = "main"
condition = "$.event_type == \"pull_request.opened\""

[[triggers]]
type = "calendar"
calendar_source = "microsoft365"
calendar_ids = ["primary", "team"]
condition = "$.organizer == \"ops\""

[[triggers]]
type = "cron"
expression = "0 */5 * * *"

[[triggers]]
type = "webhook"
path = "/sop/everything"

[[triggers]]
type = "peripheral"
board = "nucleo-f401re-0"
signal = "pin_3"
condition = "> 0"

[[triggers]]
type = "manual"

[[positions]]
step = 1
x = 320.5
y = -48.0

[[positions]]
step = 2
x = 640.0
y = 120.0
```

---

### 14. Source-vs-doc disagreements in this area (source wins)

1. **`priority_based` gating direction.** TYPES:48 says *"Critical/High → Auto, Normal/Low → Supervised."* ENGINE:5461-5469 does the opposite for Critical/High (`=> true`), with the inline `[SEC-FLIP]` note that the comment's behavior *"was `=> false`, an inversion that auto-ran the riskiest SOPs."* **Source (ENGINE:5461-5469) wins; TYPES:48 is stale.**
2. **`agent` "Required for headless triggers".** TYPES:505-508 says required; no enforcement exists (§10.4). **Source wins: optional and unvalidated.**
3. **"No `[[steps]]` in SOP.toml".** True of hand-authored docs and fixtures; false of the code (TYPES:593-594, MOD:431-432) **and false of the writer** (TYPES:663, MOD:966-968, exercised by MOD:1291/1312). **Source wins: `[[steps]]` is a real, parseable, writable, lower-precedence fallback.**
4. **"The manifest is undocumented by design."** SYNTAX:16-20 declines to enumerate manifest fields, but SYNTAX:27 names the identity triple, SYNTAX:31-49 documents three execution knobs, and SYNTAX:344 generates the full trigger-field index. The undocumented set is `priority`, `execution_mode`, `cooldown_secs`, `deterministic`-as-a-key, `agent`-as-a-key, `[[positions]]`, `[[steps]]` (§0). **[CORRECTION]** the draft claimed `name`/`description`/`version` were among the undocumented eight; they are named at SYNTAX:27 and appear in the SYNTAX:64-75 example.
5. **[ADDED] `fan-in/manual.md`.** The draft asserted every fan-in page is a generated projection; `manual.md` is entirely hand-written prose and `git.md` re-projects `channel`. Where a fan-in page's prose conflicts with TYPES:142-229, the enum wins — but `manual.md` in particular carries no field table at all and cannot be used as a field reference.

---

### 15. Open questions

1. **OPEN QUESTION — `toml::to_string_pretty` on a manifest whose steps carry `serde_json::Value`.** Resolved in part: plain steps serialize fine (MOD:1291, MOD:1312, both `unwrap()`), so the general "does `[[steps]]` serialize" question is **closed — yes**. What remains open is `SopStep::schema.input`/`schema.output` (TYPES:273/276), `capability_input` / `with` (TYPES:374), and `PlannedToolCall::args`/`pinned` (TYPES:293/296): TOML cannot represent a JSON `null`, and `steps` lacks `skip_serializing_if`. No upstream test serializes a manifest whose steps carry a `Value` containing `null`. **Egeria should parse `[[steps]]` but never emit it.** Escalate before writing any emitter.
2. **RESOLVED (was open) — `f64` from a TOML integer in `[[positions]]`.** `x = 320` (a TOML Integer) does deserialize into `StepPosition::x: f64`: the `toml` deserializer forwards an integer to `visit_i64`, and serde's `f64` visitor implements the integer visits. This is serde/`toml` behavior, not ZeroClaw code, so no upstream citation pins it — every upstream test uses float literals (MOD:1309: `x: 320.5, y: -48.0`). **Recommendation:** accept both integer and float syntax; the divergence risk is confined to layout data, which ADR-0003 already excludes from semantics.
3. **OPEN QUESTION — duplicate `[[positions]]` entries for the same `step`.** MOD:437-441 makes last-write-win a mechanical consequence of the loop, but no test or comment establishes it as intended. Reproduce the mechanical behavior; do not treat it as a contract.
4. **OPEN QUESTION — whether `name` must match the containing directory.** `load_sop` never checks (§11); `load_sop_by_name` implicitly assumes it; `validate_candidate` (PROCMEM:243-249) enforces it for proposals only. There is no upstream statement of intent for the ordinary load path. Egeria should surface the mismatch as a Finding rather than picking a side.
5. **PARTIALLY RESOLVED (was open) — TOML dialect.** The workspace pins `toml = "1.0"` (`Cargo.toml:112`) and the lockfile resolves it to **`toml 1.1.2+spec-1.1.0`** (`Cargo.lock:10403-10405`); `zeroclaw-runtime`'s dependency edge is `"toml 1.1.2+spec-1.1.0"` (`Cargo.lock:13692`, declared at `crates/zeroclaw-runtime/Cargo.toml:74`). So upstream accepts **TOML spec 1.1**, not 1.0. Nothing in the SOP code constrains the dialect further. Inline tables (`triggers = [{ type = "manual" }]`), dotted keys (`sop.name = "x"`), and multi-line strings are all valid and produce the same serde input, but are **never exercised upstream** — Egeria should accept them and treat them as untested territory. Whether Egeria should pin TOML 1.1 or 1.0 semantics (they differ on unicode bare keys, trailing commas in inline tables, newlines in inline tables, `\e`) is a **remaining OPEN QUESTION**, since the pin is a `Cargo.lock` accident rather than a documented decision.
6. **[ADDED] OPEN QUESTION — trigger array ordering significance.** `manifest.triggers` is copied verbatim into `Sop.triggers` (MOD:469) and matching iterates them, but no upstream code documents whether first-match or all-match semantics apply at dispatch, nor whether duplicate identical triggers are meaningful. Egeria should preserve order and not deduplicate.

---

### 16. Implementation checklist for the Egeria parser

- [ ] Deserialize four top-level keys `sop` / `triggers` / `positions` / `steps`; `sop` required, the other three default-empty.
- [ ] `[sop]`: **11** fields, exact names per §3; `name` + `description` required; defaults `version="0.1.0"`, `priority=normal`, `cooldown_secs=0`, `max_concurrent=1`, `deterministic=false`, `admission_policy=parallel`, `max_pending_approvals=0`; `execution_mode` and `agent` are `Option`.
- [ ] `execution_mode` absent ⇒ **do not default to `supervised` in the manifest model** — represent it as `None` and resolve against a caller-supplied default (MOD:460). Apply the `deterministic` override *first*, ahead of both (MOD:456-461).
- [ ] Enum values case-sensitive, strictly rejected when unknown (§7). Do **not** copy `parse_execution_mode`'s lenient trim/lowercase — that is the config path only.
- [ ] Triggers: internally tagged on `type`, **9** variants, field names verbatim, no aliases (§5.1).
- [ ] `webhook` and `cron` accept **no** `condition`; `manual` accepts no fields at all.
- [ ] No `deny_unknown_fields` semantics — accept and drop unknown keys (but emit a Finding; especially for `condition` on webhook/cron, `[sop.approval]`, and config-only keys in `[sop]`).
- [ ] `positions` → view data only; `step` is a step *number*; unmatched entries silently dropped; last duplicate wins; excluded from equality and the semantic hash per ADR-0003.
- [ ] `[[steps]]` parsed (all 19 fields, `with` ⇒ `capability_input`) but overridden whenever `SOP.md` exists; never emitted.
- [ ] Manifest-steps normalization (`number == 0` only, title fallback to `capability` then `kind`) differs from Markdown renumbering (positional, unconditional) — keep both.
- [ ] Distinct wildcard matchers for `mqtt` (`/`; `+` = one level; `#` = rest, and **does not absorb zero levels**) and `amqp` (`.`; `*` = one word; `#` = **zero or more** words, backtracking).
- [ ] Channel topic matching: `:` then `.` then `/`; channel case-insensitive, alias case-sensitive.
- [ ] Discovery: one level deep, non-recursive, `SOP.toml` as marker, sort by manifest `name`, warn-and-skip on any load error.
- [ ] `max_step_retries` (2) / `max_step_visits` (256) / `step_schema_enforce` (true) / `step_scope_enforce` (false) are analysis parameters with upstream defaults, not manifest fields.

---

# Part: Step grammar — `SOP.md`

## SOP.md Step-List Grammar — Implementation Specification (corrected & completed)

**Authority:** `/Users/hollow/Documents/Workflow Generator/egeria/external/zeroclaw/crates/zeroclaw-runtime/src/sop/mod.rs`, `parse_steps` (mod.rs:504–657) plus its state struct (mod.rs:659–716) and helpers (mod.rs:718–837).

This is the **only** SOP.md parser in the workspace. `grep -rn "fn parse_steps" crates/` returns exactly one definition (mod.rs:504); the other four hits are `#[test] fn parse_steps_*` (mod.rs:1390, 1414, 1621, 1638). `crates/zeroclaw-sop-graph/src/lib.rs` (479 lines) contains no markdown parsing — its only `steps` matches are doc-comment prose about `{{steps.N}}` bindings (zeroclaw-sop-graph/src/lib.rs:20, 73). Egeria must reimplement mod.rs:504–837 exactly.

**Signature:** `pub fn parse_steps(md: &str) -> Vec<SopStep>` (mod.rs:504). Returns `Vec`, **not** `Result`. No error channel, no line numbers, no diagnostics. Every malformed construct degrades silently. See §8.

**Exhaustiveness proof for the key table (§4).** `grep -n 'strip_prefix(' mod.rs` returns exactly **32** hits and `grep -n 'starts_with('` exactly **4**. They decompose completely:
- 25 in the bullet chain (mod.rs:555–633) = the 21 keys + 4 aliases,
- 4 in `parse_step_failure` (mod.rs:781, 782, 788, 789),
- 3 in `extract_bold_title` (mod.rs:830, 831, 832),
- 4 `starts_with`: mod.rs:513 (`"## "`), 553 (`"- "`), 567 (`requires_confirmation:`), 571 (`kind:`).

No prefix match exists anywhere else in the file. §4 is therefore provably complete.

---

### 1. Locating the step section

```rust
// mod.rs:512-526
if trimmed.starts_with("## ") {
    if trimmed.eq_ignore_ascii_case("## steps") || trimmed.eq_ignore_ascii_case("## Steps")
    {
        in_steps_section = true;
        continue;
    }
    // Any other ## heading ends the steps section
    if in_steps_section {
        // Flush pending step
        current.flush_into(&mut steps);
        in_steps_section = false;
    }
    continue;
}
```

| Question | Answer | Citation |
|---|---|---|
| Iteration unit | `for line in md.lines()` — `str::lines`, so `\r\n` is normalized (trailing `\r` stripped; `line.trim()` would strip it regardless). | mod.rs:509 |
| Line normalization | `let trimmed = line.trim();` — **all** subsequent matching is against the fully trimmed line. Indentation is never significant, anywhere. | mod.rs:510 |
| Heading match | `trimmed.eq_ignore_ascii_case("## steps")` — **case-insensitive (ASCII only)**. The second disjunct `\|\| trimmed.eq_ignore_ascii_case("## Steps")` is dead code, fully subsumed by the first. | mod.rs:514 |
| Exact form required | Equality, not prefix. Exactly `##`, exactly **one** space, exactly `steps`. `## Steps` ✅, `## STEPS` ✅. `##  Steps` (two spaces) ❌, `## Steps:` ❌, `## Steps ##` ❌, `###Steps` ❌, `### Steps` ❌, `# Steps` ❌. Trailing whitespace tolerated (`trim()`); leading indentation tolerated. | mod.rs:510, 514 |
| **CORRECTION — the ❌ cases are not inert** | `##  Steps`, `## Steps:`, `## Steps ##` all satisfy `starts_with("## ")` but fail the equality test, so they hit the **terminator** branch: they flush the open step and *close* the section (mod.rs:519–523). They do not merely "fail to open" it. `### Steps` / `# Steps` fail `starts_with("## ")` entirely and are ordinary lines. | mod.rs:513, 519–523 |
| **NEW — a bare `##` is not a heading** | `line.trim()` removes the trailing space, so a line whose content is `"## "` becomes `"##"`, which does **not** satisfy `starts_with("## ")`. An empty h2 falls through to body accumulation (§6) instead of terminating the section. | mod.rs:510, 513 |
| Terminator | Only a line that (after trim) `starts_with("## ")` and is not `## steps`. That is: **h2 only**. | mod.rs:513, 519–523 |
| `#` and `###` do **not** terminate | `"# Title".starts_with("## ")` is false; `"### Sub".starts_with("## ")` is false. An h1 or h3 after `## Steps` leaves the section **open** and is swallowed as step body (§6). Note `default_procedure_markdown` emits `# {name}` *before* `## Steps` (procedural_memory.rs:318), so the common h1 is discarded by the pre-section guard, not swallowed. | mod.rs:513 |
| Content before `## Steps` | Discarded: `if !in_steps_section { continue; }`. | mod.rs:528–530 |
| Content after a terminating h2 | Discarded, same guard. The pending step is flushed at the terminator. | mod.rs:520–523, 528–530 |
| EOF | Final pending step flushed unconditionally: `current.flush_into(&mut steps);` after the loop. A section left open at EOF still yields its last step. | mod.rs:653–654 |
| Double flush is safe | `flush_into` opens with `let Some(n) = self.number.take() else { return; };` and ends with `*self = Self::default();` (mod.rs:690–692, 714), so flush-at-terminator followed by flush-at-EOF emits nothing extra. | mod.rs:689–715 |
| Any `## ` line is consumed | The `continue` at mod.rs:525 fires for *every* line matching `starts_with("## ")`, so such a line can never become body text even mid-step. | mod.rs:525 |
| Repeated `## Steps` | Steps **accumulate into one list** across multiple `## Steps` sections; numbering continues monotonically (§2.1). Note the asymmetry: the `## Steps` branch `continue`s at mod.rs:517 **before** the flush at mod.rs:522, so a second `## Steps` immediately following an open step does **not** flush it — the step stays open and keeps absorbing content. | mod.rs:514–518 vs 520–523 |
| Code-fence awareness | **None.** The parser has no fence state (verified: no `` ``` `` or `fence` token anywhere in mod.rs:504–837). A `## Steps` inside a fence opens the section; a `## Foo` inside a fence closes it. | mod.rs:509–526 |

> **Implementer note.** Egeria may emit a diagnostic for a fenced or duplicated `## Steps`, but must not change the accepted set without recording a divergence.

---

### 2. Recognizing a step item

```rust
// mod.rs:808-817
/// Try to parse `N. rest` from a line, returning `rest` if successful.
fn parse_numbered_item(line: &str) -> Option<&str> {
    let dot_pos = line.find(". ")?;
    let prefix = &line[..dot_pos];
    if prefix.chars().all(|c| c.is_ascii_digit()) && !prefix.is_empty() {
        Some(line[dot_pos + 2..].trim())
    } else {
        None
    }
}
```

**Accepted syntax — `N.` followed by a space, and nothing else.**

- `1)` paren form: **not accepted.** Only the literal `". "` is searched (mod.rs:810); there is no `)` handling in mod.rs.
- `-` / `*` / `+` as a *step* marker: **not accepted.**
- `1.` with **no** trailing space (`1.**Deploy**`): **not accepted** — `find(". ")` requires the space.
- **NEW — a bare `1.` on its own line is not a step.** `line.trim()` strips the trailing space first, leaving `"1."`, which has no `". "`. So `1.` with an empty remainder never opens a step. | mod.rs:510, 810
- Digits must be **ASCII** (`is_ascii_digit`, mod.rs:812), prefix non-empty. Leading zeros (`007. Foo`) accepted lexically; the value is discarded anyway.
- The remainder is trimmed: `line[dot_pos + 2..].trim()` (mod.rs:813), so `1.   **T**` is fine.
- Indentation is irrelevant — `parse_numbered_item(trimmed)` (mod.rs:533). **A nested ordered list inside a step body starts a new step.**
- `find(". ")` finds the **first** `". "`. Hazard: a body continuation line such as `2024. was a good year` parses as a step item. `Run v1. Then check.` does not (prefix `Run v1` is not all-digits).
- **Ordering:** the numbered check (mod.rs:533) runs *before* the bullet check (mod.rs:553). A line like `- 1. foo` fails the numbered check (prefix `- 1` is not all-digits) and falls through to the bullet branch, where `1. foo` matches no key and becomes body text carrying its `- ` marker (§6).

#### 2.1 The number is read, then thrown away — positional renumbering

```rust
// mod.rs:533-540
if let Some(rest) = parse_numbered_item(trimmed) {
    // Flush previous step
    current.flush_into(&mut steps);

    let step_num = u32::try_from(steps.len())
        .unwrap_or(u32::MAX)
        .saturating_add(1);
    current.reset_for_step(step_num);
```

`parse_numbered_item` returns only `rest` — **the digits are never parsed into an integer.** `step_num` derives purely from `steps.len()` *after* the previous step was flushed, so emitted numbers are always exactly `1, 2, 3, … N` in file order. `reset_for_step` (mod.rs:682–687) does `*self = Self { number: Some(number), ..Self::default() }`, so **every** field resets — no state leaks between steps.

**Consequences the implementer must reproduce:**

1. **Gaps and out-of-order numbers are silently normalized.** `1.`, `5.`, `7.` → `1, 2, 3`. `3.`, `1.`, `2.` → `1, 2, 3` **in file order**; the written number contributes nothing, not even to sort. Confirmed end-to-end by the round-trip test at mod.rs:1480–1483, where a step rendered as `2. **wait**` comes back as `parsed[0]`.
2. **The "numbering gap" diagnostic is structurally dead on the SOP.md path.**
   ```rust
   // mod.rs:993-1001
   // Check step numbering continuity
   for (i, step) in sop.steps.iter().enumerate() {
       let expected = u32::try_from(i).unwrap_or(u32::MAX).saturating_add(1);
       if step.number != expected {
           warnings.push(format!(
               "Step numbering gap: expected {expected}, got {}",
               step.number
           ));
   ```
   This compares `step.number` against `i + 1` — the identical formula `parse_steps` used to assign it (mod.rs:537–539). For any SOP loaded from SOP.md the predicate is unsatisfiable. `docs/book/src/sop/example.md:59` advertises the warning; it can never fire for a markdown-authored SOP. **The Rust source wins.**
   - *Only live path:* `SopManifest` carries `#[serde(default)] pub steps: Vec<SopStep>` (types.rs:593–594), used **only when SOP.md is absent** (mod.rs:428–435). `normalize_manifest_steps` (mod.rs:483–496) renumbers **only** steps whose `number == 0` (mod.rs:485–487) — **and, CORRECTION/ADDITION, also backfills an empty `title` from `capability` or `kind.to_string()` (mod.rs:488–493)**, a fixup that has no SOP.md analogue. So a `[[steps]]`-in-TOML SOP with explicit `number = 5` *can* trip mod.rs:996. `[[steps]]` is not a documented authoring surface and is dead whenever SOP.md exists, but it is deserializable and `SopManifest::from_sop` **does** write it back (`steps: sop.steps.clone()`, types.rs:663) — so a `save_sop` round-trip puts the full step list into SOP.toml *and* SOP.md, with SOP.md winning on the next load. Egeria, parsing SOP.md, should treat the gap warning as its **own** rule against the author-written digits (which upstream discards), documented as a deliberate divergence.
3. **All cross-references are positional, not literal.** `next:`, `depends_on:`, `on_failure: goto:N`, and switch `goto` are `u32`s compared against the *renumbered* values. Steps `1.`, `5.`, `7.` with `- next: 5` produce `routing.next = Some(5)` against a 3-step list.
   - **CORRECTION to the draft:** upstream does not detect this *in `parse_steps`*, but it is **not** undetected overall. `validate_sop_strict` (mod.rs:1088–1123) builds `SopGraph::from_sop` (mod.rs:1108) and promotes its diagnostics: `next target step {next} does not exist` → **Error/blocking** (graph.rs:314–318), `depends_on target step {dep} does not exist` → **Error** (graph.rs:349–354), `on_failure goto target step {target} does not exist` → **Error** (graph.rs:368–373), `switch port '{name}' target step {target} does not exist` → **Error** (graph.rs:400–407), `switch port '{name}' has no target` → **Warning** (graph.rs:410–415), and `step has switch rules and a routing.next target; next is ignored because switch resolution takes precedence` → **Warning** (graph.rs:376–385). `validate_sop_strict` also blocks on empty titles (`.trim().is_empty()`, mod.rs:1097–1099) and `Duplicate step number` (mod.rs:1100–1102). None of this runs on the *load* path (mod.rs:421–481); it runs on the *save* path (mod.rs:958–961) and wherever a caller invokes it. Egeria should place these as `EGR-*` rules and note that upstream classifies dangling `next`/`depends_on`/`goto` as **blocking**, and a `goto`-less switch port as a **warning**.
   - `normalize_step_numbers` (mod.rs:336–374) remaps such references, but runs only inside `save_sop` (mod.rs:956) and bails entirely on duplicate numbers (mod.rs:337–340). Its remap drops references to steps that no longer exist, and `StepFailure::Goto` falls back to `Fail` (mod.rs:364–369).

---

### 3. Title extraction

```rust
// mod.rs:542-548
// Extract title from bold text: **title** — body
if let Some((title, body)) = extract_bold_title(rest) {
    current.title = title;
    current.body = body;
} else {
    current.title = rest.to_string();
}
```
```rust
// mod.rs:819-837
pub fn extract_bold_title(text: &str) -> Option<(String, String)> {
    let start = text.find("**")?;
    let after_start = start + 2;
    let end = text[after_start..].find("**")?;
    let title = text[after_start..after_start + end].to_string();

    // Rest is everything after the closing ** and any separator (— or -)
    let rest_start = after_start + end + 2;
    let rest = text[rest_start..].trim();
    let rest = rest
        .strip_prefix("—")
        .or_else(|| rest.strip_prefix("–"))
        .or_else(|| rest.strip_prefix("-"))
        .unwrap_or(rest)
        .trim();

    Some((title, rest.to_string()))
}
```

| Case | Behavior | Citation |
|---|---|---|
| `1. **Deploy** - Ship it.` | `title = "Deploy"`, `body = "Ship it."` | mod.rs:820–836 |
| **No separator at all** | `1. **Resolve** Do the first step` → `title = "Resolve"`, `body = "Do the first step"`. The separator is optional; `strip_prefix` chain falls through to `unwrap_or(rest)`. This exact shape appears in-tree at tools/mod.rs:2505. | mod.rs:829–834 |
| **Bold absent entirely** | `None` at the first `?`; the whole remainder becomes the title and **body starts empty**: `1. Just prose here` → `title = "Just prose here"`, `body = ""`. | mod.rs:821, 546–548 |
| **Unterminated bold** (`1. **Deploy`) | `None` at the second `?`. Title = `"**Deploy"` — literal asterisks retained. | mod.rs:823, 547 |
| **Bold mid-line** | The doc comment says "from the beginning", but the code is `text.find("**")` — **anywhere**. `1. Do the **thing** now` → `title = "thing"`, `body = "now"`. **Everything before the opening `**` is silently discarded.** The single most surprising title behavior; reproduce it. | mod.rs:819 (comment) vs 821 (code) |
| **Empty bold** (`1. ****`) | `Some(("".into(), "".into()))` — empty title, which later trips `"Step {} has an empty title"` (warning, mod.rs:1002–1004) and blocks `save_sop` (mod.rs:1097–1099). | mod.rs:821–824 |
| **Title whitespace** | The title slice is **not trimmed** (mod.rs:824). `1. ** Spaced **` yields `title = " Spaced "`. `SopStep.title` is never trimmed on flush either (`std::mem::take`, mod.rs:695). Note `validate_sop` tests `is_empty()` (mod.rs:1002) while `validate_sop_strict` tests `trim().is_empty()` (mod.rs:1097) — a whitespace-only title warns nowhere but blocks a save. | mod.rs:824, 695, 1002, 1097 |
| Separator stripping | Exactly **one** prefix, tried in order: `—` U+2014, `–` U+2013, `-` U+002D. Not `:`, not `·`, not `−` U+2212, not `--` (strips one hyphen, leaving `- foo`). Trimmed before and after. | mod.rs:829–834 |
| Third `**` and beyond | Ignored; only the first pair is consumed, the rest lands verbatim in the body. `1. **A** and **B** - x` → `title="A"`, `body="and **B** - x"`. | mod.rs:821–828 |
| Byte-index safety | All slicing is at `find` results ±2 (`"**"` is ASCII), so no char-boundary panic. Egeria must index by bytes, not chars, to match `find` semantics on multibyte titles. | mod.rs:821–827 |

---

### 4. Complete bullet-key table

**Recognition precondition** (mod.rs:553–554):
```rust
if current.number.is_some() && trimmed.starts_with("- ") {
    let bullet = trimmed.trim_start_matches("- ").trim();
```
- Requires an **open step**. Bullets between `## Steps` and the first `N.` item hit neither branch (mod.rs:553 and 645 both gate on `current.number.is_some()`) and are **silently discarded**.
- Marker gate is `trimmed.starts_with("- ")` — hyphen + **space**. `* tools: x`, `+ tools: x`, `-tools: x`, and `-\ttools: x` are **not** bullets; they fall to §6 body accumulation.
- **CORRECTION — extra spaces after the hyphen are tolerated.** `trim_start_matches("- ")` strips repeated `"- "` occurrences, then `.trim()` removes what is left. `-  tools: x` (hyphen, two spaces) → strip one `"- "` → `" tools: x"` → `.trim()` → `"tools: x"` → **recognized**. Likewise `- - - tools: a` → `tools: a`. The draft's "exactly `- `" understates the accepted set.
- Matching is `strip_prefix`/`starts_with` against a lowercase literal *including the colon*. See §7.

Enumerated from the `if`/`else if` chain at **mod.rs:555–633**. **21 keys, 25 accepted spellings** (proof at the top of this document).

| # | Key (all accepted spellings) | Aliases verified | Target field | Value grammar | Empty-value behavior | Source |
|---|---|---|---|---|---|---|
| 1 | `tools:` | — (no `tool:`, no hyphen form) | `suggested_tools: Vec<String>` (types.rs:330) | `parse_csv_list`: split `,`, trim each, drop empties | `[]` (replaces) | mod.rs:555–556; 718–724 |
| 2 | `allow-tools:` **or** `allow_tools:` | ✅ both, mod.rs:558–559 | `scope.allow: Option<Vec<String>>` (scope/mod.rs:14) | `parse_csv_list`, wrapped in `Some(..)`; `ensure_scope` materializes `scope` | `Some([])`, and `scope` becomes `Some(..)` | mod.rs:557–561; 804–806 |
| 3 | `deny-tools:` **or** `deny_tools:` | ✅ both, mod.rs:563–564 | `scope.deny: Vec<String>` (scope/mod.rs:17) | `parse_csv_list` | `[]` — **but `scope` still becomes `Some(StepToolScope{allow:None, deny:[]})`** via `ensure_scope`; the draft omitted this | mod.rs:562–566; 804–806 |
| 4 | `requires_confirmation:` | ❌ **no** `requires-confirmation:` — asymmetric with #2/#3/#13/#15 | `requires_confirmation: bool` (types.rs:334) | `val.trim().eq_ignore_ascii_case("true")`; **anything else → `false`** | `false` | mod.rs:567–570 |
| 5 | `kind:` | — | `kind: SopStepKind` (types.rs:337) | `parse_step_kind`: trim + `to_ascii_lowercase`, then `"checkpoint" \| "approval" → Checkpoint`, `"capability" → Capability`, **`_` → `Execute`** (so `execute` *and* every typo land on Execute) | `Execute` | mod.rs:571–574; 753–759; types.rs:243–251 |
| 6 | `capability:` | — | `capability: Option<String>` (types.rs:371) | `Some(val.trim().to_string())` — raw string, no validation here | `Some("")` — **not** `None` | mod.rs:575–576 |
| 7 | `with:` | — | `capability_input: Option<Value>` (types.rs:373–374, serde `rename = "with"`) | `parse_value_fragment` (§5.3) | `Some(String(""))` | mod.rs:577–578 |
| 8 | `input:` | — | `schema.input: Option<Value>` (types.rs:273) | `parse_value_fragment`; `ensure_schema` creates `StepSchema` on demand | `Some(String(""))`, schema materialized | mod.rs:579–580; 797–802 |
| 9 | `output:` | — | `schema.output: Option<Value>` (types.rs:276) | `parse_value_fragment` | `Some(String(""))`, schema materialized | mod.rs:581–582; 797–802 |
| 10 | `when:` | — | `routing.when: Option<String>` (step_contract.rs:26) | Raw trimmed string, **no expression parsing at this layer** | **No-op** — guarded by `if !val.is_empty()`; an empty value cannot clear a prior `when:` | mod.rs:583–587 |
| 11 | `next:` | — | `routing.next: Option<u32>` (step_contract.rs:29) | `val.trim().parse::<u32>().ok()` | `None` — **assigns**, so `- next:` or `- next: abc` *clears* a prior value | mod.rs:588–589 |
| 12 | `terminal:` | — | `routing.terminal: bool` (step_contract.rs:34) | `eq_ignore_ascii_case("true")`; else `false` | `false` | mod.rs:590–591 |
| 13 | `depends_on:` **or** `depends-on:` | ✅ both, mod.rs:593–594 | `routing.depends_on: Vec<u32>` (step_contract.rs:37) | `parse_u32_list`: split `,`, trim, `filter_map(parse::<u32>().ok())` — non-numeric entries silently dropped | `[]` | mod.rs:592–596; 726–731 |
| 14 | `switch:` | — | `routing.switch: Vec<SwitchRule>` (step_contract.rs:41) | `parse_switch_rules` (§5.1) | `[]` | mod.rs:597–598; 733–751 |
| 15 | `on_failure:` **or** `on-failure:` | ✅ both, mod.rs:600–601 | `on_failure: StepFailure` (types.rs:349) | `parse_step_failure` (§5.2) | `Fail` | mod.rs:599–603; 775–795 |
| 16 | `mode:` | — | `mode: Option<SopExecutionMode>` (types.rs:352) | `Some(parse_execution_mode(val))`. **Always `Some`.** Unknown/empty → `Some(Supervised)`, *not* `None`. Accepted: `auto`, `step_by_step`, `priority_based`, `deterministic`, `supervised`; trim + full `to_lowercase` | `Some(Supervised)` | mod.rs:604–605; 170–179 |
| 17 | `agent:` | — | `agent: Option<String>` (types.rs:368) | `(!v.is_empty()).then(...)` | **`None`** — empty value *clears* | mod.rs:606–608 |
| 18 | `call:` | — | `calls: Vec<PlannedToolCall>` (types.rs:356) — **push**, not replace | Strict single-line `serde_json::from_str::<PlannedToolCall>`; `{tool: String, args: Value (`#[serde(default)]`), pinned: Option<Value>}` | On **any** JSON error the bullet is **silently dropped entirely** — `if let Ok(call)`, no `else`, no body fallback | mod.rs:609–612; types.rs:289–297 |
| 19 | `prompt:` | — | `gate_prompt: Option<String>` (types.rs:386) | Raw trimmed string | **No-op** on empty (same guard shape as `when:`) | mod.rs:613–617 |
| 20 | `policy:` | — | `policy: Option<String>` (types.rs:379) | Raw trimmed string | **`None`** — empty clears | mod.rs:618–624 |
| 21 | `edit:` | — | `edit: Option<String>` (types.rs:391) | Raw trimmed string (a field name) | **`None`** — empty clears | mod.rs:625–633 |

> **Citation corrections to the draft.** The draft's `types.rs` line numbers for the last four fields were off. Correct anchors: `agent` 367–368, `capability` 369–371, `capability_input` (`with`) 372–374, `policy` 375–379, `gate_prompt` 380–386, `edit` 387–391; `SopStep` spans 311–392 (not 308–386); `PlannedToolCall` 289–297; `StepSchema` 270–277; `SopStepKind` 243–251 with `Display` at 253–261.

#### 4.1 — MISSING FROM THE DRAFT: `suggested_tools` is a legacy alias for `scope.allow`

`tools:` is not an independent hint field. `SopStep::effective_tool_scope` (types.rs:435–444) clones `scope` and, when `suggested_tools` is non-empty **and `scope.allow` is `None`**, fills `allow` from `suggested_tools`:

```rust
// types.rs:435-444
pub fn effective_tool_scope(&self) -> Option<StepToolScope> {
    let mut scope = self.scope.clone();
    if !self.suggested_tools.is_empty() {
        let scope = scope.get_or_insert_with(StepToolScope::default);
        if scope.allow.is_none() {
            scope.allow = Some(self.suggested_tools.clone());
        }
    }
    scope
}
```

The unit test asserts exactly this: `- tools: read_file, shell` alone leaves `step.scope == None` but `effective_tool_scope().allow == Some(["read_file","shell"])` (mod.rs:1400–1408). Field doc: "Legacy alias for `scope.allow`; when `step_scope_enforce` is off these are hints, not a hard restriction" (types.rs:326–328). **Precedence: an explicit `allow-tools:` wins; `tools:` only fills an absent `allow`.** Egeria's model must carry both fields separately *and* reproduce this derivation, or tool-scope analysis will be wrong for the most common authoring form (which is the one syntax.md:114, 117, 146, 151 uses).

`effective_agent` is the parallel derivation for `agent:`: step override, else the SOP's parent `agent` (types.rs:448–450; parent from `SopMeta.agent`, types.rs:629–632).

#### 4.2 Alias asymmetry — do not assume symmetry

Exactly **four** aliases, not systematic:
- `allow_tools:` (mod.rs:559), `deny_tools:` (mod.rs:564) — underscore forms of hyphen-canonical keys.
- `depends-on:` (mod.rs:594), `on-failure:` (mod.rs:601) — hyphen forms of underscore-canonical keys.

There is **no** `requires-confirmation:`, and no alias for `tools:`, `kind:`, `capability:`, `with:`, `input:`, `output:`, `when:`, `next:`, `terminal:`, `switch:`, `mode:`, `agent:`, `call:`, `prompt:`, `policy:`, `edit:`. Guaranteed by the 32/4 prefix-call census above.

#### 4.3 Chain-order and prefix collisions

Chain order: `tools → allow-tools → deny-tools → requires_confirmation → kind → capability → with → input → output → when → next → terminal → depends_on → switch → on_failure → mode → agent → call → prompt → policy → edit` (mod.rs:555–633). **No key is a prefix of another** once the trailing colon is included, so order is semantically irrelevant and Egeria may use hash-map dispatch. (`capability:` preceding `call:` is not a collision — `"capability: x"` does not start with `"call:"`.)

#### 4.4 Repeated bullets on one step

| Behavior | Keys |
|---|---|
| **Last wins** (scalar assign) | `requires_confirmation`, `kind`, `capability`, `with`, `input`, `output`, `next`, `terminal`, `mode`, `agent`, `policy`, `edit` |
| **Last wins, but an empty value is a no-op** | `when`, `prompt` |
| **Replaces the whole list** | `tools`, `allow-tools`, `deny-tools`, `depends_on`, `switch` |
| **Accumulates** | `call` — the only append-semantics key (`current.calls.push(call)`, mod.rs:611) |

#### 4.5 Undocumented keys (source wins over `syntax.md`)

`docs/book/src/sop/syntax.md` (441 lines) documents `tools`, `requires_confirmation`, `kind`, `allow-tools`, `deny-tools`, `input`, `output`, `when`, `next`, `depends_on`, `on_failure`, `mode`, `policy` (syntax.md:156–179), plus `edit` (syntax.md:234) and `with` (syntax.md:260, 273).

**No bullet-key occurrence anywhere in `docs/book/src/sop/`** for: `switch:`, `terminal:`, `prompt:`, `call:`, `agent:` as a bullet, and `capability:` as a *sub-bullet*. Verified greps:
- `switch` → one repo-wide hit, `fan-in/channel.md:5`, prose about a "per-channel dispatch switch"; **zero** in syntax.md.
- `terminal` → one hit, `example.md:59`, "terminal interface".
- `prompt:` → zero across the whole `sop/` doc tree.
- **CORRECTION:** `call:` is not literally zero — `grep -rn "call:"` matches `syntax.md:193` inside `escalation_route = "discord.oncall:987654321098765432"`. As a bullet key it is zero.
- `agent:` → two hits, `syntax.md:83` and `syntax.md:93`, both the `agent:<alias>` *approver-identity* syntax, unrelated to the step bullet.
- `capability:` → `syntax.md:272` and `syntax.md:275`, both on a **title line** (see below), never as a sub-bullet.
- The four aliases `allow_tools`, `deny_tools`, `depends-on`, `on-failure`: **zero** doc occurrences.

**Additional doc bug — the capability example does not parse as written.** syntax.md:271–276:
```md
1. **Draft** - kind: capability / capability: llm.generate
   - with: { instruction = "...", output_key = "body", echo = ["repo", "number"] }
2. **Approve** - kind: checkpoint / policy: triage
3. **Post** - kind: capability / capability: forge.comment
```
Under mod.rs:543–548, `extract_bold_title` yields `title = "Draft"` and `body = "kind: capability / capability: llm.generate"`. The step's `kind` stays `Execute` and `capability` stays `None`; only the `with:` sub-bullet is honored. Step 2's `policy: triage` is likewise swallowed into the body. The working form is the unit test at mod.rs:1638–1656:
```
1. **Status** - Check the repository.
   - kind: capability
   - capability: git.status
   - with: { require_clean = true }
```
asserting `kind == Capability`, `capability == Some("git.status")`, `capability_input == json!({"require_clean": true})` (mod.rs:1650–1655). **The Rust source wins.** Egeria must require these as sub-bullets, and should emit a finding when `kind:`/`capability:`/`policy:` text appears in a step *body*, since that is a silent authoring failure upstream ships in its own docs.

---

### 5. Value grammars for the structurally complex keys

#### 5.1 `switch:` — `name>when>goto`, `;`-separated

```rust
// mod.rs:733-751
fn parse_switch_rules(value: &str) -> Vec<SwitchRule> {
    value
        .split(';')
        .filter_map(|seg| {
            let mut parts = seg.splitn(3, '>');
            let name = parts.next().unwrap_or("").trim().to_string();
            if name.is_empty() {
                return None;
            }
            let when = parts.next().unwrap_or("").trim();
            let goto = parts.next().unwrap_or("").trim();
            Some(SwitchRule {
                name,
                when: (!when.is_empty()).then(|| when.to_string()),
                goto: goto.parse::<u32>().ok(),
            })
        })
        .collect()
}
```

Grammar: `rule (';' rule)*` where `rule := name ['>' when ['>' goto]]`.

- Segment separator `;`; field separator `>`; every field trimmed. **Order is preserved and semantically load-bearing** — rules are evaluated top to bottom, first match wins (step_contract.rs:3–6), and the catch-all "should be ordered last" (step_contract.rs:5–6).
- **Empty `name` → the whole segment is dropped** (`return None`, mod.rs:739–741). A trailing `;` is therefore harmless.
- Empty `when` → `None` = catch-all port (step_contract.rs:12: "`None` = catch-all").
- Empty or non-numeric `goto` → `None`. Silently at parse time; `SopGraph` later emits the **warning** `switch port '{name}' has no target` (graph.rs:410–415).
- **`splitn(3, '>')` is a hard trap.** The third field absorbs every remaining `>`. `- switch: hot>$.n > 5>3` splits to `["hot", "$.n ", " 5>3"]`; `" 5>3".trim().parse::<u32>()` fails, so `goto = None` and the guard is silently truncated to `"$.n"`. **A `>` comparison operator cannot appear in a switch `when` expression.** No parse-time diagnostic; the downstream signal is only the generic "has no target" warning.
- Canonical example (test, mod.rs:1426, asserted at mod.rs:1454–1468): `- switch: pull_request>$.event>3; catch_all>>2`.
- **Downstream precedence, for context:** a non-empty `switch` supersedes `routing.next` entirely (`resolve_next` doc tree, route/mod.rs:31–49, esp. 38–42), and `SopGraph` warns when both are set (graph.rs:376–385). A **false** top-level `when:` bypasses `switch` and `next` alike and takes the linear successor, unless `terminal: true`, in which case the run completes (route/mod.rs:34–38, 68–76). This resolves the syntax.md:165–167 vs 170–171 contradiction in favor of lines 170–171; **the Rust source wins.**

#### 5.2 `on_failure:` — `fail | retry:N | retry N | goto:M | goto M`

```rust
// mod.rs:775-795
fn parse_step_failure(value: &str) -> StepFailure {
    let value = value.trim();
    if value.eq_ignore_ascii_case("fail") {
        return StepFailure::Fail;
    }
    if let Some(max) = value
        .strip_prefix("retry:")
        .or_else(|| value.strip_prefix("retry "))
        .and_then(|raw| raw.trim().parse::<u32>().ok())
    {
        return StepFailure::Retry { max };
    }
    if let Some(step) = value
        .strip_prefix("goto:")
        .or_else(|| value.strip_prefix("goto "))
        .and_then(|raw| raw.trim().parse::<u32>().ok())
    {
        return StepFailure::Goto { step };
    }
    StepFailure::Fail
}
```

- `fail` is **case-insensitive** (mod.rs:777); `retry`/`goto` prefixes are **case-sensitive** (plain `strip_prefix`, mod.rs:781–782, 788–789). `RETRY:2` falls through → `Fail`.
- Two forms each: colon (`retry:2`) and space (`retry 2`). The value after the prefix is trimmed (mod.rs:783, 790), so `retry: 2` and `retry:  2` both work — this is what makes the printer's `format!("retry: {max}")` (mod.rs:842) round-trip. Note the printer emits the *space-after-colon* form for both (mod.rs:842–843).
- **Every unparseable value degrades to `StepFailure::Fail`** (mod.rs:794), which is also the `#[default]` (step_contract.rs:55–56). `retry:-1`, `banana`, `goto:x`, and an empty value all become `Fail`. No diagnostic.
- `retry: 0` is accepted as `Retry { max: 0 }`.
- `goto` target is a **positional** step number (§2.1); a dangling target is a **blocking** error in `validate_sop_strict` via graph.rs:368–373.
- Docs (syntax.md:172–173) list only `fail`, `retry:<count>`, `goto:<step>` — the space forms are undocumented. **Source wins.**

#### 5.3 `input:` / `output:` / `with:` — JSON, then TOML, then string

```rust
// mod.rs:761-773
fn parse_value_fragment(value: &str) -> serde_json::Value {
    if let Ok(json) = serde_json::from_str(value) {
        return json;
    }
    let wrapped = format!("value = {value}");
    if let Ok(toml_value) = toml::from_str::<toml::Value>(&wrapped)
        && let Some(value) = toml_value.get("value")
        && let Ok(json) = serde_json::to_value(value)
    {
        return json;
    }
    serde_json::Value::String(value.into())
}
```

Three-stage fallback, **always single-line** (the bullet is one `md.lines()` line; there is no continuation or multiline support anywhere in mod.rs:504–657). The argument arrives already trimmed at the call sites (mod.rs:578, 580, 582).

1. **JSON** — `serde_json::from_str`. Any JSON value, not just objects: `- input: 42` → `Number(42)`, `- input: true` → `Bool(true)`, `- input: "x"` → `String("x")`, `- input: null` → `Null`, `- input: [1,2]` → `Array`.
2. **TOML right-hand side** — the fragment is wrapped as `value = <frag>` and parsed as TOML. This is what makes `- with: { require_clean = true }` work (test mod.rs:1645, asserted mod.rs:1652–1655). **NEW — two consequences the draft missed, both real and surprising:**
   - **TOML comments are honored.** `- input: 1 # note` fails JSON, but `value = 1 # note` is valid TOML → result is `Number(1)`; the `# note` is silently discarded. Any fragment containing `#` after an otherwise-valid TOML value loses its tail.
   - **TOML literal (single-quoted) strings parse.** `- input: 'abc'` fails JSON but `value = 'abc'` is TOML → `String("abc")` with the quotes **removed**, unlike the stage-3 fallback which would keep them.
3. **Bare string** — anything else becomes `Value::String(raw)`, *including the empty string*. `- input: not json at all` → `String("not json at all")`. **There is no such thing as an invalid `input:`/`output:`/`with:` value.**

Schema *shape* validation ("compact JSON object with `type`, `required`, `properties`, `items`"; primitives `object|array|string|number|integer|boolean|null`, syntax.md:291–294) happens later in `sop/schema/`, not here. Egeria's parser must accept anything and defer shape checks.

#### 5.4 `depends_on:` — comma-separated `u32`

```rust
// mod.rs:726-731
fn parse_u32_list(value: &str) -> Vec<u32> {
    value
        .split(',')
        .filter_map(|item| item.trim().parse::<u32>().ok())
        .collect()
}
```
Separator is `,` **only** (not whitespace, not `;`). Each item trimmed. **`filter_map` silently drops** every non-numeric item: `- depends_on: 1, two, 3` → `[1, 3]`, no warning. Duplicates preserved, order preserved. Values are positional step numbers (§2.1); dangling values are **blocking** at graph.rs:349–354.

#### 5.5 `tools:` / `allow-tools:` / `deny-tools:` — comma-separated strings

```rust
// mod.rs:718-724
fn parse_csv_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect()
}
```
Separator is `,` **only** — **not whitespace.** `- tools: read_file shell` yields the single entry `"read_file shell"`. Each entry trimmed; empty entries dropped (`a,,b` → `["a","b"]`; a trailing comma is harmless). No quoting, no escaping — a tool name containing a comma is unrepresentable. Entries may be tool names *or* group names (scope/mod.rs:12, expanded downstream by `expand_entries`, scope/mod.rs:32–35).

Model distinction Egeria must preserve: `allow-tools:` sets `scope.allow = Some(vec)` — an empty-but-present allow-list (`Some([])`) is **not** the same as absent (`None`), per `Option<Vec<String>>` at scope/mod.rs:14, and `resolve_excluded` branches on exactly that (`allow.as_ref().is_none_or(...)`, scope/mod.rs:~55). `deny` is a plain `Vec` (scope/mod.rs:17), so absent and empty are indistinguishable. And per §4.1, an absent `allow` may still be filled from `suggested_tools`.

---

### 6. Body accumulation

Two paths reach the body, both gated on an open step.

**Path A — unrecognized sub-bullet** (mod.rs:634–640), the `else` of the key chain:
```rust
} else {
    // Continuation body line
    if !current.body.is_empty() {
        current.body.push('\n');
    }
    current.body.push_str(trimmed);
}
```
**Path B — plain continuation line** (mod.rs:644–650):
```rust
// Continuation line for step body
if current.number.is_some() && !trimmed.is_empty() {
    if !current.body.is_empty() {
        current.body.push('\n');
    }
    current.body.push_str(trimmed);
}
```

| Construct | Result | Citation |
|---|---|---|
| Continuation prose | Appended, `\n`-joined | mod.rs:645–650 |
| **Unrecognized bullet** (`- notakey: x`) | Appended **including its `- ` marker** — Path A pushes `trimmed`, not the stripped `bullet` local. Body line is `"- notakey: x"`. Never an error. | mod.rs:554 vs 639 |
| Nested / deeper-indented bullet (`  - sub`) | Indentation already stripped at mod.rs:510, so it is indistinguishable from a top-level bullet: parsed as a **key bullet** if it matches a key, else appended as body with its `- `. **Nesting depth is not modeled at all.** | mod.rs:510, 553 |
| `* item` / `+ item` / `-item` | Not bullets; appended verbatim via Path B. | mod.rs:553, 645 |
| **Blank lines** | **Dropped.** Path B's `!trimmed.is_empty()` guard; a blank line can never reach Path A (it does not start with `- `). **The body never contains an empty line — all paragraph breaks are destroyed.** | mod.rs:645 |
| **Leading/trailing whitespace on each line** | **Destroyed.** Every appended line is `trimmed` (mod.rs:510). Indented code, nested list alignment, hanging indents all flattened. | mod.rs:510, 639, 649 |
| **Code fences** | **No fence awareness whatsoever.** The `` ``` `` delimiter lines are ordinary body lines (indentation lost). Inside a fence: a line starting with `- ` is parsed as a **key bullet**; a line matching `N. ` starts a **new step**; a line starting with `## ` is **consumed and may terminate the section**. Fenced content is unsafe in a step body. | mod.rs:509–653 |
| `# h1` / `### h3` inside a step | Not terminators (§1); appended as body text via Path B. | mod.rs:513, 645 |
| `## h2` inside a step | Never body — consumed by `continue` at mod.rs:525, and terminates the section unless it is `## steps`. | mod.rs:513–525 |
| A line trimming to exactly `##` | **Is** body text — the trim defeats `starts_with("## ")`. | mod.rs:510, 513 |
| Body from the title line | `extract_bold_title`'s `rest` seeds `current.body` before any continuation appends (`current.body = body`, not `push_str`). | mod.rs:543–545 |
| **Final trim** | Whole body trimmed once on flush: `body: self.body.trim().to_string()`. Title is **not** trimmed. | mod.rs:696 vs 695 |
| Bullets before the first `N.` | Silently discarded (both paths require `current.number.is_some()`). | mod.rs:553, 645 |

**Net body model:** the concatenation, with `\n`, of every non-blank, individually-trimmed, non-`## `, non-numbered-item line in the step's span (with unrecognized `- ` bullets retaining their marker), then trimmed as a whole.

---

### 7. Case sensitivity and whitespace tolerance of keys

| Aspect | Rule | Citation |
|---|---|---|
| **Key case** | **Case-SENSITIVE.** Every key is matched against a lowercase literal. `- Tools: shell`, `- WHEN: x`, `- Kind: checkpoint` are **not** recognized and become body text. | mod.rs:555–633 |
| **Space before the colon** | **Not tolerated.** The colon is part of the matched literal. `- tools : shell` → body text. | mod.rs:555 |
| **Space after the colon** | Fully tolerated — every handler trims (`val.trim()` at the call site, or internally in `parse_csv_list`/`parse_u32_list`/`parse_switch_rules`/`parse_step_failure`/`parse_step_kind`/`parse_execution_mode`). `- tools:shell` (no space) also works. | mod.rs:554, 718–795 |
| **Space(s) after the bullet hyphen** | **Tolerated** — `-  tools: x` and `- - tools: x` both reach `tools: x` via `trim_start_matches("- ").trim()`. (Draft was wrong to require exactly one space.) | mod.rs:553–554 |
| **Leading indentation** | Fully tolerated (`line.trim()`), and therefore **carries no meaning** — a bullet at column 0 and one at column 20 are identical. | mod.rs:510 |
| **Value case** | Case-**insensitive** for: `requires_confirmation` / `terminal` (`eq_ignore_ascii_case("true")`, mod.rs:569, 591), `kind` (`to_ascii_lowercase`, mod.rs:754), `mode` (`to_lowercase`, mod.rs:171), `on_failure`'s `fail` literal (mod.rs:777). Case-**sensitive** for: `on_failure`'s `retry`/`goto` prefixes (mod.rs:781–789), and all free-text values (`capability`, `agent`, `policy`, `edit`, `when`, `prompt`, tool names). | as cited |
| **Heading case** | Case-insensitive (ASCII) — the one place `## Steps` differs from the bullets. | mod.rs:514 |
| **Unicode case** | All folding is **ASCII-only** (`eq_ignore_ascii_case`, `to_ascii_lowercase`); `mode:` alone uses full `to_lowercase` (mod.rs:171). Irrelevant for the valid value sets, but reproduce it for byte-exactness (e.g. Turkish `İ` behaves differently under the two). | mod.rs:171, 514, 569, 591, 754 |

---

### 8. Error handling: there is none in the parser

`parse_steps` cannot fail. It returns `Vec<SopStep>` (mod.rs:504), performs no I/O, and has no `Result`, no `anyhow`, no logging call, and no line-number tracking anywhere in mod.rs:504–837.

Every malformed construct resolves to one of exactly **four** silent outcomes:

1. **Treated as body text** — the dominant path. Any bullet whose key is unrecognized, misspelled, mis-cased, or spaced (`- tools : x`, `- Tools: x`, `- switchh: x`), plus every non-`- ` line. mod.rs:634–640.
2. **Silently dropped, value discarded**:
   - `call:` with invalid JSON — the **only** key that discards the *whole bullet* and leaves no trace (`if let Ok(call)` with no `else`, mod.rs:610–612).
   - Individual non-numeric entries in `depends_on:` (`filter_map`, mod.rs:729).
   - Individual empty entries in any `parse_csv_list` key (mod.rs:722).
   - Switch segments with an empty name (`return None`, mod.rs:739–741).
3. **Silently defaulted**:
   - `on_failure:` anything unparseable → `Fail` (mod.rs:794).
   - `kind:` anything unrecognized → `Execute` (mod.rs:757).
   - `mode:` anything unrecognized → `Some(Supervised)` — an *override to supervised*, not "no override" (mod.rs:177, 605).
   - `next:` / switch `goto` non-numeric → `None` (mod.rs:589, 747).
   - `requires_confirmation:` / `terminal:` anything ≠ `true` → `false` (mod.rs:569, 591).
4. **Silently stringified** — `input:` / `output:` / `with:` can never fail; the raw text becomes `Value::String` (mod.rs:772). Stage-2 TOML may additionally swallow a `#` tail or strip literal-string quotes (§5.3).

**Structural malformations are equally silent:** no `## Steps` heading → empty `Vec`; a step with no title → empty-string title; a bullet before the first numbered item → discarded; a `1.` line inside a code fence → a spurious step; `kind:`/`capability:`/`policy:` on the title line → swallowed into the body (§4.5).

**CORRECTION — where diagnostics actually live.** The draft claimed the capability check is "the only thing that can make `load_sop` return `Err`". `load_sop` (mod.rs:421–481) can also fail on `std::fs::read_to_string(&toml_path)?` (mod.rs:424), `toml::from_str::<SopManifest>(&toml_content)?` (mod.rs:425), and `std::fs::read_to_string(&md_path)?` (mod.rs:429), before ever reaching `capability::SopCapabilityRegistry::with_builtins().validate_sop(&sop)?` (mod.rs:479). Any of those makes `load_sops_from_directory` log a `WARN` and skip that SOP directory entirely (mod.rs:403–414). Post-parse diagnostics come from two places:
- `validate_sop` → warnings only (mod.rs:977–1008): empty name, empty description, no triggers, no steps, the dead numbering-gap check (§2.1), and empty step titles.
- `validate_sop_strict` → blocking + warnings (mod.rs:1088–1123): empty SOP name, empty/whitespace step titles, duplicate step numbers, planned-call binding errors (`validate_planned_call_bindings`, mod.rs:1016+), and every `SopGraph` diagnostic (mod.rs:1108–1118, sourced from graph.rs:314–416). It gates `save_sop` (mod.rs:958–961) but **never runs on the load path**.

> **This remains the largest single opportunity for Egeria.** Upstream's parser is a lossy, diagnostic-free scanner. Egeria owns `Finding` (ADR-0008) and should surface each of the ~18 silent degradations above as a located `EGR-*` finding. Each such finding is a **deliberate divergence** — it must add a diagnostic without changing the parsed result, or round-tripping against ZeroClaw breaks.

---

### 9. Round-trip: `render_steps` is NOT the inverse it claims to be

`render_steps` (mod.rs:932–948) emits `## Steps\n\n`, then per step `N. **title** - body` (or `N. **title**` when body is empty, mod.rs:935–942), then bullets indented three spaces (`   - {bullet}`, mod.rs:944). No blank line separates steps.

The doc comment asserts losslessness:
> ```
> /// Render steps back to `SOP.md` markdown, the inverse of `parse_steps`.
> /// Every contract field (tools, scope, schema, routing, failure policy,
> /// mode) becomes a sub-bullet, so render -> parse is lossless.
> ```
> — mod.rs:929–931

**It is not.** `render_step_bullets` (mod.rs:847–927) emits `tools` (851), `allow-tools` (855), `deny-tools` (858), `requires_confirmation` (862), `kind` — **checkpoint only** (864–866), `input` (869), `output` (872), `when` (876), `next` (879), `terminal` (882), `depends_on` (892), `switch` (906), `on_failure` (909–912), `mode` (915), `agent` (918), and `call` (920–924). It **never emits**:

- `capability:` and `with:` (types.rs:371, 373–374)
- `policy:` (types.rs:379)
- `prompt:` / `gate_prompt` (types.rs:386)
- `edit:` (types.rs:391)
- `kind: capability` — the guard is `if step.kind == SopStepKind::Checkpoint` (mod.rs:864), so a `Capability` step renders with **no `kind` bullet at all**.

So a `kind: capability` / `capability: llm.generate` / `with: {...}` step — the exact shape syntax.md:271–276 promotes — round-trips through `render_steps` into a plain `Execute` step with no capability and no arguments, and `save_sop` (mod.rs:954–972) writes that lossy form to `SOP.md` (mod.rs:969). (The full step *does* survive in `SOP.toml`'s `[[steps]]` via `SopManifest::from_sop`, types.rs:663 — but SOP.md wins on the next load, mod.rs:428–431, so the loss is real.) Body text is also permanently reflowed (blank lines gone, indentation gone, §6).

**ADDITION — a second, even lossier SOP.md writer exists.** `default_procedure_markdown` (procedural_memory.rs:317–336) emits `# {name}\n\n## Steps\n\n`, then `N. **title** - body`, and only two bullets: `tools:` (326–328) and `requires_confirmation: true` (330–332). Everything else — scope, schema, routing, switch, failure policy, mode, agent, calls, capability, policy, prompt, edit — is destroyed. It also always emits the ` - ` separator, so an empty body yields a trailing `- ` that re-parses to `body = ""` (harmless). Egeria importing SOPs written by the procedural-memory path must expect this reduced surface.

**Round-trip hazards Egeria's printer must guard against** (upstream does not):
- A body containing a line that matches `N. ` re-parses as a **new step** on the next load (§2). `render_steps` interpolates `step.body` raw into the title line (mod.rs:938–941), so a multi-line body's second line lands unescaped at column 0.
- `render_steps` writes `step.number` (mod.rs:936, 939), not a normalized index. Since `parse_steps` renumbers positionally, an unnormalized `Sop` silently changes numbers across a save/load cycle (demonstrated by mod.rs:1480–1483, where `titled_step(2, "wait")` returns as `parsed[0]`). `save_sop` calls `normalize_step_numbers` first (mod.rs:956) so the *save* path is consistent; a direct `render_steps` call is not.
- `pos` is never rendered into SOP.md by design; it lives in SOP.toml `[[positions]]` (types.rs:588–592, 597–603) and is merged back **by step number** at load (mod.rs:437–441). Because `parse_steps` renumbers positionally and always sets `pos: None` (mod.rs:708), a `[[positions]]` block written against author-chosen numbers silently attaches to the wrong nodes — or to none. Egeria must key positions by the normalized index.

**Egeria's printer must emit all 21 keys** to be genuinely lossless, and should treat upstream's rendering as a known-lossy reference rather than a specification. Canonical spellings upstream emits are **mixed**: hyphen for `allow-tools`/`deny-tools` (mod.rs:855, 858), underscore for `requires_confirmation`/`depends_on`/`on_failure` (mod.rs:862, 892, 910); `on_failure` values use the space-after-colon form (`retry: 2`, `goto: 5`, mod.rs:842–843). Match those for diff-stability against ZeroClaw-written files.

---

### 10. OPEN QUESTIONS — genuinely undecidable from source

1. **OPEN QUESTION — `toml::Value` → `serde_json::Value` for datetimes** in `parse_value_fragment` stage 2 (mod.rs:766–771). No test exercises it. `zeroclaw-runtime` depends on `toml = "1.0"` (crates/zeroclaw-runtime/Cargo.toml:74; workspace pin Cargo.toml:112), resolving to `toml 1.1.2+spec-1.1.0` in Cargo.lock:10403–10405. A TOML datetime serializes through serde as a private marker table rather than a plain scalar, so `- input: 2024-01-01` produces something structurally unlike any JSON an author would expect, and the exact shape is **version-coupled**. Recommend Egeria restrict stage 2 to inline tables, arrays, strings, integers, floats and booleans, reject/pass through datetimes as `Value::String`, and record the restriction as a divergence. Escalate before implementing byte-exact parity.
2. **OPEN QUESTION — two `## Steps` sections with a step open across the boundary** (§1). The non-flush at mod.rs:514–518 is almost certainly unintentional; no test covers it. Egeria should decide explicitly (recommend: match upstream, add a warning).
3. **OPEN QUESTION — `switch` guards containing `>`** (§5.1). Upstream truncates the guard and nulls the `goto` with no parse-time signal. Whether that is intended or a latent bug is not determinable from source or docs; treat the observed behavior as normative and add a finding.
4. **OPEN QUESTION — TOML comment swallowing in `parse_value_fragment`** (§5.3, new). `- input: 1 # note` → `Number(1)` while `- input: abc # note` → `String("abc # note")`. This asymmetry is an artifact of the `format!("value = {value}")` wrap, not a decision recorded anywhere. Reproduce it, but escalate whether Egeria should warn.
5. **`- capability:` with an empty value** yields `Some("")` (mod.rs:576) rather than `None`, unlike `agent:`/`policy:`/`edit:`. Whether the capability registry (mod.rs:479) rejects the empty name is outside this area; the lexical result is `Some("")`.
6. **`when:`/`prompt:` empty-value no-op vs `agent:`/`policy:`/`edit:` empty-value clear** (§4, rows 10/17/19/20/21). The asymmetry is real in source (mod.rs:585 and 615 guard; 608, 620, 629 assign) but has no stated rationale and no test. Reproduce it; do not "fix" it silently.
7. **OPEN QUESTION — `suggested_tools` vs `scope.allow` in Egeria's IR** (§4.1). Upstream keeps two fields and derives a third view (`effective_tool_scope`, types.rs:435–444). Whether Egeria's `Workflow` IR should collapse them at import (losing the ability to round-trip `tools:` back out as `tools:` rather than `allow-tools:`) is a modeling decision, not a parsing one. Escalate.

---

### 11. Errata against the prior draft (for traceability)

| # | Draft claim | Correction | Citation |
|---|---|---|---|
| 1 | Bullet marker is "exactly `- `" | `-  ` (extra spaces) and `- - ` are also accepted via `trim_start_matches("- ").trim()` | mod.rs:553–554 |
| 2 | `##  Steps` / `## Steps:` merely "❌" | They **terminate** the section and flush the open step | mod.rs:513, 519–523 |
| 3 | `deny-tools:` empty → `[]` | Also materializes `scope` to `Some(..)` via `ensure_scope` | mod.rs:566, 804–806 |
| 4 | "Upstream does not detect [dangling refs]" | `validate_sop_strict` + `SopGraph` classify them **blocking**; a `goto`-less switch port is a warning | mod.rs:1088–1123; graph.rs:314–416 |
| 5 | Capability check is "the only thing that can make `load_sop` return `Err`" | File-read and TOML-parse errors precede it | mod.rs:424, 425, 429 |
| 6 | `normalize_manifest_steps` "renumbers only `number == 0`" | It also backfills empty titles from `capability` / `kind` | mod.rs:488–493 |
| 7 | `suggested_tools` described as a plain hint list | It is a **legacy alias for `scope.allow`**, derived by `effective_tool_scope` | types.rs:326–328, 435–444; test mod.rs:1400–1408 |
| 8 | `call:` has "zero occurrences" in docs | `syntax.md:193` matches inside `discord.oncall:…`; zero **as a bullet key** | syntax.md:193 |
| 9 | types.rs anchors for `policy`/`gate_prompt`/`edit`/`capability` | Off by 2–6 lines; corrected anchors in §4 | types.rs:367–392 |
| 10 | `render_steps` is the only SOP.md writer | `default_procedure_markdown` is a second, lossier writer | procedural_memory.rs:317–336 |
| 11 | `pos` unmentioned | Always `None` from `parse_steps`; merged from SOP.toml `[[positions]]` **by step number**, which positional renumbering silently invalidates | mod.rs:708, 437–441; types.rs:588–603 |
| 12 | `parse_value_fragment` stage 2 described only as "TOML inline tables" | Also swallows `#` comments and strips literal-string quotes | mod.rs:765–771 |
| 13 | Bare `1.` unaddressed | Never opens a step (trim removes the space, so no `". "`) | mod.rs:510, 810 |
| 14 | A line trimming to `##` unaddressed | Not a heading; becomes body text | mod.rs:510, 513 |
| 15 | Exhaustiveness asserted by reading | Now **proven**: 32 `strip_prefix` + 4 `starts_with` fully accounted for | mod.rs (whole file) |

---

### 12. Files read (nothing under `external/` was modified)

- `.../crates/zeroclaw-runtime/src/sop/mod.rs` — `parse_execution_mode` 168–179; `normalize_step_numbers` 330–374; `load_sops_from_directory` 376–419; `load_sop` 421–481; `normalize_manifest_steps` 483–496; **parser 500–657**; `StepParseState` 659–716; helpers 718–837; printer 839–948; `save_sop` 950–972; `validate_sop` 976–1008; `validate_planned_call_bindings` 1010–1026+; `validate_sop_strict` 1088–1123; tests 1372–1387, 1389–1411, 1413–1471, 1473–1484, 1620–1635, 1637–1656, 1658–1676
- `.../crates/zeroclaw-runtime/src/sop/step_contract.rs` — `SwitchRule` 3–18, `StepRouting` 20–41 (+ `is_default` 43–48), `StepFailure` 51–62 (+ `is_fail` 65–68)
- `.../crates/zeroclaw-runtime/src/sop/types.rs` — `SopExecutionMode` 36–54 + `Display` 56–66; `SopStepKind` 239–251 + `Display` 253–261; `StepSchema` 265–277; `PlannedToolCall` 281–297; `StepPos` 299–306; `SopStep` 308–392; `effective_tool_scope` 435–444; `effective_agent` 446–450; `SopManifest` 582–595; `StepPosition` 597–603; `SopMeta` 605–633; `SopManifest::from_sop` 635–665
- `.../crates/zeroclaw-runtime/src/sop/scope/mod.rs` — `StepToolScope` 8–18; `resolve_excluded` 22–60+
- `.../crates/zeroclaw-runtime/src/sop/graph.rs` — routing diagnostics 305–417; binding diagnostics 554–632
- `.../crates/zeroclaw-runtime/src/sop/route/mod.rs` — `resolve_next` precedence doc tree 31–49; false-guard branch 68–76
- `.../crates/zeroclaw-runtime/src/sop/procedural_memory.rs` — `default_procedure_markdown` 317–336
- `.../crates/zeroclaw-runtime/src/tools/mod.rs:2505` — separator-less title example
- `.../crates/zeroclaw-sop-graph/src/lib.rs` — 479 lines, no markdown parsing
- `.../Cargo.lock:10403–10405`, `.../crates/zeroclaw-runtime/Cargo.toml:74`, `.../Cargo.toml:112` — `toml` version pin
- `.../docs/book/src/sop/syntax.md` (441 lines) — §3 step format 106–179; approver identities 83, 93; checkpoint edit/revise 231–250; capability section 252–287; contract enforcement 289–300
- `.../docs/book/src/sop/example.md:59` — the advertised (dead) numbering-gap warning
- `.../docs/book/src/sop/fan-in/channel.md:5` — the sole repo-wide "switch" doc hit (unrelated prose)

---

# Part: Routing and execution semantics

## ZeroClaw SOP — Routing and Execution Semantics (corrected & completed)

**Authority.** Every claim is derived from the Rust source under
`/Users/hollow/Documents/Workflow Generator/egeria/external/zeroclaw/crates/zeroclaw-runtime/src/sop/`.
Where `docs/book/src/sop/syntax.md` disagrees, **the source wins** and the disagreement is called out.
Abbreviations: `sop/…` = `.../crates/zeroclaw-runtime/src/sop/…`; `syntax.md` = `.../docs/book/src/sop/syntax.md`;
`config/schema.rs` = `.../crates/zeroclaw-config/src/schema.rs`.

Changes from the prior draft are marked **[FIX]** (wrong), **[ADD]** (missing), **[CITE]** (citation was wrong).

---

### 1. The precedence decision tree

#### 1.1 The authoritative doc-comment

`sop/route/mod.rs:31-49` — normative and matching the code beneath it:

```rust
/// Pick the next step, preserving linear behavior when no routing is declared.
///
/// Resolution precedence (highest to lowest). The branches are mutually
/// exclusive — only one of them fires per call — so the linear narrative
/// reads as a *decision tree*, not a fallback chain:
///
/// 1. **False top-level `when` guard** → terminal completes; otherwise the
///    linear successor is taken. `switch` ports and `routing.next` are
///    bypassed entirely.
/// 2. **True or absent top-level `when` guard** with a non-empty `switch` →
///    the first matching port's `goto` is taken, or the run completes if no
///    port matches. `routing.next` and the linear successor are NOT
///    consulted.
/// 3. **True or absent top-level `when` guard** with no `switch` and a
///    declared `routing.next` → that explicit successor is taken (visit
///    limit, dependency, and existence checks still apply).
/// 4. **True or absent top-level `when` guard** with no `switch` and no
///    `routing.next` → `terminal: true` completes the run; otherwise the
///    linear successor (`current_step + 1`) is taken.
```

#### 1.2 The code

`sop/route/mod.rs:50-103` (`resolve_next`), `:107-116` (`resolve_target`), `:119-125` (`resolve_linear`),
`:129-138` (`resolve_step_decision`), `:141-146` (`eligible`). **[CITE]** the draft ran `resolve_target`/
`resolve_linear`/`resolve_step_decision`/`eligible` together as "`:107-146`"; the exact spans are as listed.
The quoted bodies in the draft are byte-accurate — verified line by line.

#### 1.3 The unambiguous ordered algorithm (implement this)

> **Precondition.** `resolve_next` is called on the step that **has already executed**. `routing.when` is an
> **out-edge guard on the completed step**, never an entry guard on a candidate. A step with a false
> `when:` **still ran**. syntax.md:165-166 agrees on the timing.
>
> **[FIX] Read-site count.** The draft claimed `routing.when` is "read in exactly two places —
> route/mod.rs:65-68 and :79-81". That is wrong: `:79-81` reads `SwitchRule::when`, a different field.
> A workspace grep for `routing.when` across `crates/**/*.rs` returns exactly **one** evaluation site,
> `sop/route/mod.rs:65`, plus one write site (the parser, `sop/mod.rs:586`), one render site
> (`sop/mod.rs:875`), and test fixtures. `SwitchRule::when` is evaluated at exactly one site,
> `sop/route/mod.rs:79-82`.

```
RESOLVE_NEXT(S, R, D, last_status, max_step_visits) -> NextStep:

  0. if last_status == Failed:                                     # route/mod.rs:51-53
         return Fail("step failed")
     # DEAD on the engine path: resolve_next's sole caller (engine.rs:2129) is reached
     # only after engine.rs:2093 has already diverted every Failed status into
     # route::failure::route_failure. Verified: workspace grep for `resolve_next(`
     # yields engine.rs:2129 as the only non-test call site.

  1. cur := the FIRST s in S.steps with s.number == R.current_step  # route/mod.rs:55-59
     if none:                                                      # route/mod.rs:60-62
         return Complete
     # `.find()` — duplicate numbers are silently tolerated, later duplicate unreachable.

  2. payload := to_json_string({ "steps": { "<n>": <output of step n>
                                            for every COMPLETED step n } })  # rundata.rs:38-45
     # Only Completed results (rundata.rs:15-23). Failed and Skipped excluded.
     # ALWAYS non-empty valid JSON — at minimum `{"steps":{}}`.

  3. when_allows_jump := (cur.routing.when is None)                 # route/mod.rs:65-68
                         OR evaluate_condition(cur.routing.when, payload)

  4. if NOT when_allows_jump:                                       # route/mod.rs:70-75
         if cur.routing.terminal: return Complete                   # :71-73
         return RESOLVE_LINEAR()                                    # :74

  5. if cur.routing.switch is non-empty:                            # route/mod.rs:77-92
         for rule in cur.routing.switch, IN DECLARED ORDER:
             matched := (rule.when is None)          # None = catch-all, always matches (:81)
                        OR evaluate_condition(rule.when, payload)   # :80
             if not matched: continue                               # :83-85
             if rule.goto is None:                                  # :86-88
                 return Fail("switch port '<rule.name>' has no target")
             return RESOLVE_TARGET(rule.goto, Explicit)             # :89
         return Complete                                            # :91
         # `next`, `terminal`, and the linear successor are NEVER consulted here.

  6. if cur.routing.next is Some(t):                                # route/mod.rs:94-96
         return RESOLVE_TARGET(t, Explicit)
         # `terminal` NOT consulted; explicit `next` beats `terminal: true`.

  7. if cur.routing.terminal: return Complete                       # route/mod.rs:98-100
     return RESOLVE_LINEAR()                                        # route/mod.rs:102


RESOLVE_TARGET(t, kind):                                            # route/mod.rs:107-116
  tgt := FIRST s in S.steps with s.number == t                      # :108
  if none:
      Explicit                     -> Fail("step <t> does not exist")   # :110
      Linear and t > R.total_steps -> Complete                          # :111
      Linear                       -> Fail("step <t> does not exist")   # :112
  return RESOLVE_STEP_DECISION(t, tgt)                              # :115
  # [ADD] Existence is checked BEFORE total_steps. A Linear target that EXISTS but is
  # > R.total_steps (a stale total_steps) is routed to, not completed.

RESOLVE_LINEAR():                                                   # route/mod.rs:119-125
  return RESOLVE_TARGET(R.current_step.saturating_add(1), Linear)

RESOLVE_STEP_DECISION(t, tgt):                                      # route/mod.rs:129-138
  visits := count of r in R.step_results where r.step_number == t   # route/guard.rs:3-10
            (ALL statuses: Completed, Failed, AND Skipped)
  if NOT (visits < max_step_visits):                                # guard.rs:12-13, mod.rs:130-131
      return Fail("step <t> visit limit reached")
  if every d in tgt.routing.depends_on is a key of D.outputs:       # mod.rs:133, :141-146
      return Step(t)
  else:
      return Wait(t)                                                # mod.rs:136
```

`NextStep` — five variants, `sop/route/mod.rs:14-21`:

```rust
pub enum NextStep { Step(u32), Retry, Complete, Fail(String), Wait(u32) }
```

**Engine interpretation** (`sop/engine.rs:2138-2213`, `apply_route_decision`):

| Variant | Engine behavior | Citation |
|---|---|---|
| `Step(n)` | Re-check visit bound; emit `step_promoted`; dispatch `n` (deterministic vs LLM) | engine.rs:2149-2168 |
| `Retry` | Re-check visit bound **for `current_step_number`**; emit `step_retry`; re-dispatch `current_step_number` with `retry_input` | engine.rs:2169-2191 |
| `Complete` | deterministic → `finish_deterministic_run`; else `finish_run(run_id, Completed, None)` | engine.rs:2192-2204 |
| `Fail(reason)` | `finish_run(run_id, Failed, Some(reason))` | engine.rs:2205 |
| `Wait(n)` | `mark_step_pending(run_id, sop, n, "step {n} dependencies not satisfied")` | engine.rs:2206-2211 |

`mark_step_pending` (engine.rs:2446-2514, via `mark_step_pending_with_persist` :2456): sets
`current_step = n`, `status = Pending`, `waiting_since = now` (:2466-2468), pushes a **`Skipped`**
`SopStepResult` whose `output` is the reason (:2472-2482), records a `step_skipped` transition
(:2496-2504), returns `SopRunAction::Pending` (:2508-2513).

**Implementer trap (confirmed).** `step_visit_count` counts results of *every* status (route/guard.rs:3-10),
so each `Wait` park burns one visit of the waited-on step. The de-dup at engine.rs:2469-2471
(`last_is_same_skip`) suppresses only a **consecutive** identical skip.

#### 1.4 Entry point, numbering, and where steps actually live

- Runs start at step number **1** unconditionally: `current_step: 1` at **engine.rs:1731**
  (**[CITE]** draft said 1730), `total_steps: u32::try_from(sop.steps.len())` at **engine.rs:1732**
  (draft said 1731), both inside `activate_reserved_run` (engine.rs:1713).
- `parse_steps` numbers **positionally**, ignoring the Markdown ordinal (`sop/mod.rs:537-540`):

  ```rust
  let step_num = u32::try_from(steps.len()).unwrap_or(u32::MAX).saturating_add(1);
  current.reset_for_step(step_num);
  ```

  A SOP.md-authored SOP therefore always has contiguous `1..=N`, `total_steps == N`, and the linear
  successor `N+1 > total_steps` reliably yields `Complete`.
- `normalize_step_numbers` (mod.rs:336-374) is **not** called on load; `load_sop` (mod.rs:422-481) parses
  and validates only. Its sole caller is `save_sop` (mod.rs:956). It no-ops on duplicate numbers
  (mod.rs:337-340) and remaps `routing.next` (:354), `depends_on` (:355-360, dropping refs to removed
  steps), switch `goto` (:361-363), `on_failure: goto` → **`Fail`** when the target is gone (:364-369),
  and `{{steps.N}}` call bindings (:370-372). **[ADD]** the `Goto → Fail` downgrade and the silent
  `depends_on` drop.

**[FIX] — the `[[steps]]` question. The prior audit's fact "There is no `[[steps]]` table in SOP.toml"
is false, and the draft's softer version ("only a hand-written fallback path") is also wrong.**

- `SopManifest` declares `steps: Vec<SopStep>` with `#[serde(default)]` and **no
  `skip_serializing_if`** — types.rs:593-594.
- `SopManifest::from_sop` sets `steps: sop.steps.clone()` — **types.rs:663**.
- `save_sop` writes `toml::to_string_pretty(&SopManifest::from_sop(sop))` to `SOP.toml` **and**
  `render_steps` to `SOP.md` — mod.rs:966-969.

⇒ **Upstream's own writer emits a full `[[steps]]` array into every saved `SOP.toml`.** On load,
`SOP.md` wins outright and `manifest.steps` is discarded (mod.rs:428-435); `manifest.steps` is consulted
only when `SOP.md` is absent. So the TOML copy is dead weight that can silently drift from the Markdown.

On the fallback path, `normalize_manifest_steps` (mod.rs:483-496) fills only numbers that are literally
`0` and titles that are empty; a hand-written `[[steps]]` block **can** produce non-contiguous numbers,
and then a linear successor that is `<= total_steps` but does not exist yields
`Fail("step N does not exist")` (route/mod.rs:112).

**Egeria rule:** treat `SOP.md` as the sole step source, mirroring upstream's load precedence; parse
`[[steps]]` only when `SOP.md` is absent; and emit a **diagnostic when both exist and disagree**, because
upstream will silently prefer the Markdown.

#### 1.5 **[ADD] The SOP.md renderer is LOSSY — a save→load cycle destroys five step fields**

`render_step_bullets` (mod.rs:847-927) emits exactly these bullets, in this order:
`tools:` (:851), `allow-tools:` (:855), `deny-tools:` (:858), `requires_confirmation: true` (:862),
`kind: checkpoint` (:865, **only** when `kind == Checkpoint`), `input:` (:869), `output:` (:872),
`when:` (:876), `next:` (:879), `terminal: true` (:882), `depends_on:` (:892), `switch:` (:906),
`on_failure:` (:909-912), `mode:` (:915), `agent:` (:918), `call:` (:922).

It emits **nothing** for `capability` (types.rs:371), `capability_input` / `with` (types.rs:374),
`policy` (types.rs:379), `gate_prompt` / `prompt:` (types.rs:386), or `edit` (types.rs:391) — and it
never emits `kind: capability`.

Because `save_sop` writes `SOP.md` (mod.rs:969) and `load_sop` prefers `SOP.md` (mod.rs:428-430):

> **A `save_sop` → `load_sop` round trip silently destroys `capability:`, `with:`, `policy:`,
> `prompt:`, and `edit:`, and demotes every `kind: capability` step to `kind: execute`.**

This directly contradicts the renderer's own doc-comment, mod.rs:929-931: *"Every contract field …
becomes a sub-bullet, so render -> parse is lossless."* The upstream round-trip test
`render_parse_roundtrip_preserves_full_step_contract` (mod.rs:1230-1272) sets none of the five lost
fields, so the loss is untested.

**Consequence for approval domination:** a checkpoint's `- policy:` (its group/quorum enforcement) and
`- edit:` opt-in vanish on the first editor save. Egeria must not assume a loaded SOP's checkpoint
carries the policy its author wrote.

---

### 2. The specific questions, resolved

#### 2.1 False `when:` guard — complete, or advance linearly?

**syntax.md contradicts itself. The source says: advance to the linear successor, unless `terminal: true`.**

- syntax.md:165-167 (**WRONG**): *"`- when:` is a routing guard evaluated against accumulated
  completed-step outputs after the current step finishes. When it does not match, the run completes
  instead of dispatching another step."*
- syntax.md:170-171 (**RIGHT**): *"`- when:` guards an explicit `- next:` jump; when the condition is
  false, the run advances to the next linear step (`current_step + 1`) instead of completing."*

Source, `sop/route/mod.rs:70-75`:

```rust
if !when_allows_jump {
    if current.routing.terminal {
        return NextStep::Complete;
    }
    return resolve_linear(ctx);
}
```

| Condition | Result | Citation |
|---|---|---|
| `when` false, `terminal: false`, step `current+1` exists | `Step(current+1)` (subject to visit bound + that step's `depends_on`) | route/mod.rs:74, :115, :129-137 |
| `when` false, `terminal: false`, `current+1` missing **and** `> total_steps` | `Complete` | route/mod.rs:111 |
| `when` false, `terminal: false`, `current+1` missing **and** `<= total_steps` | `Fail("step N does not exist")` | route/mod.rs:112 |
| `when` false, `terminal: true` | `Complete` | route/mod.rs:71-73 |

syntax.md:165-167 is only accidentally right in the last-step case. Pinned by
`when_false_advances_to_linear_successor` (route/mod.rs:285-295) and `when_false_at_end_completes`
(route/mod.rs:297-307).

**A false guard also bypasses `switch` entirely** — `when_false_bypasses_switch_to_linear_successor`,
route/mod.rs:310-338, in-test comment at :325-326: *"Step 1 has a false `when` condition and a switch with
two ports / It should bypass the switch evaluation entirely and go to linear successor (step 2 = 1+1)"*.

#### 2.2 False guard on a step that also has `terminal: true`

**`Complete`** — route/mod.rs:71-73. It completes *even when a viable successor exists*, pinned by
`false_guard_terminal_with_available_successor_completes` (route/mod.rs:464-482): a false `when`,
`terminal = true`, **and** a catch-all switch arm pointing at step 2 → `NextStep::Complete`.

#### 2.3 A step with BOTH `switch:` and `next:` — which wins?

**`switch` wins; `next` is dead.** route/mod.rs:77-92 returns unconditionally from the switch block;
control never reaches `:94`. Pinned by `true_guard_unmatched_switch_with_explicit_next_completes`
(route/mod.rs:373-396) and `absent_guard_unmatched_switch_with_explicit_next_completes`
(route/mod.rs:443-462).

The authoring/validation layer agrees with a **Warning**, **`sop/graph.rs:376-385`**
(**[CITE]** draft said 371-382):

```rust
if !step.routing.switch.is_empty() && step.routing.next.is_some() {
    diagnostics.push(GraphDiagnostic {
        severity: GraphSeverity::Warning,
        step: step.number,
        message:
            "step has switch rules and a routing.next target; next is ignored because \
             switch resolution takes precedence"
                .to_string(),
    });
}
```

It also suppresses the `next` sequence wire via `switch_supersedes` (declared graph.rs:301, applied
graph.rs:303 and :321 — **[CITE]** draft said 302-304). Warning ≠ blocking (`has_errors`, graph.rs:474-478;
`save_sop` rejects only on blocking, mod.rs:958-961).

#### 2.4 A switch where NO arm matches

**`Complete`** — route/mod.rs:91. Successful termination; no fallthrough to `next`, `terminal`, or linear.

**[FIX]** the draft said "Three tests pin this exact shape" while listing four. There are **four**:
`true_guard_unmatched_switch_with_explicit_next_completes` (route/mod.rs:373-396),
`true_guard_unmatched_switch_with_linear_successor_completes` (:398-420),
`absent_guard_unmatched_switch_with_linear_successor_completes` (:422-441),
`absent_guard_unmatched_switch_with_explicit_next_completes` (:443-462).

Distinct failure mode: an arm that **does** match but has `goto: None` →
`Fail("switch port '<name>' has no target")` (route/mod.rs:86-88). At authoring time that same shape is
only a **Warning** (graph.rs:409-415), while a `goto` naming a nonexistent step is an **Error**
(graph.rs:399-408).

**[ADD] Three blocking graph Errors the draft omitted**, all of which reject a `save_sop`:
`next target step {n} does not exist` (graph.rs:314-318), `depends_on target step {d} does not exist`
(graph.rs:349-353), `on_failure goto target step {t} does not exist` (graph.rs:368-372).

**Security note for Egeria (unchanged, still correct):** an unmatched switch is a silent successful
termination. An approval step after a switch can be skipped entirely by input matching no arm, and the
run reports `Completed`. Any "approval dominates the effect" analysis must model
`switch-with-no-catch-all` as an edge to `Complete`.

#### 2.5 Switch arm evaluation order

**Strictly ordered, top-to-bottom, first match wins, short-circuit.** route/mod.rs:78-90: `continue` on
non-match, `return` on first match. Arms after the first match are never evaluated.

Type doc, `sop/step_contract.rs:3-6`:

```rust
/// A single named output port on a switch step. Rules are evaluated top to
/// bottom; the first whose `when` guard passes routes the run to `goto`. A
/// rule with `when` unset is the catch-all (n8n's "unknown"/default port) and
/// should be ordered last.
```

`when: None` is an unconditional catch-all (route/mod.rs:79-82: `None => true`; field doc
step_contract.rs:12). A catch-all placed anywhere but last makes every later arm unreachable —
"should be ordered last" is advice, not enforcement (no diagnostic exists for it). Pinned by
`when_true_evaluates_switch_correctly` (route/mod.rs:341-371): the first matching port (step 3) beats
both the catch-all (step 4) and the linear successor (step 2).

#### 2.6 `terminal: true` — whole run, or just that branch?

**Whole run: `NextStep::Complete` → `finish_run(…, Completed, None)`** (engine.rs:2192-2204). There is no
branch-local termination and no concurrency (§3). `routing.terminal` is **read** in exactly three places:
route/mod.rs:71, route/mod.rs:98, graph.rs:322. **[ADD]** it is also **written** by the graph-wiring API
`sop/wire.rs:116` and `:122` (disconnecting/connecting a sequence wire sets it).

Field doc (`sop/step_contract.rs:30-34`) uses branch language — authoring-surface framing, not runtime:

```rust
/// When true, this step ends its branch: no implicit fallthrough to the
/// following step is derived. Lets an authoring surface delete the default
/// sequence edge and leave a node free-floating between saves.
##[serde(default, skip_serializing_if = "std::ops::Not::not")]
pub terminal: bool,
```

**`terminal` is subordinate to both `switch` and `next`.** For a true/absent guard the order is
`switch` (:77) → `next` (:94) → `terminal` (:98) → linear (:102). `terminal: true` + `next: 5` routes to 5;
`terminal: true` + a matching switch arm routes to that arm. `terminal` fires only when it is the *only*
routing declaration, or on the false-guard branch (:71).

**`terminal:` appears ZERO times in syntax.md** (`grep -c terminal` → 0), exactly like `switch:`
(`grep -c switch` → 0).

**[ADD] Graph-view divergence.** The implicit fallthrough wire uses `sop.steps.get(idx + 1)` — the next
step by **vector position** (graph.rs:323) — while the runtime uses `current_step + 1` by **number**
(route/mod.rs:122). These agree only for contiguous numbering. Do not import the graph view as runtime
control flow.

---

### 3. `depends_on` — ordering, gating, or concurrency?

#### 3.1 Verdict

**`depends_on` is a runtime GATE, never a fork/join and never a concurrency directive. The runtime never
executes two steps of a run concurrently. Egeria's `parallel: false` capability declaration is correct.**

#### 3.2 Evidence

**(a) No concurrency primitives.** `grep -rEn "tokio::spawn|join_all|futures::|JoinSet|rayon|par_iter|spawn_blocking"`
over every `.rs` file under `crates/zeroclaw-runtime/src/sop/` returns **zero matches**.
(**[FIX]** the draft's "all 44 files, ~34k lines" is a miscount — the tree holds ~40 distinct `.rs` files
including subdirectories; the zero-match result is what matters and is confirmed.)

**(b) Single-cursor synchronous state machine.** `SopEngine.active_runs: HashMap<String, SopRun>`
(engine.rs:32); `SopRun.current_step: u32` — a single scalar (types.rs, `SopRun` struct). Every routing
entry point takes `&mut self` and returns one `SopRunAction`: `advance_step` (engine.rs:1770),
`route_recorded_step` (:2058), `route_decision_after_recorded_step` (:2081), `apply_route_decision` (:2138),
`dispatch_llm_step` (:2235), `resolve_deterministic_action` (:4446). `resolve_next` returns **one**
`NextStep` (route/mod.rs:50). No frontier structure exists anywhere.

**(c) Embedders hold the engine behind a `Mutex`** — `Arc<Mutex<SopEngine>>` at
`crates/zeroclaw-channels/src/filesystem.rs:28` and `:36`, `.../amqp.rs:39` and `:59`,
`.../orchestrator/mqtt.rs:21`.

**(d) `max_concurrent` bounds concurrent RUNS, not steps.** types.rs:485-486: *"Maximum simultaneous runs
of this one procedure."* Default 1 (types.rs:517-519). `SopAdmissionPolicy::Parallel` (types.rs:533-537)
likewise governs *trigger admission*: *"Admit up to `max_concurrent` concurrent runs."*
**[ADD]** the full policy set is `Parallel` (default), `Hold`, `Coalesce`, `Drop` (types.rs:532-548), with
outcomes `Admit` / `Defer` / `Coalesce` / `Drop` (types.rs:567-578). None concerns steps.

**(e) The dependency check is a pure predicate over already-recorded Completed outputs.**
route/mod.rs:141-146:

```rust
pub fn eligible(step: &SopStep, run_data: &RunData) -> bool {
    step.routing.depends_on.iter()
        .all(|dependency| run_data.outputs.contains_key(dependency))
}
```

`run_data.outputs` holds **only `Completed`** results — `RunData::from_step_results`, rundata.rs:15-23:

```rust
for result in results {
    if result.status == SopStepStatus::Completed {
        data.insert_output_str(result.step_number, &result.output);
    }
}
```

A `Failed` or `Skipped` dependency does **not** satisfy the gate. With exactly one execution cursor, a
`depends_on` can only be satisfied by a step that already ran earlier on the single path.

**(f) `eligible` is enforced at four sites, all yielding the same park:** route/mod.rs:133 (→ `Wait`),
engine.rs:2260-2267 (`dispatch_llm_step`), engine.rs:4460-4467 (`resolve_deterministic_action`),
engine.rs:3550-3557 (`clear_waiting_gate` — an approved gate whose step lost its dependency parks rather
than executes; **[CITE]** draft said 3546-3552).

#### 3.3 What actually happens on an unsatisfied dependency

`Wait(n)` → `mark_step_pending` (engine.rs:2446-2514, §1.3). Reason string:
`"step {n} dependencies not satisfied"` (engine.rs:2210, and identically at :2265, :4465, :3555).

**No automatic re-drive.** The only maintenance sweep over `Pending` runs is
`retry_capacity_blocked_gated_pends` (engine.rs:827-890), filtered by
`pending_step_blocks_direct_advance(sop, step)` (engine.rs:846) — i.e. **only checkpoint or
approval-gated steps** (engine.rs:5483-5485). A dependency pend resumes only when an agent calls
`sop_advance`, which `advance_step` permits for a non-gated pending step (engine.rs:1842-1864 — the
`bail!` fires only when `pending_step_blocks_direct_advance` is true).

**Net semantics for Egeria's IR:** `depends_on: [a, b]` on step `n` lowers to a *guard/assertion* on the
inbound edge to `n` — "a and b have Completed earlier in this run" — plus a park-on-violation edge to a
`Pending` state. It must **not** lower to a join node and creates **no** control-flow edge from `a`/`b`
to `n`. (`graph.rs:338-355` *does* draw a `FlowRole::Dependency` wire for **visualization** — **[CITE]**
draft said 337-354. It is a rendering artifact; do not import it as control flow.)

---

### 4. `on_failure` — `fail` / `retry:N` / `goto:M`

#### 4.1 Declaration

Type, `sop/step_contract.rs:50-63`:

```rust
##[serde(rename_all = "snake_case")]
pub enum StepFailure {
    #[default] Fail,
    Retry { max: u32 },
    Goto { step: u32 },
}
```

Helper `is_fail()` at step_contract.rs:65-68 (used to skip serialization, types.rs:348).

Bullet keys `- on_failure:` **and** `- on-failure:` — mod.rs:599-603. Value parser, mod.rs:775-795
(verbatim quote in the draft is accurate). Semantics:

- `fail` — matched **case-insensitively** (`eq_ignore_ascii_case`, mod.rs:777).
- `retry:<u32>` or `retry <u32>` — **[ADD] case-SENSITIVE** `strip_prefix` (mod.rs:781-782); `Retry` is
  never produced from `Retry:2` or `RETRY 2`.
- `goto:<u32>` or `goto <u32>` — same case sensitivity (mod.rs:788-789).
- **Anything unrecognized silently degrades to `Fail`** (mod.rs:794) — no parse error, no diagnostic.
  `retry:-1`, `retry:abc`, `goto:` all become `Fail`.

Renderer `render_step_failure` (mod.rs:839-845) emits `fail` / `retry: {max}` / `goto: {step}`; the
space after the colon is absorbed by the parser's `.trim()`, so these round-trip. `on_failure` is omitted
from `SOP.md` entirely when it is `Fail` (mod.rs:908).

**Egeria should surface a diagnostic on an unparseable `on_failure` value rather than silently
matching upstream's fail-closed default.**

#### 4.2 The failure router

**`on_failure` is evaluated BEFORE and INSTEAD OF `resolve_next`.** A failed step's own
`when` / `switch` / `next` / `terminal` are **never consulted** —
`sop/engine.rs:2081-2136` (`route_decision_after_recorded_step`), verbatim at :2093-2135:

```rust
if last_status == SopStepStatus::Failed {
    let failed_executions = run.step_results.iter()
        .filter(|result| result.step_number == current_step.number
                      && result.status == SopStepStatus::Failed)
        .count().try_into().unwrap_or(u32::MAX);
    let retries_consumed = failed_executions.saturating_sub(1);
    let decision = route::failure::route_failure(
        &current_step.on_failure, retries_consumed, self.config.max_step_retries);
    return Ok(match decision {
        NextStep::Fail(reason) if reason == "step failed" => { /* enrich */ }
        other => other,
    });
}
let run_data = RunData::from_step_results(&run.step_results);
Ok(route::resolve_next(&RouteCtx { sop, run, run_data: &run_data, last_status,
                                   max_step_visits: self.config.max_step_visits }))
```

`sop/route/failure.rs:4-17`, complete:

```rust
pub fn route_failure(policy: &StepFailure, retries_consumed: u32, max_retries: u32) -> NextStep {
    match policy {
        StepFailure::Fail => NextStep::Fail("step failed".into()),
        StepFailure::Retry { max } => {
            let retry_limit = (*max).min(max_retries);
            if retries_consumed < retry_limit { NextStep::Retry }
            else { NextStep::Fail("step retry limit reached".into()) }
        }
        StepFailure::Goto { step } => NextStep::Step(*step),
    }
}
```

| Policy | Behavior |
|---|---|
| `fail` (default) | Run terminates `Failed`. Reason enriched to `"Step <n> failed: <last failed output>"` (engine.rs:2111-2123). |
| `retry: N` | Effective cap `min(N, sop.max_step_retries)` (failure.rs:8). `retries_consumed < cap` → `Retry` (re-dispatch the same step number with `retry_input`); at the cap → `Fail("step retry limit reached")`. |
| `goto: M` | `Step(M)` **directly, with no visit-bound and no existence check inside the router**. Both are applied downstream: `apply_route_decision` → `visit_bound_failure` (engine.rs:2150-2152, 2215-2233), and `dispatch_llm_step` (:2242) / `dispatch_deterministic_step` call `resolve_sop_step` (engine.rs:2424-2444), which returns an **anyhow `Err`** for a missing step — not a graceful `NextStep::Fail`. `goto: M` also skips `M`'s `depends_on` at the router; `dispatch_llm_step` re-checks `eligible` at :2260 and parks `Pending`. |

`retries_consumed` accounting: the just-failed attempt has **already been appended** to `step_results`
(engine.rs:1934, `self.record_step_result(run_id, recorded.clone())?`; the fn itself is at
engine.rs:2043-2056 — **[CITE]** the draft's "engine.rs:1933, `record_step_result`" conflated call site
and definition and was off by one). So `failed_executions - 1` = prior failures = retries spent.

Worked example, `retry: 5` with default `max_step_retries = 2` → `retry_limit = 2`:
1st failure → `retries_consumed = 0` → `Retry`; 2nd → `1 < 2` → `Retry`; 3rd → `2 < 2` false →
`Fail("step retry limit reached")`. ⇒ **at most 3 total executions.** Pinned by
`retry_respects_global_limit` (route/failure.rs:23-33). `retry: 0` → `Fail` on the first failure.

#### 4.3 What counts as a failure

1. An agent-reported `SopStepResult { status: Failed }` passed to `advance_step` (engine.rs:1770).
2. **Output-schema validation failure on a Completed step**, rewritten to `Failed` in place before
   routing — LLM path engine.rs:1895-1921, deterministic path engine.rs:4204-4231
   (**[CITE]** draft said 1897-1913 / 4203-4227), checkpoint-approve path engine.rs:3216-3234.
   syntax.md:312-314 agrees.
3. **A capability step whose capability returns `success: false` or errors** —
   `execute_capability_step`, engine.rs:4100-4169 (both arms at :4141-4168 record `Failed` and route).
4. **A denied deterministic checkpoint** (§6.4).

**Asymmetry: input-schema failure does NOT route through `on_failure`.** `schema_input_failure_action`
(engine.rs:1946-1955) → `fail_step_schema_validation` (engine.rs:1989-2019) → `finish_run(…, Failed, …)`
directly. syntax.md:312-313 is accurate: only the *output* half routes.

**[ADD] Both schema checks are gated on config.** `validate_step_input` (engine.rs:1961-1974) and
`validate_step_output` (engine.rs:1975-1988) both begin `if !self.config.step_schema_enforce { return Ok(()); }`.
`sop.step_schema_enforce` defaults to **`true`** (config/schema.rs:22835-22837; syntax.md:300). With it
**off**, malformed output never becomes a failure and never reaches `on_failure` — it flows on as a
JSON string.

#### 4.4 The two settings, their real names, and their defaults

Both live in the `[sop]` table (`#[prefix = "sop"]`, config/schema.rs:22556; `SopConfig` at :22557).

| Config key | Type | Default | Defined at | Bounds |
|---|---|---:|---|---|
| `sop.max_step_visits` | `u32` | **256** | field config/schema.rs:22650-22651; default fn :22839-22841; wired into `SopConfig::default()` :22889 | *"Maximum times a routed SOP run can visit one step."* |
| `sop.max_step_retries` | `u32` | **2** | field config/schema.rs:22654-22655; default fn :22843-22845; wired :22890 | *"Maximum retries allowed by a step failure policy."* |

Both confirmed by name and default; syntax.md:303-304 documents both correctly.

**Enforcement of `max_step_visits` — three sites, one predicate** (`sop/route/guard.rs:3-13`):

```rust
pub fn step_visit_count(run: &SopRun, step_number: u32) -> u32 {
    run.step_results.iter()
        .filter(|result| result.step_number == step_number)
        .count().try_into().unwrap_or(u32::MAX)
}
pub fn within_visit_bound(run: &SopRun, step_number: u32, max_visits: u32) -> bool {
    step_visit_count(run, step_number) < max_visits
}
```

1. `route::resolve_step_decision`, route/mod.rs:130-131 → `Fail("step <t> visit limit reached")`.
2. `SopEngine::visit_bound_failure`, engine.rs:2215-2233 → `finish_run(…, Failed, "step <t> visit limit reached")`;
   called from `apply_route_decision` on `Step(n)` (:2150) and `Retry` (:2170).
3. `dispatch_llm_step` engine.rs:2243-2245 and `dispatch_deterministic_step` (same guard before dispatch).

**Critical accounting:** the count is over **all** `step_results` for that number regardless of status.
Retries (`Failed`), dependency parks (`Skipped`), and completions all consume the same per-step budget.
It is *per-step*, not per-run. `route_failure`'s `Goto` is the only routing decision that does not itself
consult the bound — `apply_route_decision` does, before dispatch.

---

### 5. The condition grammar

Single implementation: `sop/condition.rs`. Shared by step `when:` guards, switch-arm `when` guards, and
trigger `condition` fields (different payload). syntax.md:350-352 confirms the shared grammar.

#### 5.1 Grammar (EBNF — from condition.rs:3-21, :46-64, :83-119)

```
condition   ::= ε                                   (* empty/whitespace ⇒ TRUE, unconditional *)
              | json_path_form
              | direct_form

json_path_form ::= "$" path_segments WS? op WS? comparand
path_segments  ::= { "." segment }
segment        ::= identifier | integer             (* integer = array index *)
direct_form    ::= op WS? comparand                 (* no leading "$" *)

op          ::= ">=" | "<=" | "!=" | "==" | ">" | "<"
comparand   ::= <rest of string, trimmed, non-empty>
```

**No `AND`, `OR`, `NOT`, parentheses, negation prefix, boolean literal, arithmetic, function call,
wildcard, bracket indexing, recursive descent, or variables.** syntax.md:423-424 is **verified correct**.
`parse_path_op_value` (condition.rs:83-104) finds exactly one operator and splits in two; everything else
lands inside the comparand as literal text.

#### 5.2 Operators — the closed set and the **two** scan orders

`ConditionOp` is the single source of truth (condition.rs:149-157; doc at :144-148):

| Variant | Token (`token()`, :161-170) | Label (`label()`, :173-182) |
|---|---|---|
| `Eq` | `==` | is |
| `Neq` | `!=` | is not |
| `Gt` | `>` | is greater than |
| `Lt` | `<` | is less than |
| `Gte` | `>=` | is at least |
| `Lte` | `<=` | is at most |

**Evaluator scan order** — `parse_order()`, condition.rs:71-80:
`Gte, Lte, Neq, Eq, Gt, Lt` → `>=`, `<=`, `!=`, `==`, `>`, `<`.

In `parse_path_op_value` the match is `input.find(op.token())` — **substring search, not anchored**
(condition.rs:86). The *first operator token found scanning in that order over the whole string* wins,
regardless of position, so a comparand containing an operator character can hijack the split. The doc
guarantee at condition.rs:68-70 only extends to *"longest-token-first so parsing never mistakes a two-char
token (`>=`) for its one-char prefix (`>`)"*.

**[ADD] `parse_op_value` (direct form, condition.rs:107-119) is ANCHORED** — it uses
`input.strip_prefix(op.token())` (:110), not `find`. So a direct condition must *begin* with an operator.

**[ADD] `ConditionParts` uses a DIFFERENT order — an authoring/evaluator divergence.**
`ConditionParts::parse` (condition.rs:243-272) builds its token list from `ConditionOp::catalog_tokens()`
(:209-212, `strum` `EnumIter` order: `>`,`<`,`>=`,`<=`,`==`,`!=`) and sorts by `Reverse(len)` with a
**stable** sort (:254-255), yielding `>=`, `<=`, `==`, `!=`, `>`, `<`. That swaps `==` and `!=` relative to
`parse_order()`. For a condition string containing both tokens — e.g. `$.a != "b==c"` — the authoring
round-trip (`ConditionParts`) and the runtime evaluator split at **different** operators. The doc-comment
at condition.rs:68-70 claiming *"This order is the single scan order every parser and every authoring
surface reads"* is therefore not literally true.

`ConditionParts::build` (condition.rs:278-289) is the canonical emitter: `$.<path> <op> <value>`, or
`<op> <value>` when there is no path; an empty `op` yields `None` (fire on every event). Egeria should
emit exactly this shape.

#### 5.3 What the left-hand side can reference

**Only `$.steps.<N>.<path…>`, and only outputs of steps that have already `Completed` in this run.**

Payload built once per routing decision (`route/mod.rs:64`) by `RunData::to_payload` (rundata.rs:38-45):

```rust
pub fn to_payload(&self) -> Value {
    let steps = self.outputs.iter()
        .map(|(step, value)| (step.to_string(), value.clone())).collect();
    json!({ "steps": Value::Object(steps) })
}
```

Exactly `{"steps": {"1": <out1>, "3": <out3>, …}}` — nothing else. Same shape as syntax.md:352-362.

- **Binding syntax is `$.steps.N.field`, NOT `{{steps.N}}`.** The mustache `{{steps.N.path}}` /
  `{{calls.K.path}}` form is a *different* facility: planned tool-call args (`sop/binding.rs`,
  types.rs:281-286, :353-354) and the checkpoint `- prompt:` template (types.rs:380-384). Mustache
  bindings are **never** evaluated inside a `when:`, and `$.`-paths are never resolved inside a call arg.
  **[ADD]** the step **body** is also documented to carry `{{steps.N}}` bindings (types.rs:321-323) —
  a third consumer of the mustache namespace, again disjoint from `when:`.
- **The trigger payload is NOT reachable from a `when:` guard.** `to_payload` has no trigger key. The
  trigger payload reaches a step only as piped input (`step_input_value`, engine.rs:5555-5569) and as
  framed prompt context (`format_step_context`, engine.rs:5513-5553). To filter on trigger content, use a
  **trigger** `condition`, evaluated against the raw event payload.
- Dot paths walk objects by key and arrays by numeric index (`resolve_json_path`, condition.rs:122-140);
  **object-key lookup is tried before array index** (:126-130), so `$.steps.1.x` resolves `"1"` as an
  object key.
- A step output that is not valid JSON is stored as a JSON **string** (`insert_output_str`,
  rundata.rs:29-32), so `$.steps.1.anything` is unresolvable for a prose-output step. Schema-guided
  recovery of a single embedded object/array exists but **only** when the step declares an `output:`
  schema and the candidate validates (`parse_step_output_value`, rundata.rs:67-85;
  `unique_embedded_container`, :91-…).
- **[ADD]** `RunData::get_path` (rundata.rs:47-57) is a *third*, unrelated path resolver: it strips `$.`
  and converts dots to a serde_json **pointer**. It has no array/object-key fallback and is not used by
  `evaluate_condition`. Do not conflate it with `resolve_json_path`.

#### 5.4 Evaluation and coercion

`evaluate_condition`, condition.rs:3-21 (verbatim in the draft; accurate).

Comparison, `compare_values` (condition.rs:293-317): **numeric first** — if the extracted JSON value
coerces to `f64` (`Number`, or a `String` that parses — `value_as_f64`, :319-325) **and** the comparand
parses as `f64`, compare numerically via `apply_op_f64` (:336-345). Otherwise **string** comparison, after
stripping one layer of surrounding double quotes from the comparand (:304-307). JSON `true`/`false`/`null`
stringify to `"true"`, `"false"`, `""` (`value_as_string`, :327-334) — hence syntax.md:420-421's
`$.active == "true"`.

`Eq`/`Neq` on the numeric branch use an **absolute** epsilon: `(lhs - rhs).abs() < f64::EPSILON` /
`>= f64::EPSILON` (condition.rs:342-343) — exact for small integers, unreliable at large magnitude.
`Gt`/`Lt`/`Gte`/`Lte` on the **string** branch are byte-lexicographic (`lhs.as_str() > rhs`, :312-315):
an ordering comparison against a non-numeric field silently becomes a string comparison, never an error.

#### 5.5 Fail-closed set

`evaluate_condition` returns **false**, never an error, in all of these:

| Case | Citation |
|---|---|
| payload is `None` or `""` | condition.rs:9-12 |
| condition contains no recognized operator | condition.rs:103 (`parse_path_op_value` → `None`) → :31-34 |
| comparand empty after the operator | condition.rs:90-92 (path form), :112-114 (direct form) |
| path part empty / all segments empty | condition.rs:94-98 |
| payload not parseable JSON | condition.rs:25-28 |
| JSON path does not resolve (missing key, OOB index) | condition.rs:37-40, :137 |
| direct form: payload or comparand does not parse as `f64` | condition.rs:53-61 |

syntax.md:364-366 states this correctly.

**[FIX] The draft's conclusion about which causes are live in routing is wrong.** It wrote: *"In routing,
the payload is always non-empty valid JSON … so the live fail-closed causes are unresolved path and
unparseable condition."* That omits the most consequential one:

> **Every direct-form condition (no leading `$`) used as a routing guard is ALWAYS false.**
> `evaluate_direct_condition` parses the **entire payload string** as `f64` (condition.rs:53-56), and the
> routing payload is always `{"steps":{…}}` (rundata.rs:38-45), which never parses as a number.

So the live fail-closed causes in routing are: **direct form (always)**, unresolved `$.` path, empty
comparand, and no recognized operator. Egeria should emit a **diagnostic** for any step or switch-arm
guard that does not begin with `$` — it is statically dead.

#### 5.6 ⚠️ CRITICAL FOR EGERIA — can the grammar express negation?

**Short answer: no, not soundly. Do not synthesize a syntactically negated condition string for the
bypass edge.**

**(a) There is no `NOT`.** §5.1 and syntax.md:423-424.

**(b) The operator set is *pairwise complementary as an operator table*:**
`!(a > b) ≡ a <= b`, `!(a >= b) ≡ a < b`, `!(a < b) ≡ a >= b`, `!(a <= b) ≡ a > b`,
`!(a == b) ≡ a != b`, `!(a != b) ≡ a == b`. On the string branch this holds (condition.rs:309-316 is a
total order on `&str`).

**[FIX] (b′) But it does NOT hold exactly on the numeric branch.** The draft claimed *"on the numeric
branch this holds exactly"*. `value_as_f64` accepts a JSON **string** parsed by Rust's `f64::from_str`
(condition.rs:322), which accepts `"NaN"`, `"inf"`, `"-inf"`, `"infinity"`. With `lhs = NaN`,
`apply_op_f64` (condition.rs:336-345) gives `Gt/Lt/Gte/Lte` = false, `Eq` = `(NaN-rhs).abs() < ε` = false,
**and** `Neq` = `(NaN-rhs).abs() >= ε` = **false**. Both polarities are false. Complementarity fails
inside the numeric branch itself, not only at the `evaluate_condition` level.

**(c) The complement identity is FALSE at the level of `evaluate_condition`, because of §5.5.**
When the guard fails closed, the syntactically negated guard *also* fails closed. Concrete counterexample,
fully determined by source:

> `RunData` is empty (step 1 produced prose, or no JSON). Routing payload = `{"steps":{}}` (rundata.rs:38-45).
> - `evaluate_condition("$.steps.1.approved == true", "{\"steps\":{}}")` → `resolve_json_path` fails on
>   segment `"1"` (condition.rs:137) → :37-40 → **false**.
> - `evaluate_condition("$.steps.1.approved != true", …)` → same path failure → **false**.
>
> Both false. `¬(G) ≠ G'`.

**(d) What the implementer MUST do.** Do not encode the bypass edge by negating the guard *string*.
Encode it as the **complement of the guard's truth value**, which the router computes as a boolean at
route/mod.rs:65-68. For a step `k` with guard `G`:

- Introduce an opaque IR predicate `GuardHolds(step=k, expr=G)` carrying the **unparsed** condition text
  plus the evaluator's semantics (§5.4, §5.5) as its interpretation.
- Emit **exactly two** mutually exclusive, exhaustive out-edges from `k`:
  - `GuardHolds(k, G)` → the switch/next/terminal resolution of §1.3 steps 5-7;
  - `¬GuardHolds(k, G)` → the false-guard resolution of §1.3 step 4 (terminal ⇒ Complete, else linear
    successor).
- `¬GuardHolds` is a **semantic** complement over the evaluator's boolean; it must never be lowered back
  to a ZeroClaw condition string. Round-tripping such an edge to SOP.md is **lossy/impossible** — a
  backend that must print a negated guard should report a fidelity downgrade rather than emitting `!=`.
- Sound conservative refinement for static analysis: `G_neg_syntactic ⇒ ¬GuardHolds(G)` but **not**
  conversely. The fail-closed region (§5.5) plus the NaN region (§5.6b′) are extra regions where
  `¬GuardHolds` holds and no syntactic negation does. Treat that region as always-reachable unless the
  step declares an `output:` schema making the referenced path total — and even then, a schema-invalid
  output is rewritten to `Failed` and diverted to `on_failure` (§4.3) rather than reaching the router,
  **and only while `sop.step_schema_enforce` is `true`** (§4.3 note).

**(e) Practical consequence for approval domination.** A guard referencing a step output whose schema does
not guarantee the path is a **bypass edge that fires whenever the model's output is malformed**. Since the
same malformation makes *both* polarities false, an author cannot write a "require approval unless X"
guard that is safe under bad output: the false-guard branch (§2.1) advances to the linear successor
unconditionally. Egeria should flag any guard whose LHS path is not provably present in the referenced
step's declared `output:` schema, and any guard that is direct-form (§5.5, statically dead).

#### 5.7 ⚠️ Undocumented switch-arm grammar collision (`>` and `;`)

`switch:` bullets are parsed by `sop/mod.rs:733-751`:

```rust
fn parse_switch_rules(value: &str) -> Vec<SwitchRule> {
    value.split(';').filter_map(|seg| {
        let mut parts = seg.splitn(3, '>');
        let name = parts.next().unwrap_or("").trim().to_string();
        if name.is_empty() { return None; }
        let when = parts.next().unwrap_or("").trim();
        let goto = parts.next().unwrap_or("").trim();
        Some(SwitchRule { name,
            when: (!when.is_empty()).then(|| when.to_string()),
            goto: goto.parse::<u32>().ok() })
    }).collect()
}
```

Wire form: `switch: <name>><when>><goto>; <name2>><when2>><goto2>` — `;` separates ports, `>` separates
the three fields, `splitn(3, '>')` so only the **first two** `>` are separators. The renderer emits the
same shape (mod.rs:899-905: `format!("{}>{}>{}", rule.name, when, goto)`, joined with `"; "`).

**Therefore a switch arm's guard cannot contain `>` or `;`.** Trace `- switch: hot>$.steps.1.temp > 50>3`:

- `splitn(3,'>')` → `["hot", "$.steps.1.temp ", " 50>3"]`
- `name = "hot"`, `when = Some("$.steps.1.temp")` (operator and comparand lost),
  `goto = " 50>3".parse::<u32>()` → `Err` → `None`
- At runtime that arm evaluates `"$.steps.1.temp"` — no operator ⇒ `parse_path_op_value` returns `None`
  (condition.rs:103) ⇒ **false** (:31-34). Had it matched, `goto: None` ⇒
  `Fail("switch port 'hot' has no target")` (route/mod.rs:86-88).

Usable in a switch-arm guard: `==`, `!=`, `<`, `<=`. **Unusable: `>`, `>=`** (and any `;`).

**[ADD] Two further parse hazards in the same function:**
1. A segment whose **name** is empty is **dropped entirely** (mod.rs:739-741) — silently shifting every
   later arm's index.
2. `sop/wire.rs:162-163` pads `routing.switch` with `SwitchRule::default()` (name `""`, `when` `None`,
   `goto` `None`) up to the connected port index. Those padded rules render as `>>` and are then
   **destroyed** by rule (1) on the next parse, shifting the surviving ports. Round-tripping a
   wire-authored sparse switch through SOP.md is lossy.

Upstream has **no** test with an operator inside a switch guard: the round-trip fixture (mod.rs:1249-1260)
uses `when: Some("$.event")` — no operator — and the parse fixture (mod.rs:1426) uses
`switch: pull_request>$.event>3; catch_all>>2`. Genuine, unfixed grammar collision.

**Egeria must:** (i) parse switch arms with the identical `split(';')` + `splitn(3,'>')` algorithm to stay
bug-compatible; (ii) emit a diagnostic (never silent acceptance) when an arm's `when` fails to parse as a
condition, when its `goto` is absent, or when its name is empty; (iii) on print, refuse to emit an arm
whose guard contains `>` or `;` or whose name is empty — not representable.

**OPEN QUESTION:** whether upstream considers this a bug or intended. Nothing in source, tests, or docs
addresses it; `switch:` is undocumented entirely. Do not invent an escape mechanism.

---

### 5.8 **[ADD] The complete SOP.md bullet grammar (routing-relevant, exhaustive)**

`parse_steps` (mod.rs:504-657). A step opens on a numbered item (`parse_numbered_item`, mod.rs:809-817:
requires `". "` with an all-ASCII-digit prefix); the title is the first `**bold**` run
(`extract_bold_title`, mod.rs:820-837). Bullets are recognized only inside `## Steps` (mod.rs:513-530)
and only for lines beginning `- ` (mod.rs:553-554), matched in this **exact order** by `strip_prefix`:

| Bullet key(s) | Field | Silent-failure behavior | Citation |
|---|---|---|---|
| `tools:` | `suggested_tools` | CSV, empties dropped | mod.rs:555-556, 718-724 |
| `allow-tools:` / `allow_tools:` | `scope.allow` | CSV | mod.rs:557-561 |
| `deny-tools:` / `deny_tools:` | `scope.deny` | CSV | mod.rs:562-566 |
| `requires_confirmation:` | `requires_confirmation` | anything ≠ `true` (ci) ⇒ **false** | mod.rs:567-570 |
| `kind:` | `kind` | unrecognized ⇒ **`Execute`** | mod.rs:571-574, 753-759 |
| `capability:` | `capability` | — | mod.rs:575-576 |
| `with:` | `capability_input` | JSON → TOML `value =` wrap → bare string | mod.rs:577-578, 761-773 |
| `input:` / `output:` | `schema.input` / `.output` | same fragment parser | mod.rs:579-582 |
| `when:` | `routing.when` | **empty value leaves it `None`** | mod.rs:583-587 |
| `next:` | `routing.next` | unparseable ⇒ **`None`** (overwrites a prior value) | mod.rs:588-589 |
| `terminal:` | `routing.terminal` | anything ≠ `true` (ci) ⇒ **false** | mod.rs:590-591 |
| `depends_on:` / `depends-on:` | `routing.depends_on` | CSV of u32; unparseable entries **dropped** | mod.rs:592-596, 726-731 |
| `switch:` | `routing.switch` | §5.7 | mod.rs:597-598, 733-751 |
| `on_failure:` / `on-failure:` | `on_failure` | unrecognized ⇒ **`Fail`** | mod.rs:599-603, 775-795 |
| `mode:` | `mode` | `parse_execution_mode` | mod.rs:604-605 |
| `agent:` | `agent` | empty ⇒ `None` | mod.rs:606-608 |
| `call:` | `calls` (append) | invalid JSON ⇒ **silently dropped** | mod.rs:609-612 |
| `prompt:` | `gate_prompt` | empty ⇒ `None` | mod.rs:613-617 |
| `policy:` | `policy` | empty ⇒ `None` | mod.rs:618-624 |
| `edit:` | `edit` | empty ⇒ `None` | mod.rs:625-633 |
| *anything else* | **appended to the step body** | no diagnostic | mod.rs:634-640 |

**[ADD]** An unrecognized `- foo: bar` bullet is silently swallowed into the prompt body. Egeria should
diagnose unknown bullet keys rather than mirroring this.

`StepParseState::flush_into` (mod.rs:689-715) is what constructs each `SopStep`; `pos` is always `None`
from Markdown (:708) and is merged in from `SOP.toml` `[[positions]]` at load (mod.rs:437-441).

---

### 6. Step kinds

`SopStepKind` (`sop/types.rs:240-251`), `Display` at `:253-261`:

```rust
##[serde(rename_all = "snake_case")]
pub enum SopStepKind {
    #[default] Execute,   // "Normal step — executed by the agent (or deterministic handler)."
    Checkpoint,           // "Checkpoint step — pauses execution and waits for human approval."
    Capability,           // "Deterministic capability step - executed by the SOP capability registry."
}
```

Bullet parsing (`sop/mod.rs:753-759`), case-insensitive, with an **undocumented alias**:

```rust
fn parse_step_kind(value: &str) -> SopStepKind {
    match value.trim().to_ascii_lowercase().as_str() {
        "checkpoint" | "approval" => SopStepKind::Checkpoint,
        "capability" => SopStepKind::Capability,
        _ => SopStepKind::Execute,
    }
}
```

**`kind: approval` is an accepted synonym for `checkpoint`** and appears nowhere in the docs
(syntax.md:160 lists only `execute` and `checkpoint`). Any unrecognized value **silently becomes
`execute`**. Egeria should diagnose rather than silently coerce.

#### 6.1 ⚠️ Kind is honored ONLY on the deterministic path — and `requires_confirmation` ONLY off it

`step.kind` is dispatched on in **exactly one** place: `resolve_deterministic_action`,
`sop/engine.rs:4473-4567`:

```rust
match step.kind {
    SopStepKind::Checkpoint => { /* park PausedCheckpoint, persist state, notify */ }   // :4474-4553
    SopStepKind::Capability => self.execute_capability_step(sop, run_id, step, input),  // :4554
    SopStepKind::Execute    => { /* SopRunAction::DeterministicStep */ }                // :4555-4566
}
```

The **non-deterministic** path is `dispatch_llm_step` (engine.rs:2235-2344) → `resolve_step_action`
(engine.rs:5488-5508), which reads `requires_confirmation`, `mode`, `execution_mode`, and `priority` —
and **never reads `step.kind`**.

⇒ In `auto` / `supervised` / `step_by_step` / `priority_based`, a `- kind: checkpoint` step is **not a
gate**: it is dispatched as an ordinary `ExecuteStep`, and a `- kind: capability` step runs as a prose LLM
step with its `capability:` id ignored at runtime. No validation rejects this: `load_sop`'s only
kind-aware check is `SopCapabilityRegistry::validate_sop` (mod.rs:479; registry.rs:40-70), which validates
`kind == Capability` ids and `with:` input but says nothing about execution mode.

**[FIX] — and the converse is also true, which the draft got backwards.** The draft asserted
*"`requires_confirmation: true` is the mode-independent gate"* and *"forces a `WaitApproval` gate in
**every** execution mode."* **That is wrong.** Verified by workspace grep: `resolve_step_action` has
exactly one non-test call site, `engine.rs:2293`, inside `dispatch_llm_step`;
`step_requires_approval_gate` (engine.rs:5473-5481) is called only from there (:5495) and from
`pending_step_blocks_direct_advance` (:5484). `resolve_deterministic_action` (engine.rs:4446-4568) never
consults either. Independently, `execution_mode_needs_approval` returns `false` for
`SopExecutionMode::Deterministic` (engine.rs:5455).

> **The corrected rule: there is NO step-level construct that gates in both execution families.**
>
> | Construct | Gates in `auto`/`supervised`/`step_by_step`/`priority_based` | Gates in `deterministic` |
> |---|---|---|
> | `- kind: checkpoint` | **No** — plain execute node (engine.rs:5488-5508 never reads `kind`) | **Yes** — `PausedCheckpoint` (engine.rs:4474-4552) |
> | `- requires_confirmation: true` | **Yes** — `WaitApproval` (engine.rs:5474-5476, 5495-5500) | **No** — never read on that path (engine.rs:4446-4568) |
>
> `sop.deterministic = true` forces `execution_mode = Deterministic` (mod.rs:456-461; types.rs:492-495).

syntax.md:160-162 hints at half of this — *"A checkpoint step pauses **deterministic** execution at that
step. Use `requires_confirmation: true` when a step must require approval in any execution mode"* — but
the second sentence is **false for deterministic mode** and reads as advice rather than a hard constraint.

**Egeria rule to encode:** an approval gate exists on step `k` iff
`(sop.deterministic || sop.execution_mode == Deterministic) && k.kind == Checkpoint`
**or** `(!deterministic) && step_requires_approval_gate(sop, k)` (§6.2). Any approval-domination proof
that assumed either construct is universal is void; raise a finding on
`requires_confirmation: true` inside a deterministic SOP and on `kind: checkpoint` outside one.

#### 6.2 `kind: execute` (default)

Deterministic: `SopRunAction::DeterministicStep { run_id, step, input }` with the piped input and no LLM
round-trip (engine.rs:4555-4566), after `persist_active` (:4559).

Non-deterministic dispatch order inside `dispatch_llm_step` (engine.rs:2235-2344), which the implementer
must mirror:

1. `resolve_sop_step` — missing step ⇒ **anyhow `Err`** (engine.rs:2242, 2424-2444).
2. `visit_bound_failure` (:2243-2245).
3. set `current_step = n`, `status = Running`, `waiting_since = None` (:2247-2251).
4. `route::eligible` — unmet ⇒ `mark_step_pending` (:2260-2267).
5. resolve input: `input_override` else `step_input_value` (:2269-2278).
6. `schema_input_failure_action` — invalid ⇒ `finish_run(Failed)` (:2279-2281; §4.3).
7. `format_step_context` (:2283-2289), then `resolve_step_action` (:2293) → `ExecuteStep` or `WaitApproval`.
8. If parked: `pending_pool_full_reason` → `mark_step_pending` when the approval pool is full
   (engine.rs:2309-2312, **[CITE]** draft said 2311-2315); else flip to `WaitingApproval`, bump `revision`
   (:2313-2319), persist-then-release-claim (:2320-2339), `notify_park_request`.

`step_requires_approval_gate` (engine.rs:5473-5481):

```rust
fn step_requires_approval_gate(sop: &Sop, step: &SopStep) -> bool {
    if step.requires_confirmation { return true; }
    let effective_mode = step.mode.unwrap_or(sop.execution_mode);
    execution_mode_needs_approval(sop.execution_mode, sop, step)
        || execution_mode_needs_approval(effective_mode, sop, step)
}
```

with `execution_mode_needs_approval` (engine.rs:5451-5471):

| Mode | Gates when |
|---|---|
| `auto`, `deterministic` | never (on this path) — :5455 |
| `supervised` | `step.number == 1` only — :5456-5459 |
| `step_by_step` | every step — :5460 |
| `priority_based` | `critical`/`high` ⇒ **every step**; `normal`/`low` ⇒ step 1 only — :5461-5469 |

Note the `||`: a `- mode:` override can only **add** a gate, never remove one — the SOP-level mode is
always also evaluated. The `[SEC-FLIP]` comment at engine.rs:5462-5463 documents that `priority_based` +
critical/high was previously inverted (auto-ran the riskiest SOPs) and is now fail-closed.

A gate-approved step then executes the **same** step: `clear_waiting_gate` returns
`SopRunAction::ExecuteStep` (engine.rs:3588-3595) after re-checking `eligible` (:3550-3557) and the input
schema (:3566-3568). So an approval gate happens **before** the step body runs.

#### 6.3 `kind: capability`

Two additional bullets: `- capability: <id>` → `SopStep::capability` (types.rs:369-371; parse mod.rs:575-576),
and `- with: <fragment>` → `SopStep::capability_input`, serde-renamed **`with`** (types.rs:372-374; parse
mod.rs:577-578 via `parse_value_fragment`, mod.rs:761-773: JSON → TOML `value = <frag>` wrap → bare string).

Registered ids (`sop/capability/builtins.rs:11-26`, ids confirmed at :32, :48, :102, :125, :172, :206,
:225, :244 and `forge_comment.rs:181`, `llm_generate.rs:57`):
`noop`, `wait`, `approval.wait`, `json.validate`, `shell.exec`, `git.status`, `git.diff`,
`notify.channel`, `forge.comment`, `llm.generate`.

Load-time validation (`capability/registry.rs:40-70`): a `kind: capability` step **must** name a
`capability:` id (:45-50), that id **must** be registered (:51-56), and if `requires_authored_input()` its
`with:` must satisfy the capability's `input_schema` (:57-67). A violation is an `Err` from `load_sop`
(mod.rs:479) — the SOP does not load. (**[CITE]** draft said registry.rs:39-69.)

Execution (`engine.rs:4100-4169`, deterministic path only): runs synchronously inside the engine call
(:4126), then records `Completed` (:4130-4139) or `Failed` (:4141-4168) and immediately routes via
`record_deterministic_step_result` → `route_recorded_step` (engine.rs:4233-4241). A chain of capability
steps executes head-to-tail in one synchronous call — strictly sequential, never concurrent.
`forge.comment` is special-cased to `execute_forge_comment_step` (:4109-4118).

`approval.wait`, `forge.comment`, `llm.generate` (and the other injected-adapter placeholders) are
**fail-closed** without a daemon adapter (builtins.rs:20-25 comment; forge/llm constructed with `None`).
`approval.wait` never gates — it returns
`CapabilityResult::failure("approval.wait is registered but must route through checkpoint/resolve_gate wiring")`
(**builtins.rs:114-118**, **[CITE]** draft said 115-119). **Do not model `capability: approval.wait` as an
approval gate.**

Input-merge asymmetry: `SopStep::capability_call_input` (types.rs:425-433) merges the piped value into the
authored object under `input` with `entry(…).or_insert(…)` — an author-supplied `input` key wins.
But `registry.rs:86-97` (the `requires_authored_input()` path) uses `insert` at :90-93, which
**overwrites** any authored `input` with the piped value, with the explicit comment at :88-89:
*"The authored object is the trusted configuration plane. The piped value is always data, even when the
author included an `input` key."* (**[CITE]** draft said registry.rs:82-92.)

#### 6.4 `kind: checkpoint` — the approval/HITL node

**What it is.** No body execution. On reaching it the deterministic engine persists a
`DeterministicRunState` file (engine.rs:4483), flips the run to `SopRunStatus::PausedCheckpoint`
(:4496), bumps/rebases `revision` and `revision_base` (:4498-4506), releases the exec slot, optionally
sends an out-of-band notice (:4525-4546), and returns
`SopRunAction::CheckpointWait { run_id, step, state_file }` (:4548-4552). The value flowing *through* it is
`step_input_value(run, step.number)` (engine.rs:3128; :5555-5569).

**[ADD] `step_input_value` trap** (engine.rs:5555-5569):

```rust
pub(crate) fn step_input_value(run: &SopRun, step_number: u32) -> Value {
    if step_number <= 1 { return run.trigger_event.payload.as_deref().map(jsonish_value).unwrap_or(Value::Null); }
    run.step_results.last().map(step_result_value).unwrap_or(Value::Null)
}
```

The predecessor is `step_results.last()` — the last record in **append (execution) order, of any status**,
including a `Skipped` park record whose `output` is the pend reason string, or a `Failed` record whose
output is an error message. It is *not* "the previous step's Completed output". The condition `<= 1`, not
`== 1`, so step 0 (only reachable via a hand-written `[[steps]]`) also takes the trigger payload.

**Its four fields.**

| Bullet | Field | Meaning | Citation |
|---|---|---|---|
| `- policy: <name>` | `policy: Option<String>` | Names a key in `[sop.approval].policies`; the broker enforces `required_group` membership and quorum before the gate clears. `None` ⇒ `approval_mode` alone governs. A step naming a policy **absent** from config fails closed — the gate stays waiting. | types.rs:375-379; mod.rs:618-624; syntax.md:175-179 |
| `- prompt: <template>` | `gate_prompt: Option<String>` | Gate-notice template; `{{path.to.field}}` resolves against **the step's piped input** — *"pure lookups, no logic"*. Absent ⇒ automatic summary. | types.rs:380-386; mod.rs:613-617; engine.rs:2361-2363, 2369 |
| `- edit: <field>` | `edit: Option<String>` | Editable-field opt-in: the field of the piped value an approver may amend. Offered **only** on a `PausedCheckpoint` — `engine.rs:2372-2378` filters `edit_field` on `is_checkpoint` (`run.status == PausedCheckpoint`, :2360). | types.rs:387-391; mod.rs:625-633; syntax.md:234-239 |
| `- requires_confirmation: true` | `requires_confirmation: bool` | Independent of `kind`; forces `WaitApproval` — but **only on the LLM path** (§6.1 [FIX]). | types.rs:331-334; engine.rs:5474-5476 |

(**[CITE]** the draft's "engine.rs:2388-2393 filters on `is_checkpoint`" and "engine.rs:2388-2397" are
wrong; the correct spans are `is_checkpoint` at 2360, `edit_field` at 2372-2378, `can_revise` at
2379-2381, and the explanatory comment at 2355-2359.)

**Resolution outcomes.** Authoritative doc-comment, `engine.rs:3067-3072`:

```rust
/// Resolve a checkpoint decision (`PausedCheckpoint`). `Approve` resumes the
/// success path (records the checkpoint `Completed`, pipes forward down
/// `routing.next`); `Deny` takes the failure path (records the checkpoint
/// `Failed` and routes through the step's `on_failure`, exactly like a step
/// that failed execution). This is the single entry point for both outcomes;
/// callers never branch on status. `approve_step` is the `Approve`-only alias.
```

- **Approve** → recorded `Completed` with the piped value as output (engine.rs:3138-3146), output schema
  still validated (engine.rs:3216-3234), then normal `resolve_next` (:3236-3241) — so the checkpoint's own
  `when` / `switch` / `next` / `terminal` all apply. (**[CITE]** draft said 3212-3231.)
- **Amend (Edit)** → the named `edit:` field of the piped **object** is replaced with the approver's text
  (engine.rs:3129-3137), then treated **exactly like Approve** (same match arm, :3139-3146). A non-object
  piped value makes Amend a hard error (:3131-3135).
- **Revise** → does not resolve the gate; re-runs the checkpoint's `llm.generate` predecessor with the
  guidance as `revision_feedback` and re-presents. Capped at `MAX_GATE_REVISIONS = 3` per gate
  (engine.rs:5573; budget `revision - revision_base`, :2380); syntax.md:240-250.
- **Deny** → recorded `Failed`, then **routed through `on_failure`** (broker path engine.rs:3147-3162;
  agent path `deny_checkpoint`, engine.rs:3327-3441). So:
  - `on_failure: fail` (default) ⇒ run ends `Failed` (engine.rs:3288-3293)
  - `on_failure: retry:N` with budget ⇒ the checkpoint re-parks
  - `on_failure: goto:M` ⇒ **the run continues at step M despite the denial**

  Pinned by `deny_checkpoint_routes_through_on_failure_goto` (engine.rs:14563-14604 — asserts
  `SopRunAction::DeterministicStep { step.number == 3 }` and the checkpoint recorded `Failed`) and
  `deny_checkpoint_defaults_to_terminal_failure` (engine.rs:14723-14752 — asserts `SopRunAction::Failed`
  and run status `Failed`). A stale `goto` target is preflighted before any mutation:
  `resolve_sop_step(&sop, *step)?` at engine.rs:3363-3365 (agent path) and engine.rs:3148-3150 (broker
  path); test `deny_checkpoint_preflights_invalid_failure_goto_without_mutation` at engine.rs:14804.

**[ADD] `retries_consumed` on the deny paths does NOT subtract one.** Both deny sites count the *existing*
`Failed` results for the step **before** appending the denial record — engine.rs:3168-3176 (broker) and
engine.rs:3377-3388 (agent), with the explanatory comment at engine.rs:3372-3376: *"the router computes
`retries_consumed` as (Failed count - 1) after that record, so before it the current Failed count for this
step is exactly that value."* An implementer copying `failed_executions.saturating_sub(1)` into the deny
path would be off by one.

> **SOURCE OVERRIDES DOCS.** syntax.md:223 states flatly: *"On deny, the run is cancelled."* **Wrong on two
> counts.** (1) The run is not necessarily terminated — `on_failure: goto:M` continues it
> (engine.rs:3069-3072, 3147-3162; test :14563). (2) Even in the terminal case the status is
> `SopRunStatus::Failed`, not `Cancelled` (engine.rs:3288-3293; test :14723 asserts `Failed`).
> **Load-bearing for approval domination: a denied checkpoint with `on_failure: goto:M` is an
> approval-bypass edge into step M, and Egeria must emit that edge.**

**Asymmetry between the two gate species — do not conflate them.** A denied *approval gate*
(`WaitingApproval`) terminates the run **unconditionally as `Cancelled`**, and `on_failure` is never
consulted (`sop/approval/resolve.rs:158-167`):

```rust
ApprovalDecision::Deny { reason } => {
    let why = reason.unwrap_or_else(|| format!("denied by {}", principal.actor_label()));
    engine.finish_run_with_gate_event(run_id, SopRunStatus::Cancelled, Some(why), &gate_event)?;
    ResolveOutcome::Denied
}
```

| Gate species | Produced by | Denied ⇒ | Consults `on_failure`? | Before or instead of the body? |
|---|---|---|---|---|
| `WaitingApproval` | `requires_confirmation: true`, or execution mode (§6.2) — **only on the non-deterministic path** | run `Cancelled`, always terminal | **No** (resolve.rs:158-167) | **Before**; approval then executes the same step (engine.rs:3588-3595) |
| `PausedCheckpoint` | `kind: checkpoint` — **deterministic mode only** | routed through `on_failure`; `goto:M` **continues the run** | **Yes** (engine.rs:3069-3072) | **Instead of**; no body, passes the piped value through |

Both can be policied (`- policy:`), both carry a `- prompt:`; only a checkpoint accepts `- edit:`/Revise
(engine.rs:2372-2381, with the comment at :2355-2359 explaining that offering Edit/Revise on a
non-checkpoint park would render buttons whose submissions are always rejected). Amend/Revise on an
approval gate is refused before any side effect (resolve.rs:58-66, and defensively :170-174).

**[ADD] `approval_mode` is a third, orthogonal security layer** on `WaitingApproval` resolution
(resolve.rs:46-52): `is_rejected_by_approval_mode(engine.config().approval_mode, &principal)` →
`ResolveOutcome::RejectedSelfApproval`. Other non-resolving outcomes: `AlreadyResolved` / `NotWaiting`
(resolve.rs:42-43) and `DeferredAtCapacity` when the resume would exceed a concurrency cap
(resolve.rs:108-116). Egeria should model an approval gate as *possibly not clearing* even on an
approve decision.

`max_pending_approvals` (default `0` = unlimited — types.rs:500-504, `default_max_pending_approvals`
types.rs:521-523) caps how many runs of one SOP may be parked at a gate at once; over the cap, further
parks become `Pending` instead of `WaitingApproval`/`PausedCheckpoint` (engine.rs:2309-2312 and
4475-4478) and are re-promoted by the maintenance tick `retry_capacity_blocked_gated_pends`
(engine.rs:827-890).

---

### 7. Consolidated docs-vs-source disagreements

| # | Docs claim | Source truth | Severity for Egeria |
|---|---|---|---|
| 1 | syntax.md:165-167 — false `when:` ⇒ *"the run completes"* | route/mod.rs:70-75 — ⇒ **linear successor**, unless `terminal: true` | **Critical.** Wrong here means every bypass edge is missing. |
| 2 | syntax.md:170-171 — false `when:` ⇒ linear successor | **Correct**, and contradicts #1 six lines earlier | — |
| 3 | syntax.md — `switch:` documented **0 times** (`grep -c switch` = 0) | Real bullet; beats `next` and `terminal` (route/mod.rs:77-92) | **Critical.** An unmatched switch is a silent `Complete` past every downstream approval. |
| 4 | syntax.md — `terminal:` documented **0 times** | Real bullet, but **lowest** precedence (route/mod.rs:98) | High |
| 5 | syntax.md:223 — *"On deny, the run is cancelled"* (checkpoints) | Deny routes through `on_failure`; `goto:M` continues the run; terminal case is **`Failed`**, not `Cancelled` (engine.rs:3069-3072, 3288-3293; tests :14563, :14723) | **Critical** — an approval-bypass edge |
| 6 | syntax.md:160-162 — checkpoint framing reads as advisory | `kind` dispatched on **only** in `resolve_deterministic_action` (engine.rs:4473); outside deterministic mode `kind: checkpoint` is **not a gate at all** (engine.rs:5488-5508) | **Critical** |
| 6b **[ADD]** | syntax.md:161-162 — *"Use `requires_confirmation: true` when a step must require approval in any execution mode"* | **False in deterministic mode**: `resolve_step_action`/`step_requires_approval_gate` are reached only from `dispatch_llm_step` (sole call site engine.rs:2293); `resolve_deterministic_action` (4446-4568) never reads `requires_confirmation` | **Critical** — the documented universal gate does not exist |
| 7 | syntax.md:160 — `kind:` accepts *"`execute` (default) or `checkpoint`"* | Also `capability`, and `approval` as an undocumented alias for checkpoint; anything else silently ⇒ `execute` (mod.rs:753-759) | Medium |
| 8 | syntax.md — `- prompt:`, `- agent:`, `- call:`, `- capability:`, `- with:`, `- tools:`-adjacent aliases `on-failure:` / `depends-on:` / `allow_tools:` / `deny_tools:`: **0 mentions each** (verified by grep) | All parsed (mod.rs:553-641; full table §5.8) | Medium |
| 9 | syntax.md:423-424 — no AND/OR/NOT | **Correct, verified** (condition.rs:83-119) | — |
| 10 | syntax.md:303-304 — `max_step_visits` 256, `max_step_retries` 2 | **Correct** (config/schema.rs:22839-22845) | — |
| 11 | syntax.md nowhere | Switch-arm guards cannot contain `>` or `;`; `>`/`>=` unusable in a switch guard; empty-name arms silently dropped (mod.rs:733-751) | High — silently mis-parses to a dead arm |
| 12 **[ADD]** | Prior audit: *"There is no `[[steps]]` table in SOP.toml"* | **False.** `SopManifest.steps` (types.rs:593-594) has no `skip_serializing_if`, and `from_sop` populates it (types.rs:663), so `save_sop` writes `[[steps]]` on **every** save (mod.rs:966-968) — while `load_sop` prefers `SOP.md` (mod.rs:428-430) | **High** — a stale, always-present duplicate source |
| 13 **[ADD]** | mod.rs:929-931 — *"render -> parse is lossless"* | `render_step_bullets` (mod.rs:847-927) emits nothing for `capability`, `with`, `policy`, `prompt`, `edit`, or `kind: capability`. A `save_sop`→`load_sop` cycle **destroys** all five and demotes capability steps to execute. Untested (mod.rs:1230-1272) | **Critical** — a checkpoint's `policy:`/`edit:` vanish on first save |
| 14 **[ADD]** | condition.rs:68-70 — *"This order is the single scan order every parser and every authoring surface reads"* | `ConditionParts::parse` (condition.rs:243-272) derives its order from `catalog_tokens()` sorted by `Reverse(len)` → `>=`,`<=`,`==`,`!=`,`>`,`<`, swapping `==`/`!=` versus `parse_order()` (:71-80) | Medium — authoring/runtime split on strings containing both tokens |
| 15 **[ADD]** | syntax.md:364-366 — fail-closed list | Correct as far as it goes, but omits that **every direct-form guard is unconditionally false in routing** (condition.rs:53-56 vs rundata.rs:38-45) | High — statically dead guards |
| 16 **[ADD]** | graph.rs draws the implicit fallthrough from `steps.get(idx + 1)` (graph.rs:323) | Runtime uses `current_step + 1` by **number** (route/mod.rs:122). Diverges under non-contiguous numbering | Medium — do not import the graph view as control flow |

---

### 8. Open questions (undecidable from source — escalate, do not guess)

1. **OPEN QUESTION — duplicate step numbers.** `resolve_next` uses `.find()` (route/mod.rs:55-59, and
   `resolve_target` at :108), taking the first match; `normalize_step_numbers` explicitly refuses to
   renumber when duplicates exist (mod.rs:333-334: *"No-op when step numbers are ambiguous (duplicates),
   since a remap would guess"*). The SOP.md parser cannot produce duplicates (mod.rs:537-540), so this is
   reachable only via the `[[steps]]` fallback or programmatic construction. **Intended behavior
   unspecified.** Recommend Egeria reject duplicates as an error rather than mirroring first-wins.
2. **OPEN QUESTION — is the switch `>`/`;` separator collision (§5.7) a bug?** Nothing in source, tests,
   or docs addresses it. Be bug-compatible on parse and diagnostic on print; do not invent escaping.
3. **OPEN QUESTION — is an unmatched switch completing (rather than falling through) intentional?** Four
   tests pin it (route/mod.rs:373-462), so it is *deliberate*; no design note explains why a switch node
   without a catch-all silently ends a run **successfully** rather than failing. Egeria should flag
   `switch` without a catch-all arm as a finding.
4. **OPEN QUESTION — is `kind: checkpoint` outside deterministic mode a no-op gate or an unguarded
   footgun?** No validation rejects it (mod.rs:479 checks only capability steps); syntax.md:160-162 only
   advises. Raise a finding, do not silently lower.
5. **OPEN QUESTION — is `requires_confirmation: true` inside a deterministic SOP intended to be
   ignored?** (New; see §6.1 [FIX].) `resolve_deterministic_action` never reads it and
   `execution_mode_needs_approval` returns `false` for `Deterministic` (engine.rs:5455), but nothing in
   source or docs says the combination is meant to be inert — and syntax.md:161-162 says the opposite.
   Raise a finding.
6. **OPEN QUESTION — re-drive semantics for a dependency-blocked `Pending` run.** No maintenance path
   re-promotes it (engine.rs:846 filters to gated steps only); it depends on an external `sop_advance`
   (permitted by engine.rs:1842-1864). Whether this is a deliberate "agent owns the retry" contract or an
   oversight is not stated anywhere.
7. **OPEN QUESTION — `step_visit_count` counting `Skipped` parks against the budget** (route/guard.rs:3-10
   vs engine.rs:2472-2482). The consecutive-skip de-dup at engine.rs:2469-2471 suggests awareness, but no
   comment states whether non-consecutive skips consuming visit budget is intended.
8. **OPEN QUESTION — is the SOP.md render lossiness (§1.5, table row 13) known?** It contradicts the
   renderer's own doc-comment and is untested. Whether upstream intends `[[steps]]` in SOP.toml to be the
   lossless plane (it is written, but never read when SOP.md exists) is not stated. Egeria should
   round-trip losslessly and emit a fidelity warning when writing a SOP.md that would lose fields.
9. **OPEN QUESTION — do `{{steps.N}}` bindings in a step *body* actually resolve at runtime?**
   types.rs:321-323 documents that they do; `sop/binding.rs` is validated only for `calls[].args`
   (mod.rs:1010-1068) and the gate prompt. No body-substitution site was located. Do not assume they
   resolve.

---

#### Files an implementer must read in full

- `sop/route/mod.rs` (483 lines — code + **12** pinning tests at :241-482)
- `sop/route/guard.rs` (14 lines) · `sop/route/failure.rs` (34 lines)
- `sop/condition.rs` (681 lines) · `sop/rundata.rs` (275 lines) · `sop/step_contract.rs` (69 lines)
- `sop/types.rs:237-451` (`SopStepKind`, `StepSchema`, `PlannedToolCall`, `SopStep`), `:455-523` (`Sop` +
  defaults), `:525-578` (admission), `:580-670` (`SopManifest`, `SopMeta`, `from_sop`), `:694-736`
  (`SopRunStatus`, `SopStepStatus`)
- `sop/mod.rs:330-374` (renumber/remap), `:376-496` (load + manifest normalize), `:498-716` (Markdown step
  parser + `StepParseState`), `:718-845` (value parsers + `render_step_failure`), `:847-972` (renderer +
  `save_sop`), `:974-1068` (validation)
- `sop/engine.rs:1713-1770` (start), `:1770-1944` (advance + output-schema rewrite), `:1946-2056` (schema
  helpers + `record_step_result`), `:2058-2233` (routing + visit bound), `:2235-2344` (LLM dispatch),
  `:2424-2514` (step lookup + pend), `:3067-3320` (broker checkpoint decision), `:3322-3500`
  (`deny_checkpoint`), `:3512-3596` (`clear_waiting_gate`), `:4100-4242` (capability + deterministic
  record/route), `:4446-4568` (deterministic kind dispatch), `:5451-5510` (gate resolution),
  `:5513-5573` (context, piping, revision cap)
- `sop/approval/resolve.rs:32-178` (gate resolution; deny ⇒ `Cancelled` at :158-167)
- `sop/capability/registry.rs:40-110` · `sop/capability/builtins.rs:11-26, 98-119`
- `sop/graph.rs:287-478` (authoring-time diagnostics — a *view*, not the runtime) · `sop/wire.rs:80-170`
  (graph edits that write `routing.terminal` / `routing.switch`)
- `crates/zeroclaw-config/src/schema.rs:22548-22920` (`SopConfig` + every default)

---

# Part: Upstream examples and fixture material

## Fixture Corpus Harvest — Real Upstream SOP Examples (corrected & completed)

**Upstream root (READ-ONLY, unmodified):**
`/Users/hollow/Documents/Workflow Generator/egeria/external/zeroclaw`

Every `path:line` below is relative to that root. The five files that carry almost all of the load, in absolute form:

- `/Users/hollow/Documents/Workflow Generator/egeria/external/zeroclaw/crates/zeroclaw-runtime/src/sop/mod.rs` (parser, renderer, loader, save, validation, tests)
- `/Users/hollow/Documents/Workflow Generator/egeria/external/zeroclaw/crates/zeroclaw-runtime/src/sop/types.rs` (model + manifest + trigger TOML tests)
- `/Users/hollow/Documents/Workflow Generator/egeria/external/zeroclaw/crates/zeroclaw-runtime/src/sop/route/mod.rs` (routing precedence)
- `/Users/hollow/Documents/Workflow Generator/egeria/external/zeroclaw/crates/zeroclaw-runtime/src/sop/capability/builtins.rs` + `registry.rs` (capability id set + load-time validation)
- `/Users/hollow/Documents/Workflow Generator/egeria/external/zeroclaw/src/sop/mod.rs` (CLI-crate disk round-trip tests)

**Rule applied throughout: where the Rust source and `docs/book/src/sop/*.md` disagree, the Rust source wins, and both are cited.**

---

### 0. Headline findings

#### 0.1 — Zero `SOP.md` / `SOP.toml` files on disk anywhere upstream. ✅ (draft correct)

`find . -name "SOP.md" -o -name "SOP.toml"` returns nothing. Every example is inline: fenced blocks in docs, or Rust string literals / struct literals in `#[cfg(test)]` modules. Egeria's corpus must be **assembled**, not copied.

#### 0.2 — No SOP fixtures under `crates/zeroclaw-runtime/tests/`. ✅ (draft correct), but the draft's "two files" framing is wrong.

`crates/zeroclaw-runtime/tests/` holds only `cron_uses_memory_free_8695.rs`, `landlock_contract.rs`, `landlock_spawn_failure.rs`, `landlock_workspace_boundary.rs`, `scheduled_no_conversation_leak_5415.rs`. None touch SOP.

The draft says the parse-exercising tests live in "two files". They do not. **Eight** files contain literal `## Steps` markdown, and a ninth carries manifest TOML round-trips:

| File | Role |
|---|---|
| `crates/zeroclaw-runtime/src/sop/mod.rs:1125-1677` | Parser/renderer test module — contract bullets, `switch:`, policy, capability, admission policy, planned calls, positions, path traversal |
| `src/sop/mod.rs:242-638` | CLI-crate test module. `src/sop/mod.rs:1-2` is `#[allow(unused_imports)] pub use zeroclaw_runtime::sop::*;` so it exercises the **identical** parser; it owns the full disk round trips |
| `crates/zeroclaw-runtime/src/sop/types.rs:1113-1244`, `:1327-1376` | **Trigger TOML round-trips and manifest defaults — the draft missed this file entirely** (see §0.6) |
| `crates/zeroclaw-runtime/src/sop/procedural_memory.rs` | Minimal SOP.md literals + two SOP-emitting templates |
| `crates/zeroclaw-runtime/src/sop/engine.rs`, `graph.rs`, `store/mod.rs`, `approval/broker.rs`, `crates/zeroclaw-runtime/src/tools/mod.rs`, `crates/zeroclaw-runtime/src/tools/sop_workshop.rs` | struct-literal SOPs + two on-disk manifest+markdown literals |

#### 0.3 — `[[steps]]` in `SOP.toml` is real. ✅ (draft correct — audit fact #6 is wrong)

`crates/zeroclaw-runtime/src/sop/types.rs:583-595`:
```rust
pub struct SopManifest {
    pub sop: SopMeta,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub triggers: Vec<SopTrigger>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub positions: Vec<StepPosition>,
    #[serde(default)]
    pub steps: Vec<SopStep>,
}
```

`crates/zeroclaw-runtime/src/sop/mod.rs:427-435` — precedence is **SOP.md wins; `[[steps]]` is the fallback only when `SOP.md` does not exist on disk** (`md_path.exists()`, not "is non-empty"):
```rust
    let md_path = sop_dir.join("SOP.md");
    let mut steps = if md_path.exists() {
        let md_content = std::fs::read_to_string(&md_path)?;
        parse_steps(&md_content)
    } else if !manifest.steps.is_empty() {
        normalize_manifest_steps(manifest.steps)
    } else {
        Vec::new()
    };
```
**Corollary the draft missed:** an *empty or step-less* `SOP.md` that exists still wins and yields zero steps — `[[steps]]` is never consulted. That is a required fixture.

`normalize_manifest_steps` (`mod.rs:483-496`) fills `number` from 1-based position **only when `number == 0`** (so a `[[steps]]` block with explicit numbers keeps them, including gaps), and defaults an empty `title` to `capability` then `kind.to_string()`.

`types.rs:663` writes steps into the TOML with no `skip_serializing_if`, called from `mod.rs:966-969`:
```rust
    let manifest = SopManifest::from_sop(sop);
    let toml_content = toml::to_string_pretty(&manifest)?;
    std::fs::write(sop_dir.join("SOP.toml"), toml_content)?;
    std::fs::write(sop_dir.join("SOP.md"), render_steps(&sop.steps))?;
```
`docs/book/src/sop/syntax.md:14` says only *"Each SOP must have `SOP.toml`. `SOP.md` is optional, but runs with no parsed steps will fail validation."* — it never mentions `[[steps]]`. **Source wins.**

**`[[steps]]` TOML shape** (the draft gave none — the implementer needs it). Field names are serde names from `types.rs:311-392`, so `capability_input` is spelled **`with`** (`types.rs:373`) and `pos` may appear inline:
```toml
[[steps]]
number = 1
title = "Draft"
body = "..."
suggested_tools = ["shell"]
requires_confirmation = false
kind = "capability"           # snake_case: execute | checkpoint | capability (types.rs:242)
capability = "llm.generate"
with = { instruction = "..." }
mode = "auto"
policy = "prod"
gate_prompt = "..."
edit = "body"
agent = "pr_bot"
pos = { x = 320.5, y = -48.0 }
[steps.schema]
input = { type = "object" }
[steps.routing]
when = "$.steps.1.ok == true"
next = 3
terminal = false
depends_on = [1, 2]
[[steps.routing.switch]]
name = "pr"
when = "$.event"
goto = 2
[steps.on_failure.retry]      # StepFailure is externally tagged (step_contract.rs:51-63)
max = 2
```
`on_failure` is `"fail"` (unit variant) or `{ retry = { max = N } }` / `{ goto = { step = N } }`. Every field carries `#[serde(default)]`, proven by `types.rs:1327-1338` (`step_defaults`: `{"number":1,"title":"Check","body":"..."}` parses).

#### 0.4 — Step numbers in `SOP.md` are ignored. ✅ (draft correct)

`mod.rs:537-540`:
```rust
            let step_num = u32::try_from(steps.len())
                .unwrap_or(u32::MAX)
                .saturating_add(1);
            current.reset_for_step(step_num);
```
`parse_numbered_item` (`mod.rs:809-817`) uses `line.find(". ")` and requires the prefix be **non-empty and all ASCII digits**:
```rust
fn parse_numbered_item(line: &str) -> Option<&str> {
    let dot_pos = line.find(". ")?;
    let prefix = &line[..dot_pos];
    if prefix.chars().all(|c| c.is_ascii_digit()) && !prefix.is_empty() {
        Some(line[dot_pos + 2..].trim())
    } else { None }
}
```
**Additions the draft missed:** it uses the **first** `". "` in the line, `07.` is accepted (leading zeros), and `1.**Bold**` (no space after the dot) is **not** a numbered item — it becomes body continuation of the previous step. Scrambled-numeral fixture confirmed as a must-have. Note the contrast with `validate_sop` (`mod.rs:993-1001`), which warns `"Step numbering gap: expected {expected}, got {}"` — unreachable from a markdown-parsed SOP, reachable from a `[[steps]]` SOP.

#### 0.5 — `render_steps` is lossy, contrary to its own doc comment. ✅ (draft correct; consequence understated)

`mod.rs:929-931` claims *"Every contract field (tools, scope, schema, routing, failure policy, mode) becomes a sub-bullet, so render -> parse is lossless."* `render_step_bullets` (`mod.rs:847-927`) emits **no** bullet for `capability`, `capability_input` (`with:`), `policy`, `gate_prompt` (`prompt:`), or `edit`, and emits `kind:` only for `Checkpoint` (`mod.rs:864-866`) — so `kind: capability` degrades to `execute`.

**The draft's consequence is wrong.** It says "only the `[[steps]]` TOML copy retains them." The `[[steps]]` copy is *written* but **never read back**, because `save_sop` always writes `SOP.md` (`mod.rs:969`) and `load_sop` prefers an existing `SOP.md` (`mod.rs:428`). So `save_sop` → `load_sop` **permanently destroys** every capability step's identity, every `with:`, every gate `policy`/`prompt`/`edit`. This is a genuine upstream data-loss defect, uncovered by any test — `render_parse_roundtrip_preserves_full_step_contract` (`mod.rs:1227-1268`) sets none of those fields.

**Egeria's printer must emit all 21 bullets the parser accepts** (§1.1) and document the divergence. `render_step_failure` (`mod.rs:839-845`) renders with a space — `retry: 2`, `goto: 3` — which `parse_step_failure` (`mod.rs:775-795`) accepts via `strip_prefix("retry:") … .trim()`. Egeria may emit either form; accept both.

#### 0.6 — NEW: the draft's biggest factual error — the capability registry

The draft states: *"both `forge.comment` and `llm.generate` are absent from `with_builtins()`, meaning a fixture using them will not load via the plain `load_sop` path"*, and raises an open question on that basis. **This is false.** `crates/zeroclaw-runtime/src/sop/capability/builtins.rs:11-26`:
```rust
pub(super) fn register(registry: &mut SopCapabilityRegistry) {
    registry.register(NoopCapability);
    registry.register(WaitCapability);
    registry.register(ApprovalWaitCapability);
    registry.register(JsonValidateCapability);
    registry.register(ShellExecCapability);
    registry.register(GitStatusCapability);
    registry.register(GitDiffCapability);
    registry.register(NotifyChannelCapability);
    // Injected-adapter capabilities, registered here as FAIL-CLOSED placeholders
    // (adapter = None) so SOPs referencing them pass load-time validation
    // everywhere; `build_sop_engine` re-registers them with real adapters when
    // the daemon supplies one (a later `register` for the same id overwrites).
    registry.register(super::forge_comment::ForgeCommentCapability::new(None));
    registry.register(super::llm_generate::LlmGenerateCapability::new(None));
}
```
The comment says the opposite of the draft in so many words. `mod.rs:152-154` is the daemon **re-**registration, not the only one. See §5.1 for the corrected id set (**ten**, not eight) and the two extra load-time rules the draft missed.

#### 0.7 — NEW: `## Steps` section boundaries and `extract_bold_title` traps

`mod.rs:513-526`: a line is a heading only if `trimmed.starts_with("## ")`. `"### Sub"` does **not** match (index 2 is `#`, not a space), so `###` and `#` headings inside the section become **body text**, not section terminators. Only `## steps` (ASCII-case-insensitive, exact after trim — `mod.rs:514`) opens the section; **any other `## ` heading closes it and flushes the pending step** (`mod.rs:520-524`). `## Steps (v2)` therefore closes rather than opens.

`extract_bold_title` (`mod.rs:820-837`) finds the **first `**` anywhere in the line**, not at the start. So `1. Do **the** thing` yields `title = "the"`, `body = "thing"`. Separators stripped after the closing `**` are exactly `—` (em dash), `–` (en dash), `-` (ASCII hyphen), one occurrence, or nothing (`unwrap_or(rest)`). No fixture covers mid-line bold upstream; Egeria must decide (see OPEN QUESTION 7).

`mod.rs:554` uses `trimmed.trim_start_matches("- ")`, which strips **every** repetition — `- - tools: x` parses as `tools: x`. Indentation is irrelevant (the line is `trim()`ed at `mod.rs:510`), so nesting depth carries no meaning. A `- ` bullet before any numbered item is discarded (`current.number.is_some()` guard, `mod.rs:553`) and is *not* body text either, because the fallback at `mod.rs:645` has the same guard.

---

### 1. The complete grammar surface (added — the draft never enumerated it)

#### 1.1 All 21 bullet keys, in parser match order

From `mod.rs:555-633`. Order matters: the first matching `strip_prefix` wins.

| # | Key | Aliases | Line | Value handling |
|---|---|---|---|---|
| 1 | `tools:` | — | 555-556 | `parse_csv_list` → `suggested_tools` |
| 2 | `allow-tools:` | `allow_tools:` | 557-561 | `scope.allow = Some(csv)` |
| 3 | `deny-tools:` | `deny_tools:` | 562-566 | `scope.deny = csv` |
| 4 | `requires_confirmation:` | none | 567-570 | `eq_ignore_ascii_case("true")`; anything else = false |
| 5 | `kind:` | — | 571-574 | `parse_step_kind` |
| 6 | `capability:` | — | 575-576 | trimmed string (empty string allowed → `Some("")`) |
| 7 | `with:` | — | 577-578 | `parse_value_fragment` |
| 8 | `input:` | — | 579-580 | `parse_value_fragment` → `schema.input` |
| 9 | `output:` | — | 581-582 | `parse_value_fragment` → `schema.output` |
| 10 | `when:` | — | 583-587 | empty value leaves `None` |
| 11 | `next:` | — | 588-589 | `parse::<u32>().ok()` — non-numeric silently `None`, **overwriting any prior value** |
| 12 | `terminal:` | — | 590-591 | `eq_ignore_ascii_case("true")` |
| 13 | `depends_on:` | `depends-on:` | 592-596 | `parse_u32_list` — non-numeric entries silently dropped |
| 14 | `switch:` | — | 597-598 | `parse_switch_rules` |
| 15 | `on_failure:` | `on-failure:` | 599-603 | `parse_step_failure` |
| 16 | `mode:` | — | 604-605 | `parse_execution_mode` |
| 17 | `agent:` | — | 606-608 | empty → `None` |
| 18 | `call:` | — | 609-612 | `serde_json::from_str::<PlannedToolCall>`; **parse failure silently drops the call** |
| 19 | `prompt:` | — | 613-617 | empty → `None`; sets `gate_prompt` |
| 20 | `policy:` | — | 618-624 | empty → `None` |
| 21 | `edit:` | — | 625-633 | empty → `None` |
| — | anything else | — | 634-640 | appended to `body` |

**Corrections/additions vs the draft:** the draft listed the alias set but never the ordered key list, and omitted that `requires_confirmation` has **no** hyphen alias, that `next:`/`depends_on:` degrade silently on non-numeric input, and that `capability:` accepts the empty string (which then fails `validate_sop` at `capability/registry.rs:44-48` with *"is kind=capability but has no capability id"* only if `kind == capability`; a `capability:` on an `execute` step is retained and ignored).

#### 1.2 Sub-grammars

**`parse_step_kind`** (`mod.rs:753-759`) — lowercased, trimmed:
```rust
        "checkpoint" | "approval" => SopStepKind::Checkpoint,
        "capability" => SopStepKind::Capability,
        _ => SopStepKind::Execute,
```
`approval` is an undocumented alias; **any unknown value silently becomes `Execute`**, never an error. `syntax.md:160-161` says *"`- kind:` accepts `execute` (default) or `checkpoint`"* and omits `capability` entirely even though `syntax.md:252-276` uses it. **Source wins.**

**`parse_execution_mode`** (`mod.rs:170-179`): `auto`, `step_by_step`, `priority_based`, `deterministic`; **everything else, including typos, silently becomes `Supervised`** — not an error, and not `Auto`.

**`parse_step_failure`** (`mod.rs:775-795`): `fail` (case-insensitive) → `Fail`; `retry:N` **or** `retry N` → `Retry{max:N}`; `goto:N` **or** `goto N` → `Goto{step:N}`; **anything unrecognized silently becomes `Fail`**.

**`parse_switch_rules`** (`mod.rs:733-751`): `;`-separated; each segment `splitn(3, '>')` as `name>when>goto`; empty `name` drops the rule; empty `when` → `None` (catch-all port); `goto.parse::<u32>().ok()`. `catch_all>>2` = `{name:"catch_all", when:None, goto:Some(2)}`. **Because of `splitn(3,'>')`, a `when` containing `>` is truncated at the first `>`: `sev>$.value > 85>4` parses to `when: Some("$.value")` (the `.trim()` at `mod.rs:742` removes the trailing space — the draft wrote `Some("$.value ")`, which is wrong), `goto: None`.** Rendering (`mod.rs:894-907`) always emits all three fields joined by `>`, so a `goto: None` port round-trips as `name>when>`.

**`parse_value_fragment`** (`mod.rs:761-773`) — backs `with:`, `input:`, `output:`:
```rust
fn parse_value_fragment(value: &str) -> serde_json::Value {
    if let Ok(json) = serde_json::from_str(value) { return json; }
    let wrapped = format!("value = {value}");
    if let Ok(toml_value) = toml::from_str::<toml::Value>(&wrapped)
        && let Some(value) = toml_value.get("value")
        && let Ok(json) = serde_json::to_value(value) { return json; }
    serde_json::Value::String(value.into())
}
```
JSON first, then TOML-inline, then **an unparseable fragment silently becomes a JSON string** — never an error.

**`parse_csv_list`** (`mod.rs:718-724`): splits on `,`, trims, drops empties. **No validation of tool names whatsoever.**

#### 1.3 Scope groups — entirely missing from the draft

`crates/zeroclaw-runtime/src/sop/scope/groups.rs:1-26` defines four expandable **group aliases** usable in `allow-tools:` / `deny-tools:`:

| Alias(es) | Expands to |
|---|---|
| `fs`, `filesystem` | `read_file`, `write_file`, `edit_file` |
| `web`, `network` | `http_request`, `web_search` |
| `shell`, `terminal` | `shell` |
| `sop`, `sop-control`, `sop_control` | `sop_execute`, `sop_advance`, `sop_approve`, `sop_status` |

Expansion happens at enforcement time in `scope::resolve_excluded` (`scope/mod.rs:23-65`), **not** at parse time — `parse_steps` stores the literal token (`mod.rs:1443-1446` asserts `scope.allow == Some(vec!["fs"])`). Egeria's parser must therefore store `fs` verbatim, and any Egeria analysis of scope must apply this expansion table itself.

**Upstream defect worth recording:** `read_file`, `write_file`, `edit_file`, `web_search` are **not registered tool names**. The real registry has `file_read`, `file_write`, `file_edit`, `web_search_tool` (`crates/zeroclaw-tools/src/file_write.rs:38` and the enumeration in §5.2). So `- allow-tools: fs` under `step_scope_enforce = true` excludes every real filesystem tool. Flag; do not replicate silently.

#### 1.4 `SopStep::capability_call_input` — missing from the draft

`types.rs:425-433`: when a capability step has authored `with:`, the piped input from the previous step is inserted **under the key `"input"`** of that object (via `object.entry("input").or_insert(piped_input)`); when `with:` is absent, the piped value **is** the whole input. This is why `llm.generate` documents an `input` property (`llm_generate.rs:78`) and why `forge.comment` reads `repo`/`number`/`body` from either the top level or a nested `input` object (`forge_comment.rs:100-140`). Fixtures that mix authored `with:` and piping must respect it.

---

### 2. Documentation examples (verbatim inventory)

#### D1 — `SOP.toml`, `deploy-prod` — `syntax.md:64-75` ✅ citation correct
```toml
[sop]
name = "deploy-prod"
description = "Production deploy with approval"
version = "1.0.0"
max_concurrent = 1
admission_policy = "hold"
max_pending_approvals = 8

[[triggers]]
type = "manual"
```
The only hand-authored `SOP.toml` in the docs tree; `syntax.md:18-20` says *"This page intentionally does not enumerate manifest fields or provide hand-authored manifest examples."*

**Missing from the draft: the full `[sop]` field set.** The authority is `SopMeta` (`types.rs:606-633`), and it has fields the draft never lists:

| `[sop]` key | Required? | Default | Citation |
|---|---|---|---|
| `name` | **required** | — | `types.rs:608` |
| `description` | **required** | — | `types.rs:609` |
| `version` | optional | `"0.1.0"` | `types.rs:610-611`, `types.rs:668-670` |
| `priority` | optional | `normal` | `types.rs:612-613`; values `low\|normal\|high\|critical`, lowercase (`types.rs:12-21`) |
| `execution_mode` | optional | `None` → caller default | `types.rs:614-615`; values snake_case (`types.rs:39-54`) |
| `cooldown_secs` | optional | `0` | `types.rs:616-617`, `:513-515` |
| `max_concurrent` | optional | `1` | `types.rs:618-619`, `:517-519` |
| `deterministic` | optional | `false` | `types.rs:620-622` |
| `admission_policy` | optional | `parallel` | `types.rs:623-625`, `:530-548` |
| `max_pending_approvals` | optional | `0` (unlimited) | `types.rs:626-628`, `:521-523` |
| **`agent`** | optional | `None` | **`types.rs:629-632`** — parent agent alias; consumed at `mod.rs:453,477`; **absent from every doc page and from the draft** |

`SopManifest`/`SopMeta` carry no `deny_unknown_fields`, so **unknown `[sop]` keys are silently ignored**. Missing `name` or `description` is a hard TOML parse error (`mod.rs:425`) and the SOP is skipped with a WARN (`mod.rs:403-414`).

#### D2 — Approval groups/quorum, main config not `SOP.toml` — `syntax.md:81-89` ✅
```toml
[sop.approval.groups.release]
members = ["http:<paired-token-subject>", "agent:release-bot"]

[sop.approval.policies.prod]
required_group = "release"
quorum = 2
escalation_route = "oncall"
```
`syntax.md:77-79` places these in the main config. The draft's catch stands: `escalation_route = "oncall"` contradicts `syntax.md:196-198`, which requires `channel:recipient`. Docs-internal inconsistency; use the D3 form.

#### D3 — Policy with both routes — `syntax.md:186-194` ✅ (unchanged)

#### D4 — Basic step format — `syntax.md:110-123` ✅
The draft's `next: 3` observation is right and the citation checks out: `route/mod.rs:107-113`,
```rust
    let Some(step) = ctx.sop.steps.iter().find(|s| s.number == target) else {
        return match kind {
            TargetKind::Explicit => NextStep::Fail(format!("step {target} does not exist")),
            TargetKind::Linear if target > ctx.run.total_steps => NextStep::Complete,
            TargetKind::Linear => NextStep::Fail(format!("step {target} does not exist")),
        };
    };
```
Note the asymmetry the draft omitted: an **explicit** dangling target always fails; a **linear** overrun past `total_steps` completes normally. Both branches are needed as fixtures.

#### D5 — Combined routing example — `syntax.md:127-152` ✅ (analysis correct, one refinement)
Verbatim text as in the draft. The `when:`-contradiction resolution is correct: `syntax.md:165-167` says the run *completes*; `syntax.md:170-171` says it *advances linearly*. `route/mod.rs:70-75` settles it for 170-171:
```rust
    if !when_allows_jump {
        if current.routing.terminal {
            return NextStep::Complete;
        }
        return resolve_linear(ctx);
    }
```
and the normative decision tree is the doc comment at `route/mod.rs:31-49`. **Refinement:** the draft says "the guard is a no-op here". More precisely, in this example `next: 2` and the linear successor coincide, so the *outcome* is identical but the *path* is not (`resolve_target(…, Explicit)` vs `resolve_linear` → `Linear`), and the two differ on a dangling target. Egeria must model the branch, not the outcome.

Also from the same tree, and absent from the draft: **a non-empty `switch` completely suppresses `routing.next` and the linear successor** (`route/mod.rs:77-92`); if no port matches, the run **completes** (`route/mod.rs:91`); a matching port with `goto: None` is a **hard `Fail`** at runtime (`route/mod.rs:86-88`) even though the graph projection only warns (`graph.rs:1191`).

#### D6 — Headless review pipeline — `syntax.md:271-276` ✅ (draft's core finding correct)
```md
1. **Draft** - kind: capability / capability: llm.generate
   - with: { instruction = "...", output_key = "body", echo = ["repo", "number"] }
2. **Approve** - kind: checkpoint / policy: triage
3. **Post** - kind: capability / capability: forge.comment
```
**Not valid `SOP.md`.** The `kind:`/`capability:`/`policy:` pairs sit in the step *body* separated by `/`, not as `- ` bullets, so only `mod.rs:553`'s `starts_with("- ")` test on the `with:` line fires. Traced: step 1 → `title="Draft"`, `body="kind: capability / capability: llm.generate"`, `kind=Execute`, `capability=None`, but `capability_input=Some({...})`. Step 2 → `body="kind: checkpoint / policy: triage"`, `kind=Execute`, `policy=None`. **Addition the draft missed:** this still *loads* cleanly, because `capability/registry.rs:41-43` skips any step whose `kind != Capability`. It fails silently, which is worse. **Do not copy as a fixture.**

Corrected form (this is fixture #17):
```md
### Steps

1. **Draft** - Draft the triage comment.
   - kind: capability
   - capability: llm.generate
   - with: { instruction = "Summarize the issue and propose a triage label.", output_key = "body", echo = ["repo", "number"] }

2. **Approve** - Human review of the draft.
   - kind: checkpoint
   - policy: triage
   - edit: body

3. **Post** - Post the approved comment.
   - kind: capability
   - capability: forge.comment
   - with: { repo = "o/r", number = 7, body = "triage approved", channel = "git.main" }
```
Field authority — **corrected citations**: `llm.generate`'s `instruction` (required), `system`, `output_key` (default `text`), `echo` at `syntax.md:257-263` **and normatively at `capability/llm_generate.rs:71-80`** (the JSON Schema, which is what load-time validation actually runs). `forge.comment`'s `repo` (`owner/repo`), `number`, `body`, optional `channel` (`git.<alias>`) at `syntax.md:264-267`, enforced at `capability/forge_comment.rs:126-133`; the `channel` cross-check rule (`input.channel` must equal top-level `channel`) is at `forge_comment.rs:100-126` and the draft missed it.

**Critical, missed by the draft:** `llm.generate` sets `requires_authored_input() -> true` (`llm_generate.rs:88-90`). `capability/registry.rs:56-65` therefore **validates the authored `with:` against the input schema at load time**, and `registry.rs:115-129` rejects a missing or non-object `with:`:
```rust
    let configured = step.capability_input.clone().with_context(|| {
        format!("capability '{}' requires authored `with` configuration", capability.id())
    })?;
    if !configured.is_object() { bail!("… requires authored `with` configuration to be an object"); }
```
So **an `llm.generate` step without `with: { instruction = … }` fails `load_sop` outright**. `forge.comment` does not override `requires_authored_input` (default `false`, `capability/types.rs:59-61`), so it may omit `with:`.

#### D7 — `when:` guard payload shape — `syntax.md:354-362` ✅

#### D8/D9/D10 — Cookbook — `cookbook.md:9-18`, `:24-32`, `:38-46` ✅ (all three verbatim as in the draft; no manifests, per `cookbook.md:7,22,36`)

#### D11 — Directory layout — `syntax.md:7-12` ✅

#### D12 — `stagex-update` — **does not exist as SOP text** ✅ (draft correct)
`docs/book/src/sop/example.md` is 166 lines of prose with no `SOP.toml`, no `SOP.md`, no `## Steps` block. `example.md:109` introduces a **table**, `example.md:111-120`, reproduced correctly by the draft. Supporting facts verified: name `stagex-update` (`example.md:32,53-54`), location `<install>/shared/sops/stagex-update/` (`example.md:32`), `deterministic` mode (`example.md:35,103`), fired via `sop_execute` with a manual trigger (`example.md:94-99`), payload at `example.md:96`, step 3 "retry once" (`example.md:115`). **Must be labelled reconstructed.** Two corrections: the audit-key table is at `example.md:147-151` (not 146-151) and uses `<run-id>` placeholders; the `{run_id}` form the draft quotes comes from `docs/book/src/sop/observability.md:11-14`.

#### D13 — Fan-in pages carry no examples ✅ with two corrections
Directives verified: `{{#sop-trigger cron}}` `fan-in/cron.md:9`, `webhook.md:9`, `mqtt.md:9`, `amqp.md:9`, `filesystem.md:9`, `calendar.md:9`, `peripheral.md:9`, `{{#sop-trigger channel}}` at **both** `channel.md:9` and `git.md:9`, `{{#sop-trigger-index}}` at `fan-in/overview.md:18` and `syntax.md:344`.

1. **The draft omitted `fan-in/manual.md`** — it exists and contains **no** directive at all (only `sh` blocks and prose), which is why the index has 11 pages but only 10 directive sites.
2. `docs/book/src/sop/how-it-works.md`, `index.md`, and `observability.md` contain **zero** ` ```toml `/` ```md `/`## Steps`/`[sop]`/`[[triggers]]` occurrences — verified by grep. The docs tree contributes nothing further.

Authoritative trigger fields, `types.rs:142-229` (draft's summary is correct; adding the `serde` details it omitted): tag is `type`, `#[serde(tag = "type", rename_all = "lowercase")]` (`types.rs:126`), so variant names are lowercase.
- `mqtt { topic, condition? }` (145-152)
- `webhook { path }` (155-158) — **no `condition` field**
- `cron { expression }` (161-164) — **no `condition` field**
- `peripheral { board, signal, condition? }` (167-175)
- `filesystem { path, events: Vec<FilesystemEventKind> = [], condition? }` (178-187); event kinds `created|modified|deleted|renamed` (`types.rs:86-91`)
- `calendar { calendar_source, calendar_ids: Vec<String> = [], condition? }` (190-199)
- `channel { channel, alias?, condition? }` (206-216)
- `manual` (218) — unit variant, `type = "manual"` only
- `amqp { routing_key, condition? }` (221-228)

**NEW docs-vs-source conflict the draft missed:** `fan-in/git.md:13` repeatedly says the SOP's `channel` trigger has a "topic" that is "matched exactly against the event topic" (`git.<alias>:<event_type>`). **The `Channel` variant has no `topic` field** (`types.rs:206-216`); matching is by `channel` + optional `alias`, with event-type filtering pushed into `condition` — which is exactly what the doc comment at `types.rs:200-205` says (*"puts `event_type` in the payload, so an authored `condition` filters forge events by type without a second trigger shape"*) and what the upstream test does (`types.rs:1223-1236`: `channel="git"`, `alias="main"`, `condition="$.event_type == \"pull_request.opened\""`). **Source wins.** A fixture written from `git.md`'s prose would not deserialize.

---

### 3. Rust markdown fixtures (verbatim — highest value)

Draft citations R1–R14 were spot-checked line by line; ranges below are corrected where they were off.

**R1 — 3-step sensor SOP with a preceding `## Conditions`** — `src/sop/mod.rs:251-267`, test `parse_steps_basic`. Asserts at `:269-285` (draft said 270-284). Proves a `## Conditions` heading *before* `## Steps` is ignored, and per `mod.rs:519-524` any `##` heading *after* terminates and flushes. A trailing-`## Notes` fixture is required.

**R2 — no bold title** — `src/sop/mod.rs:294-299`; md literal at `:295`. Whole remainder becomes `title`; `body` stays empty (`mod.rs:546-548`).

**R3 — multi-line body** — `src/sop/mod.rs:302-315`; md at `:303-309`. Continuations joined with `\n` (`mod.rs:645-650`), `trim()`ed on flush (`mod.rs:696`).

**R4 — full disk round trip `test-sop`** — `SOP.toml` `src/sop/mod.rs:325-340`, `SOP.md` `:346-355`, asserts `:363-370`. `priority == High`, `execution_mode == Auto`, `cooldown_secs == 60`, 2 triggers, 2 steps, `location.is_some()`. Canonical manifest+markdown pair; enum values are lowercase / snake_case.

**R5 — all-trigger-types manifest** — `src/sop/mod.rs:528-554`, asserts `:556-565`. Five triggers Mqtt/Webhook/Cron/Peripheral/Manual. Note this test only deserializes `SopManifest`; it does not load from disk.

**R6 — `deterministic` overrides `execution_mode`** — `SOP.toml` `src/sop/mod.rs:576-584`, `SOP.md` `:590-601`, asserts `:608-615`. The draft cited only `:610`; the test **also** asserts `sop.deterministic` (`:611`) and all three step kinds (`:613-615`). Mechanism at `mod.rs:456-461`. A fixture with `deterministic = true` **and** a conflicting `execution_mode = "auto"` is required — `deterministic` wins unconditionally.

**R7 — checkpoint kind / default kind** — `src/sop/mod.rs:620-630`, asserts `:631-636`.

**R8 — manifest only, no `SOP.md`** — `src/sop/mod.rs:395-402`, assert `:408`. Produces the `"SOP has no steps (missing or empty SOP.md)"` warning (`crates/zeroclaw-runtime/src/sop/mod.rs:989-990`).

**R9 — execution mode falls back to caller default** — `src/sop/mod.rs:419-426`, asserts `:430-432`.

**R10 — legacy `tools:` aliases `scope.allow`** — `crates/zeroclaw-runtime/src/sop/mod.rs:1391-1396`, asserts `:1399-1410`. `suggested_tools == ["read_file","shell"]`, `scope.is_none()`, `effective_tool_scope().allow == Some([...])` (mechanism at `types.rs:435-444`: the alias applies **only when `scope.allow` is `None`** — an explicit `allow-tools:` wins and `tools:` is then advisory only; the draft did not state this precedence). ASCII `-` separator; no blank line between `## Steps` and `1.`.

**R11 — the full-contract fixture, the only `switch:` bullet upstream** — `crates/zeroclaw-runtime/src/sop/mod.rs:1416-1429`, expected parse `:1433-1470`. Verbatim as in the draft; grammar analysis in §1.2. `switch` in docs: **zero** bullet occurrences under `docs/book/src/sop/`; the single hit is the English phrase *"dispatch switch:"* at `fan-in/channel.md:5`. Confirms audit fact #1.

**R12 — `policy:` present and absent** — `mod.rs:1622-1628`, asserts `:1630-1634`.

**R13 — `capability:` + `with:` (TOML-inline)** — `mod.rs:1639-1646`, asserts `:1649-1655`. `git.status` is a real builtin (`builtins.rs:16`, id at `:207`), so this fixture loads. Both JSON and TOML-inline forms must be covered, plus the silent string degradation (§1.2).

**R14 — admission policy round trip** — `mod.rs:1665-1668`, asserts `:1671-1675`; test comment at `:1660-1661`. Only `drop` is exercised; `parallel`/`hold`/`coalesce` (`types.rs:536-547`, `syntax.md:39-49`) have no fixture.

**R15 — `canary`** — `crates/zeroclaw-runtime/src/tools/mod.rs:2500-2502` (`SOP.toml`) and `:2503-2506` (`SOP.md`, literal at `:2505`). The draft's `2499-2506` is one line early.
```md
### Steps

1. **Resolve** Do the first step
   - tools: shell
```
No separator after `**Resolve**` — handled by the `unwrap_or(rest)` branch (`mod.rs:829-834`), corroborated by `extract_bold_title_no_separator` (`src/sop/mod.rs:514-519`).

**R16 — minimal one-line SOPs** — all draft citations verified: `procedural_memory.rs:557, 582, 592, 598, 618, 635, 679, 735`; `engine.rs:7974`; `store/mod.rs:699`; `tools/sop_workshop.rs:329`. **Additions:** `procedural_memory.rs:650` and `:686` are golden expected strings (note `:650` has **no trailing newline** — proof the parser tolerates an unterminated final line), and `procedural_memory.rs:666` is a further proposal literal.

**R17 (NEW) — two SOP-emitting templates the draft missed**, both worth transcribing as fixtures because they are what upstream actually writes to disk from procedural memory:
- `procedural_memory.rs:309-315`, `default_manifest_toml` — the minimal real manifest:
  `[sop]\nname = "…"\ndescription = "…"\nversion = "0.1.0"\n\n[[triggers]]\ntype = "manual"\n`
- `procedural_memory.rs:317-336`, `default_procedure_markdown` — a **second, even lossier** renderer emitting only `# <name>`, `## Steps`, and the `tools:` / `requires_confirmation:` bullets, with a blank line after each step.

**R18 (NEW) — the trigger TOML round-trips in `types.rs`, which the draft declared nonexistent.** Its §4 claim *"filesystem / calendar / channel / amqp triggers in TOML: NONE"* is wrong for three of the four:
- `channel` — `types.rs:1222-1236`: `type="channel"`, `channel="git"`, `alias="main"`, `condition="$.event_type == \"pull_request.opened\""`
- `filesystem` — `types.rs:1183-1205`: `path="/var/inbox/**/*.json"`, `events=["created","modified"]`, `condition="$.extension == \"json\""`; plus `types.rs:1207-1220`, the events-default-empty case with `path="/var/inbox"`
- `calendar` — `types.rs:1148-1160`: `calendar_source="microsoft365"`, `calendar_ids=["primary","team"]`
- `mqtt` — `types.rs:1162-1172`: `topic="facility/pump/pressure"`, `condition="$.value > 85"`
- `manual` — `types.rs:1174-1179`: `type = "manual"` alone
- `amqp` — the draft is right here: **no TOML example**. Struct-only, e.g. `crates/zeroclaw-runtime/src/sop/dispatch.rs:1979-1983` (`routing_key: "orders.new"`) and `crates/zeroclaw-channels/src/amqp.rs:912-915` (`routing_key: "anitya.update"`).

**R19 (NEW) — minimal manifest with defaults** — `types.rs:1357-1376`, `manifest_parse`: `[sop]` with only `name`/`description` plus a `manual` and a `webhook` trigger, asserting `priority == Normal` and **`execution_mode == None`** (i.e. the manifest-level `Option`, before `load_sop`'s fallback). This is the cleanest proof of manifest defaults and belongs in the corpus.

---

### 4. Struct-literal fixtures

**S1 — `checkpoint -> forge.comment`** — `engine.rs:12580-12605` (`checkpoint_forge_comment_sop_with_channel`; the no-arg wrapper is `:12576-12578`). Forge step verbatim at `engine.rs:12541-12555`; default channel `"git.main"` at `engine.rs:12538`. Trigger payload `engine.rs:12520-12535`. Fails closed without a preceding checkpoint — `engine.rs:3941`, negative fixture `direct_forge_comment_sop` at `engine.rs:12557-12574`, assertion at `engine.rs:12969`.

**S2 — `capability -> checkpoint -> capability`** — doc comment `engine.rs:12403-12405`, body `engine.rs:12406-12438`. Capability `"noop"` (`engine.rs:12411`). **Correction: `noop` is a real registered builtin** (`builtins.rs:12`, id at `:31-33`), not a test double — it is safe and correct to use in fixtures.

**S3 — longer chains** — `engine.rs:12607-12645` (`two_checkpoint_forge_comment_sop`), `:12647-12679` (`checkpoint_mutates_before_forge_comment_sop`, tamper negative; mutator output at `engine.rs:12485-12490`), `:12681-…` (`same_step_revisit_forge_comment_sop`). `mutate.forge` (`engine.rs:12462`) **is** a test-only double — do not use it.

**S4 — `revisable-triage`** — `approval/broker.rs:991-1010` (the draft's `989` is two lines early). Base helper `checkpoint_policy_sop` at `broker.rs:693-724`: name `"checkpointed"`, `execution_mode: Deterministic` with `deterministic: false` (a real mismatch, `broker.rs:699` vs `:719`), triggers `[Manual]`, steps `[Checkpoint(policy) #1 title "checkpoint", Execute #2 title "go"]` — **lowercase titles**, not `"Checkpoint"`/`"go"` as the draft wrote. Stub capability at `broker.rs:975-987`, returning `{"body": "draft [feedback: …]"}` (`:986`), matching `syntax.md:241-250`.

**Blocking correction:** the `llm.generate` step at `broker.rs:994-1000` has **no `capability_input`**. Because it is built as a struct it never passes through `load_sop`. A markdown transcription of it **will fail to load** on `capability/registry.rs:56-65` + `:115-121`. Fixture #18 must add a `with: { instruction = "…" }` bullet, and that addition must be labelled as a reconstruction, not a transcription.

**S5 — approval config, executable form** — `crates/zeroclaw-gateway/src/api_sop.rs:314-333` (group `release` at `:316`, policy `prod` at `:325`, `required_group` at `:327`; the draft's `318-333` misses the group opener) and `crates/zeroclaw-runtime/src/tools/mod.rs:2034-2050` (group at `:2036`, member `"agent:ZeroClawOperator"` at `:2038`, policy at `:2043`, `quorum: 1` at `:2046`, both routes `None` at `:2047-2048`). Only route-populated policy: `crates/zeroclaw-channels/src/orchestrator/mod.rs:23940-23947` (`request_route: "discord.ops:room-1"` `:23943`, `escalation_route: "discord.ops:room-2"` `:23944`).

**S6 — switch-graph fixtures** — `crates/zeroclaw-runtime/src/sop/graph.rs:821-836`, three ports (valid / targetless / bad-target) on step 1 of a 2-step SOP. **Citation corrections:** the pinned wire-shape golden diagnostic `{"severity": "warning", "step": 1, "message": "switch port 'pr' has no target"}` is at **`graph.rs:1191`** (draft said 1193); the adjacency assertion `"1 -> 2 [switch:pr]"` is at **`graph.rs:1225`** (draft said 1226); the outline assertion `"1. First -> 2"` is at `graph.rs:1222` ✅. Runtime treats the same condition as fatal — `route/mod.rs:86-88`. **Addition:** `graph.rs:1219` builds the SOP with steps numbered `1, 2, 9` ("Ghost"), proving `Sop::steps` numbers need not be contiguous once outside the markdown path.

**S7 — exhaustive round-trip `SopStep`** — `crates/zeroclaw-runtime/src/sop/mod.rs:1227-1268` (draft said 1230-1272). Sets `body`, `suggested_tools=["read_file","shell"]`, `requires_confirmation=true`, `kind=Checkpoint`, `schema.input = {"type":"object","required":["ticket"]}`, **`schema.output = {"type":"boolean"}`** (the draft left this unstated; it is the only upstream example of a non-object schema), `scope{allow:["fs"], deny:["shell"]}`, routing `{when, next:2, terminal:false, depends_on:[2], switch:[pr>$.event>2, catch_all>>]}`, `on_failure=Retry{max:2}`, `mode=Some(Auto)`, plus step 2 with `routing.terminal = true` (`mod.rs:1262-1263`). **This is the only upstream `terminal: true` example** and `terminal` appears nowhere in the docs (verified: the only `terminal` hit under `docs/book/src/sop/` is the unrelated word at `example.md:59`). It is also the test that proves lossless round trip for exactly the fields it sets — and therefore the proof of §0.5's omissions.

**S8 — planned tool calls (`call:`)** — `mod.rs:1503-1518`; the two calls at `:1506-1513`. Rendered as `- call: <json of PlannedToolCall>` (`mod.rs:920-924`), parsed at `mod.rs:609-612` — **malformed `call:` JSON is silently dropped**. `PlannedToolCall` shape at `types.rs:287-297` (`tool`, `args` default, `pinned` optional). Binding validation messages: `"does not run before step 1"` (`:1529`), `"unknown step 9"` (`:1545`), `"does not run before call 1"` (`:1552`), `"malformed binding"` (`:1565`), warning `"no output schema or planned calls"` (`:1599`); the rules themselves at `mod.rs:1016-1068`.

**S9 — positions** — `mod.rs:1305-1330`, values `x: 320.5, y: -48.0`. `StepPosition { step, x, y }` at `types.rs:597-603`; merged onto `SopStep::pos` at load (`mod.rs:437-441`, matched **by step number**, so a `[[positions]]` entry naming a nonexistent step is silently ignored); written back from `pos` at `types.rs:652-662`. Aligns with Egeria ADR-0003: round-trip, exclude from the semantic hash.

**S10 (NEW) — path-traversal negatives** — `mod.rs:1327-1369` exercises the hostile-name set `["../escape", "..", ".", "/etc/shadow", "a/b", "a\\b", "../../etc/cron.d/evil", ""]` against load/delete/save/create. Rule at `mod.rs:203-218` (`resolve_sop_dir`): exactly one `Component::Normal`, and no `/`, `\`, or NUL. Worth a fixture if Egeria's importer takes SOP names from user input.

---

### 5. Real names

#### 5.1 Capabilities — **ten**, not eight (draft corrected)

`SopCapabilityRegistry::with_builtins()` (`capability/registry.rs:18-22` → `capability/builtins.rs:11-26`) registers all ten:

| id | Citation | Notes |
|---|---|---|
| `noop` | `builtins.rs:12`, id `:31-33` | returns input unchanged; **real, usable in fixtures** |
| `wait` | `builtins.rs:13`, id `:46-48` | **missing from the draft entirely**; input `{milliseconds?, seconds?}`, output `{waited_ms}` (`builtins.rs:60-73`), capped at 60 000 ms (`builtins.rs:9`, `:87-89`) |
| `approval.wait` | `builtins.rs:14`, id `:100-102` | fail-closed placeholder |
| `json.validate` | `builtins.rs:15`, id `:127-129` | |
| `shell.exec` | `builtins.rs:17`, id `:169-171` | fail-closed; input requires `command` |
| `git.status` | `builtins.rs:18`, id `:207-209` | fail-closed |
| `git.diff` | `builtins.rs:19`, id `:225-227` | fail-closed |
| `notify.channel` | `builtins.rs:20`, id `:243-245` | fail-closed |
| `forge.comment` | `builtins.rs:24`, id `forge_comment.rs:180-182` | registered as a fail-closed placeholder **in `with_builtins()`** |
| `llm.generate` | `builtins.rs:25`, id `llm_generate.rs:56-58` | ditto; **requires authored `with:`** |

The re-assertion test the draft cited (`builtins.rs:301-315`) lists eight ids including `noop` and `wait` — it simply does not re-assert the two injected-adapter ids, which is what the draft mistook for absence. `mod.rs:152-154` is the daemon **re-registration** with real adapters ("a later `register` for the same id overwrites", `builtins.rs:21-23`).

Test-only doubles that must **not** appear in fixtures: `mutate.forge` (`engine.rs:12462`).

**Load-time capability validation (`mod.rs:479` → `capability/registry.rs:40-68`) enforces three things, not one:**
1. `kind: capability` with no `capability:` → error *"is kind=capability but has no capability id"* (`registry.rs:45-48`).
2. Unknown capability id → error *"references unknown capability '…'"* (`registry.rs:51-55`).
3. For a capability with `requires_authored_input()` (today: `llm.generate` only) → `with:` must exist, be an object, and validate against the capability's `input_schema` (`registry.rs:56-65`, `:115-129`).

A `capability:` bullet on a non-capability step is **not** validated (`registry.rs:41-43`).

> **OPEN QUESTION 1 (restated on a correct premise).** ADR-0005 forbids linking `zeroclaw-runtime`, so Egeria's parser cannot consult the live registry. Does Egeria (a) hard-code the ten ids above as a manifest and reject unknown ones at parse time, mirroring `load_sop`; (b) accept any id and surface an `EGR-*` finding; or (c) accept silently? The upstream behavior is unambiguous — an unknown id is a **load failure**, not a warning — so option (c) diverges. Whichever is chosen, the `llm.generate` authored-`with:`-object rule must be mirrored or explicitly declined. Escalate.

#### 5.2 Tool names

The draft's "103 real ones" is wrong on both the count and the framing. Enumerating `fn name(&self) -> &str` across `crates/zeroclaw-tools/src/` and `crates/zeroclaw-runtime/src/tools/` yields **104** distinct strings; the draft's list is missing `TodoWrite`. More importantly, the set is not "103 real ones": it includes test doubles and fakes (`always_fails`, `fake`, `mock`, `mcp_fake`, `echo_tool`, `counting`, `query_echo`, `noop`) and several tools are feature-gated, so not all are registered in any given build (`default_tools`, reached via `mod.rs:76-90`).

The 104:
```
TodoWrite always_fails ask_user backup browser browser_delegate browser_open calculator canvas
channel_room claude_code claude_code_runner cloud_ops cloud_patterns codex_cli composio
content_search counting cron_add cron_list cron_remove cron_run cron_runs cron_update
data_management deliver_file discord_search echo_tool email_read email_search
escalate_to_human fake file_download file_edit file_read file_upload file_upload_bundle
file_write gemini_cli git git_forge git_operations glob_search google_workspace
hardware_board_info hardware_memory_map hardware_memory_read http_request image_gen
image_info jira knowledge linkedin llm_task matrix mcp_fake mcp_prompts mcp_resources
memory_export memory_forget memory_purge memory_recall memory_store microsoft365 mock
model_routing_config noop notion opencode_cli poll project_intel proxy_config pushover
query_echo reaction read_skill report_template schedule screenshot security_ops
send_message_to_peer send_via sessions_current sessions_delete sessions_history
sessions_list sessions_reset sessions_send shell skill_manage skill_view skills_list
sop_advance sop_approve sop_execute sop_list sop_status sop_workshop text_browser
tool_search vi_verify weather web_fetch web_search_tool
```

Cross-check of names actually used in upstream SOP examples (draft's table verified and extended):

| Used in | Real tool? |
|---|---|
| `http_request` (`syntax.md:114,151`; `cookbook.md:13`; `example.md:116,119`; `mod.rs:1507`) | ✅ |
| `shell` (`syntax.md:117,146-148`; `cookbook.md:16`; `example.md:113-120`; `tools/mod.rs:2505`) | ✅ |
| `file_read` (`cookbook.md:42`; `example.md:113,116,118`) | ✅ |
| `file_write` (`example.md:114,116`) | ✅ |
| `memory_recall` (`cookbook.md:28`) | ✅ |
| `memory_store` (`cookbook.md:45`; `src/sop/mod.rs:259`) | ✅ |
| `pushover` (`cookbook.md:31`; `src/sop/mod.rs:266`) | ✅ |
| `git_operations` (`example.md:118`) | ✅ |
| `calculator` (`mod.rs:1509`) | ✅ |
| `fs` (`mod.rs:1421`) | ❌ not a tool — a **scope group alias** (`scope/groups.rs:1,15-16`), see §1.3 |
| `read_file` (`mod.rs:1394,1400,1407,1233`; `scope/groups.rs:6`) | ❌ not a registered tool; real name is `file_read`. Upstream's own fixture **and its own scope-group table** use the nonexistent name |
| `write_file`, `edit_file`, `web_search` (`scope/groups.rs:6-7`) | ❌ not registered; real names `file_write`, `file_edit`, `web_search_tool` |
| `gpio_read` / `gpio_write` (`src/sop/mod.rs:259,262,623`) | ❌ not tools — `zeroclaw-hardware` protocol opcodes |

**Recommendation (unchanged from the draft, now better grounded):** author Egeria fixtures with `file_read`/`file_write`; keep one fixture with a deliberately bogus tool name to pin that the parser does **not** validate tool names (`parse_csv_list`, `mod.rs:718-724`, accepts any non-empty token), and one with `allow-tools: fs` to pin group-alias pass-through.

#### 5.3 Other real identifiers

Draft list verified, with additions marked **NEW**:
- SOP names: `deploy-prod`, `stagex-update`, `test-sop`, `multi-trigger`, `det-sop`, `no-steps`, `default-mode`, `canary`, `authoring`, `checkpointed`, `revisable-triage`, `deploy`, **`amqp-sop`** (`dispatch.rs:1979`, `amqp.rs:912`), **`looping-deploy`** (`broker.rs:~727`), **`det-cp`** (`engine.rs:12396`)
- Approval group / policy names: group `release`; policies `prod`, `triage`, `oncall`
- Approval member forms: `http:<subject>`, `ws:<subject>`, `agent:<alias>`, bare `ZeroClawOperator`, `agent:release-bot`, `agent:ZeroClawOperator`, channel-qualified `channel:discord.ops:123456789012345678` (`syntax.md:91-97`, `:210-214`)
- Routes: `discord.ops:123456789012345678`, `discord.oncall:987654321098765432`, `discord.ops:room-1`, `discord.ops:room-2`; format rule `channel:recipient` at `syntax.md:196-198`
- Channel keys: `git.main`, `amqp.anitya`, `matrix.announce`, `discord.worker`, `discord.ops`; **`telegram` bare + `telegram/prod` aliased** (`types.rs:1100-1110`)
- Git topics (`fan-in/git.md:13`): `git.main:pull_request.opened`; event types `issue_comment.created`, `issues.opened`, `pull_request.opened`, `pull_request.closed`, `pull_request.merged`, `pull_request_review_comment.created`, `workflow_run.completed`, `workflow_run.failed`, `release.published` — **but see §2/D13: these are matched via `condition`, not a `topic` trigger field**
- Conditions (`syntax.md:373-385`, `:400-409`): `$.value > 85`, `$.value >= 85`, `$.temp < 25`, **`$.temp <= 25`**, `$.status == "critical"`, `$.status != "error"`, `$.count == 42`, `$.data.sensor.value > 85`, `$.readings.1 == 20`, `$.active == "true"`, **`$.nonexistent > 0` (the negative case)**, `$.repo == "octo/repo"` (`git.md:13`), `$.text == "deploy"` (`channel.md:13`), **`$.event_type == "pull_request.opened"`** (`types.rs:1227`), **`$.extension == "json"`** (`types.rs:1189`), **`$.value > 90`** (`src/sop/mod.rs:536`); direct-numeric `> 0`, `>= 5`, `< 100`, `== 42`, `!= 0`, `> 3.14`. Grammar constraints at `syntax.md:411-424`: single comparison only, operators matched longest-first `>= <= != == > <`, no `AND`/`OR`/`NOT`, no wildcards/filters/recursive descent, booleans compare as the quoted strings `"true"`/`"false"`, empty condition matches unconditionally, everything else fails closed
- Board id `nucleo-f401re-0`, signal `pin_3`; cron `0 */5 * * *`; webhook path `/sop/test`; MQTT topics `sensors/temp`, **`facility/pump/pressure`**; **filesystem paths `/var/inbox/**/*.json`, `/var/inbox`**; **calendar source `microsoft365`, ids `["primary","team"]`**; **AMQP routing keys `anitya.update`, `orders.new`**
- Audit keys: `sop_run_{run_id}`, `sop_step_{run_id}_{step_number}`, `sop_approval_{run_id}_{step_number}`, `sop_timeout_approve_{run_id}_{step_number}` (`observability.md:11-14`; the `<run-id>` variant at `example.md:147-151`)

---

### 6. Grammar constructs with no upstream example — corrected table

Counts re-verified by grep over `docs/book/src/sop/` and the crate tree.

| Construct | Docs example? | Rust example? | Notes |
|---|---|---|---|
| `- switch:` | **NONE** (0 bullets; the sole `switch` hit is prose at `fan-in/channel.md:5`) | Yes — **only** `mod.rs:1426` (bullet) + `mod.rs:1252-1261`, `graph.rs:821-836` (struct) | Confirms audit fact #1 |
| `- terminal:` | **NONE** (0 occurrences) | Struct only — `mod.rs:1263` | Parser `mod.rs:590-591`; renderer `mod.rs:881-883` |
| `- prompt:` (`gate_prompt`) | **NONE** | **NONE** | Parser `mod.rs:613-617`; field doc `types.rs:380-386`. Never rendered |
| `- edit:` | Prose only — `syntax.md:234` | **NONE** | Parser `mod.rs:625-633`; field doc `types.rs:387-391`. Never rendered |
| `- agent:` | **NONE** | Round-trip only — `mod.rs:1474-1484`, alias `"pr_bot"` | Parser `mod.rs:606-608`; resolution `types.rs:446-450`, test `mod.rs:1486-1493` |
| `- call:` | **NONE** | `mod.rs:1503-1518` | |
| `allow_tools:` / `deny_tools:` | **NONE** (docs show hyphens only) | **NONE** | `mod.rs:559`, `:564` |
| `depends-on:` / `on-failure:` | **NONE** (docs show underscores only) | **NONE** | `mod.rs:594`, `:601` |
| `kind: approval` | **NONE** | **NONE** | `mod.rs:755` |
| `kind: capability` (as a bullet) | Broken pseudo-example only (D6) | `mod.rs:1643` | `syntax.md:160-161` omits `capability` from the accepted values — **docs wrong, source wins** |
| `on_failure: goto:N` | `syntax.md:148` only | Struct — `mod.rs:1192`, `1223` | Space forms `goto N` / `retry N` also accepted (`mod.rs:781-790`); **no example of either space form** |
| `on_failure: fail` (explicit) | **NONE** | **NONE** | `mod.rs:777-779`; never rendered (`mod.rs:908`) |
| `mode:` values ≠ `auto` | **NONE** | `mod.rs:1428` (`auto`) only | Unknown values silently → `Supervised` (`mod.rs:170-179`) |
| `requires_confirmation: false` (explicit) | **NONE** | **NONE** | Parses to `false`; never rendered |
| `admission_policy` = `parallel`/`hold`/`coalesce` | `syntax.md:39-49` prose; `hold` in D1 | `drop` only (`mod.rs:1667`) | |
| Quorum ≥ 2 | `syntax.md:87`, `:189` | **NONE** — every Rust fixture uses `quorum: 1` | |
| `amqp` trigger in TOML | **NONE** (directive only) | **NONE** (struct only: `dispatch.rs:1979-1983`, `amqp.rs:912-915`) | |
| `filesystem` / `calendar` / `channel` triggers in TOML | **NONE** (directives only) | **YES — draft was wrong**: `types.rs:1183-1205`, `:1207-1220`, `:1148-1160`, `:1222-1236` | |
| `[sop] agent` | **NONE** | **NONE** | `types.rs:629-632`; consumed `mod.rs:453,477` |
| `[[steps]]` in `SOP.toml` | **NONE** | **NONE** (only reachable via `save_sop`) | §0.3 — real and load-bearing |
| `[[positions]]` | **NONE** | `mod.rs:1305-1330` | |
| `llm.generate` as a markdown step | Broken pseudo-example only (D6) | Struct only — `broker.rs:994-1000`, `llm_generate.rs:284-287`, `:313-317` | And **all** struct instances omit `with:`, which would fail `load_sop` |
| `forge.comment` as a markdown step | Broken pseudo-example only (D6) | Struct only — `engine.rs:12541-12555` | |
| Non-object schema (`{"type":"boolean"}`) | **NONE** | `mod.rs:1236` | |
| Nested/indented sub-bullets | **NONE** | **NONE** | Indentation carries no meaning (§0.7) |

---

### 7. OPEN QUESTIONS — genuinely undecidable from source

1. **Capability-id validation in Egeria.** See §5.1. Upstream rejects unknown ids at load; ADR-0005 says Egeria parses the documented grammar without linking the runtime. Escalate.
2. **Silent degradation of `with:` / `input:` / `output:`.** `parse_value_fragment` (`mod.rs:761-773`) turns an unparseable fragment into `Value::String`. `- output: {broken` yields the string `"{broken"` as the output *schema*. Whether the schema compiler (`crates/zeroclaw-runtime/src/sop/schema/compile.rs`) then rejects it is outside this area. **Recommend a diagnostic in Egeria plus a fixture either way, and record the divergence.**
3. **Duplicate bullets on one step.** Every scalar is last-write-wins (`mod.rs:555-633`), and `ensure_scope(...).allow = Some(...)` (`mod.rs:561`) overwrites rather than appends. Two `- tools:` bullets → the second replaces the first. `- switch:` likewise replaces the entire rule list; only `- call:` accumulates (`mod.rs:611`). **No upstream test covers any of this.** Undecidable whether it is intended.
4. **`switch:` `when` containing `>`.** `splitn(3,'>')` truncates (§1.2). No test, no doc. **Flag; do not "fix" it in Egeria without an explicit decision** — silently accepting `$.value > 85` in a port would make Egeria's parse diverge from ZeroClaw's runtime behavior.
5. **`render_steps` losing `capability`/`with`/`policy`/`prompt`/`edit`/`kind: capability`** (§0.5) — and the resulting `save → load` data loss. Egeria's printer must decide; **recommend emitting all 21 bullets and documenting the divergence explicitly**, since a printer that matches upstream's omissions is a printer that destroys data.
6. **`### `/`# ` headings and blank-line handling inside a step.** Derivable (§0.7: `###` and `#` become body text; a `## ` heading terminates) but **untested upstream**. Blank lines are skipped by the `!trimmed.is_empty()` guard at `mod.rs:645`, so a step's body silently concatenates across blank lines. Worth fixtures; behavior is derivable, so this is a low-risk escalation.
7. **NEW — mid-line bold.** `extract_bold_title` (`mod.rs:820-824`) finds the first `**` anywhere, so `1. Do **the** thing` yields `title="the"`, `body="thing"`. No upstream fixture. Undecidable whether this is intended; Egeria must pick and pin it.
8. **NEW — an empty-but-present `SOP.md` beside a populated `[[steps]]`.** `mod.rs:428` branches on `md_path.exists()`, so the empty file wins and the SOP loads with zero steps plus the `"SOP has no steps"` warning (`mod.rs:989-990`). No test covers it. Confirm Egeria mirrors file-existence (not file-emptiness) as the discriminator.
9. **NEW — `- capability:` on a non-capability step.** Parsed and retained (`mod.rs:575-576`) but never validated (`registry.rs:41-43`) and never rendered (`mod.rs:847-927`). Does Egeria keep it, drop it, or emit a finding?

---

### 8. Recommended fixture corpus (28, revised)

| # | Fixture | Grounded in | Status |
|---|---|---|---|
| 1 | `deploy-prod` (manifest + 2 steps, schemas, policy, dangling `next: 3`) | D1 + D4 | verbatim |
| 2 | `alert-remediation` (5-step combined routing) | D5 | verbatim |
| 3 | `hitl-deploy` (cookbook 1) | D8 | verbatim md, synthesized toml |
| 4 | `iot-alert` (cookbook 2, MQTT trigger) | D9 + R5/R18 mqtt | verbatim md + verbatim trigger |
| 5 | `daily-digest` (cookbook 3, cron trigger) | D10 + R5 cron | verbatim md + verbatim trigger |
| 6 | `test-sop` (manifest + md, 2 triggers, priority/mode/cooldown) | R4 | verbatim |
| 7 | `multi-trigger` (5 trigger types) | R5 | verbatim |
| 8 | `all-triggers` (adds filesystem, calendar, channel, amqp) | R18 + `types.rs:142-229` | 3 verbatim TOML + amqp constructed |
| 9 | `det-sop` (deterministic overrides mode; checkpoint) | R6 | verbatim |
| 10 | `det-conflict` (`deterministic = true` **and** `execution_mode = "auto"`) | `mod.rs:456-461` | **constructed** |
| 11 | `sensor-valve` (3 steps, `## Conditions` preamble, trailing `## Notes`) | R1 + `mod.rs:519-524` | verbatim + extension |
| 12 | `full-contract` (every bullet incl. `switch:`) | R11 | verbatim |
| 13 | `all-bullets` (the 16 keys R11 omits: `capability`, `with`, `policy`, `prompt`, `edit`, `agent`, `call`, `terminal`, `kind: approval`, `tools`, `requires_confirmation`, and all five aliases) | §1.1 | **constructed** |
| 14 | `capability-git-status` (`kind`/`capability`/`with` TOML-inline **and** JSON forms) | R13 | verbatim + variant |
| 15 | `policy-gate` (policy present + absent) | R12 | verbatim |
| 16 | `admission-drop` + variants for `parallel`/`hold`/`coalesce` | R14 + `types.rs:530-548` | one verbatim, three constructed |
| 17 | `canary` (no separator after bold title) | R15 | verbatim |
| 18 | `no-steps` (toml only, no SOP.md) | R8 | verbatim |
| 19 | `empty-md` (present-but-empty SOP.md + populated `[[steps]]`) | §0.3, `mod.rs:428` | **constructed** — pins existence-not-emptiness |
| 20 | `default-mode` (mode falls back to caller default) | R9 | verbatim |
| 21 | `minimal-manifest` (only `name`+`description`; defaults assert) | R19 (`types.rs:1357-1376`) + `procedural_memory.rs:309-315` | verbatim |
| 22 | `capability-forge-comment` (draft → checkpoint → post) | **corrected** D6 + S1 field values | reconstructed — see §2/D6 |
| 23 | `revisable-triage` (`llm.generate` → policied checkpoint) | S4 — **plus a mandatory `with: { instruction = … }`** | reconstructed, not transcribed (§4/S4) |
| 24 | `stagex-update` (8-step deterministic) | **reconstructed** from D12 table | reconstructed |
| 25 | `terminal-guard` (false `when:` + `terminal: true`, and false `when:` with `next:` pointing **away** from the linear successor) | S7 + `route/mod.rs:70-75` | **constructed** — the only shape where `syntax.md:165-167` and `:170-171` diverge observably |
| 26 | `switch-ports` (valid / targetless / bad-target; plus a `when` containing `>`) | S6 + §1.2 | transcribed + constructed |
| 27 | `planned-calls` (`- call:` with `{{steps.N}}`/`{{calls.K}}` bindings, plus a malformed `call:` that must be dropped) | S8 | transcribed + constructed |
| 28 | `positions` (`[[positions]]` + steps; plus a `[[positions]]` entry for a nonexistent step) | S9 + `mod.rs:437-441` | verbatim + constructed |
| 29 | `toml-steps-fallback` (`[[steps]]`, no SOP.md, exercising `with`/`routing`/`on_failure` TOML shapes and `number = 0` renumbering) | §0.3 | **constructed** |
| 30 | `scrambled-numbers` (`3.`, `1.`, `9.`, `07.`, plus `1.**NoSpace**` as a non-item) | §0.4 | **constructed** |
| 31 | `degradation` (unknown `kind:`, unknown `mode:`, unparseable `on_failure:`, `- output: {broken`, bogus tool name, `allow-tools: fs`) | §1.2, §1.3 | **constructed** — pins every silent-degradation path |
| 32 | `hostile-names` (path-traversal SOP directory names) | S10 (`mod.rs:203-218`, `:1327-1369`) | verbatim value set |

Fixtures 8 (amqp part), 10, 13, 16 (three of four), 19, 22, 23, 24, 25, 26 (partial), 27 (partial), 28 (partial), 29, 30, 31 have **no copyable upstream source** and must be constructed against this spec. Everything else is verbatim or a faithful transcription of a struct literal — and each transcription must be labelled as such in `fixtures/sops/INDEX.md`.
