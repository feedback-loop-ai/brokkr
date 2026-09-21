## Context

Issue #307 asks for the operator's 2026-09-20 hire: the engine smith is Astra,
then Fable, both at `high`. Changing only the roster fails today, and the code
says exactly where. Facts below are read from this tree (HEAD `5ef97115`),
not from the proposal.

**The agent.** `agents/implementer-engine.json` chains `["fable","opus"]`
(both Claude), efforts `high`/`high`, declares
`tools: {"allow": ["cargo","git"], "mcp": []}` and no `hands`. Its charter is
the shared `charters/implementer.md`; limits are 2 attempts, 7200 s.

**Resolution** (`crates/brokkr-runtime/src/agents.rs`). `compose` builds one
candidate's argv. With `agent.hands` set and a boxed boundary it appends the
adapter's `hands.workspace` fragment verbatim (and records it as
`Candidate::hands_fragment`) or refuses with the `hands unsupported` reason;
the `else if let Some(allow)` arm, which is where the `tool_permissions`
refusal lives, is not reached. With no hands and `agent.allow` set it needs
`adapter.tool_permissions` and otherwise refuses with the reason quoted in the
spec. `report_under` passes `boundary.is_boxed()`; unboxed resolution appends
nothing for hands *and* never consults the tool list.

**Codex** (`adapters/codex.json`): `tool_permissions` is the measured gap
(`{"unsupported": "codex-cli 0.148.0 restricts by sandbox CLASS…"}`);
`hands.workspace` is `--sandbox read-only`, the two `-c mcp_servers.brokkr.*`
registrations and `default_tools_approval_mode="approve"`; `hands.harness.work`
is `["--sandbox","workspace-write"]`; `astra` maps to `gpt-6-astra`.
**Claude** (`adapters/claude.json`): `tool_permissions` names `cargo` →
`Bash(cargo:*)` and `git` → `Bash(git:*)` under `--allowedTools`;
`hands.workspace` is `--tools "" --strict-mcp-config --mcp-config
{hands_mcp_json} --allowedTools mcp__brokkr__workspace`; there is **no**
`hands.harness`. The adapter loader (`agents/load.rs`, the `hands` arm) makes
`workspace` mandatory whenever `hands` is an object and reads `hands` absent or
`{"unsupported": …}` as *both* members missing — so "harness work without
workspace hands" is not a representable adapter.

