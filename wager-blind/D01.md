# Design — seat Astra as the boxed engine smith (issue #307)

Change: `2026-09-21-307-astra-engine-smith`
Phase: design (solo seat). Proposal and both capability deltas
(`astra-engine-smith`, `boxed-work-provider-admission`) are ruled clear
and are not edited here.

## Context

The operator ruled (2026-09-20/21) that the engine smith hires Astra, then
Fable, both at high effort, under decision 0043's existing workspace
confinement. The current `agents/implementer-engine.json` chains
`fable → opus` with a `tools.allow: ["cargo", "git"]` declaration, so every
namespace bundle hiring it fails at Codex: Codex declares
`tool_permissions` unsupported (it restricts by sandbox class, not tool
name — `adapters/codex.json`), and decision 0016 makes an inexpressible
restriction a hard compile refusal. Decisions 0043 (hands are one boxed
tool) and 0046 (the boundary is named; `harness` vs `namespace` are separate
paths) already supply the mechanism; this change only seats the ruled hire
through them.

Source facts read before deciding (all line references to the tree as it
stands):

- `crates/brokkr-runtime/src/agents.rs::compose` (lines 798–857): when
  `agent.hands.is_some()` and the site is boxed, the tool list is **not
  consulted**; the adapter's `hands.workspace` fragment is required and
  appended whole (0043 ruling 2). Without hands, `allow` requires
  `tool_permissions` and refuses word for word otherwise. Both checks run
  over **every** chain link (`resolve_report`, lines 1004–1021).
- `crates/brokkr-runtime/src/bundle.rs::enforce_hands_boundary` (lines
  2610–2700): under a boxed boundary it returns at once (0043's law,
  unchanged). Under `harness`, an agent-resolved **work** seat requires
  `hands.harness.work` on **every** link, naming seat, link, provider and
  the writable-sandbox reason citing 0046 rulings 1 and 4. Under `open` a
  work seat runs at the harness default. `boundary.is_boxed()` selects the
  boxed arm (`report_under`, agents.rs lines 962–989).
- `crates/brokkr-runtime/src/engine.rs::compose_site` (lines 4270–4318) and
  `hands_command` (lines 4509–4579): under `Namespace` a model site's argv
  (which already carries the adapter fragment from resolution) is expanded
  — `{hands_mcp_json}` via `mcp_config`, `{hands_args_toml}` via
  `serve_args`, `{brokkr}` via the current executable. Under `Harness` the
  candidate's class-selected harness fragment (`gate`/`work`) is appended
  with `{result_path}`/`{brokkr}` expanded and **no workspace tool is
  served**. The two paths never mix.
- `crates/brokkr-protocol/src/hands.rs`: `HandsSpec { network, binds }`
  with closed `workspace` vocabulary; `serve_args` (`hands serve
  --workdir <dir> --spec <json>`); `execute_in` runs `bash -lc`; `box_argv`
  binds the workdir read-write at its own path plus declared binds.
- `adapters/codex.json`: no per-tool flag (measured `tool_permissions`
  gap); `hands.workspace` = `--sandbox read-only` + `mcp_servers.brokkr`
  `-c` keys incl. `default_tools_approval_mode="approve"`;
  `hands.harness.work` = `--sandbox workspace-write`.
  `adapters/claude.json`: `hands.workspace` = `--tools ""`,
  `--strict-mcp-config`, `--mcp-config {hands_mcp_json}`, `--allowedTools
  mcp__brokkr__workspace`; **no** `hands.harness` member (unmeasured,
  fail-closed). Its `tool_permissions` map (`cargo → Bash(cargo:*)`, …)
  exists but must never fire beside hands.
- Precedent boxed agents (`reviewer.json`, `review-correctness.json`,
  `review-security.json`): `network: false`, `~/.cargo` overlay masking
  `credentials.toml`+`credentials`, `~/.rustup` read-only, no `tools`, no
  `boundary`. The smith copies this fragment exactly.

Binding operator rulings for this design: chain is `astra` then `fable`;
the owed restriction is 0043's workspace confinement, not a per-tool
allow-list, and **no new per-tool enforcement contract is designed**; the
boxed smith uses `hands.workspace` alone while `hands.harness.work` stays
the separate unboxed path; a tool-listed agent with no hands on a provider
that cannot express one stays refused word for word.

