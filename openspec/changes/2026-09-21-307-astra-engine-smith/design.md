## Context

Status: proposed. Change: `2026-09-21-307-astra-engine-smith`.

This design adopts the committed proposal and both capability deltas
(`specs/astra-engine-smith/spec.md`,
`specs/boxed-work-provider-admission/spec.md`) unchanged. It is the design
phase's only artifact and the only file this seat commits. Neither the
proposal nor either spec is edited here; nothing in them was found to be
undesignable, so no `upstream` return is made.

The operator's rulings this design is bound by, as the proposal records them:
the engine smith's chain is `astra` then `fable`; the restriction this change
owes is decision 0043's existing workspace confinement, not a per-tool
allow-list, and no new per-tool enforcement contract is designed here; the
boxed smith uses `hands.workspace` alone and `hands.harness.work` stays the
separate unboxed path; and an agent carrying a tool list on a provider that
cannot express one, with no hands, stays refused word for word.

### The code as it stands

Read before deciding; line numbers are this tree's (base `5ef97115`).

- `crates/brokkr-runtime/src/agents.rs:714` — `compose`. Model flag, then the
  effort pin, then one three-armed decision: `agent.hands.is_some() && boxed`
  appends `adapter.hands` (`hands.workspace`) or refuses at `:804`; `else if
  let Some(allow)` maps the tool list or refuses at `:823`; MCP needs last.
  The `else if` is load-bearing — an agent with hands never consults
  `tools.allow` on *either* boundary, so a declared list beside hands is dead
  data rather than a second grant.
- `crates/brokkr-runtime/src/agents.rs:1004` — `resolve_report`. Every entry's
  gap refuses, not just the chosen one; the record pins the agent digest, the
  charter digest and a digest over *every* consulted adapter.
- `crates/brokkr-runtime/src/agents.rs:945`/`:962` — `report`/`report_under`.
  `report` is `report_under(Namespace)`; `report_under` and `resolve_report`
  are `pub(crate)`, so a harness-boundary resolution can only be written from
  inside the crate.
- `crates/brokkr-runtime/src/bundle.rs:2085` and `:2124` — the two places a
  resolver error gains its `seat '<name>': ` prefix. A refusal that must name
  the seat has to come from a *compile*, never from `resolve` alone.
- `crates/brokkr-runtime/src/bundle.rs:2086-2120` — D33: for a hands agent
  whose every link maps, `enforce_model_policy` (and through it the hands law)
  judges *before* `resolve_report` reports a capability gap.
- `crates/brokkr-runtime/src/bundle.rs:2610` — `enforce_hands_boundary`.
  Returns at once when the site has no hands or the boundary is boxed; under
  `harness` a work site needs `hands.harness.work` on **every** link
  (`:2682`), with the measured reason appended.
- `crates/brokkr-runtime/src/engine.rs:4270` — `compose_site`. The `Namespace`
  arm calls `hands_command` and never reads `candidate.harness`; the `Harness`
  arm appends the class-selected harness fragment with `{result_path}` and
  `{brokkr}` expanded and serves no MCP.
- `crates/brokkr-runtime/src/engine.rs:4509` — `hands_command`. For a model
  site it expands `{hands_mcp_json}`, `{hands_args_toml}` and `{brokkr}`
  *inside* tokens (`part.replace`), which is why codex's
  `mcp_servers.brokkr.command="{brokkr}"` survives `expand_command`
  (`bundle.rs:3678`, which only replaces a token equal to `{brokkr}`).
- `crates/brokkr-protocol/src/hands.rs:127`, `:1217`, `:1229` — `HandsSpec`
  (network + binds), `mcp_config`, `serve_args`. `HandsSpec::parse` refuses a
  `boundary` key by name (`BOUNDARY_IS_THE_REALMS`) and the vocabulary is
  closed to `kind`, `network`, `binds`.
- `adapters/codex.json` — `tool_permissions` unsupported with a measured
  reason; `hands.workspace` = `--sandbox read-only` plus three `-c` pairs
  including `default_tools_approval_mode="approve"`; `hands.harness.work` =
  `--sandbox workspace-write`; `models.astra` = `gpt-6-astra`.
