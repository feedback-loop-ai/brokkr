# Change: The boundary is named — decision 0046, enactment slice (i)

## Why

Decision 0043 boxed the model's hands in a bubblewrap namespace and
refuses a boxed bundle at run start on a machine without bubblewrap.
Since the verifier and shipper became boxed `exec` scripts, that refusal
reaches every shipped delivery recipe and the quickstart itself: macOS
and Windows compile every bundle and run none of them.

Decision 0046 (accepted 2026-09-05) names the axis instead of adding a
flag: a box has a `boundary`, declared by the realm, pinned in the
manifest, refused where the machine cannot build it, recorded in the
journal, and shown wherever the model is shown, with a run judged under
`harness` or `open` rendered *unboxed*. This change is ruling 6's slice
(i), the word and the pin.

## Context

A `boundary` is one of `namespace`, `seatbelt`, `container`, `harness` or
`open`, declared by the realm, because the machine a realm runs on is the
realm's fact and never a bundle's; absent, it reads `namespace`, which is
what every bundle meant until today. The manifest pins the resolved
boundary per site, so a run under one boundary and a run under another
are two identities (0043 ruling 4; 0021 ruling 5). The record says which
boundary stood, every readout that names a seat's model names its
boundary, and a run whose gate stood under `harness` or `open` is
rendered *unboxed* wherever the run is summarised — the delivery gate's
check summary included. `driver.confine` retires into the `container`
boundary and is refused until slice (iii) measures it.

After this slice a macOS operator runs every shipped bundle, the review
offices under their harness's own sandbox, and every readout says so.

A specification is worth writing because the commission leaves facts to
record before a line is built: the seat-record version is v4, not the
v3 the decision names, because #202 landed v3 under decision 0034
rulings 6 and 7; the installed claude has no `read-only` permission
mode, so `hands.harness` is measured rather than copied from the
decision's prose; the journal does not carry a site's class, which the
*unboxed* rule needs; the seat box that authors this change holds no
`claude` binary, so that measurement is the operator's; and the readings
the decision leaves open — work seats with hands under `harness`,
dialect validate steps under `harness`, how a read-only judge delivers
its result file, what the seat input says when no box stands, and which
surfaces count as a readout of a seat's model — are ruled below with
their reasons, so the next judge reads the refutation instead of raising
the finding.

## What Changes

In the commission's order of value:

1. **The word (ruling 1).** `contracts/realms.v4.schema.json` is v3 plus
   one optional per-realm field `boundary`, an enum of exactly
   `namespace`, `seatbelt`, `container`, `harness`, `open`; absent reads
   `namespace`. `contracts/run-manifest.v9.schema.json` is v8 plus
   `boundary`, a map from hands site label to the resolved word, present
   exactly when the manifest has `hands`. The realms loader reads v4 and
   refuses the word under an older label. The bundle parser refuses
   `boundary` at any site and inside `hands`, naming the realm as its
   home. `brokkr compile` prints the manifest, which under v9 carries
   `boundary` keyed by the same site labels as `hands`, so each boxed
   site's boundary is read off the printout beside its box spec; the
   printout gains no second copy of the map. This repository's
   `realms.json` is untouched: its absence means `namespace`.
2. **Refusal and doctor (ruling 2).** `refuse_unboxable` speaks the
   whole vocabulary — `namespace` needs bubblewrap 0.10 or newer,
   `seatbelt` needs `sandbox-exec`, `container` needs `docker` or
   `podman`, `harness` and `open` are always offered — and `run`,
   `resume` and `rerun` refuse at start naming the seats that need the
   boundary. `seatbelt` and `container` are named, pinned and admitted
   at compile in this slice and refused at start on every machine, after
   the tool check, naming the slice of ruling 6 that builds them (D21).
   `brokkr doctor` prints one line naming the boundaries a run can start
   under on this machine with this engine and, for the two unbuilt ones,
   whether their tool is on PATH.
