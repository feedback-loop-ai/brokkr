# Design — Astra as the boxed engine smith (issue #307)

## Context

The operator ruled the engine smith's chain `astra` then `fable`, both at
`high`. Editing only the roster fails today, and the failure is correct:
`agents/implementer-engine.json` carries `tools.allow: ["cargo", "git"]` and
no hands, `adapters/codex.json` declares `tool_permissions` unsupported, and
`agents.rs::compose` refuses the Codex link with the "MORE power" capability
error. The 2026-09-21 rulings on the issue answer both triage escalations:
the restriction this seat owes is decision 0043's existing workspace
confinement, not a command allow-list, and decision 0046's namespace and
harness launches stay two separate paths.

This design was written against the tree at `5ef97115`. The facts it stands
on, each read from the code rather than from the proposal:

1. **Resolution** — `crates/brokkr-runtime/src/agents.rs::compose`
   (lines 714–899). After the model and effort pins it branches once:
   `if agent.hands.is_some() { if boxed { …append adapter.hands… } }
   else if let Some(allow) = &agent.allow { …map or refuse… }`.
   With hands, `tools.allow` is never read — under *any* boundary. Boxed and
   without `adapter.hands` it refuses with `the provider declares hands
   unsupported[ (<reason>)], so the agent's hands cannot be put in the box and
   the agent would run with the harness's own tools`. Without hands and
   without `adapter.tool_permissions` it refuses with `the provider declares
   tool_permissions unsupported[ (<reason>)], so the agent's restriction to
   {allow:?} cannot be expressed and the agent would run with MORE power than
   it declares`. Both ride `ResolveError::Capability`, whose frame names the
   agent, provider and model and ends `A capability the provider cannot
   express fails compilation here rather than degrading silently at run time`.
2. **Whole chain** — `agents.rs::resolve_report` returns the first
   `entry.gap` of *every* entry, not of the chosen one, so an expressible
   later link never rescues a refused earlier one (or the reverse).
   `report_under` passes `boundary.is_boxed()` as `boxed`.
3. **Admission** — `crates/brokkr-runtime/src/bundle.rs::resolve_reference`
   (lines 2053–2165) calls `report_under`, then — for a hands agent whose
   chain is fully mapped — `enforce_model_policy`, whose first statement is
   `enforce_hands_boundary` (lines 2610–2700), and only then
   `resolve_report`. `enforce_hands_boundary` returns at once under a boxed
   boundary; under `harness` it walks every candidate and refuses a work link
   whose adapter has no `hands.harness.work` with `seat '{what}' link {link}
   resolves to provider '{provider}', which declares no `hands.harness.work`
   fragment{measured}: a capability gap — under the `harness` boundary a work
   seat with hands writes the tree only under the harness's own writable
   sandbox as the adapter addresses it (decision 0046 rulings 1 and 4)`.
   `resolve_reference` prefixes resolver errors as `seat '{what}': {e}`.
   `record_hands` writes the agent's `HandsSpec` into the site table, which
   becomes `Bundle::hands` and the manifest's `hands`/`boundary` maps.
4. **Launch** — `crates/brokkr-runtime/src/engine.rs::compose_site`
   (lines 4270–4318) is the pure function `Engine::compose` calls. Under
   `BuiltBoundary::Namespace` a model site is
   `hands_command(command, Some(spec), workdir, roots)`; under `Harness` it is
   the unboxed argv plus `candidate.harness.work` (work) or `.gate` (gate)
   with `{result_path}` and `{brokkr}` expanded; under `Open`, the command
   alone. `hands_command` (lines 4509–4579) substring-replaces
   `{hands_mcp_json}`, `{hands_args_toml}` and `{brokkr}` in every token.
   The TOML it writes is `[` + each `serve_args` element quoted with only `\`
   and `"` escaped + `]`.
5. **Server arguments** — `crates/brokkr-protocol/src/hands.rs::serve_args`
   is `["hands","serve","--workdir",<workdir>,"--spec",<spec.to_value()>]`;
   `mcp_config` wraps it as `{"mcpServers":{"brokkr":{"command","args"}}}`;
   `HandsSpec::to_value` is `{"kind":"workspace","network",
   "binds":[{"path","mode","mask"}]}`.