- `adapters/claude.json` — `tool_permissions` with the `cargo`/`git` map;
  `hands.workspace` = `--tools`, `""`, `--strict-mcp-config`, `--mcp-config`,
  `{hands_mcp_json}`, `--allowedTools`, `mcp__brokkr__workspace`; **no**
  `hands.harness` member at all; `models.fable` = `claude-fable-5-1`.
- `agents/implementer-engine.json` — today `["fable","opus"]`, both `high`,
  `tools.allow: ["cargo","git"]`, no hands.
- `agents/reviewer.json` — the shipped precedent for the two toolchain binds.
- Decisions [0043](../../../docs/decisions/0043-the-hands-are-one-tool.md)
  (rulings 1, 2, 5, 7 and the codex consequence) and
  [0046](../../../docs/decisions/0046-the-boundary-is-named.md) (rulings 1 and
  4). Hosts are Linux and macOS only, decision 0063.

No cargo command was run: this seat's box has no network and a design needs
none. Every claim below is a source claim, cited to the file and function that
carries it; each is re-proved by a named test in the implementation phase.

## Goals / Non-Goals

**Goals.**

1. Seat Astra then Fable on `implementer-engine` under decision 0043's
   existing workspace confinement, declared so an operator can read it.
2. Prove, deterministically, the existing compile outcomes and the two launch
   compositions the roster now depends on, each with removal evidence and each
   asserting reason text rather than `is_err()`.
3. Move exactly the manifest identities the declaration actually moves, each
   with a measured digest and a history line saying why.
4. Leave the guide and decision 0043 describing the confinement that is
   actually declared, and state the evidence boundary honestly.

**Non-Goals.**

- No per-tool enforcement contract, command allow-list, shell-command policy,
  transport semantics or native-tool bypass prevention (proposal D1).
- No adapter capability edit, no protocol change, no boundary vocabulary
  change, no new contract version, no dependency.
- No new semantic decision: this enacts 0043 and 0046. The house rule that a
  semantic change carries a `proposed` decision does not fire, and 0043's
  status stays `accepted` (proposal D4).
- No live Astra run, no publication, no workflow-runner invocation.
- No change to `contracts/`, `policy/phase-machine.json`, `policy/schemas/`,
  `fixtures/`, `reference/`, or the GPT/Flash scoped roster.

## Decisions

### D1 — The declaration: the ruled hire, the existing box, and nothing else

