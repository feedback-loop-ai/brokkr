# Implementation Plan: The agent library

**Feature slug**: `agent-library` · **Spec**: [spec.md](spec.md) ·
**Tasks**: [tasks.md](tasks.md)

Everything below is grounded in files read in this worktree at
`c857137`. Line references are to that commit.

## Module layout

```
agents/                              NEW  library, repo root
  <name>.json                        16 definitions
  charters/<file>.md                 14 charters, git mv'd out of recipes
adapters/                            NEW  provider data, repo root
  {claude,lanetally,codex,dsh,exec}.json
contracts/
  run-manifest.v3.schema.json        NEW  v1 + optional `agents`
  effect-provenance.v1.schema.json   NEW  the additive payload fields
  README.md                          EDIT the amended prose rule + reason
crates/brokkr-runtime/src/
  agents.rs                          NEW  the pure resolver
  agents/tests.rs                    NEW
  bundle.rs                          EDIT parse `agent:`, call resolve, manifest key
  engine.rs                          EDIT provenance, candidate selection
crates/brokkr-core/src/dispatch.rs    EDIT refuse `agents` on the v2 lineage
crates/brokkr-protocol/src/lib.rs     EDIT AttemptReport gains `accepted: bool`
crates/brokkr-protocol/src/process.rs EDIT set it (already tracked in a local)
crates/brokkr-view/src/lib.rs         EDIT Provenance on Participant, VIEW_VERSION 2
crates/brokkr-cli/src/
  agents.rs, agents/tests.rs         NEW  list / show
  main.rs, doctor.rs, render.rs,
  tui.rs, ui.rs + ui.html, compare.rs EDIT
bundles/self, recipes/panel-review,
recipes/sdd                          EDIT seats reference agents
README.md, ARCHITECTURE.md           EDIT
```

**Untouched**: `crates/brokkr-protocol/src/adapters.rs`. Its
`AdapterKind` match arms are stream-format parsers — they fold
`claude`'s stream-json, `codex`'s `--json`, `dsh`'s profile output —
not the model/flag arms decision 0016 forbids, and everything after
`--` is already passed through verbatim (`command.extend(extra…)`). So
compile-time argv composition is sufficient and the forbidden match arm
is one we simply never write. Also untouched: `brokkr-store`,
`fixtures/`, `policy/`, `reference/`, every frozen contract's bytes.

## The resolver

`crates/brokkr-runtime/src/agents.rs`, one pure function and its data:

```rust
pub enum Presence { Available, Unavailable, Unknown }

/// Provider -> presence. `unspecified()` is what compile passes:
/// no facts, so no availability filtering and no availability notice.
pub struct Availability(BTreeMap<String, Presence>);

pub struct Candidate {
    pub model: String,        // abstract name
    pub provider: String,
    pub argv: Vec<String>,    // composed, unexpanded ({brokkr} intact)
}

pub struct Resolution {
    pub agent: String,
    pub charter: PathBuf,
    pub limits: Limits,
    pub inputs: Option<Vec<String>>,
    pub candidates: Vec<Candidate>,   // ordered; [0] is the choice
    pub record: Value,                // the manifest record (spec Q1)
    pub notices: Vec<Notice>,
}

pub fn resolve(
    library: &Library,
    adapters: &Adapters,
    availability: &Availability,
    name: &str,
) -> Result<Resolution, ResolveError>
```

`Library` and `Adapters` are loaded once per compile from their roots
(`Library::load(&Path)`, `Adapters::load(&Path)`), each returning a
parse error naming the file and key. `resolve` itself takes no paths and
performs no I/O — that is what makes AC-1's "nothing spawned" a property
of the type rather than of review discipline.

`argv` is composed **unexpanded**: `{brokkr}` stays a literal token, so
`parse_command`'s existing expansion, its `./`-relative handling and its
`scan_secret_refs` lint all run over the composed argv exactly as over
inline argv. That reuse is AC-11 for free and keeps machine-local
absolute paths out of everything digested.

`ResolveError` is a closed enum; each variant carries the names its
message must print (agent, provider, capability, item, files consulted).
The coverage gate means each variant lands with its test.

`Bundle::compile(dir)` keeps its exact signature and gains a sibling
`Bundle::compile_with(dir, library_root, adapters_root)`; `compile`
delegates with the defaults `agents` and `adapters` (CWD-relative, the
same convention as `--recipes-dir`). **The library is read only when a
seat says `agent:`** — a bundle with no agent reference never touches
those directories, which is what makes a missing library a non-event for
every existing recipe and test. Fixture directories are how AC-9 adds a
provider without a Rust edit.

## Data shapes

### `agents/<name>.json`

```json
{
  "description": "Synthesises panel positions into the committed spec.",
  "charter": "charters/chief-architect.md",
  "models": ["fable", "opus", "sonnet"],
  "tools": {
    "allow": ["cargo", "git", "python3", "pytest", "ls", "rg", "mkdir", "specify"],
    "mcp": []
  },
  "limits": { "max_attempts": 2, "timeout_seconds": 3600 }
}
```

