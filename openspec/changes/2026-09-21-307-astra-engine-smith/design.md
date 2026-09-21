# Design — Astra as the boxed engine smith (issue #307)

Status: proposed. Change: `2026-09-21-307-astra-engine-smith`.
Adopted proposal and both capability deltas: `1ab50740` and `5ef97115`,
on production base `f4392a73`. The two specs are
[astra-engine-smith](specs/astra-engine-smith/spec.md) and
[boxed-work-provider-admission](specs/boxed-work-provider-admission/spec.md).

## Context

The operator ruled on 2026-09-20 that the engine smith hires Astra (Codex)
first and Fable (Claude) second, both at high effort, under decision 0043's
existing workspace hands. The proposal's D1–D5 already fix the meaning: the
restriction this change owes is 0043's empty-root filesystem and network
boundary, not an `["cargo","git"]` command allow-list; `hands.workspace` is
the boxed path and `hands.harness.work` stays the separate unboxed path; no
new per-tool contract or boundary composition is designed; and a tool list
on a provider that cannot express one, with no hands, stays refused word for
word.

This design was written after reading the code the specification touches. It
settles what the implementation must do and, more importantly, what it must
not do, so the data-only change cannot drift into a semantic one.

What the tree already does (the load-bearing source facts):

- `agents/implementer-engine.json` currently chains `["fable","opus"]` with
  a `tools.allow: ["cargo","git"]` and no `hands`. The signature `fable` is
  served by Claude; Claude *can* express a tool list, which is why the
  current file compiles. `astra` is served by Codex, whose adapter declares
  `tool_permissions` unsupported with a measured gap
  (`crates/brokkr-runtime/src/agents.rs` `compose`, the `else if let
  Some(allow)` arm; `adapters/codex.json`).
- `agents.rs::compose` consults `agent.allow` **only** in the `else` branch
  of `if agent.hands.is_some()`. When hands are present and the site is
  boxed, the adapter's `hands.workspace` fragment is appended and the tool
  list is structurally unread. When hands are present and the site is
  unboxed, `compose` appends nothing; `bundle.rs::enforce_hands_boundary`
  judges the site instead.
- `agents::load.rs::parse_agent` accepts `hands` and `tools` independently;
  `parse_adapter` reads `hands.workspace` / `hands.harness` and an absent
  `hands` as unsupported fail-closed.
- `bundle.rs::resolve_reference` calls `report_under(..., boundary)` and,
  before `resolve_report`, runs `enforce_model_policy` (and therefore
  `enforce_hands_boundary`) over the whole chain when the agent has hands
  and every link is mapped. The seat-level wrapper is what puts
  `seat '<site>':` in a refusal.
- `bundle.rs::enforce_hands_boundary` returns at once for a boxed boundary;
  under `harness` an agent-resolved **work** seat refuses on any link whose
  adapter declares no `hands.harness.work`. `adapters/claude.json` declares
  no `hands.harness` at all; `adapters/codex.json` declares
  `hands.harness.work: ["--sandbox","workspace-write"]`.
- `engine.rs::compose_site` selects `hands_command(...)` for a boxed
  boundary and appends `candidate.harness.gate`/`.work` for `harness`.
  `engine.rs::hands_command` expands `{hands_mcp_json}`,
  `{hands_args_toml}` and `{brokkr}` in a model seat's argv from
  `brokkr_protocol::hands::{mcp_config, serve_args}`.
- `brokkr_protocol::hands::HandsSpec::parse` is a closed vocabulary
  (`kind`, `network`, `binds`); a `boundary` key, unknown key, bad mode or a
  malformed mask is a load refusal.

Read-only material (`contracts/`, `policy/`, `fixtures/`, `reference/`,
`extensions/`) is untouched. Hosts are Linux and macOS only (decision 0063);
no Windows obligation, branch, fixture or evidence is written. This seat has
no `cargo` and no network, so nothing here claims a build or test result.

## Goals / Non-Goals

**Goals:**

- Change exactly one production data file:
  `agents/implementer-engine.json`.