**Compile law** (`crates/brokkr-runtime/src/bundle.rs`).
`enforce_hands_boundary` returns at once for a site without hands and for a
boxed boundary ("what hands mean under a box is decision 0043's law,
unchanged"). Under `harness` it walks *every* candidate and refuses a work
seat whose link has no `hands.harness.work`, naming the seat, the 1-based
link, the provider and decisions 0046 rulings 1 and 4. The resolver-level
refusal is wrapped as `bundle: seat '<seat>': agent … cannot be served by
provider … on model …: <reason>. A capability the provider cannot express
fails compilation here rather than degrading silently at run time`.

**Launch** (`crates/brokkr-runtime/src/engine.rs`). `compose_site` selects by
boundary: `Namespace` → `hands_command` (expands `{hands_mcp_json}`,
`{hands_args_toml}` and `{brokkr}` from `brokkr_protocol::hands::mcp_config` /
`serve_args`, i.e. `hands serve --workdir <wd> --spec <HandsSpec JSON>`);
`Harness` → the candidate's `harness.work` fragment appended to the unboxed
argv, no server; `Open` → the command alone. The two paths share no code and
no data.

**The box** (`brokkr_protocol::hands::box_argv`): empty root; workdir bound
read-write at its own path; declared binds added (`ro`, `rw`, or `overlay`
with masks); private `HOME`/`/tmp`; `--unshare-net` unless the spec grants it;
`CARGO_HOME`/`RUSTUP_HOME` exported when those binds are declared; seat
commits unsigned (0043 ruling 6). When the workdir is a linked git worktree the
common git directory outside it is bound read-write too (0043 ruling 6; the
known-open write set is recorded in `box_argv` and decision 0054).

**Why the roster change alone fails.** Astra's link has no hands, an allow
list, and `tool_permissions` unsupported: `compose` returns the "MORE power"
capability gap at link 1 of every bundle that seats the smith (today only
`recipes/triage`, case `engine`).

**Rulings that bind this design** (issue #307, 2026-09-20/21): the chain is
`astra` then `fable`; the restriction owed is decision 0043's existing
workspace confinement, not a per-tool allow-list, and no per-tool enforcement
contract is designed; the boxed smith uses `hands.workspace` alone while
`hands.harness.work` stays the separate unboxed path; a tool-listed agent with
no hands on a provider that cannot express a list stays refused word for word.
Hosts are Linux and macOS only (decision 0063).

## Goals / Non-Goals

**Goals.** Make the ruled hire compile and compose, by data; prove by tests
that each compile rule the ruling leans on holds *as specified* and that the
shipped smith's actual namespace launch is what the spec says; move only the
digests that legitimately move, with history; make the guide and decision 0043
say what is declared, no more; keep expressibility and live proof apart.

**Non-goals.** No new per-tool enforcement contract, command parsing,
transport semantics or native-tool bypass prevention. No adapter, protocol,
contract, schema, dependency or boundary-vocabulary change. No
`hands.harness.work` for Claude and no guessed fragment. No change to the
GPT/Flash crew, to any other agent, to charters, to the recipe table, or to
`recipes/triage`'s own files. No new decision document and no status change
on any decision. No live Astra run, no publication, no lowered coverage bar.
No seatbelt/container proof: those boundaries are boxed by `is_boxed()` and
not built by the engine at this HEAD.

## Decisions

### D1 — One data edit: rewrite `agents/implementer-engine.json`

The whole production change is this file, replacing `models`, `efforts`,
`tools`, adding `hands`, and keeping `description`, `charter` and `limits`
byte for byte:

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

The `hands` block is `reviewer.json`'s, bind for bind and in the same order,
so one toolchain policy exists for every boxed office that builds Rust. The
description stays: it is rendered in `agents list` and the guide, and nothing
about the office's duty changed. The charter stays, so `library_data.rs`'s
charter pins and the `implementer`/`implementer-engine` shared-charter
assertion are untouched. No `boundary` key: the loader refuses one
(`BOUNDARY_IS_THE_REALMS`) and the realm owns that axis.

*Alternatives rejected.* (a) Keep `tools` beside `hands` "for the record":
`roster.rs::tool_grants_keep_house_tools_explicit_and_effort_never_rises_on_fallback`
already fails an agent with `tools` beside `hands`, and the spec forbids it.
(b) A second agent (`implementer-engine-boxed`): more roster surface, a recipe
rewire, a second `SCOPED_OFFICES`-style exemption question, for no gain.
(c) Keep `opus` as a third link: the ruling is `astra` then `fable`. (d) A bare
`"hands": "workspace"`: no toolchain, so `cargo` is absent from the box's
`PATH`.

### D2 — The declared policy: no network, masked Cargo overlay, read-only rustup

`network: false` matches every boxed work office except release-manager, whose
explicit grant serves publication. The workdir is *not* listed: `box_argv`
binds it read-write itself, and a listed absolute path would be
machine-specific and duplicate it. `~/.cargo` is an overlay (writes stay in a
per-seat upper layer; the host's `~/.cargo/bin` is never written) with both
credential files masked; `~/.rustup` is read-only. `~/.cargo` and `~/.rustup`
declared ⇒ `CARGO_HOME`/`RUSTUP_HOME` are exported and `PATH` reaches the
proxies (`box_argv`), which is the mechanism by which the smith can run
`cargo` at all.

*Rejected.* `network: true` (with the resolver/cert binds): would let the smith
fetch crates and reach any host; nothing in the ruling asks for it and the
review offices work without it. A read-write `~/.cargo` bind: 0043 ruling 2
rejects it as a persistence path. An `rw` bind of the checkout: duplicates the
workspace mount.

*Accepted trade-off.* With no network the smith cannot fetch a crate that is
not already in the cached registry or vendored, and a cold `~/.cargo` fails
`cargo build`. The smith's own result vocabulary (`broken`, `blocked`) is how
that surfaces; the controller's live measurement will show whether Astra
handles it. Also accepted: a single MCP call is capped at
`MAX_TIMEOUT_MS` = 600 000 ms, and the `target/` directory inside the workdir
persists across calls, so long builds proceed incrementally; the boxed verify
gate still runs the full suite.

### D3 — No compiler change; the rules are proved, not rewritten

Reading the code against every scenario, each is already satisfied:

| Rule | Where it lives | Reading |
|---|---|---|
| Tool-listed, no hands, Codex → exact refusal | `agents.rs::compose`, `else if let Some(allow)` arm | `{declared}, so the agent's restriction to {allow:?} cannot be expressed and the agent would run with MORE power than it declares`; `{allow:?}` of a `Vec<String>` renders `["cargo", "git"]` |
| Hands replace the list; fragment appended verbatim | `compose`, `if agent.hands.is_some()` / `boxed` arm | the list is never read there; `hands_fragment` is the adapter's tokens |
| No `hands.workspace` → refusal, never a borrow | same arm (`adapter.hands.ok_or_else`), plus the loader | harness `work` cannot exist without `workspace` (loader), and `compose` never reads `harness` |
| Every chain link needs `harness.work` under `harness` | `bundle.rs::enforce_hands_boundary`, `SeatClass::Work if under_harness` | loops all candidates; message carries seat, link, provider, member, measured reason, 0046 rulings 1 and 4 |
| Namespace never carries the harness fragment; harness never carries MCP | `engine.rs::compose_site` | two disjoint arms |

So **no production Rust line is added in the expected case.** Contingency,
stated so a red test has a rule: if a test in D4 fails for a *real* gap, the
repair is the smallest edit inside the function named above, the diagnostic
text stays byte-identical, decisions 0043 and 0046 semantics are preserved, the
repair gets its own removal proof, and a repair that needs a new contract
(a per-tool grammar, a combined launch, a harness fragment for Claude) stops
the slice and returns `upstream` instead.

*Rejected.* (a) Restructuring `compose` into an explicit `(hands, boxed,
allow)` match "for clarity": churn in a hot, fully covered function, with the
risk of moving diagnostics, to buy nothing the tests do not already buy.
(b) A run-manifest *notice* when a tool list is ignored beside hands: new
recorded behaviour, moves identity for every such agent, not requested.
(c) Making the agent loader refuse `tools` beside `hands`: stronger than
0043 ruling 2 (which keeps the list "for the record"), would change other
operators' agents, and the shipped-roster guard already exists in `roster.rs`.

### D4 — Where each proof lives, and the seam it uses

Each seam is an existing, pure, host-independent one; none needs a process, a
namespace, a network, bubblewrap or a provider.

* **Compile tests** go in `bundle/model_policy_tests.rs` beside
  `the_gate_law_reads_the_boundary_for_sites_that_declare_hands`. Only
  `Bundle::compile_under` names the seat, and the spec requires the refusal to
  identify seat, agent, provider and model. The existing `Fixture` supplies
  `stage`/`compile_roots`/`refusal_under`/`boxed_seat`/`shipped_adapters`; two
  small helpers are added there (see D5).
* **Resolver-level A1 fixture** goes in `agents/tests.rs` beside
  `hands_replace_the_tool_list_with_the_adapters_fragment`, using its `Tree`,
  `claude_body()` and `refusal` helpers. That existing test's
  `!argv.any(|p| p == "--allowedTools")` is sound only because its fragment
  has no such flag; it cannot tell a retired grant from the MCP grant, so it
  is **left alone** and complemented, not edited.
* **Launch tests** go in `engine/boundary_tests.rs`, following
  `the_shipped_codex_harness_work_seat_composes_the_preserved_rejoin`: shipped
  `Library`/`Adapters`, `report_under` + `resolve_report`, then
  `compose_site(BuiltBoundary::Namespace, SeatClass::Work, candidate.argv,
  Some(&spec), Some(&candidate), &workdir, &[], result_path, None)`. That is
  the function `Engine::compose` calls; `resolve_report` output is what
  `bundle.rs` expands and hands to it (`expand_command` only rewrites an exact
  `{brokkr}` token; `hands_command` rewrites the rest, so the two passes agree).
* **Shipped-data pins** go in `tests/roster.rs` (agent bytes, box argv) and
  `tests/adoption.rs` (the triage `engine` case), where the repository already
  pins first hires and recipe resolution.

*Independence.* The expected values are literals written in the test: the
ordered fragments, `--effort high`, the `HandsSpec` JSON. Identity data that
is not the claim under test — the driver prefix and the concrete model ids —
is read from the raw adapter JSON with `serde_json`, never through `Adapters`,
so the resolver is not its own oracle and a legitimate model remap does not
falsely fail a *hands* test. The engine executable is
`std::env::current_exe()`, as in the existing tests. The Codex
`mcp_servers.brokkr.args=[…]` value is TOML, but the engine emits only quoted
strings with `\\` and `\"` escapes, which is valid JSON; the test decodes it
with `serde_json`, and a parse failure is itself a failure. Temporary workdirs
are `std::fs::canonicalize`d before use so expected and produced text agree on
macOS (`/var` → `/private/var`); no test touches `fixtures/` or `contracts/`.

*Rejected.* (a) A fake `codex`/`claude` on `PATH` driven through a spawn:
needs processes and a box, not runnable in the seat or portable to macOS, and
proves the fake. (b) A golden-file argv snapshot: an oracle that follows the
code. (c) Asserting through `hands_command` alone: skips the boundary select
that the spec's "not combined with the harness fragment" claim is about.
(d) One monolithic test per provider: a single failure would hide which claim
broke; the removal proofs need each claim to fail alone.

### D5 — Compile fixtures are the shipped Codex adapter, edited

`fn codex_fixture(edit: impl FnOnce(&mut Value)) -> Value` loads
`adapters/codex.json` as a `Value` and applies `edit`; the test writes it into
the `Fixture`'s `adapters/` directory (its judge/newcomer/silent providers map
other models, so `astra` is unique). Variants used:

| Variant | Edit |
|---|---|
| bare gap | `tool_permissions = "unsupported"` (shipped carries a measured reason; the spec's exact text is the bare form) |
| measured gap | `tool_permissions = {"unsupported": "<reason>"}` |
| no hands | remove `hands` |
| hands gap | `hands = {"unsupported": "<reason>"}` |
| harness work absent | keep `hands.workspace`, `hands.harness = {"gate": […]}` |
| harness work gap | `hands.harness.work = {"unsupported": "<reason>"}` |

Deriving from the shipped bytes keeps the `efforts` vocabulary, `effort_flag`,
model map and workspace fragment truthful to real Codex instead of a
hand-written shape that could drift. A new `Fixture::write_tooled_agent(name,
models, efforts, hands: Option<Value>)` writes a work agent with
`tools: {"allow": ["cargo","git"], "mcp": []}`, chain `["astra"]`, effort
`{"astra": "high"}` and the optional hands (the smith's binds). Test-only; the
shipped roster rule is unaffected. Seat name is the fixture's `work`; agent
`engine-smith`; provider `codex`; model `astra`.

### D6 — A1: distinguish the source of a grant, with a control

The capable-provider fixture (`agents/tests.rs`) is `claude_body()` plus
`hands.workspace = ["--tools","","--strict-mcp-config","--mcp-config",
"{hands_mcp_json}","--allowedTools","mcp__brokkr__workspace"]`, and a boxed
agent carrying `tools.allow = ["cargo","git"]`. Asserted on
`resolution.candidates[0]`: `hands_fragment` equals the fragment exactly and
`argv.ends_with(&fragment)`; `--allowedTools` occurs exactly once, followed by
`mcp__brokkr__workspace`; no token contains `Bash(`. **The control**: the same
agent *without* hands resolves to `--allowedTools Bash(cargo:*),Bash(git:*)`
in the same test, proving the fixture can emit the retired grant and that the
absence above is a consequence of precedence, not of a fixture that could
never produce it. This is the spec's "flag spelling alone does not
distinguish the two sources".

For the *shipped* pair the same property is pinned by literal argv in the
launch tests (D4), where the placeholder-expanded fragment is compared
element for element.

### D7 — What the two launch tests assert

Both resolve `implementer-engine` under `Boundary::Namespace` against the
shipped `agents/` and `adapters/`, pick the candidate by provider, and compose
for a canonical temp workdir `wd` and a result path `wd/results/p.json`.

**Codex (Astra).** Expected argv is `[exe, "driver", "codex", "--"]` (driver
tokens from raw `adapters/codex.json`, `{brokkr}` → `exe`), then `--model
<models.astra from raw JSON> --effort high`, then the shipped fragment as
literals: `--sandbox read-only`, `-c mcp_servers.brokkr.command="<exe>"`, `-c
mcp_servers.brokkr.args=<TOML array>`, `-c
mcp_servers.brokkr.default_tools_approval_mode="approve"`. The decoded array
must equal `["hands","serve","--workdir",wd,"--spec",S]` where `S`, parsed as
JSON, equals the literal `{"kind":"workspace","network":false,"binds":[
{"path":"~/.cargo","mode":"overlay","mask":["credentials.toml","credentials"]},
{"path":"~/.rustup","mode":"ro","mask":[]}]}` — written in the test, not
produced by `HandsSpec::to_value`. Absence assertions over the whole argv:
no `workspace-write`, no `--allowedTools`/`--tools`, no `Bash(`, no
`{hands_`, no `{brokkr}`, no `{result_path}`, and the result path string does
not occur (the harness `gate` fragment that would carry it is not composed).
Also `spawn.env == SpawnEnv::Inherit`, `spawn.rewalk == None`, and
`argv == hands_command(candidate.argv, …)` is *not* the assertion — that would
make the helper its own oracle; the literal vector is.

**Claude (Fable).** Expected argv is the raw driver tokens
(`… "--permission-mode","acceptEdits"`), `--model <models.fable>`, `--effort
high`, then in order `--tools`, `""`, `--strict-mcp-config`, `--mcp-config`, `J`,
`--allowedTools`, `mcp__brokkr__workspace`. `J` parsed equals
`{"mcpServers":{"brokkr":{"command":exe,"args":["hands","serve","--workdir",
wd,"--spec",S]}}}` with the same `S` literal, so **both providers are shown
to receive one hands policy**. `--allowedTools` occurs once; no `Bash(`; no
`workspace-write`; no unexpanded placeholder.

The two literals for `S` and the Claude fragment are deliberate pins of
shipped bytes: an intentional edit to either must update the test, which is
what "preserved" means.

### D8 — Shipped-data pins

* `roster.rs::the_engine_smith_hires_astra_then_fable_inside_declared_workspace_hands`
  reads the raw agent JSON and asserts: `models == ["astra","fable"]`;
  `efforts == {"astra":"high","fable":"high"}`; no `tools` key; no `boundary`
  key (top level or in `hands`); `hands.network == false`; `hands.binds` equals
  exactly the two literal binds; every bind path is `~/`-relative (none absolute
  or checkout-shaped), no bind has mode `rw`; `charter`, `limits` unchanged.
* `roster.rs::the_engine_smiths_box_binds_only_the_workdir_writably`
  (`#[cfg(unix)]`): parses the shipped agent's `HandsSpec`, builds a temp
  home with `.cargo/{credentials.toml,credentials}` and `.rustup`, and calls
  the pure `brokkr_protocol::hands::box_argv` with `GitFacts::default()`. It
  asserts `--unshare-net`; the only `--bind` sources are the private home, the
  private tmp and the canonical workdir; no `--bind-try`; exactly one
  `--overlay-src <home>/.cargo`; `--ro-bind /dev/null` over both credential
  names; `--ro-bind-try <home>/.rustup`; `--setenv CARGO_HOME` and
  `--setenv RUSTUP_HOME`. This is how "writable project access uses the
  existing workspace mount" and "no additional writable host bind" are proved
  against the shipped bytes rather than against 0043's own fixture spec. (It
  deliberately does not model a linked worktree: that adds the git common-dir
  bind, an accepted, documented residual — see Risks.)
* `roster.rs::is_house_tool_grant` loses its `("implementer-engine", "cargo" |
  "git")` arm: with no `tools` it is dead, and a dead allow-list arm would
  read as if the grant still existed. Nothing else in the file changes;
  `a_codex_lane_is_chained_only_into_boxed_or_toolless_offices` now counts one
  more office and passes on its own rule (its `>= 8` floor is untouched).
* `adoption.rs`: `TRIAGE` gains `("implement:engine", "gpt-6-astra",
  "b750b0a4…")` (the implementer charter digest already pinned for `SELF`'s
  `implement`; the engine smith shares that charter), and `expected_argv`'s
  hands arm adds `site == "implement:engine"` so the first-hire argv is pinned
  element for element with the codex hands list. The `implement:` tool-grant
  arm keeps serving `implement:design`. A new
  `triage_engine_case_records_the_smiths_boxed_hands` asserts
  `manifest["hands"]["implement:engine"]` equals the `S` literal,
  `manifest["boundary"]["implement:engine"] == "namespace"`, and
  `manifest["agents"]["implement:engine"]` records `chain == ["astra","fable"]`
  and `provider == "codex"`. Select-case labels are `"{phase}:{case}"`
  (`parse_select`), as the existing `sites()` helper assumes. This is the one
  place the *shipped recipe* is shown to resolve the ruled hire; the compile
  fixtures prove the agent, this proves the recipe.

*Rejected.* Also pinning `implement:chore`/`implement:feature` in `TRIAGE`:
unrelated offices, unchanged, out of scope.

### D9 — Digests: exactly one bundle identity moves

A manifest moves when an agent record it resolves, a hands/boundary entry, or
a pinned file changes. `agents/` is not under any recipe directory, and an
ancestor's digest "covers its own files and its own ancestors — never the
leaf's agent resolution" (`bundle/compose.rs`, comment above the chain loop).
The only bundle whose `implement` seat resolves `implementer-engine` is
`recipes/triage`; `recipes/night-shift` and `recipes/gpt-flash` list
`implement` in `override.seats` and replace the whole seat, so they resolve
neither the smith nor its cases.

| Pin | Moves? | Why |
|---|---|---|
| `tests/witness_digests.rs` `WITNESSES["recipes/triage"]` | **yes** | `manifest["agents"]["implement:engine"]` changes (`agent_digest`: the JSON bytes; `adapter_digest`: consulted providers `{claude}` → `{claude, codex}`; `chain`, `model`, `provider`), and `manifest["hands"]` / `["boundary"]` gain an `implement:engine` entry |
| `bundle/compose_tests.rs::a_composed_bundles_manifest_is_pinned`, the `recipes/triage` digest | **yes** | the same value; the test's own comment says the two agree |
| the other nine `WITNESSES`; `UNCOMPOSED` (`recipes/fast`, `recipes/panel-review`, `bundles/self`, `bundles/verify`) | no | none resolves the smith; no adapter, charter, table or script byte changes |
| `night-shift`, `gpt-flash` | no | override `implement`; ancestor digest excludes agent resolution |

The new value is *measured*, never computed by hand: run
`pinned_bundles_keep_their_recorded_digest` and the composed-manifest test,
copy the reported right-hand value into both, re-run. A different witness
moving is a finding to explain, not a value to paste. Each history block gains
one dated paragraph in the file's existing voice — witnesses: "The 2026-09-21
engine-smith hire (issue #307) moves `recipes/triage` alone: `implementer-engine`
now hires astra then fable at high and declares workspace hands in place of a
tool list, so its resolution record, the consulted adapter set and the `hands`
and `boundary` maps change; the other nine are unchanged." — and the composed
test's comment. Earlier history is not rewritten. `every_witness_manifest_
satisfies_the_v9_contract_it_claims` stays valid (`hands` and `boundary` are
keyed together by construction).

The harness pin `every_shipped_bundle_compiles_under_harness_once_the_
fragments_are_measured` keeps its outcome: `recipes/triage` already refuses at
`analyze:check` (the dialect step), the seat walk is alphabetical (the JSON map
is a `BTreeMap`; no `preserve_order` feature), and `analyze` precedes
`design:chief` (a hands agent whose first link is claude and which would refuse
first) and `implement`. The new smith cannot become the first refusal, so
`assert_refused_at_the_dialect_step` and the counts (15 compile, 6 refuse)
stand. The implementer confirms by running it, not by this argument.

### D10 — Documentation: three files, no new claim

1. **`docs/guides/provider-adapters.md`.** Add `### A restricted work seat on a
   provider with no tool list` at the end of `## Hands`, before `## Resume`.
   It says, in the guide's voice: a provider without native per-tool flags
   (Codex) serves a restricted work seat by the agent's declared hands; under
   `namespace` that requires `hands.workspace`, so writable work goes through
   the MCP `workspace` tool and Codex's native sandbox stays `read-only`; the
   restriction is the existing filesystem and network boundary (declared binds,
   network off unless granted), **not** a command allow-list — the boxed tool
   runs `bash -lc` and nothing here parses or filters commands; the box also
   binds a linked worktree's git directory read-write (0043 ruling 6), so the
   writable surface is the workdir and, for a worktree, the repository's git
   directory — never described as "only the worktree"; Codex's native shell is
   still available read-only outside the box, host-read secrecy is not
   promised, and provider traffic belongs to the harness outside the box (0043
   Consequences); and `hands.harness.work` is a *separate, unboxed* route with
   its own whole-chain admission (0046), never composed with the workspace
   fragment. Two existing sentences in the claude paragraph are corrected
   *in place*: "every hands agent chains `opus`" becomes "every hands agent's
   chain reaches claude (`opus`, or `fable` for the engine smith)". The
   sentence beginning "**claude** declares **no** `hands.harness` member yet"
   and every phrase `crates/brokkr-cli/tests/contributing.rs::the_boundary_
   guides_keep_every_section_and_gained_the_rows` pins are left verbatim; the
   doctor sample lines that `doctor/tests.rs` and `composite/tests.rs` read
   from this guide are not touched; the rename guard reads this file, so no
   retired product or crate name appears.
2. **`docs/guides/agent-library.md`.** The sample `brokkr agents list` row
   becomes `implementer-engine	astra → fable	…` (tab-separated, description
   unchanged). Decision 0045 ruling 2's own enforcement binding names this
   transcript as a roster surface, so it is corrected in the same commit even
   though the spec's literal guide requirement names only the adapters guide.
3. **`docs/decisions/0043-the-hands-are-one-tool.md`.** Append, after
   Consequences, `## Operator note — 2026-09-21 (issue #307)`. It records that
   the operator applied rulings 2 and 5 as written to seat the engine smith on
   astra then fable; that the smith declares workspace hands and no tool list,
   and the restriction is the filesystem/network boundary, so an earlier
   commission's reading that the boxed smith stays limited to `cargo` and `git`
   is **withdrawn** (ruling 2 already says the list is not consulted); that no
   new per-tool contract, command policy or boundary composition is designed or
   implied; that the namespace launch (MCP server + read-only native sandbox)
   and the harness `hands.harness.work` launch are different paths and are not
   combined; that the shipped Fable link has no `hands.harness.work` and so
   refuses under `harness`; and that this applies decisions 0043 and 0046 and
   creates or accepts nothing. The `Status:` line, date line and every ruling
   are untouched; no decision file is added, so `decisions_index.rs` (which
   derives the index from files and `Status:` lines) needs no row.

A single new doc test, `crates/brokkr-cli/tests/contributing.rs::the_engine_
smith_confinement_is_described_as_declared`, reads both documents, collapses
whitespace as the existing test does, and asserts: the new heading and the
phrases "not a command allow-list", "`hands.workspace`", "outside the box",
"`hands.harness.work`" and "separate" in the guide; the note heading, the word
"withdrawn", and "creates or accepts" in 0043; that 0043's first twelve lines
still carry `Status: accepted (operator ruled in chat, 2026-09-03)`; that
ruling 2's sentence "When a site has hands, its tool allow-list is not
consulted" is still present; and that `Status:` occurs once. The strings are
chosen when the prose is written and the test is written from the prose, so
the test pins the claims, not the rendering.

*Rejected.* (a) Editing decision 0045's roster table or its "the smith is
claude" sentence: a decision's text is the operator's; see Open questions.
(b) Editing `recipes/triage/README.md`: it is a pinned recipe file, so an edit
would add a second, unattributable reason for the triage digest to move.
(c) Describing the confinement as "least privilege" or "only the worktree":
false for linked worktrees and for Codex reads.

### D11 — Evidence discipline

* **Removal proofs are procedure, not prose.** For each row of the register
  below the implementer applies the mutation in the working tree, runs *only*
  the named test, records the failing assertion text, restores with `git
  checkout -- <path>`, and re-runs to green. A mutation that fails to compile,
  fails an unrelated test, or changes a fixture instead of production code does
  not count. The register is transcribed into the implementation's evidence
  record (the tasks ledger and the delivery notes), one row per line.
* **Commands** (the seat may lack them; unavailable ones are recorded as
  *pending*, never as passing): `cargo fmt --all -- --check`; `cargo clippy
  --workspace --all-targets --all-features --locked -- -D warnings`; each of
  the seven crates' suites separately with `--all-features --locked`;
  `cargo test --workspace`; `cargo run --locked -p brokkr-cli -- compile
  --bundle bundles/self` (and `bundles/verify`); `openspec validate
  2026-09-21-307-astra-engine-smith --strict --no-interactive`. Exact
  coverage (`bash scripts/coverage-exact.sh`) needs the boundary tests' nested
  namespace and cannot run in the box: it is run on CI or the host, its result
  is *pending* until then, and the threshold is never lowered. The expected
  change adds **no production line**, so coverage cannot fall by construction;
  a contingent repair (D3) adds lines and is covered by its own tests.
* **Workspace suite hang.** The CLI crate's suite has been observed to
  deadlock when its whole binary runs together; a hang is investigated by
  scoping the run to one crate or test, never by skipping a test.
* **macOS.** Every new runtime test is pure (no process, no bubblewrap, no
  seatbelt), uses canonical temp roots, and is not `cfg(windows)`-anything.
  The box-argv test is `#[cfg(unix)]`. To exercise the `/private/var`
  spelling on Linux, run the two launch tests once with `TMPDIR` pointing
  through a symlink; a non-canonical root must not change any expected value
  because tests canonicalize first. No Windows obligation is added (0063).
* **Live proof stays the controller's.** Nothing here runs Astra. The delivery
  notes say: *the ruling is expressible — it compiles, composes and its
  protections fail when removed; it is not proved by a live smith.* The
  pending measurement is named: the first live Astra implementation through
  Codex's workspace hands running Cargo and Git, a real commit, verify
  passing. Notes describe evidence and residuals and never instruct a gate.

## How each requirement is proved

Test names are the ones this design adds; "existing" names are unchanged and
must stay green. `T3`–`T6` are in `bundle/model_policy_tests.rs`, `T1`–`T2` in
`agents/tests.rs`, `L1`–`L2` in `engine/boundary_tests.rs`, `R1`–`R2` in
`tests/roster.rs`, `A1`–`A2` in `tests/adoption.rs`, `Dg` in
`tests/witness_digests.rs` + `bundle/compose_tests.rs`, `Dc` in
`crates/brokkr-cli/tests/contributing.rs`.

| Spec requirement / scenario | Proof | Asserts (reason text where a refusal) |
|---|---|---|
| astra-engine-smith · hires the ruled chain · *both hires resolve* | `T6` namespace half, `A1`, `R1` | candidates `[codex, claude]`, efforts `high`/`high`, `--model` = raw adapter ids, manifest `hands["work"]` = the `S` literal, `boundary == {"work":"namespace"}` |
| … *writable access uses the workspace mount* | `R1`, `R2`, `L1`/`L2` | no absolute/`rw`/`boundary`/`network` in the agent; only workdir, private home/tmp are `--bind` sources; server `--workdir` = canonical `wd` |
| … *inactive tools removed, workspace grant remains (A1)* | `R1` (no `tools`), `T1` (with control), `L1`, `L2`, existing `tool_grants_keep_house_tools_…` | Codex: no per-tool flag; Claude: fragment intact, `--allowedTools` once = `mcp__brokkr__workspace`; no `Bash(` anywhere |
| witnessed identities · *hiring bundle changes identity for the declared reason* | `Dg` | the pinned `recipes/triage` value equals a fresh compile in both files; both history blocks cite #307 |
| … *independent rosters retain guarantees* | existing `every_shipped_panel_seats_at_least_two_providers`, `gpt_flash_shape`, whole runtime suite | unchanged; no exception is added anywhere |
| guide and decision · *allow-list escalation answered*, *composition escalation answered*, *note creates nothing* | `Dc` | phrases and the `Status:` line described in D10 |
| live proof · *deterministic before live*, *quality evidence available or pending* | delivery notes (D11) | expressibility ≠ live; every unrun command named *pending* |
| boxed-work-provider-admission · **no-hands refusal** · *Codex cannot express cargo and git without hands* | `T3` | full string: `bundle: seat 'work': agent 'engine-smith' cannot be served by provider 'codex' on model 'astra': the provider declares tool_permissions unsupported, so the agent's restriction to ["cargo", "git"] cannot be expressed and the agent would run with MORE power than it declares. A capability the provider cannot express fails compilation here rather than degrading silently at run time`; measured variant carries `(<reason>)` after `unsupported` |
| … **namespace hands replace the list** · *same fixture gains hands and compiles* | `T4` success half | `Ok`; `manifest["hands"]["work"]` and `boundary`; candidate `hands_fragment` = the fixture adapter's declared fragment and is the argv tail; no `--allowedTools`; explanatory `expect` message |
| … *unsupported workspace adapter refuses* | `T4` two refusal halves | `the provider declares hands unsupported` **and** `so the agent's hands cannot be put in the box and the agent would run with the harness's own tools`; the measured half also contains `(<reason>)` |
| … *fixture distinguishes MCP grant from retired grants (A1)* | `T1` | as D6 |
| … *no borrowing of `hands.harness.work`* | `T2` | `Adapters::load` error contains `needs 'workspace' as an array of strings` for `hands: {"harness": {"work": […]}}` |
| **harness work is separate** · *admitted by its work fragment* | `T5` | compiles under `Boundary::Harness`; candidate `hands_fragment` empty; `compose_site(Harness, Work, …)` argv ends `--sandbox workspace-write`, has no `mcp_servers.brokkr`, no `--allowedTools`; boundary `harness`; `bundle.hands["work"]` still recorded |
| … *missing harness work refuses* | `T5` | `bundle: seat 'work' link 1 resolves to provider 'codex', which declares no `hands.harness.work` fragment: a capability gap — under the `harness` boundary a work seat with hands writes the tree only under the harness's own writable sandbox as the adapter addresses it (decision 0046 rulings 1 and 4)`; with `(<reason>)` after `fragment` when measured; the fixture keeps `hands.workspace`, proving it does not satisfy the rule |
| … *shipped Fable fallback keeps the whole chain honest* | `T6` harness half | refusal contains `seat 'work' link 2 resolves to provider 'claude'`, `hands.harness.work`, `writes the tree only under the harness's own writable sandbox`, and does **not** contain `link 1` |
| **shipped launch as composed** · Codex | `L1` | D7 |
| … Claude | `L2` | D7 |
| **every protection has removal evidence** | the register below | one row each, restored and re-run |

Also proved by existing tests, not duplicated: the workdir read-write bind, the
overlay/mask/`ro` bind argv and `--unshare-net` for a *fixture* spec
(`hands/tests.rs::the_namespace_is_built_from_an_empty_root_and_binds_what_
the_spec_names`); `every_shipped_agent_resolves_at_compile_time` (chain ≥ 2,
`chosen_index` 0, no notices); the shipped-harness pins discussed under D9.

## Removal-proof register

Each mutation is applied alone, the named test alone is run, the failing
assertion is recorded, the tree is restored, the test is re-run green.

| # | Protection removed | Mutation | Must fail |
|---|---|---|---|
| M1 | tool-list refusal | `compose`: skip the `allow` arm when `tool_permissions` is `None` | `T3`: "expected a compile refusal" |
| M2 | refusal text | `compose`: drop "MORE power" | `T3` exact equality |
| M3 | hands admission | `compose`: replace the `adapter.hands.ok_or_else(…)?` refusal with an empty fragment | `T4` no-hands and hands-gap halves; `T4` success half (fragment absent) |
| M4 | hands precedence | `compose`: `else if let Some(allow)` → a separate `if let` after the hands block | `T4` success half ("MORE power" on Codex); `T1` (`--allowedTools` twice, `Bash(` present) |
| M5 | fragment append | `compose`: remove the `argv.extend(fragment…)` line | `T1` tail; `T4`; `L1`; `L2` |
| M6 | MCP grant kept | `compose`: after the extend, drop a trailing `--allowedTools <x>` pair when `agent.allow` is set | `T1` (presence / exact-once) |
| M7 | harness work rule | `enforce_hands_boundary`: make the `Work if under_harness` arm a no-op | `T5` refusal; `T6` harness half |
| M8 | whole-chain rule | `enforce_hands_boundary`: look at `candidates.first()` only | `T6` harness half (link 2 must refuse) |
| M9 | no borrowing | loader `hands` arm: `workspace` optional | `T2` (`unwrap_err` on `Ok`) |
| M10 | paths never combined | `compose_site` `Namespace` arm also appends `candidate.harness.work` | `L1` (`workspace-write`, vector); `L2` (vector) |
| M11 | Codex MCP registration | remove the `mcp_servers.brokkr.command` pair from `adapters/codex.json` | `L1` vector |
| M12 | Claude MCP registration | remove `--mcp-config {hands_mcp_json}` from `adapters/claude.json` | `L2` vector |
| M13 | forwarded workdir | `serve_args`: emit `"."` for `--workdir` | `L1`, `L2` decoded server args |
| M14 | forwarded policy | drop the `credentials` mask (or set `network: true`) in the agent | `R1`, `R2`, `A2`, `L1`, `L2` (`S` literal) |
| M15 | Codex read-only sandbox | `--sandbox read-only` → `workspace-write` in `adapters/codex.json` | `L1` |
| M16 | retired list absent | restore `tools` in the agent **and** apply M4 | `L2` (`Bash(` appears); `L1`/`T4` (compile refuses "MORE power"); `R1` |
| M17 | ruled chain | revert to `fable`, `opus` or change an effort | `R1`, `A1`, `T6`, `L1`, `L2` |
| M18 | no dead tools beside hands | restore `tools` alone | `R1`; existing `tool_grants_keep_house_tools_…` |
| M19 | no extra writable bind | add `{"path":"/abs/checkout","mode":"rw"}` | `R1`; `R2` (extra `--bind-try`) |
| M20 | pinned digest | revert the triage value in one file | `Dg` left/right |
| M21 | doc claims | delete the guide subsection, the note, or edit the `Status:` line | `Dc` |

These are composition and compile evidence. They do not prove live Codex
behaviour and add no native-tool enforcement contract.

## Risks / Trade-offs

Each is accepted knowingly; none is hidden by the tests.

* **Confinement is filesystem and network, not commands.** Anything runnable
  in the box may run: `bash -lc` with the workdir writable. That is the ruling.
  The tool list it replaces was never a stronger control on Codex (there was
  none) and on Claude protected less than the box (0043 Context).
* **The writable surface is larger than "the worktree".** For a linked git
  worktree the box also binds the repository's common git directory read-write
  so `git commit` works (0043 ruling 6); `box_argv` records the resulting
  ref-store/object-store exposure as known-open (decision 0054). The smith
  commits, so it carries that residual. The guide and note say so; the design
  does not pin it as a feature and does not attempt to narrow it here.
* **Codex's native shell is outside the box.** `--sandbox read-only` stops
  native writes, not native reads: the smith on Astra can read host files,
  credential files included (0043 Consequences). Provider traffic and the
  worktree's contents reaching the provider are the harness's, outside the box
  (0036's axis, unchanged; no secret binding is involved and the box refuses
  one beside hands). Stated in the guide; not fixed.
* **No network.** Uncached crates cannot be fetched; a cold `~/.cargo` fails
  builds. The result vocabulary (`broken`, `blocked`) carries it; the
  controller's live run measures it. Per-call time is capped at 600 s.
* **Resume is cold.** Codex's `work-site` resume shape covers `harness` and
  `not applicable` and *declines* the boxed MCP coordinate
  (`restrictions-unavailable`); Claude's `boxed-workspace` shape is
  `unmeasured` and disabled. A retried or resumed engine attempt therefore
  starts a fresh session on either link. The Claude link behaves the same today
  and Astra had no engine seat before, so nothing regresses; the office's two
  attempts and 7200 s limit are unchanged.
* **A harness realm cannot seat the smith yet.** Under `harness`, link 2
  (Fable) has no `hands.harness.work`, so any bundle seating the smith refuses
  there naming `claude` — deliberately, per the spec, rather than acquiring a
  guessed fragment. Before this change an agent with no hands compiled under
  `harness`; a custom bundle that seats `implementer-engine` under a harness
  realm now refuses. No shipped bundle regresses: `recipes/triage` already
  refuses under `harness` at its dialect step (D9). `open` is unchanged and
  untested beyond the existing law ("the harness's own default").
* **macOS.** Boxed offices already refuse at run start off Linux (0043 ruling
  7; the engine does not build `seatbelt`/`container` at this HEAD). The smith
  adds no new platform limit — `recipes/triage` already boxes its reviewers.
  Tests are pure and run on macOS.
* **Pinned literals couple tests to shipped bytes** (the `S` spec and Claude's
  workspace fragment). Intended: that coupling is what "preserved" means.
  Concrete model ids and driver prefixes are read from raw adapter JSON to
  avoid the same coupling where it proves nothing.
* **The fixture is derived from the shipped Codex adapter.** A legitimate
  edit to Codex's efforts vocabulary flows into the fixtures, which is the
  point; the exact-text tests (`T3`, `T5`) do not depend on those fields.
* **A contingent repair is possible** (D3) and would add production lines and
  a removal proof; the reading above says none is needed, and the design does
  not pre-authorise more than the smallest edit.
* **Digests are unmeasured here.** This seat cannot run Cargo; D9 names
  which pins move and how the values are obtained. If measurement disagrees
  with the analysis, the analysis is what changes.

## Migration Plan

1. Land the data edit, the tests, both digest pins with history, the two
   guide edits and the 0043 note as one commit, so no intermediate state has
   a failing pin or a false guide.
2. Order inside the implementation: tests first against the *current* tree
   where they can pass (`T3`, `T2`, the `T4`/`T5` halves that use fixtures,
   `T1`), then the data edit, then re-pin from reported values, then docs and
   `Dc`, then the removal register.
3. In-flight `recipes/triage` runs pinned the old manifest and cannot resume
   under the new bundle (`manifest_diff` refuses on a changed non-file field);
   finish or park them before landing, or start fresh — the identity rule
   working as designed, not a new refusal.
4. Rollback is a revert of that one commit; no data migrates, no contract or
   schema changes, nothing else reads the agent's old shape.
5. Handoff, recorded pending until real: remote CI on all three operating
   systems, MSRV and exact coverage; the controller's live Astra measurement
   (Cargo and Git through Codex's workspace hands, a real commit, verify
   passing). Signing and push remain the operator's.

## Open Questions

* **Decision 0045's roster text.** Its table row `implementer-engine | fable
  high | opus high` and its sentence "the smith is claude" now describe the
  previous roster. This slice corrects the `agents list` transcript that 0045
  binds (D10.2) but leaves the decision's text, which is the operator's and is
  outside the spec. Suggested follow-up: a dated operator note on 0045 (the
  same form as the 0043 note), on the operator's ruling.
* **`agent-library.md` scope.** D10.2 goes one file beyond the spec's literal
  documentation requirement because the transcript would otherwise be false
  and 0045 binds it. If the operator prefers strict spec scope, drop that row
  edit; nothing else depends on it.
* **`adoption.rs` row and doc test.** D8's `implement:engine` pin and D10's
  `Dc` test are additions beyond the spec's minimum proofs, chosen because the
  recipe-level resolution and the prose claims are otherwise unpinned. Either
  can be cut without touching a spec requirement; the removal register rows
  that name them (`A1`, `A2`, `Dc`) would go with them.
* **Live behaviour.** Whether Astra, given only a `bash`-through-MCP tool and a
  read-only native sandbox, completes engine work end to end is unmeasured
  and belongs to the controller.