3. **The record (ruling 3).** `effect/started` carries `boundary` beside
   `provenance`, published as `contracts/effect-boundary.v1.schema.json`
   under the contracts README's extension rule.
   `contracts/seat-record.v4.schema.json` adds `boundary` to the
   finishing checkpoint and the successful result, stamped by the engine,
   `not applicable` for a site without hands. `brokkr-view` exposes the
   boundary beside every model cell it carries and as a run-level fact;
   `inspect`, `watch`, the TUI and the web console render it; `costs`
   and `compare` name it from the seat records; `export` carries it as
   data; a run in which a gate site stood under `harness` or `open` is
   rendered *unboxed*, in the delivery gate's check summary too. Old
   journals render an explicit absence, never a default.
4. **Which boundaries may hold a gate (ruling 4).** `adapters/codex.json`
   and `adapters/claude.json` gain `hands.harness` — `gate`, `work` and
   `result` — with the seat's result path reaching a fragment through
   the `{result_path}` token (D23); dsh and lanetally declare none.
   `enforce_model_policy` gains the boundary axis for sites that declare
   hands; an exec gate under `harness` or `open` is admitted only for
   the bundle's own pinned script — a `./` token naming a file under the
   layer that declared the seat (D24) — run in the fixed environment D22
   lists, the in-box marker never set, and the network off where the
   platform can say so. At run time the argv of a site with hands
   follows the boundary: today's box under `namespace`, the harness's
   own sandbox under `harness`, nothing of Brokkr's under `open`.
5. **`driver.confine` is refused (ruling 5).** The parser refuses the
   field naming decision 0046 and the `container` boundary; the engine's
   `docker run` wrapper and the `Confine` type are deleted; the
   recipe-authoring row points at 0046.

Guides are updated and never trimmed; decision 0046 gains a one-line
`## Erratum`; every witness and compose digest that moves is re-pinned
with 0046 as the stated reason.

## Capabilities

### New Capabilities

- `realm-boundary`: the boundary word — its closed vocabulary in one
  shared type, its home in the realm map as `forge.realms/v4`, its
  default, its resolution at compile and on resume, its refusal inside a
  bundle or an agent, and its printout by `brokkr compile`.
- `boundary-manifest-pin`: `run-manifest.v9` — the per-site boundary map
  as bundle identity, the frozen-contracts pins, the witness and compose
  digests re-pinned for 0046, resume under the pinned boundary, and the
  Looper lineage's refusal.
- `boundary-availability`: the machine's answer — `refuse_unboxable` over
  the whole vocabulary at `run`, `resume` and `rerun`, `brokkr doctor`'s
  offered-boundaries line, and `brokkr init`'s warning.
- `boundary-record`: the journal's answer — `effect/started.boundary` as
  a numbered extension schema, `seat-record.v4`, the engine's stamping of
  finishing checkpoints and results, the store's dispatch by engine line,
  and the seat input and prompt that name the boundary a seat stands
  under.
- `boundary-readouts`: the rendering — the view's boundary cell beside
  every model cell it carries and its run-level unboxed fact, `inspect`,
  `watch`, the TUI, the web console, `costs` and `compare` from the seat
  records, `export` as data, explicit absence for old journals, and the
  delivery gate's check summary.
- `gate-boundary-policy`: decision 0021's gate law under the boundary
  axis — `hands.harness` in the adapters, model gates per boundary, the
  bundle-pinned-script reading for exec gates and dialect steps, the
  run-time argv per boundary and class, and the judge's result door.
- `driver-confine-retirement`: decision 0008's container confinement
  refused by name, its engine wrapper and type deleted.
- `boundary-guides`: the prose — provider-adapters, recipe-authoring,
  quickstart, journal-and-verification, read-surfaces, repository-layout,
  driver-authoring, ARCHITECTURE, the contracts README, and the
  decision's erratum.

### Modified Capabilities

None. `openspec/specs/` holds no capability yet; under decision 0042's
addendum the truth is seeded from accepted decisions, and these deltas
are the first written against it. Every requirement cites the ruling
that authorises it.

## Impact

