# 0057 — The crossing: a realm publishes a file, and the realm that depends on it pins the bytes

> **Number.** Drafted as 0054 on the phase-2 branch; renumbered to 0057 during integration because shipped main already assigns 0054 to the DSH linked-worktree metadata decision. The proposal's semantics and status are unchanged.

Status: proposed
Date: 2026-09-09

## Context

Decision 0023 drew the world and deferred the paths between its realms.
Its ruling 7 says so in as many words: "Bifröst crossings (published,
digest-pinned inter-realm contracts) and multi-realm runs … build on
this map when ruled." This decision is the first of those two, and it
rules the vocabulary only.

What the tree holds today, measured rather than assumed:

- **Phase 1 shipped (0023).** `realms.json` is at this repository's root
  and names one realm — this repository, `docs/house-rules.md`,
  `openspec`, journal `.forge/forge.db` — under `forge.realms/v3`. The
  map is pinned and embedded per run, and `brokkr realms` reads it back.
- **Many hearths shipped (0026 ruling 1).** `forge.realms/v2` gave a
  realm its own `journal`; a realm that names none still falls back to
  the world's.
- **The house and the dialect are the realm's (0041 ruling 8, 0042).**
  `forge.realms/v3` carries both as repository-relative declarations,
  refused if they leave the tree that owns them
  (`is_repository_relative` in `crates/brokkr-core/src/realms.rs`).
- **The boundary is the realm's (0046 ruling 1).** `forge.realms/v4`
  carries the closed five-word vocabulary; absent, it reads `namespace`.
- **And no world had ever held two DISTINCT repositories until Phase 2
  slice (i).** Every multi-realm map exercised before it — the many-hearth
  fixtures, the v3 house and dialect fixtures — pointed its realms at one
  tree. Commits `8c80277`, `68116ee` and `740c462` landed the fixture
  that does not: `two_repositories()` and `TWO_REPOSITORIES` in
  `crates/brokkr-runtime/src/realms/tests.rs` build two real git
  repositories at two different HEADs and prove that four surfaces answer
  for the second one. That fixture is the ground a crossing needs, and it
  is why this vocabulary can be ruled now and could not be ruled before:
  until a world could hold two repositories, "between realms" had no
  referent.

So the question this decision answers is narrow, and everything under it
is already ruled: **what does one realm say about a contract another
realm owns?**

## The name

Decision 0019 ruling 10 governs: the machine's own mouth is plain, the
Edda keeps the myth. Bifröst is already spent — `docs/lore/edda.md`
gives it to the bridge crate, and that entry is corrected in this change
because it still names the crate `forge-bridge`, which the 0019 rename
made `brokkr-bridge`. The crate is the Looper producer bridge and has
nothing to do with realms; the name stays where it landed rather than
being reassigned here.

The mechanism is therefore a **crossing**, a plain word, in the schema
(`publishes`, `consumes`), in the code (`PublishedCrossing`,
`ConsumedCrossing`) and in every refusal. No Edda entry is added: this
mechanism is named for what it does.

## Decision

1. **A crossing is a FILE the publishing realm owns.** It is named
   repository-relative, on exactly the terms `house` and `dialect` are
   named — no absolute path, no drive letter, no parent escape, judged
   portably on every host. Its BYTES are the contract. It is not a
   package, not a registry entry, not a URL; Brokkr fetches nothing, in
   this slice or any later one this decision rules on. A realm says what
   it publishes as `publishes: [{name, path}]`, where `name` is in the
   realm-name grammar and is unique within the realm that publishes it.

2. **A consuming realm names the publisher and pins the bytes.**
   `consumes: [{name, realm, sha256}]`: the crossing's name as its
   publisher publishes it, the realm that publishes it, and a lowercase
   64-character hex sha256 **over the published file's RAW bytes**. Raw
   bytes, not canonical JSON: a crossing may be a JSON schema, a
   `.proto` or a Markdown document, and only the publisher's own format
   knows what canonicalising would mean. The map's own pin stays what it
   was — canonical JSON over the map (0023 ruling 4) — because the map is
   JSON and Brokkr owns it. The two rules do not conflict; they apply to
   two different objects, and this decision says which is which so that
   no later reader has to guess.

