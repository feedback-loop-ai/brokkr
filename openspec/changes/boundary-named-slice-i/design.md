# Design: The boundary is named — decision 0046, enactment slice (i)

The chief's synthesis of the council's two positions (simplicity,
robustness) for the change `boundary-named-slice-i`, ruled on the
council's third sitting. The proposal's `## Decisions` (D1–D26) are the
specify seat's readings of the decision; this document rules on the
design that enacts them, records where the two positions were adopted,
rejected or combined and on what evidence, and amends the proposal's
decisions it moves (D6, D8, D12, D13, D16, D17, D22, D24, D25, each
marked *amended in design* there) so that the proposal, the deltas and
this design say one thing.

The first sitting produced twenty rulings, DD1–DD20. The second sitting
read them against the tree and disputed them from both sides; its
reconciliation is kept below as the record of what was ruled and why.
The third sitting found the artefacts converged: simplicity withdrew six
claims on the second sitting's evidence, pressed one with evidence the
council had not weighed, offered two small cuts and an ordering;
robustness pressed nine objections against the second sitting's draft
and re-measured the installed harnesses. Every third-sitting claim is
ruled in its own table with its evidence. Four rulings move: DD8 flips
on the decisions' own words, DD9 gains a spawn-time recheck, DD19's
trigger changes from a step name to the record's own `model`, and DD16
refines its fourth step; DD5, DD12, DD18 and DD20 are restated where the
objection was new; the rest stand.

Returned from analyze a second time (2026-09-06) with four findings
whose earliest owner is the proposal (D27–D30); three amend this
document and none moves a ruling. DD20's shipped pin now names, per
bundle, the ground the compiler reaches first — `claude` for
`bundles/self` and `recipes/panel-review`, the `analyze` check step for
`recipes/triage` and `recipes/night-shift` — because phases compile in
name order and the earlier clause "the agent link of every sequence
precedes its dialect step" was false for the two dialect recipes; DD20
also places ruling 4's own binding, a `harness` gate on the shipped
codex adapter admitted and on dsh refused, in `model_policy_tests.rs`
against the shipped library; DD12 gains the console's page-side pair
helper, because `ui.html` reads the flattened wire and no Rust helper
can reach it; and the docs list gains the two blueprint pages that
still present the container trust class.

Returned from clarify a fifth time and cleared (2026-09-06), after the
operator ruled the two questions that had parked the loop — D31 and
D32, recorded in the proposal by commit `a2d1eff` and folded into
boundary-record's prompt requirement and gate-boundary-policy's
pinned-bytes and argv requirements, each with its scenario. This
document predated both rulings, so DD1–DD20 were reconciled against a
design in which the hands paragraph was rendered for every driver and
the pinned-bytes law was gate-scoped. The council sat a fourth time on
exactly that fold: simplicity pressed four claims about where the two
rulings land in the tree, robustness carried both rulings through and
re-raised nine objections. Every fourth-sitting claim is ruled in its
own table below, on seams re-read for it. Two rulings are added — DD21
for D31, DD22 for D32 — and four are restated where the rulings touch
them: DD5 names the read that answers a site's class instead of
calling the fact free, DD9 is rescoped to every exec site with hands
and cites the requirement's new title, DD16's step (4a) says so, and
DD18 gains the exec arm's class-freedom and the prompt renderer's
parameter. No ruling flips, and the tasks are corrected in the same
commit rather than left to the tasks phase, because rows that cite a
requirement title that no longer exists are not a coherent artefact.

## Context

What the tree holds, read for this design (paths are the evidence a
judge can re-read):

- **The world is pinned at start, the bundle is compiled before.**
  `Engine::start_in_world` (`crates/brokkr-runtime/src/engine.rs`)
  takes a compiled `Bundle` and an `Option<World>`, pins the world into
  the manifest with `World::pinned` and journals it in `run/started`.
  The bundle's manifest — where the v9 `boundary` map lives — is
  computed at compile by `manifest_for`. The two inputs are produced by
  two calls, so two parties can disagree about the boundary unless
  something compares them.
- **The realms pin already embeds `realms.json` verbatim.** `World::pin`
  writes `{"source", "sha256", "map": content}`, and `manifest_digest`
  hashes the whole manifest. A realm that declares a boundary already
  moves every digest that pins it; the v9 map is legibility, not the
  identity mechanism. It is still ruled (ruling 1) and still built.
- **The boundary is realm-wide.** Ruling 1: declared by the realm, a
  bundle never names it; issue #214's per-site relaxation is unruled.
  Every hands site of one run stands under one word.
- **One `effect/started` covers several sites of mixed class.**
  `recipes/triage`'s specify, clarify and design steps are sequences
  (and `recipes/night-shift` inherits them): the chief architect —
  `agents/chief-architect.json`, class work, hands, chain `fable`,
  `astra`, `opus` — followed by a dialect validate step, class gate,
  with the synthetic hands the compiler records. `recipes/panel-review`'s
  review is a panel. The journal carries no site's class: `effect/started`
  carries `effect_id`, `attempt_id`, `driver` and, when any site is
  agent-resolved, `provenance`; `phase/entered` carries `phase`, `head`,
  `case`.