## Goals

1. Ship `agents/implementer-engine.json` as `models: ["astra", "fable"]`,
   `efforts: {astra: high, fable: high}`, workspace hands
   (`network: false`, the two toolchain binds above), no `tools`, no
   `boundary`; charter, description and limits untouched.
2. Prove the existing compiler needs no change: temporary Codex fixtures
   show the no-hands refusal verbatim, namespace admission via declared
   workspace support, refusal without it, and the separate harness
   outcomes — all with reason-text assertions, never bare `is_err()`.
3. Prove the shipped smith's actual namespace launch composition on both
   links (MCP registration, workdir/binds/network forwarding, Codex
   read-only sandbox, Claude's complete workspace fragment with exactly
   one `--allowedTools mcp__brokkr__workspace` and no retired grants),
   with per-claim removal evidence.
4. Re-measure and re-pin every moved witness/compose digest with a #307
   history entry; keep panel diversity, the GPT/Flash roster and the whole
   runtime suite green.
5. Record the confinement honestly in `docs/guides/provider-adapters.md`
   and a dated 2026-09-21 note on decision 0043 (status stays `accepted`;
   no new semantic decision is asserted — this enacts existing accepted
   semantics, so no new decision file is owed).

## Non-goals

- No adapter, protocol, contract, policy-table, fixture, reference or
  boundary-vocabulary change. `contracts/`, `policy/`, `fixtures/`,
  `reference/`, `extensions/` and the issue #226 ledger are out of slice.
