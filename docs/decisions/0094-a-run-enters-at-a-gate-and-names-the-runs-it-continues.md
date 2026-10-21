# 0094 — A run of any recipe enters at a gate over work already on the branch, and names the runs it continues

Status: proposed
Date: 2026-10-21

Built: unbuilt (#788)
Amends: 0051

## Context

Issue #788, part of epic #790 (cheaper recovery). `stopped` is terminal.
A run whose work is finished and committed, and which then stopped at a
hard rule or exhausted a gate's attempts, has one road back: a fresh run
of the same recipe, which begins at the table's `initial`, repeats intake
and design, and lets the design seat commit again over a branch that
already holds the design. Over 2026-10-18 and 19 one adopter's epic
re-drove six runs, each spending 15 to 25 minutes and a design commit on
intake and design before it reached verify. The shipped `landing` recipe
is `fast` entered at a classify gate (decision 0051), but it is Brokkr's
own (`extends brokkr-fast`), it begins where a branch written by hand
begins, and it says nothing of the runs whose work it finishes. Every
other recipe, an adopter's own frontier recipe among them, has no later
entry.

The pieces already exist. Every driver-bearing site carries a class,
`gate` or `work` (decision 0021 ruling 1), and `SeatBody::is_gate` reads
it for a single, a panel, a sequence and a selector. The compiler walks
the table from `initial` and refuses a path to a non-stop terminal that
avoids the protected phase (`assert_phase_unavoidable`, decision 0051's
constitutional lint); the core holds the same walk for decision 0050's
liveness. The engine enters `machine.initial` at `Cursor::Start`, and the
fold accepts whatever `phase/entered` follows. A seat the run returns to
receives `returned_from` (decision 0022), today only on a second visit.
`run/started` is built once (`Bundle::run_started`), and `layers` (#697)
is the precedent for an additive, optional, absent-by-default member.
`brokkr run` and `brokkr queue add` share one `LaunchArgs`. The
contribution gate reads one `Brokkr-Run` line under a native run-id
grammar, verifies the journal offline, and cuts the tier by the ship
anchor's patch map from the merge-base (decisions 0033, 0038). The run
view derives `recipe` from `run/started`, and the console, the TUI and
`brokkr ui` only paint it (decision 0071 ruling 7).

What is missing is one word the operator can say at start, `enter`, the
record of what the entered run continues, and the rule that says which
phases may be entered without a roster.

Weighed and set aside, each recorded in the change's proposal (D1 to D30)
and design:

- **A composed landing overlay any recipe derives.** An overlay's
  `initial` cannot be told from a recipe's own first phase, so the
  compiler could not refuse an entry at a work phase without a second
  marker; a recipe is static data and cannot name the runs one re-drive
  continues; and every adopter would write one overlay per recipe and
  gate, each restating one word (decision 0071 ruling 5).
- **The entry compiled into the bundle, or a manifest version.** A
  contract mint under a proposed decision (decision 0082 ruling 1), a
  second home for one fact, and every witness of an entered run's bundle
  moved. The manifest pins the recipe; the journal records the run.
- **The entry carrying a typed strategy and change for the skipped
  phases.** Selection and the `strategy_in` conditions read `RunState`,
  folded from this run's journal alone; a field the fold must read is a
  v2 event, which the frozen `{"const": 1}` of every manifest schema rules
  out, and a synthetic triage ruling would be a ruling no seat made.
- **Reading a continued run's fold, or its verify, into this run.** The
  chain is unverified (ruling 5), and a check that holds only sometimes
  reads as a verification it is not.
- **Promoting verify to a compiled role and refusing an entry from which
  it is avoidable.** It refuses `review` in every shipped table and
  `classify` in `landing`, against the issue's own list, and makes a
  landing's `initial` not an entry.
- **Fetching and verifying continued runs at the gate, as a preflight is
  fetched.** A continued run stopped before any anchor existed, and the
  gate's verifier requires a completed anchored run; the patch map already
  spans every commit a predecessor's journal could vouch a subset of.
- **A `queued-launch/v2` carrying the entry.** No queue asks for it
  (decision 0071 ruling 6).
- **Recording the table's order on entered runs only.** It leaves the
  run view two rail orders for good and a landing's graph wrong.

No contract file moves. `run/started` gains two additive, optional,
absent-by-default fields under the amended rule of decision 0016, as
`charters` and `layers` did; their numbered extension schema is minted
when this decision is accepted (decision 0082 ruling 1), and until then
this decision and the Rust types are their shape.

## Rulings

1. **An entry is a fact of the run, admitted by six facts of the table
   and its seats, and the engine holds no roster.** `brokkr run --enter
   <phase>` starts the run at the named phase of the table it runs under
   instead of the table's `initial`; the table, the compiled bundle, its
   digest, the run manifest and the witnesses do not move. The entry is
   admitted when, judged in this order: the phase is one of the table's
   and not a terminal; its name matches `^[a-z0-9][a-z0-9._-]{0,63}$`, the
   grammar the contribution gate prints; every driver-bearing site of its
   seat is a gate by the class it wrote, the seat's compiled whole-gate
   fact, a dialect step that declares none being work whether or not the
   realm's dialect supplies it; the protected phase is the entry
   itself or is unavoidable from it; no work phase is met by a forward
   walk from the entry, a forward ruling being one whose `next` stands
   after its `from` in the table's `phases` list; and no seat, step or
   ruling group the walk meets needs a journal fact that only a phase the
   walk does not enter can set (ruling 4). Any other entry is refused when
   the recipe and the entry are compiled, before a run id is minted or an
   event is written, by a typed error naming the phase, the first fact it
   failed with its witness, and the entries the table admits, derived by
   the same six facts in `phases` order. The third fact's witness is the
   first site whose class is work in the seat's written order, a
   sequence's steps in written step order, a dialect step by the class it
   wrote and one the dialect does not supply, which compiles no site,
   named by its step name and written class as uncompiled; the compile
   records it where it sets the whole-gate fact, so the two cannot
   disagree. A recipe whose own `initial` is a
   gate starts as it always did when no entry is named, and its `initial`
   is an entry like any other when it passes admission. `--enter` and
   `--continues` are `brokkr run`'s alone: a Looper-bound run refuses an
   entry by name, `brokkr queue add` refuses the flags as arguments it
   does not know, and a queued launch carrying an entry is refused before
   it is encoded.

   **Enforcement binding:** `crates/brokkr-runtime/src/bundle/entry.rs`
   (`admit`, `admitted`, `AdmittedEntry`, `EntryError`) and its tests in
   `bundle/tests.rs`; `crates/brokkr-runtime/tests/table_lints.rs` pins
   the admitted set of every shipped table by shape, `verify, review` in
   the `fast`-shaped tables, `panel-review`, `self`, `preflight` and
   `verify`, `classify, verify, review` in `landing`, `verify` in
   `research` and `research-dsh`, none in `triage`, `gpt-flash` and
   `night-shift`, each refused phase with its first fact and witness;
   `crates/brokkr-cli/tests/machine_proof/entered_run.rs` (the
   acceptance: a synthetic recipe listing intake and design ahead of
   implement, entered at `verify` over a finished branch, reaching
   `SHIP-COMPLETE` with no work seat); `crates/brokkr-runtime/tests/queued_launch.rs`
   and `crates/brokkr-cli/tests/cli_reference_tests.rs` for the flags'
   home.

2. **The protected phase stays unavoidable from wherever a run begins,
   and an entry at the protected phase is the operator's word, written
   down.** The fourth fact is decision 0051's constitutional lint seeded
   at the entry instead of `initial`, and both read one core walk,
   `Machine::reach_avoiding`, so the law has one home. In every shipped
   table `ship` is refused: `SHIP-COMPLETE` reaches `done` without review,
   and an entry there would let a run's own journal vouch a head no seat
   of that journal judged. The recovery for a ship that failed is decision
   0092's retry when it parked, or an entry at `review` when it stopped.
   An entry at the protected phase is admitted wherever the six facts
   admit it, `landing` included; the run's own journal then holds no
   verify, and it vouches a head no seat of that journal tested. That is a
   departure from "verified before judged", taken on the operator's word
   as decision 0051 ruling 3 takes the class's word for prose, with the
   platform suite on the pull request as the same backstop; unlike the
   class, it is journaled, and every surface, `run/started.entry.phase`,
   `context.entry`, the run view and the gate's check line, names the gate
   entered. The engine reads nothing of a continued run's verify. The
   by-hand road is untouched: a landing started without an entry
   classifies the branch, and one code path still routes to verify.

   **Enforcement binding:** `Machine::reach_avoiding` in
   `crates/brokkr-core/src/policy/audit.rs`, read by
   `assert_phase_unavoidable` (its refusal text unchanged) and by the
   admission's fourth fact; the `ship` refusal and the review-entry cases
   in `bundle/tests.rs`, `machine_proof/entered_run.rs` and
   `crates/brokkr-cli/tests/delivered_by_brokkr.rs` (a run entered at
   review vouches, and its line says so); `recipes/landing/README.md`.

3. **A work phase is reached in an entered run only by a return, and a
   return hands its finding to a seat the run has not visited.** The
   fifth fact follows forward rulings alone, so a table from which a work
   phase is reached forward is refused, which is "implement is entered
   only on a return" made mechanical for every work phase. In an entered
   run a seat receives `returned_from` on its first visit when the ruling
   that sent it is a return, a ruling whose `next` stands before its
   `from` in the `phases` list; a forward ruling into an unvisited phase
   hands nothing new, a revisit hands what it always did, and triage
   receives nothing on any visit (decision 0041 ruling 6). The rule reads
   the ruling's direction and never the entry's position. One predicate
   of `brokkr-core`, `Direction::of` over the table's order, a closed
   enum `Forward`, `Return` or `Loop` and none for a phase the order does
   not name, is read by the admission's fifth fact, the return rule and
   the run view's roads back, and nothing else decides a direction. No
   seat input of a run without an entry moves.

   **Enforcement binding:** `Direction` in `crates/brokkr-core/src/policy.rs`
   and its tests; `seat_context` in `crates/brokkr-runtime/src/engine/input.rs`
   with the pins in `engine/tests.rs` (the first return after
   `VERIFY-FAIL` handed, the forward first visit after `IMPL-OK` not, the
   ordinary run's input digest standing, a landing's first return still
   without); the forward-work refusals in `bundle/tests.rs`.

4. **A skipped phase's fact is an admission fact, never a carried one.**
   Three journal facts can be needed by a phase an entered run reaches
   without entering the phase that sets them, and the table holds no
   others: `strategy`, which only the strategy phase's ruling sets
   (decision 0041 ruling 6), needed by a seat that selects on it with no
   `default`, and by a `(phase, result)` group whose rulings that carry no
   `strategy_in` condition do not rule every valuation decision 0050
   ruling 4's sweep puts to them; and the dialect's `{change}`, needed by
   a dialect step that expands it, whose source is an earlier step of its
   own sequence when the seat declares `change`, else a result of one of
   the five change phases listed before it whose seat declares `change`,
   supplied only when every road from the entry to the step enters such a
   phase. The two `strategy` facts are judged over one breadth-first walk
   that follows every ruling and never enters the strategy phase; the
   `{change}` fact over the reach from the entry with the step's sources
   removed; the three rank in that order, each naming its own walk's
   first witness. A need the walk cannot supply is refused at admission,
   never left to park. The five change phases are one roster, read by the
   engine's `{change}` expansion and the admission alike. Under rulings
   1, 3 and this one the triage-shaped tables admit no entry today, and
   each refusal names its own first fact: `triage` and `night-shift`
   refuse `verify` and `review` by this ruling, on a seat that selects
   with no `default`, `triage` naming `implement` by `VERIFY-FAIL` and
   `night-shift`, whose `implement` is a plain work seat, naming `review`
   by `VERIFY-PASS`; `gpt-flash`, whose review positions are written
   `work` ahead of its chief, refuses `review` by ruling 1 and `verify` by
   ruling 3, review reached forward. The road to one is the recipe
   author's, a `default` case on a selecting seat and gate-classed judges
   where a work site stands in a judging phase, a recipe change named and
   not ruled here. Nothing of a continued run's fold is read into this
   run's.

   **Enforcement binding:** `Machine::unruled_without` in
   `crates/brokkr-core/src/policy/audit.rs` and its tests;
   `CHANGE_SOURCES` in `crates/brokkr-runtime/src/phase_name.rs`, read by
   `nearest_change` and the admission's sixth fact; the fixtures in
   `bundle/tests.rs` (a selector with a default admitted, a group only a
   skipped strategy decides refused, a `{change}` step with no road to its
   source refused, two facts failing with the first named).

5. **The chain is the operator's word, and the gate reports it while
   vouching the patch map.** `--continues <run>`, repeatable and only
   with `--enter`, names the earlier runs the entered run continues, each
   resolved once as `--run` is, held as a canonical id with the feature
   its own start recorded. Each must exist in the journal the run is
   written to, be concluded, `completed` or `stopped` (a parked run is
   resumed or concluded first, decision 0092 ruling 4), match the native
   run-id grammar `^[a-z0-9-]{0,32}-[0-9a-f]{8}$` the gate reads, not be
   Looper-bound, and be named once. A Looper-bound run is told apart by
   what its start pinned, the run-manifest/v2 lineage (`schema`
   `forge-run-manifest/v2`), never by its id, which is the dispatch's
   `forge_run_id`, any string of 1 to 512 characters, and may match the
   native grammar; every Looper-bound run is refused as a run to
   continue. Every refusal comes before an event is written, and nothing
   else, no realm, repository, recipe or verify, is compared: existence,
   conclusion, the id's grammar and the lineage its start pinned are the
   checks. The
   contribution gate reads `entry` from the vouching run's verified
   journal, names the gate entered and each continued run's id on its
   check line, never a feature text, and refuses an `entry` it cannot
   read, a phase outside ruling 1's grammar or a continued id outside the
   native grammar; an absent `entry` reads as today. The gate's claim is
   this and no more: it vouches the shipping run's whole-branch patch map,
   cut from the merge-base of the default branch (decision 0038 ruling 1),
   and reports the runs the operator named; it fetches no continued run's
   evidence and imports no result, verify, review, counter or security
   clearance from one, so a missing or unpublished predecessor supplies no
   authority and skips no gate. The two grammars are held equal to the
   compiler's and the launch's by one test.

   **Enforcement binding:** the `--continues` refusals in
   `crates/brokkr-runtime/src/launch.rs` with `launch/tests.rs`;
   `is_native_run_id` in `crates/brokkr-runtime/src/engine/entry.rs`,
   held to a minted id; the Looper-bound refusal in `launch.rs`, with
   a concluded Looper-bound run whose id is native-shaped refused in
   `launch/tests.rs`; `scripts/delivered-by-brokkr.sh`;
   `crates/brokkr-cli/tests/delivered_by_brokkr.rs` (an entered run over
   a branch two runs built vouches at `vouched` with the chain on its
   line, a malformed entry refuses, the patterns are the compiler's and
   the launch's, and a journal carrying the new fields verifies under the
   current verifier).

6. **The journal records the entry and the table's order, two additive
   fields the fold never reads.** `run/started` carries `entry`, an
   object of `phase` and `continues`, the continued runs in the order
   named each with `run_id` and `feature`, `continues` present as an empty
   array when no run was named, and `entry` absent on every run started
   without one; and `phases`, the compiled table's `phases` list verbatim,
   on every start. Both are typed once at the edge, `deny_unknown_fields`,
   with absence told from malformation by the key: a present `entry` that
   does not read is refused by the engine where it reads it, and the run
   view, which refuses no journal the fold accepts, states it unreadable
   with the reader's error (ruling 8). The engine enters the entry
   instead of `initial` at `Cursor::Start`, at the first drive and at a
   resume in the window after `run/started`, admitting it again by ruling
   1 over the bundle the manifest gate just proved, and refuses one that
   does not read or no longer admits rather than fall back to `initial`;
   it reads `phases` from no journal. The fold reads neither, `RunState`
   gains nothing, every journal written before folds, verifies and views
   as it did, and the engine change moves no manifest, table or charter
   witness. Their numbered extension schema is minted when this decision
   is accepted (decision 0082 ruling 1); until then the Rust types are
   their shape, and the vocabulary is published in `contracts/README.md`'s
   extension paragraph under this decision.

   **Enforcement binding:** `Bundle::run_started` in
   `crates/brokkr-runtime/src/bundle/origins.rs`; `RunEntry` and
   `RunEntry::of_started` in `crates/brokkr-core/src/entry.rs` with their
   tests (absent, empty chain, `null`, a scalar, a missing or unknown
   member); `first_phase` in `crates/brokkr-runtime/src/engine/entry.rs`
   with the resume-window, no-longer-admits and does-not-read pins in
   `engine/tests.rs`; the ordinary-start pin (no `entry` key, the same
   manifest digest, `phases` equal to the table's) in
   `machine_proof/entered_run.rs`; `crates/brokkr-runtime/tests/witnesses.json`
   standing in every row the engine change touches.

7. **Every seat but triage is told the entry, and the review judges the
   branch from its merge-base, by a clause this run does not write.**
   Every seat input of an entered run except triage's carries
   `context.entry`, the phase and the continued runs by `run_id` and
   `feature`, derived from `run/started.entry` alone; triage reads no
   history (decision 0041 ruling 6) and carries none; the continued runs'
   results are not handed; no seat input of a run without an entry moves.
   The five library review charters whose scope names the run's beginning
   and the two recipe copies pinned to the first scope the delivery as
   everything on the branch since its merge-base with the default branch,
   the cut the anchor's patch map takes, and say that `context.entry`
   means the branch holds the work of the runs it names and all of it is
   the seat's to judge; a library test holds the seven to the clause by
   path and every charter and recipe role to the rule that none scopes its
   read to the run's beginning. This run's own review panel is pinned to
   three of the seven and the dispatch door refuses a charter whose bytes
   moved (decision 0066 ruling 5), so this run writes none of them: the
   clause, its tests and the witness rows that follow the bytes land by
   hand on the branch after this run ships, and a landing compiled over
   the clause, `--recipe landing --enter classify --continues <this
   run>`, judges the whole branch, vouches the final head, and is the pull
   request's `Brokkr-Run`. This run's land ends with the push of the
   branch and its evidence; no pull request naming this run is sealed or
   queued; the pull request that merges names the landing, so no default
   branch ever holds `--enter` without the clause.

   **Enforcement binding:** `seat_context` in `engine/input.rs` with the
   reviewer, triage and ordinary-input pins in `engine/tests.rs`;
   `crates/brokkr-runtime/tests/roster.rs` (the two copies byte for byte
   their source, standing); `crates/brokkr-runtime/tests/library_data.rs`
   (the clause by path and the rule over both trees, the follow-on
   commit's); the change's `tasks.md`, whose charter task stays open until
   that commit and whose last task is the landing and the pull request;
   the implementer's notes, which open with the instruction that ends the
   land at the push.

8. **The run view names the gate entered and the runs continued, derived
   once, and its rail stands in the order the start recorded.** `RunView`
   derives `entry` from `run/started`: the gate, the continued runs each
   led by its title with its id after, and one sentence, `entered at
   <phase>, continuing <title> (<id>)` per run joined by `, `, or
   `entered at <phase>, continuing no earlier run`. A continued run's
   title is `brokkr_view::title` of its feature, the house's one
   derivation of a run's one-line name: its first line that says
   anything, clamped at a word to 60 display columns. Every one-line
   surface leads with it, the line `brokkr run` prints, the console's and
   `brokkr ui`'s headers and the TUI's header line, so a multi-line
   feature never spills; the journal, `context.entry` and `brokkr
   inspect --json` keep the feature whole. The console's run header,
   `brokkr ui` and the TUI paint it and derive nothing of their own;
   `brokkr run` keeps `run started: <id>` exactly, the id alone on its
   line, and prints the sentence on the line after it from the same
   derivation; nothing is painted or printed for a run without an entry.
   A present `entry` the view cannot read is stated, `entry unreadable:`
   with the reader's error, where the sentence would stand, and the rest
   of the run is painted as the fold gives it: a refusal is reported,
   never repaired into an ordinary run. The rail stands in the
   recorded `phases` order, each phase the journal entered once with its
   visits, a phase the order does not name after them by first entry, and
   a road back is a ruling the direction predicate of ruling 3 calls a
   return over that order, so an entered run's and a landing's first
   return are drawn and the forward step after one is not; a journal
   whose start recorded no order keeps first-entry order and the same
   predicate over it, which is how #813 drew it. The view's wire version
   is one past `main`'s when the change merges `main`, 26 expected
   behind #787's 25, and the console host pins it; every pin of the
   version moves with it.

   **Enforcement binding:** `crates/brokkr-view/src/entry.rs`
   (`entry_sentence`, the one text led by each continued run's title,
   and the unreadable entry stated) and `rail.rs`, with
   `crates/brokkr-view/src/tests/returns.rs` (a run entered at verify
   sent back once, a run entered at review reforged, a landing's first
   return, a journal with no order reading as before); `VIEW_VERSION` and
   `extensions/dsh/brokkr-console/lib/host/readouts.js`; the painters'
   tests in `crates/brokkr-cli/src/ui/tests.rs`, the TUI snapshots and the
   console's; the start-line pin in `machine_proof/entered_run.rs`.

9. **`landing` stays a recipe, and a judgment that could not land is
   relit as a landing that names the run it continues.** `landing` adds a
   seat and two rules in front of `fast`'s, a conditional entry read from
   the branch's class that a flag cannot express (decision 0051 ruling 1);
   it is untouched, and an entry composes over it as over any recipe,
   admitting `classify`, `verify` and `review`. `review-first` is `fast`
   entered at verify with a forced crew (decision 0060); its difference is
   the crew, and it is untouched, as are decision 0060's two open items.
   What the two share with every entry is one law in one place: the
   protected phase is unavoidable from where a run begins, held by the
   compiler for every `initial` and for every entry over the same walk.
   Decision 0051 ruling 4 is amended: a run stopped at the reforging bound
   or a judging pass over a branch is followed by a landing on the ruled
   head that names the run it continues, and a stopped run of a recipe
   that admits its gate is re-driven there with `--enter` instead.

   **Enforcement binding:** `crates/brokkr-runtime/tests/landing_shape.rs`
   (unchanged, since the recipe is); the landing-without-entry case and
   the `--enter classify --continues` case in
   `machine_proof/entered_run.rs`; `recipes/landing/README.md`;
   `docs/guides/recipe-authoring.md`, which says which phases a table
   admits and how an author widens the set.

## Consequences

- A re-drive over a finished branch costs the gates alone: a verify's
  minutes, one judge's read, and the smith only on a finding, against
  the 15 to 25 minutes and the design commit each of the six re-drives
  spent. An entry at a work phase is refused when it is typed, with the
  entries the table admits in the refusal.
- Every `fast`-shaped table, `panel-review`, `self`, `preflight` and
  `verify` admit `verify` and `review`; `landing` admits `classify`,
  `verify` and `review`; `research` and `research-dsh` admit `verify`;
  the triage-shaped tables admit none, each refusal naming its first
  fact and witness.
- Every ordinary run's seat inputs, digests, manifest and witnesses stand;
  its `run/started` gains the one `phases` list. No contract file, no
  frozen surface, no fold arm and no store schema moves.
- The engine's constitutional walk, the sweep over a group's
  unconditioned rulings and the direction of a ruling each have one home
  in `brokkr-core`, read by the compiler, the engine and the view.
- The public API of `brokkr-core`, `brokkr-runtime` and `brokkr-view`
  gains the types above; the snapshots under `quality/public-api/` are
  re-measured on the pinned nightly and cited in the pull request.
- The charters' clause and its witnesses land by hand after this run,
  and the pull request that merges names the landing that judged the
  whole branch over them.
- Named, not ruled: the return rule extended to every run, so a
  landing's first return hands its finding; a `default` case on
  `recipes/triage`'s selecting seats; a `queued-launch/v2`; an entry on a
  Looper-bound run; the gate lighting a landing itself (decision 0051).