`agents/implementer-engine.json` becomes exactly:

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
      {"path": "~/.cargo", "mode": "overlay", "mask": ["credentials.toml", "credentials"]},
      {"path": "~/.rustup", "mode": "ro"}
    ]
  },
  "limits": {"max_attempts": 2, "timeout_seconds": 7200}
}
```

Description, charter and limits are untouched; `tools` is removed; no
`boundary` key is added and none may be (`HandsSpec::parse` refuses it by
name); no writable bind is added.

Why each part, and what was rejected:

- **`["astra","fable"]` at `high`/`high`.** Codex maps `astra` and declares
  `--effort` with `high` in its vocabulary; Claude maps `fable` likewise. The
  fallback-effort invariant in
  `tests/roster.rs::tool_grants_keep_house_tools_explicit_and_effort_never_rises_on_fallback`
  holds because `high >= high`.
- **Remove `tools` rather than keep it "for the record".** Rejected keeping
  it: `compose`'s `else if` makes it dead on both providers, and the roster
  test already refuses dead tools beside hands (`roster.rs:193`). Keeping it
  would be a Claude-only phantom grant that reads as a second restriction and
  is none. Decision 0043 ruling 5 kept the review agents' lists for the record
  on 2026-09-03; the ruling-2 review correction recorded in the witness
  history reversed that, so this follows the current rule, not the old one.
  The now-unreachable `("implementer-engine", "cargo" | "git")` arm of
  `tests/roster.rs::is_house_tool_grant` is removed with it, so re-adding a
  tool list to this office fails twice rather than once.
- **`network: false`.** The four boxed work offices (`analyst`, `clarifier`,
  `chief-architect`, `intake-sdd`) declare it. Rejected the release manager's
  `network: true` plus resolver/CA binds as precedent: that grant exists to
  publish, and an engine smith that can reach the network out of the box
  widens the very confinement this change is supposed to be declaring.
- **The two toolchain binds, copied from `agents/reviewer.json`.** `~/.cargo`
  `overlay` with `credentials.toml` and `credentials` masked, `~/.rustup`
  `ro`. Rejected `rw` on `~/.cargo` — decision 0043's own first review found
  it a persistence path out of the box. Rejected adding a bind for the
  checkout: the box binds the workdir read-write already, so a checkout bind
  would be both redundant and machine-specific, and an agent file is copied
  into other realms.
- **No `boundary`.** Ruling 1 of 0046: the realm declares it. A key here is a
  load refusal, and the delta says the same.

**Trade-off accepted.** The smith becomes a boxed office, so decision 0043
ruling 7 refuses a run that hires it where `bwrap` is absent, and 0043's
Linux-only boundary applies. This costs no host that was not already lost:
`recipes/triage` and `recipes/night-shift` already hire boxed review offices
and `refuse_unboxable` refuses the whole run on any of them, so a macOS
operator is refused today for the same reason. No new macOS or Windows
obligation is created (decision 0063).

### D2 — No production Rust changes; the compiler already rules every outcome

Source inspection says the delta is data and prose only. Each required outcome
already has a named owner:

| Outcome | Owner | Result |
|---|---|---|
| tool list, no hands, codex | `agents.rs:823-840` | refusal, word for word |
| hands, namespace, `hands.workspace` present | `agents.rs:804-820` | fragment appended, list not consulted |
| hands, namespace, `hands` absent/unsupported | `agents.rs:804-818` | capability refusal naming the box |
| hands, harness, `hands.harness.work` present | `bundle.rs:2682` + `engine.rs:4300-4313` | admitted; `--sandbox workspace-write` appended |
| hands, harness, member missing on any link | `bundle.rs:2682` | refusal naming the link, provider, member, rulings 1 and 4 |

Three consequences the implementation must respect rather than rediscover:

1. **Seat naming comes from the compiler.** `resolve` names agent, provider
   and model; only `bundle.rs:2085`/`:2124` prefix `seat '<name>': `. Every
   spec scenario that requires the seat named is therefore a *compile* test in
   `bundle/model_policy_tests.rs`, not a resolver test.
2. **D33 ordering.** For a hands agent with every link mapped, the hands law
   runs first. Under `namespace` it returns immediately (boxed), so the
   missing-`hands.workspace` case surfaces as the resolver's capability
   refusal — which is what the delta demands ("a capability refusal, not a
   malformed-fixture or unrelated tier error"). The fixture must declare
   `trust_tier: trusted` so a tier refusal cannot stand in for it.
3. **Harness composition needs no suppression rule.** `compose` appends the
   workspace fragment only when `boxed`, and `compose_site`'s `Namespace` arm
   never reads `candidate.harness`. The two paths are mutually exclusive by
   construction, not by a check — which is what the "SHALL NOT be composed
   there" requirements need, and what their removal proofs mutate.

**Contingency, bounded in advance.** If a test written to this design fails,
the repair is the smallest edit consistent with 0043 and 0046, it preserves
both refusal strings byte for byte, and it does not widen an admission to make
a roster compile. Weakening a rule to admit the hire, or to raise coverage, is
refused; the alternative is reporting the gap. Any such repair adds production
lines and therefore new exact-coverage obligations — recorded, not waived.

Alternatives rejected: teaching `compose` a codex-specific mapping from
`tools.allow` onto `--sandbox` classes (0043 already rejected this as a
one-provider patch that puts a coarsening in an adapter); adding a
"hands imply these commands" check (unauthorized, and it is the withdrawn
premise); making `report_under`/`resolve_report` public so the harness proofs
could live in `tests/` (a public API widening bought for test convenience).

### D3 — Where each proof lives, and what it asserts

Test-only code, in files the coverage gate already excludes (`*_tests.rs`,
`tests/`). Fixtures are temporary trees; `fixtures/` is never touched.

**`crates/brokkr-runtime/src/agents/tests.rs`** — resolver-level argv facts,
where no seat name is required:

- `hands_keep_the_workspace_grant_and_drop_the_retired_tool_list` — the A1
  fixture. A test-only agent with the smith's hands *and*
  `tools.allow: ["cargo","git"]`; a Claude-shaped fixture adapter whose
  `tool_permissions` maps that list onto `--allowedTools
  Bash(cargo:*),Bash(git:*)` and whose `hands.workspace` carries the shipped
  seven tokens ending `--allowedTools mcp__brokkr__workspace`. Resolve under
  namespace; assert the seven-token fragment appears in order, that
  `--allowedTools` occurs **exactly once** with the argument exactly
  `mcp__brokkr__workspace`, and that no argument contains `Bash(cargo:*)` or
  `Bash(git:*)`. The same flag spelling on both sources is deliberate: flag
  absence must not masquerade as proof of precedence.

**`crates/brokkr-runtime/src/bundle/model_policy_tests.rs`** — compile-level,
seat named; uses the file's existing `Fixture`, `write_adapter`, `adapter`,
`write_agent_file`, `boxed_seat`, `compile_roots` and `shipped_adapters`:

- `a_tool_listed_work_seat_without_hands_keeps_codexs_exact_refusal` — **two
  arms, and this is not optional.** Arm one: a codex fixture declaring
  `"tool_permissions": "unsupported"` bare, so the delta's sentence appears
  *contiguously* and is compared as one string: `the provider declares
  tool_permissions unsupported, so the agent's restriction to ["cargo", "git"]
  cannot be expressed and the agent would run with MORE power than it
  declares`. Arm two: the same fixture with
  `{"unsupported": "<measured reason>"}`, which parenthesises the reason
  between the two halves; assert both halves and the reason. Both arms assert
  `seat 'work'`, the agent name, `provider 'codex'`, the model and the
  trailing "A capability the provider cannot express fails compilation here"
  explanation. A single-arm test cannot satisfy the verbatim demand, because a
  measured reason splits the sentence.
- `namespace_hands_replace_the_tool_list_only_through_declared_workspace` —
  the same fixture agent and adapter, tool list retained, hands added:
  compiles; assert the resolved candidate's provider, model and effort, the
  workspace fragment present, no tool-list flag, the recorded hands (network
  false, the declared binds) and the manifest `boundary` entry `namespace` — a
  positive expectation, not an absent error. Then the same with the adapter's
  `hands` removed, and again with `{"unsupported": "<reason>"}`: refusal
  naming seat, agent, provider and model, containing `the provider declares
  hands unsupported`, the reason where supplied, and `so the agent's hands
  cannot be put in the box and the agent would run with the harness's own
  tools`.
- `a_codex_hands_work_seat_is_admitted_by_its_harness_work_fragment` — the
  test-only hands agent on a codex fixture declaring both `hands.workspace`
  and `hands.harness.work: ["--sandbox","workspace-write"]`, compiled under
  `Boundary::Harness`: admitted, manifest `boundary` entry `harness`, the
  candidate carrying the adapter's `harness` declaration; the launch composed
  through `compose_site(BuiltBoundary::Harness, SeatClass::Work, …)` ends
  `--sandbox workspace-write` and contains no `mcp_servers.brokkr`, no
  `--mcp-config` and no per-tool flag. Then the member absent, and again
  `{"unsupported": "<reason>"}`: refusal naming the link, provider,
  `hands.harness.work`, the measured reason, `writes the tree only under the
  harness's own writable sandbox` and `decision 0046 rulings 1 and 4`; plus a
  control that the *workspace* fragment being present does not satisfy it.
  The recorded hands stay in the manifest and Brokkr enforces none of them
  here — asserted as the recorded-but-unenforced fact, not implied.