3. **The refusals, all of them pure.** `brokkr-core` performs no I/O
   (decision 0003, constitutional boundary 1), so every rule here is a
   rule about the map's text and nothing else. A map is refused when:

   1. a `consumes` entry names a realm this world does not hold;
   2. a `consumes` entry names a crossing that realm does not publish;
   3. one realm publishes the same crossing name twice, or consumes the
      same crossing name twice;
   4. a pin is not 64 lowercase hex characters — the same spelling of
      "malformed digest" every other digest in this build is held to,
      which is why `is_sha256_hex` now lives in `brokkr-core::canonical`
      and the dispatch envelope's pins read it from there;
   5. `publishes` or `consumes` appears in a map calling itself v1, v2,
      v3 or v4 — the refusal a v2 `journal` gets inside a v1 map, a v3
      `house` inside a v2 map and a v4 `boundary` inside a v3 map, now
      written once as `older_than` so the fifth version's gate reads
      exactly like the second's;
   6. **a realm consumes its own published crossing.** Refused. A
      crossing is between realms: a realm that pinned its own file would
      be pinning something it can simply open, at a digest that moves
      whenever its own tree does, recording a dependency on itself as
      though it were a contract. The publishing realm's own use of its
      own file needs no vocabulary at all.
   7. **either word is written as `null`.** A written `null` is not an
      absence: `realms.v5` types both lists `array`, and v1 through v4
      have no such property at all, so `"publishes": null` is a map that
      its own contract file refuses at every version. It is refused here
      too, by name — and, being written, it is judged by the version gate
      first, so a v4 map cannot slip v5 vocabulary past that gate by
      naming nothing with it. This is a rule about the reader as much as
      the map: the obvious Rust shape reads a missing property and a
      written `null` into the same `None`, which would have let a map be
      accepted that no validator would accept, so the two states are held
      apart in the type and the third state — a realm that never said the
      word — stays the only absence there is.

   A published crossing is also held to its own shape — a name in the
   realm-name grammar, a non-empty repository-relative path — because a
   name that cannot be read back out of evidence is not a name.

4. **Absent both properties, a v5 map is a v4 map.** Same realms, same
   journals, houses, dialects and boundaries, resolved the same way; and
   every v1, v2, v3 and v4 map keeps loading byte for byte as it does
   today. A world that never drew a crossing notices nothing.

5. **Nothing is read, resolved or verified in this slice.** The pin is
   judged as a SHAPE and never against bytes on disk. Reading a
   published file, resolving a `path` against the publishing realm's
   tree, re-deriving a digest, and any readout, doctor line or manifest
   pin that would report a crossing are later slices, in the crate that
   is allowed to touch a filesystem. The vocabulary lands first,
   deliberately, exactly as `forge.realms/v4` landed the boundary's
   shape before the slices that built one.

6. **Minimal, by ruling — 0023 ruling 2 again.** Two properties, five
   field names, and nothing else. No content type, no description, no
   version field on a crossing, no registry, no fetch configuration, no
   compatibility relation between two pins. Unknown fields are refused
   inside the new entries exactly as they are at the realm and map
   levels, so any of those must arrive as `forge.realms/v6` rather than
   as drift inside a file still calling itself v5.

## What this decision does NOT settle

Two questions are named here and left open on purpose. Neither is
answerable by a vocabulary, and answering either speculatively would put
a mechanism in the schema before there is a ruling behind it.

- **How a consumed crossing's bytes reach a consuming realm that is not
  co-located.** In a world whose realms are checked out side by side the
  file is simply there. In a world where they are not, something has to
  carry it, and something has to say what happens when the publisher
  moves the file and the pin no longer matches — the messenger along the
  trunk, Ratatoskr, which decision 0023 named in the same breath as
  Bifröst and deferred to Phase 3. This decision does not choose fetch,
  vendoring, submodule or wait-for-the-operator, and the schema carries
  no field that presumes one.

- **Whether a consuming realm may plant a keep-ref in a publishing
  realm's repository.** Decision 0028 already names this exact gap: its
  consequences record that "cited objects that this repository does not
  hold — another realm's head, or an object already collected before
  anything was planted — are reported as a gap rather than silently
  dropped." A pinned crossing is an exhibit of the same kind, and its
  bytes live in a repository this run may not be able to write to. It
  stays a gap. This decision neither widens 0028's namespace across
  realms nor rules that it may not be widened.

## Consequences

- **A world of two repositories can now say what passes between them,
  and say it in evidence.** The map is already pinned and embedded per
  run (0023 ruling 4), so a run in a consuming realm testifies to the
  digest it was built against, from the journal alone, forever — before
  any machinery exists to check that digest.
