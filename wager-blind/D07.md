# Design — Astra as the boxed engine smith (issue #307)

## Context

Commission: the solo design seat of change
`2026-09-21-307-astra-engine-smith` (run
`design-wager-issue-307-astra-as--119de78b`). The proposal and both
capability deltas (`specs/astra-engine-smith/spec.md`,
`specs/boxed-work-provider-admission/spec.md`) are adopted, clarified and
ruled clear; this phase authors `design.md` only. The operator rulings that
bind the design, quoted from the commission: the smith's chain is
`astra` then `fable`; the restriction the change owes is decision 0043's
existing workspace confinement, not a per-tool allow-list, and no new
per-tool enforcement contract is designed here; the boxed smith uses
`hands.workspace` alone and `hands.harness.work` stays the separate unboxed
path; an agent carrying a tool list on a provider that cannot express one,
with no hands, stays refused word for word. Proposal decisions D1–D5 are
adopted as settled; this design turns them into named functions, data,
tests and pins.

The code the specification touches, read before deciding (every citation
re-verified against the working tree at the design phase):

- `crates/brokkr-runtime/src/agents.rs` — `compose` (714–899) is the whole
  capability law. The hands branch (798–821) fires whenever
  `agent.hands.is_some()`; under a boxed boundary it requires
  `adapter.hands`, appends the fragment into `argv`, records
  `hands_fragment`, and never reads `agent.allow`. The `else if let
  Some(allow)` branch (822–857) is reachable only with no hands: it refuses
  with `the provider declares tool_permissions unsupported…, so the agent's
  restriction to {allow:?} cannot be expressed and the agent would run with
  MORE power than it declares` (823–842), a measured gap riding the same
  sentence in parentheses. The hands gap refuses with `the provider declares
  hands unsupported…, so the agent's hands cannot be put in the box and the
  agent would run with the harness's own tools` (804–818).
  `entry_for`/`report_under` (901–989) pass `boundary.is_boxed()` as the
  `boxed` argument, so the boundary alone selects the branch;
  `resolve_report` (1004–1095) refuses on ANY chain entry's gap (1009–1013),
  not only the chosen one. `ResolveError::Capability`'s display names agent,
  provider and model and closes with the fixed explanation sentence
  (619–629) — but no seat: the seat name enters only in `bundle.rs`.
- `crates/brokkr-runtime/src/bundle.rs` — the seat wrapper prefixes
  `seat '{what}': ` onto every resolver error (2085 and 2124), so a
  compiled refusal names the seat, agent, provider and model in one string.
  `enforce_hands_boundary` (2610–2700) is the unboxed half of the law: it
  returns at once for boxed boundaries (2616–2618); under `harness` a
  hands-declaring work seat needs `hands.harness.work` on every link
  (2682–2693), refusing with `seat '{what}' link {link} resolves to provider
  '{provider}', which declares no \`hands.harness.work\` fragment…: a
  capability gap — under the \`harness\` boundary a work seat with hands
  writes the tree only under the harness's own writable sandbox as the
  adapter addresses it (decision 0046 rulings 1 and 4)`; under `open` a
  work seat runs at the harness's default (2695–2697).
- `crates/brokkr-runtime/src/engine.rs` — `compose_site` (4270–4318)
  selects the launch by `BuiltBoundary`: `namespace` wraps the whole
  command in `hands_command` (4296–4298); `harness` appends the
  class-selected `hands.harness` fragment to the unboxed argv, expanding
  `{result_path}` and `{brokkr}` (4299–4313); `open` inherits the bare
  command. The engine's own `compose` (1448–1488) feeds a candidate's argv
  as the command and the selected `Candidate` as the link, so composing
  `candidates[i].argv` with `Some(&candidates[i])` mirrors production.
  `hands_command` (4509–4579) expands `{brokkr}`, `{hands_mcp_json}` and
  `{hands_args_toml}` at compose time (no spawn), the latter built from
  `brokkr_protocol::hands::serve_args` with `\` and `"` escaped.
- `crates/brokkr-protocol/src/hands.rs` — `HandsSpec` (117–197) is
  `{network, binds}` with `Bind{path, mode, mask}`; `parse` refuses
  `boundary` beside hands (153–155) and every unknown key (156–162).
  `execute_in` (1066–1121) runs `/bin/bash -lc <command>` inside
  `box_argv`'s empty root — no command allow-list exists anywhere on the
  path. `mcp_config` (1217–1226) registers one server `brokkr`
  (`SERVER_NAME`, 35) with `command` and `args: serve_args(...)`;
  `serve_args` (1229–1238) is exactly `hands serve --workdir <workdir>
  --spec <spec JSON>`.
- `crates/brokkr-runtime/src/agents/load.rs` — `parse_agent` (480–562)
  whitelists exactly `description, charter, models, efforts, tools, hands,
  limits, inputs` (487–498), parses `hands` through `HandsSpec::parse`
  (536–544), and takes the agent digest over the whole source bytes
  (560) — any byte of the declaration moves every hiring bundle's
  recorded identity.
