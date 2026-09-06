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

After this slice a macOS operator runs every shipped bundle whose gates
are its own scripts — nine of the thirteen — and, once the claude
measurement lands and a decision admits the dialect step, every shipped
bundle, the review offices under their harness's own sandbox; every
readout says so either way (D19, D6).

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
   whether their tool is on PATH. The engine refuses at its own entry a
   world whose realm resolves a boundary other than the one the bundle
   was compiled under (DD3), so the pinned word and the started world
   are one.
3. **The record (ruling 3).** `effect/started` carries `boundary` beside
   `provenance`, published as `contracts/effect-boundary.v1.schema.json`
   under the contracts README's extension rule.
   `contracts/seat-record.v4.schema.json` adds `boundary` to the
   finishing checkpoint and the successful result, stamped by the engine,
   `not applicable` for a site without hands. `brokkr-view` exposes the
   boundary beside every model cell it carries and as a run-level fact;
   `inspect`, `watch`, `seats` — a thin verb, D8 — the TUI and the web
   console render it; `costs` and `compare` name it from the seat records, and `compare`'s
   `resolution` map from the view; `export` carries it as data; a run in which a gate site stood under `harness` or `open` is
   rendered *unboxed*, in the delivery gate's check summary too. Old
   journals render an explicit absence, never a default.
4. **Which boundaries may hold a gate (ruling 4).** `adapters/codex.json`
   gains `hands.harness` — `gate`, `work` and `result` — with the
   seat's result path reaching a fragment through the `{result_path}`
   token (D23); `adapters/claude.json` gains its members as the
   operator's measurement records them and declares none until then
   (D19); dsh and lanetally declare none.
   `enforce_model_policy` gains the boundary axis for sites that declare
   hands, as one hands law that runs before its class read (design
   DD22); an exec site with hands — gate or work (D32) — under `harness`
   or `open` is admitted only for
   the bundle's own pinned script — a `./` token naming a file under the
   layer that declared the seat, the layer re-walked at every unboxed
   spawn (D24) — run in the fixed environment D22 lists, the in-box
   marker never set, and the network off where the platform can say so
   — with no new verb: the engine spawns the dispatch it already spawns,
   behind a network prefix on Linux (D25, D12); a dialect validate or
   check step, which holds its gate boxed today, is refused under
   `harness` and `open` until a decision admits it (D6). At run time the argv of a site with hands
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
  default, its resolution at compile, on resume and on rerun, its
  refusal inside a bundle or an agent, the engine's fence that starts a
  run only under the word its bundle was compiled under, and its
  printout by `brokkr compile`.
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
  `watch`, `seats`, the TUI, the web console, `costs` and `compare` from
  the seat records, `export` as data, explicit absence for old journals, and the
  delivery gate's check summary.
- `gate-boundary-policy`: decision 0021's gate law under the boundary
  axis — `hands.harness` in the adapters, model gates per boundary, the
  bundle-pinned-script reading for exec sites with hands (D32) with
  its spawn-time re-walk, the dialect step's refusal at an unboxed gate, the run-time
  argv per boundary and class with the unboxed exec dispatch pinned
  token for token, and the judge's result door.
- `driver-confine-retirement`: decision 0008's container confinement
  refused by name, its engine wrapper and type deleted.
- `boundary-guides`: the prose — provider-adapters, recipe-authoring,
  quickstart, journal-and-verification, read-surfaces, repository-layout,
  driver-authoring, ARCHITECTURE, the two blueprint pages
  (extension-model, target-architecture) whose trust rows still present
  the container class, the contracts README, and the decision's
  erratum.

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
  engine's two entry fences — the unbuilt boundaries, and a world whose
  realm resolves a boundary other than the bundle's — its effect-start
  path, argv composition and record stamping, `World`'s boundary
  resolution, `manifest_diff` naming `boundary` when it is the non-file
  field that differs; `Confine` and `confined_command` deleted),
  `brokkr-protocol` (the seat prompt paragraph in both result
  deliveries; beside `hands`, the unboxed exec environment table, the
  network prefix and its probe; `DriverProcess::spawn` taking the
  environment the child starts with), `brokkr-store`
  (seat-record v4 embedded and
  dispatched), `brokkr-view` (one flattened `ModelAtBoundary` unit beside every
  model cell — participant, node, checkpoint row, journal row — the
  run-level fact, `VIEW_VERSION` 9), `brokkr-cli` (`compile`, `run`/`resume`/`rerun`,
  `doctor`, `init`, `seats` — a new thin verb, D8 — `render.rs`,
  `tui.rs`, `ui.html`, and `compare.rs` for `costs` and `compare`;
  `rerun` discovers the world, D13; `HandsCommand` gains no verb, D25).