- **Contracts (new files beside frozen ones, never edits):**
  `contracts/realms.v4.schema.json`, `contracts/run-manifest.v9.schema.json`,
  `contracts/seat-record.v4.schema.json`,
  `contracts/effect-boundary.v1.schema.json`; rows in
  `contracts/README.md`; entries in
  `crates/brokkr-runtime/tests/frozen_contracts.rs`.
- **Crates:** `brokkr-core` (the `Boundary` type; `forge.realms/v4` in
  the map loader), `brokkr-runtime` (bundle parser and `manifest_for`,
  `enforce_model_policy` with the pinned-script check, the adapter
  loader's `hands.harness` — `gate`, `work`, `result` — and the
  `{result_path}` token, the agent resolver's fragment choice, the
  engine's entry fence for `seatbelt` and `container`, its effect-start
  path, argv composition and record stamping, `World`'s boundary
  resolution, `manifest_diff` naming `boundary` when it is the non-file
  field that differs; `Confine` and `confined_command` deleted),
  `brokkr-protocol` (the seat prompt paragraph in both result
  deliveries, the unboxed exec wrapper beside `hands` with its
  environment table and the `unshare` probe), `brokkr-store`
  (seat-record v4 embedded and
  dispatched), `brokkr-view` (a boundary cell beside every model cell —
  participant, node, checkpoint row, journal row — the run-level fact,
  `VIEW_VERSION` 9), `brokkr-cli` (`compile`, `run`/`resume`/`rerun`,
  `doctor`, `init`, `render.rs`, `tui.rs`, `ui.html`, and `compare.rs`
  for `costs` and `compare`).
- **Data:** `adapters/codex.json` and `adapters/claude.json` gain
  `hands.harness`; `realms.json` is untouched.
- **Identity:** every pinned bundle that declares hands moves once,
  because the manifest gains `boundary`; every inline gate that pins the
  codex or claude adapter moves, because the adapter file changed. The
  witness table and the compose pins are re-pinned with 0046 as the
  stated reason. A bundle that boxes nothing and consults neither adapter
  keeps its digest.
- **Scripts and CI:** `scripts/delivered-by-brokkr.sh` renders the word;
  `.github/workflows/ci.yml` is unchanged (the job runs the base branch's
  script).
- **Docs:** the eight guides and pages named under `boundary-guides`, the
  contracts README, and the erratum in
  `docs/decisions/0046-the-boundary-is-named.md`.
- **Deleted:** `Confine`, `confined_command`,
  `crates/brokkr-runtime/tests/confine_test.rs`, and the docker-gated
  machine-proof scenario.
- **Out of the blast radius:** `policy/phase-machine.json`,
  `policy/schemas/`, `reference/`, `fixtures/evaluator/corpus.ndjson`,
  every frozen contract, the event vocabulary, `fold`, and every
  historical journal.

## Decisions

Each is a reading the decision leaves open, ruled here with its reason
and encoded as a scenario in the owning delta — the dialect's place for
an answered ambiguity — so that a judge reads the refutation as part of
the artifact rather than raising the finding again (decision 0042 ruling
6: a change handed in is adopted, validated and amended with reasons).

- **D1 — The axis is scoped to sites that declare hands.** A site
  without hands has no box whose boundary could be named: it keeps
  decision 0043's tool-list path unchanged under every boundary, its
  record says `not applicable`, and ruling 4's gate law does not touch
  it. Reason: `recipes/fast`'s inline reviewer declares no hands and is
  admitted today under `namespace`; its situation is identical under
  `open`, and a realm's word cannot change a fact about a site that has
  no box. Reading "`open` never holds a model gate" as reaching a
  hands-less gate would make the same seat lawful in one realm and
  unlawful in another for no difference in what it can touch.
- **D2 — One word per run, pinned per site.** The boundary is the
  realm's, so every hands site of a run stands under one word; the
  manifest still pins it per site because the manifest's `hands` map is
  per site and ruling 1 says "the resolved boundary per site".