- Prove, with reason-bearing tests, that the existing compiler and engine
  rules already express the ruled hire, the boxed namespace launch and the
  two harness refusals — with no new contract.
- Re-measure every digest this declaration moves and explain it in the pin
  collections.
- Make the guide and the 0043 note state the confinement that is actually
  declared, including its limits.

**Non-Goals:**

- No command allow-list, shell-command policy, transport contract, native
  tool-removal switch, or new per-tool enforcement.
- No `adapters/*.json` edit; no protocol, dependency, boundary-vocabulary,
  contract, fixture or policy change.
- No change to `hands.harness.work` semantics, no guessed Claude
  `hands.harness` fragment, and no combined namespace/harness launch.
- No live Astra smith, no provider call, no nested namespace, no publication.
- No sweep of unrelated pre-existing documentation drift.

## Decisions

### D1. Change the declaration only; name the functions that must not move

The production edit is one JSON object in
`agents/implementer-engine.json`:

- `models` becomes exactly `["astra","fable"]` (order is the hire order);
- `efforts` becomes exactly `{"astra":"high","fable":"high"}`;
- a `hands` object is added: `{"kind":"workspace","network":false,"binds":[
  {"path":"~/.cargo","mode":"overlay","mask":["credentials.toml",
  "credentials"]},{"path":"~/.rustup","mode":"ro"}]}` — the exact shape
  `agents/reviewer.json` already ships;
- `tools` is removed entirely (the `allow` list and the empty `mcp` list);
- `description`, `charter` and `limits` are preserved verbatim.

Source reading finds no compiler gap, so no Rust production line is planned.
The functions that must not change are the contract the tests prove:
`agents.rs::{compose, entry_for, report_under, resolve_report}`;
`agents/load.rs::{parse_agent, parse_tools, HandsSpec::parse caller,
parse_adapter's hands branch}`; `bundle.rs::{resolve_reference,
enforce_hands_boundary, record_hands, manifest_for, parse_select}`;
`engine.rs::{compose_site, hands_command}`;
`brokkr_protocol::hands::{HandsSpec::parse, serve_args, mcp_config,
box_argv}`.

If a new test exposes a real gap, the only authorized repairs are (i) a
message-shape bug in the existing refusal arms that preserves the reason
text verbatim, or (ii) the precedence already decided by 0043 ruling 2
(having hands suppresses `tools.allow`). Anything else — a new flag, a
command parser, a boundary change, an adapter edit — is an upstream fault
and must be returned, not improvised. The proposal's "smallest repair"
permission is bounded to that reading.

Rejected alternatives:

- **Keep `tools` for the record.** `0043` ruling 5 kept the review gates'
  grants "for the record", but ruling 2 makes a list beside hands dead on
  both providers; `crates/brokkr-runtime/tests/roster.rs` already refuses
  dead tools beside hands, and the proposal's D2 rejects a misleading
  Claude-only grant. The historical grants live in decision 0043 and the
  journal.
- **Teach Codex a per-tool list (a new adapter capability).** This is the
  withdrawn premise of the escalated issue; the operator ruled it out. Codex
  restricts by sandbox class; its native shell cannot be removed, and the
  box is the stronger statement anyway (0043 context).
- **Add an `["astra","fable"]`-only adapter or a new hands kind.** No
  vocabulary or capability change is authorized; the existing data
  expresses the hire.
- **Pin efforts lower on fallback.** The operator ruled both `high`; a
  falling hire would contradict the commission.

### D2. The restriction is 0043's box, never a command list

`hands.workspace` under `namespace` binds the worktree read-write at its own
path, the host toolchain read-only, and the declared `~/.cargo` overlay
(with both credentials masked) and `~/.rustup` read-only. What the model may
run is `bash -lc` inside that box; what it can touch is the boundary. The
guide and the tests must say exactly that and must not describe a command
allow-list, command parsing or transport policy. `network: false` unshares
the network; the box's git facts and unsigned-commit environment come from
`hands::box_argv` and `git_facts`, not from the agent file.