- No per-tool allow-list enforcement, command parsing, transport semantics
  or native-tool bypass prevention (proposal D1; spec A1 distinction
  preserved: the ban covers arguments *generated from the retired
  `tools.allow` list*, never Claude's shipped MCP workspace grant).
- No live Astra smith, no network use, no workflow runner, no publication.
- No Windows obligations (decision 0063: hosts are Linux and macOS only);
  temporary test roots are canonicalized.
- No combining of the namespace and harness paths; no guessed Claude
  harness fragment (the shipped harness refusal on link 2 is intended
  behavior until the operator measures that member).

## Decisions

### D1 — The agent declaration: ruled chain, precedent hands, tools removed

New `agents/implementer-engine.json`:

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

That is: models `fable,opus → astra,fable`; efforts follow the models;
`tools` deleted; `hands` added byte-identical in shape to the reviewer
precedent; no `boundary` key (0046 ruling 1: the realm declares it);
charter, description, limits unchanged. Effort ranking stays flat
(high/high), satisfying the roster's never-rise-on-fallback pin. Concrete
resolution: `astra → codex/gpt-6-astra`, `fable → claude/claude-fable-5-1`;
both adapters declare `high` in their effort vocabularies, so resolution
succeeds on both links under namespace today.

*Alternatives rejected:*

- Keep `tools.allow` beside `hands` as "record". Rejected: `compose`
  ignores it whenever hands exist, so it is dead text; worse, the roster
  pin `tool_grants_keep_house_tools_explicit…` refuses dead tools beside
  shipped hands, and proposal D2 + spec A1 require its removal. A
  test-only fixture (not the shipped agent) carries both fields to prove
  replacement precedence.
- Chain `astra` alone (no fallback). Rejected: the ruling hires a chain of
  two; single-provider panels/seats also weaken the fleet's vendor-diversity
  posture, and `resolve_report`'s whole-chain rule means the fallback's
  admission is load-bearing evidence, not decoration.
- Mint a narrower bind set (e.g. drop the Cargo overlay, bind read-only).
  Rejected: the smith builds and tests Rust (`cargo`) — the overlay is what
  makes that writable-yet-host-safe; reviewers already ship exactly this
  fragment, so reuse is the conservative choice.
- Grant `network: true` (release-manager precedent). Rejected: boxed work
  agents (`analyst`, `clarifier`, `chief-architect`, `intake-sdd`) declare
  `network: false`; the smith has no network need; the release grant serves
  release work and is not this seat's precedent.

### D2 — No production-code change is designed; one conditional repair site

Source inspection says the compiler already implements the required
semantics (Context above). The design therefore changes **no production
line** unless a new test exposes a gap, in which case the smallest repair
consistent with 0043/0046 applies, preserving every refusal string byte
for byte. Candidate repair sites, in likelihood order — and the only
production sites implementation may touch:

1. `agents.rs::compose` — only if hands-over-tools precedence or the
   no-hands diagnostic deviates from the words the specs pin.
2. `bundle.rs::enforce_hands_boundary` — only if harness whole-chain
   admission/refusal deviates.
3. `engine.rs::compose_site` / `hands_command` — only if namespace vs
   harness composition leaks a fragment or placeholder across paths.

*Functions and data that must NOT change* (any edit here is out of
slice): `HandsSpec::parse`/`to_value`, `serve_args`, `mcp_config`,
`box_argv`, `execute_in`, `tool_definition`; the adapter loaders and
`report_under`/`resolve_report` (whole-chain rule, `boundary.is_boxed()`
selection); `stamp_boundary`, `hands_command`'s token expansion, the
`{hands_mcp_json}`/`{hands_args_toml}`-in-harness refusal; both adapters'
`hands.workspace` fragments; `tool_permissions_gap`/`hands_gap`
fail-closed readings; the GPT/Flash agents; `contracts/`, `policy/`,
`fixtures/`, `reference/`.

*Alternatives rejected:*

- "Harden" `compose` to also strip unknown flags or enforce command
  allow-lists. Rejected: that is the withdrawn premise (proposal D1); the
  owed restriction is the filesystem/network boundary, and a new
  enforcement contract is explicitly out of slice.
- Teach Codex a synthetic per-tool flag. Rejected: the adapter's measured
  gap (`tool_permissions.unsupported` with the sandbox-class reason) is a
  field measurement; inventing a flag the CLI does not accept would be
  fabrication, and hands already express the restriction.
- Normalise/refactor refusal strings while touching them. Rejected: the
  no-hands diagnostic is pinned word for word by spec; any repair preserves
  it verbatim.

### D3 — Test plan: where each requirement is proved

All new tests use temporary fixture adapters/agents (never the frozen
`fixtures/` tree, never a live provider, never a nested namespace) and
canonicalized temporary workdirs. Rejected inputs assert diagnostic
**reason text**; positive tests assert concrete compiled/composed facts.

**A. Resolution (`crates/brokkr-runtime/src/agents/tests.rs`).**
Temporary Codex fixture (`tool_permissions: "unsupported"`, models
`astra → <id>`, effort vocabulary incl. `high`) + temporary work agent
with `tools.allow: ["cargo","git"]`, no hands:

- No-hands refusal: compile under namespace refuses; assert the reason
  names seat, agent, provider `codex`, model, and carries verbatim
  `the provider declares tool_permissions unsupported, so the agent's
  restriction to ["cargo", "git"] cannot be expressed and the agent would
  run with MORE power than it declares` (plus the existing
  capability-error explanation; a measured-gap variant asserts the
  supplied reason survives in the same shape).
- Namespace admission: same agent + workspace hands, adapter gains
  `hands.workspace` → compiles; assert recorded hands, namespace boundary,
  Codex workspace fragment present, no tool-list flag — a success
  expectation, not absence-of-error.
- Missing-workspace refusal: hands-declaring agent vs adapter with hands
  absent/unsupported → refuses naming seat/agent/provider/model with
  `the provider declares hands unsupported` and `so the agent's hands
  cannot be put in the box and the agent would run with the harness's own
  tools`; measured `hands_gap` reason preserved.
- A1 precedence (capable-provider fixture): Claude-shaped fixture whose
  native map renders `["cargo","git"]` as `--allowedTools
  Bash(cargo:*),Bash(git:*)` while its workspace fragment carries
  `--allowedTools mcp__brokkr__workspace`; test-only agent with both hands
  and `tools.allow` resolves under namespace with the **entire** workspace
  fragment intact, `--allowedTools` occurring **exactly once** with
  `mcp__brokkr__workspace`, and no argument containing `Bash(cargo:*)` or
  `Bash(git:*)`.

**B. Boundary admission (`crates/brokkr-runtime/src/bundle/model_policy_tests.rs`).**
Same Codex-only hands fixture (+ `hands.harness.work: ["--sandbox",
"workspace-write"]`):

- Harness admission: compiles under harness; launch via `compose_site`
  carries `--sandbox workspace-write`, no MCP server, no tool flag;
  boundary `harness`, hands recorded but unenforced by Brokkr.
- Harness refusal: `hands.harness.work` absent/unsupported → refuses naming
  seat, link, provider, `hands.harness.work`, and the writable-sandbox
  reason citing 0046 rulings 1 and 4; measured gap preserved; workspace
  support does not satisfy it.
- Whole-chain fallback honesty: minimal work seat on the **shipped** smith
  + shipped adapters under harness refuses on link 2 (`claude`,
  `hands.harness.work`, writable-sandbox reason) although Codex alone
  compiles — no guessed fragment, no fallback omission, no combined launch.
  Open/other-boundary behavior untouched (existing tests keep pinning it).

**C. Shipped launch composition
(`crates/brokkr-runtime/src/engine/boundary_tests.rs`, via `compose_site`
+ `hands_command` — the production path, not a helper compared to
itself; runnable without provider/network/namespace; canonical workdir).**
Load the shipped agent + both shipped adapters, resolve under namespace,
inspect each candidate's launch:

- Astra/Codex: model `gpt-6-astra`, effort high; `mcp_servers.brokkr`
  registered with the engine executable + `hands serve` args; decoded serve
  args carry the canonical workdir and the **complete** declared spec
  (network false, both toolchain binds, both Cargo masks); native sandbox
  exactly `--sandbox read-only`; shipped MCP approval config present; and
  nowhere: `workspace-write`, harness fragment, per-tool flag, or an
  unexpanded `{hands_mcp_json}`/`{hands_args_toml}`/`{brokkr}` token.
- Fable/Claude: model `claude-fable-5-1`, effort high; complete ordered
  fragment `--tools ""`, `--strict-mcp-config`, `--mcp-config <expanded
  MCP JSON>`, `--allowedTools mcp__brokkr__workspace`; decoded MCP JSON
  registers brokkr with `hands serve` args carrying workdir + full spec;
  `--allowedTools` exactly once granting only `mcp__brokkr__workspace`; no
  `Bash(cargo:*)`/`Bash(git:*)`; no harness fragment/placeholder. No
  adapter edit required.

**D. Roster and suites (`crates/brokkr-runtime/tests/roster.rs`,
`gpt_flash_shape`, full crate suite).**
New/adjusted pins: the shipped smith resolves `astra,fable` at high/high
with the two binds and no `tools`; the dead-tools-beside-hands assertion
now covers it; remove the `("implementer-engine", "cargo"|"git")` row from
`is_house_tool_grant` so a future tool grant cannot return unaccounted
(the row is inert once `tools` is gone — removing it is the tighter,
fail-closed pin). `shipped_claude_implementer_can_commit` is unaffected
(it seats `implementer`, not the engine smith; the smith commits through
the MCP box, not `Bash(git:*)`). `a_codex_lane_is_chained_only_into_boxed_or_toolless_offices`
gains the smith as another boxed office. `every_shipped_panel…`, the
`gpt_flash_shape` suite (GPT/Flash roster untouched) and the entire
`brokkr-runtime` suite must pass unweakened.

**E. Removal evidence (result notes, per the spec's removal rule).**
Every test in A–C names: the removed protection, the targeted test, the
observed assertion failure **for the claimed reason**, the restoration,
and the passing rerun. In particular: consulting the retired list
reintroduces `Bash(cargo:*)`/`Bash(git:*)` and fails the absence
assertions, while dropping the MCP grant fails the presence assertions
(proving flag spelling alone is not the discriminator); removing either
MCP registration or altering forwarded workdir/binds/network breaks the
matching assertion; replacing Codex's read-only sandbox, or adding any
harness fragment or tool-list flag on Codex, breaks the absence
assertions; altering any part of Claude's required fragment breaks its
preservation assertion. A broken-mutation compile failure, an unrelated
refusal, a fixture-only change, or a comment claiming failure does not
count. Observations are recorded as composition evidence, never as live
Codex behavior or a new enforcement contract.

### D4 — Digests: what moves, what does not, and why

The agent file's digest changes (new chain + hands replacing the tool
list); both adapters' digests do **not** (no adapter edits); the engine
version does not. The manifest identity moves exactly where the agent and
hands identity flow:

- Move: `recipes/triage` (the only shipped bundle seating
  `implementer-engine`, at its `implement:engine` select-case), and its
  composed descendants `recipes/night-shift` and `recipes/gpt-flash`
  (`extends: triage`), in `tests/witness_digests.rs` and
  `bundle/compose_tests.rs` wherever represented — re-measured from actual
  compiles, history blocks attributing the move to issue #307 (Astra/Fable
  hire, effort pins and hands replacing the tool list), old history
  retained, no hash guessed.