- **D3 — `effect/started.boundary` names the site, the word and the
  class.** The journal does not carry a site's class today, and ruling 3
  renders *unboxed* for a *gate* site. The entry therefore carries
  `gate`, engine-owned and additive under the contracts README's
  extension rule, so the view and the gate script ask the journal
  exactly the decision's question instead of joining site labels against
  the manifest or widening the rule to every boxed site.
- **D4 — The engine stamps the seat record.** The engine is the only
  party that knows which boundary it built, so it adds `boundary` to the
  finishing checkpoint and the successful result before the store's
  append fence validates them (0034 ruling 6). Drivers and their
  conformance field sets do not change; the exec driver's `not
  applicable` is the engine's stamp for a site without hands.
- **D5 — `hands.harness` carries two fragments, `gate` and `work`.**
  Ruling 4 names the read-only fragment under which a model may judge.
  Two shipped work offices declare hands — `chief-architect` and
  `intake-sdd` — and must write and commit under `harness`, which a
  read-only fragment forbids; and the codex harness's own default under
  `exec` is not writable either, so "no fragment" is not a working
  answer for them. `hands.harness` is therefore an object whose `gate`
  member is ruling 4's read-only fragment and whose `work` member is the
  harness's own writable sandbox, each an argv fragment or
  `{"unsupported": "<measured reason>"}` on the three-shape convention
  `tool_permissions` already uses. A work site under `harness` whose
  adapter declares no `work` fragment is refused at compile as the
  capability gap it is.
- **D6 — Dialect steps are admitted under `harness` and `open` as pinned
  argv, and this is the reading most in need of the operator's word.** A
  dialect validate or check step is an exec gate whose argv is the
  dialect's own, pinned in the run manifest by the dialect's content
  digest (0042 ruling 1) together with the tool's name and version
  (dialect v2). The bytes that run — the `openspec` or `specify` binary —
  are not pinned by digest, so this is weaker than ruling 4's
  bundle-pinned script, and the proposal says so rather than dressing it
  up. It is admitted anyway, on the same cleared-environment and network
  terms and with the run marked unboxed, because the alternative refuses
  every artifact phase on macOS and breaks ruling 6's own promise that a
  macOS operator runs every shipped bundle. If the operator refuses this
  reading, the scenario "A dialect step under harness is admitted" is
  the one line to delete, and `recipes/sdd` and `recipes/triage` then
  compile only under a boxed boundary.
- **D7 — A judge under `harness` still delivers its result file.** The
  result file is the only channel the engine reads, and a read-only
  harness sandbox leaves nothing writable. The `gate` fragment must
  therefore be measured to leave exactly one write door, the result
  path; the candidates are named under *Measurements*. An adapter whose
  measured read-only mode leaves no door declares `gate` unsupported
  with the reason and is refused at a `harness` gate, which is the
  honest outcome rather than a judge that cannot speak.
- **D8 — `brokkr seats` is the seats table.** No verb of that name
  exists in this tree; the decision's "`brokkr seats`" is read as the
  seats table `brokkr inspect` and `brokkr watch` render. `brokkr
  export` writes the journal and its manifest as data and renders no
  prose, so its part of ruling 3 is the record itself: the exported
  journal carries the word and `verify-run` accepts it.
- **D9 — Seat-record v4 dispatches by the 0.9 line.** `engine` carries
  no position within a line, so v4's boundary is the line this slice
  lands in: an engine at or after `0.9.0` reads v4, the `0.8` line keeps
  v3, earlier engines keep v1. The tagged 0.9.0 and 0.9.1 engines wrote
  no `boundary`, which v4 admits as absent (0034 ruling 5).
- **D10 — An inline model site with hands under `harness` or `open` is
  refused at compile.** Its argv is the author's and carries the box's
  own tokens (`{hands_mcp_json}`, `{hands_args_toml}`), which no
  fragment substitution can rewrite honestly. The refusal names the
  repair: seat it through an agent, or run the realm under a boxed
  boundary. No shipped bundle has such a site.
- **D11 — `open` adds nothing.** Under `open` a site with hands runs on
  the adapter's base driver argv with the harness's own default
  permissions; Brokkr adds no fragment and serves no tool. Whatever the
  harness denies by default is the harness's fact and the guide says so.
- **D12 — The network is turned off with the unprivileged form of
  `unshare`.** `unshare -n` alone needs privilege; the wrapper uses a
  user namespace first and is applied only when `unshare` is on PATH and
  the kernel permits, probed at run time, never assumed.
- **D13 — `rerun` compiles without a world today.** It therefore
  compiles under `namespace`; widening `rerun` to take a map is a
  change of its own and is not made here.
- **D14 — The extension field is absent by default, which is what makes
  it legal at event_schema 1.** The contracts README's amended rule
  (decision 0016) admits an additive payload field at `event_schema: 1`
  only when it is optional, absent by default and published as a
  numbered extension schema; a field on every `effect/started` would
  fail the second condition and be a v2 event, which the closed `type`
  enum and the frozen manifest schemas forbid. `boundary` therefore
  rides `effect/started` on the exact terms `provenance` set: for an
  attempt with at least one boxed site, and a run over a bundle that
  boxes nothing journals byte-identical payloads. Ruling 3's "every" is
  honoured where the record can afford it: the seat record carries
  `boundary` on every finishing checkpoint and successful result, `not
  applicable` for a site without hands.
- **D15 — The seat input's `hands: boxed` marker stays true.** The
  marker exists to tell a seat that the workspace tool is its only
  writer. Under `namespace`, `seatbelt` and `container` it stays, byte
  for byte; under `harness` and `open` no box of Brokkr's stands and no
  workspace tool is served, so the marker is absent rather than false,
  and the input's `boundary` field — present for every site with hands,
  under every boundary — carries the word the prompt paragraph keys off.
  A site without hands carries neither, as today. The data stays the
  plain word; a marker that read `boxed` over an open seat would be the
  false statement in the record that ruling 3 exists to prevent.
- **D16 — Every model cell, not only the seats table.** Ruling 3 says
  every readout that shows a seat's model, and the tree shows one in
  more places than the seats table: the view carries a `model` cell on
  the phase rail's nodes, on checkpoint rows and on journal rows, the
  terminal's decision trail prints `· model <x>`, and `brokkr costs` and
  `brokkr compare` name a seat's model from the seat records through
  `compare::seat_costs`, outside the view. The boundary therefore
  travels beside every one of them, and the pin test scans for a model
  cell read rather than for a list of surfaces this proposal could
  leave short. The absence word differs by surface for a reason: a view
  cell is the absent mark with a note, as every other cell is; a
  seat-costs record is JSON data and says `not recorded`, the way the
  same record says `not reported` for a model no driver reported.
- **D17 — The finishing checkpoint is the one whose `step` ends in
  `-session-finished`.** The schema cannot tell a finishing checkpoint
  from a per-turn one, so v4 admits `boundary` on any checkpoint, and
  the engine stamps only the record the view already treats as the
  finishing one. The engine's stamp replaces a driver's word: a driver
  never learns which boundary stood, so a value it wrote can only be a
  guess.
- **D18 — Under `harness` and `open` the hands policy is declared, not
  enforced.** `hands.network` and `hands.binds` stay in the manifest as
  the site declared them, because they are bundle identity; nothing of
  Brokkr's enforces them there, because no box of Brokkr's stands. The
  harness's own sandbox decides, and *unboxed* on every readout is the
  statement of exactly that. Dropping the policy from the manifest under
  those boundaries would move digests for no change in what the bundle
  asks.
- **D19 — Ruling 6's promise is conditional on the measurement, and a
  gap is reported, not fudged.** Every shipped bundle compiles under
  `harness` exactly when the codex and claude adapters declare both
  `hands.harness` members as fragments; the work offices that declare
  hands chain both providers. Codex's answers are its documented sandbox
  classes; claude's must be measured. If the measurement finds no mode
  that leaves a judge its result door, or no writable mode a work seat
  can stand in, the shipped bundles refuse under `harness` by name and
  the implementation reports the promise as unmet for the operator to
  rule on — a decision is amended only by a decision (0042 addendum,
  ruling 1), so no rule is widened and no fragment is declared
  unmeasured to make the tree compile. An empty fragment is a legal
  measured answer where the driver argv already stands in the mode.
- **D20 — The deltas name their enforcement bindings, and open with a
  `## Purpose`.** The dialect's own instructions ask a spec to avoid
  internal names; this realm's truth is seeded from decisions whose
  rulings bind by name (0042 addendum, ruling 2), and a delta that names
  the binding is one a judge can test against the tree and one the
  analyze check can trace to a task. The names stay. Each delta opens
  with `## Purpose` because the dialect archives that section into the
  main spec it creates, and a truth tree seeded with `TBD` placeholders
  would say nothing about what each capability is for.