Unknown keys are rejected. `models` is ordered and non-empty.
`tools.allow` is ordered — the order is the `--allowedTools` order, so
AC-5's element-for-element argv equality holds. `tools.allow` **absent**
declares no tool restriction; `[]` is rejected as ambiguous.
`tools.mcp` entries are `{"server": "<name>", "optional": true?}` —
`optional` exists only here, which is what makes "optional on a
restriction" unrepresentable rather than merely forbidden. `inputs` is
permitted (the ruling names it) and no shipped agent uses it: 0007's
existing default — a seat's inputs are its phase's rule-referenced
inputs — already does the right thing for all sixteen. `inputs` on both
the agent and the seat is a compile error.

### `adapters/<provider>.json`

```json
{
  "provider": "claude",
  "binary": "claude",
  "driver": ["{brokkr}", "driver", "claude", "--",
             "--permission-mode", "acceptEdits"],
  "models": { "fable": "claude-fable-5", "opus": "claude-opus-5" },
  "model_flag": "--model",
  "tool_permissions": {
    "flag": "--allowedTools", "separator": ",",
    "names": { "cargo": "Bash(cargo:*)", "git": "Bash(git:*)",
               "specify": "Bash(specify:*)" }
  },
  "mcp": { "flag": "--mcp-config", "servers": {} }
}
```

Any of `model_flag`, `tool_permissions`, `mcp` may instead be the string
`"unsupported"` — **the declaration of absence is explicit, never
inferred from an empty map**, because an empty map is ambiguous between
"cannot" and "not filled in yet". `adapters/exec.json` is the honest
degenerate case: `"model_flag": "unsupported"`, `"tool_permissions":
"unsupported"`, `"mcp": "unsupported"`, `"models": {}`.

**The implementer fills each provider's driver prefix and flags from
`crates/brokkr-protocol/src/adapters.rs` and the provider CLI's
documented flags, and writes `"unsupported"` wherever the truth is not
established.** Guessing a flag here is the quiet substitution this
decision exists to refuse; `"unsupported"` is always the safe answer,
because it fails loudly at compile rather than silently at run time.

Composition is a lookup and a join — no template language. An abstract
tool name with no entry on the resolved provider *is* the capability
gap; the same machinery covers it with no extra concept, and there is no
substitution function whose branches the coverage gate would have to
cover.

### The agent roster (16 definitions, 14 charters)

Digests are the current `roles/*.md` bytes; `BASE` is
`cargo,git,python3,pytest,ls,rg,mkdir` and `SPECIFY` is `BASE +
specify`, matching the argv in the tree today.

| agent | charter (git mv from) | tools | adopted by |
|---|---|---|---|
| `intake` | self `roles/intake.md` `d27fd198` | BASE | self, panel-review |
| `intake-speckit` | sdd `roles/intake.md` `af614654` | SPECIFY | sdd |
| `implementer` | self `roles/implementer.md` `3c0e869e` | BASE | self, panel-review |
| `implementer-speckit` | sdd `roles/implementer.md` `3720b487` | SPECIFY | sdd |
| `verifier` | self `roles/verifier.md` `b2c93f74` | BASE | self, panel-review |
| `verifier-speckit` | *same file* | SPECIFY | sdd |
| `shipper` | self `roles/shipper.md` `df94781f` | BASE | self, panel-review |
| `shipper-speckit` | *same file* | SPECIFY | sdd |
| `reviewer` | self `roles/reviewer.md` `6015367d` | BASE | self |
| `review-correctness` | panel-review `ce423d91` | BASE | panel-review |
| `review-security` | panel-review `555a5937` | BASE | panel-review |
| `review-security-speckit` | sdd `de00a51d` | SPECIFY | sdd |
| `review-spec-compliance` | sdd `416f9e17` | SPECIFY | sdd |
| `chief-architect` | sdd `design-chief.md` `757657c8` | SPECIFY | sdd |
| `position-simplicity` | sdd `design-simplicity.md` `d00dfc71` | SPECIFY | sdd |
| `position-robustness` | sdd `design-robustness.md` `f96e1467` | SPECIFY | sdd |

`verifier`/`verifier-speckit` and `shipper`/`shipper-speckit` point at
the **same charter file**: identical bytes, differing only in tools. Two
agents may share a charter; nothing is copied.

`recipes/sdd`'s `design > speckit-check` step stays inline. `recipes/fast`
and `bundles/verify` adopt nothing.

## Engine changes