- **A crossing name is one name inside one realm.** Two publishers using
  the same crossing name cannot both be consumed by a single realm: the
  map is refused rather than resolved by pair. That is deliberate — a
  crossing name is how a realm refers to the thing it depends on, and one
  name must mean one file. Giving each publisher its own namespace would
  be a v6 and needs a world that wants one.
- **A pin is a fact about bytes, not about meaning.** Reformatting a
  published JSON schema breaks every pin on it, and this is correct for
  a contract whose consumers may not parse JSON at all. A publisher that
  wants formatting freedom publishes a file whose format defines it.
- **Coverage and bytes.** `contracts/realms.v5.schema.json` lands beside
  v1–v4; v4's bytes join the pinned frozen set in the same change, so
  "beside, never inside" is machine-checked for this version too. The
  production table, the corpus and `reference/` are untouched.

## Addendum: Phase 2 close-out, 2026-09-10

This addendum records what building the Decision taught. It changes no
ruling above; 0057 stays `proposed`, because only the operator accepts a
decision.

### What Phase 2 built

The vocabulary landed first (`c02d97d`), then the loader that resolves
every publication against the publishing realm's own tree and holds each
consumer's pin to the raw bytes it finds (`07ed4ab`). A crossing that
moved refuses `run`, `rerun`, `resume` and `compile` before a seat spawns
(`b9b2ee6`, `ca0c765`), in one wording every surface reads from the one
`WorldError`. A run records the digests it stood on beside the map in
`run-manifest/v10` (`9111ee2`) — declaration and observation deliberately
kept apart. `brokkr doctor` reports every crossing and refuses nothing;
an unreadable publication is the publisher's line, and its consumers'
pins are `unchecked`, never matching (`ca0c765`). `brokkr realms` reads
each realm's publications and consumptions out under its own line, and
`brokkr muninn run` carries the same report into the world's dossier,
where a moved pin is a finding under the consuming realm. Both read that
one already-computed report; neither opens a crossing file, hashes a byte
or compares a pin a second time.

### Ratatoskr, sharpened

The readouts already separate two reasons a consumer's pin cannot be
confirmed. A **moved** pin is the consumer's own fault — the publisher's
bytes are not the bytes it declared — and the consumer's run refuses. An
**unchecked** pin is the publisher's fault: the published file could not
be read at all, the publisher's own line carries that, and the consumer is
warned without being charged. A world whose realms are not co-located
adds a THIRD reason that file cannot be read: it has not been fetched yet.
From inside `resolve_crossings` today that is indistinguishable from
"never published", and Phase 2 could not tell them apart because it never
fetched anything.

Whatever Ratatoskr becomes, it must give that third state its own word.
This slice's readouts and doctor's line already promise a reader that
`unchecked` means "the publisher's file is unreadable", and "not fetched
yet" is nobody's fault at all. Folding it into `unchecked` would blame a
publisher that did nothing wrong; folding it into `moved` would refuse a
run over bytes that may be perfectly current. Neither is acceptable, so
the messenger has to be able to say "not here yet" before it may say
either.

### 0028's gap, sharpened

The fault-direction split above is itself evidence for the gap 0028
leaves. Attribution followed WHO the run-time consequence falls on: a
moved pin refuses the CONSUMER's run, so it is the consumer's finding,
even though the bytes physically belong to the publisher. If a keep-ref is
ever allowed to plant across realms — the question 0028's Consequences
leave open, where an object "this repository does not hold" is a reported
gap — the same rule likely wants to govern it: the realm whose run would
break is the realm whose journal should carry the finding. This is offered
as a steer for whoever rules that widening, not a ruling made here.

### What Phase 3 inherits

Decision 0023 ruling 7 names what follows: "multi-realm runs (one feature,
per-realm implement and gates, a join before done — the recorded heritage
shape)". Phase 2 leaves that shape's inputs in place — a map of distinct
repositories, each realm's own tree, journal, boundary, house and dialect,
and crossings between them resolved and refused at load — and does not
build the shape itself.

Two things are said honestly rather than implied. Nothing in Phase 2
exercised a SECOND realm consuming a crossing under load: every crossing
fixture is two realms in one workspace, and the consuming realm never
executed, so the refusal is proved at load and not yet under a running
consumer. And no crossing was tested whose publishing realm declares no
`boundary`, `house` or `dialect` of its own, so how 0046 and 0041 compose
with 0057 at a publisher that is only a publisher is unproven. Both are
work Phase 3 should carry, not facts Phase 2 established.