- `the_shipped_engine_smith_refuses_under_harness_on_its_claude_fallback` — a
  minimal `Fixture` work seat naming the shipped `implementer-engine` against
  the shipped `agents/` and `adapters/` roots under `Boundary::Harness` (a
  fixture seat, *not* `recipes/triage`, whose dialect step refuses earlier and
  would prove nothing about the chain). Assert the refusal names `link 2`,
  `provider 'claude'`, `hands.harness.work` and the writable-sandbox reason;
  assert as a control that a single-link fixture agent naming only `astra` on
  the shipped codex adapter *is* admitted under harness — the link the chain
  would otherwise have stopped at. No Claude fragment is invented.

**`crates/brokkr-runtime/src/engine/boundary_tests.rs`** — composed launches,
following the `the_shipped_codex_harness_work_seat_composes_the_preserved_rejoin`
precedent, which already loads the shipped tree and calls `report_under` +
`resolve_report` + `compose_site`:

- `the_shipped_engine_smith_composes_its_codex_namespace_launch` — resolve the
  shipped `implementer-engine` under `Boundary::Namespace`, take the `codex`
  candidate, compose with `compose_site(BuiltBoundary::Namespace,
  SeatClass::Work, candidate.argv, hands, Some(candidate), workdir, &[],
  result_path, None)`. Assert: `--model gpt-6-astra`, `--effort high`;
  `-c mcp_servers.brokkr.command="<engine exe>"`;
  `-c mcp_servers.brokkr.default_tools_approval_mode="approve"`;
  `--sandbox read-only` present as an adjacent pair and `workspace-write`
  absent anywhere; the `mcp_servers.brokkr.args=` token decoded — its TOML
  array of double-quoted strings is byte-identical to a JSON array, so
  `serde_json::from_str::<Vec<String>>` decodes it without calling
  `serve_args` — and equal to `["hands","serve","--workdir",<canonical
  workdir>,"--spec",<spec json>]`, with `<spec json>` parsed and compared to a
  literal `json!({"kind":"workspace","network":false,"binds":[…]})` written
  out in the test; no `--allowedTools` and no `Bash(` anywhere; no
  `{hands_mcp_json}`, `{hands_args_toml}` or `{brokkr}` left in any token.