Rejected alternatives: expressing the restriction as Codex's
`workspace-write` sandbox class (a one-provider coarsening that widens the
grant), or relying on the native Codex shell (read-only but whole-host and
credential-readable, per 0043's consequences). The box is the point.

### D3. Hands precedence, and the one `--allowedTools` that is not a retired grant (A1)

`compose` reads the tool list only when the agent has no hands. The A1
clarification is therefore encoded as a source distinction, not a flag-name
distinction: a capable provider fixture maps the retired
`tools.allow: ["cargo","git"]` onto the same flag Claude's workspace
fragment uses (`--allowedTools`), while the fragment grants
`mcp__brokkr__workspace`. The test asserts `--allowedTools` occurs exactly
once with the MCP value and that no argument contains `Bash(cargo:*)` or
`Bash(git:*)`, so flag absence cannot masquerade as precedence. The shipped
Claude fragment stays byte-identical:
`--tools`, `""`, `--strict-mcp-config`, `--mcp-config`,
`{hands_mcp_json}`, `--allowedTools`, `mcp__brokkr__workspace`.

Rejected alternative: dropping the MCP workspace grant to prove "no tool
list". That contradicts 0043 ruling 2 and would leave the boxed Claude seat
unable to call the one tool the box serves; the spec's A1 scenarios forbid
it. Removing the grant is a valid *removal mutation* (the presence
assertion must fail), not a fix.

### D4. Namespace and harness stay two separate paths

Under `namespace`, `compose` appends `hands.workspace` and
`compose_site` calls `hands_command`; the MCP server is registered and the
Codex native sandbox is `--sandbox read-only` from the same fragment. Under
`harness`, `compose` appends nothing, `enforce_hands_boundary` requires
`hands.harness.work` on **every** link, and `compose_site` appends that
fragment with `{result_path}`/`{brokkr}` expanded; no workspace MCP server
is served. A realm that declares `harness` cannot run the smith until
Claude declares a measured `hands.harness.work`; the correct behaviour is to
refuse on link 2, not to guess a fragment or combine the paths.

Rejected alternatives: let namespace borrow `workspace-write` from the
harness path; let missing harness work silently degrade to the harness's
default; or add a synthetic Claude fragment. Each is a boundary change the
spec's `boxed-work-provider-admission` R3 forbids.

### D5. Proof inventory: requirement → test → reason text → removal proof

The new proofs are test-only. Fixture names that must match the spec
(`codex`, `claude`) are deliberate: the engine stays vendor-blind, but the
refusal wording names them, so the fixture must too.

#### astra-engine-smith R1 — the ruled chain through workspace hands

| Scenario | Test (file) | Assertion |
|---|---|---|
| S1.1 shipped resolves both hires | `the_shipped_engine_smith_records_the_ruled_chain_hands_and_namespace` (`src/bundle/model_policy_tests.rs`) | seat `work` names `implementer-engine`; candidates are `[astra/codex/high, fable/claude/high]`; `bundle.hands["work"]` has `network == false` and exactly the two binds above; `manifest["boundary"]["work"] == "namespace"`; the raw agent file has no `tools` key |
| S1.2 writable access uses the existing mount | `the_shipped_engine_smiths_astra_launch_exposes_only_the_declared_workspace_box` (`tests/astra_engine_smith.rs`, new) | the decoded `mcp_servers.brokkr.args` carry the canonical workdir and the complete spec; the spec's binds are exactly the two toolchain binds; no extra writable host bind, no checkout path, no `network:true`, no `boundary` key in the agent |
| S1.3 `tools` removed, MCP grant intact (A1) | `hands_precedence_keeps_only_the_workspace_mcp_grant` (`src/agents/tests.rs`) + the two existing roster tests `a_codex_lane_is_chained_only_into_boxed_or_toolless_offices` and `tool_grants_keep_house_tools_explicit_and_effort_never_rises_on_fallback` (`tests/roster.rs`) | no `tools` object in the shipped file; Codex argv has no per-tool flag; Claude argv has `--allowedTools` exactly once with `mcp__brokkr__workspace`; neither argv has `Bash(cargo:*)`/`Bash(git:*)` |

#### astra-engine-smith R2 — identities and invariants

| Scenario | Test | Assertion |
|---|---|---|
| S2.1 the hiring bundle moves for the stated reason | `pinned_bundles_keep_their_recorded_digest` (`tests/witness_digests.rs`) and `a_composed_bundles_manifest_is_pinned` (`src/bundle/compose_tests.rs`) | the `recipes/triage` pin is re-recorded from an actual compile; both history blocks attribute the move to the #307 hire, efforts and hands replacing the old list |
| S2.2 independent rosters hold | `every_shipped_panel_seats_at_least_two_providers`, `the_fetch_grant_is_held_by_the_researcher_alone_and_never_by_a_gate`, `library_data.rs::every_shipped_agent_resolves_at_compile_time`, the whole `gpt_flash_shape` suite, and `cargo test --workspace` | gpt-flash keeps its own scoped offices; panel diversity untouched; every shipped agent still resolves with no notices and a real fallback chain |

#### astra-engine-smith R3 — guide and decision note

| Scenario | Test | Assertion |
|---|---|---|
| S3.1 allow-list escalation answered | `the_handed_work_seat_guidance_and_the_0043_note_are_recorded` (`tests/astra_engine_smith.rs`) | the guide says the confinement is the existing filesystem and network boundary, not a command allow-list; the note withdraws the earlier enforcement claim |
| S3.2 composition escalation answered | same test | the guide keeps namespace (MCP server + Codex read-only native sandbox) and harness (`hands.harness.work`, no Brokkr box) as separate routes |
| S3.3 the note adds no new rule | same test | `docs/decisions/0043…md` still reads `Status: accepted`, keeps its 2026-09-03 historical text, and adds a dated 2026-09-21 #307 note |

The doc regression reads the two text files and asserts the requirement's
stable anchor phrases; it does not pin whole paragraphs. Removal mutation:
delete a required clause and the corresponding assertion fails; restore.

#### boxed-work-provider-admission R1 — the no-hands refusal

`a_codex_tool_list_without_hands_refuses_word_for_word`
(`src/bundle/model_policy_tests.rs`): a fixture Codex adapter with
`tool_permissions: "unsupported"`, a work agent listing exactly
`["cargo","git"]` and no hands, compiled under namespace. The refusal must
name `seat 'work'`, `agent 'smith'`, `provider 'codex'`, `model 'astra'`, the
existing capability explanation, and verbatim:

> `the provider declares tool_permissions unsupported, so the agent's restriction to ["cargo", "git"] cannot be expressed and the agent would run with MORE power than it declares`

A variant adds a capable later link to prove a work class, a namespace realm
and an available fallback do not bypass the refusal.

#### boxed-work-provider-admission R2 — hands replace the list

| Scenario | Test | Assertion |
|---|---|---|
| S2.1 the same fixture gains hands and compiles | `the_same_codex_fixture_gains_hands_and_compiles_under_namespace` | compile succeeds; `bundle.hands["work"]` is the declared spec; the manifest record is `codex`/namespace; candidate argv carries the Codex workspace fragment and no tool-list flag |
| S2.2 hands need a workspace-capable adapter | `declared_hands_cannot_use_an_unsupported_workspace_adapter` | refusing seat/agent/provider/model with `the provider declares hands unsupported` and `so the agent's hands cannot be put in the box and the agent would run with the harness's own tools`; a `{"unsupported":"…"}` reason is preserved |
| S2.3 MCP grant vs retired list (A1) | `hands_precedence_keeps_only_the_workspace_mcp_grant` | as in R1/S1.3 |

#### boxed-work-provider-admission R3 — harness is separate

| Scenario | Test | Assertion |
|---|---|---|
| S3.1 the harness work fragment admits | `the_same_codex_hands_fixture_is_admitted_by_its_harness_work_fragment` | compile under `harness` succeeds; boundary is `harness`; `hands` is recorded; `compose_site(Harness, Work, …)` appends `--sandbox workspace-write`, no MCP server, no `--allowedTools` |
| S3.2 missing harness work refuses | `missing_harness_work_remains_a_reason_bearing_refusal` | refusal names seat, link 1, provider `codex`, `hands.harness.work`, and verbatim the writable-sandbox reason citing **decision 0046 rulings 1 and 4**; a measured gap reason is appended |
| S3.3 shipped Fable fallback keeps the chain honest | `the_shipped_engine_smith_refuses_under_harness_on_the_claude_link` | a minimal fixture seat naming the shipped `implementer-engine` refuses on **link 2**, `provider 'claude'`, `hands.harness.work`, even though link 1 (`codex`) declares work; no guessed fragment and no fallback omission |

#### boxed-work-provider-admission R4 — the shipped namespace launch

| Scenario | Test | Assertion |
|---|---|---|
| S4.1 the complete Codex launch | `the_shipped_engine_smiths_astra_launch_exposes_only_the_declared_workspace_box` | model `gpt-6-astra`, effort `high`; `mcp_servers.brokkr.command="{exe}"`, `mcp_servers.brokkr.args=[…]`, `mcp_servers.brokkr.default_tools_approval_mode="approve"`, `--sandbox read-only`; the args decoded as a JSON array are `["hands","serve","--workdir",<canonical wd>,"--spec",<json>]`, and the spec is `network:false` with both binds and both Cargo masks; no `workspace-write`, no harness fragment, no per-tool list flag, no unexpanded `{hands_mcp_json}`/`{hands_args_toml}`/`{brokkr}` |
| S4.2 the complete Claude launch | `the_shipped_engine_smiths_fable_launch_preserves_its_workspace_fragment` | model `claude-fable-5-1`, effort `high`; the ordered fragment `--tools`, `""`, `--strict-mcp-config`, `--mcp-config`, `<expanded JSON>`, `--allowedTools`, `mcp__brokkr__workspace`; the JSON registers `brokkr` with the executable and `serve_args`; `--allowedTools` occurs once; no `Bash(cargo:*)`/`Bash(git:*)`; no harness fragment or unexpanded placeholder |

The Codex `args=` token is a JSON string array: `hands_command` builds it with
JSON string escaping (`\\` and `\"`), so the test decodes it with
`serde_json::from_str::<Vec<String>>` rather than a helper-to-helper
comparison. Both launches use `tempfile::tempdir()` canonicalized before use,
so the macOS `/var` → `/private/var` spelling is stable.

#### boxed-work-provider-admission R5 — removal evidence

Every protection above is proved by one mutation, its targeted test, the
expected assertion failure, a restore and a passing rerun. Compile errors,
unrelated refusals, a changed fixture alone and comments do not count.

| Protection | Mutation (restored after) | Assertion expected to fail |
|---|---|---|
| fail-closed tool list | `compose` returns `Ok`/an empty flag when `adapter.tool_permissions` is `None` | R1 exact-reason / `seat 'work'` assertion |
| hands precedence | consult `agent.allow` even when `agent.hands.is_some()` | A1 `--allowedTools` count and `Bash(cargo:*)` absence |
| workspace-capability refusal | treat `adapter.hands == None` as an empty fragment in `compose` | R2/S2.2 `hands unsupported` assertion |
| harness work admission | read `candidate.harness.gate` for a work seat in `compose_site` | R3/S3.1 `--sandbox workspace-write` tail |
| whole-chain harness law | stop `enforce_hands_boundary` at the first passing link | R3/S3.3 link-2 `claude` refusal |
| missing harness work | delete the `harness.work.is_none()` arm for work seats | R3/S3.2 reason-bearing refusal |
| MCP registration | drop `mcp_servers.brokkr.*` from the Codex `hands.workspace` fragment (fixture) | S4.1 registration/args assertions |
| native sandbox | change the Codex fragment's class to `workspace-write` (fixture) | S4.1 `--sandbox read-only` |
| forwarded workdir/binds/network | alter `serve_args`, or drop a bind/mask/`network:false` from the agent | S4.1 decoded-spec assertions |
| Claude fragment preservation | drop `--allowedTools mcp__brokkr__workspace` from the Claude fragment (fixture) | S4.2 presence assertion |
| retired-grant absence | have the resolver consult the retired list (fixture) | S4.2 / A1 `Bash(cargo:*)` absence |
| doc clauses | delete a required guide/note clause | the doc regression's phrase assertion |

Implementation must record the actual mutation, command and failing assertion
(not the expected one printed above), then restore and re-run.

### D6. Identity: only `recipes/triage` moves, and both pin collections say so

`agents/implementer-engine.json` is resolved by exactly one shipped leaf:
`recipes/triage`'s `implement:engine` select case (and nowhere in
`gpt-flash`, whose engine case names its own scoped office, nor in
`night-shift`, which overrides `implement` with an inline dsh driver). The
manifest moves because the `agents["implement:engine"]` record carries
`agent_digest` and the resolved chain, and `manifest["hands"]`/
`["boundary"]` gain the `implement:engine` site.