6. **Adapters** — `adapters/codex.json` `hands.workspace` is `--sandbox
   read-only` plus three `-c mcp_servers.brokkr.*` pairs (command, args,
   `default_tools_approval_mode="approve"`); `hands.harness.work` is
   `--sandbox workspace-write`. `adapters/claude.json` `hands.workspace` is
   `--tools "" --strict-mcp-config --mcp-config {hands_mcp_json}
   --allowedTools mcp__brokkr__workspace`; it declares **no**
   `hands.harness`. The adapter loader refuses a workspace token inside a
   harness fragment (`load.rs::harness_hands`).
7. **Precedent** — `chief-architect` (`fable → astra → opus`, hands, no
   tools) is seated work-class in `recipes/triage`'s design sequence, so a
   boxed Codex *work* link already compiles under namespace in a pinned,
   shipped bundle. `reviewer` and `release-manager` already declare exactly
   the `~/.cargo` overlay (masks `credentials.toml`, `credentials`) and
   `~/.rustup` read-only binds this seat adopts.
8. **Who hires the smith** — only `recipes/triage` (`implement` select, case
   `engine`). `recipes/night-shift` overrides the whole `implement` seat with
   an inline dsh driver; `recipes/gpt-flash` overrides it with
   `gpt-flash-implementer-engine`. `bundles/self`, `recipes/panel-review` and
   the rest never name it.

## Goals / Non-Goals

**Goals**

- Seat the ruled hire as data, under the confinement 0043 already builds.
- Turn the five admission outcomes and the two shipped namespace launches
  into reason-bearing, independently-expected regressions.
- Give every claimed protection a recorded removal proof.
- Re-pin exactly the identities that move, measured, with history.
- Say in the guide and in a dated 0043 note what the confinement is and is
  not.

**Non-Goals**

- No per-tool enforcement contract, command parsing, transport semantics or
  native-tool bypass prevention (operator ruling; proposal D1).
- No combination of `hands.workspace` with `hands.harness.work`; no guessed
  Claude `hands.harness` fragment (proposal D3).
- No edit to `adapters/*.json`, `contracts/`, `policy/`, `fixtures/`,
  `reference/`, the protocol crate, or any dependency.
- No live Astra smith, no network, no workflow runner, no Windows
  obligation (decision 0063).
- No new semantic decision document: this applies accepted 0043 and 0046.

## Decisions

### DD1 — No production Rust changes; the tests are the deliverable

Facts 1–4 already satisfy every admission and composition requirement in
both specs. The implementation therefore edits **no** line of:

| Must not change | Why it is already right |
| --- | --- |
| `agents.rs::compose`, `capability_gap`, `entry_for`, `report_under`, `resolve_report`, `ResolveError::Capability`'s text | hands precede the list; both refusals exist word for word; every link is judged |
| `bundle.rs::resolve_reference`, `enforce_model_policy`, `enforce_hands_boundary`, `measured`, `record_hands` | harness work is judged per link before capability gaps; boxed boundaries return early |
| `engine.rs::compose_site`, `hands_command`, `Engine::compose` | the two launches are already mutually exclusive arms of one `match` |
| `hands.rs::HandsSpec::{parse,to_value}`, `serve_args`, `mcp_config` | the spec and workdir already travel whole |
| `agents/load.rs` (agent and adapter loaders) | `hands` and `harness` shapes already load and refuse as specified |
| `adapters/codex.json`, `adapters/claude.json` | both fragments are the ones the spec preserves; a byte edit to either would move every witness that consults it, for no reason this change can name |

If a new test fails against the unmodified tree, that is a finding, not
licence to improvise: the implementer stops, records the failing assertion,
and makes the smallest repair consistent with 0043/0046 that leaves both
refusal texts byte-identical. Any such repair owes its own tests, literal
100% exact coverage of every added line and its own removal proof, and is
named in the delivery notes. This design predicts the branch is not taken.

*Rejected — a defensive refactor of `compose` (e.g. extracting a
`restriction()` helper so precedence is "more testable").* It would add
production lines to cover, risk the refusal bytes, and prove nothing the
argv assertions below do not already prove from outside.