- `the_shipped_engine_smith_composes_its_claude_namespace_launch` — the
  `claude` candidate: `--model claude-fable-5-1`, `--effort high`, and the
  fragment present **in order** — `--tools`, the empty argument,
  `--strict-mcp-config`, `--mcp-config`, the expanded JSON, `--allowedTools`,
  `mcp__brokkr__workspace` — with the JSON parsed and its
  `mcpServers.brokkr.command` and `.args` asserted as above; `--allowedTools`
  exactly once granting only `mcp__brokkr__workspace`; no `Bash(cargo:*)`, no
  `Bash(git:*)`, no `workspace-write`, no harness fragment, no placeholder
  left. Preserving this fragment requires no adapter edit and the test says so
  by loading `adapters/claude.json` as shipped.

Both launch tests build their expectations **literally** rather than by
calling `hands_command`, `serve_args` or `mcp_config`: the delta forbids
comparing a helper to itself. The existing
`compose_site_follows_the_boundary_and_the_class`, which does compare against
`hands_command`, stays as it is — it is a different claim, about arm
selection. Both canonicalize their temporary workdir with
`std::fs::canonicalize` before composing, because macOS resolves `TMPDIR`
through `/private/var` and an uncanonicalized path would let the
forwarded-workdir assertion pass for the wrong reason (decision 0063 names
macOS a host).

**`crates/brokkr-runtime/tests/library_data.rs`** —
`the_engine_smith_hires_astra_then_fable_through_workspace_hands`: load the
shipped library and adapters, resolve, assert candidate 0 = `codex` /
`gpt-6-astra` / `high` and candidate 1 = `claude` / `claude-fable-5-1` /
`high`; `resolution.hands` network false with exactly the two binds, their
modes and both masks; and, on the raw JSON, no `tools` key, no `boundary`
key, no bind with `mode: "rw"`, and no absolute checkout path.

**Unchanged and load-bearing.**
`every_shipped_panel_seats_at_least_two_providers`, the `gpt_flash_shape`
suite, `the_library_holds_the_decision_0041_roster`,
`shipped_claude_implementer_can_commit`,
`hands_replace_the_tool_list_with_the_adapters_fragment`,
`a_restriction_the_provider_cannot_express_is_a_hard_failure`,
`ruling_4s_own_binding_is_pinned_against_the_shipped_adapters` and
`a_measured_claude_gap_is_reported_not_papered_over` are not edited. Neither
is `every_shipped_bundle_compiles_under_harness_once_the_fragments_are_measured`:
phases compile in name order, `analyze` precedes `implement`, so the three
dialect bundles still refuse at `analyze:check` and
`assert_refused_at_the_dialect_step`'s `!refusal.contains("claude")` still
holds. If that pin does move, the expectation is re-derived from the new
refusal and explained in place; it is not loosened.