- **Data:** `adapters/codex.json` gains `hands.harness`;
  `adapters/claude.json` gains it when the operator's measurement
  lands (D19); `realms.json` is untouched.
- **Identity:** every pinned bundle that declares hands moves once,
  because the manifest gains `boundary`; every inline gate that pins an
  adapter file that changed moves with it — codex's in this slice,
  claude's when its measurement lands. The
  witness table and the compose pins are re-pinned with 0046 as the
  stated reason. A bundle that boxes nothing and consults neither adapter
  keeps its digest.
- **Scripts and CI:** `scripts/delivered-by-brokkr.sh` renders the word;
  `.github/workflows/ci.yml` is unchanged (the job runs the base branch's
  script).
- **Docs:** the eight guides and pages named under `boundary-guides`,
  the two blueprint pages whose trust rows still present the container
  class (`docs/extension-model.md`, `docs/target-architecture.md`), the
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
- **D6 — A dialect step is refused under `harness` and `open` until a
  decision admits it** (amended in design, DD8; flipped on the third
  sitting). A dialect validate or check step holds its gate today
  boxed: decision 0042 ruling 4 says the validator is "run as a boxed
  exec step ... boxed under decision 0040 ruling 3, class gate", and
  the compiler passes a synthetic boxed exec gate through
  `enforce_model_policy` for every dialect step, so it is admitted by
  0043 ruling 3's box exactly as the shipped verify script is. Under
  `harness` or `open` no box stands, and its command — the dialect's
  argv behind `{brokkr} driver exec --` — is not the bundle's own
  pinned script, which is the one thing 0046 ruling 4 admits at an
  unboxed gate. Earlier sittings read the step as admitted on the
  argument that both readings pin a declaration and run a host tool;
  the argument holds about the bytes and widens a ruling by a design
  note, which 0042's addendum ruling 1 forbids, so it is withdrawn. The
  compiler refuses the step under `harness` and `open` naming the step,
  ruling 4, 0042 ruling 4 and a boxed boundary as the road.
  Consequence: `recipes/triage` and `recipes/night-shift`, which
  extends it — the only shipped bundles with dialect steps (D26) —
  compile under boxed boundaries only, and ruling 6's promise is unmet
  for them on this ground as well as the claude measurement's until the
  operator rules; the design records the amendment's text (DD8, Open
  questions) so the operator can accept or refuse it in a decision of
  its own number, after which the refusal is one arm to delete.
- **D7 — A judge under `harness` still delivers its result file.** The
  result file is the only channel the engine reads, and a read-only
  harness sandbox leaves nothing writable. The `gate` fragment must
  therefore be measured to leave exactly one write door, the result
  path; the candidates are named under *Measurements*. An adapter whose
  measured read-only mode leaves no door declares `gate` unsupported
  with the reason and is refused at a `harness` gate, which is the
  honest outcome rather than a judge that cannot speak.
- **D8 — `brokkr seats` is a thin verb over the seats table** (amended
  in design, DD11). No verb of that name existed; the decision and the
  commission both name it, so it exists: `brokkr seats --run <id>`
  renders the seats block `brokkr inspect` renders, from the same view,
  and `--json` emits the participants beside the run-level boundary
  fact and the view version. It derives nothing of its own. `brokkr
  export` writes the journal and its manifest as data and renders no
  prose, so its part of ruling 3 is the record itself: the exported
  journal carries the word and `verify-run` accepts it.
- **D9 — Seat-record v4 dispatches by the 0.9 line.** `engine` carries
  no position within a line, so v4's boundary is the line this slice
  lands in: an engine at or after `0.9.0` reads v4, the `0.8` line keeps
  v3, earlier engines keep v1. The tagged 0.9.0 and 0.9.1 engines wrote
  no `boundary`, which v4 admits as absent (0034 ruling 5).