- `adapters/codex.json` — `tool_permissions: {"unsupported": …}` naming the
  sandbox-class axis (33–35); `hands.workspace` = `--sandbox read-only` +
  three `-c` `mcp_servers.brokkr.*` tokens incl.
  `default_tools_approval_mode="approve"` (38–47); `hands.harness.work` =
  `["--sandbox", "workspace-write"]` (55–58); `mcp: "unsupported"` (36).
  `adapters/claude.json` — capable `tool_permissions` mapping `cargo`/`git`
  to `Bash(cargo:*)`/`Bash(git:*)` (33–50); `hands.workspace` = `--tools ""`,
  `--strict-mcp-config`, `--mcp-config {hands_mcp_json}`,
  `--allowedTools mcp__brokkr__workspace` (55–65); no `hands.harness`
  member at all.
- `agents/implementer-engine.json` — today `models ["fable","opus"]`,
  efforts high/high, `tools {allow ["cargo","git"], mcp []}`, charter
  `charters/implementer.md`, limits `{2, 7200}`. It shares the implementer
  charter byte for byte (`tests/library_data.rs` 211–214 pins that).
- Consumers: `recipes/triage/bundle.json` seats the smith at
  `implement`'s `engine` select case (line 29); `recipes/night-shift` and
  `recipes/gpt-flash` extend `triage` (their `@compose/0000/triage`
  ancestor digest rides their identity; gpt-flash's own engine case seats
  the untouched `gpt-flash-implementer-engine`). `bundles/self` seats
  intake/implementer/reviewer and `bundles/verify` seats no library agent —
  neither hires the smith. `tests/roster.rs` refuses dead `tools` beside
  `hands` on every shipped agent (193–199) and pins
  `("implementer-engine", "cargo" | "git")` in `is_house_tool_grant` (32),
  an arm the removal orphans but does not weaken. `tests/roster.rs` 158's
  effort-never-rises check (211–227) admits high→high. `tests/
  witness_digests.rs` pins ten manifests (221–242 hold the three movers);
  `bundle/compose_tests.rs` pins the four UNCOMPOSED digests (1069–1084)
  and, separately, `recipes/triage`'s digest (1218).
- Test harnesses that already exist and are reused, not reinvented:
  `agents/tests.rs`'s `Tree` tempdir library+adapters harness (6–54) with
  the resolver-seam refusal helper, and the shipped bare-vs-measured
  sentence pair (260–300); `bundle/model_policy_tests.rs`'s `Fixture`
  (57–200: `write_adapter`, `write_agent_file`, `compile_roots` against
  any agents/adapters roots under a stated boundary, `refusal_under`) and
  `scratch_adapters` (3787), which copies the SHIPPED adapters and applies
  a mutation — the removal-evidence vehicle that never edits shipped
  bytes; `engine/boundary_tests.rs`'s direct `compose_site` driving
  (293ff); `tests/library_data.rs`'s shipped-roots helpers `workspace`,
  `library`, `adapters` (21, 169–175) and the `resolve_agent` re-export
  (`lib.rs` 14 = `agents::resolve`, namespace default).

Hosts are Linux and macOS (decision 0063); the namespace box itself remains
Linux-only by decision 0043's own title, and nothing here runs one.

## Goals

- Name the exact shipped declaration that seats the ruled hire and the
  exact tests that prove every scenario of both deltas, each with its
  reason text and its removal proof.
- Keep every refusal, branch and fragment on its existing production path:
  the design plans a test-and-data change with no compiler edit, and names
  the smallest authorized repair if the tests expose a gap.
- Move exactly the pins the declaration moves, with histories that say why.

## Non-goals

- No adapter edit: `adapters/codex.json` and `adapters/claude.json` are
  inputs, and the Claude namespace proof must pass on the shipped bytes.
- No per-tool allow-list, no shell-command policy, no MCP transport change,
  no native-tool bypass prevention, no boundary vocabulary change, no new
  decision document, no status change on 0043.
- No live Astra smith, no workflow-runner run, no network, no frozen
  `fixtures/`, `contracts/`, `policy/`, `reference/` or `extensions/`
  bytes, no tasks.md from this phase.

## Decisions

### D1 — The compiler is already right: this change is agent data plus tests

Source inspection of the functions above satisfies every scenario of
`boxed-work-provider-admission` without an edit: the tool-list refusal
exists and is word-exact (`compose` 823–842); hands replace the list and
suppress its arguments on every provider, capable or not, because the
`else if` is unreachable once hands are declared (798–857); the hands-gap
refusal exists (804–818); the boundary split is total
(`enforce_hands_boundary` 2616–2697, `compose_site` 4293–4317); the
whole-chain rule iterates every candidate (2650–2698); and the shipped
Claude fragment is appended verbatim by the existing namespace path, so its
preservation needs no adapter edit.

Alternatives:

