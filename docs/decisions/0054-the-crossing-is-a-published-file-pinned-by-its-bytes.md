# 0054 — The crossing: a realm publishes a file, and the realm that depends on it pins the bytes

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