- Stay: `recipes/fast`, `bundles/self`, `bundles/verify`, `recipes/node`,
  `recipes/preflight` and all other pins — they do not hire the smith and
  their adapter set is unchanged; any movement there is a stop-and-explain
  signal, not a re-pin.
- The `agents` resolution record (chain + consulted adapter digests) moves
  with the agent digest; the manifest `hands` map gains the smith's sites.

*Alternative rejected:* re-pin by editing expected hashes to whatever the
run prints without reading the diff. That is precisely what the history
blocks and the "measured, not guessed" rule forbid; each moved pin must be
tied to the declared cause.

### D5 — Prose: guide and decision note

- `docs/guides/provider-adapters.md`: the Hands section gains the ruled
  reading — a provider without native per-tool flags serves a restricted
  seat through declared hands; under namespace, writes go through MCP
  hands with Codex's native sandbox read-only; confinement is the existing
  filesystem/network boundary, never a command allow-list; 0043's limits
  stay (native shell readable outside the box, no host-read secrecy, egress
  belongs to the harness); `hands.harness.work` keeps its unboxed meaning
  and admission requirements.
- `docs/decisions/0043-the-hands-are-one-tool.md`: append a dated
  2026-09-21 issue-#307 note recording the operator ruling — the
  allow-list-enforcement claim is withdrawn, no new per-tool contract or
  boundary composition is designed, Codex-limitation and egress facts
  restated — without touching historical text or the `accepted` status.

