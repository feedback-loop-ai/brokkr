# Design: The boundary is named — decision 0046, enactment slice (i)

The chief's synthesis of the council's two positions (simplicity,
robustness) for the change `boundary-named-slice-i`. The proposal's
`## Decisions` (D1–D26) are the specify seat's readings of the decision;
this document rules on the design that enacts them, records where the
two positions were adopted, rejected or combined and on what evidence,
and amends five of the proposal's decisions (D6, D8, D13, D22, D24, each
marked *amended in design* there) so that the proposal, the deltas and
this design say one thing.

## Context

What the tree holds, read for this design (paths are the evidence a
judge can re-read):

- **The world is pinned at start, the bundle is compiled before.**
  `Engine::start_in_world` (`crates/brokkr-runtime/src/engine.rs`)
  takes a compiled `Bundle` and an `Option<World>`, pins the world into
  the manifest with `World::pinned` and journals it in `run/started`.
  The bundle's manifest — where the v9 `boundary` map lives — is
  computed at compile by `manifest_for`. Two parties can therefore
  disagree about the boundary unless something compares them.
- **The realms pin already embeds `realms.json` verbatim.** `World::pin`
  writes `{"source", "sha256", "map": content}`, and `manifest_digest`
  hashes the whole manifest. A realm that declares a boundary already
  moves every digest that pins it; the v9 map is legibility, not the
  identity mechanism. It is still ruled (ruling 1) and still built.
- **The boundary is realm-wide.** Ruling 1: declared by the realm, a
  bundle never names it; issue #214's per-site relaxation is unruled.
  Every hands site of one run stands under one word.
- **Dialect steps are outside the gate law today.** A validate or check
  step compiles as `StepBody::Dialect`, class gate, and the compiler's
  step loop calls `enforce_model_policy` for singles only
  (`crates/brokkr-runtime/src/bundle.rs`, the `StepBody::Dialect { .. }
  => {}` arm). Its hands are a synthetic spec the compiler records
  (`record_hands(&dialect_site, &synthetic, ..)`), and the engine
  composes it through `hands_command` like any boxed exec (the dialect
  arm of the sequence executor in `engine.rs`). Decision 0042 ruling 1
  is what admits it: the argv is the dialect's own, pinned by the
  dialect's content digest.
- **The box has a private home, and the toolchain enters only by
  declared binds.** `crates/brokkr-protocol/src/hands.rs` binds a
  per-session private directory at `HOME`, a private `/tmp`, sets
  `CARGO_HOME`, `RUSTUP_HOME` and `NPM_CONFIG_CACHE` exactly when the
  site's `hands.binds` name `~/.cargo`, `~/.rustup`, `~/.npm`, and hides
  each bind's `mask` entries behind `/dev/null`. `bundles/self`'s verify
  seat binds `~/.cargo` as an overlay masking `credentials.toml` and
  `credentials`, and `~/.rustup` read-only.
- **The manifest walk pins bytes through symlinks and skips two
  names.** `manifest_for` walks with `is_file()` and `fs::read`, both of
  which follow a symlink, so a link's target bytes are pinned under the
  link's key; it skips a top-level `realms.json` and `dialects/` and
  refuses `secrets.env`. An ancestor layer is pinned by the same walk
  over its own directory (`bundle/compose.rs`, the ancestor loop).
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
- **`verify-run` is parse, chain, fold.** The store's `verify_export`
  makes the three checks the import gate makes; seat records are
  validated at append by the store's fence (0034 ruling 6); payload
  extension fields (`provenance`, `head`) are validated nowhere.
- **The view derives per site from `effect/started.provenance`,** keyed
  by the `member` tag (`crates/brokkr-view/src/lib.rs`, the
  `EffectStarted` arm of the scan), and carries a `model` cell on
  `Participant`, `Node`, `CheckpointRow` and `JournalRow`;
  `VIEW_VERSION` is 8. The terminal seats table prints `model` and
  `effort`; the trail prints `· model <x>`; the TUI and the console read
  the same cells; `costs` and `compare` read `model` off the seat
  records themselves, and `compare`'s `resolution_of` reads each
  participant's view-derived model a second time into the `resolution`
  map that `resolution_divergence` compares.