- **D21 — `seatbelt` and `container` are named, pinned and admitted at
  compile in this slice, and refused at start on every machine.** Ruling
  1 gives all five words a pin and ruling 4 admits gates under the three
  boxed boundaries, so a realm that declares `seatbelt` compiles today
  and its manifest carries the identity slice (ii) will run under. But
  no engine of this slice composes a seatbelt or a container box — that
  is what ruling 6's slices (ii) and (iii) measure — and the boundary is
  never simulated (0043 ruling 1). So `refuse_unboxable` refuses both at
  `run`, `resume` and `rerun`: after the tool check, so an empty PATH is
  refused naming the tool as ruling 2 requires; and with the tool
  present, naming the tool found, the slice that builds the boundary and
  `harness` as the road open today. The engine refuses the same bundle
  at its own entry before any journal row, so a library caller reaches
  no composition that does not exist, and composition is written over
  the three boundaries this engine builds, never with an arm for a word
  it cannot — an arm no test could reach. `doctor` says *offered* only
  of a boundary a run can start under here: `namespace` with its
  bubblewrap, `harness` and `open`; for `seatbelt` and `container` it
  reports the tool as a readiness fact and names the slice. Reason: a
  run admitted because `sandbox-exec` is on PATH would run its seats
  under no box at all while the record said `seatbelt` — the false
  statement ruling 3 exists to prevent — and a compile refusal would
  leave the word without the pin ruling 1 promises. The refusal is the
  one line slice (ii) deletes, and nothing else moves when it does.