- **Codex restricts by sandbox class, not by tool name.**
  `adapters/codex.json` declares `tool_permissions` unsupported ("codex
  exec -s|--sandbox takes read-only|workspace-write|danger-full-access
  and there is no per-tool allow-list flag"); its `hands.workspace`
  fragment opens `--sandbox read-only` and adds the boxed tool beside
  it. `crates/brokkr-protocol/src/adapters.rs` pins the three classes as
  verified against codex-cli 0.148.0. So a codex seat has no tool-list
  path: what it may write is a class, addressed only by a fragment.
- **Claude's tool-list path is the driver argv plus the grant.**
  `adapters/claude.json`'s driver carries `--permission-mode
  acceptEdits`; a seat's declared tools reach it as `--allowedTools`.
  The chief architect declares no `tools`, because under the box the
  workspace tool is its only writer; the proposal's *Measurements*
  records that a bare `acceptEdits` prompts for every shell call and a
  non-interactive seat answers a prompt with a denial.
- **The dialect step holds its gate boxed, by 0042's own words and the
  compiler's own call.** Decision 0042 ruling 4 says the validator is
  "run as a boxed exec step ... boxed under decision 0040 ruling 3,
  class gate". The compiler builds a synthetic site for every dialect
  step — `{"class":"gate", "hands":"workspace", "driver":{"command":
  ["{brokkr}","driver","exec","--"]}}` — at the verify-phase fold and
  in the sequence step loop (`crates/brokkr-runtime/src/bundle.rs`),
  and passes it through `enforce_model_policy` and `record_hands`
  before the step is built; the empty `StepBody::Dialect { .. }` arm
  in the step loop is the second call site, not an exemption. The step
  is admitted at its gate by 0043 ruling 3 — a deterministic command
  whose blast radius is the box — exactly as the shipped verify script
  is. Its argv is the dialect's own, pinned by the dialect's content
  digest in the world pin; the manifest walk skips `dialects/`.
- **The box has a private home, and the toolchain enters only by
  declared binds.** `crates/brokkr-protocol/src/hands.rs` binds a
  per-session private directory at `HOME`, a private `/tmp`, sets
  `CARGO_HOME`, `RUSTUP_HOME` and `NPM_CONFIG_CACHE` exactly when the
  site's `hands.binds` name `~/.cargo`, `~/.rustup`, `~/.npm`, fixes
  `USER` and `LOGNAME` to `runner`, and hides each bind's `mask` entries
  behind `/dev/null`. `bundles/self`'s verify seat binds `~/.cargo` as an
  overlay masking `credentials.toml` and `credentials`, and `~/.rustup`
  read-only.
- **The manifest walk pins bytes through symlinks and skips two
  names, and an ancestor's identity is recomputable.** `manifest_for`
  walks with `is_file()` and `fs::read`, both of which follow a
  symlink, so a link's target bytes are pinned under the link's key; it
  skips a top-level `realms.json` and `dialects/` and refuses
  `secrets.env`. An ancestor layer is pinned by the same walk over its
  own directory (`bundle/compose.rs`, the ancestor loop):
  `manifest_for(layer.dir, layer.name, <its deeper ancestors>, None,
  None, <no hands>, <no select>)`, digested by `sha256_hex`; `Ancestor`
  keeps the layer's `dir`, and the compiled `Bundle` keeps `chain` and
  `roots`. So the identity a layer had at compile can be re-derived
  from the tree as it stands at any later moment, with the functions
  the compiler already has.
- **The work seat writes the tree the gate's layer lives in.**
  `bundles/self/scripts/verify-seat.sh` is under the repository the
  implement seat edits — the workspace tool binds the worktree, which
  holds `bundles/` and `recipes/` — and every delivery recipe runs its
  verify gate after its implement seat. Under the namespace box a
  script the seat edited runs boxed; unboxed, it runs as the operator's
  uid on the host.
- **The tree already judges paths without canonicalising.**
  `artifact_failures` (`engine.rs`) accepts a relative path with no
  `\`, no `.` or `..` component and no absolute prefix, then reads
  `metadata` (not `symlink_metadata`): "content, not provenance". That
  is the platform-neutral precedent the pinned-script check follows.
- **`rerun` compiles with no world.** `Cmd::Rerun` calls `compile_in`
  and `Engine::start` (`crates/brokkr-cli/src/lib.rs`): today a rerun's
  manifest carries no realms pin and no dialect, whatever the workspace
  declares. `run` discovers the world and compiles in it; `resume`
  compiles from the pinned one and the engine rebuilds its world with
  `World::from_manifest`.
- **`verify-run` is parse, chain, fold; the gate's evidence is the
  operator's push.** The store's `verify_export` makes the three checks
  the import gate makes; seat records are validated at append by the
  store's fence (0034 ruling 6) — every `effect/checkpointed` payload
  and every `effect/succeeded` `result`; payload extension fields
  (`provenance`, `head`) are validated nowhere. The delivery gate
  (`scripts/delivered-by-brokkr.sh`) reads a run's journal from
  `refs/heads/brokkr-runs/<run>` on the evidence remote, which the
  operator pushes; `verify_evidence` runs `verify-run` (chain verified,
  run completed) and matches the anchor's head hash and seq. Nothing in
  this slice reads a journaled manifest's `boundary` values as
  evidence: the view reads that manifest for one boolean, whether
  `hands` is present, and the script reads the presence of the two keys.
- **The finishing checkpoint is the protocol's, and every record that
  names a model is the engine's to stamp.** Every driver brokkr-protocol
  ships — claude, codex, dsh, lanetally and exec alike — writes
  `<driver>-session-finished` as its last checkpoint with `model`
  inserted after the harness's metadata (`not applicable` for exec), and
  `crates/brokkr-cli/tests/driver_conformance.rs` asserts that step and
  that `model` for every built-in adapter; the view finds the record by
  the same suffix. Driver checkpoints reach the store through two
  pass-throughs in `engine.rs` — the `run_driver` closure for singles
  and steps, and the panel receiver loop — both of which already call
  `tag_member`; the engine's own `panel-member-finished` and
  `sequence-step-finished` markers carry `model` too, and the view
  reads `model` off any checkpoint. A panel's `effect/succeeded` result
  is `aggregate_results`' `{result, notes}`, no `model`; a sequence's
  ending result is its ending step's driver result, appended as it is.
- **The view names a participant for every site the journal names.**
  `scan_participants` (`crates/brokkr-view/src/lib.rs`) creates the
  effect's own participant at `effect/requested`, one per `provenance`
  entry at `effect/started` — "a site named here gets a participant
  even if it never checkpoints", decision 0016's single derivation
  point — and one per `member` tag a checkpoint carries, all through
  one `ensure` keyed by effect and tag. `select_candidates` skips every
  site with an empty chain, so `provenance` names only agent-resolved
  sites, and an inline member or step has a row only from its first
  checkpoint. The view already reads the pinned manifest out of
  `run/started` for the agents roster (`/manifest/agents`), and carries
  a `model` cell on `Participant`, `Node`, `CheckpointRow` and
  `JournalRow`; the four derive `Serialize` only — no `Deserialize`, no
  `deny_unknown_fields` — and `VIEW_VERSION` is 8. The terminal seats
  table prints `model` and `effort`; the trail prints `· model <x>`;
  the TUI and the console read the same cells; `costs` and `compare`
  read `model` off the seat records themselves, and `compare`'s
  `resolution_of` reads each participant's view-derived model a second
  time into the `resolution` map that `resolution_divergence` compares
  by structural equality of the two maps' values.
- **The delivery gate composes three lines** after verifying the
  journal: the tier line, the vouched line and the docs-tier preflight
  line, with the run's journal and `jq` already in hand.
- **`init` warns in words 0046 makes false.** `Cmd::Init` prints, when
  `bwrap` is absent, that the scaffolded seats "will refuse to run here
  — the shipped gates require Linux with bubblewrap on PATH".
- **The gate law returns early for a work site, and its adapter
  destructure is unconditional after that.** `enforce_model_policy`
  (`crates/brokkr-runtime/src/bundle.rs`) reads `parse_class` first,
  returns `Ok` for a work-class site with no secret binding, and then
  destructures `agents.as_mut().expect("a gate-class or secret-binding
  seat opens the adapters")`. `needs_adapters` opens the adapters for a
  bundle that names an `agent`, a `dialect`, a `secrets` binding or a
  gate-class site, and for nothing else; `Adapters::load` then fails
  where `adapters/` is absent, naming those three reasons. So a bundle
  whose only hands site is a work-class exec seat compiles with
  `agents == None`, which the comment above the load says it may. The
  0043 admission of an exec gate inside the same function reads the
  hands fact off the raw site (`driver == exec` and `raw.hands`
  present). `record_hands` runs right after `enforce_model_policy` at
  every one of the eight call sites that can declare hands, takes the
  site's raw value and the agent-supplied spec, and already refuses on
  a non-model ground (hands plus a secret binding, decision 0043).
  Every one of those call sites holds the declaring layer's directory:
  the seats loop resolves `dir` from `seat_origin` (and `case_origin`
  for a selected case) before the body is parsed — "an inherited
  seat's `role` and `./`-prefixed argv resolve against the layer that
  WROTE them".
- **A panel has one class, and the engine already reads the chosen
  body's class where `effect/started` is composed.** `parse_panel`
  refuses a mixed panel by name ("a panel may judge or work, never do
  both"), so `PanelMember` keeps no class and needs none: the fact
  lives one level up — `Seat.has_gate` for a single or a panel seat,
  `SeatBody::Select`'s `case_gates` and `default_gate` for a selected
  case, `SequenceStep.class` for a step and every member of a step
  panel. `arms_effect_gate_head` reads exactly that, through
  `seat.body.selected_is_gate(strategy, seat.has_gate)`, at the line
  after the payload is built; `invocation_sites` yields the tag and the
  chain of every site, not its class. The shipped select seats
  (`recipes/triage`'s implement and review) put a work case beside a
  gate case, which is why the seat's own `has_gate` is not the answer
  for a selected case and `selected_is_gate` is.
- **One prompt renderer, one production caller, the driver kind in
  scope.** `render_prompt(input)` (`crates/brokkr-protocol/src/adapters.rs`)
  keys the hands paragraph off `input.hands == "boxed"` and nothing
  else; its one production caller is `run_seat`, whose next statement
  is `invoke(kind, …)` with `kind: AdapterKind` in scope — the exec
  driver included, since the shipped verify seat's prompt reaches its
  script through `{prompt_file}`, and `verify-seat.sh` and
  `ship-seat.sh` read the result path off that prompt by line (`case
  "$trimmed" in /*.json)`), never off the paragraph. Its other five
  callers are tests. The seat input names no driver kind, and the
  engine's comment at the mark says why a key would cost more than a
  parameter: "the mark is part of the requested input, so the digest
  covers it". `mark_boxed` is the one helper that writes the mark, at
  four call sites (the single, the panel member, the sequence step and
  the dialect step).
- **What this seat's box holds:** `unshare` (util-linux), `bwrap`,
  `docker`, `jq`, `python3`, `openspec` 1.12.0; no `claude`, no
  `codex`, no `cargo`. The claude measurement is therefore not this
  seat's to make either (see Open questions). The robustness seat
  reports Claude Code 2.1.263 advertising `--restricted`,
  `--permission-prompts none` and `--strict-mcp-config`, and Codex
  0.153.2 offering `--sandbox read-only` and `--output-last-message`;
  this seat could not confirm them and records them as candidates only.

## Goals

1. Enact rulings 1–5 of decision 0046 as slice (i) of ruling 6, in the
   commission's order of value, with every fact typed once and read
   from the one place it is produced.
2. Make the honest outcome the cheap one: refusal where no mechanism
   stands, explicit absence where the record is silent, and the plain
   word in every datum with the adjective only in rendering.
3. Keep the exact-coverage gate reachable: every branch this slice adds
   lives in a pure helper with an obvious test, and no arm exists for a
   mechanism this engine does not build.
4. After the slice, a macOS operator with a realm declaring `harness`
   runs every shipped bundle that has no dialect step — conditional on
   the claude measurement (D19) — and every readout says *unboxed*; the
   two recipes with dialect steps wait on the operator's word (DD8).

## Non-goals

- Building `seatbelt` or `container` (slices ii and iii); anything that
  reads their profiles, images or mounts.
- A per-site boundary (#214), a `--boundary` flag, an environment
  override or a test hook that sets the word outside the realm map.
- Changing what `hands` allows (`network`, `binds`), any driver's trust
  tier, the result contract, the event vocabulary, or `EVENT_SCHEMA`.
- Admitting a dialect step's argv at an unboxed gate: 0046 ruling 4
  names the bundle's own pinned bytes and 0042 ruling 4 boxes the step;
  a decision admits it, not this design (DD8).
- Ruling the three questions 0046 leaves open (a `harness` vouch and
  `by-hand`, the container image pin, seatbelt and the hooks path).
- Editing any frozen contract, `policy/phase-machine.json`, `reference/`
  or the evaluator corpus; adding `boundary` to this repository's
  `realms.json`.

## The council, reconciled — second sitting

Each row names the claim, what each position argued on its second
sitting, and the ruling with its evidence. Where a position is rejected,
the reason is here and nowhere else. A claim neither position raised
again is not re-argued. Rulings the third sitting moved are marked in
the third sitting's table below, which governs where the two differ.

| Claim | Simplicity | Robustness | Ruling and evidence |
|---|---|---|---|
| Where the type lives | one closed enum beside `Realm`; accepts DD1 | one closed enum, exhaustive matches, and **no `Default`**: a general default lets an old journal or a forgotten argument read as `namespace` evidence | **Adopted, both; the default goes.** `Boundary` beside `Realm`, five variants, parse/display pair, no `Default`; absence becomes `namespace` in exactly one resolver, `Realm::boundary()`; the record's sentinel stays `Option<Boundary>` through one serde helper (DD1). |
| A per-site execution plan | none; two of five boundaries only refuse | `SiteBoundaryPlan` built once after composition, carrying class, layer, capability, script identity, delivery route | **Rejected again.** Every fact the plan would carry is on the compiled bundle where its producer wrote it — `SeatClass` on the site, the declaring layer where `expand_command` resolved `./`, `Bundle.hands` per site, the chain link's adapter — and a copy is a second thing to keep true. Three exhaustive `match`es read them (DD2). |
| The engine's entry fence | cut it: no shipped caller hands `start_in_world` a world disagreeing with its bundle; a fence only a synthetic test trips proves the fence exists | keep it, and fence every library entry before `create_run` | **Stands.** `start_in_world` takes a bundle compiled by one call and a world discovered by another; the commission's own sentence — the engine reads the realm's boundary *through the realms map the run was started with* — is true only where the two agree, and the fence is the line that makes them one. `rerun` was, until DD6, exactly the caller that would have tripped it. One comparison, one error, one constructed-call test; the requirement analyze asked for (DD3). |
| The v9 map is validated | — | the compiler asserts `keys(hands) == keys(boundary)`; readers of a resumed or supplied manifest repeat the check | **Rejected.** The two maps are written by one loop, so the assertion guards a state the compiler cannot produce; a forged manifest is a forged digest, and `verify-run` stays parse, chain and fold (DD4). |
| Journaling the site's class | cut it: the word is realm-wide, so `effect/started.boundary` is one scalar and *unboxed* is "the run's word is `harness` or `open`"; a run with hands but no gate site is then called unboxed, which is true rather than widened | keep the per-site list with an engine-owned gate fact, and validate it before any consumer reads it | **Per-site stands; validation rejected.** Ruling 3 says *gate* site and the journal carries no class; `recipes/triage`'s sequences put a work chief and a gate validate step under one `effect/started`; the scalar answers ruling 3 correctly for every shipped bundle and wrongly for a bundle whose only boxed sites are work offices, and leaves the per-seat cell of a mixed attempt to a join against the manifest's site labels — the lookup both positions want out of the view. The cost counted is not paid: same `invocation_sites` walk as `provenance`, one strict-reading helper, one `jq` (DD5, DD14). |
| "Every `effect/started`" read literally | a narrowed reading, to be reported to the operator in those words | a narrowed reading, to be ruled by the operator, never chosen silently | **Adopted, both.** D14 and DD5 name it a narrowed reading; the completion note carries it to the operator (task 11.6). |
| Rerun | accepts DD6, withdrawing last sitting's objection | discover the world as `run` does; never fall through to `namespace` | **Stands** (DD6). |
| `hands.harness.work` | cut it: ruling 4 governs gates; a work seat under `harness` runs at the harness's default as it does under `open`; refusing it under the stricter word inverts the vocabulary | cut it: ruling 4 authorises a read-only fragment, not a writable native mode; refuse work seats with hands under `harness` until the operator rules the capability | **Stands against both.** Codex's `tool_permissions` is unsupported, so its writable class is addressable only by a fragment and there is no tool-list path for a codex work seat; the chief architect chains `astra`; simplicity's default leaves the chief read-only on astra and shell-less on fable and opus, compiling under `harness` and failing its charter at run time — the quiet failure robustness warns of; robustness's refusal is what an undeclared member already yields today, by name, so the member is the operator's data path out of it. Ruling 1 defines `harness` for every site as the harness's own sandbox *as its adapter fragment addresses it*; ruling 4 narrows gates to the read-only one (DD7). |
| Dialect steps under `harness`/`open` (D6) | — | do not exempt an external binary without saying so: route it, refuse it, or record the exception | **Ruled admitted on this sitting; flipped on the third** — the second sitting read the step as admitted outside the gate law by 0042 ruling 1, and 0042 ruling 4 says otherwise (DD8, third sitting). |
| Pinned-script check | accepts DD9 as the security boundary of the slice | canonical containment, symlink defence, a digest recheck before every spawn, or an honest statement of the residual race | **Stood with the residual stated; the recheck is adopted on the third sitting** on evidence this sitting did not weigh (DD9). |
| The unboxed environment | accepts DD10; probe the network once | private home, never the operator's; and honesty: clearing the environment confines nothing on disk, and a test that moves `$HOME` must not be described as proving secrets unreadable | **Adopted.** The test's claim is renamed — it proves the environment, not the filesystem — and the guide says an unboxed script may open any absolute host path the operator's uid may read, which is the fact *unboxed* renders (DD10). |
| The network probe | a memoised PATH lookup plus one probe per run: DD15 forbids citing the outcome, so per-dispatch freshness buys nothing anyone may report | probe the exact prefix before every spawn, in the cleared environment | **Simplicity adopted.** The answer depends on the kernel and on the `unshare` found on the engine's own `PATH`, which every composed table carries verbatim; a second probe in the same process learns nothing. Once per engine process, at the first unboxed exec dispatch, in that dispatch's environment, remembered (DD15). |
| `brokkr seats` | one arm, no third wire object: `--json` is what `inspect` already emits | a thin verb over the same derivation | **Combined.** The verb exists, renders `inspect`'s seats block, and its `--json` is the view model verbatim — the bytes `inspect --json` prints — so no new object is versioned (DD11). |
| Readouts | `boundary: Cell` beside `model: Cell`; the roster-style pin test is the enforcement ruling 3 commissions; the flattened unit is a second enforcement with a `flatten` on a pinned wire shape | a composite unit on every carrier so omission is impossible by type | **Stands, costs verified.** The carriers are serialize-only, so `flatten` changes no wire byte; a construction site changes once whether a field or a unit is added; what the unit buys is a name, `served`, for the pin test to grep, which a bare second field does not give it (DD12). |
| `compare` diverges on boundary | cut it: unruled, a different feature | first-class, like a model difference | **Stands, at zero cost.** `resolution_divergence` compares the two maps' values structurally; once the pair rides the `resolution` map, a boundary difference diverges with no code written for it, and hiding it would take code (DD12). |
| Old journals | one absence — `no boundary recorded` — and no manifest read inside the view; `not applicable` only where the record carries it | never `not applicable` reconstructed for old evidence; explicit absence only | **Adopted, both.** DD13 is amended: one absence note for every model row of a journal that recorded nothing; `not applicable` only from an entry or a record that carries it; the view reads the journaled manifest once, for whether the run declares hands, as it already reads the agents roster from the same `run/started` (DD13). |
| Availability states | one `offered` table; refuse `seatbelt`/`container` as unbuilt after the probe | offered / missing tool / present but unbuilt / not admitted by adapter, never collapsed; refusal names every site; tests assert the store untouched; doctor shares the probe | **Both, already in DD17:** `Offer::{Offered, MissingTool, Unbuilt{slice, tool}}`, one table for the three verbs and `doctor`, every hands seat named, no row written. "Not admitted by adapter" is compile's verdict, not availability's. |
| The engine-side unbuilt refusal | cut it: a second refusal surface to keep in step with `refuse_unboxable` | library entries must refuse before journal creation | **Stands, as the narrowing.** Composition takes a three-word view of the five-word enum; the narrowing needs a refusing arm, and that arm is the engine's fence — not a second table, one `match` arm with one test (DD17). |
| The shipped-refusal pin | cut the second test: the rule is pinned by the dsh case, and the test moves the day the measurement lands | — | **Folded.** One test, two halves: the scratch library with both members planted proves the promise; the shipped adapters as they stand pin which bundles refuse and why. The second half is the pin that names the operator's unmet part; it moves once, by design (DD20). |
| `init`'s warning | cut: uncommissioned prose with a test | — | **Stands.** Today's text says the shipped gates require Linux with bubblewrap; after 0046 that is false, and the quickstart's platform paragraph is commissioned for the same reason (DD17). |
| The finishing checkpoint | — | do not detect it by a string suffix alone; thread an explicit lifecycle stamp | **Stood on the suffix; the trigger moves on the third sitting** to the record's own `model` key (DD19). |
| The aggregate's stamp | — | define the scalar rule for an aggregate over mixed sites | **Adopted.** A panel's aggregate is the engine's own `{result, notes}` and names no model, so it carries no boundary — the cell rides beside the model, and there is none; a sequence's ending result is its ending step's driver result and carries that step's word (DD19). |
| Strict reading of the extension | — | a shared validator before any consumer; malformed evidence must not look boxed | **Rejected again.** Every consumer renders an entry outside the vocabulary as *not recorded*, loudly (`· boundary not recorded` in the check summary), never as the boxed word, so corrupt evidence cannot acquire a boxed-looking summary; no validator is added for an extension field where none has ever been (DD14). |
| The claude measurement | undeclared, fail-closed, refused by name, reported as ruling 6's unmet part | behavioural fixture before any claim; refuse claude gates under `harness` until shown | **Stands, both** (DD20). |
| Windows shell gates | — | the shipped `sh` gates parse a Unix result path | **Recorded as an open question**, not built: ruling 6 names Windows, nothing here measures it. |

## The council, reconciled — third sitting

Each row names the claim as the third sitting made it, and the ruling
with the evidence read for it. Where this table and the second sitting's
differ, this one governs.

| Claim | Simplicity | Robustness | Ruling and evidence |
|---|---|---|---|
| Six claims withdrawn: the engine's fence (DD3), `hands.harness.work` (DD7), the flattened unit (DD12), `compare`'s divergence axis, DD17's engine arm, `init`'s warning | withdrawn on the second sitting's evidence | re-raises the `work` member only | **Recorded.** DD3, DD12 and DD17 stand as ruled; DD7 is answered again below. |
| The shape of `effect/started.boundary` | one object, `{word, gate}`: ruling 1 makes the word uniform, `select_candidates` skips empty chains so `provenance` names only agent-resolved sites and the list is a strict superset whose extra entries — the inline exec gates, most shipped hands sites — have no carrier and are read only by the run-level reduction; the one thing the list buys is a live cell, and if kept that should be the stated reason | the per-site list with the engine-owned gate fact | **The list stands, on the view's own mechanism.** `scan_participants` creates a participant for every `member` tag an `effect/started` names — decision 0016's "a site named here gets a participant even if it never checkpoints" — so every entry has a carrier by the same `ensure` call `provenance` uses, and the superset is the point: a boxed inline member or step, which provenance omits, gets its row and its cell at start instead of at its first checkpoint. One derivation rule then serves every attempt shape — the entry by tag, then a record that names a model, then absence — where the object needs a single-site special case or leaves every member and step of a mixed attempt blank until a driver record lands; and the per-site gate fact answers ruling 3 with the site named, at no cost beyond the loop the view already runs for provenance. The stated reason is the one simplicity asked for: the live cell and the uniform rule (DD5). |
| DD4: the v9 map is validated semantically | — | a digest proves which bytes were journaled, not that `keys(hands) == keys(boundary)`; a foreign or damaged but correctly chained manifest must not pass as valid | **Rejected again, on what reads the map.** No consumer of this slice reads a journaled manifest's `boundary` values: composition follows `Bundle.boundary` (DD3), the view reads the manifest for one boolean and the gate script for the presence of two keys, and a resumed run is compared whole against a fresh compile by `manifest_diff`. A journal reaches the delivery gate only from the evidence branch the operator pushes, after `verify-run`'s chain check. A validator would guard a reading nobody makes (DD4). |
| DD8: dialect steps under `harness`/`open` | — | a dialect digest pins a declaration, not the bundle's bytes; route it through a pinned script or refuse it until the operator rules; a normative admission cannot also be an open question | **Adopted; DD8 flips.** Decision 0042 ruling 4 admits the validate step "as a boxed exec step ... boxed under decision 0040 ruling 3, class gate", and the compiler passes a synthetic boxed exec gate through `enforce_model_policy` for every dialect step — so the step holds its gate by the box, exactly as the shipped verify script does, and under `harness` or `open` it is an exec gate whose command is not the bundle's own pinned script. 0046 ruling 4's narrowed reading names "the bundle's own pinned bytes"; the second sitting's reading admitted the realm's dialect declaration in their place, which widens a ruling, and 0042's addendum forbids a design note to do that. Refused at compile by name; the admission is a decision's, and its text is recorded (DD8, Open questions). |
| DD9: containment, drift and the recheck | accepts DD9; a pre-spawn digest recheck is not needed | canonical containment, symlink defence, a verdict bound to the manifest entry's digest and rechecked before every spawn, or an honest statement of the race | **The recheck is adopted; containment is rejected.** The unboxed exec gate is admitted because its bytes are pinned — ruling 4's own reason — where the boxed gate is admitted because its blast radius is bounded (0043 ruling 3); and the implement seat writes the tree the gate's layer lives in, minutes before the gate runs. So the engine re-walks the declaring layer at every unboxed exec spawn and refuses the dispatch when a pinned byte moved, and the residual is the interval between that walk and the `exec`. Containment stays rejected: a symlink's target bytes are what the walk pins and what runs, and an unboxed script can `exec` any host path in one line, so containing the path protects nothing the environment does not already fail to protect (DD9). |
| DD14: malformed evidence | — | a v9 run whose entries are missing, duplicated, mistyped or unknown must fail verification, or corruption can drop the adjective while the vouch stays admissible | **Rejected again, on how a malformed entry can arise.** The engine writes from the closed type; accidental corruption breaks the hash chain and `verify-run` refuses the journal; a malformed entry therefore needs a re-forged chain, and a forger who re-chains writes `namespace` as easily as `chroot`, so shape validation defends against nothing. The honest rendering of the unrepresentable case is *not recorded*, loud on every surface and in the check summary (DD14). |
| DD15: the probe is cached | — | probe the exact prefix in the exact environment before every spawn; a process-wide answer is stale across engines and a replaced `unshare` | **Rejected again.** No reader may cite the outcome, so freshness has no consumer; the stale direction is the network on where it could have been off, reported by nobody, which is the constraint itself; per-spawn probing spawns a process per gate to learn what nothing may say (DD15). |
| DD19: the finishing checkpoint by suffix | — | an engine-owned fact from a driver-owned string; thread a lifecycle kind; overwrite a driver's value before the fence | **Adopted in substance; no kind is threaded.** The stamp's trigger is the record's own `model` key — the fact the boundary rides beside, which conformance asserts on every finishing checkpoint and which the engine's own member- and step-finished markers carry too — applied at the two pass-throughs that already tag members and at the engine's two markers; a record that names no model carries no boundary, and a driver's word on such a record is dropped there (DD19). |
| DD20: codex declared from documentation | — | help output identifies candidates and proves nothing; `gate` is unsupported until a behavioural fixture | **Stands.** `--sandbox read-only` is ruling 4's own word for codex, and a design that refused it would amend the decision; the door is declared from the tool's record with its capture measurement recorded as pending and the operator's. An unmeasured door fails as a missing result, loudly, and never as a false word: `harness` is true of the seat whatever the door does (DD20). |
| `hands.harness.work` | withdrawn | ruling 4 authorises a read-only fragment for gates, not a writable mode for work seats; refuse until an operator ruling defines the capability | **Stands.** The refusal robustness asks for is what an undeclared member yields today, by name; the disagreement is only whether the operator's act that lifts it is data or a decision, and ruling 1 answers it — `harness` is the harness's own sandbox "as its adapter fragment addresses it", said of every site — and adapters are data (0016) (DD7). |
| "Every `effect/started`" | report it | keep it blocking at the owning requirement until ruled | **Reported, not blocked.** The narrowing is forced where 0046 ruling 3 meets 0016's extension rule in the contracts README; both answers leave this slice's code identical but for one presence condition, so proceeding discards nothing, and the completion note carries the question (DD5, task 11.6). |
| `RunBoundary.text` | cut it: prose on a versioned wire model that three surfaces then cannot phrase; put the sentence in the rendering helper | — | **Stands.** The console prints from `/api/view/<run>` and may compose nothing on the page; without the text on the wire it would compose the adjective. The view model already carries rendered prose in every `Cell.note`, and one spelling on four surfaces is what "wherever the run is summarised" asks for (DD12). |
| `SpawnEnv` | an `Option<BTreeMap>`: one fewer public type in brokkr-protocol | — | **Stands.** Two arms either way; a `None` at a spawn site reads as *no environment*, the opposite of inheriting the engine's, and the crate's rule is exhaustive matches over named variants (DD18). |
| Landing order inside step (4) | the exec/pinned-script path before the adapter fragments: every shipped inline hands site is a `./` exec gate and every hands agent chains opus, so the pinned-script path carries the whole macOS promise until the claude measurement lands | — | **Adopted.** The facts hold in the tree (ten hands agents, all chaining `opus`; every inline hands site a `./` exec dispatch); the gate law lands whole and fail-closed, the exec composition next, the adapters last (DD16). |
| The quickstart names the split | which bundles run under `harness` today and which refuse, by name, or the paragraph is a promise | — | **Adopted.** Nine compile, four refuse, and two of the four on a second ground; the paragraph names them and points at the pin test that is the record (DD20, boundary-guides). |
| Windows shell gates | — | the shipped `sh` gates parse a Unix result path | **Open question**, as before. |

## The council, reconciled — fourth sitting

The sitting on D31 and D32. Each row names the claim as the fourth
sitting made it and the ruling with the evidence read for it; where this
table and an earlier one differ, this one governs.

| Claim | Simplicity | Robustness | Ruling and evidence |
|---|---|---|---|
| Where D32's law lives | in `record_hands`: the obvious home, `enforce_model_policy`, returns early for a work site without secrets and then `expect`s the adapters, so the operator's own work-class scenario — two work exec sites, no agent, no gate, no secret — panics there rather than refusing; widening `needs_adapters` instead makes the adapter root a compile input for a bundle that names no agent | a separate, total helper the work early-return cannot bypass, placed before it | **Adopted in substance, refused on the home.** The panic is real: `needs_adapters` leaves `agents == None` for that bundle and the destructure after the early return is unconditional (Context). The law is therefore one function, `enforce_hands_boundary`, total over class, and it is the *first* statement of `enforce_model_policy` — before `parse_class`'s early return and before the adapter destructure — reading the adapters only on the chain path, where the `agent` key has opened them, and on the inline path only the raw command and the declaring directory; the operator's bundle compiles with `agents == None` as today. It stays inside `enforce_model_policy` because ruling 4's enforcement binding names that function and `model_policy_tests.rs`, and a design note does not move a binding (0042's addendum, ruling 1); `record_hands` stays 0043's. D10's inline-model refusal and DD7's `work` fragment move into the same law for the same reason: both stood behind the early return (DD22). Robustness's "separate total helper" is this function. |
| D32 is a deletion | no class test anywhere in the unboxed exec path; the operator's scenario is a row in the grammar table, not a branch; tasks 6.3, 9.9 and 9.10 lose the word *gate* rather than gaining a work case | a work-class admitted and refused pair that reaches compile, composition, environment construction and the spawn-time re-walk | **Both adopted; they are one ruling.** The law's exec arm, `compose_site`'s exec arms and the spawn re-walk read no class — every shipped inline hands site is a gate, so a work arm would be a branch only a constructed bundle reaches, and the literal coverage gate would have to carry it. The scenario binds as rows: the compile pair in the grammar table, the same bundle compiled with no `adapters/` reachable, and a `compose_site` pair over one exec command with the class flipped, argv and environment equal; the environment and the re-walk are reached by the one path there is, which the gate's end-to-end test drives, and a second box test for the work class would drive no line the first does not (DD22, DD18). The task rows are corrected here. |
| D31's discriminator | a parameter — `render_prompt(input, kind)`, five call sites, no wire byte, no digest moved; the input's `boundary` at an exec site is read by nothing and the design should say so; the engine side is one line in `mark_boxed` | a typed fact handed to the renderer, never inferred from expanded argv | **Adopted, both.** `run_seat` holds `kind` at the call (Context); a journaled key that exists to steer prose is what the mark's own comment forbids; and inference from argv would repeat `hands_command`'s positional read. The paragraph is suppressed under `AdapterKind::Exec`; the mark helper writes `boundary`, the marker and `result_delivery` at its four call sites; the unread key is stated as evidence, not dead data (DD21). |
| DD5's "the per-site gate fact is free" | false for panel members: `PanelMember` keeps no class, so the engine cannot say `gate` for a member tag; add `Bundle.gate_hands`, a set keyed as `hands` is, filled in `record_hands` | — | **The finding is refused on the tree, so the field is not needed.** `PanelMember` keeps no class because a panel has one — `parse_panel` refuses a mixed panel by name — and the fact is kept where the compiler keeps it: `Seat.has_gate` for a single or a panel seat, the select's `case_gates` for a selected case, `SequenceStep.class` for a step and its panel's members; the engine reads exactly that at the effect start today (`arms_effect_gate_head`, through `selected_is_gate`). A set of hands sites could not answer in any case: DD5's entry carries `gate` for a hands-less site too. What the claim earns is a sentence: DD5 now names the read instead of calling the fact free (DD5). |
| Carrying D31 and D32 through the design and the tasks | both need a decision entry, or a smith reading `design.md` alone builds the pre-ruling shape; the tasks were untouched by the ruling commit | revise DD9, DD10, DD15, DD18 and the task rows in dependency order; a title change alone leaves the work-seat early return and the composition untested | **Adopted.** DD21 and DD22 are the entries; DD9 is rescoped and cites the new title; DD18 is amended; DD10 and DD15 needed no words, their subject having been "an unboxed exec dispatch" since the second sitting; the stale rows are corrected in `tasks.md` in this commit, with the work-class rows and the exec-prompt row added. |
| The delivery script annotates every summary line | — | *unboxed* on the first line only leaves a contradictory check summary | **Already the readouts delta's rule:** the tier line and the vouch line both end with `· unboxed`, and a docs-tier preflight run is read the same way on its own line. Nothing to move. |
| Re-raised from the third sitting: DD4's semantic validator, canonical containment (DD9), DD14's strict validation, DD15's per-spawn probe, DD20's codex measurement, the `work` member (DD7), the literal "every `effect/started`" (DD5), the site plan (DD2), the capability states (DD17) | — | re-raised in the third sitting's words | **Stand as ruled there.** Nothing new was read for any of them and none names a seam the third sitting did not weigh; the reasons are in each ruling's own text. |
| DD8 enlarges the pinned-script exception; DD19 detects the finishing checkpoint by suffix | — | — | **Moot: the objections read the second sitting's draft.** DD8 flipped to a refusal and DD19's trigger moved to the record's own `model` key on the third sitting; both texts stand below. |

## Decisions

### DD1 — One `Boundary` type beside `Realm`, no ambient default; the sentinel is an `Option`

`brokkr_core::realms::Boundary { Namespace, Seatbelt, Container,
Harness, Open }` with `FromStr`/`Display` on the five words and **no
`Default`**: absence becomes `namespace` in exactly one place,
`Realm::boundary()`, the realm's own resolver, so a reader of evidence —
a journal, a manifest, a record — holds an `Option<Boundary>` and cannot
turn a missing word into `namespace` by default. `Realm.boundary:
Option<Boundary>` is read by the `forge.realms/v4` loader (refused under
v1–v3 by name, as `house` and `dialect` are held to theirs). Every crate
uses the type; no boundary crosses a crate boundary as a string. The
seat record's `not applicable` is `Option<Boundary>` through one serde
helper (`None` ↔ `"not applicable"`), so the realms enum cannot admit
the sentinel and the schema enums are the same five words, plus the
sentinel only where the record admits it.

*Alternatives:* a `boundary.rs` module (rejected: five words do not need
a directory entry); a six-variant enum with `NotApplicable` (rejected:
it would let a realm declare it); `Default = Namespace` (the first
sitting's shape, rejected on robustness's evidence: an
`unwrap_or_default` in a reader of old evidence would print `namespace`
where 0031 ruling 3 demands an absence).

### DD2 — Three matches, no trait, no plan object

Availability (`offered`), the gate law (`enforce_model_policy`) and
argv composition (`compose_site`) each `match` the enum exhaustively.
`seatbelt` and `container` have arms only in `offered` and in the
engine's entry fence (D21); composition is written over `namespace`,
`harness` and `open` and takes a three-variant view of the enum, so no
arm exists that a test cannot reach. The facts robustness's plan would
carry are read where they already live: `SeatClass` on the seat, member
and step; the declaring layer's directory where `expand_command`
resolved `./`; `Bundle.hands` per site; the resolved adapter's
`hands.harness` on the chain link; the layer's pinned identity in the
manifest and the compose chain the bundle keeps.

*Alternative:* a `BoundaryProvider` trait with five implementations, or
a `SiteBoundaryPlan` built after composition (rejected: two of the five
exist only to return an error until slices ii and iii, and a plan is a
copy of facts the bundle already holds — a second thing to keep true,
which is the drift the plan was meant to prevent).

### DD3 — One resolution, one field, one fence

`Bundle::compile_with_realm` takes the realm's resolved boundary
(`namespace` in no realm) and exposes `Bundle.boundary`; `manifest_for`
writes the v9 map from it. `Engine.boundary` is set from the bundle at
`start_in_world`, `resume` and `start_with_dispatch` and never re-read
from disk. `start_in_world` refuses, before `create_run`, when the world
it is given resolves a different boundary for the operated repository
than the bundle was compiled under (`EngineError::BoundaryMismatch`,
naming both words; no world resolves `namespace`). The fence is
realm-boundary's requirement *The engine starts a run only under the
boundary its bundle was compiled under*, with its own scenarios.
`resume` needs no second fence: `manifest_diff` names `boundary` when
the map differs. The Looper lineage refuses a manifest carrying `hands`
or `boundary` as it refuses every key beyond its six.

Simplicity's cut, withdrawn on the third sitting, was refused on the
second: `start_in_world` takes two inputs produced by two calls, and
the commission's sentence — the engine reads the realm's boundary
*through the realms map the run was started with* — is true only where
the two agree. The fence is the line that makes them one; `rerun` was,
until DD6, the shipped caller that would have tripped it; the test is
one constructed call.

*Alternative:* deriving the engine's word from `World::from_manifest`
at every use (rejected: a second lookup that can disagree with the
compiled bundle; the fence makes the two one).

### DD4 — The v9 map is derived, co-present by schema, equal by construction

`manifest_for` writes `boundary` in the loop that writes `hands`, one
entry per key, every value the bundle's word. `run-manifest.v9`
declares `dependencies` in both directions. No verifier rule asserts
key equality: the two maps come from one loop, and an exported manifest
that disagreed with itself would have a different digest than the one
its journal chain pins. Robustness's third-sitting distinction between
integrity and validity is right and changes nothing here, because no
consumer of this slice reads a journaled manifest's `boundary` values:
composition follows `Bundle.boundary` (DD3), the view reads the
manifest for one boolean, the gate script reads the presence of two
keys, and a resumed run is compared whole against a fresh compile. A
check would guard a reading nobody makes.

*Alternative:* a `verify-run` structural rule, or a compile-time
assertion over the two key sets (rejected: `verify-run` is parse, chain
and fold by design, and the assertion would guard a state the compiler
cannot produce — a branch no test reaches honestly).

### DD5 — `effect/started.boundary` mirrors `provenance` and adds the class

The engine writes `boundary` as a list keyed by the same `member` tags
`provenance` uses, one entry per invocation site of the attempt, each
`{member, boundary, gate}` — the realm's word for a site with hands,
`not applicable` for one without, `gate` the site's class —
constructed from the same `invocation_sites(body)` traversal as
`provenance`, so the tag sets cannot drift. It is present exactly when
at least one site of the attempt declares hands; an attempt over a
bundle that boxes nothing journals byte-identical payloads. This is a
*narrowed reading* of ruling 3's "every `effect/started`": the contracts
README admits an extension field at `event_schema: 1` only when it is
optional and absent by default, and a field on every start would be a
v2 event outside this slice's contract list. Ruling 3's "every" is
carried literally by the seat record instead (every record that names a
model, DD19). If the operator wants the literal reading, that is a new
event lineage and a return to the operator, not a wider field; both
answers leave this slice's code identical but for one presence
condition, which is why the question is reported and not blocking.
Published as `contracts/effect-boundary.v1.schema.json`; `fold` never
reads it.

Simplicity's second-sitting scalar was refused on the tree:
`recipes/triage`'s specify, clarify and design steps are sequences, so
one `effect/started` covers the chief (work, hands) and a dialect
validate step (gate, synthetic hands); the journal carries no class; a
scalar answers ruling 3's question correctly for every shipped bundle
and wrongly for a bundle whose only boxed sites are work offices.
Simplicity's third-sitting object, `{word, gate}`, concedes the class
and is refused on the view's own mechanism. `scan_participants`
creates a participant for every `member` tag an `effect/started` names
(0016's "a site named here gets a participant even if it never
checkpoints"), through the same `ensure` call `provenance` entries
take; so every entry of the list has a carrier, and the entries
provenance lacks — the inline members and steps whose chain is empty —
are exactly the sites that would otherwise have no row until their
first checkpoint and no cell until a driver record names a model. One
derivation rule then serves every attempt shape — the entry by tag,
then a record naming a model, then absence — where the object needs a
single-site special case or leaves every member and step of a mixed
attempt blank until its driver record lands. The per-site gate fact costs no field (fourth
sitting, against simplicity's `gate_hands` set): a panel has one class —
`parse_panel` refuses a mixed one by name — so the engine reads the
class where the compiler kept it, `seat.body.selected_is_gate(strategy,
seat.has_gate)` for a single, a panel member or a selected case (the
read `arms_effect_gate_head` already makes at the same line) and
`step.class` for a step and every member of a step panel; `invocation_sites`
yields the tag and the chain, and one sibling read yields the class,
which answers ruling 3 with the site named.
The cost counted is not paid: the entries come from the same walk as
`provenance`, the strict reading is one helper in the view and one
`jq` in the script, and the schema file exists under either shape.
Simplicity asked that the live cell be the stated reason if the list
stayed; it is, together with the uniform rule.

### DD6 — `rerun` discovers the world, as `run` does (amends D13)

`Cmd::Rerun` discovers the workspace map (`World::discover(workspace,
None)`), compiles with `compile_in_realm` against the operated
repository, calls `refuse_unboxable`, and starts with
`Engine::start_in_world` under that world; its `--db` handling is
unchanged. Consequence, stated: a rerun's manifest now carries the
realms pin and the dialect, as a `run`'s does, which is a behaviour
change for a verb that today pins neither. Reason: a rerun that ignores
the realm's word refuses on a `harness` Mac for want of bubblewrap and
runs boxed on a `harness` Linux while the realm says otherwise; both
are the cross-machine substitution 0046 was written to prevent.

*Alternative (robustness, first sitting):* compile from the source
run's embedded world (rejected: `rerun` is "a NEW run under another
bundle or recipe, no stored linkage"; the source's world is not the new
run's identity, and the bundle may differ).

### DD7 — `hands.harness.work` stands (D5), fail-closed

The adapter's `hands.harness` object carries `gate`, `work` and
`result` as the delta reads. A work-class site with hands under
`harness` whose resolved chain has a link without a `work` fragment is
refused at compile naming the link.

Both positions cut the member on the second sitting, for opposite ends,
and both were refused on three facts of the tree. First,
`adapters/codex.json` declares `tool_permissions` unsupported — codex
restricts by sandbox class, not by tool name — so a codex seat's
writable class is addressable only by a fragment; there is no tool-list
path for a codex work seat. Second, the chief architect, the one shipped
work office with hands, chains `fable`, `astra`, `opus`, so it reaches
codex through astra. Third, simplicity's "the harness's default"
therefore leaves the chief read-only on astra and, on fable or opus, on
`--permission-mode acceptEdits` with no tool grant, where every shell
call prompts and a non-interactive seat answers with a denial (the
proposal's *Measurements*): `recipes/triage` compiles under `harness`
and the chief fails its charter at run time — the quiet failure
robustness warns of, and worse than a refusal. Simplicity withdrew on
the third sitting; robustness re-raised the refusal, and the answer is
the same: an unconditional refusal is what an undeclared member already
yields today, by name, so the member is the operator's data path out of
it without a new decision. The authority is ruling 1's own table:
`harness` is "the harness's own sandbox as its adapter fragment
addresses it", said of every site; ruling 4 narrows gates to the
read-only fragment and says nothing that forbids a work fragment; and
adapters are data, never match arms (0016), so the operator's act that
declares the member is a data change. Under `open` (D11) the same seat
runs at the harness's default, and whether that default writes is the
harness's fact, which the guide states; `harness` promises an addressed
sandbox and refuses where it cannot address one — fail-closed, not an
inversion of the vocabulary. What a `work` fragment allows is the
harness's fact, and the record renders the run *unboxed*, which is the
statement ruling 3 asks for.

### DD8 — A dialect step is refused at an unboxed gate until a decision admits it (amends D6; flipped on the third sitting)

A dialect validate or check step compiled under `harness` or `open` is
refused at compile, naming the step, decision 0046 ruling 4, decision
0042 ruling 4 and a boxed boundary as the road open today. The
evidence is the decisions' own words and the compiler's own call: 0042
ruling 4 admits the validator "as a boxed exec step ... boxed under
decision 0040 ruling 3, class gate", and the compiler builds a
synthetic boxed exec gate for every dialect step and passes it through
`enforce_model_policy` — so the step holds its gate today by 0043
ruling 3's box, exactly as the shipped verify script does, and not by
a standing admission outside the gate law. Under `harness` or `open`
no box stands and the step is an exec gate whose command — the
dialect's argv, `openspec validate …` behind `{brokkr} driver exec --`
— is not the bundle's own pinned script. 0046 ruling 4 admits at an
unboxed gate "a script that is the bundle's own pinned bytes" and
nothing else; the second sitting's reading admitted the realm's pinned
dialect declaration in its place on the argument that both readings
pin a declaration and run a host tool. The argument is not wrong about
the bytes; it is a widening of a ruling by a design note, which
0042's addendum ruling 1 forbids ("no spec artifact, no clarification
scenario and no design note changes what a ruling means"). The second
sitting's reading is withdrawn, and the coherence robustness asked for
is restored: the scenario is normative — refused — and the open
question is the operator's amendment, not the design's.

The consequence is stated in full. `recipes/triage` and
`recipes/night-shift`, the only shipped bundles with dialect steps,
compile under a boxed boundary only; under `harness` they refuse on
two independent grounds — the claude members (DD20) and the dialect
step — and ruling 6's promise that a macOS operator runs every shipped
bundle is unmet for them on both until the operator rules on each. The
amendment that admits the step is one ruling's worth of text, recorded
under Open questions so the operator can accept or refuse it without
re-deriving the case; when it lands, the refusal is one arm to delete
and the step composes on the pinned-script terms (the fixed
environment, the network prefix where the probe passes, the run
rendered *unboxed*), which is why the deltas keep no composition
scenario for a dialect step under `harness` or `open`: no reachable arm
may exist for a step the compiler refuses.

*Alternative:* admit the step, as the second sitting did (rejected: it
amends ruling 4 by a design note); route the step through a bundle
script that calls the tool (rejected: the recipe author would wrap a
dialect the realm declares, and the wrapper's bytes would pin nothing
about the tool it runs — the same host-tool fact, dressed as a script).

### DD9 — The pinned-script check is a grammar and a lookup at compile, and a re-walk at spawn (amends D24; amended on the third sitting; rescoped by D32 on the fourth)

The compiler judges every exec site that declares hands — gate or
work, by D32, the class unread (DD22) — under `harness` or `open` on the
*raw* command, before `expand_command` erases the `./` spelling, and
asks one question: is the script token a key the declaring layer's
manifest walk pins? The grammar (bare interpreter names before exactly
one script token, then unjudged arguments) and the lookup (plain `./`
components, a regular file by `metadata` under the declaring layer's
directory, not a key the walk skips) are stated once, in
gate-boundary-policy's requirement *An exec site with hands under
harness or open holds only for pinned bytes*; the tasks cite it and
restate nothing.
What this design fixes is the shape: the walk's skip rule is one
function shared with `manifest_for`, so the two cannot drift; the
lookup follows a symlink because the walk pins through one (`is_file()`
and `fs::read` both follow), so the bytes pinned under the key are the
bytes that run; the declaring layer's directory is the one
`expand_command`'s caller already holds; and nothing is canonicalised
and no two spellings are compared, which is how `/private/var` and `\`
are handled — a token spelled either way is refused as not
`./`-relative.

The third sitting adds the recheck, on evidence the second did not
weigh. The boxed exec gate is admitted because its blast radius is the
box (0043 ruling 3); the unboxed one is admitted because "what a pinned
script does is the digest's fact" (0046 ruling 4) — and the implement
seat writes the tree the gate's layer lives in, minutes before the gate
runs, so without a recheck the digest's fact at the gate is whatever
bytes the previous seat left. At every unboxed exec dispatch spawn the
engine therefore re-derives the declaring layer's pinned identity with
the functions the compiler already has — the leaf's `files` map by the
same walk, or an ancestor's compose digest by the same `manifest_for`
call over `Ancestor.dir` and its deeper chain — and compares it with
what the run manifest pins; any moved, missing or added pinned byte
refuses the dispatch before anything spawns, journaled as the attempt's
failure naming the layer and the first key that differs. The whole
layer is re-walked, not the script alone, because a script sources its
siblings and a sibling is as easy to edit. The residual is stated
rather than denied: the interval between the re-walk and the `exec`,
which the guide names. The namespace box keeps no recheck, and the
asymmetry is the decision's: one gate is admitted by its walls, the
other by its bytes.

*Alternatives:* canonical containment (rejected: needs both sides to
exist, behaves differently per platform, proves less than the walk's
own key set, and guards a path when an unboxed script can `exec` any
host path in one line); hashing the script token alone at spawn
(rejected: misses the siblings it sources); staging the layer's bytes
into the run's scratch and running the copy (rejected: breaks a
script's relative references to the repository and moves the pinned
argv per run); a recheck by `manifest_diff` against a full recompile
(rejected: a recompile reads agents and adapters the layer's identity
does not cover, and costs a compile per gate for the same answer).

### DD10 — The unboxed environment mirrors the box (amends D22)

Under `harness` and `open` an exec dispatch starts from an empty
environment and holds the box's own table (the allow-list `hands.rs` in
`brokkr-protocol` composes) with each entry that states a fact of the
namespace replaced by the fact that stands outside one. The table is
stated once, in gate-boundary-policy's requirement *An unboxed exec
dispatch runs in a fixed environment*; this design fixes the principle
and the reason for every entry that differs from the box's:

- `HOME` and `TMPDIR` are private directories created for the attempt
  under the run's scratch, as the box binds a private home and `/tmp`;
  never the operator's, because the real home carries the credential
  files — `.ssh`, `.netrc`, `.cargo/credentials.toml` — that clearing
  the environment was meant to take out of reach.
- `PATH` is the engine's own, because the box's fixed `PATH` names
  mounts (`/runtime`, the bound `~/.cargo/bin`) that exist only inside
  a namespace, and the interpreter and the toolchain proxies are found
  where the operator's shell finds them.
- `USER` and `LOGNAME` are the engine's own where the box fixes both to
  `runner`: the box writes a passwd naming its mapped uid `runner`, so
  the name is true inside it; the unboxed dispatch runs as the
  operator's own uid — D12's second mapping restores it — and a name
  that does not match the uid would be a false statement.
- The toolchain locators (`CARGO_HOME`, `RUSTUP_HOME`,
  `NPM_CONFIG_CACHE`) follow the site's `hands.binds` exactly as the
  box's do, `~` being the engine's home as `expand_home` reads it. A
  bind's `mask` cannot be enforced outside a namespace, and the guide
  says so: under `harness` the shipped verify script can read
  `~/.cargo/credentials.toml`, which is the fact *unboxed* renders.
- `BROKKR_HANDS_BOX` is inherited only when the engine itself already
  stands inside a box and never set by the dispatch, because it is the
  marker every box-building test skips on.
- The box's fixed switches and the bundle's `git.identity` pass
  unchanged; on Windows only, the process-bootstrap set passes
  verbatim, with `USERPROFILE` the operator's (see Open questions).

One pure function composes it from the engine's environment, the
engine's home, the site's spec, the identity and the two scratch paths;
the network probe runs in it. Release-blocking test: an engine
environment carrying `GH_TOKEN`, `ANTHROPIC_API_KEY`, `SSH_AUTH_SOCK`
and a `HOME` under which a `.ssh/id` and a `.cargo/credentials.toml`
are planted; the composed table carries none of the three, `HOME` is
the private directory, a spawned `sh -c 'cat "$HOME/.ssh/id"'` fails,
and `CARGO_HOME` names the planted `.cargo` because the shipped verify
seat declares that bind. Robustness's honesty is adopted in the claim
the test makes: it proves the *environment* hands nothing over, not
that the filesystem is confined — the same script naming the
operator's home by absolute path reads it — and the guide says that
clearing the environment confines nothing on disk: an unboxed script
may open any host path the operator's uid may read, which is the fact
*unboxed* renders.

*Alternative (proposal D22 as returned from clarify):* inherit `HOME`
and the locators verbatim (rejected: the real home carries credential
files the environment clearing was meant to take away, and the box
itself never hands it over).

### DD11 — `brokkr seats` is a thin verb whose JSON is the view (amends D8)

`brokkr seats --run <id> [--realms] [--db] [--json]` renders the seats
block the `inspect` renderer already produces, from the same `RunView`,
and with `--json` prints the view model verbatim — the same bytes
`brokkr inspect --run <id> --json` prints, under the same
`view_version`. No derivation of its own and no wire object of its own:
one arm, one test, one row in the read-surfaces guide. Reason: the
decision and the commission both name the verb, and a verb that exists
is a truer reading than a table that answers to the name; simplicity is
right that a third JSON shape would be a third contract to version for a
verb whose whole justification is that the name should be true, so the
JSON face is `inspect`'s, and a script that wants the seats reads
`.participants` and `.boundary` from it as it would from `inspect`.

### DD12 — One `ModelAtBoundary` unit, flattened, on every model-bearing carrier

`brokkr_view::ModelAtBoundary { model: Cell, boundary: Cell }` replaces
the bare `model: Cell` on `Participant`, `Node`, `CheckpointRow` and
`JournalRow` as a `#[serde(flatten)]` field named `served`, so the wire
keeps `model` and gains `boundary` as siblings (`VIEW_VERSION` 9) while
a Rust renderer cannot take the model without the boundary in reach.
One pair helper prints the pair, with a JSON face beside its text face;
the terminal seats table gains a `boundary` column beside `model`, the
trail prints `· model <x> · boundary <y>`, the TUI and the console read
the two cells. `RunView` gains `boundary: RunBoundary { word: Cell,
unboxed: bool, text: String }`, derived once: `unboxed` is true exactly
when a valid `effect/started.boundary` entry has `gate` true and
`harness` or `open`; every run header prints `text`. `costs` and
`compare` reduce `boundary` off the seat records as they reduce
`model`, `not recorded` when no record carries one; `compare`'s
`resolution` map, which reads each participant's view-derived model a
second time, carries the pair through the helper's JSON face, and
`resolution_divergence` — which compares the two maps' values
structurally — diverges on `boundary` as on `model` with no code
written for it. The roster-style pin test reads every readout source
and fails where `served.model` is read outside the pair helper — its
text face for the renderers, its JSON face for `compare` — or where a
seat-costs record's `model` key is rendered without `boundary`.

Simplicity's second-sitting two-cell shape was refused on verified cost
and the objection is withdrawn: the four carriers derive `Serialize`
only, so `flatten` changes no wire byte and no `--json` reader can tell
the two shapes apart; a construction site changes once whether a field
or a unit is added; and the divergence axis is free. What the unit
buys is a name, `served`, for the pin test to grep.

The web console is the one readout with no Rust source to scan.
`ui.html` reads `item.model`, `part.model` and `e.model` off the
flattened wire — `model` and `boundary` as siblings — and a rule
phrased as "`served.model` read outside the pair helper" never reaches
it, since the page has no `served`; as first written the console either
always failed the pin or silently escaped it. The page therefore
carries its own pair helper: one script function that takes a carrier
and returns the two cells, the only place in the page that names
`.model`, and the pin test scans `ui.html` for a `.model` read outside
that function — the way `agent_readouts.rs` already scans the page for
a composed provenance sentence — as it scans the Rust sources for
`served.model` outside theirs. The console's rendering tests (task
10.9) prove the cells land in one row; the scan proves no later edit
reads one without the other. Simplicity's
third-sitting cut of `RunBoundary.text` is refused: the web console
prints from the model `/api/view/<run>` serves and the readouts
requirement forbids it to compose the adjective on the page, so the
adjective must be on the wire or the page composes it; the view model
already carries rendered prose in every `Cell.note` ("no boundary
recorded"), so a rendered run-level line is the model's idiom, not a
new one; and one spelling on four surfaces is exactly what "wherever
the run is summarised" asks for. The data under it — `word` and
`unboxed` — stays typed for any consumer that wants its own phrasing.

### DD13 — Absence in old journals is one note; `not applicable` only from data (amended)

A participant's `boundary` cell is derived from its site's
`effect/started.boundary` entry (last attempt winning, as provenance
does); when the attempt journaled no entry — an attempt no site of
which declares hands — from the `boundary` a record of its attempt
carries beside a `model` (DD19: its finishing checkpoint, its
successful result, or the engine's own member- or step-finished
marker); and otherwise it is the absent mark with the note `no boundary
recorded`. `not applicable` is therefore rendered only when an entry or
a record actually carries the sentinel, never derived from the manifest
for a site the engine never described. A journal written before this
change carries no entry and no stamp, so every model row of it — boxed
or not — renders the one absence, and no surface prints `namespace`
for it. The run-level fact reads the journaled manifest once, for one
boolean, exactly as the view already reads the same manifest for the
agents roster: whether the run declares hands. A run whose manifest
declares hands and whose journal holds no valid entry renders the
run-level fact absent with the same note — an old boxed journal, or a
new run before its first boxed attempt, and the note is true of both; a
run whose manifest declares no hands renders nothing for it, neither
boxed nor unboxed. The gate script reads the same two facts the same
way.

The first sitting's second note (`not applicable` with `no hands
declared`, derived from the manifest's `hands` keys) is withdrawn on
both positions' evidence: it made the view speak for an engine that
recorded nothing, and it cost a per-site join against the manifest's
labels that nothing else in the view performs.

### DD14 — Strict reading without a new validator

The engine writes entries from the closed type, so a malformed entry is
unrepresentable for a journal this engine wrote. For any journal, one
reader in the view — and the same rule in one `jq` in the gate script —
treats an entry whose word is outside the six (five words and the
sentinel) or which lacks a `member` tag as *not recorded* for that
site, never as boxed or unboxed; `costs` and `compare` read the seat
records' stamps and never the entries; the gate script prints `·
boundary not recorded` for such a run. No append-time validator is
added for the extension: seat records are validated at append because
0034 rules it; extension fields never have been, and `verify-run` stays
parse, chain and fold. Robustness's fear that corrupt evidence acquires
a boxed-looking summary does not arise, and the third sitting's
sharper form — that corruption could drop the adjective while the
vouch stays admissible — has no path: accidental corruption breaks the
hash chain and `verify-run` refuses the journal at the gate's first
line, so a malformed entry needs a re-forged chain, and a forger who
can re-chain writes the boxed word as easily as a malformed one. Shape
validation defends against nothing; *not recorded* is rendered loudly
on every surface and is never the boxed word.

### DD15 — The network narrowing is probed once and is never evidence (amended)

D12 and D25 stand as clarify returned them in the prefix's tokens and
the probe's arms: the prefix is composed argv, the probe is the prefix
around `true` in the dispatch's environment, the answer is not
journaled. Amended in one respect, on simplicity's evidence: the probe
runs **once per engine process**, at the first unboxed exec dispatch,
in that dispatch's environment against its search path, and its answer
is remembered for every later dispatch of the run. The answer depends
on the kernel and on the `unshare` found on the engine's own `PATH`,
which every composed table carries verbatim; a second probe in the same
process learns nothing, and DD15's constraint forbids anyone to report
what it would learn — which is also why robustness's per-spawn probe is
refused again: freshness has no consumer, and the stale direction (the
network on where it could have been off) is reported by nobody. Added
constraint, unchanged: no readout, guide row or check summary states
that the network was off; the guides describe the prefix as a narrowing
the engine attempts on Linux and the record as saying *unboxed* either
way.

### DD16 — Landing order (refined on the third sitting)

The commission's order is the order of value and the tasks keep it as
their grouping; the landing order is: (1) ruling 5 — delete `Confine`,
`confined_command`, the `confine` fields, `confine_test.rs` and the
docker machine-proof scenario, add the parser refusal naming 0046 and
`container`; (2) ruling 1 — the enum, the v4 realms loader, the two
contracts, the parser refusal of `boundary`, the v9 map, `compile`'s
printout; (3) ruling 2 — `offered`, `refuse_unboxable`, the engine
entry fence, `doctor`, `init`; (4) ruling 4, in three steps whose order
simplicity's third-sitting evidence fixes — every shipped inline hands
site is a `./` exec gate and every hands agent chains `opus`, so the
pinned-script path carries the whole macOS promise until the claude
measurement lands: (4a) the gate law with its whole boundary axis,
fail-closed — the hands law that runs before the gate law's class read
(DD22), with the pinned-script grammar and lookup for every exec site
with hands, the dialect-step refusal, and the model arms that under `harness` admit
only a link declaring a fragment (none does until 4c) and under `open`
refuse; (4b) the unboxed exec composition — `SpawnEnv`, the fixed
environment, the network prefix and probe, `compose_site` replacing the
`hands_command(confined_command(..))` nesting with its `namespace` and
exec arms, and the spawn-time re-walk; (4c) the adapters' `hands.harness`
in the loader and the two adapter files, `compose_site`'s model arms
with the harness fragments and `{result_path}`, and the door; (5)
ruling 3 — the effect entries, seat-record v4 and the stamp, the view
unit and run-level fact, every readout, the `seats` verb, the gate
script; (6) re-pin every moved witness and compose digest once, from the
tests' left/right pairs, naming 0046 in the doc comment; then guides
and the erratum. Deletion first removes a parameter from every
`hands_command` call site before those sites are touched again; a
landing that stops after (4b) is already a working macOS story for the
nine exec-gated bundles rather than a contract with nothing behind it.

### DD17 — Availability is one probe table

`offered(path) -> BTreeMap<Boundary, Offer>` in `brokkr-cli`, with
`Offer::Offered(detail) | MissingTool(name) | Unbuilt { slice, tool:
Option<found> }`: `namespace` probes `bwrap` (and, for a bundle with an
overlay bind, 0.10 as today); `seatbelt` probes `sandbox-exec` and
reports `Unbuilt("ii")`; `container` probes `docker` then `podman` and
reports `Unbuilt("iii")`; `harness` and `open` are `Offered`.
`refuse_unboxable(bundle, path)` returns `Ok` for a bundle with no hands
site and otherwise judges `offered(path)[bundle.boundary]`, naming the
boundary, the tool, every seat that declares hands, and the ruling or
slice; it fires before any row, so a test asserts the store untouched.
`doctor` prints the same map on one `boundaries` line. The engine's
entry fence (D21) refuses `seatbelt` and `container` before any row —
not a second table but the refusing arm of the narrowing from five
words to the three composition is written over, one `match` arm with
one test, which is what lets composition carry no arm for a word it
cannot build (DD2). `init`'s warning speaks the vocabulary because its
present text — "the shipped gates require Linux with bubblewrap on
PATH" — becomes false the day a realm may declare `harness`; the
quickstart's platform paragraph is commissioned for the same reason.

### DD18 — Argv composition and the spawn environment

`compose_site(boundary, class, command, hands, harness_fragments,
workdir, roots, result_path, prefix) -> SiteSpawn { argv, env }` replaces
the `hands_command(confined_command(..))` nesting at the four call
sites (single, panel member, sequence step, dialect step). Under
`namespace` it returns today's argv token for token with
`SpawnEnv::Inherit`; under `harness` a model site gets the adapter's
`gate` or `work` fragment by class with `{result_path}` and `{brokkr}`
expanded and no MCP server, and an exec site gets the compiled command
behind the remembered network prefix when the probe passed, with
`SpawnEnv::Exactly(table)`; under `open` a model site gets the base
driver argv and an exec site the same as under `harness`. Before an
unboxed exec dispatch spawns, the engine re-walks its declaring layer
(DD9) and refuses on drift; the walk is a pure function of the layer's
directory and its pinned identity, and the refusal is the attempt's
failure. `DriverProcess::spawn` takes the `SpawnEnv` — a named
two-variant enum, not an `Option<BTreeMap>`, because a `None` at a
spawn site reads as *no environment*, the opposite of inheriting the
engine's, and the two behaviours deserve the names they are chosen by.
The seat input carries `boundary` for every site with hands, the
`hands: boxed` marker only under a box of Brokkr's, and
`result_delivery: last-message` when the gate fragment's door is the
harness's capture (D15, D23); the prompt paragraph follows.

Amended on the fourth sitting for D32 and D31. The exec arms — the
compiled command, the prefix, `SpawnEnv::Exactly` — and the spawn-time
re-walk read no class: `compose_site`'s `class` argument is consulted by
the `harness` model arm alone, to choose `gate` or `work` (DD7), and a
work-class exec site with hands takes the path the shipped verify gate
takes, token for token (DD22). The seat input's three keys are written
by one helper, the mark `mark_boxed` grows into, at its four call sites;
the prompt's paragraph follows the driver kind `render_prompt` is now
told, and an exec site's prompt carries none (DD21).

### DD19 — The stamp rides beside the model: every record that names one carries the word (amends D17; amended on the third sitting)

`stamp_boundary(record, site_boundary: Option<Boundary>)` is one
helper with one rule: a record that carries `model` carries
`boundary` beside it — the site's word, or the sentinel for a site
without hands — replacing any value a driver wrote; a record that
carries no `model` carries no `boundary`, and a driver's word on such
a record is dropped. The trigger is the record's own key, never a step
name: the boundary rides beside the model, which is ruling 3's own
sentence about the readouts applied to the datum they read. It is
applied at the two pass-throughs through which every driver record
reaches the store — the `run_driver` closure that serves singles,
sequence steps and dialect steps, and the panel receiver loop — both
of which already tag members, and at the engine's own two markers,
`panel-member-finished` and `sequence-step-finished`, which carry a
member's or a step's `model` and would otherwise name a model without
its boundary. So the finishing checkpoint carries the word, as the
decision says, because conformance asserts `model` on it for every
built-in driver; the successful result carries it for the same reason;
a per-turn checkpoint carries it exactly when a driver names a model
on it, which is true of it; and a failed attempt's word survives in
every record that named a model, where the second sitting's rule left
it only in the effect entry. The aggregate rule robustness asked for
stands: a panel's `effect/succeeded` result is the engine's own
`{result, notes}` and names no model, so it carries no `boundary` —
the cell rides beside the model, and there is none; a sequence's ending
result is its ending step's driver result, appended as it is, and
carries that step's word. The store's fence validates under v4,
dispatched by the 0.9 line (D9).

*Alternatives:* the second sitting's suffix (`-session-finished`),
rejected on robustness's objection — the protocol's lifecycle name is
not a driver-owned string, since conformance asserts it, but a rule
keyed on a name needs a second rule to strip a driver's word from every
other record, and the `model` key makes both one; threading a lifecycle
kind from the engine (rejected: the engine cannot know a streamed
checkpoint is the last until the process ends, and the record already
carries the fact that selects it); stamping every record of the
attempt unconditionally (rejected: it would stamp the engine's
model-less markers and would need the aggregate exception anyway).

### DD20 — The measurement is recorded, never guessed (D19, restated; the shipped pin restated on the third sitting)

`adapters/codex.json` declares `gate` as `--sandbox read-only
--output-last-message {result_path}` with `result` `last-message` and
`work` as `--sandbox workspace-write`, from codex's own documented
classes. `adapters/claude.json` declares `gate` and `work` only as
measured on the operator's machine against the installed 2.1.x, with
the version and what each denies and allows recorded in the guide; the
candidates are the proposal's plus the robustness seat's report of
`--restricted` and `--permission-prompts none` on 2.1.263, which this
seat could not confirm. If no combination leaves a judge its result
door or a work seat its `cargo` and `git`, the member is declared
`{"unsupported": "<measured reason>"}`, the shipped bundles that seat
claude refuse under `harness` by name, and the implementation reports
ruling 6's promise as unmet for the operator — no rule widened, no
fragment declared unmeasured.

Robustness's third-sitting objection — that codex's `gate` is declared
from documentation, not behaviour — is refused on the decision's
text: `--sandbox read-only` is ruling 4's own word for codex, and a
design that declared it unsupported would amend the decision. The door
is the design's addition (D23), declared from the tool's record with
its one unmeasured fact — whether the capture lands under the
`read-only` class — recorded in the guide as pending and the
operator's; if it fails, the failure is a missing result, loud, and
`gate` flips to unsupported with the reason. Nothing false enters the
record either way: `harness` is true of a seat under that fragment
whatever the door does.

When the measurement cannot be made by the implementing seat — its
tool grant is `cargo` and `git`, and `claude` is not a command it may
run — the claude members stay undeclared, which is the loader's
fail-closed reading and refuses the same sites by name: every shipped
bundle that seats an agent with hands whose chain reaches claude —
`bundles/self`, `recipes/panel-review`, `recipes/triage` and
`recipes/night-shift`; every hands agent chains `opus` — refuses under
`harness` naming `claude`, the member and the site, and every other
shipped bundle compiles. Every other task is finished and committed,
nothing is reported blocked for want of the measurement, and the
completion note names it as the operator's with the recipe, the
candidates and the version. The tree's proof is **one test with two
halves**: in a scratch copy of the adapter library with both members
planted as fragments, every shipped bundle without a dialect step
compiles under `harness` — eleven of the thirteen — and `recipes/triage`
and `recipes/night-shift` refuse naming their dialect step and ruling 4
(DD8); against the shipped adapters as they stand, exactly the four
bundles above refuse, each naming the ground the compiler reaches
first, and the nine others compile. That ground is not the same for all
four, and an earlier draft of this paragraph said it was: it claimed
the agent link of every sequence precedes its dialect step, so that
`claude` would be the first refusal everywhere. The tree says otherwise.
The compiler walks a bundle's seats as a `serde_json::Map`, which is a
`BTreeMap` here (`preserve_order` is not enabled; the lock carries no
`indexmap`), so phases compile in name order and `analyze` is first;
`recipes/triage`'s `analyze` and `clarify` sequences are `[check
(dialect), judge (agent)]`, and the openspec dialect declares an
`analyze` check, so the DD8 refusal of `analyze:check` is reached
before any claude link — in triage, and in night-shift, which inherits
the seat. The pin therefore reads: `bundles/self` refuses at `review`
naming `claude`, `hands.harness.gate` and the site (its reviewer chains
`astra`, `fable`, `opus`; the second link is claude's);
`recipes/panel-review` refuses at `review:correctness` the same way
(its judge chains `sol`, `opus`; panel members compile in name order
too); `recipes/triage` and `recipes/night-shift` refuse naming
`analyze:check`, ruling 4 and 0042 ruling 4, the claude ground standing
behind that refusal unreached. The second half is the pin that names
the operator's unmet part, and it moves for a known reason: the
measurement landing as a data change to `adapters/claude.json` and the
pins that name its digest, or a decision admitting the dialect step,
after which the last two name `claude` until the measurement lands —
simplicity is right that it is a pin, and that is why it stays: a pin
that moves for a known reason is the record of the reason.

Ruling 4's own binding — `model_policy_tests.rs`, "a `harness` gate on
codex admitted, on dsh refused" — is pinned in that file against the
shipped adapter library, not against a fixture shaped like either
provider, because the shipped `adapters/codex.json` is otherwise never
compiled at a `harness` gate: every shipped hands agent chains `opus`,
so the shipped-bundle half above exercises only claude refusals, the
scratch half plants the members into a copy, and the data test only
loads the file. One fixture gate agent whose agent file declares hands
and whose chain is `astra` alone compiles under `harness` against
`adapters/` as it stands and is admitted, its manifest pinning codex's
digest; the same fixture chaining one dsh model alone, and one lanetally
model alone, is refused under `harness` with the refusal it earns under
`namespace`, the two texts equal, because 0021's refusals come first
under every boundary and neither adapter declares `hands.harness`. The
arms of the chain rule stay on fixture providers, as the file's charter
requires (D29).

### DD21 — The hands paragraph is prose for a model: `render_prompt` is told the driver kind (D31)

`render_prompt(input: &Value, kind: AdapterKind) -> String`. The
paragraph — today's workspace-tool sentence under a boxed boundary, the
`harness` paragraph with its `file` or `last-message` door, the `open`
paragraph — is composed under the four model kinds from `input.hands`,
`input.boundary` and `input.result_delivery`, and under
`AdapterKind::Exec` it is the empty string whatever the input carries;
the rest of the prompt is unchanged, so `verify-seat.sh` and
`ship-seat.sh`, which read the result path off the prompt by line, read
what they read today. The one production caller, `run_seat`, already
holds `kind` — its next statement is `invoke(kind, …)` — so the change
is one signature and five call sites, no wire byte and no digest moved.
The engine's side is one helper: `mark_boxed` becomes the mark that
writes `boundary` for every site with hands under every boundary,
`hands: boxed` only when Brokkr builds the box, and `result_delivery:
last-message` for a `harness` gate whose fragment's door is the capture
(DD18), at the same four call sites. Consequence, stated so that a
later reader does not delete it as dead: at an exec site the input's
`boundary` is read by nothing at run time — the driver renders no
paragraph from it and the script reads the composed environment — and
it stays because D31 puts the word in the requested input of every site
with hands, the digest covers it, and the journal is where the
boundary-record scenario reads it back.

*Alternatives:* a driver-kind key in the seat input (rejected: the
mark's own comment — "the mark is part of the requested input, so the
digest covers it" — makes a key that exists to steer prose a journaled
field forever, on every site that gets one, and moves seat-input digests
beyond the set the boundary already moves); inferring the kind from the
expanded argv (rejected, on robustness's evidence: `hands_command`
already identifies exec by argv position after `./` is erased, and a
second such read is a second thing to keep true); a private
`render_prompt_for(input, model_backed)` behind today's signature
(rejected: a wrapper whose only caller is a test is a line the coverage
gate must then justify).

### DD22 — One hands law, total over class, before the gate law's class read (D32; fixes D10's and DD7's home; the pinned-bytes requirement retitled)

`enforce_hands_boundary(what, boundary, dir, raw, from_agent:
Option<&HandsSpec>, candidates, adapters: Option<&Adapters>)` is one
function with three arms and no class read on its exec arm, and it is
the first statement of `enforce_model_policy` at every site — before
`parse_class`'s early return for a work site without secrets and before
the destructure that `expect`s the adapters — so it runs for every site
that declares hands whatever its class, and it runs on a bundle that
never opened the adapters. A site without hands returns at once. Under
`namespace`, `seatbelt` and `container` it returns at once too: what
hands mean under a box is 0043's law, unchanged. Under `harness` and
`open`:

- an inline exec site — its raw `driver.command` a `{brokkr} driver
  exec --` dispatch — is judged by D24's grammar and lookup against
  `dir`, the declaring layer's directory every caller already holds
  (`seat_origin`, `case_origin`): admitted for the bundle's own pinned
  `./` script, refused otherwise naming ruling 4 and 0021 and, for a
  spelling, the spelling; work or gate, the class is not read (D32);
- an inline model site — a `{brokkr} driver <model driver>` dispatch —
  is refused naming the seat and the repair (D10), and any other inline
  command is refused as a bare program by the grammar's own arm;
- an agent-resolved site — `candidates` non-empty, `from_agent` the
  spec — is judged by its chain against the adapters the `agent` key
  opened, so the `Option` is `Some` on this path by `needs_adapters`'
  own rule and the arm's `expect` names that rule: under `harness` every
  link declares `hands.harness.gate` for a gate site and
  `hands.harness.work` for a work site (DD7), refused otherwise naming
  the link, the provider and the member; under `open` a gate site is
  refused naming ruling 4 and a work site is admitted to run at the
  harness's default (D11). The class is read here and nowhere else in
  the law, because here it selects a fragment — the one thing ruling 1
  gives class to do under `harness`.

`enforce_model_policy` then continues exactly as today: the class early
return, 0021's tier and grant refusals, the 0043 admission of a boxed
exec gate — which reads the hands fact off the raw site and is
therefore true under every boundary, what the hands *are* under the
boundary having been ruled by the law above — and the witness. The
dialect step's refusal (DD8) is not an arm of this law: the two sites
that build the synthetic gate (the verify-phase fold and the step loop)
share one helper that builds it, and the refusal lives in that helper
before the gate law runs, so the law never sees a dialect step under
`harness` or `open`, and its script-less arm is reached by an inline
command instead.

The exec path reads no class after the law either: `compose_site`'s
exec arms, the fixed environment, the network prefix and the spawn-time
re-walk are keyed on "an unboxed exec dispatch" (DD18, DD10, DD15, DD9),
which is what D32 makes true and what the coverage gate needs — every
shipped inline hands site is a gate (`bundles/self`, `bundles/verify`,
`recipes/fast`, `recipes/node`, `recipes/panel-review`,
`recipes/preflight`, `recipes/research`), so a work arm would be a
branch only a constructed bundle reaches, and D32 says not to write it.
The operator's scenario binds as rows. In `model_policy_tests.rs`: the
work-class pair — `./scripts/lint.sh` admitted, `true` refused naming
ruling 4 and 0021 — in the grammar's table beside the gate rows, and
the same bundle compiled with no `adapters/` directory reachable,
admitted and refused the same, which pins that the law reads no adapter
and cannot reach the `expect`. In the argv tests: `compose_site` over
one exec command with the class flipped, `Work` and `Gate`, argv and
environment equal. The environment and the re-walk are reached by the
one path there is, driven end to end by the gate's own test under
`BROKKR_HANDS_BOX`'s skip; a second box test for the work class would
reach no line the first does not.

Simplicity's home for the law, `record_hands`, was refused on one
ground: ruling 4's enforcement binding names `enforce_model_policy` and
`model_policy_tests.rs`, and a design note does not move a binding
(0042's addendum, ruling 1). Everything else simplicity read is adopted
— the panic at the `expect` on the operator's own scenario, the refusal
to widen `needs_adapters`, the law as a hands law rather than a class
law. Robustness's "separate total helper the early return cannot
bypass" is this function.

*Alternatives:* the law after the early return, with `needs_adapters`
widened to any hands site (rejected: the adapter root becomes a compile
input for a bundle that names no agent, gate or secret — the one thing
the comment above the load says it need not be — and `Adapters::load`
then fails wherever `adapters/` is absent, for a check that reads no
adapter); the law in `record_hands` (rejected on the binding alone);
a class arm in the exec path with a work case (rejected: D32, and an arm
no shipped bundle reaches).

## Shape, by crate

- **brokkr-core** — `realms.rs`: `Boundary` (no `Default`),
  `Realm.boundary`, `Realm::boundary()`, `SCHEMA_V4`, the version fence
  for the field; the `not applicable` serde helper.
- **brokkr-runtime** — `bundle.rs`: `Bundle.boundary`,
  `compile_with_realm`'s boundary argument, the `boundary` refusal at
  every site and inside `hands`, the `driver.confine` refusal, the v9
  map in `manifest_for`, the shared walk-exclusion function, the hands
  law `enforce_hands_boundary` — the pinned-script grammar and lookup
  for every exec site with hands, D10's inline-model refusal and the
  chain's fragments by class — as the first statement of
  `enforce_model_policy`, before its class read (DD22); the shared
  helper that builds a dialect step's synthetic gate, with DD8's
  refusal inside it; the declaring layer's directory, which every
  caller already holds, handed to the law, and the layer's pinned
  identity re-derived at spawn by the walk `manifest_for` already
  performs (a pure `layer_drift(layer, pinned) -> Option<key>`).
  `engine.rs`: `Engine.boundary`, the two entry fences,
  `boundary_entries` beside `select_candidates`, reading each site's
  class through `selected_is_gate` and `step.class` (DD5), the hands
  mark `mark_boxed` grows into, writing `boundary`, the marker and
  `result_delivery` at its four call sites (DD21), `compose_site`, the
  spawn-time re-walk before an unboxed exec dispatch, the remembered
  network prefix (probed once), the stamp helper at the two
  pass-throughs and the two markers, `manifest_diff` naming `boundary`;
  `confined_command` and `Confine` deleted. The adapter loader:
  `hands.harness` with `gate`/`work`/`result`, `{result_path}` admitted
  there and the two workspace tokens refused.
- **brokkr-protocol** — `process.rs`: `SpawnEnv` on `spawn`;
  `adapters.rs`: `render_prompt(input, kind)`, the prompt paragraph per
  boundary and delivery for the model kinds and none for exec (DD21);
  `hands.rs`: the unboxed environment function beside the box's table
  it mirrors, and the network prefix and its probe as pure functions.
- **brokkr-store** — `seat-record.v4` embedded and dispatched
  (`SeatRecordVersion::V4`, the 0.9 line).
- **brokkr-view** — `ModelAtBoundary`, `RunBoundary`, the boundary scan
  beside the provenance scan (creating a participant per entry as
  provenance does), the record-stamp fallback, the one manifest boolean
  beside the agents roster read, `VIEW_VERSION` 9.
- **brokkr-cli** — `offered` and `refuse_unboxable` in one module;
  `run`/`resume`/`rerun`; `doctor`'s line; `init`'s warning; `seats`
  (the seats block, or the view verbatim under `--json`); `render.rs`,
  `tui.rs`, `ui.html`, `compare.rs` reading the unit.
- **Contracts** — `realms.v4`, `run-manifest.v9`, `seat-record.v4`,
  `effect-boundary.v1`, README rows, frozen-contracts entries.
- **Data and scripts** — the two adapters; `delivered-by-brokkr.sh`.
- **Docs** — the pages `boundary-guides` names and the erratum, plus a
  `seats` row in read-surfaces and a `rerun` note in its verb row; the
  quickstart names which shipped bundles run under `harness` today and
  which refuse, and why; and the two blueprint pages whose trust rows
  still present the container class — `docs/extension-model.md` and
  `docs/target-architecture.md` — point at 0046's `container` boundary
  and slice (iii), no row or section removed (D30).

## Risks

- **The claude fragments may not measure clean, or are not measured by
  the seat at all.** In either case the chief and the reviewers on
  claude refuse under `harness` by name, `codex` is the only macOS
  judge until the operator's measurement lands, and the promise is
  reported unmet (DD20). Accepted: the alternative is an unverified
  security claim in the file whose purpose is to be the verified one.
- **The dialect recipes refuse under `harness` until a decision admits
  the step** (DD8): `recipes/triage` and `recipes/night-shift` run
  boxed only, and on a Mac they wait on two rulings, not one. Accepted:
  the alternative widens ruling 4 by a design note, and the amendment
  is recorded ready for the operator's word.
- **The spawn-time re-walk refuses a gate whose layer moved** (DD9): a
  seat or an operator editing a bundle directory during a run fails
  the gate loudly, naming the key. Accepted: that is the gate doing its
  work, and the message names the file; the walk is over the layer's
  own files, tens of them, once per unboxed exec dispatch.
- **The unboxed environment may break the shipped verify script on a
  real Mac** (a toolchain outside `~/.cargo`, a `cargo` that needs a
  variable the table drops). It fails loudly at the verify gate with the
  script's own error, on the first run; the table is one function to
  amend. Accepted.
- **A bind's `mask` is not enforced outside a namespace, and the
  environment confines nothing on disk.** Under `harness` the verify
  script's build scripts can read `~/.cargo/credentials.toml` and any
  host path the operator's uid may read (DD10). This is the fact
  *unboxed* states; the guide names it. Accepted as ruled: the
  mitigation the operator ruled is the cleared environment and the
  network off where the platform can say so.
- **The network prefix varies across kernels and policies.** The probe
  answers once per engine process; nothing claims the outcome (DD15).
  A machine whose answer changes mid-run runs its later dispatches on
  a stale answer — with the network on where it could have been off, or
  a prefix that fails loudly at the gate — and neither is reported as a
  fact, so nothing false is recorded. Accepted.
- **A work seat under `harness` waits on a measured `work` fragment**
  (DD7): until claude's is measured, `recipes/triage` and
  `recipes/night-shift` refuse under `harness` by name. Accepted: the
  refusal is the honest state and the member is the data path out.
- **Every boxed bundle's digest moves once, and every inline gate on
  an adapter that gained `hands.harness` moves with it — codex now,
  claude when its measurement lands.** Re-pinned once at the end
  (DD16); a plain bundle is the fixed point that witnesses the key is
  absent by default.
- **`rerun` changes behaviour** (DD6): its manifest gains the realms
  pin and dialect. Its tests move; its guide row says so.
- **Coverage.** The gate is literal; every arm this design adds is in a
  pure helper with a table test (`offered`, the grammar, the layer
  re-walk, the environment, the probe, `compose_site`, the stamp, the
  view derivation), and the end-to-end box tests skip under
  `BROKKR_HANDS_BOX`.
- **`enforce_model_policy` gains a law before its class read** (DD22):
  every site with hands is judged by the boundary before 0021's class
  law runs, and a smith who places the law after the early return
  reproduces the panic on the operator's scenario. Defended by the
  no-adapters row in `model_policy_tests.rs`, which cannot pass with
  the law behind the destructure. Accepted.
- **The input's `boundary` at an exec site is read by nothing at run
  time** (DD21). It is evidence the digest covers and the journal
  carries, and the design says so where a later reader would look.
  Accepted.
- **Windows is named by ruling 6 and measured by nobody** (Open
  questions).

## Migration

- Contracts: four new files beside the frozen ones; nothing edited.
- Journals: old journals need nothing and render one explicit absence
  on every model row (DD13); journals from the tagged 0.9.0 and 0.9.1
  engines validate under v4 because it is additive (D9); records this
  engine writes carry `boundary` beside every `model` (DD19).
- Bundles: no shipped bundle declares `boundary` or `driver.confine`;
  none moves for either reason. Every bundle with hands moves once for
  the manifest key; the witness table and compose pins are re-pinned
  with 0046 as the reason. The two recipes with dialect steps compile
  under boxed boundaries only until a decision admits the step (DD8).
- Realms: this repository's `realms.json` is untouched; absence reads
  `namespace`. A macOS adopter adds `"boundary": "harness"` to their
  realm under `forge.realms/v4`.
- Adapters: `codex.json` gains `hands.harness` now; `claude.json` gains
  it when the operator's measurement lands and declares none until
  then; `dsh` and `lanetally` declare none and are refused at a
  `harness` gate as at a boxed one.
- CLI: `rerun` pins the world it discovers; `seats` is new and its
  `--json` is `inspect`'s; `doctor` gains one line; `init`'s warning
  changes wording. `--json` consumers pin `view_version` 9.
- Decision 0046 gains its one-line erratum; the `Status:` line is
  untouched.

## Open questions

1. **claude's `hands.harness.gate` and `.work`** — the operator's to
   measure against the installed 2.1.x (the proposal's candidates plus
   `--restricted`/`--permission-prompts none` as reported for 2.1.263)
   by the recipe the proposal's *Measurements* section gives; until
   then the members are undeclared, the four bundles DD20 names refuse
   under `harness` by name, and the completion note says so. Landing
   the measurement is a data change to the adapter and to the pins that
   name its digest.
2. **codex's capture under `--sandbox read-only`** — whether
   `--output-last-message` lands under the read-only class, measured
   the same way by the operator; declared meanwhile from the decision's
   word and the tool's record and recorded as pending in the guide; if
   it fails, `gate` is unsupported with the reason.
3. **The dialect step at an unboxed gate (DD8)** — refused until a
   decision admits it. The amendment is one ruling, offered here for
   the operator to accept or refuse in its own number: *a dialect
   validate or check step, whose argv is the realm's pinned dialect
   declaration (0042 ruling 1) and whose tool is named and versioned by
   the dialect, may hold its gate under `harness` and `open` on the
   pinned-script terms — the fixed environment, the network narrowed
   where the platform can say so, the run rendered unboxed — because
   its argv is the digest's fact as a bundle script's bytes are, and
   the tool it runs is a host tool as `bash` and `cargo` are.* Until
   then `recipes/triage` and `recipes/night-shift` compile under boxed
   boundaries only; when it lands, one arm is deleted and the step
   composes on the exec terms already built.
4. **"Every `effect/started`" read literally** — needs a v2 event
   lineage and is outside this slice's contracts; the operator says
   whether the narrowed reading (DD5) suffices. Either answer changes
   one presence condition and nothing else.
5. **Windows under `harness`/`open`** — the environment table is
   exercised by the pure test on Windows CI; the shipped `sh` gates
   parse a Unix result path and need a POSIX shell, and `USERPROFILE`
   stays the operator's. Nothing here claims a Windows run; a Windows
   measurement is its own slice.
6. **The three questions 0046 leaves unruled** stay unruled here; #214
   (per-site boundary) is not built for.