- **D10 — An inline model site with hands under `harness` or `open` is
  refused at compile** (its home fixed in design, DD22). Its argv is the author's and carries the box's
  own tokens (`{hands_mcp_json}`, `{hands_args_toml}`), which no
  fragment substitution can rewrite honestly. The refusal names the
  repair: seat it through an agent, or run the realm under a boxed
  boundary. No shipped bundle has such a site.
- **D11 — `open` adds nothing.** Under `open` a site with hands runs on
  the adapter's base driver argv with the harness's own default
  permissions; Brokkr adds no fragment and serves no tool. Whatever the
  harness denies by default is the harness's fact and the guide says so.
- **D12 — The network is turned off with util-linux `unshare`, in a
  form that leaves the script the operator's own uid, no capability
  and a loopback.** `unshare --net` alone needs privilege, so the
  prefix first opens a user namespace with the engine's uid mapped to
  root (`--map-root-user`), the one mapping under which the `sh` it
  execs keeps the capability to bring the loopback up (`ip link set lo
  up`); it then execs a second `unshare --map-user=<uid>
  --map-group=<gid>` that maps root back to the engine's own ids, so
  the dispatch runs as the operator, its capabilities dropped on exec
  and the network namespace inherited. The loopback is not optional:
  the namespace box gives a script `localhost` (its `/etc/hosts` and
  bubblewrap's `--unshare-net`), and the shipped verify gate binds
  `127.0.0.1` in `crates/brokkr-cli/src/ui/tests.rs`. The second
  mapping is not optional either: a root-in-namespace script overrides
  the permission bits of every file the operator owns, and the tree's
  own permission-denial tests — a `0o000` transcript left unread in
  `crates/brokkr-protocol/src/adapters/tests.rs`, a read-only journal
  refused in `crates/brokkr-store/src/tests.rs` — would pass falsely.
  Every layer replaces itself by exec, so the PID the engine holds is
  the driver's and the deadline kill reaches it as today. The probe is
  the prefix itself around `true`, run at each unboxed exec dispatch's
  spawn in that dispatch's environment against its search path; a
  non-zero exit — no `unshare`, a kernel or AppArmor that refuses
  unprivileged user namespaces, no `ip`, a util-linux older than 2.38
  without `--map-user` — skips the prefix and the dispatch runs with
  the network on, never assumed either way. The probe's answer is not
  journaled: the boundary word is `harness` or `open` either way and
  the record says unboxed; the network namespace narrows what an
  unboxed script reaches and claims nothing the record must carry.
  The seat box that authored this change refuses nested user
  namespaces (`unshare: unshare failed: Operation not permitted`,
  measured 2026-09-06), which is the case the probe exists for. The
  prefix's exact tokens and the probe's arms are stated once, in
  gate-boundary-policy's requirement *The argv of a site with hands
  follows the boundary and the class*; this entry carries the reasons.
- **D13 — `rerun` discovers the world as `run` does** (amended in
  design, DD6). `rerun` compiles today with no world at all — no realms
  pin, no dialect — and would therefore compile every rerun under
  `namespace`: refused on a `harness` Mac for want of bubblewrap, boxed
  on a `harness` Linux while the realm says otherwise, both the
  cross-machine substitution 0046 was written to prevent. It therefore
  discovers the workspace map, compiles in the operated repository's
  realm, passes `refuse_unboxable`, and starts under that world, exactly
  as `run`; its `--db` handling is unchanged. The source run's embedded
  world is not used: `rerun` is a new run under a possibly different
  bundle with no stored linkage, so the discovered realm is its
  identity, as for `run`. A rerun's manifest consequently gains the
  realms pin and the dialect, which its tests and guide line record.
- **D14 — The extension field is absent by default, which is what makes
  it legal at event_schema 1.** The contracts README's amended rule
  (decision 0016) admits an additive payload field at `event_schema: 1`
  only when it is optional, absent by default and published as a
  numbered extension schema; a field on every `effect/started` would
  fail the second condition and be a v2 event, which the closed `type`
  enum and the frozen manifest schemas forbid. `boundary` therefore
  rides `effect/started` on terms of the same shape as `provenance`'s,
  not the same terms: `provenance` is present exactly for an attempt
  with at least one agent-resolved site, `boundary` exactly for an
  attempt with at least one site that declares hands — the shipped
  exec verify gate carries the second and not the first — and a run
  over a bundle that boxes nothing journals byte-identical payloads. This is a narrowed reading of ruling 3's *every*, and the design
  names it one (DD5): the literal reading needs a new event lineage
  outside this slice's contracts and is the operator's to ask for.
  Ruling 3's "every" is honoured where the record can afford it: the seat record carries
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
  the phase rail's nodes, on checkpoint rows and on journal rows; the
  terminal's decision trail prints `· model <x>`; `brokkr costs` and
  `brokkr compare` name a seat's model from the seat records through
  `compare::seat_costs`, outside the view; and `compare` names it once
  more from the view, in the `resolution` map that `resolution_of`
  builds per participant and `resolution_divergence` compares. The
  boundary therefore travels beside every one of them as one flattened
  `ModelAtBoundary` unit — `served: {model, boundary}` on the
  participant, the node, the checkpoint row and the journal row, so the
  wire keeps `model` and gains `boundary` beside it and a renderer
  cannot take one without the other in reach (design DD12) — read
  through one pair helper with a text face for the renderers and a JSON
  face for `compare`'s map, which carries `boundary` beside `model` and
  diverges on it as it does on `model`; and the pin test scans for a
  model cell read outside that helper rather than for a list of
  surfaces this proposal could leave short; the web console, which
  reads the flattened wire and has no Rust helper to route through,
  carries one pair helper of its own in the page and is scanned for a
  `.model` read outside it (D28). The absence word differs by
  surface for a reason: a view cell is the absent mark with a note, as
  every other cell is; a seat-costs record is JSON data and says `not
  recorded`, the way the same record says `not reported` for a model no
  driver reported.
- **D17 — The stamp rides beside the model** (amended in design, DD19).
  The schema cannot tell one checkpoint from another, so v4 admits
  `boundary` on any checkpoint, and the engine stamps by one rule: a
  record that names a `model` carries `boundary` beside it — the
  finishing checkpoint, whose `model` the driver conformance suite
  asserts for every built-in driver, the successful result, the
  engine's own member- and step-finished markers, and any per-turn
  checkpoint a driver names a model on — and a record that names none
  carries none. The trigger is the record's own key, never a step
  name. The engine's stamp replaces a driver's word and drops it from a
  model-less record: a driver never learns which boundary stood, so a
  value it wrote can only be a guess.
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
  The same holds when the measurement cannot be made at all. The seat
  that implements this change has a tool grant of `cargo` and `git` and
  no `claude` it may invoke, so the claude members stay undeclared —
  absence is the loader's fail-closed reading, refused at a `harness`
  gate or work seat naming `claude`, the member and the site — every
  shipped bundle that seats a claude-chained agent with hands refuses
  under `harness`, `bundles/self` and `recipes/panel-review` naming
  `claude` and `recipes/triage` and `recipes/night-shift` naming the
  dialect step the compiler reaches first with the claude ground behind
  it (D27), every other task is finished and committed,
  and the completion note names the measurement as the operator's with
  the recipe, the candidates and the version to measure against.
  Nothing is reported blocked for want of it: the fail-closed tree is
  the deliverable, and a block would discard the rest. The measurement
  lands afterwards as a data change to the adapter, moving the
  inline-gate pins that name its digest, with 0046 as the reason.
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
- **D22 — The unboxed exec environment is the box's own table with each
  namespace fact replaced by the fact that stands outside one, and the
  in-box marker is never set** (amended in design, DD10). Ruling 4's
  "environment cleared" takes the operator's variables — keys, tokens,
  sockets — out of a script's reach, and the box's own allow-list (0043
  ruling 1) already says what a script needs. The table is stated once,
  in gate-boundary-policy's requirement *An unboxed exec dispatch runs
  in a fixed environment*; this entry carries the reasons for each entry
  that differs from the box's. `HOME` and `TMPDIR` are private
  directories created for the attempt and never the operator's, because
  the real home carries the credential files — `.ssh`, `.netrc`,
  `.cargo/credentials.toml` — that clearing the environment was meant to
  take out of reach, and the box itself never hands it over. `PATH` is
  the engine's own, because the box's fixed `PATH` names mounts that
  exist only inside a namespace and the interpreter and the toolchain
  proxies are found where the operator's shell finds them. `USER` and
  `LOGNAME` are the engine's own where the box fixes both to `runner`:
  the box writes a passwd that names its mapped uid `runner`, so the
  name is true inside it, while the unboxed dispatch runs as the
  operator's own uid — D12's second mapping restores it — and a name
  that does not match the uid would be a false statement. The toolchain
  locators follow the site's `hands.binds` as the box's do; a bind's
  `mask` cannot be enforced outside a namespace, so under `harness` the
  shipped verify script can read `~/.cargo/credentials.toml`, which is
  the fact *unboxed* renders and the guide names. `BROKKR_HANDS_BOX` set
  outside a box would be a false statement that also switches off every
  test the constraints require to skip only inside one.
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
- **D24 — "Pinned bytes" is checked by construction, as a grammar and a
  lookup, not by comparing paths** (amended in design, DD9 and DD22). The
  compiler already expands a `./` command token against the directory
  of the layer that declared the seat, and the manifest walk of that
  layer digests every file under it, so the check reads the raw command
  before that expansion and asks one question: is the script token a
  key the declaring layer's walk pins? The grammar and the lookup are
  stated once, in gate-boundary-policy's requirement *An exec site with
  hands under harness or open holds only for pinned bytes*; this entry
  carries the
  reason. A canonicalising comparison needs the file to exist on both
  sides, behaves differently per platform, and proves less than the
  walk's own key set; so nothing is canonicalised and no two spellings
  are compared, which is how the rule is platform-neutral — a
  `/private/var` or `C:\` spelling is refused as not `./`-relative,
  never compared, and `bash -c '…'` is refused because an option token
  precedes the script. The verdict is re-derived where it matters: at
  every unboxed exec dispatch spawn the engine re-walks the declaring
  layer against the identity the manifest pins and refuses on a moved,
  missing or added byte (DD9), because the unboxed gate is admitted for
  its bytes where the boxed one is admitted for its walls, and the
  implement seat writes the tree the layer lives in.
- **D25 — No new verb: the engine spawns the dispatch it already
  spawns, in a fixed environment, behind a prefix.** The clarify seat
  found the unboxed exec wrapper unnamed and its argv unpinned. Ruled:
  `HandsCommand` gains no verb. `brokkr hands exec` exists because a
  namespace must be built by a process that then execs into it; under
  `harness` and `open` nothing is built, so a wrapper verb would be a
  second process whose only work is `env_clear()`, which
  `DriverProcess::spawn` can do where the child is already spawned. It
  therefore takes the environment the child starts with — the engine's
  own, today's behaviour and every model site's under every boundary,
  or exactly the composed table of D22 — and the network prefix of D12
  is argv the engine composes. The exec dispatch under `harness` and
  `open` is the compiled command itself behind that prefix: `{brokkr}`
  and `./` were expanded at compile by `expand_command` against the
  declaring layer's directory, and `{prompt_file}` stays literal
  because the exec driver stages the prompt and expands it. The argv is
  one pure function of the dispatch, the probe's answer and the
  engine's ids, pinned token for token in gate-boundary-policy for the
  shipped verify seat with the probe passing and failing; a dialect step
  is refused before it could take the road (D6). No guide row is added because no verb is;
  driver-authoring.md's sentence about `brokkr hands exec` is
  qualified instead.
- **D26 — The bundles named are the ones that exist.** `recipes/sdd`
  was folded into `recipes/triage`'s strategy select by #176 (881c4e4)
  and no longer exists. The chief architect is seated by
  `recipes/triage`'s specify and design steps and, through inheritance,
  by `recipes/night-shift`, which overrides only its implement seat;
  `agents/intake-sdd.json` is hired by no shipped bundle. The
  measured-gap scenario and D6 name those two bundles, and the
    obligation stays the one the requirement above them states — every
  bundle under `recipes/` and `bundles/` — so no scenario names a
  bundle no test can compile.
- **D27 — The shipped pin names the ground the compiler reaches first,
  per bundle** (returned from analyze, 2026-09-06). An earlier draft
  said the agent link of every sequence precedes its dialect step, so
  that `claude` would be the first refusal the compiler reaches for
  all four bundles that refuse under `harness` against the shipped
  adapters. That is false for two of them: the compiler walks a
  bundle's seats as a `serde_json::Map`, a `BTreeMap` in this tree
  (`preserve_order` is off), so phases compile in name order and
  `analyze` is first; `recipes/triage`'s `analyze` and `clarify`
  sequences are `[check (dialect), judge (agent)]` and the openspec
  dialect declares an `analyze` check, so the dialect step's refusal
  (D6) is reached before any claude link, in triage and in
  `recipes/night-shift`, which inherits the seat. The scenario and the
  task therefore pin `bundles/self` (at `review`) and
  `recipes/panel-review` (at `review:correctness`, panel members
  compiling in name order too) as refusing naming `claude`, the member
  and the site, and `recipes/triage` and `recipes/night-shift` as
  refusing naming `analyze:check`, with the claude ground behind it
  unreached; the "precedes" reasoning is deleted rather than
  qualified. Reason: a pin's value is that it names why each bundle
  refuses today, and it moves for two known reasons — the measurement
  landing, and a decision admitting the dialect step — both of which
  the record now states.
- **D28 — The web console has a page-side pair helper, and the pin
  scans the page for it** (returned from analyze, 2026-09-06). The pin
  rule "`served.model` read outside the pair helper" cannot reach
  `ui.html`: the page reads `item.model`, `part.model` and `e.model`
  off the flattened wire, which carries `model` and `boundary` as
  siblings and no `served`, and there is no Rust helper to route
  through, so as written the console either always failed the pin or
  silently escaped it. Ruled: the page carries one pair helper of its
  own — a single script function taking a carrier and returning the
  two cells, the only place in the page that names `.model` — and the
  pin test scans `ui.html` as it scans the Rust sources, failing and
  naming the line where `.model` is read outside that function, the
  way `agent_readouts.rs` already scans the page for a composed
  provenance sentence; the console's own rendering tests prove the
  cells land in one row. Reason: one rule, held in each language by
  its one helper, is what makes the pin a pin; excluding the page from
  the scan would leave the one surface without a Rust type to a test
  that proves today's rendering and nothing about tomorrow's edit.
- **D29 — Ruling 4's binding is pinned against the shipped adapters,
  not a fixture shaped like them** (returned from analyze,
  2026-09-06). Ruling 4 names `model_policy_tests.rs` and says "a
  `harness` gate on codex admitted, on dsh refused"; an earlier draft
  of the verification obligations softened that to "a provider shaped
  like codex", and no test then compiled the shipped
  `adapters/codex.json` at a `harness` gate — every shipped hands agent
  chains `opus`, so the shipped-bundle pin exercises only claude
  refusals, the scratch-library test plants the members into a copy,
  and the data test only loads the file. Ruled: one fixture gate agent
  whose agent file declares hands and whose chain is `astra` alone
  compiles under `harness` against the shipped adapter library in
  `model_policy_tests.rs` and is admitted, its manifest pinning codex's
  digest; the same fixture chaining one dsh model alone, and one
  lanetally model alone, is refused under `harness` by the hands law,
  naming the link, the provider and the missing `hands.harness.gate`,
  in the same file — not with its `namespace` refusal, which is 0021's
  trust-tier text and comes second under `harness` (D33). The
  chain arms stay on fixture providers, as that file's charter
  requires. Reason: the decision binds by name, and a test against the
  file it names is the only one that moves when that file does.
- **D30 — The blueprint pages follow the guides** (returned from
  analyze, 2026-09-06). The sweep of pages describing the container
  trust class named driver-authoring, recipe-authoring and
  ARCHITECTURE and missed two: `docs/extension-model.md`'s seat-field
  table, whose `trust` row says the tier "decides what the engine
  mounts into the sandbox", and `docs/target-architecture.md`'s runner
  table, whose `policy-confined` row is an OCI container with a pinned
  digest. Both are living blueprints — target-architecture carries a
  name note dated 2026-09-02 — so both are amended rather than left as
  they are: the `trust` row says the wall is the realm's `boundary`
  (0046) and the tier decides what is mounted inside it; the runner
  rows point at 0046's `container` boundary, declared by the realm and
  refused at start until slice (iii) measures it. No row or section is
  removed, and the pages' status lines are untouched. Reason: a reader
  who reaches either page from ARCHITECTURE would otherwise find the
  retired wrapper presented as a working runner.

- **D31 — The harness paragraph is prose for a model, and a script reads
  none** (operator ruled in chat, 2026-09-06, after the clarify loop
  parked). The seat input of every site with hands carries `boundary`
  and, when Brokkr builds the box, `hands: boxed`, whatever the site's
  driver; but the rendered prompt's boundary paragraph — today's
  workspace-tool sentence under a boxed boundary, the harness paragraph
  and the `open` paragraph — is rendered for model-backed sites only.
  An exec site's prompt carries no hands paragraph under any boundary:
  its script reads the environment the fixed-environment requirement
  composes, and "the result path is the one file that sandbox lets it
  write" is false of a script. Reason: the clarifier's first open
  question — exec gates are exactly the sites that run under `harness`
  in this slice and the prescribed wording was false for them.

- **D32 — Class decides only whether a site may hold a gate; every
  unboxed exec dispatch is judged and run on one ground** (operator
  ruled in chat, 2026-09-06). A work-class exec site that declares hands
  is legal today and boxed; under `harness` and `open` it is admitted
  on exactly the pinned-bytes terms an exec gate is admitted on, refused
  on the same terms and naming the same decisions, and when admitted it
  is spawned in the fixed environment, behind the network prefix, and
  re-walked at spawn exactly as a gate is. The pinned-bytes requirement
  therefore reads "an exec site that declares hands", the argv
  requirement's "an admitted exec gate's" reads "an admitted exec
  site's", and nothing about an unboxed exec dispatch is left to the
  smith. Reason: the clarifier's second open question — the rules were
  gate-scoped in two places and unscoped in a third, and the literal
  coverage gate cannot absorb a branch nobody ruled.

- **D33 — The hands law is the first statement of the gate policy, and
  a harness refusal names the fragment** (operator ruled in chat,
  2026-09-06, after the analyze loop parked). DD22 placed the hands law
  before the class early-return and before decision 0021 ruling 2's
  trust-tier refusal, to close the work-class panic; D29 and its
  scenario still said a dsh or lanetally chain under `harness` is
  refused "word for word" as under `namespace`, which that order makes
  impossible. Ruled: the order is DD22's; the `harness` refusal of a
  chain whose link declares no `hands.harness.gate` names the link, the
  provider and the missing fragment; the `namespace` refusal stays
  0021's; no scenario or task asserts the two texts equal. Reason: the
  reverse order reopens the panic, and a refusal that names the missing
  fragment is the more legible of the two.

- **D34 — Two residuals accepted by the operator, for Muninn** (ruled
  in chat, 2026-09-06). The analyst's third visit raised two low
  findings the operator accepted as residuals rather than return the
  change a fourth time: (a) the delivery gate's check summary leaves one
  state unstated — a manifest carrying both `hands` and `boundary`
  keys and a journal with no `effect/started.boundary` entry yet, where
  the view renders "no boundary recorded" and the script appends
  nothing; unreachable for a completed anchored run over a shipped
  bundle, since each runs a hands gate before `done`. (b) The gate-law
  requirement does not itself state the arm D11, DD22 and task 6.10
  carry — a work-class chain site with hands under `open` is admitted
  asking no fragment of any link. Both are recorded here as debt
  Muninn patrols (decision 0020), and neither is a defect the smith
  should read as licence: the script and the arm are built as the
  design and the tasks say.

## Measurements the implementing seat cannot make

Two facts must be measured on the operator's machine and recorded,
never guessed (0046 ruling 4's own words). Neither is the implementing
seat's to make: its tool grant is `cargo` and `git`, and neither
`claude` nor `codex` is a command it may run. Until each is measured
the tree carries the honest state named below, the implementation
finishes every other task and commits, and its completion note lists
both measurements as the operator's, with the recipe.

- **claude's `hands.harness.gate` and `hands.harness.work`.** The
  installed claude's `--permission-mode` choices are `acceptEdits`,
  `auto`, `bypassPermissions`, `manual`, `dontAsk`, `plan` — there is no
  `read-only` value. Candidates for `gate`: `--permission-mode dontAsk`
  with `--allowedTools` naming the read tools and an edit rule scoped to
  `{result_path}` (`result` `file`), `--permission-mode plan` if it can
  still write the result file, and the `--restricted` /
  `--permission-prompts none` pair the robustness seat reports on
  2.1.263, unconfirmed. Candidates for `work`: `--permission-mode
  acceptEdits` with the shell allowed, or the harness's own sandbox
  settings with the shell auto-allowed when sandboxed — a bare
  `acceptEdits` prompts for every shell call, and a non-interactive seat
  answers a prompt with a denial, so the work fragment must be measured
  to leave the chief its `cargo` and `git`. The recipe: under each
  candidate, run one gate seat whose prompt asks it to read a file
  outside the worktree, write one inside it and deliver its result — the
  mode passes as `gate` when only the result file lands; run one work
  seat that must run `cargo test` and commit — the mode passes as `work`
  when both succeed with no prompt. Until the measurement is recorded,
  `adapters/claude.json` declares no `hands.harness` member: absence is
  the loader's fail-closed reading, and the provider-adapters guide says
  the members are undeclared pending the operator's measurement. The
  binary that guide's doctor transcript already records is claude
  2.1.251, so the measurement is against that line; for `work` the empty
  fragment is a candidate answer only if the measurement shows the
  driver's own `--permission-mode acceptEdits` grants the shell. The
  guide records the version each was measured against and what it
  denies and allows.
- **codex's result door under `--sandbox read-only`.** The fragment
  `--sandbox read-only` is ruling 4's own word for codex, and
  `--output-last-message` is the flag `codex exec --help` documents and
  the codex driver already admits on a resume, so the declaration is
  from the decision and the tool's own record and is not a guess (D23).
  The fact still unmeasured is whether the capture lands while the
  sandbox class is `read-only`: the guide records the door as declared
  from the tool's record with that measurement pending and the
  operator's — the same recipe, one gate seat under the fragment
  delivering its result and nothing else — and if the measurement shows
  the capture does not land, `gate` becomes unsupported with that
  reason.

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
  tier, or the result contract.
- Reading a rerun's world from the source run's embedded pin: `rerun`
  discovers the workspace map as `run` does (D13, DD6), because the
  source run's world is not the new run's identity.
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
line; `model_policy_tests.rs` — a `harness` gate on the shipped codex
adapter admitted and on the shipped dsh and lanetally adapters refused
as at a boxed gate, each compiled against the shipped adapter library,
the chain arms on fixture providers (D29), an `open` model
gate refused, an `open` exec gate admitted for a bundle-pinned `./`
script and refused for a `{brokkr}`-external command, an escaping and an
absolute path refused, a `/private/var` spelling and a `\`-spelled token
refused as not `./`-relative without any path being compared; the engine's `effect/started` field, the stamped record, the
store's refusal of a wrong word, and argv per boundary as pure argv
tests — the unboxed exec dispatch pinned token for token with the
network probe passing and failing, the probe's own arms on a planted
search path, and `DriverProcess::spawn`'s two environments; the view's cell beside every model cell and its run-level fact,
an old journal's absence, a hands-less site's `not applicable` under any
engine, `costs` and `compare` naming the word, the engine's stamp
replacing a driver's, every shipped bundle without a dialect step compiling under `harness`
in a scratch copy of the adapter library with both `hands.harness`
members planted as fragments while the two with dialect steps refuse
naming the step, and, against the shipped adapters as they stand,
exactly four bundles refusing on the ground the compiler reaches first
— two naming `claude`, two naming their `analyze` check step (D27) —
until the measurement lands, `compare`'s `resolution`
map carrying the pair and diverging on it, every readout pinned
roster-style with the console's page scanned for a `.model` read
outside its own pair helper (D28), and the two blueprint pages' trust
rows pointing at the boundary (D30); the
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
file; the layer re-walk at spawn on an untouched, an edited and an
inherited layer, and a dialect step refused under `harness`; a harness gate composed with its door pointing at that seat's real
result path; and, from the design: `brokkr seats` rendering the block
`inspect` prints and its `--json` shape; `rerun` compiling and starting
under the discovered realm and refusing under a word the machine
cannot build; the engine's entry fence refusing a world whose boundary
differs from the bundle's before `run/started`; the private home with
planted secrets a spawned script cannot read and the locators set from
the site's binds; `bash -c` and a `./dialects/` token refused; and an
entry outside the vocabulary rendered *not recorded* by the view and
the gate script.