1. **`AttemptReport` gains `accepted: bool`.** `run_attempt` already
   tracks it in a local (`process.rs:221`); the field only surfaces what
   the process layer knows. That plus `checkpoints.is_empty()` and the
   `Failed` outcome *is* the fail-to-start predicate — structural, with
   no stderr sniffing. `DriverRun::SpawnFailed` satisfies it trivially.
   The predicate is also the mid-session boundary mechanised: once
   `Accepted` arrives, fallback is structurally unreachable, so "do not
   widen this" is enforced by construction rather than by a comment.
2. **Candidate selection per invocation site.** `SeatBody::Single` (and
   `StepBody::Single`, and `PanelMember`) gain
   `candidates: Vec<Candidate>`, **empty for inline seats** — so the
   existing execute path is unchanged when it is empty and inline
   behaviour cannot move. Index for site *s* = the number of prior
   attempts for this effect whose event for site *s* carried
   `start_failure: true`, clamped to `candidates.len() - 1`. Derived by
   scanning the effect's events, the way `engine.rs` already scans them
   for the last error — so `fold` and `RunState` are untouched (AC-13)
   and a restart cannot change the model (AC-15).
3. **`effect/started` payload gains `provenance`**, a list over
   invocation sites, absent when no site is agent-resolved. The existing
   `driver` label is unchanged, so non-adopting journals stay
   byte-identical.
4. **`effect/failed` gains `start_failure` and the member tag** when the
   predicate holds.
5. `manifest_diff` learns to name an `agents` difference instead of
   falling through to "non-file manifest fields differ".

## Surfaces (decision 0013)

`brokkr-view` gains `Provenance { agent, model, provider, chain_index,
fallback: bool }` on `Participant` — whose existing
`member: Option<String>` is already the per-invocation key — derived
once at the `EventType::EffectStarted` arm. Run-level notices are read
from `run/started.payload.manifest.agents[*].notices`: the run manifest
is **already** journaled in that payload, so compile-time notices reach
every surface with no new event field at all. `VIEW_VERSION` → 2.

`render.rs`, `tui.rs`, `ui.html` and `compare.rs` render that one model;
none of them formats provenance itself. `compare.rs` today has its own
aggregation and does not go through `brokkr-view`; it gains
`resolution_divergence` by **calling** the `brokkr-view` derivation for
each run rather than by re-deriving — the single derivation without a
refactor of `seat_costs`.

## Risks and mitigations

| # | Risk | Mitigation |
|---|---|---|
| 1 | Adopting recipes' digests move when charters leave their dirs, and a charter edit stops being pinned | The `agents` manifest key carries `charter_digest` — the pin that replaces the lost `manifest.files` entry. AC-4 pins the *non*-adopters; a second golden asserts an adopting bundle's digest moves when its charter's bytes change |
| 2 | The v2 (Looper) lineage silently drops `agents` and breaks resume | `build_run_manifest_v2` refuses adopting bundles with a named error (AC-19). Found by reading `dispatch.rs:422`, not by testing it in anger |
| 3 | The amended `contracts/README.md` rule becomes a licence to add fields freely | The amendment is narrow and testable — optional, absent by default, published as a numbered extension schema, and never read by `fold` — and AC-13 machine-checks the last clause |
| 4 | New payload fields are silently dropped by `brokkr-bridge`'s allowlist | Ruled deliberately: they are dropped, asserted by a test naming the ruling (AC-19) |
| 5 | A packed provenance string forces six consumers to parse a grammar and gets truncated at the bridge's 256 chars | Structured fields; nothing parses a label |
| 6 | Compile-time probing makes digests machine-dependent | `Bundle::compile` passes `Availability::unspecified()`; purity is a property of `resolve`'s signature |
| 7 | Resolved argv containing an expanded `{brokkr}` reaches a digest | The manifest record carries names and digests only; argv is composed unexpanded |
| 8 | An agent falls back onto a provider that cannot express its tool restrictions, silently widening its power | Capability checks run over **every** chain entry; `optional` is unrepresentable on a restriction |
| 9 | A capability matched per class lets an agent run without its named MCP server and report a content failure | Matching is per named item |
| 10 | `agent: "../../etc/passwd"`, or two agents differing only by case | Canonicalised, containment-checked paths; `^[a-z][a-z0-9-]*$`; case-insensitive uniqueness (AC-20) |
| 11 | A `secrets.env` in the new trees becomes an offline-guessing oracle | `manifest_for`'s refusal extended to `agents/` and `adapters/` (AC-11) |
| 12 | The 100% line/branch/function gate makes every error variant and every rendering branch a test | Deliberately small surfaces: no template language, a closed two-name capability vocabulary, one optional view field rather than twelve, canonical JSON for `agents show` instead of a formatter |
| 13 | Adopting recipes change which model they run | Stated in the spec as intended: an explicit pinned model replaces an invisible provider default. Non-adopters unchanged |
| 14 | The self-forge loop runs `bundles/self`, which this slice edits | Land the library and resolver before adoption (tasks are ordered that way); adoption is the last movement and is one commit that can be reverted alone |