Two pins therefore move and no others:

- `crates/brokkr-runtime/tests/witness_digests.rs::WITNESSES` —
  `recipes/triage` (`d95b41d9…f336f` at this base) is replaced by the
  measured value, with a new history paragraph attributing it to the #307
  Astra/Fable hire, high efforts and workspace hands replacing the retired
  tool list.
- `crates/brokkr-runtime/src/bundle/compose_tests.rs::a_composed_bundles_manifest_is_pinned`
  — the `recipes/triage` inline expectation is replaced by the same measured
  value, with the same explanation.

`recipes/gpt-flash` and `recipes/night-shift` do **not** move: an ancestor
digest is computed by `bundle::compose::resolve` with no agent records and no
hands, so it excludes agent resolution; and neither leaf resolves
`implementer-engine` itself. `recipes/fast`, `recipes/node`,
`recipes/preflight`, `recipes/wager-harness`, `recipes/research`,
`recipes/research-dsh`, `recipes/panel-review`, `bundles/self` and
`bundles/verify` do not resolve the smith and keep their pins. No digest is
guessed: both values are copied from the failing `assert_eq!` output of a
real `cargo test --workspace`.

The one dead test clause removed is
`roster.rs::is_house_tool_grant`'s `("implementer-engine","cargo"|"git")`:
after this change the office declares no list, and leaving a clause that
grants house-tool standing to a declaration nobody writes would misstate the
roster. `a_codex_lane_is_chained_only_into_boxed_or_toolless_offices`
remains the guard against a future codex-lane office that re-adds `tools`.