- **D22 — The unboxed exec environment is a fixed table, and the in-box
  marker is never set.** Ruling 4's "environment cleared" takes the
  operator's variables — keys, tokens, sockets — out of a script's
  reach. The box's own allow-list (0043 ruling 1) names what a script
  needs and nothing else, but its values are box paths that name nothing
  outside a namespace, and `BROKKR_HANDS_BOX` set outside one would be a
  false statement that also switches off every test the constraints
  require to skip only inside a box. The unboxed dispatch therefore
  starts from an empty environment and holds exactly what
  `gate-boundary-policy` lists: inherited verbatim from the engine's own
  environment, and only when set there, the locators (`PATH`, `HOME`,
  `USER`, `LOGNAME`, `TMPDIR`, `CARGO_HOME`, `RUSTUP_HOME`,
  `NPM_CONFIG_CACHE`, and on Windows the process-bootstrap set without
  which no Windows program starts), the in-box marker itself when the
  engine already stands inside a box, and the box's own fixed entries
  (`C.UTF-8`, `CI`, the updater and telemetry switches, the unsigned
  commit triple, the bundle's `git.identity`). No other key passes. A
  script finds `cargo` where the operator's own PATH finds it and the
  toolchain where `HOME` or the operator's own `CARGO_HOME` points,
  which is why the shipped verify gate runs under `harness` on a rustup
  machine. Reason for inheriting `PATH` and `HOME` rather than deriving
  them: they locate the interpreter and the toolchain on all three
  platforms, they carry no secret, and a derived PATH is a second copy
  of a fact the operator's shell already owns.
- **D23 — The result door is a token in adapter data.** A judge under
  `harness` has one write door, and both measured candidates need the
  seat's result path, which exists only at spawn. `{result_path}` joins
  the fragment vocabulary: expanded by the engine at spawn beside
  `{brokkr}`, admitted only in `hands.harness` fragments, where the two
  workspace tokens are refused because no workspace tool is served
  there. `hands.harness.result` says how a gate seat's result reaches
  the engine: `file`, the default, when the seat writes the file itself
  through a door the fragment scopes to the expanded path;
  `last-message`, when the harness's own capture writes the seat's final
  message there — codex's `--output-last-message`, which its driver
  already admits on a resume. The prompt's result contract follows the
  declaration. Reason: adapters are data, never match arms (0016); a
  driver that appended the flag from its own input would be provider
  behaviour in Rust, and the flag would be missing from the adapter file
  whose digest the manifest pins as the witness of what authorised the
  gate (0021). The `work` fragment needs no door: a writable worktree
  holds the result path.
- **D24 — "Pinned bytes" is checked by construction, not by comparing
  paths.** The compiler already expands a `./` command token against the
  directory of the layer that declared the seat, and the manifest walk
  of that layer digests every file under it. So a token is the bundle's
  own pinned script exactly when it is `./`-relative, its components are
  plain names (no `..`, no `.`, no empty component, no `\`, no absolute
  prefix) and the joined path is a regular file at compile. No path is
  canonicalised and no two spellings are compared, which is how the rule
  is platform-neutral: a `/private/var` or `C:\` spelling is refused as
  not `./`-relative, never compared. Reason: a canonicalising comparison
  needs the file to exist on both sides, behaves differently per
  platform, and proves less — a file under the declaring directory that
  the walk did not pin is impossible, so the existence check is the
  whole proof.

## Measurements the seat box cannot make

The box that authors and implements this change binds the host toolchain
read-only and holds no `claude` binary; the implementer's tool grant is
`cargo` and `git`. Two facts must be measured on the operator's machine
and recorded, never guessed (0046 ruling 4's own words):

- **claude's `hands.harness.gate` and `hands.harness.work`.** The
  installed claude's `--permission-mode` choices are `acceptEdits`,
  `auto`, `bypassPermissions`, `manual`, `dontAsk`, `plan` — there is no
  `read-only` value. Candidates for `gate`: `--permission-mode dontAsk`
  with `--allowedTools` naming the read tools and an edit rule scoped to
  `{result_path}` (`result` `file`), or `--permission-mode plan` if it
  can still write the result file. Candidates for `work`:
  `--permission-mode acceptEdits` with the shell allowed, or the
  harness's own sandbox settings with the shell auto-allowed when
  sandboxed — a bare `acceptEdits` prompts for every shell call, and a
  non-interactive seat answers a prompt with a denial, so the work
  fragment must be measured to leave the chief its `cargo` and `git`.
  The provider-adapters guide records the claude version each was
  measured against and what it denies and allows. The binary that
  guide's doctor transcript already records is claude 2.1.251, so the
  measurement is against that line; for `work` the empty fragment is a
  candidate answer only if the measurement shows the driver's own
  `--permission-mode acceptEdits` grants the shell.
- **codex's result door under `--sandbox read-only`.** The read-only
  sandbox blocks every file write the model asks for. The door is the
  harness's own `--output-last-message {result_path}` capture with
  `result` `last-message` (D23): the flag is documented by
  `codex exec --help` and already admitted by the codex driver on a
  resume, so the declaration comes from the tool's own record; whether
  the capture lands under the read-only class is measured on the
  operator's machine, and if it does not, `gate` is declared unsupported
  with the reason.

## Constraints (all binding)

- Read-only, never edited: `contracts/run-manifest.v1` to `v8`,
  `contracts/realms.v1` to `v3`, `contracts/seat-record.v1` to `v3`,
  `contracts/effect-provenance.v1`, `policy/phase-machine.json`,
  `policy/schemas/`, `reference/`, `fixtures/evaluator/corpus.ndjson`.
  A contract change is a new numbered file beside the old one.
- Nothing else is renumbered; the erratum is one line under
  `## Erratum` and the decision's `Status:` line is untouched.
- Every witness and compose digest that moves is re-pinned from the
  tests' left/right pairs with 0046 as the stated reason in the pin
  file's doc comment.
- Any test that drives a boxed step end to end skips under the in-box
  marker `BROKKR_HANDS_BOX` (the pattern in
  `crates/brokkr-cli/tests/hands.rs`).
- Path comparisons are canonical-safe and platform-neutral: macOS
  `/private/var`, Windows separators; CI runs the suite on ubuntu, macos
  and windows.
- The data carries the plain boundary word; only rendering carries the
  adjective *unboxed*. Old journals render an explicit absence, never a
  default (0031 ruling 3).
- The gate script stays bash 3.2-compatible (macOS): no `mapfile`.
- Production code is Rust under `crates/` (decision 0009); adapters are
  data, never match arms (decision 0016).
- Gates before success: `cargo fmt --all -- --check`;
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`;
  `cargo test --workspace --no-fail-fast`;
  `cargo package --workspace --allow-dirty`; `brokkr compile --bundle
  bundles/self` and `--bundle bundles/verify`; `scripts/coverage-exact.sh`
  at literal 100% of lines and branches, a mark repeated at several call
  sites being one helper.
- Commit with the repository's message style; never push.

## Out of scope

- `seatbelt` measured on a Mac (slice ii); `container` absorbing decision
  0008's image, network and mounts (slice iii). A realm that declares
  either compiles and pins the word today and refuses at start naming
  the slice (D21).
- The three questions 0046 leaves unruled: a `harness` vouch merging
  without `by-hand`; the container image pin; whether seatbelt masks the
  hooks path.
- Adding `boundary` to this repository's `realms.json`.
- Changing what `hands` allows (`network`, `binds`), any driver's trust
  tier, the result contract, or `rerun`'s lack of a map.
- A new decision document: 0046 is accepted; this slice enacts it.

## Verification obligations

The scenarios in the deltas are the tests' shape. Named here because the
commission names them: a frozen-contracts entry per new file and the
old bytes pinned; a realms test per word, for absence, for the word
under an older label and for an unknown word; parser refusals for
`boundary` at every site kind, inside `hands`, in an agent, and for
`driver.confine`; the manifest map present exactly with `hands` and
every witness manifest valid under v9; `refuse_unboxable` once per
boundary on an empty PATH, `harness` and `open` passing; the doctor
line; `model_policy_tests.rs` — a `harness` gate on a provider shaped
like codex admitted, on one shaped like dsh refused, an `open` model
gate refused, an `open` exec gate admitted for a bundle-pinned `./`
script and refused for a `{brokkr}`-external command, an escaping and an
absolute path refused, a `/private/var` spelling and a `\`-spelled token
refused as not `./`-relative without any path being compared; the engine's `effect/started` field, the stamped record, the
store's refusal of a wrong word, and argv per boundary as pure argv
tests; the view's cell beside every model cell and its run-level fact,
an old journal's absence, a hands-less site's `not applicable` under any
engine, `costs` and `compare` naming the word, the engine's stamp
replacing a driver's, every shipped bundle compiling under `harness`
with both fragments declared, and every readout pinned roster-style; the
gate script printing *unboxed* for a harness-judged run and nothing for
a boxed one; `seatbelt` with a `sandbox-exec` on the path and
`container` with a `docker` or a `podman` still refusing naming the
slice, the engine's own entry fence writing no row, and the doctor line
saying which boundaries are offered and which are only ready; the
unboxed environment as a pure-function pin — the shipped verify gate on
a rustup machine, a secret that does not pass, the operator's own
locators that do, the marker never set; the loader's `result` and token
refusals; the pinned-script check on the shipped verifier, on an
inherited seat, on every refused spelling and on a `./` token naming no
file; and a harness gate composed with its door pointing at that seat's
real result path.