*Rejected — teaching the Codex adapter a synthetic `tool_permissions`.* The
provider has no such flag (measured in the adapter's own gap text); it would
turn an honest refusal into a silent widening and contradict the ruling that
no-hands stays refused word for word.

### DD2 — The shipped declaration, byte for byte

`agents/implementer-engine.json` becomes (key order follows
`agents/reviewer.json`, two-space indent, trailing newline):

```json
{
  "description": "Engine-class implementer: builds core, store, contract, and policy work selected by triage.",
  "charter": "charters/implementer.md",
  "models": [
    "astra",
    "fable"
  ],
  "efforts": {
    "astra": "high",
    "fable": "high"
  },
  "hands": {
    "kind": "workspace",
    "network": false,
    "binds": [
      {
        "path": "~/.cargo",
        "mode": "overlay",
        "mask": [
          "credentials.toml",
          "credentials"
        ]
      },
      {
        "path": "~/.rustup",
        "mode": "ro"
      }
    ]
  },
  "limits": {
    "max_attempts": 2,
    "timeout_seconds": 7200
  }
}
```

`tools` is removed (proposal D2): with hands beside it the list is dead on
both providers, and `roster.rs::tool_grants_keep_house_tools_explicit_and_effort_never_rises_on_fallback`
already refuses `tools` beside `hands`, as
`a_codex_lane_is_chained_only_into_boxed_or_toolless_offices` refuses it
beside a Codex lane. `network` is written explicitly although `false` is the
parser's default: the four boxed work offices do, and an explicit `false` is
what a reviewer reads. No `rw` bind is declared — the box already binds the
workdir read-write, and a second writable bind or an absolute checkout path
is exactly what the spec forbids. No `boundary` key: `HandsSpec::parse`
refuses one, and the realm owns that axis. `description`, `charter` and
`limits` keep their bytes, so `library_data.rs::the_library_holds_the_decision_0041_roster`
(shared implementer charter) still holds. Effort does not rise on fallback
(`high` → `high`).

*Rejected — keeping `tools` "for Claude".* `compose` ignores it on Claude
too once hands exist, so it would be a declaration that reads as a
restriction and restricts nothing — the precise dishonesty 0043 ruling 2
removed, and the roster test fails it.

*Rejected — `opus` as a third link.* The ruling names two.

One stale exemption goes with it: the `("implementer-engine", "cargo" | "git")`
arm of `roster.rs::is_house_tool_grant` no longer matches any shipped grant
and is deleted, so the exemption table cannot outlive what it exempts. The
`chained >= 8` floor in the Codex-lane test is left alone; it is a floor.

### DD3 — Where each regression lives, and what it may not lean on

Four test files, chosen by which production seam each one reads. All are
in-crate or integration tests of `brokkr-runtime`; none needs a provider, a
network, `bwrap` or a nested namespace, and none writes under `fixtures/`.

**(a) `src/agents/tests.rs` — resolver precedence, capable provider (A1).**
`hands_keep_the_workspace_grant_and_retire_the_tool_list`. A `Tree` with a
test-only agent (`tools.allow: ["cargo","git"]` *and* the DD2 hands) and a
Claude-shaped adapter whose `tool_permissions` maps `cargo → Bash(cargo:*)`,
`git → Bash(git:*)` on flag `--allowedTools`, and whose `hands.workspace` is
the seven shipped Claude tokens. Asserts, against literals written in the
test:

- `candidates[0].argv` equals driver + `--model` + concrete + `--effort` +
  level + the seven fragment tokens, in order (the placeholder is still
  `{hands_mcp_json}` at this layer — expansion is the engine's, see (c));
- `--allowedTools` occurs exactly once and the following token is
  `mcp__brokkr__workspace`;
- no token contains `Bash(cargo:*)` or `Bash(git:*)`;
- `resolution.hands` equals the declared spec.

A control in the same test resolves the *same* adapter with the agent's
`hands` key deleted and asserts `--allowedTools`, `Bash(cargo:*),Bash(git:*)`
— so the fixture demonstrably serves both sources through one flag, and the
absence above cannot be the fixture's doing.

**(b) `src/bundle/model_policy_tests.rs` — the five admission outcomes.**
These use the file's `Fixture` and `compile_bounded`, with one new helper
that writes the smith-shaped test agent verbatim (models, efforts, optional
`tools`, optional hands) — `write_agent_file` cannot express tools or binds
and is not widened. The Codex fixture adapter is `adapter("codex", trusted)`
plus `models {"astra": "gpt-test"}`, `model_flag`, `efforts`, `effort_flag`,
`tool_permissions: "unsupported"`, and per case a `hands` member. The seat is
`{"agent": "smith", "class": "work", …}` named `work`.

| # | Test | Boundary | Fixture difference | Asserts |
| --- | --- | --- | --- | --- |
| 1 | `a_tool_listed_smith_without_hands_is_refused_word_for_word` | namespace | agent: tools, no hands; chain `astra` (Codex fixture) then a capable second link | the whole error string **equals** `seat 'work': agent 'smith' cannot be served by provider 'codex' on model 'astra': the provider declares tool_permissions unsupported, so the agent's restriction to ["cargo", "git"] cannot be expressed and the agent would run with MORE power than it declares. A capability the provider cannot express fails compilation here rather than degrading silently at run time`; a second compile with `{"unsupported": "<reason>"}` equals the same string with ` (<reason>)` after `unsupported` |
| 2 | `the_same_smith_with_hands_compiles_under_namespace` | namespace | agent gains hands, keeps tools; adapter declares `hands.workspace` | `bundle.boundary == Namespace`; `bundle.hands["work"]` equals the declared spec; `manifest["boundary"]["work"] == "namespace"`; the single candidate is provider `codex`, model `astra`, effort `high`; its argv ends with the fixture fragment; no token equals `--allowedTools` or contains `Bash(` |
| 3 | `declared_hands_need_workspace_support_not_harness_work` | namespace | adapter `hands` absent, then `{"unsupported": "<reason>"}` | error names `seat 'work'`, `agent 'smith'`, `provider 'codex'`, `model 'astra'`, contains `the provider declares hands unsupported` and `so the agent's hands cannot be put in the box and the agent would run with the harness's own tools`; the measured arm contains the reason; neither mentions `tier` or `hands.harness` |
| 4 | `a_codex_only_smith_is_admitted_under_harness_by_its_work_fragment` | harness | adapter declares `workspace` and `harness.work ["--sandbox","workspace-write"]` | compiles; `bundle.boundary == Harness`; `bundle.hands["work"]` still the declared spec (recorded, unenforced); candidate `hands_fragment` empty and argv carries no `mcp_servers.brokkr`; then `crate::engine::compose_site(BuiltBoundary::Harness, SeatClass::Work, …)` yields argv == candidate argv + `--sandbox workspace-write`, with no token containing `mcp_servers`, `hands serve`, `{hands_`, `--allowedTools` or `read-only` |
| 5 | `missing_harness_work_refuses_even_with_workspace_support` | harness | `workspace` intact, `harness.work` absent, then `{"unsupported": "<reason>"}` | error contains `seat 'work' link 1 resolves to provider 'codex'`, `` `hands.harness.work` ``, `writes the tree only under the harness's own writable sandbox`, `(decision 0046 rulings 1 and 4)`; the measured arm contains the reason |

A sixth test in the same file compiles against the **shipped** roots —
`the_shipped_smith_under_harness_refuses_on_its_claude_link`: a minimal
staged bundle whose `work` seat is `{"agent":"implementer-engine",
"class":"work"}`, `compile_roots(…, <workspace>/agents, <workspace>/adapters,
Boundary::Harness)`. It asserts `seat 'work' link 2 resolves to provider
'claude'`, `` `hands.harness.work` `` and the writable-sandbox reason, and that
the message does **not** name `link 1`. A minimal bundle avoids
`recipes/triage`'s earlier gate refusals under harness, which would mask this
one.

**(c) `src/engine/boundary_tests.rs` — the shipped namespace launches.**
Two tests share one helper that stages the same minimal bundle, compiles it
with `Bundle::compile_under(…, Boundary::Namespace)` against the shipped
`agents/` and `adapters/`, canonicalizes a `tempfile` workdir
(`/private/var` on macOS), and calls the production `compose_site(
BuiltBoundary::Namespace, SeatClass::Work, candidate.argv.clone(),
bundle.hands.get("work"), Some(candidate), &workdir, &bundle.roots,
"<workdir>/.forge/results/<id>.json", None)` for `candidates[0]` (Astra) and
`candidates[1]` (Fable). That is exactly the call `Engine::compose` makes.

*Independence rule.* The expectations never call `hands_command`,
`serve_args`, `mcp_config` or `HandsSpec::to_value` — the existing
`compose_site_follows_the_boundary_and_the_class` compares the helper to
itself, which the spec rules out here. Instead the test owns two literals:
`EXPECTED_SPEC = json!({"kind":"workspace","network":false,"binds":[
{"path":"~/.cargo","mode":"overlay","mask":["credentials.toml","credentials"]},
{"path":"~/.rustup","mode":"ro","mask":[]}]})`, and `expected_args =
["hands","serve","--workdir",<canonical workdir>,"--spec", …]`, whose last
element is compared after `serde_json::from_str` to `EXPECTED_SPEC` (so key
order inside the spec string is not pinned, only its content). `exe` is
`std::env::current_exe()` of the test binary — the same source production
reads.

`the_shipped_astra_smith_launch_is_the_read_only_codex_plus_boxed_hands`
asserts the **complete** argv:
`[exe, "driver", "codex", "--", "--model", "gpt-6-astra", "--effort", "high",
"--sandbox", "read-only", "-c", "mcp_servers.brokkr.command=\"<exe>\"", "-c",
"mcp_servers.brokkr.args=<toml>", "-c",
"mcp_servers.brokkr.default_tools_approval_mode=\"approve\""]` — every token
literal except `<toml>`, which is cut after the `=` and decoded. The engine's
TOML array is basic strings with only `\\` and `\"` escapes, which is also a
JSON array, so `serde_json` decodes it and no TOML dependency is added; the
decoded vector must equal `expected_args`. Then the absences, over every
token: no `workspace-write`, no `--allowedTools`/`--tools`/`Bash(`, no
`{hands_`, no `{brokkr}`, no `{result_path}`, and `--sandbox` occurs once.

`the_shipped_fable_smith_launch_keeps_its_whole_workspace_fragment` asserts
the complete argv `[exe, "driver", "claude", "--", "--permission-mode",
"acceptEdits", "--model", "claude-fable-5-1", "--effort", "high", "--tools",
"", "--strict-mcp-config", "--mcp-config", <json>, "--allowedTools",
"mcp__brokkr__workspace"]`; decodes `<json>` and compares it to
`{"mcpServers":{"brokkr":{"command":<exe>,"args":expected_args}}}` with the
spec element decoded as above; `--allowedTools` occurs exactly once; no token
contains `Bash(cargo:*)` or `Bash(git:*)`; no `--sandbox`, `workspace-write`
or `{hands_` anywhere.

Both also assert `bundle.manifest["agents"]["work"]` records chain
`["astra","fable"]`, `chosen_index` 0, provider `codex`, and
`manifest["boundary"]["work"] == "namespace"`.

*Accepted trade-off.* Pinning the full argv means a future, legitimate
adapter edit (a new Codex `-c`, a Claude flag) breaks these tests as well as
the witnesses. That is intended: the spec asks for "the complete relevant
argv", and an edit to a boxed work seat's launch should be read by a person.
*Rejected — asserting only `contains`*: such a suite passes while a second
`--sandbox workspace-write` is appended after the first.

**(d) `tests/roster.rs` — the declaration itself.**
`the_engine_smith_hires_astra_then_fable_in_workspace_hands` reads the JSON
file and asserts: `models == ["astra","fable"]`; `efforts` is exactly the
two-key object at `high`; `hands` equals the DD2 object (whose `~/.rustup`
bind carries no `mask` key); `tools` and `boundary` absent; no string
anywhere in the document starts with `/`; exactly two binds, none `rw`;
`charter == "charters/implementer.md"`; `limits == {max_attempts: 2,
timeout_seconds: 7200}`. It then resolves the agent through the public
`resolve` against the shipped trees and asserts providers
`["codex","claude"]` and efforts `high`/`high`.
`every_shipped_panel_seats_at_least_two_providers`, the whole
`gpt_flash_shape` suite and `library_data` run unmodified; none is edited,
exempted or weakened.

### DD4 — Which identities move, and why only those

An agent file lives in `agents/`, outside every bundle directory, so it never
appears in a manifest's `files`. It reaches identity only through the hiring
site: `manifest.agents[<site>]` (`agent_digest`, `adapter_digest`, `chain`,
`model`, `provider`) and the `hands`/`boundary` maps. An ancestor's digest
(`bundle/compose.rs`, lines 817–846) is computed with `agents: None` and no
hands, so a base's hires do not leak into its descendants' `@compose/` rows.

From fact 8, the predicted movement is **one identity, pinned twice**:

| Pin | File | Moves because |
| --- | --- | --- |
| `recipes/triage` in `WITNESSES` | `tests/witness_digests.rs` | the `implement` engine case's record changes: new `agent_digest`; `adapter_digest` now hashes `{claude, codex}` where it hashed `{claude}`; chain `["astra","fable"]`, model `astra`, provider `codex`; and the site joins the manifest's `hands` and `boundary` maps |
| the literal in `a_composed_bundles_manifest_is_pinned` | `src/bundle/compose_tests.rs` | the same compile; the two values must agree |

Predicted **not** to move: `recipes/night-shift` and `recipes/gpt-flash`
(they replace the `implement` seat and inherit triage only through a
files-only ancestor digest), `recipes/fast`, `node`, `preflight`,
`wager-harness`, `research`, `research-dsh`, `bundles/verify`, and the
`UNCOMPOSED` four (`fast`, `panel-review`, `bundles/self`, `bundles/verify`).
No adapter digest moves, because no adapter byte does.

The prediction is a plan, not a pin. The implementer runs both pin tests
after the agent edit, copies the **reported right-hand value** of each
failing `assert_eq!` (the files' standing rule: "updated only from the tests'
reported left/right pairs"), and treats any *other* moved pin as a defect to
explain before re-pinning — never as a hash to absorb. Each collection's
history comment gains an appended paragraph, replacing nothing:

> The 2026-09-21 ruling on #307 moves `recipes/triage` alone: the engine
> smith now hires astra@high → fable@high and declares workspace hands in
> place of its `cargo`/`git` tool list, so its resolution record consults the
> codex adapter beside claude and the site joins the manifest's `hands` and
> `boundary` maps. `night-shift` and `gpt-flash` replace the implement seat
> and keep their digests, as do the other seven.

`docs/guides/recipe-authoring.md` and `docs/guides/agent-library.md` carry
illustrative `recipes list` / `agents list` transcripts that already lag the
roster (the `chief-architect` row predates its current chain) and are bound
by no test beyond their `$` marker. They are outside the spec's documentation
requirement and are not touched; the lag is named here rather than half-fixed.

### DD5 — Removal evidence: one table, one protocol

Evidence is recorded by the implement phase in
`openspec/changes/2026-09-21-307-astra-engine-smith/removal-evidence.md`
(precedent: `boundary-seatbelt-slice-ii/evidence-residuals.md`). Protocol per
row: apply exactly one mutation → run only the targeted test
(`cargo test -p brokkr-runtime <name>`) → record the failing assertion's own
message → `git checkout --` the mutated file → rerun green. A build error, an
unrelated refusal, or a test-fixture edit alone is not a row. A mutation of a
*shipped data file* counts where the protection **is** that data (the
adapter's `read-only`, Claude's MCP grant, the agent's masks).

| # | Protection | Mutation | Test that must fail, and how |
| --- | --- | --- | --- |
| R1 | no-hands refusal exists | `compose`: on missing `tool_permissions`, fall through instead of `ok_or_else` | (b)1 — compile succeeds; "expected the tool_permissions refusal" |
| R2 | refusal text is exact | `compose`: `MORE power` → `more power` | (b)1 — string equality, diff shown |
| R3 | a later link cannot bypass | `resolve_report`: `continue` past a gapped entry | (b)1 — compile succeeds on the capable second link |
| R4 | hands precede the list | `compose`: test `agent.allow` before `agent.hands` | (b)2 fails with the tool_permissions refusal; (a) fails `--allowedTools` count 2 and `Bash(cargo:*)` present |
| R5 | workspace fragment is appended | `compose`: drop `argv.extend(fragment…)` | (b)2 fragment-suffix assertion; (a) full-argv equality |
| R6 | hands are recorded | `resolve_report`: `hands: None` | (b)2 `bundle.hands["work"]` missing |
| R7 | missing workspace refuses | `compose`: missing `adapter.hands` yields an empty fragment | (b)3 — compile succeeds |
| R8 | namespace never borrows harness work | `compose`: on missing `adapter.hands`, append `adapter.harness.work` | (b)3 — compile succeeds with `workspace-write` in argv |
| R9 | harness composes the work fragment | `compose_site` Harness arm: select `gate` for `Work` | (b)4 argv equality (`read-only` present) |
| R10 | harness serves no box | `report_under`: pass `true` for `boxed` | (b)4 `hands_fragment` non-empty, `mcp_servers` present |
| R11 | harness work is required | `enforce_hands_boundary`: empty the `Work if under_harness` arm | (b)5 — compile succeeds |
| R12 | workspace support does not satisfy it | same arm: refuse only when the candidate's adapter also lacks `hands` | (b)5 — compile succeeds |
| R13 | every link is judged | `enforce_hands_boundary`: `candidates.iter().take(1)` | (b)6 — shipped smith compiles under harness |
| R14 | Codex MCP registration | `adapters/codex.json`: delete the `command` pair; separately, `hands_command`: skip the `{hands_args_toml}` replace | (c) Astra argv equality; `{hands_` absence |
| R15 | workdir / spec forwarded whole | `hands_command`: pass `Path::new("/")`; separately `&HandsSpec::default()`; separately agent data: drop a mask, set `network: true` | (c) both — decoded `expected_args` / `EXPECTED_SPEC` equality; (d) for the data rows |
| R16 | Codex native sandbox read-only | `adapters/codex.json` workspace: `read-only` → `workspace-write` | (c) Astra argv equality and `workspace-write` absence |
| R17 | no harness fragment under namespace | `compose_site` Namespace arm: append `candidate.harness.work` | (c) Astra `--sandbox` count and `workspace-write` absence |
| R18 | Claude fragment preserved | `adapters/claude.json`: delete `--allowedTools`, `mcp__brokkr__workspace`; separately delete `--strict-mcp-config` | (c) Fable argv equality and grant-count assertion |
| R19 | no retired grant on shipped Claude | R4's mutation **plus** restoring `tools` to the agent | (c) Fable `Bash(cargo:*)` absence; `--allowedTools` count 2 |
| R20 | no tool-list flag on Codex | R4's mutation **plus** restoring `tools` to the agent | (c) Astra — the helper's compile expectation fails carrying R1's refusal text: a list can reach Codex only as that refusal, and the row records it as such |
| R21 | declaration shape | agent data: add `tools`; reorder to `["fable","astra"]`; add an `rw` bind | (d) the matching field assertion |

R19 and R20 are stated honestly: on shipped data the precedence mutation
alone cannot produce a Cargo/Git grant, because the list is gone. The
retired-grant protection on the *shipped* launch is the conjunction of "no
`tools`" (R21) and "hands precede" (R4), and those rows remove both. The
fixture test (a) proves precedence in isolation.

All of R1–R21 is **composition evidence**. None of it says what a live
`codex exec` does with the argv, and none of it is a native-tool enforcement
contract.

### DD6 — The guide and the dated note

`docs/guides/provider-adapters.md` gains one subsection directly after the
paragraph that introduces the `workspace` fragment and before
`### hands.harness`: *"A provider without per-tool flags: restricting by
hands"*. Content, in this order: (1) a work seat that must be restricted on
a provider with `tool_permissions` unsupported declares hands; under
`namespace` that requires `hands.workspace`, writes go through the MCP
server, and Codex's own sandbox stays `read-only`; (2) the confinement is the
box's filesystem and network boundary — the workspace tool runs `bash -lc`,
and nothing parses or filters commands, so it is **not** a `cargo`/`git`
allow-list; (3) 0043's limits, kept: Codex's native shell remains available
read-only outside the box, host-read secrecy is not promised, provider
traffic is the harness's and outside the box; (4) `hands.harness.work` is the
separate, unboxed route with its own whole-chain admission — never composed
with the workspace fragment; the shipped Claude adapter declares none, so
the engine smith refuses under `harness` on its second link; (5) the engine
smith named as the worked example. Any list of agent CLIs in the new prose
reads "claude, codex or dsh". The guide's `A provider adapter is **data**`
marker and every existing heading stay.

`docs/decisions/0043-the-hands-are-one-tool.md` gains a final section
`## Operator note — 2026-09-21 (issue #307)`, appended after `Consequences`.
The status line, rulings and consequences are not edited. The note records:
the engine smith's hire and hands; that the earlier commission's claim of
allow-list enforcement is withdrawn; that no per-tool contract and no
boundary composition is being designed; that namespace and harness remain
two launches under 0046; and that this is application of accepted 0043 and
0046, creating and accepting nothing. The decision index is test-derived
from title and status, which do not move; the implementer runs the index
test to confirm.

*Rejected — a new `proposed` decision.* House rules ask for one on a
semantic change; this is not one (proposal D4), and minting a document the
operator must accept for a roster edit would misstate what happened.

### DD7 — Requirement → proof map

| Requirement (spec) | Proved by |
| --- | --- |
| Smith hires the ruled chain through workspace hands (`astra-engine-smith`) | (d); (c) manifest-record assertions; R21 |
| Writable access uses the existing mount | (c) decoded `--workdir` and two binds only; (d) no `rw`, no absolute path; R15 |
| Tools removed, MCP grant remains (A1) | (d) no `tools`; (c) both launches; (a); R4, R18, R19 |
| Witnessed by actual compiled identities | DD4 pins and history; `pinned_bundles_keep_their_recorded_digest`, `a_composed_bundles_manifest_is_pinned`, `recipes_that_opted_into_nothing_keep_their_digests` green |
| Independent rosters keep their guarantees | unmodified `every_shipped_panel_seats_at_least_two_providers`, `gpt_flash_shape`, full `cargo test -p brokkr-runtime` |
| Guide and decision record | DD6; decision-index and `rename_guard` tests green |
| Deterministic evidence is not the live smith | delivery-notes wording below; no test claims it |
| No-hands refusal word for word (`boxed-work-provider-admission`) | (b)1; R1–R3 |
| Namespace hands replace the list only via `hands.workspace` | (b)2, (b)3, (a); R4–R8 |
| Harness work is a separate, whole-chain case | (b)4, (b)5, (b)6; R9–R13 |
| Shipped namespace launch tested as composed | (c) both; R14–R20 |
| Every protection has removal evidence | DD5's file, one row per R-number |

Quality evidence for the implement phase, each recorded as run or as
**pending** — never inferred: `cargo fmt --all -- --check`; clippy with
`-D warnings`; each of the seven crates' suites separately;
`cargo test --workspace` (under a timeout: the workspace run has hung in
`brokkr-cli` before; a hang is recorded as a hang, with the crate-scoped runs
beside it); `compile --bundle bundles/self`; `openspec validate --strict`;
`scripts/coverage-exact.sh` on the host or CI outside the box, threshold
untouched. DD1 adds no production line, so exact coverage has nothing new to
cover; if DD1's repair branch is taken, its lines are owed. Result notes
describe evidence and residuals and say "expressible, not proved by a live
smith"; the controller's first live Astra implementation (Cargo and Git
through Codex's boxed hands, a real commit, verify passing) is listed as
pending. No note instructs a gate to pass.

## Risks / Trade-offs

- **A Codex engine smith is a live unknown.** Compilation proves the hire is
  expressible; whether `gpt-6-astra` builds engine work through one MCP tool
  within 7200 s is the controller's measurement. Fable is the fallback link
  and the limits are unchanged. Accepted.
- **Codex keeps a read-only native shell outside the box.** On the Astra
  link the model can read host files, credentials included; only its writes
  and its Cargo/Git runs are confined. This is 0043's recorded limit, not
  new; DD6 repeats it rather than softening it. Accepted by ruling.
- **No command restriction.** Inside the box the seat may run anything the
  toolchain binds and the workdir allow. The former `cargo`/`git` list was
  never expressible on Codex and is withdrawn, not replaced. Accepted by
  ruling.
- **No boxed resume on either link.** Codex's `work-site` shape names
  `hands: "none"` under `harness`, and Claude's `boxed-workspace` shape is
  `unmeasured`, so every retry of this seat starts cold. That is today's
  behaviour for every boxed office; it costs tokens, not correctness.
- **Harness realms cannot hire this smith.** The shipped Claude adapter has
  no `hands.harness.work`, so the chain refuses on link 2. Intended, tested
  by (b)6, and the right failure until someone measures Claude's fragment.
- **Hosts.** On macOS the bundle compiles and the composition tests run; a
  namespace run refuses at start (0043 ruling 7). Nothing here changes that.
  A machine on bubblewrap < 0.10 refuses the overlay bind, exactly as it
  already does for `reviewer`.
- **Full-argv pins are brittle by design** (DD3's trade-off).
- **DD5's data mutations touch shipped files.** Each is reverted with
  `git checkout --`, the row records the restored green run, and the final
  tree must show `adapters/` clean in `git status`.
- **The digest prediction could be wrong.** If another pin moves, DD4's rule
  applies: explain first. The likeliest cause would be a hirer this design
  missed; `grep -rn implementer-engine recipes bundles` is the check.

## Migration Plan

Order matters, because the agent edit reddens the pins until they are
re-measured:

1. Add tests (a) and (b)1–5 against the unmodified tree; they pass on
   fixtures alone. (b)6, (c) and (d) are written now and fail for the
   expected reason (the shipped smith still lists tools and no hands);
   record that as the pre-change observation.
2. Edit `agents/implementer-engine.json` (DD2) and drop the dead
   `is_house_tool_grant` arm. (b)6, (c) and (d) turn green.
3. Run the two pin tests; re-pin from reported values; append both history
   paragraphs (DD4).
4. Run DD5's removal rows; write `removal-evidence.md`.
5. Write the guide subsection and the 0043 note (DD6).
6. Run the quality list; record pending items as pending; commit. Never push.

Rollback is one revert: no schema, contract, journal or adapter byte moves,
and an in-flight `recipes/triage` run pinned to the old manifest digest
refuses the new bundle as it would after any identity change.

## Open Questions

None blocks implementation. Two are recorded for the operator, neither
answered by guesswork here:

1. Should the illustrative `agents list` / `recipes list` transcripts in the
   guides be regenerated in a separate docs slice? They already lag (DD4).
2. After the controller's first live Astra measurement, does Claude's
   `hands.harness.work` get measured so this smith can serve harness realms?
   That is a new adapter declaration and its own change.