### D4 — Requirement-to-proof matrix, with removal evidence

Every rejected input asserts its diagnostic reason; every positive asserts
composed facts. "Removal" means a **production** mutation, run, observed to
fail the named assertion, restored, rerun green.

| Requirement / scenario | Proof | Removal mutation |
|---|---|---|
| the smith hires the ruled chain; hands and boundary recorded | `the_engine_smith_hires_astra_then_fable_through_workspace_hands` | declaration test; its control is the digest pins in D5 |
| writable access is the workspace mount; no checkout path, extra bind or network | same test, plus the codex launch test's decoded spec | in `serve_args`, drop `--spec` or a bind ⇒ the decoded-spec equality fails |
| `tools` removed while the MCP grant remains (A1) | `hands_keep_the_workspace_grant_and_drop_the_retired_tool_list` and both launch tests | in `compose`, turn the `else if let Some(allow)` into an `if` ⇒ `Bash(cargo:*)` appears and the absence assertion fails. Separately, truncate `argv.extend(fragment…)` by two tokens ⇒ the MCP grant presence/order assertion fails |
| moved digests are measured and explained | the two pin collections (D5) | alter one pinned digit ⇒ the pin test fails naming the bundle |
| panels, GPT/Flash and the runtime suite intact | existing tests, unedited | n/a |
| the guide and the 0043 note describe the declared confinement | the documentation inventories stay green; the note is read by the implementation's doc assertions | n/a — prose |
| no-hands tool-list refusal, word for word | `a_tool_listed_work_seat_without_hands_keeps_codexs_exact_refusal` (both arms) | replace the `ok_or_else` at `agents.rs:823` with a silent skip ⇒ the compile succeeds and the reason assertion fails, not merely some other error |
| namespace admission only through declared workspace support | `namespace_hands_replace_the_tool_list_only_through_declared_workspace` | delete `argv.extend(fragment.iter().cloned())` at `agents.rs:819` ⇒ the fragment-presence assertion fails |
| refusal without `hands.workspace` | the same test's second and third arms | replace `adapter.hands.as_ref().ok_or_else(…)` with a default empty fragment ⇒ the expected-reason assertion fails |
| harness work admitted by its own fragment; no workspace fragment there | `a_codex_hands_work_seat_is_admitted_by_its_harness_work_fragment` | in `compose_site`'s `Harness` arm make `SeatClass::Work` select `None` ⇒ the `--sandbox workspace-write` assertion fails. Converse: in the `Namespace` arm append `candidate.harness.work` ⇒ the launch tests' harness-fragment absence assertion fails |
| missing harness work stays a reason-bearing refusal | the same test | make the `SeatClass::Work if under_harness` arm return `Ok(())` when `work.is_none()` ⇒ the refusal disappears and the reason assertion fails |
| the whole chain, on the shipped Fable fallback | `the_shipped_engine_smith_refuses_under_harness_on_its_claude_fallback` | change `enforce_hands_boundary`'s candidate loop to `.take(1)` ⇒ link 2 is never judged, the refusal assertion fails while the single-link control still passes |
| the complete codex namespace launch | `the_shipped_engine_smith_composes_its_codex_namespace_launch` | remove the `{hands_args_toml}` replacement in `hands_command` ⇒ the placeholder/decode assertion fails; change the workdir `serve_args` forwards ⇒ the forwarded-workdir assertion fails |
| the complete claude namespace launch, grant preserved | `the_shipped_engine_smith_composes_its_claude_namespace_launch` | remove the `{hands_mcp_json}` replacement ⇒ the decoded-server assertion fails; truncate the appended fragment ⇒ the ordered-fragment and single-`--allowedTools` assertions fail |

Two notes the implementation must not blur. First, "Codex's native sandbox is
read-only" and "the shipped approval configuration is present" are adapter
*data* carried verbatim by `compose`; their production removal proof is the
fragment-truncation mutation above. Mutating a scratch copy of
`adapters/codex.json` is a useful supplementary control but is "a changed
fixture alone", which the delta refuses as removal evidence — it may be
recorded beside, never instead. Second, these are observations about Brokkr's
composition, not about how Codex or Claude behave when handed that argv, and
the record says so rather than implying a native-tool enforcement contract.

