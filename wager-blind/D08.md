# Design: Astra as the boxed engine smith (issue #307)

## Context

Status: proposed. Change: `2026-09-21-307-astra-engine-smith`.

This design adopts the proposal committed at `1ab50740` and amended at
`5ef97115`, and both capability deltas
([astra-engine-smith](specs/astra-engine-smith/spec.md),
[boxed-work-provider-admission](specs/boxed-work-provider-admission/spec.md))
as clarified and ruled clear. It was authored solo, on branch
`wager-307-design-qwen-max` at production base `5ef97115`; there was no
council, so there are no positions to reconcile. Line anchors below were
read at that commit and cite functions by name first, because line numbers
drift.

The operator rulings that bind this design (proposal D1, D3, D5; the
2026-09-20 hire ruling and the 2026-09-21 issue #307 rulings):

1. The engine smith's chain is `astra` then `fable`, both efforts `high`.
2. The restriction this change owes is decision 0043's **existing**
   workspace confinement — the empty-root filesystem and network boundary —
   not a per-tool allow-list. No new per-tool enforcement contract,
   command parsing, transport semantics or native-tool bypass prevention
   is designed here.
3. The boxed smith uses `hands.workspace` alone. Decision 0046's separate
   `hands.harness.work` path stays unboxed, separately admitted per link,
   and is never composed into a namespace launch.
4. An agent carrying a tool list on a provider that cannot express one,
   with no hands, stays refused **word for word**.

What the code as it stands already does (read, not assumed):

- `agents.rs::compose` (fn `compose`, ~714–899) appends, in order: the
  model pin, the effort pin, then exactly one restriction branch. When
  `agent.hands.is_some()` **and** the boundary is boxed
  (`brokkr_core::realms::Boundary::is_boxed` — namespace/seatbelt/
  container), it requires `adapter.hands` (the `hands.workspace`
  fragment) and appends that complete fragment, consulting neither
  `tools.allow` nor `tool_permissions`; a provider without workspace
  hands earns `ResolveError::Capability` with the reason
  `the provider declares hands unsupported ({measured reason}), so the
  agent's hands cannot be put in the box and the agent would run with the
  harness's own tools`. Only when the agent declares **no** hands does the
  `allow` branch run, and a provider declaring `tool_permissions`
  unsupported earns the Capability reason
  `the provider declares tool_permissions unsupported ({measured reason}),
  so the agent's restriction to ["cargo", "git"] cannot be expressed and
  the agent would run with MORE power than it declares`. The Display for
  `Capability` wraps either reason as
  `agent '{agent}' cannot be served by provider '{provider}' on model
  '{model}': {capability}. A capability the provider cannot express fails
  compilation here rather than degrading silently at run time`, and
  `bundle.rs::resolve_reference` wraps that with `seat '{what}': …`. A
  bare `"unsupported"` string in an adapter carries no measured reason, so
  the parenthetical is absent — which is exactly how the spec's verbatim
  reason text is spelled (`agents/load.rs` fn `capability`, ~176–194;
  `tool_permissions_gap`/`hands_gap` are `None` for the bare form).
- `bundle.rs::resolve_reference` (~2053–2166) resolves an agent seat under
  the realm's boundary, runs the D33 pre-pass
  (`enforce_model_policy` → `enforce_hands_boundary`) over **every mapped
  link** before `resolve_report`, lints and `expand_command`s each
  candidate argv (so `{brokkr}` as a whole token becomes the machine-local
  executable, while substring tokens like
  `mcp_servers.brokkr.command="{brokkr}"` and `{hands_args_toml}` survive
  to spawn), and records the resolution under the flattened site key
  (`implement:engine` for the triage select case, `format!("{phase}:{case}")`).
- `bundle.rs::enforce_hands_boundary` (~2610–2700) returns immediately for
  boxed boundaries; under `harness` a work seat with hands needs
  `hands.harness.work` on **every** link, refusing with
  `seat '{what}' link {link} resolves to provider '{provider}', which
  declares no \`hands.harness.work\` fragment{ (measured reason)}: a
  capability gap — under the \`harness\` boundary a work seat with hands
  writes the tree only under the harness's own writable sandbox as the
  adapter addresses it (decision 0046 rulings 1 and 4)`.
- `engine.rs::compose_site` (~4270–4317) is the one launch composer:
  `BuiltBoundary::Namespace` routes a model site through
  `engine.rs::hands_command` (~4509–4582), which expands
  `{hands_mcp_json}` (via `brokkr_protocol::hands::mcp_config`),
  `{hands_args_toml}` (via `hands::serve_args`, TOML-quoted) and the
  `{brokkr}` substring, and returns `SiteSpawn::inherit`;
  `BuiltBoundary::Harness` appends only the class-selected
  `candidate.harness` fragment (`work` for a work seat) with
  `{result_path}`/`{brokkr}` expanded, serves no workspace tool, and never
  reads `hands.workspace`; `BuiltBoundary::Open` appends nothing.
- `brokkr_protocol::hands`: `HandsSpec::parse` refuses a `boundary` key
  (the word is the realm's, decision 0046 ruling 1) and any key outside
  `kind`/`network`/`binds`; `serve_args` is
  `["hands","serve","--workdir",<workdir>,"--spec",<spec JSON>]`;
  `HandsSpec::to_value` serializes with sorted keys
  (`binds` < `kind` < `network`; per bind `mask` < `mode` < `path`);
  `box_argv` binds the workdir read-write at its own path — the existing
  workspace mount that supplies writable project access, unchanged here.
- Shipped data: `adapters/codex.json` declares `tool_permissions`
  unsupported **with a measured reason**, a `hands.workspace` fragment
  (`--sandbox read-only`, the three `mcp_servers.brokkr.*` `-c`
  assignments including `default_tools_approval_mode="approve"`), and both
  `hands.harness` members (`gate` with `--output-last-message
  {result_path}`, `work` as `["--sandbox","workspace-write"]`, door
  `last-message`). `adapters/claude.json` declares a full
  `tool_permissions` map (including `cargo → Bash(cargo:*)`,
  `git → Bash(git:*)`), a `hands.workspace` fragment
  (`--tools ""`, `--strict-mcp-config`, `--mcp-config {hands_mcp_json}`,
  `--allowedTools mcp__brokkr__workspace`), and **no** `hands.harness`.
  `agents/implementer-engine.json` today: models `["fable","opus"]`, both
  efforts `high`, `tools.allow ["cargo","git"]`, no hands, charter
  `charters/implementer.md`, limits `{2, 7200}`. `agents/reviewer.json`
  already declares the exact hands block the smith needs (`network: false`,
  `~/.cargo` overlay masking `credentials.toml` and `credentials`,
  `~/.rustup` read-only). The smith is hired at exactly one shipped site:
  `recipes/triage/bundle.json`, seat `implement`, select case `engine`,
  class `work`. `recipes/night-shift` overrides `implement` with an inline
  dsh lane; `recipes/gpt-flash` overrides it with `gpt-flash-implementer-engine`;
  neither resolves the shipped smith.
- Existing pins that constrain the edit and must stay green:
  `tests/roster.rs::tool_grants_keep_house_tools_explicit_and_effort_never_rises_on_fallback`
  (hands ⇒ **no** `tools` key; first effort ≥ every fallback effort),
  `a_codex_lane_is_chained_only_into_boxed_or_toolless_offices` (an agent
  naming a codex lane may carry no `tools`),
  `every_shipped_panel_seats_at_least_two_providers`;
  `tests/library_data.rs::every_shipped_agent_resolves_at_compile_time`
  (chosen 0, no notices, ≥2 candidates, `argv[0] == "{brokkr}"`) and the
  shared-charter assertion in `the_library_holds_the_decision_0041_roster`;
  `tests/adoption.rs` (resolved argv pinned element-for-element per
  rostered triage site);
  `tests/crucible_review_sequence.rs::every_review_site_is_witnessed_by_its_agent_resolution`
  (`manifest.agents` must keep an `implement:engine` entry);
  `bundle/model_policy_tests.rs::every_shipped_bundle_compiles_under_harness_once_the_fragments_are_measured`
  and `a_measured_claude_gap_is_reported_not_papered_over` (the three
  dialect bundles refuse under harness at `analyze:check` — alphabetically
  the first seat — before `implement` is ever parsed, so the smith's new
  hands cannot reorder those pins);
  `tests/gpt_flash_shape.rs` (the scoped GPT/Flash crew and
  `drivers`-parity with triage);
  `tests/witness_digests.rs` (10 pins) and `bundle/compose_tests.rs`
  (`UNCOMPOSED` 4 pins + the composed `recipes/triage` pin).

Validation on this design visit: none executed. The commission forbids
building or testing (`no cargo invocation is needed to write a design, and
the box has no network`), so no formatting, clippy, test, bundle-compile,
coverage or strict-OpenSpec result is claimed here; they are implementation
obligations recorded in the Migration Plan. Nothing outside this change
directory was edited, and no frozen byte (`contracts/`, `fixtures/`,
`policy/`, `reference/`) was touched or is planned to be.

## Goals / Non-Goals

**Goals:**

- Land the operator's declaration: `implementer-engine` hires
  `["astra","fable"]` at `{"astra":"high","fable":"high"}` with workspace
  hands (`network: false`, the reviewer's exact toolchain binds) and no
  `tools` object; charter, limits and description preserved; no boundary
  declared anywhere in the agent.
- Prove — with reason-bearing, removal-evidenced regressions — that the
  existing admission rules already carry the ruling: the verbatim no-hands
  tool-list refusal, namespace admission through declared
  `hands.workspace`, refusal without it, hands-over-tools precedence that
  keeps Claude's MCP grant and drops the retired `Bash(...)` grants, and
  the separately resolved harness outcomes including the whole-chain
  refusal on the shipped Fable link.
- Prove the shipped smith's actual namespace launch composition on both
  providers against independent expectations, with canonicalized temporary
  workdirs, on Linux and macOS only (decision 0063).
- Move exactly the digests that move — `recipes/triage` in both pin
  collections — with history entries attributing the move to issue #307,
  and demonstrate that inherited uses and ancestors stay byte-identical.
- Make the guide and a dated note on decision 0043 describe the confinement
  actually declared, distinguish expressibility from the controller's
  still-pending first live Astra implementation, and change no status and
  no historical text.
- Zero production-code edits: name every function that must not change and
  the contingency rule if a test nonetheless exposes a gap.

**Non-goals:**

- No per-tool allow-list enforcement contract, command-name parsing, shell
  policy inside the box, MCP transport change, or native-tool bypass
  prevention (operator ruling; proposal D1).
- No adapter capability change: `adapters/codex.json` and
  `adapters/claude.json` keep their bytes; both `hands.workspace`
  fragments are preserved verbatim.
- No guessed Claude `hands.harness.work` fragment; the harness realm keeps
  refusing the smith's Fable link until the operator measures one.
- No boundary-vocabulary change, no new decision document, no status
  change to 0043 or 0046, no combined namespace+harness launch.
- No change to `gpt-flash-implementer-engine` (the GPT/Flash crew stays
  independent), to any other agent, to panel diversity, to `muninn`, or to
  the `realm-boundary` / `gate-boundary-policy` semantics the proposal
  leaves in force.
- No live provider, network access or nested namespace in any test; no
  Windows obligation; no edit to frozen trees.
- No seatbelt/container enablement (decision 0046 ruling 6 slices remain
  their own changes); under those realms the smith compiles as boxed and
  the existing run-start `UnbuiltBoundary` refusal still stands.
- This phase authors no `tasks.md` and no code; the deliverable is this
  file.

## Decisions

### D1 — The declaration is one data file, shaped by the shipped precedents

`agents/implementer-engine.json` becomes exactly (pretty-printed JSON,
`hands` block byte-identical in content to `agents/reviewer.json`'s):

```json
{
  "description": "Engine-class implementer: builds core, store, contract, and policy work selected by triage.",
  "charter": "charters/implementer.md",
  "models": ["astra", "fable"],
  "efforts": {"astra": "high", "fable": "high"},
  "hands": {
    "kind": "workspace",
    "network": false,
    "binds": [
      {
        "path": "~/.cargo",
        "mode": "overlay",
        "mask": ["credentials.toml", "credentials"]
      },
      {
        "path": "~/.rustup",
        "mode": "ro"
      }
    ]
  },
  "limits": {"max_attempts": 2, "timeout_seconds": 7200}
}
```

Why each field is what it is:

- `astra` maps only in `adapters/codex.json` (`gpt-6-astra`), `fable` only
  in `adapters/claude.json` (`claude-fable-5-1`); both providers declare
  `high` in `efforts`, so both links compose model and effort pins. Effort
  never rises on fallback (`high ≥ high`), satisfying the roster rule.
- The hands block is the reviewer's: the overlay carries the Cargo home
  (the registry cache and git database a build needs) with both credential
  files — `credentials.toml` and `credentials` — masked under it,
  `~/.rustup` is read-only, and the workdir itself is **not** bound here —
  `hands::box_argv` binds it read-write at its own path already (0043
  ruling 1). An extra bind would duplicate the workspace
  mount; any absolute checkout path would make the shipped agent
  machine-specific, which the spec forbids.
- `network: false` is the operator's ruling and the boxed-work precedent
  (`analyst`, `clarifier`, `chief-architect`, `intake-sdd`, `reviewer`);
  `release-manager`'s `network: true` serves release work and is not this
  smith's precedent (proposal D2). Cargo builds inside the box run against
  the overlay's warm registry cache with the `Cargo.lock` in the worktree;
  a cold cache without network fails the build loudly — an operational
  fact the controller's live measurement will observe, not something this
  slice may paper over with a network grant.
- `tools` is **removed**, not kept beside hands: 0043 ruling 2 ignores it
  on both providers, `roster.rs` already refuses dead tools beside shipped
  hands, and keeping a Claude-only `Bash(cargo:*),Bash(git:*)` rendering
  would misstate the smith's restriction on the very change whose point is
  that the restriction is the box. Charter and limits are preserved; the
  shared `charters/implementer.md` keeps its bytes, so every charter
  digest pin (`library_data.rs`, `adoption.rs`) is unmoved.
- No `boundary` key: the loader's `only_keys` would refuse a top-level
  one and `HandsSpec::parse` refuses one inside `hands`; the realm selects
  the axis (0046 ruling 1).

Alternatives rejected: keeping `tools` as documentation (dead declaration,
refused by the roster pin, misleading per proposal D2); binding the
workdir or a checkout path explicitly (duplicates the automatic mount,
machine-specific, spec-forbidden); `network: true` for crate fetches
(contradicts the ruling and the boxed-work precedent); declaring
`boundary: namespace` in the agent (0046 ruling 1; loader refuses);
chaining `sol` or `opus` instead of the ruled pair (the ruling is the
chain); putting the smith on a new scoped agent file (the ruling seats the
shipped office; the GPT/Flash scoped-crew pattern is decision 0058's, for
a forced crew, and duplicating the office would fork the charter
accounting).

### D2 — Zero production-code change is the design; the smallest-repair rule is the only contingency

Reading the four functions the commission names shows the ruling is
already expressible, exactly as proposal D3 claims:

- `agents.rs::compose` — the smith has hands and both providers declare
  `hands.workspace`, so both links append their complete fragment and
  never consult the retired list; `resolve_report` keeps failing any gap
  on **any** entry, so the whole chain is judged.
- `bundle.rs::enforce_hands_boundary` — under namespace the law returns at
  the boxed guard; under harness it walks every link and refuses on the
  first missing `hands.harness.work` (claude, link 2) with the existing
  reason; `resolve_reference`'s D33 pre-pass runs it before capability
  resolution, so the refusal order is unchanged.
- `engine.rs::compose_site` + `hands_command` — the namespace arm expands
  the two workspace tokens and the `{brokkr}` substring from the
  candidate's already-compiled argv; the harness arm appends only
  `candidate.harness.work`; nothing combines them.
- `hands.rs` — `serve_args`/`mcp_config`/`HandsSpec` need no new field:
  the smith's policy is exactly the shipped vocabulary.

Therefore **no function in `crates/` is edited**. The following must stay
byte-identical (any diff there is a design violation, not an improvement):
`agents.rs` (`compose`, `report_under`, `resolve`, `resolve_report`, the
`ResolveError` texts), `agents/load.rs` (all parsers), `bundle.rs`
(`resolve_reference`, `enforce_model_policy`, `enforce_hands_boundary`,
`measured`, `manifest_for`, `body_manifest`, `expand_command`),
`bundle/compose.rs` (ancestor digest rule), `engine.rs` (`compose_site`,
`hands_command`, `Engine::compose`), `brokkr-protocol/src/hands.rs`
(everything), and both adapter files.

Contingency (proposal): if, and only if, a required regression exposes a
gap between the spec's verbatim texts and the code, the repair is the
smallest edit consistent with decisions 0043 and 0046 that preserves every
existing refusal string verbatim. Two source-shape pins bound any such
repair: `agents/tests.rs::the_resolver_module_reaches_for_nothing_outside_its_arguments`
(no `std::fs`/`std::env`/`Command`/clock may appear in `agents.rs`) and
`brokkr-cli/tests/boundary_readouts.rs` (readout source shapes). A repair
would also re-open the exact-coverage obligation for the added production
lines. The design predicts no repair is needed.

Alternatives rejected: implementing per-tool enforcement in
`hands::execute_in` (operator ruling forbids; would be a new semantic
change needing its own proposed decision); appending `harness.work` under
namespace "for safety" (violates ruling 3 and `compose_site`'s one-arm-per-
boundary law); teaching `compose` to emit tool flags beside hands
(rejected by proposal D5 — it would contradict 0043 ruling 2 and re-create
the A1 defect); relaxing `enforce_hands_boundary` so the codex link alone
admits the chain under harness (violates the whole-chain rule and the
spec's "no fallback omission").

### D3 — Admission regressions live in `bundle/model_policy_tests.rs`, on temporary fixtures, asserting reason text verbatim

Home: `model_policy_tests.rs` already hosts the hands law's pins, its
`Fixture` (tempdir bundle + tempdir `agents/`+`adapters/` trees) and the
helpers `compile`, `compile_bounded`, `refusal_under`, `compile_roots`,
`boxed_seat`, `scratch_adapters`, `compile_shipped`. The new tests extend
that suite (house rule) and never touch the frozen `fixtures/` tree. One
new test-local helper `write_agent_body(name, body)` writes a charter plus
a raw agent JSON, because the existing `write_agent_file` cannot express
`tools` beside `hands`.

The file header boasts that every provider name there is invented. The
spec requires the refusal to name provider `codex`, so the fixture adapter
declares `"provider": "codex"` — still synthetic data in a tempdir, never
the shipped file. The header comment gains one sentence recording that
#307's verbatim-reason scenarios name the provider word on temporary
fixtures, and that the engine still matches declarations, not vendors.

Fixture family (all tempdir-written, no live provider):

- `codex` fixture adapter: `driver ["{brokkr}","driver","codex","--"]`,
  `models {"astra": "gpt-6-astra"}`, `model_flag "--model"`,
  `efforts ["high"]`, `effort_flag "--effort"`,
  `tool_permissions "unsupported"` (**bare** — so the refusal spells the
  spec's sentence with no parenthetical), `mcp "unsupported"`, and per-test
  `hands` variants: absent; `{"workspace": [<codex-shaped fragment with
  both tokens>]}`; `{"unsupported": "<measured reason>"}`;
  `{"workspace": [...], "harness": {"work": ["--sandbox","workspace-write"]}}`;
  `{"workspace": [...], "harness": {"work": {"unsupported": "<reason>"}}}`.
- `claude`-shaped fixture adapter (A1): `models {"fable":
  "claude-fable-5-1"}`, full `tool_permissions`
  (`--allowedTools`, `,`, `cargo → Bash(cargo:*)`, `git → Bash(git:*)`),
  and `hands.workspace`
  `["--tools","","--strict-mcp-config","--mcp-config","{hands_mcp_json}","--allowedTools","mcp__brokkr__workspace"]`
  — the same flag spelling on both grant sources, so flag-name absence
  cannot masquerade as proof of precedence (proposal D5).
- Fixture agents: `smith-no-hands` (`tools.allow ["cargo","git"]`, chain
  `["astra"]` or `["astra","fable"]`, efforts high, no hands);
  `smith-hands` (same list **plus** the smith's exact hands object);
  `smith-hands-claude` (hands + list, chain `["fable"]`).

Tests (names follow the house's sentence style):

1. `a_tool_listed_work_seat_keeps_codexs_exact_refusal_without_hands` —
   `boxed_seat("smith-no-hands","work")` compiles under namespace (the
   fixture default) and refuses. The assertion compares **text**, never
   `is_err()`: the refusal contains
   `seat 'work': agent 'smith-no-hands' cannot be served by provider 'codex' on model 'astra': the provider declares tool_permissions unsupported, so the agent's restriction to ["cargo", "git"] cannot be expressed and the agent would run with MORE power than it declares`
   and retains the trailing capability explanation
   `A capability the provider cannot express fails compilation here rather
   than degrading silently at run time`. Variants in the same test: (a) a
   measured `{"unsupported": "<reason>"}` declaration keeps
   `(<reason>)` in the same shape; (b) a two-link chain
   `["astra","fable"]` with the capable `claude` fixture present still
   refuses on link 1 — an available later candidate does not bypass the
   refusal; (c) the work class and namespace realm are the setting, not an
   escape.
2. `the_same_tool_listed_seat_compiles_once_hands_and_workspace_support_are_declared`
   — the same agent **keeping its tool list** plus the smith's hands
   object, the codex fixture declaring `hands.workspace`, compiled under
   `Boundary::Namespace`: success asserted positively —
   `bundle.hands["work"]` equals the declared spec (network false, both
   binds, both masks), `bundle.manifest["boundary"] == {"work":
   "namespace"}`, the selected candidate's argv contains the complete
   declared fragment (`--sandbox read-only`, the three
   `mcp_servers.brokkr.*` `-c` tokens with `{hands_args_toml}` and
   `"{brokkr}"` still literal at compile time) and contains **no**
   `--allowedTools` token and no `Bash(` substring — replacement
   suppresses list-generated arguments, not the fragment's own grants.
3. `declared_hands_refuse_a_codex_adapter_without_workspace_support` —
   `smith-hands` against (a) the hands-absent fixture and (b)
   `{"unsupported": "no flag swaps the tool surface"}`: both refuse with
   `the provider declares hands unsupported` and
   `so the agent's hands cannot be put in the box and the agent would run
   with the harness's own tools`, (b) preserving the supplied reason in
   parentheses. The test asserts this is the capability refusal (seat,
   agent, provider, model all named), not a malformed-fixture or tier
   error. No borrowing of `hands.harness.work` can occur, and variant (c)
   pins why: a `hands` object declaring `harness` but no `workspace`
   member is refused at LOAD (`'hands' needs 'workspace' as an array of
   strings` — once the object is neither absent nor `unsupported`, the
   loader requires the workspace member), so no provider can hold only a
   work fragment for boxed hands to resolve against.
4. `hands_supersede_the_tool_list_without_removing_the_workspace_mcp_grant`
   (A1) — `smith-hands-claude` against the claude-shaped fixture under
   namespace: compiles; the candidate argv preserves the **entire**
   fragment in order; `--allowedTools` occurs exactly once and its value
   is `mcp__brokkr__workspace`; no argument contains `Bash(cargo:*)` or
   `Bash(git:*)`; the recorded hands equal the declaration. This is the
   precedence proof: same flag, two sources, only the fragment's grant
   survives.
5. `a_hands_work_seat_is_admitted_under_harness_by_its_work_fragment_alone`
   — `smith-hands` (chain `["astra"]`) against the codex fixture declaring
   both members, compiled under `Boundary::Harness`: success;
   `bundle.manifest["boundary"] == {"work": "harness"}`; the candidate
   argv carries **no** workspace fragment (compose runs unboxed: the
   `hands.workspace` tokens are absent, because under harness
   `compose` appends nothing); the recorded hands spec is still in
   `bundle.manifest["hands"]` — recorded, but Brokkr enforces none of it
   here. The launch half calls `crate::engine::compose_site(
   BuiltBoundary::Harness, SeatClass::Work, expand_command(dir,
   &candidate.argv), Some(&spec), Some(&candidate), workdir, &roots,
   "/r/p.json", None)` and asserts the argv gained exactly
   `["--sandbox","workspace-write"]`, with no `mcp_servers`, no
   `--mcp-config`, no `--allowedTools` and no unexpanded `{…}` token
   anywhere: admission does not imply a Brokkr box.
6. `a_missing_or_unsupported_harness_work_member_keeps_its_reason_bearing_refusal`
   — the same fixture with `hands.harness.work` absent, then explicitly
   `{"unsupported": "<reason>"}`: both refuse under harness naming seat,
   link, provider, `hands.harness.work`, the writable-sandbox sentence and
   `(decision 0046 rulings 1 and 4)`; the measured variant carries
   `(<reason>)`; workspace support does not satisfy the work member.
7. `the_shipped_engine_smiths_claude_fallback_refuses_a_harness_realm_on_the_whole_chain`
   — `Fixture::compile_roots(boxed_seat("implementer-engine","work"),
   <shipped agents/>, <shipped adapters/>, Boundary::Harness)` (the
   pattern of `a_measured_claude_gap_is_reported_not_papered_over`, which
   seats the shipped chief the same way). The minimal fixture bundle
   avoids unrelated earlier refusals (no dialect steps). The refusal is
   asserted verbatim:
   `seat 'work' link 2 resolves to provider 'claude', which declares no
   \`hands.harness.work\` fragment: a capability gap — under the
   \`harness\` boundary a work seat with hands writes the tree only under
   the harness's own writable sandbox as the adapter addresses it
   (decision 0046 rulings 1 and 4)`, plus
   `!refusal.contains("codex")` — link 1 (astra/codex, which **does**
   declare `work`) alone would compile; the whole-chain rule is what
   refuses. No guessed claude fragment, no fallback omission, no combined
   launch is introduced to make it pass.

Rejected alternatives: putting these in `agents/tests.rs` (its `Tree`
proves the resolver but cannot name a **seat** — the seat wrapper lives in
`resolve_reference` — and the spec requires the seat in the diagnostic);
using the shipped `adapters/codex.json` for the verbatim refusal (its
`tool_permissions` carries a measured reason, so the refusal would read
`unsupported (codex-cli 0.148.0 restricts by sandbox CLASS, …)` — the
spec's word-for-word sentence is the bare form; the shipped file's shape
stays proven by tests 7–9 and the existing `a_measured_gap_…` unit test);
a new integration-test file (house rule: extend the suite that already
proves the law); asserting `is_err()` anywhere (spec forbids).

### D4 — The shipped launch composition is proved in `engine/boundary_tests.rs` against independent literal expectations

Home: `boundary_tests.rs` is `compose_site`'s own suite and already
imports `Library`, `Adapters`, `Availability`, `Candidate` and the
composition helpers; `agents/tests.rs::the_shipped_adapters_declare_their_harness_as_the_record_says`
is the precedent for reading shipped data from a unit suite. Two tests,
sharing one setup: `Library::load(root/agents)` +
`Adapters::load(root/adapters)` (root = workspace via
`CARGO_MANIFEST_DIR/../..`), `resolve_agent(…, "implementer-engine")`
(the namespace-default resolution `Bundle::compile` uses), a
`tempfile::tempdir()` whose path is **canonicalized** (macOS `/private`
symlink; decision 0063 hosts only), a result-path string, and per
candidate: `bundle::expand_command(dir, &candidate.argv)` then
`compose_site(BuiltBoundary::Namespace, SeatClass::Work, …, Some(&spec),
Some(&candidate), workdir, &[], result_path, None)` — the production path
token for token.

Independence rule (spec): expectations are literal strings plus the two
runtime facts (`EXE` = `std::env::current_exe()`, `W` = canonical workdir).
The tests must **not** call `hands::serve_args`, `hands::mcp_config`,
`hands_command` or `HandsSpec::to_value` to build an expectation — no
comparing a helper to itself. The literal spec JSON, in serde_json's
sorted-key order, is:

```text
S = {"binds":[{"mask":["credentials.toml","credentials"],"mode":"overlay","path":"~/.cargo"},{"mask":[],"mode":"ro","path":"~/.rustup"}],"kind":"workspace","network":false
```

8. `the_shipped_engine_smiths_codex_launch_is_exactly_the_declared_hands_route`
   — candidate 0 is `provider "codex"`, `model "astra"`,
   `effort Some("high")`; the composed argv equals, element for element:
   `[EXE, "driver", "codex", "--", "--model", "gpt-6-astra", "--effort",
   "high", "--sandbox", "read-only", "-c",
   "mcp_servers.brokkr.command=\"EXE\"", "-c",
   "mcp_servers.brokkr.args=T", "-c",
   "mcp_servers.brokkr.default_tools_approval_mode=\"approve\""]` where
   `T = ["hands","serve","--workdir","W","--spec","S-escaped"]` is the
   TOML array literal (each element double-quoted, `\` and `"` escaped —
   so `S` appears inside `T` with every `"` spelled `\"`). Decoded
   assertions: strip the `mcp_servers.brokkr.args=` prefix, compare the
   array's quoted elements to `["hands","serve","--workdir",W,"--spec",S]`
   — the server receives the canonical workdir and the complete declared
   policy (network false, both toolchain binds, both Cargo masks).
   Absences: no token contains `workspace-write`, `--allowedTools`,
   `Bash(`, `{hands_args_toml}`, `{hands_mcp_json}`, `{brokkr}` or
   `{result_path}`; the native sandbox pair is exactly
   `("--sandbox","read-only")` from `hands.workspace`; the shipped
   approval-mode `-c` assignment is present. `spawn.env ==
   SpawnEnv::Inherit`, `spawn.rewalk.is_none()` — the namespace arm's
   existing contract.
9. `the_shipped_engine_smiths_claude_launch_preserves_its_complete_workspace_fragment`
   — candidate 1 is `provider "claude"`, `model "fable"`,
   `effort Some("high")`; the composed argv equals
   `[EXE, "driver", "claude", "--", "--permission-mode", "acceptEdits",
   "--model", "claude-fable-5-1", "--effort", "high", "--tools", "",
   "--strict-mcp-config", "--mcp-config", M, "--allowedTools",
   "mcp__brokkr__workspace"]` where `M` is the literal
   `{"mcpServers":{"brokkr":{"args":["hands","serve","--workdir","W","--spec","S"],"command":"EXE"}}}`
   (asserted by `serde_json::from_str` on the token against a `json!`
   value built from `EXE`, `W` and the literal `S` text — order-insensitive
   equality, content-independent of the helper). `--allowedTools` occurs
   exactly once, granting only `mcp__brokkr__workspace`; no argument
   contains `Bash(cargo:*)` or `Bash(git:*)`; the driver's existing
   `--permission-mode acceptEdits` prefix is part of the preserved shipped
   bytes and stays (the fragment's `--tools ""` empties the native tool
   surface it addresses); no harness fragment, no unexpanded token.
   Preserving this fragment requires **no adapter edit** — the test reads
   the shipped file.

Both tests run without a provider, network or nested namespace:
`compose_site` is pure argv composition, and the box itself is never
built (the `bwrap` execution path stays the protocol crate's own,
already-tested territory — the workdir-as-writable-workspace fact below
the serve line is `box_argv`'s existing `--bind workdir` and is not
re-proved here).

Rejected alternatives: asserting `compose_site(…) == hands_command(…)`
(the tautology the spec forbids — the existing `compose_site_follows_…`
test keeps that shape check; these two add content); driving a real
`Engine::start` (needs a store, a dispatch and a spawn — the deterministic
composition is the pure function the argv tests read directly, per
`compose_site`'s own doc); placing the tests in `tests/` as an
integration file (the unit module already holds every import and the
shipped-data precedent; integration placement would duplicate helpers).

### D5 — Shipped-data pins: declaration facts in `library_data.rs`, resolution facts in `adoption.rs`

10. `tests/library_data.rs::the_engine_smith_declares_the_ruled_chain_and_hands_and_no_tools`
    — reads the shipped JSON (via `Library::load` and the raw file):
    `models == ["astra","fable"]`; `efforts == {"astra":"high",
    "fable":"high"}`; `tools` key absent; `hands` parses to network false
    with exactly the two binds (`~/.cargo` overlay with the two masks,
    `~/.rustup` ro, no third bind, no `rw` bind); every bind path starts
    `~/` (no machine-specific checkout, no absolute host path); the source
    contains no `boundary` key; `charter == "charters/implementer.md"`;
    limits preserved. Then resolves under namespace
    (`resolve_agent`) and asserts both candidates: chosen 0 = codex/astra,
    link 2 = claude/fable, both effort high; the codex argv has no
    `--allowedTools` token; the claude argv has exactly one, granting
    `mcp__brokkr__workspace`; neither argv contains `Bash(cargo:*)` or
    `Bash(git:*)`; both fragments match their adapters' declared
    `hands.workspace` vectors exactly (A1 at the resolution level, before
    spawn expansion).
11. `tests/adoption.rs` — extend `TRIAGE` with
    `("implement:engine", "gpt-6-astra", <implementer.md digest
    b750b0a4…>)`, and teach `expected_argv` that `implement:engine` takes
    the **hands** fragments (the codex/claude arms already exist for the
    review sites) instead of falling into the `implement:`-prefixed tools
    branch. Effort falls out of the existing `_ => "high"` arm. This pins
    the selected candidate's complete compiled argv (element-for-element,
    the file's existing discipline) and the charter digest.
12. `tests/adoption.rs::the_engine_case_pins_both_ruled_hires_under_namespace_hands`
    (new, compiling `recipes/triage` once) — asserts the four manifest
    facts the spec's first scenario names:
    `manifest["select"]["implement"]["engine"] ==
    {"agent":"implementer-engine","candidates":[{"provider":"codex",
    "model":"astra","effort":"high"},{"provider":"claude","model":"fable",
    "effort":"high"}]}`;
    `manifest["agents"]["implement:engine"]` carries
    `chain ["astra","fable"]`, `chosen_index 0`, `model "astra"`,
    `provider "codex"`, and an `adapter_digest` over both consulted
    adapters (codex joins claude);
    `manifest["hands"]["implement:engine"]` is the declared spec (network
    false, exactly the two binds);
    `manifest["boundary"]["implement:engine"] == "namespace"` and
    `bundle.boundary == Boundary::Namespace`.

Unchanged and deliberately not weakened: `roster.rs` (all three cited
rules pass on the new declaration — D1 shows why), `gpt_flash_shape.rs`
(the scoped crew and `drivers` parity are untouched),
`crucible_review_sequence.rs` (`implement:engine` keeps its `agents`
record), `is_house_tool_grant`'s now-inert `("implementer-engine",
"cargo" | "git")` arm — kept, because the map is the accounting of house
grants a charter may not name, `implementer`/`implementer-sdd` still use
the shared arms, and two independent pins (`hands ⇒ no tools`, codex-lane
⇒ no tools) already refuse any route back to a dead or inexpressible
list. Deleting the arm was weighed and rejected as churn on a rule table
the spec requires to keep passing as-is.

### D6 — Exactly one digest moves: `recipes/triage`, in both pin collections

Why triage moves — one declaration, three manifest facts (all inside
`manifest_for`'s inputs): (1) the `agents["implement:engine"]` record
moves: new agent bytes (new `agent_digest`), `adapter_digest` now hashing
`{"claude":…,"codex":…}` instead of `{"claude":…}`, `chain
["astra","fable"]`, `model "astra"`, `provider "codex"`; (2) the
`select["implement"]` record's engine case moves: candidates
codex/astra/high + claude/fable/high replace claude/fable/high +
claude/opus/high; (3) `hands` and `boundary` each gain the
`implement:engine` entry (written in one loop, key sets equal by
construction, still run-manifest v9 — the schema is untouched).

Why nothing else moves:

- The other nine witnesses (`fast`, `node`, `preflight`, `night-shift`,
  `wager-harness`, `research`, `research-dsh`, `gpt-flash`,
  `bundles/verify`) resolve no changed byte. `night-shift` overrides
  `implement` with its inline dsh lane; `gpt-flash` overrides it with the
  scoped crew; both still resolve the rest of triage's offices, whose
  agents and adapters are untouched.
- The four `UNCOMPOSED` compose pins (`fast`, `panel-review`, `self`,
  `verify`) consult the smith never.
- `night-shift`'s and `gpt-flash`'s `@compose/0000/triage` entries stay
  byte-identical: an ancestor's digest covers only that layer's own file
  bytes and its own ancestors — "never the leaf's agent resolution or the
  adapter declarations that authorised its gates" (`compose.rs::resolve`,
  ~817–853). Agent-library bytes are not triage-layer files. This is the
  difference from the #303 move, where the triage **charter** lived in the
  layer and did ride every descendant.

Work items: update the `recipes/triage` pin in `tests/witness_digests.rs`
(WITNESSES, currently `d95b41d9…`) and the same value in
`bundle/compose_tests.rs::a_composed_bundles_manifest_is_pinned` — both
from the tests' reported left/right pairs after the declaration edit,
never guessed or recomputed by hand (the witness file's own header rule).
Append one history entry to each doc-comment block, in the established
voice, attributing the move to issue #307: the operator's Astra/Fable
hire, its effort pins and boxed hands replace the retired tool list, so
the recorded agent, select, hands and boundary identity of the one hiring
bundle moves; the entry states explicitly that night-shift and gpt-flash
keep their pins because both override the seat and an ancestor digest
covers layer bytes only. No old history is replaced. Additionally,
strengthen `a_composed_bundles_manifest_is_pinned`: its current
`.is_some()` check on night-shift's `@compose/0000/triage` becomes an
equality pin against a measured constant, asserted for **both**
descendants (one shared constant — both extend triage directly), with a
comment saying the constant is measured before the agent edit and must
survive it. That makes "unaffected pins SHALL remain unchanged" a
positive assertion on the ancestor entries the spec names, rather than an
inference from the descendants' overall witness pins.

Rejected alternatives: re-pinning all ten witnesses "to be safe"
(forbidden — unaffected pins must not change, and a moved pin without a
moved manifest is a guessed hash); regenerating the rendered
`brokkr recipes list` fence in `docs/guides/recipe-authoring.md` (its
triage digest prefix `2bc18b05f87a` was already stale before this change —
it predates even `d95b41d9…` — and the fences are historical captures no
test compares; see Open Questions); bumping `ENGINE_VERSION` (no engine
change).

### D7 — Documentation: one guide clarification, one dated note, no new decision

`docs/guides/provider-adapters.md`, "## Hands" section:

- Immediately after the paragraph opening "The `workspace` fragment is
  what a site with hands runs under the `namespace` boundary" — that is,
  directly before the `### hands.harness` heading — add a paragraph
  stating, concretely: a provider without a native per-tool
  flag serves a restricted work seat through this declaration — the
  engine smith (issue #307) hires `astra` on codex, whose adapter measures
  `tool_permissions` unsupported, and compiles under `namespace` because
  the agent declares workspace hands; codex's own sandbox stays
  `read-only` and every writable action goes through the
  `brokkr hands serve` MCP tool inside the box. The confinement is
  decision 0043's filesystem and network boundary — what a command can
  touch — never a list of command names: inside the box the model may run
  any shell command; what it can reach is the worktree, the masked Cargo
  overlay and the read-only toolchain. 0043's limits stand verbatim: the
  native shell remains available read-only outside the box, host-read
  secrecy is not promised, and provider traffic rides the harness's
  credential and network outside the box. The `hands.harness.work` route
  keeps its unboxed 0046 meaning and its per-link admission — the two
  paths are never parts of one launch.
- Coherence fix in the **claude** paragraph (~322–330): "every hands agent
  chains `opus`" becomes false the moment the smith chains `fable` with
  hands, and the tool-grant clause describes a seat that no longer
  carries a tool list. Minimal edit: the parenthetical becomes a true
  statement of which chains reach claude (the review offices' and the
  chief's `opus`/`fable` lanes, the release manager's `opus`, and — since
  issue #307 — the engine smith's `fable` fallback), keeping every other
  fact: the measurement is the operator's, against the installed 2.1.x
  line; until recorded, every shipped bundle whose hands chain reaches
  claude refuses under `harness`; the record of which is the pin test in
  `model_policy_tests.rs`. No other sentence in the guide changes.

`docs/decisions/0043-the-hands-are-one-tool.md` — append a dated note
(new trailing section, e.g. `## Note — 2026-09-21 (issue #307)`),
changing neither the status line (`Status: accepted (operator ruled in
chat, 2026-09-03)`) nor any historical text. The note records, in this
order: (a) the operator's 2026-09-20/21 rulings — the engine smith hires
astra then fable at high effort, and the restriction it owes is this
decision's existing workspace confinement (rulings 1 and 2), not the
`["cargo","git"]` allow-list the seat previously declared; (b) the
earlier commission's allow-list-enforcement claim is **withdrawn** — no
new per-tool enforcement contract, command parsing, transport semantics or
native-tool bypass rule is designed or accepted by this change; (c) the
boxed smith composes the existing `hands.workspace` fragment, and decision
0046's separate `hands.harness.work` path stays unboxed and separately
admitted; (d) deterministic compilation and composition evidence proves
the ruling **expressible**, not live — the controller's first live Astra
implementation (Cargo and Git through the boxed hands, a real commit,
verify passing) remains pending and is recorded as such. The note applies
existing rulings; it creates and accepts no new semantic rule, which is
why no `proposed` decision document accompanies this change (house rule:
a proposed decision is for a **semantic** change; proposal D4 rules this
one an enactment).

Alternatives rejected: authoring a new decision document (nothing semantic
changes; 0043/0046 already carry the law; a new number would imply a new
rule); editing 0043's rulings or 0045's roster table in place (historical
facts are not replaced — 0045's table records the roster **as ruled on
2026-09-05**, and this change's own history entry in the pin files is
where the new hire is attributed); rewriting the guide's measured-gap
prose (the codex `tool_permissions` reason stays the adapter's measured
words).

### D8 — The record artifacts get one durable text pin

13. `tests/library_data.rs::the_guide_and_the_0043_note_record_the_declared_confinement`
    — the crucible pattern (prose matched with line breaks collapsed, so
    a rewrap cannot fail it but a deleted clause must): the guide contains
    the confinement-is-not-a-command-allow-list statement, the
    read-only-native-sandbox-under-namespace statement, and 0043's three
    preserved limits; the note exists dated 2026-09-21, names issue #307,
    states the withdrawal, states that no new per-tool contract is
    designed, distinguishes expressibility from the pending live
    measurement, and the file's status line and `## Rulings` heading are
    unchanged. Placement: this suite already pins the shipped agents' and
    adapters' bytes; the guide paragraph and the note are the shipped
    record of the same declaration. A separate new test file was rejected
    (one test does not justify a new suite; the house rule is to extend).

### D9 — Removal evidence: one mutation per protection, mapped to the test it must break

Each mutation is applied to the working tree, the targeted test is run and
must fail **on the assertion that names the claimed protection** (a
compile failure from a broken mutation, an unrelated refusal, a
fixture-only change, or a comment does not count), then the mutation is
restored (`git checkout --` / re-apply the pinned value) and the targeted
test rerun green. Every entry — mutation, test, failing assertion,
restoration, passing rerun — is recorded in the delivery record. Matrix:

| # | Mutation (temporary) | Targeted test / assertion that must fail |
|---|---|---|
| M1 | `agents.rs::compose`: consult `allow` even when hands exist (drop the `else`) | 4, 9: `Bash(cargo:*)`/second `--allowedTools` appears → absence assertions fail |
| M2 | `agents.rs::compose`: silently skip an unsupported `tool_permissions` instead of refusing | 1: compile succeeds → the expected refusal text is absent |
| M3 | `agents.rs::compose`: admit boxed hands with no `adapter.hands` (empty fragment) | 3: compiles → hands-unsupported refusal absent; 2: fragment assertion fails |
| M4 | `bundle.rs::enforce_hands_boundary`: admit a harness work seat with no `work` member | 6, 7: compile succeeds → both refusals absent |
| M5 | `engine.rs::compose_site` Namespace arm: append `candidate.harness.work` too | 8: `workspace-write` absence fails; existing `compose_site_follows_…` also fails |
| M6 | `adapters/codex.json`: `hands.workspace` `read-only` → `workspace-write` | 8: exact-sandbox assertion fails; 11 (adoption's shipped-argv pin) also fails |
| M7 | `adapters/codex.json`: delete the `mcp_servers.brokkr.args=-c` pair | 8: decoded server-args assertion fails |
| M8 | `adapters/claude.json`: delete `--allowedTools mcp__brokkr__workspace` from the fragment | 9: fragment-equality and exactly-one-grant presence assertions fail (loss of the required grant, per proposal D5) |
| M9 | `adapters/claude.json`: append `Bash(cargo:*),Bash(git:*)` to the fragment's grant | 9, 10: retired-grant absence assertions fail |
| M10 | `agents/implementer-engine.json`: `network` → `true` | 10, 12, 8/9 decoded-spec assertions fail |
| M11 | `agents/implementer-engine.json`: drop one mask / add an `rw` bind | 8, 9, 10, 12: spec-literal and exactly-two-binds assertions fail |
| M12 | `agents/implementer-engine.json`: re-add `tools` beside hands | `roster.rs` dead-tools-beside-hands and codex-lane pins fail |
| M13 | `agents/implementer-engine.json`: swap chain to `["fable","astra"]` | 12: chosen provider/model and candidate-order assertions fail |
| M14 | `engine.rs::hands_command`: leave `{hands_args_toml}`/`{hands_mcp_json}` unexpanded | 8, 9: unexpanded-placeholder absence assertions fail |

M6–M11 mutate shipped data temporarily; the restored state is verified by
`git diff --exit-code` on those paths plus the witness pins passing at
their (updated) values. No mutation is committed; no production rule is
weakened to improve coverage or admit the roster (spec).

### D10 — Evidence boundary: what this change may claim, and what stays pending

The delivery record for the implementation phase states, in words the
result notes carry (never instructions to a gate):

- **Expressible, not live.** Compilation, launch inspection, fixtures and
  removal proofs show the ruled hire composes under decision 0043's
  existing confinement. They are not a live Astra smith. The controller's
  first live measurement — Cargo and Git through Codex's boxed hands, a
  real commit, verify passing — is named pending.
- **Pending, never lowered:** `bash scripts/coverage-exact.sh` (external
  host/CI, outside the workspace box, literal 100% threshold — this change
  plans zero production lines, so the gate's input set is unchanged, but
  the external result must still exist before tagging); remote CI on the
  final head; the claude `hands.harness.work` measurement (operator's);
  the codex `hands.gate` capture measurement (guide's existing pending
  note). Anything the implementing box cannot run (namespace nesting,
  network) is recorded as pending evidence, not as a pass or a skip
  dressed as one.
- **Available before reporting success:** `cargo fmt --all -- --check`;
  `cargo clippy --workspace --all-targets --all-features --locked --
  -D warnings`; every crate's suite separately and
  `cargo test --workspace`; `cargo run --locked -p brokkr-cli -- compile
  --bundle bundles/self` (and `bundles/verify`); strict OpenSpec
  validation of this change; all removal-evidence entries (D9).
- Hosts are Linux and macOS (decision 0063): every temporary root is
  canonicalized, no test adds a Windows obligation, and no test creates a
  namespace (composition is pure argv; the box stays the protocol crate's
  already-gated territory).

## How each requirement is proved

Test numbers are D3–D5's; mutation numbers are D9's. "Verbatim" texts live
in unchanged production code and are asserted as literal substrings.

| Requirement (spec) | Proof | Removal evidence |
|---|---|---|
| astra R1, scenario "resolves both ruled hires" | 12 (select + agents + hands + boundary records of compiled `recipes/triage`), 11 (selected candidate's complete argv + charter digest), 10 (declaration + both resolved candidates) | M13 (chain swap breaks chosen-provider/order), M10/M11 (hands facts), M12 (roster rules bite) |
| astra R1, scenario "writable project access uses the existing workspace mount" | 8/9 (decoded serve arguments: `--workdir W` + the complete spec; the rw workdir bind below the serve line is `box_argv`'s existing, separately tested behavior — unchanged), 10 (no checkout path, no extra writable bind, network false, no `boundary` field) | M7 (registration/args lost), M14 (placeholders unexpanded), M10/M11 |
| astra R1, scenario A1 "inactive tools removed, workspace grant remains" | 10 (no `tools` key; codex argv without a per-tool flag; claude argv with exactly one `--allowedTools mcp__brokkr__workspace`; no `Bash(…)` anywhere), 4 (fixture precedence with one flag spelling on both sources), 8/9 (shipped composed launches) | M1 (consulting the retired list adds `Bash` grants / a second `--allowedTools`), M8 (losing the MCP grant fails its presence assertion), M9 (adding a retired grant fails the absence assertions) |
| astra R2, scenario "a hiring bundle changes its identity" | D6: the triage pin in `witness_digests.rs` and `compose_tests.rs`, both updated only from reported left/right pairs, both history blocks attributing the move to #307; the strengthened `@compose/0000/triage` constant proves the ancestors did not move | M10/M13 (any further declaration edit moves triage again and fails the pins — digest sensitivity is the standing proof) |
| astra R2, scenario "independent rosters retain their guarantees" | the unchanged `roster.rs` suite (panel diversity, dead-tools rule, codex-lane rule, effort-never-rises), the unchanged `gpt_flash_shape` suite, and the whole `brokkr-runtime` suite green | M12 |
| astra R3, all three scenarios | D7's guide paragraph and dated note, pinned durably by 13 (confinement-is-not-an-allow-list, read-only native sandbox, 0043's three limits, withdrawal, expressible-vs-live, unchanged status line) | Doc-clause deletion fails 13 (beyond B-R5's required compile/composition set; recorded for completeness) |
| astra R4, both scenarios | D10's delivery-record obligations: the notes say "expressible, not proved live", name the controller's Cargo/Git/commit/verify measurement as pending, and list each quality gate as available-or-pending at the literal threshold. Review-checked record text; no test may claim a live smith | n/a (a record obligation, not a code protection) |
| boxed R1 "exact refusal without hands" | 1 + variants: verbatim `the provider declares tool_permissions unsupported, so the agent's restriction to ["cargo", "git"] cannot be expressed and the agent would run with MORE power than it declares`, seat/agent/provider/model named, trailing capability explanation intact, measured-reason shape preserved; the existing `agents/tests.rs` pins (`a_restriction_the_provider_cannot_express_is_a_hard_failure`, `a_measured_gap_refuses_exactly_as_a_bare_unsupported_does`) stay green untouched | M2 (silently skipping the gap loses the refusal) |
| boxed R2 "hands replace the list only through declared workspace support" | 2 (admission: recorded hands, namespace word, complete fragment, no tool-list flag), 3 (refusals: verbatim `the provider declares hands unsupported` + `so the agent's hands cannot be put in the box and the agent would run with the harness's own tools`, measured reason preserved, load-refusal of the harness-only shape), 4 (precedence keeps the MCP grant and drops the `Bash` grants) | M1, M3 |
| boxed R3 "harness work is the separate whole-chain case" | 5 (admission by `work` alone: harness boundary word, recorded-but-unenforced hands, launch with `--sandbox workspace-write` and no MCP/tool-list token), 6 (missing and measured-unsupported refusals: verbatim `…declares no \`hands.harness.work\` fragment…: a capability gap — …(decision 0046 rulings 1 and 4)`), 7 (shipped smith refuses on link 2 `claude`, link 1 unnamed), plus the existing hands-law pins and the two shipped harness pins in `model_policy_tests.rs` staying green untouched | M4 (dropping the work requirement), M5 (namespace borrowing the work fragment) |
| boxed R4 "the shipped namespace launch is tested as composed" | 8 and 9: production path (`resolve_agent` → `expand_command` → `compose_site`), canonicalized tempdir, complete literal expectations for argv, `T`, `M` and `S`, every absence the scenarios list | M6, M7, M8, M9, M14, M5 |
| boxed R5 "removal evidence for every protection" | D9's matrix, executed and recorded entry by entry in the delivery record: mutation, targeted test, the assertion that failed for the claimed reason, restoration, passing rerun; no `is_err()`-style assertion anywhere in 1–9; positive tests assert concrete compiled/composed facts | the matrix itself |

## Risks / Trade-offs

- **Digest guessing.** The single highest-risk mechanical step is
  transcribing the new triage digest. Mitigation: pins are updated only
  from the failing tests' reported left/right pairs, both files carry the
  same value, and the strengthened `@compose/0000/triage` constant is
  measured **before** the agent edit and asserted unchanged after.
  Accepted cost: two more constants to maintain in compose_tests.
- **Tautological launch proofs.** The temptation is to build expectations
  with `serve_args`/`mcp_config`. The D4 independence rule forbids it and
  spells the literal `S`, `T` and `M` shapes; the sorted-key spelling is a
  fact of serde_json without `preserve_order` (verified: the feature
  appears nowhere in the lockfile). If a future dependency enabled
  preserve_order the literals would break loudly, not silently — accepted.
- **The smith can run any command in the box.** Trading the
  `["cargo","git"]` list for the boundary widens the command surface and
  narrows the reach: this is 0043's original argument and the operator's
  ruling, recorded in the note. The design accepts it without adding
  compensating command filters (non-goal).
- **Cold caches without network.** `network: false` plus the overlay means
  a machine with an empty `~/.cargo` cannot fetch crates inside the box.
  Accepted: it fails loudly as a build error; the release-manager precedent
  for `network: true` is release work only, and granting it here would
  contradict the ruling.
- **Harness realms keep refusing the smith.** Until claude's `work` member
  is measured, a `harness` realm cannot seat the engine route — by design
  (test 7 pins it). Accepted: a guessed fragment would be exactly the
  "different bundle identity wearing the pinned digest" hazard 0046
  refuses.
- **Refusal-text coupling.** Tests 1–7 assert verbatim strings; any future
  rewording of a diagnostic now fails loudly in more places. Accepted as
  the spec's intent ("word for word"), and the strings live in unchanged
  production code, so this change introduces no drift itself.
- **Doc text pin brittleness.** Test 13 couples two prose artifacts to
  assertions. Mitigation: whitespace-collapsed `contains` matching on a
  handful of durable clauses (crucible precedent), not full-text pins.
- **Fixture provider named `codex`.** Slight tension with
  `model_policy_tests.rs`'s invented-names header; resolved by the D3
  header sentence — the engine still matches declarations, and the shipped
  file is untouched. Trade-off accepted for the spec's verbatim scenario.
- **Stale rendered fences** in `agent-library.md` (`implementer-engine
  fable → opus`) and `recipe-authoring.md` (old triage digest) become one
  more line staler. Accepted: they were already stale for unrelated
  agents and digests before this change, no test reads them, and the
  proposal's Impact does not include them; regenerating them here would
  mix unrelated drift into a ruled change. Recorded as a residual for a
  future docs sweep (Open Questions).

## Migration Plan

No data migration exists: journals are append-only history; a run started
under the old triage digest keeps its pinned identity, and decision
0021/0022's digest law already refuses to resume it against the new
compile — the intended semantics, not a new hazard. Contracts, the
run-manifest v9 shape, `policy/phase-machine.json` and `realms.json` are
untouched. Ordered implementation steps (for the tasks phase to slice):

1. Baseline: run `tests/witness_digests.rs`, `bundle/compose_tests.rs` and
   the runtime suite green on the untouched tree; record night-shift's and
   gpt-flash's current `@compose/0000/triage` constant (they must survive).
2. Edit `agents/implementer-engine.json` (D1). No other data file moves.
3. Run the two pin suites; take the new triage digest from the reported
   left/right pairs; update both pins and both history blocks (D6);
   strengthen the ancestor assertion; rerun green.
4. Add tests 1–7 (D3), 8–9 (D4), 10–12 (D5). All must pass with **zero**
   production edits; any failure is an exposed gap and triggers D2's
   contingency rule only.
5. Write the docs (D7), then test 13 (D8).
6. Full gates: fmt, clippy `-D warnings`, per-crate suites,
   `cargo test --workspace`, `compile --bundle bundles/self`,
   `compile --bundle bundles/verify`, strict OpenSpec validation of this
   change (`openspec validate 2026-09-21-307-astra-engine-smith --strict`
   and `--all`), `git diff --check`.
7. Removal-evidence pass (D9): each mutation, targeted failure, restore,
   green rerun — recorded entry by entry.
8. Delivery record (D10): expressible-vs-live wording, pending list,
   available evidence; commit in the repository's style; never push.

Rollback is a single-file revert of the agent declaration plus the pin
history entries; because no production code changes, nothing else can be
left behind.

## Open Questions

None blocking this design. Recorded residuals, each with an owner and no
claim of resolution:

1. **Controller's first live Astra smith** — Cargo and Git through the
   boxed hands, a real commit, verify passing. Owner: controller, after
   landing. Until then every record says "expressible", never "proved
   live" (D10).
2. **Claude `hands.harness.work` measurement** — owner: operator, against
   the installed 2.1.x line; until it lands, harness realms refuse the
   smith's Fable link, which test 7 pins and the guide's edited paragraph
   explains. No guessed fragment may pre-empt it.
3. **Stale rendered fences** — `docs/guides/agent-library.md`'s
   `brokkr agents list` capture (the smith's row, and rows already stale
   for `analyst`/`chief-architect` since decision 0045) and
   `docs/guides/recipe-authoring.md`'s `brokkr recipes list` capture (its
   triage digest prefix predates at least the latest pin move). Owner: a
   future docs sweep the operator may commission; out of this change's
   Impact by the proposal's own scoping, and no test reads either fence.
4. **Exact-coverage external result** — owner: host/CI outside the
   workspace box; recorded pending in the delivery record until it exists,
   at the literal threshold (release configuration; D10).