### D7. Documentation, the decision note, and the house-rules boundary

- `docs/guides/provider-adapters.md` gains a subsection under `## Hands`
  (before or after `### hands.harness`) stating: a provider with no native
  per-tool flag serves a restricted work seat through declared hands; under
  `namespace` it needs `hands.workspace`, writable work goes through the MCP
  `workspace` tool, and Codex's native sandbox is `read-only`; the
  confinement is the existing filesystem and network boundary and never a
  command allow-list; Codex's native shell remains available read-only
  outside the box, host-read secrecy is not promised, and provider traffic
  belongs to the harness outside the box; `hands.harness.work` is the
  separate unboxed route with its existing admission rule. Cross-reference
  the 2026-09-21 note below.
- `docs/decisions/0043-the-hands-are-one-tool.md` gains a dated
  `## Addendum — 2026-09-21, operator ruled: the engine smith is boxed by
  the existing workspace hands` section recording the issue #307 ruling,
  withdrawing the earlier allow-list-enforcement claim, and stating that no
  new per-tool contract or boundary composition is designed. Its status line
  and historical rulings are not edited.
- `docs/guides/agent-library.md` line 26's illustrative `agents list`
  snippet still names `implementer-engine fable → opus`; this change
  falsifies that one row, so it becomes `astra → fable`. The other rows'
  pre-existing drift is out of scope and is not swept.