### D5 — Exactly which identities move, and why

The agent digest of `implementer-engine` changes, so every *compiled* bundle
that resolves that office moves. Composition digests cover an ancestor's own
files and its own ancestors (`bundle/compose.rs:817-846`), never the leaf's
agent resolution — so descent alone moves nothing.

| Pin | Moves? | Reason |
|---|---|---|
| `tests/witness_digests.rs` → `recipes/triage` | **yes** | hires `implementer-engine` at `implement:engine` |
| `tests/witness_digests.rs` → `recipes/night-shift` | **yes** | extends `triage` and overrides no implement case |
| `tests/witness_digests.rs` → `recipes/gpt-flash` | **no** (predicted) | re-declares all four implement cases on `gpt-flash-implementer-engine`; the triage layer digest is a file digest |
| `bundle/compose_tests.rs` → the inline `recipes/triage` digest in `a_composed_bundles_manifest_is_pinned` | **yes** | same bundle, same reason; must agree with the witness pin |
| `compose_tests.rs::UNCOMPOSED` (`fast`, `panel-review`, `bundles/self`, `verify`) | **no** | none resolves this office |
| the remaining six witnesses | **no** | same |

Both moving manifests additionally gain `hands["implement:engine"]` and
`boundary["implement:engine"]` — a select case's label is `"{phase}:{case}"`
(`bundle.rs:3150`). Both already carry those maps for their review offices, so
`run-manifest.v9` still describes them and no contract version is added.

The re-pin recipe: compile each bundle, read the left/right pair the failing
assertion reports, write that value — never a recomputed guess — and append
one history line to each of the two history blocks naming issue #307, the
Astra/Fable hire, the effort pins, and the hands that replace the tool list.
Existing history is appended to, never rewritten. If `recipes/gpt-flash`, or
any bundle outside this table, moves, that is an unmodelled dependency: stop,
explain it in the history block, and do not re-pin on autopilot.

### D6 — Prose: the guide, the dated note, and one stale listing

**`docs/guides/provider-adapters.md`.** Additive only. `## Hands` gains a
short paragraph: a provider with no per-tool flag (codex) serves a restricted
*work* seat by declaring hands; under `namespace` that requires
`hands.workspace`, writable work goes through the MCP server, and the
restriction is the empty-root filesystem and network boundary rather than a
list of commands the seat may run. It repeats 0043's limits in the same
breath — codex's native shell stays available read-only outside the box,
host-read secrecy is not promised, provider traffic belongs to the harness
outside the box — and points at `### hands.harness` for the separate unboxed
route with its own admission rule, without merging the two into one launch.
Every marker the documentation tests read survives verbatim, including
``declares **no** `hands.harness` member yet``, the three `hands.harness` table
rows and the doctor guide's composite wording.

**`docs/decisions/0043-the-hands-are-one-tool.md`.** One appended section in
the file's existing convention (`## Addendum — <date>, <status>: <title>`, as
decision 0035 uses), dated 2026-09-21, recording the operator's issue #307
ruling: the boxed smith's restriction is this decision's own filesystem and
network boundary; the earlier commission's allow-list-enforcement claim is
withdrawn; no per-tool contract and no new boundary composition is designed;
`hands.workspace` and `hands.harness.work` remain the two separate paths under
0046. It states "Status unchanged: accepted" in its opening line, edits no
historical ruling and adds no new ruling number. The `Status:` line at the
head of the file is not touched, so
`crates/brokkr-cli/tests/decisions_index.rs` stays green with no index edit.

**`docs/guides/agent-library.md`.** Its `$ brokkr agents list` sample still
reads `implementer-engine  fable → opus`, and its closing line says "The
review agents declare hands". Both become false. No test compares that sample
to the library, so the alternatives are to refresh the one row and the one
clause, or to leave them and record a documentation residual. This design
chooses to refresh them: a rendered listing that no longer renders is exactly
the drift the doctor guide test exists to prevent elsewhere, and the edit is
two lines with no requirement attached. It is flagged because the proposal's
Impact paragraph did not enumerate this file; the operator may strike it, and
then the residual is recorded in the delivery notes rather than silently left.