- *Ship a defensive compiler change* (e.g. a dedicated hands-over-tools
  flag or an explicit `hands_fragment` re-check). Rejected: every scenario
  already passes against the named lines, and an unused guard is a rule
  with no refusing input — the exact-coverage gate would then demand
  reachability nothing exercises.
- *Wait for the tests and decide then.* Rejected as a design: the
  commission asks the design to commit to the smallest authorized repair
  shape now. If, and only if, an implemented test exposes a gap, the repair
  is the smallest edit inside `compose`'s two existing branches that
  preserves both refusal texts verbatim and keeps
  `enforce_hands_boundary`/`compose_site`'s boundary split intact; anything
  larger is a new change.

Trade-off accepted: the design asserts "no compiler change is needed" from
inspection, not from a run — the proof obligation stays on the tests, and
the contingency is named rather than pretended away.

### D2 — The smith's declaration: the reviewer's hands, the old charter, no `tools`

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

(spelled with the file's existing two-space layout; every key is inside the
loader's whitelist, `load.rs` 487–498). Every term is forced:

- `models`/`efforts` are the ruled chain and effort pins verbatim. Equal
  high/high keeps the effort-never-rises check (roster.rs 211–227) green
  without an exemption.
- The binds are copied byte for byte from `agents/reviewer.json` (decision
  0043 ruling 5's spelling): the `~/.cargo` overlay masking both
  `credentials.toml` and `credentials`, and `~/.rustup` read-only. No new
  bind semantics, no bwrap-version surprise, and the box law already
  covers these paths. The worktree's writable mount is what workspace hands
  do automatically (`box_argv` binds the workdir), so no workspace bind,
  no `rw` toolchain path, and no checkout-specific absolute path is
  declared — the spec forbids duplicating or naming one, and
  `resolution.hands` asserting *exactly* network false plus the two binds
  is the proof that nothing else rides along. A `boundary` key is refused
  by the loader itself, so "the realm selects that axis" is loader law,
  not review discipline.
- `tools` is deleted outright (which also drops the empty `tools.mcp` —
  `parse_tools` yields `allow: None, mcp: []` either way, so no MCP
  behavior changes). `compose` consults `tools.allow` on no path once
  hands are declared — boxed (hands branch) or unboxed (the `else if` is
  behind the hands `if`) — so keeping it would be a dead declaration, and
  the shipped roster rule refuses dead tools beside hands (roster.rs
  193–199). The charter, the description and the limits are kept, so the
  smith stays the same office; the charter file is untouched and the
  charter-sharing pin (`library_data.rs` 211–214) still holds.

Alternatives:

- *Keep `tools.allow` for the record* (as 0043 ruling 5 did for the review
  agents before the correction removed them). Rejected: unlike the review
  agents at the time, this agent is born with hands; a dead list beside
  hands is exactly what the shipped roster rule refuses, and proposal D2
  rules the test-only dual declaration as the way to prove replacement.
- *A separate engine charter.* Rejected: nothing in the commission changes
  the office's text, and a new charter file moves pins and invites drift
  from `implementer.md` for zero semantic gain.
- *`hands: "workspace"` shorthand.* Rejected: the spec names network and
  binds, so the object form is required; the explicit `"kind"` matches the
  reviewer's spelling.

Trade-off accepted: `is_house_tool_grant`'s `("implementer-engine", …)` arm
becomes unreachable data. Removing it would be a second, unrelated edit to
a shipped-rule test; leaving it costs one dead match arm that still
documents the office's historical grants. Left as is.

### D3 — The refusal regressions: compile-facing cases at the bundle seam, precedence at the resolver seam

One seam fact decides the shape. The exact refusal sentences are composed
in `compose` (agents.rs 804–818, 823–842), but the *seat name* enters only
at bundle.rs's seat wrapper (2085, 2124) — `ResolveError`'s own display
carries agent, provider and model and no seat. The deltas require the
refusals to name the seat AND preserve the reason word for word, so the
assertions are split by seam, each where it can actually hold:

- *Bundle seam* — `crates/brokkr-runtime/src/bundle/model_policy_tests.rs`,
  on the existing `Fixture`/`compile_roots`/`refusal_under` helpers. A work
  seat (`{"agent": <fixture agent>, "class": "work"}`) compiled under
  `Boundary::Namespace` against scratch fixture adapters. The compiled
  error string then carries `seat 'work': agent '…' cannot be served by
  provider 'codex' on model 'astra': <sentence>. <closing explanation>` —
  seat, agent, provider, model, reason and explanation asserted in one
  literal. This is also the only seam where a *class* exists (the resolver
  reads none), which is how the delta's "a work class … SHALL NOT bypass"
  is exercised rather than assumed.
- *Resolver seam* — `crates/brokkr-runtime/src/agents/tests.rs`, on the
  existing `Tree` harness, for the A1 precedence fixture (the delta's own
  verb is "resolves") and alongside the shipped bare-vs-measured pair
  (260–300, stays green) that pins the sentence bytes independent of any
  seat wrapper.

Fixtures (all tempdir; no shipped byte, no frozen `fixtures/` file, no
provider, no nested namespace). A codex-shaped adapter — `provider
"codex"` (the file name names the provider in `Adapters::load`), trusted,
`driver ["{brokkr}","driver","codex","--"]`, `models {"astra":
"gpt-6-astra"}`, `model_flag "--model"`, `efforts ["high"]`,
`effort_flag "--effort"`, `mcp "unsupported"` (the shipped shape), and per
case: `tool_permissions` as the bare string `"unsupported"` (so the refusal
carries no measured parenthetical), a measured `{"unsupported": "<reason>"}`,
or a capable map; `hands` absent, `{"unsupported": "<reason>"}`, or a
`workspace` fragment. A work agent fixture — `models ["astra"]` (or the
two-link chain below), `efforts {"astra":"high"}`, `tools.allow
["cargo","git"]`, hands per case. The admission fixture mirrors the
shipped codex declaration exactly: bare unsupported `tool_permissions`
PLUS a `hands.workspace` fragment — one fixture proves admission and, on
its removal run (agent loses hands), returns the exact MORE-power refusal.

Cases:

1. *No-hands refusal, word for word* (bundle seam). Agent with the list,
   no hands; adapter with bare `tool_permissions: "unsupported"`. Assert
   the compiled refusal contains, exactly: `the provider declares
   tool_permissions unsupported, so the agent's restriction to ["cargo",
   "git"] cannot be expressed and the agent would run with MORE power than
   it declares`, beside the `seat 'work': ` prefix, the `agent '…' cannot
   be served by provider 'codex' on model 'astra': ` naming, and the
   closing explanation sentence. The `{allow:?}` rendering of `["cargo",
   "git"]` (space after the comma) is part of the asserted bytes.
2. *Measured gap* (bundle seam). The same fixture with
   `{"unsupported": "<measured>"}`: the same sentence with the supplied
   reason inside the parenthetical, mirroring
   `a_measured_gap_refuses_exactly_as_a_bare_unsupported_does`
   (agents/tests.rs 281) one seam up.
3. *No bypass* (bundle seam). The tool-listed, hands-less agent chained
   `["astra", "fable"]` with a capable claude-shaped second adapter still
   refuses with the codex sentence: `resolve_report` refuses on any entry's
   gap (agents.rs 1009–1013). Namespace realm, work class and an available
   later candidate are all present in this one compile, so the scenario's
   "SHALL NOT bypass" is exercised, not assumed.
4. *Namespace admission* (bundle seam). The same agent plus hands; the
   admission fixture adapter (bare unsupported `tool_permissions` retained
   beside the `hands.workspace` fragment). Assert success with concrete
   facts, never bare `is_ok()`: `manifest["hands"]["work"]` equals the
   declared spec, `manifest["boundary"]["work"]` equals `"namespace"`,
   `bundle.hands["work"]` is the declared spec, and the seat's
   `candidates[0].argv` carries the fragment tokens after the effort pin
   with `candidate.hands_fragment` equal to the fragment.
5. *Hands-gap refusal* (bundle seam). Hands on the agent; `hands` absent,
   then `{"unsupported": "<reason>"}`, on the adapter. Assert the seat/
   agent/provider/model naming plus both halves of the sentence: `the
   provider declares hands unsupported` (with and without the
   parenthesized reason) and `so the agent's hands cannot be put in the
   box and the agent would run with the harness's own tools`. The
   hands-declaring agent in this fixture also carries the tool list, which
   proves the refusal takes the hands path rather than falling through to
   the tool path the same provider also cannot serve.
6. *A1 fixture — the two sources of one flag* (resolver seam). A
   claude-shaped capable adapter whose `names` map `cargo`/`git` to
   `Bash(cargo:*)`/`Bash(git:*)` and whose `hands.workspace` ends
   `--allowedTools mcp__brokkr__workspace`; a test-only agent carrying the
   same hands AND `tools.allow ["cargo","git"]` (legal in a fixture
   library; the shipped roster rule reads only `agents/`, so nothing ships
   with both). Under `resolve` (namespace default): the complete fragment
   appears in order in the candidate argv, `--allowedTools` occurs exactly
   once with the argument `mcp__brokkr__workspace`, and no argument
   contains `Bash(cargo:*)` or `Bash(git:*)`; `resolution.hands` is the
   declared spec. Flag spelling alone cannot distinguish the sources — the
   same flag is used for both — which is what makes the count-and-argument
   assertion the proof of precedence.

Alternatives:

- *Reuse the shipped codex adapter in a scratch tree.* Rejected: the
  shipped file's measured `tool_permissions` reason would put a
  parenthetical inside the word-for-word assertion, and editing it per case
  would make the shipped bytes load-bearing for a regression that must
  survive unrelated adapter edits.
- *Assert on the error kind only.* Rejected by the delta itself: reason
  text is the contract (agents.rs 603–605), and `is_err()` proves nothing.
- *Assert the seat prefix from the resolver seam.* Rejected as
  unimplementable: `ResolveError` carries no seat; the prefix exists only
  where a seat is compiled. Hence the split above rather than one seam.

Trade-off accepted: the fixture codex adapter is a parallel copy of the
shaped surface, so a future shipped-adapter field (like `resume`) can drift
from the fixture without breaking it. The fixture pins only the fields the
law reads; that narrowing is deliberate.

### D4 — The boundary regressions: bundle-level, under a staged harness realm

New tests in `crates/brokkr-runtime/src/bundle/model_policy_tests.rs`, on
the existing `Fixture`/`compile_roots`/`scratch_adapters` helpers, so the
boundary, the class and the adapters are test inputs:

- *Harness admission, composed.* The hands agent (tool list retained), a
  codex-shaped adapter declaring BOTH `hands.workspace` and
  `hands.harness.work: ["--sandbox", "workspace-write"]`, compiled under
  `Boundary::Harness`. Assert compile succeeds, the manifest stamps
  `boundary: harness` for the site, and the resolution carries no
  workspace fragment (under `harness` `compose`'s boxed branch is not
  taken, agents.rs 800). Then compose the launch the production way —
  `compose_site(BuiltBoundary::Harness, SeatClass::Work, candidate.argv,
  Some(&spec), Some(&candidate), workdir, roots, result_path, None)` — and
  assert the argv gains exactly `--sandbox workspace-write` after the base
  command and carries no `mcp_servers.brokkr` token and no tool-list flag:
  admission without implying that Brokkr enforces the binds or network
  policy, which is 0046's own wording.
- *Missing harness work.* The same fixture with `hands.harness.work`
  absent, then declared `{"unsupported": "<measured reason>"}`. Assert the
  refusal names the seat, `link 1`, provider `codex`, `hands.harness.work`,
  the writable-sandbox reason and `(decision 0046 rulings 1 and 4)`, with
  the measured reason inside the existing parenthetical; workspace support
  alone (the admission fixture's fragment is still declared) does not
  satisfy it.
- *The shipped fallback keeps the whole chain honest.* A minimal fixture
  bundle seating the SHIPPED `implementer-engine` with the SHIPPED
  adapters under `Boundary::Harness` (`compile_roots` against the shipped
  roots) — no recipe, so no dialect-step or unrelated refusal precedes it
  (the shipped triage family refuses at its dialect step under harness
  today, per `every_shipped_bundle_compiles_under_harness…`'s
  `assert_refused_at_the_dialect_step` arm, and must not be the vehicle).
  Assert the refusal is on link 2, provider `claude`, naming
  `hands.harness.work` and the same reason, even though the Astra/Codex
  link admits alone.
- *No borrowing, either direction.* The harness admission's launch
  assertion shows no `mcp_servers.brokkr` token under harness; the D5
  namespace launches show no `workspace-write`. `open`-boundary behavior is
  untouched: no new test re-rules it, and the existing open-arm tests stay
  green unmodified.

Alternative: *drive these through `recipes/triage` itself.* Rejected — the
shipped recipe carries policy, dialect steps and other hands sites, so a
refusal there proves nothing about the smith's own chain; the delta's
"avoiding unrelated earlier bundle refusals" names this trap.

### D5 — The shipped launches: one composition test per provider, on production output

New tests in `crates/brokkr-runtime/src/engine/boundary_tests.rs`, next to
`compose_site_follows_the_boundary_and_the_class`, but sourced differently:
they stage a minimal fixture bundle (the `model_policy_tests` staging
shape) whose work seat names the shipped `implementer-engine`, compile it
under `Boundary::Namespace` against the SHIPPED `agents/` and `adapters/`
roots, and feed the RESOLVED artifacts — `bundle.hands["work"]`,
`SeatBody::Single { command, candidates }` — into the production
`compose_site(BuiltBoundary::Namespace, SeatClass::Work, candidates[i]
.argv.clone(), Some(&spec), Some(&candidates[i]), …)` for each link, with
a canonical workdir (`tempdir().path().canonicalize()`, because macOS
spells `/var` as `/private/var`) and a canonical result path. Nothing
spawns: no provider, no network, no namespace, no bwrap —
`compose_site`/`hands_command` are pure string work plus `current_exe`,
exactly what the engine's own `compose` feeds them (engine.rs 1473–1483).

- *Codex (Astra) launch.* Candidates[0] is astra → `gpt-6-astra` at `high`
  on codex. Independent expectations, asserted against the composed argv:
  the driver prefix and `--model gpt-6-astra --effort high`; the native
  sandbox exactly the pair `--sandbox read-only`; the three `-c`
  `mcp_servers.brokkr.*` tokens with `default_tools_approval_mode
  ="approve"` present; the `mcp_servers.brokkr.args` TOML array decoded —
  unescape (`\"`→`"`, `\\\\`→`\`), then compare the `--spec` element as
  parsed JSON against the literal expected value `{kind: "workspace",
  network: false, binds: [{~/.cargo, overlay, mask [credentials.toml,
  credentials]}, {~/.rustup, ro, mask []}]}` and the `--workdir` element
  against the canonical workdir string, matching `serve_args(workdir,
  spec)`'s documented order `hands, serve, --workdir, …, --spec, …`; the
  engine executable where `{brokkr}` stood; and the absences: no
  `workspace-write`, no `--allowedTools`, no `{brokkr}`/`{hands_mcp_json}`/
  `{hands_args_toml}` literal anywhere.
- *Claude (Fable) launch.* Candidates[1] is fable → `claude-fable-5-1` at
  `high` on claude; the composed argv contains, in order, the complete
  shipped fragment: `--tools`, the empty argument, `--strict-mcp-config`,
  `--mcp-config`, the expanded MCP JSON, `--allowedTools`,
  `mcp__brokkr__workspace`. The decoded `--mcp-config` JSON registers one
  server `brokkr` whose `command` is the engine executable and whose
  `args` decode to the same workdir/spec facts as the Codex case — the
  same hands policy on both providers, asserted twice from the two
  launches. `--allowedTools` occurs exactly once; no `Bash(cargo:*)`, no
  `Bash(git:*)`, no harness fragment token, no unexpanded placeholder.
  The test edits no adapter: if the shipped fragment drifts, THIS test
  moves, which is the preservation proof the delta asks for.
- Both tests assert the full relevant argv span rather than a helper's
  output echoed back: the expected fragment sequences are written as
  literals in the tests, taken from the shipped adapter files, not read
  from `candidate.hands_fragment`; the decoded spec comparison is against
  literals spelled in the test, with `serve_args` consulted only for
  argument order.

Alternative: *assert only the decoded server policy.* Rejected — the
delta's scenarios name argv tokens (`--sandbox read-only`, the ordered
Claude fragment, the approval mode), and token-level expectations are what
make the removal matrix below discriminate.

Trade-off accepted: the literal expected fragments duplicate
`adapters/*.json` bytes inside the tests. That duplication IS the
independence the delta demands ("rather than compare a helper to itself");
when an adapter legitimately changes, the test fails and is re-pinned
beside the adapter change, exactly like a digest witness.

### D6 — Removal evidence: one named mutation per protection

Every test above is paired, in its comment, with the mutation whose
removal must fail ITS assertion for the claimed reason, then restore and
re-run green. Removal runs mutate a COPY (the `scratch_adapters` copy of
the shipped tree, or the fixture tree's own files), point the same
assertions at the copy, observe the named failure, restore, and re-run
against the unmutated inputs. The matrix (recorded in the delivery
evidence, not asserted by CI):

| Protection | Targeted test | Removal that must fail it |
|---|---|---|
| no-hands refusal | D3 case 1 | fixture adapter grows a capable `tool_permissions` map → the seat compiles, the exact-sentence assertion fails |
| measured-gap shape | D3 case 2 | the measured reason is dropped from the fixture → the parenthetical assertion fails |
| whole-chain no-bypass | D3 case 3 | the codex fixture link is deleted from the chain → the seat compiles via the capable fallback, the names-codex assertion fails |
| namespace admission | D3 case 4 | agent loses `hands` → the MORE-power refusal returns (the admission fixture keeps its bare unsupported declaration), success assertions fail |
| hands-gap refusal | D3 case 5 | adapter gains a `hands.workspace` fragment → admission, refusal assertions fail |
| hands-over-tools precedence (absence half) | D3 case 6 | agent loses `hands` (capable map retained) → `Bash(cargo:*),Bash(git:*)` appear, absence assertion fails |
| workspace-grant preservation (presence half) | D3 case 6 / D5 Claude | scratch adapter's fragment loses the `--allowedTools mcp__brokkr__workspace` pair → presence assertion fails |
| harness admission + launch | D4 admission | adapter loses `hands.harness.work` → the link refusal returns, success and launch assertions fail |
| harness refusal + whole-chain | D4 shipped fallback | scratch claude adapter gains a `work` fragment → seat compiles, refusal assertion fails |
| Codex MCP registration | D5 Codex | scratch adapter's `args=` token dropped → registration/decode assertion fails |
| forwarded workdir/binds/network | D5 both | scratch AGENT's hands flip `network: true` or drop the `~/.rustup` bind → decoded spec assertion fails |
| read-only native sandbox | D5 Codex | scratch fragment's `read-only` becomes `workspace-write` → sandbox and absence assertions fail |
| tool-list flag on Codex | D5 Codex | a `--allowedTools`-shaped token planted in the scratch workspace fragment → absence assertion fails |
| Claude fragment integrity | D5 Claude | any member removed, reordered or reworded in the scratch fragment → the ordered-preservation assertion fails |

A compile failure from a broken mutation, an unrelated refusal, or a
changed fixture alone does not count; each entry names the assertion that
fails, and every mutation is restored with a passing rerun. Rejected
inputs assert reasons, positive tests assert facts — the matrix inherits
that from D3–D5's wording.

### D7 — The pins that move, and the ones that must not

One data edit, three moved manifests, four pin values, two files:

| Pin | File | Today | Moves because |
|---|---|---|---|
| `recipes/triage` | `tests/witness_digests.rs` WITNESSES (229–230) and `bundle/compose_tests.rs` (1218) | `d95b41d9…` | its `implement:engine` case resolves the smith: the agent digest is the whole source's sha256 (`load.rs` 560), the resolution record's `agent_digest` changes, and the manifest's `hands` entry appears for the site |
| `recipes/night-shift` | `tests/witness_digests.rs` (221–222) | `cce3966a…` | extends `triage`; the `@compose/0000/triage` ancestor digest rides its identity |
| `recipes/gpt-flash` | `tests/witness_digests.rs` (241–242) | `f5324186…` | extends `triage` for the same reason, though its own engine case seats the untouched `gpt-flash-implementer-engine` |

Both history blocks gain one dated sentence: the operator's #307 ruling
hires `astra → fable` at high effort and replaces the smith's tool list
with decision 0043's workspace hands, so the agent's recorded identity —
and every bundle hiring it — moves. Old history stays; values are recorded
from actual compiles, never guessed.

Must NOT move, asserted by the unchanged pins themselves:
`recipes/fast`, `recipes/node`, `recipes/preflight`,
`recipes/wager-harness`, `recipes/research`, `recipes/research-dsh`,
`bundles/verify` (WITNESSES) and the UNCOMPOSED four — `recipes/fast`,
`recipes/panel-review`, `bundles/self`, `bundles/verify`
(`compose_tests.rs` 1069–1084) — none hires the smith and no adapter byte
changes. No test pins the smith's models, efforts or agent digest (the
only code references outside `gpt-flash-*` are `library_data.rs` 129's
name-list entry, 211–214's charter-sharing pin and roster.rs 32's orphaned
grant arm), so no other pin can move.
`library_data.rs`'s charter digests, `adoption.rs`'s charter-digest roster
and `gpt_flash_shape.rs`'s scoped-roster assertions are untouched by the
same argument; `every_shipped_panel_seats_at_least_two_providers`
(roster.rs 458) and the `gpt_flash_shape` suite pass unmodified (the smith
is a strategy-selected single seat, not a panel member, and the Flash crew
is a different agent set). `bundles/self` still compiles — the release
validation requires it — but its digest is not this change's to move.

Alternative: *re-pin only triage and derive the rest at test time.*
Rejected: the witness law pins actual compiles of every shipped bundle;
deriving descendants lazily would weaken the byte-identity promise the
header of `witness_digests.rs` states.

### D8 — Documentation: the guide section and the dated note, nothing else

- `docs/guides/provider-adapters.md`: a new subsection directly under
  `## Hands` (line 247, beside the existing `### \`hands.harness\`` at
  276), stating: a provider with no native per-tool flags serves a
  restricted work seat through declared hands; under `namespace` that
  requires `hands.workspace`, the writable work happens through the MCP
  `workspace` tool, and Codex's native sandbox stands `--sandbox
  read-only` beside it; the confinement is the existing empty-root
  filesystem and network boundary, never a command allow-list; 0043's
  limits stand — Codex's native shell remains available read-only outside
  the box, host-read secrecy is not promised, provider traffic belongs to
  the harness outside the box; and `hands.harness.work` keeps its unboxed
  meaning — namespace composition never appends it and harness admission
  never implies a Brokkr box. The existing `hands.harness` prose already
  carries most of the route split; the new text cross-references it
  instead of repeating the member table, and no `cargo`/`git` command
  restriction is claimed anywhere.
- `docs/decisions/0043-the-hands-are-one-tool.md`: one dated append at the
  end, headed in the house addendum convention (`## Addendum — 2026-09-21,
  …`, as decision 0035's addenda are headed) and dated with the issue:
  the engine smith is seated on the existing workspace hands; the earlier
  statement that an enforcement contract for a command allow-list would
  follow is withdrawn; no new per-tool contract, transport semantics or
  native-tool bypass prevention is designed; the status line stays
  `accepted` and every historical ruling is quoted untouched.

Alternatives: *a new proposed decision document.* Rejected — the
commission rules this is 0043's existing semantics applied, and house
rules reserve `proposed` status for semantic changes; manufacturing one
would contradict proposal D4. *Silence in the guide.* Rejected — the delta
makes the guide's wording a requirement with scenarios, not optional prose.

### D9 — The evidence boundary, stated once and enforced by wording

Deterministic compilation and composition prove the ruling is EXPRESSIBLE:
the refusals fire, the fragments compose, the pins move for the named
reason. None of it is a live Astra smith. The delivery record (result
notes, tasks-phase evidence, PR description) says exactly that and names
the controller's pending measurement — Cargo and Git through Codex's
boxed hands, a real commit, verify passing — as the first live proof.
Unavailable executions (this phase is commissioned not to build or run
anything; the namespace box cannot be nested here) and the external
exact-coverage run are recorded as pending, never as passes, and the
literal 100% threshold is not touched. New tests use canonical temporary
roots, carry no `cfg(windows)` branch and no Windows obligation (decision
0063), and write no frozen bytes.

## Proof map

Every requirement, to its test, its asserted text and its removal row:

- `astra-engine-smith` — *hires the ruled chain through workspace hands*:
  a shipped-roots resolution test in `tests/library_data.rs` (helpers at
  21, 169–175; `resolve_agent` = namespace-default resolve) asserting
  candidates `astra/gpt-6-astra/high/codex` then
  `fable/claude-fable-5-1/high/claude`, `resolution.hands` = network false
  + exactly the two binds with both masks, the source JSON carrying no
  `tools` key and no `boundary` key, and both adapters' fragments in the
  candidates' `hands_fragment`; the namespace boundary stamp itself is
  asserted at the bundle seam (D3 case 4's manifest map) and the roster's
  dead-tools rule (roster.rs 193–199) is itself part of the proof.
  Removal: D6 rows for admission and precedence.
- *writable project access uses the existing workspace mount*: D5's two
  launch tests (decoded `--workdir` = canonical tempdir; decoded spec =
  network false + exactly the two toolchain binds with both Cargo masks)
  plus the library_data assertions that the agent declares no extra
  writable bind, no network grant and no boundary field. Removal: D6's
  forwarded-workdir row.
- *witnessed by actual compiled identities*: `witness_digests.rs` (3 moved
  pins + history) and `compose_tests.rs` (1 moved pin + history), with
  `every_witness_manifest_satisfies_the_v9_contract_it_claims`,
  `every_bundle_in_the_tree_compiles`,
  `every_shipped_panel_seats_at_least_two_providers`, `gpt_flash_shape`
  and the full `brokkr-runtime` suite as the unchanged-guarantees half.
  Removal: reverting the agent edit fails the three moved-pin assertions;
  an unexplained move fails review, not CI — the history sentence is the
  artifact.
- *the guide and decision record*: content assertions are human-reviewed
  against the three scenarios (allow-list escalation answered,
  composition escalation answered without combining paths, status
  untouched); the tests that keep the surrounding claims honest are
  `every_shipped_bundle_compiles_under_harness…` and
  `a_measured_claude_gap_is_reported_not_papered_over` (unchanged and
  green).
- *deterministic evidence vs the first live smith*: wording discipline in
  the delivery record; no test can or may discharge it.
- `boxed-work-provider-admission` — *tool-listed seat without hands keeps
  its exact refusal*: D3 cases 1–3 (bundle seam: seat prefix, naming,
  sentence, measured shape, no-bypass), removal rows 1–3. *Namespace hands
  replace the list only through declared workspace support*: D3 cases 4–6
  plus D1's unchanged-resolution finding, removal rows 4–6. *Harness work
  is a separate case*: D4 (admission+launch, missing-work measured and
  bare, shipped fallback link 2), removal rows 7–8. *Shipped launches as
  composed*: D5 (Codex and Claude, independent literal expectations),
  removal rows 9–14. *Every protection has removal evidence*: D6 in full.

## Risks

- **The inspection-based "no compiler change" claim is wrong.** Mitigated
  by D1's named contingency and by the tests being written first against
  the shipped bytes; a gap surfaces as a failing scenario, not as a
  silent pass.
- **TOML/JSON decoding in tests re-implements the engine.** The `args`
  array is decoded with the same quoting rules `hands_command` writes
  (unescape `\"` and `\\\\`, then JSON-parse the spec element); a mismatch
  fails loudly in the launch test, and the decoded-value comparison
  against test-spelled literals (not against `serve_args`'s own output)
  keeps it honest.
- **Fixture drift from the shipped adapters.** Accepted deliberately
  (D3/D5): the fixtures pin the law's inputs, the D5 tests pin the shipped
  bytes; together they cover both.
- **Digest pins recorded without a run.** This phase is commissioned not
  to build or test, so the design invents no hashes — implementation
  measures and re-pins from actual compiles, as the witness header
  demands. (The proposal's `cargo` ENOENT account was that phase's record;
  whether or not a later seat's box carries the binary changes nothing
  about this phase's commission or the pin discipline.)
- **macOS path spelling** in workdir assertions — mitigated by
  canonicalization (D5), which is also the proposal's stated rule.

## Migration

None for adopters: the change is one shipped agent declaration plus tests
and prose. `recipes/triage`-family users regain a compilable `engine`
strategy under the default namespace realm — the ruled hire compiles where
the same hire carrying a tool list would refuse today, because `astra`
rides Codex and Codex cannot express a list (proposal Why); under a
`harness` realm the smith still refuses until claude's
`hands.harness.work` is measured — the shipped-Claude refusal is preserved
on purpose (D4), and no guessed fragment is added. The dated 0043 note and
the guide section are additive; no contract, schema, fixture or frozen
byte moves.

## Open questions

None that block implementation. Two facts are deliberately recorded as
pending rather than resolved here: the controller's first live Astra
measurement (D9), and the external exact-coverage run required by the
release configuration before any tag. Both are handoff obligations, not
design inputs.