- No new house decision is authored: this change applies accepted 0043 and
  0046 and the operator's #307 ruling. Decision 0043's status stays
  `accepted`.

### D8. Evidence boundary: expressibility now, live Astra in the controller's hands

Deterministic compile, launch decoding and removal proofs show the ruling is
*expressible*; they do not prove a live Astra smith. The first live
measurement remains the controller's after landing: Cargo and Git through
Codex's workspace hands, a real commit, and verify passing. Result notes must
say that plainly, name the controller's measurement as pending, and never
instruct a gate to pass. `cargo` is absent in this seat, so formatting,
clippy, every crate's suite, `cargo test --workspace`, `bundles/self`,
strict OpenSpec validation and literal exact coverage are recorded as
pending until observed elsewhere; the coverage gate is never lowered.

## Risks / Trade-offs

- **The smith becomes boxed, so it is Linux-bound under `namespace`.** On
  macOS the bundle still compiles and the run refuses at start until
  decision 0046 slice (ii) `seatbelt` lands; a `harness` realm refuses at
  compile because Claude declares no `hands.harness.work`. This is accepted:
  the operator ruled the chain and the box, and macOS stays supported
  through the eventual seatbelt boundary. The guide and test must not hide
  it.
- **`network: false`.** The smith cannot fetch; its commit is local and
  publishing is outside its office. A future networked need is a new ruling,
  not a quiet bind.