### D7 — The evidence boundary

Deterministic compilation and composition show the ruling is **expressible**.
They do not show a live Astra smith. The first live implementation stays the
controller's measurement after landing: Cargo and Git through Codex's
workspace hands, a real commit, and verify passing. Compilation, launch
inspection, mocks and removal tests do not discharge it, and the delivery
notes record it as a pending residual rather than instructing any gate.

Because no production line is added, the exact-coverage denominator is
unchanged and the literal 100% threshold is untouched. It still cannot be
measured from inside the box — boundary tests create a namespace the box
deliberately refuses to nest — so it is recorded as pending against the host
or CI run, never as a pass and never at a lower threshold. The same holds for
everything this seat could not execute: `cargo` is absent here, so formatting,
clippy, each crate's suite, `cargo test --workspace` and the `bundles/self`
compile are obligations of the implementation phase, not results claimed by
this design.

## Risks / Trade-offs

1. **The re-pin is the riskiest step.** Two files, two bundles, and digests
   that must be copied from a real compile. Mitigation: D5 states the expected
   movers *before* measurement, so an unexpected mover is a finding rather
   than a paste.
2. **`recipes/gpt-flash` not moving is a prediction.** It rests on
   `compose.rs`'s ancestor-digest rule and on gpt-flash re-declaring all four
   implement cases. If it moves, D5's rule applies: explain, then re-pin.
3. **A boxed smith is a Linux-only run.** Accepted in D1; no host is newly
   lost, and the guide already carries 0043's Linux statement.
4. **The two-arm refusal test is easy to get wrong.** A single
   measured-reason fixture cannot carry the delta's contiguous sentence.
   Called out in D3 so the implementation does not discover it by assertion
   failure.
5. **Codex keeps a read-only view of the host in its native shell.** That is
   0043's recorded consequence, not a regression introduced here; the D6
   paragraph repeats it rather than letting "boxed" imply more than it is.
   Nothing here narrows it and nothing claims to.
6. **Fallback asymmetry under `harness`.** The shipped smith refuses on its
   Claude link. That is the honest state of the Claude measurement, and
   inventing a fragment to make a chain compile is what the delta forbids.
   Namespace realms — the default — are unaffected.
7. **Astra's first engine-class delivery is unmeasured.** The hire is the
   operator's ruling and this change makes it expressible; whether Astra
   implements engine work well is the controller's measurement, and no test
   here stands in for it.

## Migration Plan

No data migration, no contract version, and no in-flight run affected beyond
the ordinary rule that a moved bundle identity makes an in-flight run refuse a
stale offer. The implementation order is dependency order:

1. `agents/implementer-engine.json` (D1) and the `is_house_tool_grant` arm.
2. The compile and resolver tests (D3) — green after the declaration lands.
3. The two composed-launch tests (D3).
4. Re-measure and re-pin the two moved digests; append both history lines (D5).
5. The guide paragraph, the 0043 addendum and the agent-library refresh (D6).
6. Removal evidence for every row of D4's table: mutate, run the one targeted
   test, record the failing assertion, restore, rerun green.
7. Full local validation — fmt, clippy `-D warnings`, each crate's suite,
   `cargo test --workspace`, `compile --bundle bundles/self`, strict OpenSpec
   validation — with exact coverage recorded pending its host or CI run.

Rollback is a revert of one data file, two pins and three prose edits; no
state is written anywhere that a revert would strand.

## Open Questions

1. **`docs/guides/agent-library.md`** — D6 refreshes the stale listing row and
   the "review agents declare hands" clause. The proposal's Impact paragraph
   did not name this file. If the operator holds the change to the enumerated
   surface, drop both edits and record the stale sample as a residual.
2. **`gpt-flash-implementer-engine`** — the forced crew keeps its own scoped
   smith, untouched here by design (the delta preserves GPT/Flash's separate
   crew). Whether that office should follow the same hands declaration is a
   separate commission, not a gap in this one.
3. **Claude's `hands.harness` members** remain the operator's pending
   measurement, recorded in the guide since 0046. This change neither advances
   nor depends on it; it only makes one more chain refuse under `harness`
   until it lands.