## Trade-offs accepted

1. **Codex's read-only-outside-the-box view remains** (0043
   consequences): the native shell keeps a read-only view of the host
   while writes go through the box. Accepted because no Codex switch
   removes that shell; documented in the guide rather than fixed here.
2. **The shipped smith refuses under `harness`** (Claude lacks measured
   `hands.harness.work`). Accepted as the fail-closed reading: a guessed
   fragment would be fabrication; the operator's measurement is the
   recorded residual.
3. **Deterministic evidence ≠ live proof.** Compilation, composition and
   removal tests show the ruling is *expressible*; the first live Astra
   implementation (Cargo + Git through Codex's boxed hands, a real commit,
   verify passing) stays the controller's post-landing measurement. No
   gate is instructed to pass on this slice's evidence.
4. **Overlay binds need bubblewrap ≥ 0.10** (same as the reviewers). The
   smith inherits that floor; machines on older bwrap refuse at run start
   per 0043 ruling 7 — existing behavior, not new.
5. **Design carries an expected-zero production diff.** If the tests expose
   a gap, the repair budget is minimal and refusal-text-frozen (D2); that
   residual implementation risk is stated here rather than hidden.

## Risks

| Risk | Likelihood | Mitigation |
|---|---|---|
| Tests expose a real compiler gap | Low (source read says semantics exist) | D2's bounded repair sites; refusal text verbatim; re-validate strict |
| Digest churn collides with main moving the same pins | Low–medium | Re-measure from actual compiles at implementation time; never hand-edit hashes |
| Reviewer finds the prose over/under-claims the box | Medium | Guide + note reviewed against 0043 consequences text; expressibility-vs-live-proof split explicit |
| Exact-coverage gate (100%, literal) on new/changed lines | Owed | Implementation runs `scripts/coverage-exact.sh` on a host outside the box; until then recorded pending, never lowered |
| `cargo`/clippy/fmt/self-bundle verification unavailable in a boxed seat | Certain here | Recorded as pending preparation evidence; implementation runs the full validation list on a capable host |

## Migration

No migration. No contract version, policy table, realm map, or stored
journal shape changes. Bundles compiled before this change that do not
hire the smith keep byte-identical manifests. Runs in flight resume under
their pinned manifest; the new smith applies to compiles after landing.

## Open questions

None blocking: the two triage escalations (restriction shape, path
separation) and returned clarification A1 (MCP grant vs retired grants)
are settled operator rulings encoded in the specs. Recorded residuals
(not design questions): the operator's `hands.harness` measurement for
Claude, and the controller's first live Astra implementation measurement
after landing.
