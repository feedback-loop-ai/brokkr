# Design: Astra is the boxed engine smith — issue #307

Status: proposed. Change: `2026-09-21-307-astra-engine-smith`.

Adopt the proposal and both capability deltas committed at `5ef97115`
("specify: seat Astra as the boxed engine smith", then "specify:
distinguish workspace grants from tool lists"), on production base
`5ef97115`. This design is authored by the design phase's sole seat: there
is no council and no position paper behind it, and every ruling below is
mine alone, made against the code as it stands. The proposal's D1–D5 are
the specify seat's readings of the operator's rulings; this document
designs to them and does not move any of them.

What this change is, in one sentence: **one data file changes** —
`agents/implementer-engine.json` — and everything else is tests, two
documentation edits and one dated note, because the compiler already
admits, refuses and composes exactly what the operator ruled. The design's
main job is to prove that sentence against the source, name the tests that
pin it, enumerate every digest that moves and why, and fence what must not
move.

## Context

What the tree holds, read for this design (paths and line numbers are the
evidence a judge can re-read; all line numbers are against `5ef97115`):

- **The hire is refused today for one reason, and it is a data reason.**
  `agents/implementer-engine.json` declares `models: ["fable", "opus"]`,
  `efforts: {"fable": "high", "opus": "high"}`,
  `tools: {"allow": ["cargo", "git"], "mcp": []}` and no `hands`. Seating
  Astra first (`astra` maps to `gpt-6-astra` on `adapters/codex.json`,
  decision 0045) makes the chain `astra → fable`, and the first link
  resolves through the codex adapter, which declares
  `tool_permissions: {"unsupported": "codex-cli 0.148.0 restricts by
  sandbox CLASS, …"}`. With no `hands` on the agent,
  `crates/brokkr-runtime/src/agents.rs::compose` (lines 822–842) hits the
  `else if let Some(allow) = &agent.allow` arm and refuses. Adding hands
  removes the refusal without touching code: when `agent.hands.is_some()`
  (line 799), the tool list is never consulted — decision 0043 ruling 2,
  stated in the comment at lines 801–803.
- **The boxed admission path already exists and is fail-closed.** Under a
  boxed boundary, `compose` requires the adapter's `hands.workspace`
  fragment (lines 804–818) and refuses otherwise with
  `the provider declares hands unsupported … so the agent's hands cannot
  be put in the box and the agent would run with the harness's own tools`.
  Both shipped providers the smith's chain reaches declare it:
  `adapters/codex.json` `hands.workspace` is `["--sandbox", "read-only",
  "-c", "mcp_servers.brokkr.command=\"{brokkr}\"", "-c",
  "mcp_servers.brokkr.args={hands_args_toml}", "-c",
  "mcp_servers.brokkr.default_tools_approval_mode=\"approve\""]`, and
  `adapters/claude.json` `hands.workspace` is `["--tools", "",
  "--strict-mcp-config", "--mcp-config", "{hands_mcp_json}",
  "--allowedTools", "mcp__brokkr__workspace"]`. No adapter byte moves in
  this change.
- **The whole-chain rule already exists.** `report_under`
  (agents.rs:962) maps every chain entry, and `resolve_report`
  (agents.rs:1004–1021) fails on the **first** entry with a gap — a later
  candidate cannot bypass a refusal. Under `harness`,
  `crates/brokkr-runtime/src/bundle.rs::enforce_hands_boundary`
  (lines 2649–2698, reached from `resolve_reference` through
  `enforce_model_policy`'s first statement at bundle.rs:2394, the D33
  gate that judges every mapped hands link before any capability gap)
  walks **every** candidate and requires `hands.harness.work` for a
  work seat, refusing per link with
  `seat '{what}' link {link} resolves to provider '{provider}', which
  declares no `hands.harness.work` fragment…: a capability gap — under
  the `harness` boundary a work seat with hands writes the tree only
  under the harness's own writable sandbox as the adapter addresses it
  (decision 0046 rulings 1 and 4)`. The shipped claude adapter declares
  **no** `hands.harness` member, so a smith chain `astra → fable` under
  harness refuses on link 2 even though link 1 compiles alone. That is
  the spec's demanded outcome; nothing is introduced to make it pass.
- **The launch composition is pure and already shaped.**
  `crates/brokkr-runtime/src/engine.rs::compose_site` (lines 4270–4318)
  selects by boundary: `Namespace` calls `hands_command` (lines 4509–4579),
  which expands `{hands_mcp_json}` through
  `brokkr_protocol::hands::mcp_config` (brokkr-protocol/src/hands.rs:1217)
  and `{hands_args_toml}` through `serve_args` (hands.rs:1229 —
  `["hands", "serve", "--workdir", <workdir>, "--spec", <spec JSON>]`,
  TOML-array-encoded with only `\` and `"` escaped, engine.rs:4563–4570),
  and expands every `{brokkr}` occurrence, including the ones embedded in
  codex's `-c` tokens. `Harness` appends the adapter's class-selected
  `hands.harness` fragment; `Open` appends nothing. A work seat under
  namespace composes **no** `workspace-write` and **no** harness fragment.
- **The hands spec is closed and refuses the boundary word.**
  `HandsSpec::parse` (hands.rs:136–182) admits only `kind: "workspace"`,
  `network`, `binds`; a `boundary` key is refused with
  `BOUNDARY_IS_THE_REALMS` — so the smith's declaration cannot smuggle
  the realm's axis, exactly as the spec requires. `Bind` (hands.rs:199+)
  admits `path` (`~/` allowed), `mode: ro|rw|overlay`, `mask` (bare
  filenames).
- **Precedents for every declared value.** `agents/reviewer.json` ships
  exactly the hands this change declares for the smith: `network: false`,
  `binds: [{"path": "~/.cargo", "mode": "overlay", "mask":
  ["credentials.toml", "credentials"]}, {"path": "~/.rustup", "mode":
  "ro"}]`. `agents/analyst.json`, `clarifier.json`, `chief-architect.json`
  and `intake-sdd.json` ship `network: false` with no binds. The
  release-manager's `network: true` serves release work (crate
  publishing) and is not this smith's precedent. The workspace mount
  itself is supplied by the hands tool, not by a declared bind: decision
  0043 ruling 1 binds the workdir read-write at its own path.
- **Who resolves the smith today.** `grep implementer-engine` over the
  tree: `recipes/triage/bundle.json` only — the `implement` seat's
  `engine` case (`{"agent": "implementer-engine", "class": "work"}`),
  site key `implement:engine`. `bundles/self` seats `implementer` (not
  the smith); `recipes/night-shift` overrides `implement` with an inline
  dsh driver; `recipes/gpt-flash` overrides the case with
  `gpt-flash-implementer-engine`. This is the entire blast radius of the
  data edit and it is why exactly one witness digest moves (DD6).
- **The pin collections.** `crates/brokkr-runtime/tests/witness_digests.rs`
  `WITNESSES[5]` pins `recipes/triage` at
  `d95b41d920e0ca5db3012a4eae51449d16505733c4530a4eb83445dff36f336f`,
  with a dated history comment above it.
  `crates/brokkr-runtime/src/bundle/compose_tests.rs::
  a_composed_bundles_manifest_is_pinned` pins the same value for triage's
  `manifest_digest()`. `compose.rs::resolve` (lines 817–853) computes an
  ancestor's digest with `agents: None`, `drivers: None`, `no_hands` —
  an ancestor's digest **never** covers the leaf's agent resolution — so
  `@compose/0000/fast` inside triage and `@compose/0000/triage` inside
  night-shift and gpt-flash do not move when an agent file moves.
- **The roster guards already police the new shape.**
  `tests/roster.rs::
  tool_grants_keep_house_tools_explicit_and_effort_never_rises_on_fallback`
  refuses dead `tools` beside shipped hands and requires effort to never
  rise on fallback (`astra: high`, `fable: high` satisfies it).
  `roster.rs::a_codex_lane_is_chained_only_into_boxed_or_toolless_offices`
  walks every agent chaining a codex lane and refuses a `tools` object
  beside it — the smith, with hands and no tools, passes it.
  `tests/adoption.rs` pins triage's resolved sites in `TRIAGE` and their
  argv in `expected_argv`; the engine case is not currently pinned there
  (DD7 closes that gap).
- **Test homes with working fixtures.**
  `src/bundle/model_policy_tests.rs::Fixture` writes a temp
  agents/adapters/bundle tree, `write_boxed_agent`, `compile_bounded`,
  `refusal_under`, and `compile_roots` — which can compile a fixture
  bundle against the **shipped** roots, the pattern
  `ruling_4s_own_binding_is_pinned_against_the_shipped_adapters` already
  uses. `src/engine/boundary_tests.rs` loads the shipped library and
  adapters and calls the public `compose_site`
  (`the_shipped_codex_harness_work_seat_composes_the_preserved_rejoin` is
  the template). Both are pure: no provider, no network, no namespace is
  opened, so they run on Linux and macOS alike (decision 0063).
- **Docs under test.** `crates/brokkr-cli/tests/contributing.rs` pins
  kept phrases in `docs/guides/provider-adapters.md` (`A provider
  adapter is **data**`, `## Hands`, `### hands.harness`, the table rows);
  the pins check presence, so additive prose is safe.
  `decisions_index.rs` derives the index from each decision's `Status:`
  line in the first twelve lines — appending a dated note to the end of
  0043 moves nothing. `docs/guides/agent-library.md` prints a sample
  `brokkr agents list` whose `implementer-engine` row says
  `fable → opus`; it is stale the moment this change lands (DD8).

The operator rulings that bind this design, from the commission:

1. The engine smith's chain is **astra then fable**.
2. The restriction this change owes is **decision 0043's existing
   workspace confinement**, not a per-tool allow-list, and **no new
   per-tool enforcement contract is designed here**.
3. The boxed smith uses **`hands.workspace` alone**; `hands.harness.work`
   stays the separate unboxed path.
4. An agent carrying a tool list on a provider that cannot express one,
   with no hands, **stays refused word for word**.

## Goals / Non-Goals

**Goals**

- Ship the ruled smith declaration: `["astra", "fable"]`, both efforts
  `high`, workspace hands with `network: false` and the two toolchain
  binds; no `tools`; charter and limits unchanged; no `boundary`.
- Prove the existing compile rules it rides with reason-bearing,
  seat-naming regressions on temporary fixture adapters — the no-hands
  refusal word for word, namespace admission, the missing-`hands.workspace`
  refusal, and the harness admission/refusal pair — plus the
  whole-chain refusal on the shipped smith and adapters.
- Prove the actual namespace launch composition for both links (Codex's
  MCP registration and read-only native sandbox; Claude's complete
  workspace fragment with exactly one `--allowedTools mcp__brokkr__workspace`),
  by decoding the composed argv, not by comparing a helper to itself.
- Re-measure and re-pin every moved digest with a dated history entry,
  and keep every unaffected pin byte-identical.
- Clarify `docs/guides/provider-adapters.md`, append the dated 2026-09-21
  note to decision 0043, and keep every other claim in the docs true.

**Non-Goals**

- No production Rust change is planned or expected; the proposal's
  escape clause (smallest repair only if a test exposes a gap, refusal
  text preserved verbatim) is carried in DD2 as a constraint, not a
  licence.
- No adapter edit, no protocol, contract, dependency or boundary
  vocabulary change; `contracts/`, `fixtures/`, `policy/`, `reference/`
  and `extensions/` are untouched (frozen).
- No per-tool command enforcement, shell parsing, transport semantics or
  native-tool bypass prevention — in code or in prose.
- No `hands.harness.work` measurement for Claude and no guessed
  fragment: the shipped Fable fallback **refuses** under harness.
- No live Astra smith: this seat has no network; the first live
  implementation is the controller's measurement after landing.
- No roster change beyond the smith: GPT/Flash's separate crew, panel
  diversity and the other 34 library agents stay as they are.

## Decisions

### DD1 — The ruled declaration, exactly

`agents/implementer-engine.json` becomes:

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

Every member is traceable: the models and efforts to the operator's
2026-09-20 ruling (proposal D1); `network: false` to the boxed work
agents' precedent; the two binds to `agents/reviewer.json`, which already
masks both Cargo credential files and keeps the Rust toolchain read-only
(the smith needs `cargo` and `git` to build and commit); the charter and
limits to the proposal's "keep its charter and limits"
(`charters/implementer.md`, digest
`b750b0a401fa7fc1aad5dd929bf136cf961b12d2e11ac9fc67995927ea686ad7`, is
unchanged — `library_data.rs` already asserts the smith shares the
implementer charter). The `tools` object is deleted, not emptied: an
empty `tools.allow` is refused by the loader ("ambiguous between granting
nothing and declaring no restriction"), and a populated one is a dead
Claude-only grant beside hands (0043 ruling 2; the roster rule already
refuses it).

**Alternatives rejected**

- *Keep `tools.allow` beside hands.* Rejected: dead weight the resolver
  ignores on both providers, and the shipped-roster rule
  (`tool_grants_keep_house_tools_explicit_and_effort_never_rises_on_
  fallback`) refuses it. The A1 fixture (DD4) proves the precedence
  mechanism on a test-only agent instead.
- *`network: true`, following the release manager.* Rejected: the
  release manager's grant exists to publish crates; the engine smith
  builds and commits inside the box, and every other boxed implementer
  and judge declares false. A wider grant would also change what the
  manifest records without a ruling.
- *Declare an extra bind for the checkout.* Rejected: workspace hands
  bind the workdir read-write at its own path by construction (0043
  ruling 1); a declared checkout path would be machine-specific and the
  spec forbids it (the "Writable project access uses the existing
  workspace mount" scenario).
- *Declare `boundary`.* Rejected: `HandsSpec::parse` refuses the key
  (`BOUNDARY_IS_THE_REALMS`); the realm owns the word (0046 ruling 1).
- *A `rw` bind for `~/.cargo`.* Rejected: the box needs the Cargo home
  readable for the registry cache and writable only through the overlay
  the existing reviewer precedent already declares; `rw` would widen the
  blast radius with no ruled need.

**Trade-offs accepted:** the smith's box now also mounts the host Cargo
and Rustup trees, masked as declared — the same exposure every reviewer
run already has; and with hands declared, the smith loses the (never
enforced on codex anyway) tool-list refusal semantics — that is the
ruling's point.

### DD2 — No production code changes; the fence around the escape clause

Source inspection says the four rules this change stands on already
exist: `compose`'s hands precedence (agents.rs:799–821), the boxed
`hands.workspace` requirement (agents.rs:804–818), the no-hands
tool-list refusal (agents.rs:822–842), the whole-chain harness law
(bundle.rs:2649–2698), and the namespace composition arm
(engine.rs:4294–4296 with `hands_command`). The design therefore plans
**zero** production-line changes, and the exact-coverage obligation
("every added production line") is satisfied vacuously — the gate still
has to run and report its literal threshold.

The proposal permits a repair "only if the tests expose a gap". This
design constrains that clause so it cannot become a widening:

- Any repair must be the smallest edit consistent with decisions 0043
  and 0046, must preserve both refusal texts character-for-character
  (the tool-list refusal and the hands-unsupported refusal), and must be
  reported in the delivery record with the failing test that exposed it.
- If the gap is in the **specification** rather than the code (a scenario
  that no honest implementation of the current source can satisfy), the
  implementer reports `upstream` rather than bending the code.

**What must not move** (the negative fence, checked by existing pins):
`adapters/codex.json` and `adapters/claude.json` bytes (the fragments are
preserved, and both pin collections would move if they changed — see
DD6's "unaffected" list); `ENGINE_VERSION`; every refusal string quoted
above; the frozen trees (`contracts/`, `fixtures/`, `policy/`,
`reference/`); the other 34 agent files and their charters; every recipe
and bundle file.

**Alternatives rejected:** *pre-emptively harden `compose` (e.g. refuse
tools-AND-hands at load time).* Rejected: it would change refusal
semantics the spec pins verbatim, move adapter-independent behaviour the
witnesses do not cover, and solve a problem the roster rule already
polices on shipped data while A1 polices it on fixtures.

### DD3 — Test allocation: four existing homes, no new files

| New tests | Home | Why |
|---|---|---|
| Fixture compile battery (six tests, DD4) | `crates/brokkr-runtime/src/bundle/model_policy_tests.rs` | The boundary law and capability refusals at compile live here; `Fixture` already offers `compile_bounded`, `refusal_under`, `compile_roots`, `write_boxed_agent`, and the shipped-adapter precedent `ruling_4s_own_binding…`. |
| Namespace launch composition (two tests, DD5) | `crates/brokkr-runtime/src/engine/boundary_tests.rs` | argv/env composition per boundary is this file's subject; it already loads the shipped library and adapters and calls `compose_site` (`the_shipped_codex_harness_work_seat_composes_the_preserved_rejoin`). |
| Roster pin of the ruled hire | `crates/brokkr-runtime/tests/adoption.rs` | This is where the shipped recipes' resolved sites and argv are pinned element-for-element (DD7). |
| Digest re-pins + history | `tests/witness_digests.rs`, `src/bundle/compose_tests.rs` | The two pin collections the spec names. |

**Alternatives rejected:** a new test module or integration file
(rejected: the house keeps each law's proofs beside the law; a new file
would orphan them from the fixtures that already exist); putting the
no-hands refusal in `src/agents/tests.rs` (rejected: that file asserts
at resolver level, where no *seat* name exists, and the spec explicitly
requires the seat to be named); `crates/brokkr-cli` integration tests
(rejected: no binary behaviour changes; the CLI surface is untouched).

### DD4 — One fixture thread, five scenarios, reason text asserted

All fixture scenarios live in `model_policy_tests.rs` and share one
fixture thread, so "the same fixture" in the spec is checkable by
reading one file. One load rule shapes the layout: an abstract model
name maps to exactly one provider per adapters directory
(`load.rs::Adapters` refuses a double mapping outright), so each test's
`Fixture` instance writes only the adapters that test's rows need, and
the **`boxed-worker` agent file is byte-identical across the whole
battery** (test 1's third row adds one sibling agent, `boxed-worker-
two`, beside it — a different name, so the shared thread stays
comparable):

- **`boxed-worker`** — a work agent, `tools.allow: ["cargo", "git"]`,
  efforts `{"astra": "high"}`, chain `["astra"]`, **no hands** (the
  no-hands row), later the **same agent plus hands** (the smith's exact
  spec), and in one row plus the retained tool list (the A1 row).
- **`codex-fixture.json`** — a codex-shaped adapter: `provider: "codex"`,
  trusted, `models: {"astra": "gpt-6-astra"}`, `judges: ["astra"]`,
  `model_flag: "--model"`, `efforts: [… "high" …]`, `effort_flag:
  "--effort"`, `tool_permissions: "unsupported"` **as the bare string**
  (no measured reason — the shipped codex adapter's reason would render a
  parenthetical the spec's exact text does not carry), and a
  `hands.workspace` fragment shaped like codex's real one. Variants are
  written per row: hands absent, `{"unsupported": "<measured reason>"}`,
  and `hands.harness.work` present or absent.
- **`claude-fixture.json`** — written only in the rows that need a
  capable provider (the A1 test, and test 1's third row mapping `opus`):
  a Claude-shaped adapter mapping the row's abstract model to a
  claude-style concrete id,
  with real `tool_permissions` (`flag: "--allowedTools"`,
  `separator: ","`, `names: {"cargo": "Bash(cargo:*)", "git":
  "Bash(git:*)"}`) **and** a `hands.workspace` fragment that includes
  `--allowedTools mcp__brokkr__workspace` — the same flag for both
  sources, so flag-name absence cannot masquerade as proof of precedence
  (proposal D5).

Every fixture is written under `Fixture`'s `tempfile::TempDir`; the
frozen `fixtures/` tree is never read or mutated, no live provider is
consulted, and the temp roots are canonicalized where a path is asserted
(macOS `/var` → `/private/var`). Composition assertions in this file
call the public `crate::engine::compose_site` directly — the same
function the engine calls at spawn.

The six tests and their exact assertions:

1. **`a_tool_listed_work_seat_without_hands_keeps_its_exact_refusal`** —
   `boxed-worker` (no hands) + codex fixture, compiled under namespace.
   Asserts the refusal contains, character for character:
   `the provider declares tool_permissions unsupported, so the agent's
   restriction to ["cargo", "git"] cannot be expressed and the agent
   would run with MORE power than it declares`
   — plus the four names: `seat 'work'`, `agent 'boxed-worker'`,
   `provider 'codex'`, `model 'astra'` (the wrapper at bundle.rs:2085 and
   agents.rs:620–622 supplies them). A second row replays the refusal
   with `tool_permissions: {"unsupported": "<reason>"}` and asserts the
   measured reason appears inside the parentheses — the existing
   diagnostic shape (agents/tests.rs::`a_measured_gap_refuses_exactly_
   as_a_bare_unsupported_does` is the resolver-level precedent). A third
   row answers the spec's no-bypass line at seat level: a sibling agent
   `boxed-worker-two` with chain `["astra", "opus"]` — link 1 on the
   codex fixture, link 2 mapped by the capable Claude-shaped fixture —
   still refuses with the same text naming `model 'astra'`, because
   `resolve_report` returns the first entry's gap before any later
   candidate is considered. Work class cannot bypass either: the seat is
   `class: "work"` and the refusal fires from resolution, before any
   class-based early return could matter.
2. **`the_same_fixture_gaining_hands_compiles_and_records_them`** — the
   same agent plus `hands` (the smith's exact spec), adapter declaring
   `hands.workspace`, namespace. Asserts success **and** facts: the
   candidate's provider is `codex`, model `gpt-6-astra`, effort `high`;
   `bundle.hands["work"]` equals the declared spec; the manifest's
   `hands["work"]` and `boundary["work"] == "namespace"`; the candidate
   argv ends with the workspace fragment and carries no tool-list flag.
3. **`declared_hands_refuse_an_adapter_without_workspace_hands`** — the
   hands-declaring agent against the codex fixture with `hands` absent
   (and, in a second row, `{"unsupported": "<measured hands reason>"}`),
   namespace. Asserts `seat 'work'`, `agent 'boxed-worker'`,
   `provider 'codex'`, `model 'astra'`, and the two clauses
   `the provider declares hands unsupported` and `so the agent's hands
   cannot be put in the box and the agent would run with the harness's
   own tools`; the measured row keeps its supplied reason. Explicitly
   not a tier or malformed-fixture error: the fixture declares trusted
   and loads cleanly.
4. **`hands_replace_the_tool_list_on_a_capable_provider`** (A1) — the
   agent carrying hands **and** `tools.allow: ["cargo", "git"]` against
   the Claude-shaped fixture, namespace. At resolution, asserts the
   complete workspace fragment appears in order (`--tools`, `""`,
   `--strict-mcp-config`, `--mcp-config`, `{hands_mcp_json}`,
   `--allowedTools`, `mcp__brokkr__workspace`); then composed through
   `compose_site` for a canonical temp workdir, asserts the
   `--mcp-config` token parses as the MCP config naming this executable
   with `hands serve` args carrying that workdir and the declared spec
   (the requirement's "after placeholder expansion"); and across both
   forms, `--allowedTools` occurs **exactly once** with argument
   `mcp__brokkr__workspace` and **no** argument contains `Bash(cargo:*)`
   or `Bash(git:*)`.
5. **`a_harness_work_seat_admits_through_its_declared_work_fragment`** —
   the hands agent against the codex fixture declaring `hands.harness.
   work: ["--sandbox", "workspace-write"]`, compiled under `harness`.
   Asserts compile, `boundary["work"] == "harness"`, the recorded hands
   spec, and — composed through `compose_site` — the argv ends
   `--sandbox workspace-write`, contains no `mcp_servers.brokkr` and no
   tool-list flag, and `hands["work"]` stays recorded (unenforced by
   Brokkr, which the test states in its expectation message).
6. **`the_shipped_engine_smith_chain_refuses_under_harness_on_the_fable_
   link`** — the file's own `boxed_seat("implementer-engine", "work")`
   (`{"results": ["pass", "fail"], "class": "work", "agent":
   "implementer-engine"}`), compiled with `compile_roots` against the
   **shipped** `agents/` and `adapters/` roots under `harness` — no
   dialect step, so no earlier refusal can mask the target (the shipped
   `recipes/triage` under harness refuses at `analyze:check` first,
   which is why this scenario needs the minimal seat the spec names).
   Asserts the refusal names `seat 'work'`, **link 2**, `provider
   'claude'`, `hands.harness.work`, and the existing writable-sandbox
   reason citing `decision 0046 rulings 1 and 4` — and that link 1
   (astra/codex) is not the refusal's subject. No planted Claude
   fragment, no fallback omission.

A companion refusal row in test 5 (or a seventh test if clearer):
`hands.harness.work` **absent** while `hands.workspace` stays intact —
the refusal must still fire (workspace support does not satisfy the
harness member), with the same reason text.

### DD5 — Launch composition: decode the launch, never compare a helper to itself

Two tests in `engine/boundary_tests.rs`, both following the shipped-load
pattern: `Library::load(agents)`, `Adapters::load(adapters)`,
`report_under(…, "implementer-engine", Boundary::Namespace)` →
`resolve_report`; then `compose_site(BuiltBoundary::Namespace,
SeatClass::Work, candidate.argv.clone(), hands.as_ref(),
Some(candidate), &canonical_workdir, &roots, "/r/p.json", None)` —
`hands` read off the shipped agent, `roots` a canonical temp dir, a real
result path passed to prove the namespace arm ignores it.

**`the_engine_smiths_astra_launch_registers_the_boxed_workspace_route`**
asserts on the returned `SiteSpawn.argv`:

- argv[0] is this binary; the driver tokens are `driver codex --`;
  `--model gpt-6-astra` and `--effort high` are present as pair;
- the `-c mcp_servers.brokkr.command="<exe>"` token equals this
  executable (embedded `{brokkr}` expansion);
- the `-c mcp_servers.brokkr.args=<array>` token, after stripping the
  `mcp_servers.brokkr.args=` prefix, parses with
  `serde_json::from_str::<Vec<String>>` (the escaping
  `replace('\\', "\\\\").replace('"', "\\\"")` is JSON string escaping
  for path-safe values) into exactly
  `["hands", "serve", "--workdir", <canonical workdir>, "--spec", <spec
  JSON>]`; the `<spec JSON>` element then parses as JSON and equals the
  declared spec: `kind == "workspace"`, `network == false`, `binds ==
  [{path: "~/.cargo", mode: "overlay", mask: ["credentials.toml",
  "credentials"]}, {path: "~/.rustup", mode: "ro"}]`;
- the native sandbox tokens `--sandbox read-only` appear (from
  `hands.workspace`) and `workspace-write` appears **nowhere**;
- the shipped approval configuration
  `mcp_servers.brokkr.default_tools_approval_mode="approve"` remains;
- no `--allowedTools`, no `{hands_mcp_json}`, no `{hands_args_toml}`,
  no `{result_path}` literal anywhere.

**`the_engine_smiths_fable_launch_preserves_the_complete_claude_
fragment`** asserts:

- the driver prefix `driver claude -- --permission-mode acceptEdits`,
  `--model claude-fable-5-1`, `--effort high`;
- the fragment appears in order: `--tools`, `""`,
  `--strict-mcp-config`, `--mcp-config`, <JSON>, `--allowedTools`,
  `mcp__brokkr__workspace`;
- the `<JSON>` parses as `mcp_config`: `mcpServers.brokkr.command ==
  <this executable>` and its `args` decode (plain JSON strings) to the
  same `["hands", "serve", "--workdir", <canonical workdir>, "--spec",
  <spec JSON>]` with the complete spec as above;
- `--allowedTools` occurs **exactly once** across the whole argv and its
  argument is `mcp__brokkr__workspace`; no argument contains
  `Bash(cargo:*)` or `Bash(git:*)`;
- no `workspace-write`, no `{result_path}`, no unexpanded hands
  placeholder, and `SiteSpawn.env == SpawnEnv::Inherit`.

Both tests assert decoded facts (JSON parsing, element-for-element
arrays) rather than `contains` strings, and never call
`hands_command`/`mcp_config`/`serve_args` to build an expectation — the
one thing the spec's "rather than compare a helper to itself" forbids.
The expected constant arrays are spelled out in the test. Both are pure:
no `bwrap`, no provider, no namespace is opened, so they run on Linux
and macOS (decision 0063) and nowhere else is owed.

**Alternatives rejected:** asserting `spawn.argv ==
hands_command(...)` (helper-to-itself — proves nothing); `contains`
probes only (weaker than decoding; would not catch a dropped bind);
driving the real engine to spawn (needs a provider and a box; the spec
wants the composition function's output, which is exactly the public
`compose_site`).

### DD6 — Digest movement: exactly one manifest, both pins, dated history

The only shipped artifact whose identity moves is `recipes/triage`, and
for three recorded reasons inside one edit: its `implement:engine`
resolution record changes (`agent_digest` for the new declaration bytes;
`adapter_digest` now consults `{"claude": …, "codex": …}` instead of
claude alone; `chain`/`model`/`provider` change to the ruled hire), and
its manifest gains `hands["implement:engine"]` and
`boundary["implement:engine"] == "namespace"` beside it. Therefore:

- `tests/witness_digests.rs` `WITNESSES[5]` re-pins `recipes/triage`
  from the **actual compile** (the test's own failure message reports
  left/right — never a guessed hash), and the history comment above
  `WITNESSES` gains a dated paragraph:
  *The 2026-09-21 Astra engine smith ruling (issue #307) moves
  `recipes/triage` alone: the engine case's office now hires
  astra@high → fable@high through boxed workspace hands instead of a
  claude-only tool list, so the `implement:engine` record gains the
  codex adapter and the manifest gains the site's hands and boundary
  keys. The other nine are unchanged.*
- `src/bundle/compose_tests.rs::a_composed_bundles_manifest_is_pinned`
  re-pins the same measured value with the same dated paragraph in its
  inline comment, restating that it agrees with the
  `tests/witness_digests.rs` pin.
- **Nothing else moves.** `@compose/0000/fast` (triage's ancestor) does
  not cover the leaf's agent resolution (compose.rs:817–839); `recipes/
  night-shift` and `recipes/gpt-flash` do not resolve the smith (their
  `implement` seats override the case away); `bundles/self`, `recipes/
  panel-review`, `bundles/verify`, `recipes/fast` never consult
  `implementer-engine`; no adapter, charter, recipe, script, policy or
  engine version moves. `UNCOMPOSED` stays byte-identical, and the
  unaffected nine `WITNESSES` stay byte-identical — the regression is
  the point.
- `every_witness_manifest_satisfies_the_v9_contract_it_claims` keeps
  passing by construction: `hands` and `boundary` are written in one
  loop over the same keys (bundle.rs:3837–3846).

**Alternatives rejected:** re-measuring and re-pinning *every* witness
"to be safe" (rejected: it destroys the very regression — an unaffected
pin that moved means something unruled happened); recording the move
only in the commit message (rejected: the spec demands the history
blocks carry it).

### DD7 — Pin the ruled hire where it ships

`tests/adoption.rs::TRIAGE` gains one row:
`("implement:engine", "gpt-6-astra", "b750b0a401fa7fc1aad5dd929
bf136cf961b12d2e11ac9fc67995927ea686ad7")` — the implementer charter
digest, which `library_data.rs` already asserts the smith shares.
`expected_argv` learns that `implement:engine` is a hands site on the
codex provider: it expects the codex workspace fragment
(`--sandbox read-only`, the three `-c` tokens with the placeholders
still literal — resolution composes them unexpanded, and
`assert_adopted` normalizes the expanded `argv[0]` back to `{brokkr}`
as every row does) and **no** `--allowedTools` grant, while
`implement:design`, `implement:chore` and `implement:feature` keep
their `Bash(cargo:*),Bash(git:*)` arms (their agents still declare
tools and no hands). This makes the shipped resolution of the ruled hire
— model, effort, fragment, charter — fail loudly if any of it drifts,
in the file whose entire subject is "the roster resolves to what it
pinned".

**Alternatives rejected:** relying on the fixture battery and digest pin
alone (rejected: neither asserts the *argv* the shipped recipe actually
resolves; adoption.rs exists precisely for that); pinning gpt-flash's
engine case too (unnecessary: it is a different agent, already pinned
by `gpt_flash_shape.rs`).

### DD8 — Documentation: two guided edits, one dated note, one truth repair

- **`docs/guides/provider-adapters.md`** — an additive paragraph in the
  `## Hands` section (after the `hands.harness` subsection's own prose,
  before `## Resume`): a provider without native per-tool flags serves a
  restricted work seat through declared hands — under `namespace` it
  must declare `hands.workspace`; writable work happens through the MCP
  `workspace` tool while the native Codex sandbox stays read-only; the
  confinement is the existing filesystem-and-network boundary (empty
  root, declared binds, network flag), **never** a command allow-list;
  and the two routes stay distinct — namespace composes the MCP server
  plus the read-only native sandbox, `hands.harness.work` addresses the
  harness's own writable sandbox with no Brokkr box. The paragraph keeps
  0043's limits word-for-word where they already bind: the native shell
  remains available read-only outside the box, host-read secrecy is not
  promised, provider traffic belongs to the harness outside the box.
  Additivity keeps `contributing.rs`'s kept-phrase pins green.
- **`docs/decisions/0043-the-hands-are-one-tool.md`** — a dated section
  appended at the end:
  `## Note — 2026-09-21, issue #307: the engine smith's hands are the
  box, not a tool list`. It records: the operator's 2026-09-21 ruling
  (the engine smith hires astra then fable at high effort and declares
  workspace hands with the two toolchain binds); the **withdrawal** of
  the earlier commission's claim that the change would enforce a
  `cargo`/`git` allow-list (workspace hands execute shell commands
  subject to the existing filesystem and network boundary, so the smith
  is not limited to commands named `cargo` and `git`); that **no new
  per-tool enforcement contract and no boundary composition is
  designed** — the note applies existing decisions 0043 and 0046 and
  changes neither's semantics; and that expressibility is not live
  proof (the controller's first live Astra measurement is pending).
  The `Status:` line, every heading above and every historical ruling
  stay untouched — append-only, so `decisions_index.rs` is unaffected.
- **`docs/guides/agent-library.md`** — the sample `brokkr agents list`
  row becomes `implementer-engine	astra → fable`. Not named by the
  proposal, but the block claims to show the machine's output; leaving
  it would make the guide lie about the roster the day the change lands.
  No test pins the line; the edit is one row. Recorded here as a
  deliberate, visible scope addition rather than smuggled drift.
- **`docs/decisions/0045-astra-is-a-judge.md`** stays untouched: its
  roster table is dated history (the decision's own measurement), not a
  living reference, and only 0043 carries a sanctioned append.

**Alternatives rejected:** rewriting the Hands section wholesale
(rejected: the kept-phrase pins and the settled prose both argue for
addition); putting the ruling in a new decision document (rejected: the
spec says a dated note on 0043 without changing its status — a new
decision would assert new semantics this change explicitly does not
design); skipping agent-library.md (rejected: stale machine claims are
the defect decision 0001 exists to prevent).

### DD9 — Evidence boundary and the shape of the result notes

The delivery record must say: the ruling is **expressible** (compiles,
composes, is pinned) and **not proved by a live smith**; the first live
Astra implementation remains the controller's measurement after landing
— Cargo and Git through Codex's workspace hands, a real commit, verify
passing — recorded as pending; formatting, clippy with warnings denied,
each crate's suite, `cargo test --workspace`, both bundle compiles
(`bundles/self`, `bundles/verify`) and strict OpenSpec validation are
the quality gates; `scripts/coverage-exact.sh` runs on the host/CI
outside the namespace box and stays a literal 100% with no lowered
threshold, recorded as pending until its external result exists. The
notes describe evidence and residuals and never instruct a gate to pass.

## Risks / Trade-offs

- **The smith cannot run under `harness` realms.** Link 2 (fable/claude)
  lacks `hands.harness.work`, so a harness realm refuses the chain. This
  is the standing state for every boxed work agent whose chain reaches
  claude (the reviewer today), it is the spec's demanded outcome, and
  the remedy is the operator's Claude measurement — never a guessed
  fragment. Trade-off accepted and pinned by test 6.
- **Deterministic evidence is all this change claims (proposal D2/D4).**
  No live Codex or Claude behaviour is claimed; the launch
  tests are composition evidence only. The record says so (DD9).
- **Digest measurement is a real dependency.** The implementer must run
  the suite and read the failure's left/right pair; a guessed hash fails
  the pin by construction. On a box without cargo (like the specify
  seat's), the re-pin is impossible and must be reported as pending
  rather than guessed.
- **Removal evidence mutates production rules temporarily.** A botched
  restoration would break the suite; mutations run one at a time, are
  never committed, and the full suite re-runs after the last
  restoration. The mutations are enumerated in the migration plan so the
  evidence run is mechanical, not improvised.
- **`every_shipped_bundle_compiles_under_harness_once_the_fragments_
  are_measured`'s triage pin** still refuses at `analyze:check` under
  harness because phases compile in name order (`analyze` < `implement`)
  — the new hands on `implement:engine` do not surface there. If seat
  ordering ever changes, the engine-case refusal would surface first;
  noted here so a future mover of that pin knows why.
- **The `agent-library.md` row edit is scope the proposal did not
  name.** Kept to one row, justified in DD8; flagged for the operator in
  the delivery record.

## Migration Plan

Implementation order, each step leaving the suite green before the next:

1. **Edit `agents/implementer-engine.json`** to DD1's exact bytes.
2. **Re-measure and re-pin** — run `cargo test -p brokkr-runtime`; the
   `recipes/triage` pins in `witness_digests.rs` and `compose_tests.rs`
   fail with left/right pairs; record the measured values and the dated
   history paragraphs (DD6). `every_bundle_in_the_tree_compiles`,
   `every_shipped_agent_resolves_at_compile_time` and the roster rules
   must already be green at this point — if any is not, stop and
   investigate before writing any new test (a red existing pin here
   means the declaration is wrong, not the pin).
3. **Extend `adoption.rs`** with the `implement:engine` row and the
   hands arm in `expected_argv` (DD7).
4. **Add the fixture battery** in `model_policy_tests.rs` (DD4), then
   **the two launch tests** in `engine/boundary_tests.rs` (DD5).
5. **Docs**: the provider-adapters paragraph, the 0043 note, the
   agent-library row (DD8).
6. **Full gates**: `cargo fmt --all -- --check`;
   `cargo clippy --workspace --all-targets --all-features --locked --
   -D warnings`; `cargo test -p <crate> --all-features --locked` for
   each of the seven crates; `cargo test --workspace --all-features
   --locked`; `cargo run --locked -p brokkr-cli -- compile --bundle
   bundles/self` and `--bundle bundles/verify`;
   `openspec validate 2026-09-21-307-astra-engine-smith --strict
   --no-interactive` (and `--all --strict`); `bash
   scripts/coverage-exact.sh` on the host or CI outside the namespace
   box — recorded pending until that external result exists, threshold
   literal.
7. **Removal evidence** — one mutation at a time, never committed, full
   suite re-run after the final restoration. Each row: mutation →
   targeted test → the specific assertion that fails (not a compile
   error, not an unrelated refusal) → restoration → passing rerun:

   | # | Protection (site) | Mutation | Fails in | Failing assertion |
   |---|---|---|---|---|
   | 1 | Tool-list refusal, agents.rs:822–842 | make the no-hands arm fall through without refusing (skip the `ok_or_else`) | test 1 | the refusal-text assertion: compile now succeeds |
   | 2 | Measured reason preserved, agents.rs:827–832 | drop the `({reason})` branch | test 1, measured row | the measured-reason assertion |
   | 3 | Boxed hands requirement, agents.rs:804–818 | append nothing and return `Ok` when `adapter.hands` is absent | test 3 | the hands-unsupported refusal assertion |
   | 4 | Hands-over-tools precedence, agents.rs:799–822 | consult `tools.allow` even when hands exist (append the grant after the fragment) | test 4 and the Fable launch test | `--allowedTools` count becomes two; `Bash(cargo:*)` appears |
   | 5 | Workspace fragment appended, agents.rs:819–820 | skip `argv.extend(fragment)` | tests 2, 5 and both launch tests | the fragment-presence assertions; no MCP registration |
   | 6 | Whole-chain harness law, bundle.rs:2649–2698 | check only the first link (`break` after link 1) | test 6 | the link-2/claude refusal assertion: compile now succeeds |
   | 7 | Namespace composition arm, engine.rs:4294–4296 | return the uncomposed command | both launch tests | unexpanded `{hands_args_toml}`/`{hands_mcp_json}` assertions |
   | 8 | Declared hands data (agent file) | drop the `~/.rustup` bind; flip `network` to true; drop one Cargo mask | both launch tests | the decoded spec's binds/network assertions |
   | 9 | Codex native sandbox (adapter data) | `--sandbox read-only` → `workspace-write` in `adapters/codex.json` | the Astra launch test and the adoption row | the read-only assertion / absence-of-`workspace-write` |
   | 10 | Claude workspace fragment (adapter data) | drop `--allowedTools mcp__brokkr__workspace` from `adapters/claude.json` | the Fable launch test and test 4's fixture expectation | the exactly-one-grant presence assertion |
   | 11 | Claude fragment completeness (adapter data) | drop `--strict-mcp-config` (and, separately, `--tools ""`) | the Fable launch test | the in-order fragment assertion |
   | 12 | Harness fragment separation, engine.rs:4297–4315 | append the harness work fragment under namespace too | both launch tests | the absence-of-`workspace-write`/fragment assertions |

   Rows 8–11 are temporary local edits of shipped **data** — restored
   byte-identically afterwards (the witness pins double as the
   restoration proof: a mis-restored adapter or agent file moves
   `recipes/triage` again and fails the re-pin).

8. **Delivery record** — result notes per DD9: expressible-not-live, the
   controller's pending measurement, the gates' status, pending external
   exact coverage, the removal-evidence table above with observed
   outcomes, and the `agent-library.md` scope note.

## How each requirement is proved

### `astra-engine-smith`

| Requirement / scenario | Proof |
|---|---|
| The engine smith hires the ruled chain… — *The shipped engine smith resolves both ruled hires* | `adoption.rs` `TRIAGE` row `implement:engine` (DD7) plus `every_shipped_agent_resolves_at_compile_time` (existing, now exercises the ruled chain) — Astra/Codex first, Fable/Claude second, both high, hands recorded (`bundle.hands`/manifest `hands`/`boundary` = namespace via `every_witness_manifest_satisfies_the_v9_contract…` and the re-pinned triage manifest itself). |
| — *Writable project access uses the existing workspace mount* | The Astra/Fable launch tests (DD5): the decoded `serve` args carry the canonical workdir and **exactly** the two declared toolchain binds with both Cargo masks, so no checkout path and no additional writable bind can be reaching the box; `network` decodes false; and `HandsSpec::parse` refuses a `boundary` key structurally, while `roster.rs::a_codex_lane_is_chained_only_into_boxed_or_toolless_offices` already walks every agent chaining a codex lane and refuses a `tools` object beside it. Removal row 8. |
| — *The inactive tools declaration is removed… (A1)* | `library_data.rs::the_library_holds_the_decision_0041_roster` (unchanged names, shared charter) with the A1 fixture (test 4) proving precedence, and the Fable launch test proving the shipped candidate carries exactly one `--allowedTools`, the MCP grant, and no `Bash(cargo:*)`/`Bash(git:*)`; the Astra launch test proves no per-tool flag on Codex. Removal: rows 4, 9, 10. |
| Changed hires and hands are witnessed… — *A hiring bundle changes its identity for the declared reason* | The re-pinned `WITNESSES[5]` and compose pin, measured, with the dated history paragraphs (DD6). |
| — *The independent rosters retain their guarantees* | `every_shipped_panel_seats_at_least_two_providers`, the whole `gpt_flash_shape` suite and the full crate suite run green with no weakening (the GPT/Flash engine smith is a distinct agent, untouched; the dead-tools rule is unchanged and now covers the smith). |
| The guide and decision record… — *The allow-list escalation is answered…* | The provider-adapters paragraph and the 0043 note say the confinement is the existing filesystem/network boundary and withdraw the earlier enforcement claim (DD8); neither artifact adds command parsing, transport semantics or native-tool bypass prevention — reviewable as the diff. |
| — *The composition escalation is answered without combining paths* | The same paragraph's two-route sentences (namespace = MCP server + read-only native sandbox; harness work = its declared writable fragment, no Brokkr box), pinned by tests 5 and 6 and both launch tests. |
| — *The note does not create or accept a new semantic rule* | The 0043 diff is append-only: `Status: accepted` unchanged, rulings untouched (verified by `decisions_index.rs` and by reading the diff). |
| Deterministic evidence does not stand in… — *A deterministic composition passes before any live measurement* | Result notes per DD9 (expressible, not live; controller's Cargo/Git/commit/verify measurement pending). |
| — *The required quality evidence is available or explicitly pending* | Migration steps 6–8: the named gates, with external exact coverage recorded as pending, never a pass or a lowered threshold; tests use canonical temporary roots, add no Windows obligations (0063) and write no frozen bytes (fixtures are `Fixture` temp trees). |

### `boxed-work-provider-admission`

| Requirement / scenario | Proof (test → assertion → removal row) |
|---|---|
| A tool-listed work seat without hands… — *Codex cannot express a cargo and git list without hands* | Test 1 → the exact refusal text (quoted in DD4) plus seat/agent/provider/model names; measured-reason row keeps the supplied reason → removal rows 1–2. |
| Namespace hands replace the tool list… — *The same tool-listed fixture gains hands and compiles* | Test 2 → success with recorded hands and namespace, candidate facts (provider/model/effort, fragment appended) → removal rows 3, 5. |
| — *Declared hands cannot use an unsupported workspace adapter* | Test 3 → the two hands-unsupported clauses with the four names; measured row keeps its reason; a capability refusal, not a tier or fixture error → removal row 3. |
| — *A fixture distinguishes the MCP grant from retired tool-list grants (A1)* | Test 4 → complete fragment in order, `--allowedTools` exactly once with `mcp__brokkr__workspace`, no `Bash(cargo:*)`/`Bash(git:*)` → removal rows 4 and 10. |
| Harness work is a separate case… — *The same Codex-only hands agent is admitted by its harness work fragment* | Test 5 → compiles under the existing rule; launch carries `--sandbox workspace-write`, no MCP server, no tool flag; boundary stays harness; hands recorded but unenforced → removal rows 5, 6, 12. |
| — *Missing harness work remains a reason-bearing refusal* | Test 5's refusal row (workspace intact, work absent/unsupported) → the link, provider, `hands.harness.work`, the 0046 rulings 1-and-4 reason; measured reason preserved → removal row 6. |
| — *The shipped Fable fallback keeps the whole chain honest* | Test 6 → refusal on link 2, provider `claude`, naming `hands.harness.work` and the writable-sandbox reason, with link 1 admitted; no guessed fragment → removal row 6. |
| The shipped engine smith's namespace launch… — *The complete Codex namespace launch…* | Astra launch test → the decoded argv/spec/sandbox/approval facts and the absence set (DD5) → removal rows 7, 8, 9, 12. |
| — *The shipped Claude launch preserves its complete workspace fragment (A1)* | Fable launch test → the in-order fragment, decoded MCP JSON, exactly one `--allowedTools` with only the MCP grant, no retired grants, no placeholder → removal rows 7, 10, 11. |
| Every claimed compile and composition protection has removal evidence | The migration plan's twelve-row table: each names the removed protection, the targeted test, the specific failing assertion, the restoration and the passing rerun; rejected inputs assert their diagnostic reason (never `is_err()`); positive tests assert concrete facts; no production rule is weakened (DD2's fence). |

## Open Questions

- **O1 — The measured digest.** The new `recipes/triage` manifest digest
  exists only after an actual compile; the design pins the *procedure*
  (read the failing test's left/right pair) and forbids guessing. Not
  answerable at design time.
- **O2 — Claude's `hands.harness.work`.** Unmeasured; the smith (and
  every boxed chain reaching claude) refuses under harness realms until
  the operator lands a measurement. Out of scope here by ruling; the
  refusal is pinned, not papered over.
- **O3 — The first live Astra smith.** The controller's measurement
  after landing (Cargo/Git through the boxed hands, a real commit,
  verify passing). This change's evidence is expressibility only.
- **O4 — The `agent-library.md` row.** Included as truth maintenance
  (DD8); if the operator rules the guide's sample output frozen, the
  row is the one line to revert — recorded here so the choice is
  explicit rather than silent.