- **The delivery gate composes three lines** after verifying the
  journal: the tier line, the vouched line and the docs-tier preflight
  line (`scripts/delivered-by-brokkr.sh`).
- **What this seat's box holds:** `unshare` (util-linux 2.41), `bwrap`,
  `docker`, `jq`, `openspec` 1.12.0; no `claude`, no `codex`, no
  `cargo`. The claude measurement is therefore not this seat's to make
  (see Open questions). The robustness seat reports Claude Code 2.1.263
  advertising `--restricted` and `--permission-prompts none`; this seat
  could not confirm it and records it as a candidate only.

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
   runs every shipped bundle — conditional on the claude measurement
   (D19) — and every readout says *unboxed*.

## Non-goals

- Building `seatbelt` or `container` (slices ii and iii); anything that
  reads their profiles, images or mounts.
- A per-site boundary (#214), a `--boundary` flag, an environment
  override or a test hook that sets the word outside the realm map.
- Changing what `hands` allows (`network`, `binds`), any driver's trust
  tier, the result contract, the event vocabulary, or `EVENT_SCHEMA`.
- Ruling the three questions 0046 leaves open (a `harness` vouch and
  `by-hand`, the container image pin, seatbelt and the hooks path).
- Editing any frozen contract, `policy/phase-machine.json`, `reference/`
  or the evaluator corpus; adding `boundary` to this repository's
  `realms.json`.

## The council, reconciled

Each row names the claim, what the evidence says, and the ruling. Where
a position is rejected, the reason is here and nowhere else.

| Claim | Simplicity | Robustness | Ruling and evidence |
|---|---|---|---|
| Where the type lives | one closed enum beside `Realm` in `brokkr-core`, no new module | a closed enum in `brokkr-core`, never free strings between crates | **Adopted, both.** `Boundary` beside `Realm`, five variants, parse/display pair; the record's sentinel is `Option<Boundary>` serialised as `not applicable` by one helper, so the realms enum cannot admit the sentinel and no second vocabulary exists (DD1). |
| A `BoundaryPlan` / per-site execution plan | a trait or plan is dead branches for two boundaries that only refuse | a typed plan carrying class, layer, capability and script identity, parsed once | **Combined.** No trait, no plan object: the facts robustness wants already live on the compiled bundle (class on seats, members and steps; the declaring layer where `expand_command` resolves `./`; hands per site) and the pinned-script admission is a compile-time verdict the manifest pins. Three exhaustive `match`es over the enum; `seatbelt`/`container` never reach composition (DD2, D21). |
| Resolution point | once at compile; the engine reads one memoised field, never disk | compile must not probe PATH; execution must never reread `realms.json` or adapters; a fence at the engine entry | **Adopted, both.** `Bundle.boundary` is set by `compile_with_realm`; `Engine.boundary` is that field; `start_in_world` refuses before `create_run` when the world's realm resolves a different word than the bundle was compiled under (DD3). |
| The v9 map is the identity mechanism | no — the realms pin already moves the digest; the map is legibility | the map must equal the hands keys and the plan; a verifier must assert it | **Combined.** The map is derived in the loop that writes `hands`, so the key sets are equal by construction and the schema states co-presence with `dependencies`; a separate `verify-run` rule is rejected — `verify-run` is parse/chain/fold and a forged map is a forged digest (DD4). |
| Journaling the site's class | cut it; derive *unboxed* from `run/started` with one `jq` | keep a per-site structured extension with an engine-owned gate class | **Robustness adopted.** Ruling 3 says *gate* site; the manifest names hands sites but no class; cutting the flag either widens the rule to every boxed site or makes every consumer recompile the bundle. The cost is one boolean written in the traversal that already writes `provenance`, so the tags cannot drift (DD5). |
| `effect/started` carries nothing the manifest lacks | accepted as ruled; one field, not two | the omission for a plain attempt is a narrowed reading of "every" and must be called one | **Both adopted.** The word is redundant with the manifest and the gate class is what the entry adds; the field rides terms of `provenance`'s shape — present exactly when a site of the attempt declares hands, as `provenance` is present exactly when one is agent-resolved — and D14 now says *narrowed reading* in those words (DD5). |
| Rerun | — | reject D13: reading no world silently substitutes `namespace` | **Robustness adopted, differently.** `rerun` is a new run under a possibly different bundle ("no stored linkage"), so the source run's world is not its identity; the discovered realm is, exactly as for `run`. D13 is amended: `rerun` discovers the world, compiles in it and starts in it (DD6). |
| `work` fragment under `harness` (D5) | if no measured claude combination works, declare none and be refused | reject the writable `work` member as unruled; refuse work sites with hands under `harness` | **D5 stands.** Ruling 1 defines `harness` as "the harness's own sandbox as its adapter fragment addresses it"; ruling 4 governs gates only; D11 already admits a work seat under `open` at the harness's default, so refusing the same seat under the stricter word would invert the vocabulary. Absence stays fail-closed at compile (DD7). |
| Dialect steps under `harness`/`open` (D6) | — | reject: an external binary is not the bundle's pinned bytes | **D6 stands, with the reason sharpened.** The dialect step is admitted today outside `enforce_model_policy` by 0042 ruling 1; the objection that the tool's bytes are unpinned applies equally to `bash` and `cargo` under the shipped verify script, so it proves too much. The proposal's "weaker reading" wording is withdrawn: both readings pin a declaration and run a host tool (DD8). |
| Pinned-script check | one pure predicate over argv and roots, canonical-safe | a real parser with an interpreter grammar, canonical containment, symlink defence, digest recheck before spawn | **Combined.** Construction, not comparison (D24), plus robustness's grammar: bare interpreter names only before the script, no option token, one script token, the rest arguments; and the walk's own exclusions shared so `./realms.json` and `./dialects/…` are refused as unpinned. Canonicalisation rejected: nothing is compared, and the walk pins through symlinks so following one is the digest's own reading. The pre-spawn digest recheck is rejected: the namespace box has none either, and resume's `manifest_diff` is the drift fence at the durable boundary (DD9). |
| The unboxed environment | `env_clear()` plus `PATH`, `HOME` and the engine's own entries, one helper | never hand the real `HOME` or caches; private home; plant-a-secret test | **Robustness adopted, by mirroring the box.** The box already answers this: private `HOME` and `TMPDIR`, toolchain locators exactly when the site's binds declare them. D22 is amended to that principle — the box's table with each namespace fact replaced by the fact that stands outside one, `USER` and `LOGNAME` included — the table itself stated once, in the delta; one pure function; the secret-planting test is release-blocking (DD10). |
| `brokkr seats` | — | a thin verb rather than a silent reinterpretation | **Robustness adopted.** The decision and the commission both name the verb; a thin `seats` over the view's seats block makes the named contract true at the cost of one arm. D8 is amended (DD11). |
| Readouts | `Participant.boundary` and one run-level fact; let the pin test derive the rest | a composite model-and-boundary unit on every carrier so omission is impossible by type | **Combined.** The four carriers are needed as wire fields ("in the same row" is a promise to `--json` readers too), so the derivation cannot be short of them; one flattened `ModelAtBoundary` unit and one pair helper — a text face for the renderers, a JSON face for `compare`'s `resolution` map, which reads the participant's model a second time — give the type lock, and the roster-style pin test stays as ruling 3's binding (DD12). |
| Old journals | absence via `cell_of` | absence, and never `not applicable` inferred from the manifest | **Combined.** A boxed site with no recorded word renders the absent mark with `no boundary recorded`; a site the pinned manifest names no hands for renders `not applicable` with the note `no hands declared` — a derivation from journaled evidence, not a default (DD13). |
| Strict reading of the extension | — | a shared strict parser before any consumer; malformed evidence must not look boxed | **Combined.** No new validator: the engine writes from the closed type. Every consumer treats an entry outside the vocabulary or without a tag as *not recorded*, never as boxed or unboxed (DD14). |
| Network isolation | best-effort, one probe, injectable | best-effort; never presented as evidence | **Adopted, both** (D12/D25 as clarify returned them); plus the guide constraint that no readout or page states the network was off (DD15). |
| Landing order | delete `Confine` first; re-pin once at the end | — | **Adopted** (DD16). |
| Windows shell gates | — | the shipped `sh` gates parse a Unix result path | **Recorded as an open question**, not built: ruling 6 names Windows, nothing here measures it. |

## Decisions

### DD1 — One `Boundary` type beside `Realm`; the sentinel is an `Option`

`brokkr_core::realms::Boundary { Namespace, Seatbelt, Container,
Harness, Open }` with `FromStr`/`Display` on the five words, a
`Default` of `Namespace`, and `Realm.boundary: Option<Boundary>` read
by the `forge.realms/v4` loader (refused under v1–v3 by name, as
`house` and `dialect` are held to theirs). `Realm::boundary()` returns
the resolved word. Every crate uses the type; no boundary crosses a
crate boundary as a string. The seat record's `not applicable` is
`Option<Boundary>` through one serde helper (`None` ↔ `"not
applicable"`), so the realms enum cannot admit the sentinel and the
schema enums are the same five words, plus the sentinel only where the
record admits it.

*Alternatives:* a `boundary.rs` module (rejected: five words and a
default do not need a directory entry); a six-variant enum with
`NotApplicable` (rejected: it would let a realm declare it).

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
`hands.harness` on the chain link.

*Alternative:* a `BoundaryProvider` trait with five implementations
(rejected: two of them exist only to return an error until slices ii
and iii, which is two sets of branches the coverage gate cannot reach
honestly).

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
boundary its bundle was compiled under*, with its own scenarios. `resume` needs no second fence: `manifest_diff`
names `boundary` when the map differs. The Looper lineage refuses a
manifest carrying `hands` or `boundary` as it refuses every key beyond
its six.

*Alternative:* deriving the engine's word from `World::from_manifest`
at every use (rejected: a second lookup that can disagree with the
compiled bundle; the fence makes the two one).

### DD4 — The v9 map is derived, co-present by schema, equal by construction

`manifest_for` writes `boundary` in the loop that writes `hands`, one
entry per key, every value the bundle's word. `run-manifest.v9`
declares `dependencies` in both directions. No verifier rule asserts
key equality: the two maps come from one loop, and an exported manifest
that disagreed with itself would have a different digest than the one
its journal chain pins.

*Alternative:* a `verify-run` structural rule (rejected: `verify-run` is
parse, chain and fold by design, and the check would guard a state the
compiler cannot produce).

### DD5 — `effect/started.boundary` mirrors `provenance` and adds the class

The engine writes `boundary` as a list keyed by the same `member` tags
`provenance` uses, one entry per invocation site of the attempt, each
`{member, boundary, gate}` — the realm's word for a site with hands,
`not applicable` for one without, `gate` the site's class — constructed
from the same `invocation_sites(body)` traversal as `provenance`, so the
tag sets cannot drift. It is present exactly when at least one site of
the attempt declares hands; an attempt over a bundle that boxes nothing
journals byte-identical payloads. This is a *narrowed reading* of
ruling 3's "every `effect/started`": the contracts README admits an
extension field at `event_schema: 1` only when it is optional and
absent by default, and a field on every start would be a v2 event
outside this slice's contract list. Ruling 3's "every" is carried
literally by the seat record instead (every finishing checkpoint and
successful result). If the operator wants the literal reading, that is
a new event lineage and a return to the operator, not a wider field.
Published as `contracts/effect-boundary.v1.schema.json`; `fold` never
reads it.

*Alternative (simplicity):* no `gate` flag, derive from `run/started`
(rejected: the manifest carries no class; the derivation would either
widen ruling 3 to every boxed site or require the bundle).

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

*Alternative (robustness):* compile from the source run's embedded world
(rejected: `rerun` is "a NEW run under another bundle or recipe, no
stored linkage"; the source's world is not the new run's identity, and
the bundle may differ).

### DD7 — `hands.harness.work` stands (D5), fail-closed

The adapter's `hands.harness` object carries `gate`, `work` and
`result` as the delta reads. A work-class site with hands under
`harness` whose resolved chain has a link without a `work` fragment is
refused at compile naming the link. Reason the objection is refuted:
ruling 1's table defines `harness` as the harness's own sandbox as the
adapter fragment addresses it; ruling 4 constrains gates; under `open`
(D11) the same work seat already runs at the harness's default, so
refusing it under `harness` would make the stricter word the one that
runs less. What a `work` fragment allows is the harness's fact and the
record renders the run *unboxed*, which is the statement ruling 3 asks
for.

### DD8 — Dialect steps are the same reading, not a weaker one (amends D6)

A dialect validate or check step is admitted under `harness` and `open`
on the pinned-script terms (the fixed environment, the network prefix
where the probe passes, the run rendered *unboxed*). It is not an
exception carved by this slice: the step is admitted today outside
`enforce_model_policy` by 0042 ruling 1, its argv pinned by the
dialect's content digest and the tool's name and version. The
proposal's characterisation of this as "the weaker of the two readings"
is withdrawn: a `./` script's bytes are pinned and the `bash` and
`cargo` it runs are not; the dialect file's bytes are pinned and the
`openspec` it names is not. Both pin a declaration and run a host tool.
The scenario "A dialect step under harness is admitted" remains the one
line the operator deletes to refuse it, in which case `recipes/triage`
and `recipes/night-shift` compile only under a boxed boundary.

### DD9 — The pinned-script check is a grammar and a lookup (amends D24)

The compiler judges an exec hands gate under `harness` or `open` on the
*raw* command, before `expand_command` erases the `./` spelling, and
asks one question: is the script token a key the declaring layer's
manifest walk pins? The grammar (bare interpreter names before exactly
one script token, then unjudged arguments) and the lookup (plain `./`
components, a regular file by `metadata` under the declaring layer's
directory, not a key the walk skips) are stated once, in
gate-boundary-policy's requirement *An exec gate under harness or open
holds only for pinned bytes*; the tasks cite it and restate nothing.
What this design fixes is the shape: the walk's skip rule is one
function shared with `manifest_for`, so the two cannot drift; the
lookup follows a symlink because the walk pins through one (`is_file()`
and `fs::read` both follow), so the bytes pinned under the key are the
bytes that run; the declaring layer's directory is the one
`expand_command`'s caller already holds; and nothing is canonicalised
and no two spellings are compared, which is how `/private/var` and `\`
are handled — a token spelled either way is refused as not
`./`-relative. The verdict is a compile fact the manifest pins; at run
time the engine spawns the compiled command (D25) and re-derives
nothing.

*Alternatives:* canonical containment (rejected: needs both sides to
exist, behaves differently per platform, and proves less than the
walk's own key set); a pre-spawn digest recheck (rejected: the
namespace box has none, and `manifest_diff` at resume is the drift
fence the tree already draws).

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
seat declares that bind.

*Alternative (proposal D22 as returned from clarify):* inherit `HOME`
and the locators verbatim (rejected: the real home carries credential
files the environment clearing was meant to take away, and the box
itself never hands it over).

### DD11 — `brokkr seats` is a thin verb (amends D8)

`brokkr seats --run <id> [--db] [--json]` renders the seats block the
`inspect` renderer already produces, from the same `RunView`, and with
`--json` emits `{view_version, boundary, participants}`. No derivation
of its own; one arm, one test, one row in the read-surfaces guide.
Reason: the decision and the commission both name it, and a verb that
exists is a truer reading than a table that answers to the name.

### DD12 — One `ModelAtBoundary` unit, flattened, on every model-bearing carrier

`brokkr_view::ModelAtBoundary { model: Cell, boundary: Cell }` replaces
the bare `model: Cell` on `Participant`, `Node`, `CheckpointRow` and
`JournalRow` as a `#[serde(flatten)]` field named `served`, so the wire
keeps `model` and gains `boundary` as siblings (`VIEW_VERSION` 9) while
a Rust renderer cannot take the model without the boundary in reach.
One pair helper prints the pair, with a JSON face beside its text face; the terminal seats table gains a
`boundary` column beside `model`, the trail prints `· model <x> ·
boundary <y>`, the TUI and the console read the two cells. `RunView`
gains `boundary: RunBoundary { word: Cell, unboxed: bool, text: String
}`, derived once: `unboxed` is true exactly when a valid
`effect/started.boundary` entry has `gate` true and `harness` or
`open`; every run header prints `text`. `costs` and `compare` reduce
`boundary` off the seat records as they reduce `model`, `not recorded`
when no record carries one, and `compare` reports a difference as a
divergence; `compare`'s `resolution` map, which reads each
participant's view-derived model a second time, carries the pair
through the helper's JSON face and `resolution_divergence` diverges on
`boundary` as on `model`. The roster-style pin test reads every readout
source and fails where `served.model` is read outside the pair helper
— its text face for the renderers, its JSON face for `compare` — or
where a seat-costs record's `model` key is rendered without
`boundary`.

### DD13 — Absence and `not applicable` in old journals

For a journal with no `boundary` anywhere: a participant whose site the
pinned manifest's `hands` names renders the absent mark with the note
`no boundary recorded`; one it does not name renders `not applicable`
with the note `no hands declared`. The second is a derivation from
journaled evidence (the manifest rides `run/started`), not a default,
and the note says which. No surface prints `namespace` for an old
journal; the run-level fact is absent with the same note when any
boxed site lacks a word.

### DD14 — Strict reading without a new validator

The engine writes entries from the closed type, so a malformed entry is
unrepresentable for a journal this engine wrote. For any journal, every
consumer — the view's derivation, `costs`/`compare`, the gate script —
treats an entry whose word is outside the six (five words and the
sentinel) or which lacks a `member` tag as *not recorded* for that
site, never as boxed or unboxed; the gate script prints `· boundary not
recorded` for such a run. No append-time validator is added for the
extension: seat records are validated at append because 0034 rules it;
extension fields never have been, and `verify-run` stays parse, chain
and fold.

### DD15 — The network narrowing is never evidence

D12 and D25 stand as clarify returned them: the prefix is composed
argv, the probe is the prefix around `true` in the dispatch's
environment, the answer is not journaled. Added constraint: no readout,
guide row or check summary states that the network was off; the guides
describe the prefix as a narrowing the engine attempts on Linux and the
record as saying *unboxed* either way.

### DD16 — Landing order

The commission's order is the order of value and the tasks keep it as
their grouping; the landing order is: (1) ruling 5 — delete `Confine`,
`confined_command`, the `confine` fields, `confine_test.rs` and the
docker machine-proof scenario, add the parser refusal naming 0046 and
`container`; (2) ruling 1 — the enum, the v4 realms loader, the two
contracts, the parser refusal of `boundary`, the v9 map, `compile`'s
printout; (3) ruling 2 — `offered`, `refuse_unboxable`, the engine
entry fence, `doctor`, `init`; (4) ruling 4 — `hands.harness` in the
loader and the two adapters, the gate law with the pinned-script
grammar, `compose_site` with the harness fragments, `{result_path}`,
the unboxed environment and the network prefix, `DriverProcess::spawn`'s
environment parameter; (5) ruling 3 — the effect entries, seat-record
v4 and the stamp, the view unit and run-level fact, every readout, the
`seats` verb, the gate script; (6) re-pin every moved witness and
compose digest once, from the tests' left/right pairs, naming 0046 in
the doc comment; then guides and the erratum. Deletion first removes a
parameter from every `hands_command` call site before those sites are
touched again.

### DD17 — Availability is one probe table

`offered(path) -> BTreeMap<Boundary, Offer>` in `brokkr-cli`, with
`Offer::Offered(detail) | MissingTool(name) | Unbuilt { slice, tool:
Option<found> }`: `namespace` probes `bwrap` (and, for a bundle with an
overlay bind, 0.10 as today); `seatbelt` probes `sandbox-exec` and
reports `Unbuilt("ii")`; `container` probes `docker` then `podman` and
reports `Unbuilt("iii")`; `harness` and `open` are `Offered`.
`refuse_unboxable(bundle, path)` returns `Ok` for a bundle with no hands
site and otherwise judges `offered(path)[bundle.boundary]`, naming the
boundary, the tool, the seats and the ruling or slice. `doctor` prints
the same map on one `boundaries` line; `init`'s warning speaks the
vocabulary. The engine's entry fence (D21) refuses `seatbelt` and
`container` before any row for library callers.

### DD18 — Argv composition and the spawn environment

`compose_site(boundary, class, command, hands, harness_fragments,
workdir, roots, result_path, probe) -> SiteSpawn { argv, env }` replaces
the `hands_command(confined_command(..))` nesting at the four call
sites (single, panel member, sequence step, dialect step). Under
`namespace` it returns today's argv token for token with
`SpawnEnv::Inherit`; under `harness` a model site gets the adapter's
`gate` or `work` fragment by class with `{result_path}` and `{brokkr}`
expanded and no MCP server, and an exec site gets the compiled command
behind the network prefix when the probe passes, with
`SpawnEnv::Exactly(table)`; under `open` a model site gets the base
driver argv and an exec site the same as under `harness`.
`DriverProcess::spawn` takes the `SpawnEnv`. The seat input carries
`boundary` for every site with hands, the `hands: boxed` marker only
under a box of Brokkr's, and `result_delivery: last-message` when the
gate fragment's door is the harness's capture (D15, D23); the prompt
paragraph follows.

### DD19 — The seat record stamp is one helper

`stamp_boundary(record, site_boundary: Option<Boundary>)` is applied
where a driver's checkpoint and result pass from `run_driver` to the
store, to the finishing checkpoint (the one whose `step` ends in
`-session-finished`) and to the successful result, replacing any value
a driver wrote; per-turn checkpoints are untouched. The store's fence
validates under v4, dispatched by the 0.9 line (D9). Panel members,
sequence steps and dialect steps reach the same helper because they
reach the same `run_driver`.

### DD20 — The measurement is recorded, never guessed (D19, restated)

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
candidates and the version. The tree's proof that the compiler and the
roster keep ruling 6's promise runs in a scratch copy of the adapter
library with both members planted as fragments; a second test pins the
shipped refusal set and moves when the measurement lands as a data
change to `adapters/claude.json` and the pins that name its digest.
Codex's declaration is from the decision's own word and the tool's
record; the one fact still unmeasured about it — whether the capture
lands under the `read-only` class — is recorded in the guide as
pending and the operator's, and flips `gate` to unsupported with the
reason if it fails.

## Shape, by crate

- **brokkr-core** — `realms.rs`: `Boundary`, `Realm.boundary`,
  `SCHEMA_V4`, the version fence for the field; the `not applicable`
  serde helper.
- **brokkr-runtime** — `bundle.rs`: `Bundle.boundary`,
  `compile_with_realm`'s boundary argument, the `boundary` refusal at
  every site and inside `hands`, the `driver.confine` refusal, the v9
  map in `manifest_for`, the shared walk-exclusion function, the
  pinned-script grammar and lookup, the boundary axis in
  `enforce_model_policy`; the declaring layer's directory, which
  `expand_command`'s caller already holds, is handed to the check.
  `engine.rs`: `Engine.boundary`, the two entry fences,
  `boundary_entries` beside `select_candidates`, `compose_site`, the
  stamp helper, `manifest_diff` naming `boundary`; `confined_command`
  and `Confine` deleted. The adapter loader: `hands.harness` with
  `gate`/`work`/`result`, `{result_path}` admitted there and the two
  workspace tokens refused.
- **brokkr-protocol** — `process.rs`: `SpawnEnv` on `spawn`;
  `adapters.rs`: the prompt paragraph per boundary and delivery;
  `hands.rs`: the unboxed environment function beside the box's table
  it mirrors, and the network prefix and its probe.
- **brokkr-store** — `seat-record.v4` embedded and dispatched
  (`SeatRecordVersion::V4`, the 0.9 line).
- **brokkr-view** — `ModelAtBoundary`, `RunBoundary`, the boundary scan
  beside the provenance scan, `VIEW_VERSION` 9.
- **brokkr-cli** — `offered` and `refuse_unboxable` in one module;
  `run`/`resume`/`rerun`; `doctor`'s line; `init`'s warning; `seats`;
  `render.rs`, `tui.rs`, `ui.html`, `compare.rs` reading the unit.
- **Contracts** — `realms.v4`, `run-manifest.v9`, `seat-record.v4`,
  `effect-boundary.v1`, README rows, frozen-contracts entries.
- **Data and scripts** — the two adapters; `delivered-by-brokkr.sh`.
- **Docs** — the pages `boundary-guides` names and the erratum, plus a
  `seats` row in read-surfaces and a `rerun` note in its verb row.

## Risks

- **The claude fragments may not measure clean, or are not measured by
  the seat at all.** In either case the chief and the reviewers on
  claude refuse under `harness` by name, `codex` is the only macOS
  judge until the operator's measurement lands, and the promise is
  reported unmet (DD20). Accepted: the
  alternative is an unverified security claim in the file whose purpose
  is to be the verified one.
- **The unboxed environment may break the shipped verify script on a
  real Mac** (a toolchain outside `~/.cargo`, a `cargo` that needs a
  variable the table drops). It fails loudly at the verify gate with the
  script's own error, on the first run; the table is one function to
  amend. Accepted.
- **A bind's `mask` is not enforced outside a namespace.** Under
  `harness` the verify script's build scripts can read
  `~/.cargo/credentials.toml` (DD10). This is the fact *unboxed*
  states; the guide names it. Accepted as ruled: the mitigation the
  operator ruled is the cleared environment and the network off where
  the platform can say so.
- **The network prefix varies across kernels and policies.** The probe
  answers per spawn; nothing claims the outcome (DD15).
- **Every boxed bundle's digest moves once, and every inline gate on
  an adapter that gained `hands.harness` moves with it — codex now,
  claude when its measurement lands.** Re-pinned once at the end
  (DD16); a plain bundle is the fixed point that witnesses the key is
  absent by default.
- **`rerun` changes behaviour** (DD6): its manifest gains the realms
  pin and dialect. Its tests move; its guide row says so.
- **Coverage.** The gate is literal; every arm this design adds is in a
  pure helper with a table test (`offered`, the grammar, the
  environment, `compose_site`, the stamp, the view derivation), and the
  end-to-end box tests skip under `BROKKR_HANDS_BOX`.
- **Windows is named by ruling 6 and measured by nobody** (Open
  questions).

## Migration

- Contracts: four new files beside the frozen ones; nothing edited.
- Journals: old journals need nothing and render explicit absence
  (DD13); journals from the tagged 0.9.0 and 0.9.1 engines validate
  under v4 because it is additive (D9).
- Bundles: no shipped bundle declares `boundary` or `driver.confine`;
  none moves for either reason. Every bundle with hands moves once for
  the manifest key; the witness table and compose pins are re-pinned
  with 0046 as the reason.
- Realms: this repository's `realms.json` is untouched; absence reads
  `namespace`. A macOS adopter adds `"boundary": "harness"` to their
  realm under `forge.realms/v4`.
- Adapters: `codex.json` gains `hands.harness` now; `claude.json` gains
  it when the operator's measurement lands and declares none until
  then; `dsh` and `lanetally` declare none and are refused at a
  `harness` gate as at a boxed one.
- CLI: `rerun` pins the world it discovers; `seats` is new; `doctor`
  gains one line; `init`'s warning changes wording. `--json` consumers
  pin `view_version` 9.
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
3. **D6/DD8** — the operator confirms or refuses the dialect-step
   reading by keeping or deleting one scenario.
4. **"Every `effect/started`" read literally** — needs a v2 event
   lineage and is outside this slice's contracts; the operator says
   whether the narrowed reading (DD5) suffices.
5. **Windows under `harness`/`open`** — the environment table is
   exercised by the pure test on Windows CI; the shipped `sh` gates
   parse a Unix result path and need a POSIX shell, and `USERPROFILE`
   stays the operator's. Nothing here claims a Windows run; a Windows
   measurement is its own slice.
6. **The three questions 0046 leaves unruled** stay unruled here; #214
   (per-site boundary) is not built for.