- **Overlay binds need bubblewrap ≥ 0.10.** Ubuntu 24.04's 0.9 refuses at
  run start; accepted exactly as the shipped review offices are.
- **Removing `tools` drops the visible command grant.** Accepted under 0043
  ruling 2; the historical grant stays in the decision and journal. The box
  is a stronger write bound than the list, while Codex's native read-only
  shell remains outside it.
- **One identity re-pin.** `recipes/triage` moves once. Accepted and
  explained in both pin collections; no other digest is touched.
- **A doc regression can be brittle.** Anchor on the requirement phrases
  (boundary vs allow-list, read-only native shell, separate harness route,
  accepted status, dated withdrawal), not on surrounding prose.
- **No production line is planned, but a test could expose a gap.** D1 bounds
  any repair; an unbounded repair is an upstream return, not a silent
  semantic change.
- **Exact coverage.** No production line is added, so the literal 100% gate
  is not expected to move; if any line is added it needs its exercising case,
  and the host/CI result is reported, never assumed.

## Migration Plan

1. Edit `agents/implementer-engine.json` exactly as D1. Keep the charter and
   limits; add the reviewer-shaped `hands`; remove `tools`.
2. Author the fixture tests in `src/agents/tests.rs` and
   `src/bundle/model_policy_tests.rs`, the shipped-launch/doc tests in
   `tests/astra_engine_smith.rs`, and run each targeted test; record the
   removal mutation, the observed failing assertion, the restore and the
   passing rerun.
3. Compile every witness (`cargo test -p brokkr-runtime --all-features
   --locked` and `cargo test --workspace`); replace the `recipes/triage`
   pin in `witness_digests.rs` and `compose_tests.rs` with the measured
   values and add the history paragraphs. Do not guess.
4. Apply the guide and 0043 note edits from D7, plus the one `agent-library`
   row; run the doc regression.
5. Run the release validation set in the release configuration: format,
   clippy `-D warnings`, workspace tests, `compile --bundle bundles/self`
   and `--bundle bundles/verify`. Record the host/CI exact-coverage result
   as pending until it exists; never lower the threshold.
6. Commit the change directory's implementation in the repository message
   style. Do not push, publish, or run a live smith.

Rollback is a coherent revert of the agent declaration, the tests, the two
pins and the docs together. A partial revert would leave a pin that no
compile matches, which the witness test refuses.

## Open Questions

No unresolved design choice is delegated to implementation. The following are
recorded obligations, not choices:

- The first live Astra implementation is the controller's measurement after
  landing; this design and its tests stop at expressibility.
- The two moved `recipes/triage` digests exist only after a real compile and
  must be copied from it.
- The pre-existing drift in the other `docs/guides/agent-library.md` sample
  rows predates this change and stays out of scope.
