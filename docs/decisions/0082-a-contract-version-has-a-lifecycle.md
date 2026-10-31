# 0082 — A contract version has a lifecycle: it is minted under an accepted ruling, tabled with its writer and reader, and read for a stated window

Status: accepted — operator ruled in chat 2026-10-12
Date: 2026-10-11

Built: partial (#361) — slice 1 built rulings 1, 2, 4 and 5 (the table, its check and the minting rule); ruling 3's windows, retirement and FROZEN binding are unbuilt

## Context

A contract under `contracts/` changes only by a new numbered version
beside the old one. That law has held since the first implementation,
and it is enforced: the loaders refuse a label they do not read, and a
byte-pin test holds most of the older files still. Nothing governs the two ends of a version's
life. Nothing says what authority may mint one, and nothing says how long
an older one must be read. Issue #361 measured this at `ba52724e`.
Research entry 0022 measures it again at `3ad0f66c`, file by file:

- 42 schema files in 16 families. 17 are current, 14 are older versions
  still read, and 11 are record only: older local run manifests that no
  reader accepts, because resume refuses any pinned manifest unequal to
  its recompile.
- 33 files were minted under an accepted ruling. Five were minted under a
  ruling still `proposed`: `seat-record.v5` (0056), `realms.v5` and
  `run-manifest.v10` (0057), `phase-entered-head.v1` (0039), and
  `effect-cleanup.v1` (0006's addendum of 2026-09-27). `realms.v7` was
  minted under 0075, a number with no file in the tree.
  `run-manifest.v13` was minted on the operator's in-chat ruling of
  2026-10-10 (#430, PR #621), which no decision text records. The Looper
  pair, `dispatch-envelope.v2` and `run-manifest.v2`, names no decision.
  A declined proposal leaves this directory. Frozen bytes cannot follow it
  out.
- `docs/guides/versioning.md`'s status table names 20 files and omits 22,
  and three of its sentences are stale. Nothing checks it against the
  tree.
- Only seat records are dispatched by the engine line that wrote them
  (`crates/brokkr-store/src/seat_record.rs`), so only seat records have a
  reader whose age can be stated. Today it keeps every version since v1,
  including v2, which no input selects.
- 14 files are not pinned by digest, among them the superseded dialect
  versions v1 and v2.
- `forge.phase-machine/v1` has no schema file. Its only instance is the
  frozen production table, and every shipped recipe table declares v2.

Four questions were weighed.

**Who may mint.** (a) Only an accepted ruling. (b) Any ruling, with the
version marked `enacted, pending acceptance` until the operator rules.
(c) A new decision status word, `enacted`, in the closed vocabulary.
(a) is the simplest rule, and the evidence shows (b)'s cost: seven files
minted on authority that is pending or absent. (c) widens a vocabulary the
whole ledger reads to solve a problem that only contract rows have. The
pending state is recorded where it applies, on the contract's row, and
only for the versions minted before this decision. Whatever the rule, a
test must read it from a closed form. A decision's `Status:` line is one
already. An addendum's heading is not: the ledger holds 25
`## Addendum` headings in at least seven forms, three with no status word
at all, and a heading such as "not yet ruled" holds the word `ruled`.

**How long an older version is read.** (a) For ever, as today. (b) A
window in engine minor lines. (c) A window in calendar time. Ten minor
lines shipped between 2026-08-29 and 2026-10-02, so a window in lines is
short at today's cadence. It is still the unit the store already
dispatches on, and the unit a release bumps. (c) would make a release
depend on a date nothing in the tree records. A window applies only where
the engine wrote the version into a journal. A file the operator writes,
such as a realms map or a host file, is not aged by a release.

**Where the table lives.** A table generated into `versioning.md`, or one
written by hand and checked by a test. The decisions index and the
research index are both written by hand and checked, so the house has one
form already.

**`forge.phase-machine/v1`.** Add a schema, or record why it has none. A
v1 schema written now would describe one frozen file after the fact. The
v1 rule level is open on purpose (`crates/brokkr-core/src/policy/errors.rs`),
so such a schema would either admit any key, proving nothing, or close
what the loader leaves open, so that the schema and the loader disagree.

## Rulings

1. **A new contract version is minted only under an accepted ruling.**
   A file added to `contracts/` names the ruling it was minted under: a
   decision, or an addendum inside one.
   - **A decision** mints if its `Status:` reads `accepted` and nothing
     supersedes it. A `withdrawn` decision was never accepted and mints
     nothing. A `proposed` one does not mint. The build adds the
     transition sentence to the decisions README's vocabulary bullet: a
     proposal's `Status:` never becomes `superseded`, and an accepted
     decision is never withdrawn. The sentence is about the status word
     only. A part of a proposal may still be taken over by pointer: 0055
     reads `Status: proposed` over `Superseded in part by: 0073` today.
     0055 mints no version, so no row names it and no walk below reads
     its pointer.
   - **A superseded decision** keeps a minting only when its successor
     says so. The test never asks when a version landed, because nothing
     in the tree records it. It reads the successor instead. A decision
     that carries a `Superseded by:` or a `Superseded in part by:` pointer
     admits a row only when exactly one decision its pointer names carries
     a header line reading exactly `Keeps the minting of:
     contracts/<file>`, with the row's file. A pointer may name several
     decisions, comma-separated as the decisions README writes them, and
     each is walked; two that keep the same file refuse as ambiguous,
     naming both (Ruled item 9). That successor must admit by this same rule, so a
     chain of supersessions is walked to an accepted decision that nothing
     supersedes, and a number met twice on the walk refuses. A pointer
     naming a number with no decision in the ledger refuses by the default
     below, and the ledger's own gate already refuses it as `names no other
     decision` (`decisions_index.rs:436-437`, `Finding::DanglingPointer`).
     Any other case refuses by default, with one text:
     `contracts/<file>: minted under <old>, which is superseded by <new>,
     and <new> does not keep its minting`. So the pull request that
     supersedes a minting decision fails until the successor names each
     file whose minting it keeps, and a version that lands later under
     the old number is refused, since no successor names a file it has
     not seen. Because the walk ends only at an accepted decision, a
     decision that supersedes a minting decision lands on `main`
     accepted, never as a proposal, unlike every decision today
     (Ruled item 10). A `Keeps the minting of:` line is admitted only for a file
     whose row is minted under a decision the keeper supersedes, directly
     or along the walk. Any other such line is refused as unused (decision
     0071 ruling 6). The build adds the line to the decisions README's
     header vocabulary.

     No decision is superseded whole today. Two that are superseded in
     part mint versions: 0041 (`run-manifest.v7`, `run-manifest.v8`,
     `realms.v3`, `phase-entered-case.v1`) and 0043 (`run-manifest.v6`),
     both by 0045, which does not touch those contracts. The build gives
     0045 one `Keeps the minting of:` line for each of the five files,
     for the operator to confirm (Ruled item 8). The other five decisions
     superseded in part (0008, 0030, 0033, 0049, 0055) mint no version.
   - **An addendum** mints only by its own status line, never by its
     heading. The first non-blank line under its `## Addendum — <date>`
     heading reads exactly `Addendum status: accepted (operator, <date>)`,
     with `<date>` in `YYYY-MM-DD` form, and that whole line is the one
     form that admits. That date is the day the operator accepted the
     addendum, which may follow its heading's date, so the two are not
     required equal. The heading's date alone selects the addendum a row
     names, and the status line's date is read for its form only. A line opening `Addendum status: proposed` or
     `Addendum status: withdrawn` refuses by that word. Any other line in
     that place, or none, refuses as unreadable. The decision holding the
     addendum must itself admit.
   - **An erratum inside a ruling**, such as 0034 ruling 7 ("added
     2026-09-05, operator ruled", `0034-the-seat-record.md:123-124`), is
     the decision's own text and reads under the decision's `Status:`.

   Four addenda mint a version today, and the build gives each its status
   line once, transcribed from its heading, for the operator to confirm in
   the build's pull request (Ruled item 8). 0042's 2026-09-04 and 2026-09-06
   addenda and 0068's 2026-10-10 addendum read
   `Addendum status: accepted (operator, <the heading's date>)`, and
   0006's 2026-09-27 addendum reads `Addendum status: proposed`. The other
   21 addenda are not touched, and no status is ever read from them, since
   no row names them. An accepted addendum's line never changes: a later
   ruling that replaces it is a new addendum or decision, so it never
   fails a row it admitted.
   A decision whose proposal is still open may describe a version, and its
   slice may build the version on a branch. The version lands on `main`
   only once the ruling that mints it is accepted.

   **Enforcement binding:** `crates/brokkr-cli/tests/decisions_index.rs`
   gains `every_contract_version_is_minted_under_an_accepted_ruling`,
   reading the `Minted under` cell of every row of ruling 4's table. The
   cell is one decision number, optionally followed by the date of the
   addendum that minted it (`0042 (2026-09-06 addendum)`), or one of
   ruling 2's listed cells or its withdrawn form of one, matched whole.
   Every other form refuses as unreadable. Three files,
   `run-manifest.v1`, `event-envelope.v1` and `driver-protocol.v1`, were
   frozen "under decisions 0003–0005" (`contracts/README.md:3-4`), a
   range the cell does not read. The build writes `0003` for each: its
   ruling places the event contract, bundle shape and driver protocol in
   the target architecture (`0003-native-rust-runtime.md:98-100`), its
   bold `**Status**:` line reads `accepted`, which the index test reads
   bold or plain (`decisions_index.rs:494-498`), and it carries no
   supersession pointer. The decision must be in the
   ledger with a `Status:` reading `accepted` or `superseded`; a
   `proposed` or `withdrawn` one never mints, and only ruling 2's list
   records a version that stands without it.
   If it carries a `Superseded by:` or `Superseded in part by:` pointer,
   the walk above must reach an accepted decision that nothing supersedes
   and that keeps the row's file. A pointer naming several decisions is
   walked through each, and exactly one must keep the row's file; two
   that keep it refuse as ambiguous, naming both. A named addendum must have exactly one heading
   in that file opening `## Addendum — <date>`; none, or two, refuses,
   naming the file. Its status line is matched whole against the one admitting form,
   and the heading's text after the date is never read. The test reads the
   status of the ruling a cell names, never that ruling's text against the
   row's file, so a cell naming the wrong accepted decision passes it; the
   build transcribes each cell from research 0022 for the operator to
   confirm (Ruled item 8). Removal controls
   for the build, each planted, run, seen failing with the test's own
   text, and removed:
   - a row minted under a proposed decision fails, naming the file and the
     decision;
   - a negated addendum, headed `## Addendum — 2026-10-12, host v2, not
     yet ruled` over `Addendum status: not yet ruled`, refuses as
     unreadable, naming the file and line;
   - an unknown one, `Addendum status: ruling requested`, refuses as
     unreadable;
   - a heading ending `, ruled` with no status line refuses as unreadable,
     which shows the heading is not read;
   - `Addendum status: accepted (not yet ruled)` refuses as unreadable;
   - `Addendum status: proposed` refuses as proposed;
   - a row's decision set to `withdrawn` fails;
   - an existing accepted mint survives supersession when the successor
     names its file: set a row's decision to `superseded`, with its
     `Superseded by:` pointer to a planted accepted successor carrying
     `Keeps the minting of: contracts/<that row's file>`, and it passes;
   - a new row under a superseded decision whose successor does not name
     it refuses: under the same plant, add a row for a new file minted
     under the superseded decision, and see it fail with
     `contracts/<new file>: minted under <old>, which is superseded by
     <new>, and <new> does not keep its minting`;
   - delete one of 0045's five `Keeps the minting of:` lines and see its
     row fail with the same text, naming 0041 or 0043 and 0045, which
     shows a supersession in part is read like a whole one;
   - a `Keeps the minting of:` line naming a file minted under a decision
     the keeper does not supersede refuses as unused;
   - two keepers: point 0041's `Superseded in part by:` at 0045 and a
     planted accepted decision, give the plant the `Keeps the minting
     of: contracts/run-manifest.v7.schema.json` line the build gives
     0045, and see the row refuse as ambiguous, naming 0045 and the
     plant, which shows the walk counts keepers rather than stopping at
     the first;
   - a number met twice: plant two decisions that each carry a
     `Superseded by:` pointer to the other, mint a row under one, and
     see the row refuse, naming the repeated number, rather than the
     test looping or admitting.

2. **The versions already minted without an accepted ruling stand, and
   are marked.** Their bytes stay frozen. Each row reads
   `enacted, pending acceptance` after its authority, and the test from
   ruling 1 admits that marker only for this closed list, by exact file
   and cell:

   | File | Minted under, as recorded |
   |---|---|
   | `seat-record.v5` | 0056, enacted, pending acceptance |
   | `realms.v5` | 0057, enacted, pending acceptance |
   | `run-manifest.v10` | 0057, enacted, pending acceptance |
   | `phase-entered-head.v1` | 0039, enacted, pending acceptance |
   | `effect-cleanup.v1` | 0006 (2026-09-27 addendum), enacted, pending acceptance |
   | `realms.v7` | 0075 (unmerged, draft PR #486), enacted, pending acceptance |
   | `run-manifest.v13` | operator ruling 2026-10-10 (#430, PR #621), enacted, pending acceptance |
   | `dispatch-envelope.v2` | no decision (PR #28), enacted, pending acceptance |
   | `run-manifest.v2` | no decision (PR #28), enacted, pending acceptance |

   The list only shrinks. When the operator accepts the decision, the row
   drops the marker and names it plainly, and a listed row whose decision
   is then accepted fails the test until it does. A listed row whose
   decision reads `withdrawn` keeps its place with `withdrawn` in place of
   the marker: its cell is the listed cell with `enacted, pending
   acceptance` replaced by `withdrawn`, matched whole, as in
   `0056, withdrawn`. The test admits that form only for a listed file
   whose cell names a decision in the ledger, and only while that
   decision's `Status:`, or the named addendum's status line, reads
   `withdrawn`. Under any other status it refuses, and a row naming no
   decision in the ledger has no withdrawn form. One whose decision reads `superseded` is refused, because a
   proposal's `Status:` never becomes `superseded`. Every row off the list passed the check
   on the day it landed, so its decision was accepted then. A decision that minted
   a contract version never leaves this directory. If the operator
   declines it, it stays with status `withdrawn` and a dated note, and the
   withdrawal rules its versions' reader state. A version that lost its
   authority keeps its reader under ruling 3 like any older version, and
   retires only as ruling 3's binding admits a `retired` row. A journal
   version nothing ever wrote reads `Last written` `never`, so its
   deadline is the build's landing line and it retires in the release
   after. Two rows need a home in
   the tree before they can leave the list. `realms.v7` needs either 0075
   landing or a dated erratum to 0065, where PR #526 ported the
   provisional tier. `run-manifest.v13` needs a dated erratum to 0068's
   2026-10-10 addendum recording the version beside `forge.host/v1`. Both
   errata are the operator's to make. `forge.host/v1` stands as it is,
   under 0068's addendum of 2026-10-10, once that addendum carries the
   status line ruling 1 transcribes from its heading, which says `ruled`. A
   later Looper-lineage version needs an accepted ruling like any other.

   **Enforcement binding:** the closed list inside the test from ruling
   1. The build adds this exception to the decisions README's sentence on
   declined proposals. Removal controls: add a tenth file with the
   marker, see it refused as outside the list, then restore; write
   `seat-record.v5`'s cell as `0056, withdrawn` while 0056 reads
   `proposed` and see it refused, then set 0056's `Status:` to
   `withdrawn` and see it pass.

3. **An older version's reader is kept for a stated window, and its bytes
   are never touched.** Every family is one of two classes.
   - A **journal** family is one the engine or its drivers write into a
     journal: the run manifests, the seat record, the event envelope and
     its payload extensions, the operator supersede, and the dialect.
     The reader of an older version is kept for N engine minor lines
     after the last line that wrote it (N is asked below, recommended 3).
     In the first release past the window the reader is removed, unless
     an accepted decision rules a longer window for that row, and the row
     then names it. A window is never shorter than N, and never longer
     without that decision. After that, export, import verification and offline
     verification refuse a journal whose engine line no kept reader
     covers. The refusal names the engine line that wrote it, so the
     operator verifies it with that engine. It is never read under a newer
     contract and never skipped.
   - An **input** family is one an operator or a counterpart writes and
     the engine reads live: the realms map, the host file, the tool
     dialect, the phase-machine table, the dispatch envelope and the
     driver protocol. No release ages it. An input
     label's reader is removed only under an accepted decision that names
     the release (decision 0071 ruling 6), and `brokkr doctor` names the label to move
     to for at least one minor line before that release. The decision
     names both lines in one header line, and the binding below reads it.
   - A **record** row is an older version of the local run-manifest
     lineage and nothing else: `run-manifest.v1` and `v3` to `v12`
     today, under the current `run-manifest.v13`. The Looper's
     `run-manifest.v2` is not of that lineage. A record row has no reader
     to remove, since resume refuses any pinned manifest unequal to its
     recompile. Its bytes stay as the description of what older journals
     hold. Ruling 4's test binds a record row by its file and its loader,
     never by its `State` cell, so no other row leaves the window and
     retirement checks by being relabelled `record`.

   A family has one class, and every row of it carries that value. The
   criterion is the reader of a pinned copy. A family whose copies pinned
   into a run are read back at resume by the same reader that admits it
   live at launch is an input family, because that reader's labels are
   ruled as an input's. A family with any copy read back by a reader that
   the live path does not use, or read back only, is a journal family.
   The dispatch envelope is pinned and read back at resume
   (`crates/brokkr-runtime/src/engine.rs:653-655`) through the same
   `DispatchEnvelopeV2::verify` that reads it live at launch
   (`crates/brokkr-runtime/src/launch.rs:340-368`), so it is input. The
   dialect is a journal family: a dialect file is read at v3 only
   (`crates/brokkr-runtime/src/dialect.rs:262-268`), so v1 and v2 are read
   only as the pins older runs carry into their manifests, which a resume
   rehydrates (`crates/brokkr-runtime/src/realms.rs:1001-1004`). A realms
   map is pinned into a run too, but its live loader reads every label, so
   its pins are read by an input reader and the family stays input.

   Whatever reader is removed, every file under `contracts/` keeps its bytes, and
   is pinned by digest from the day it lands, not from the day its
   successor arrives.

   Applied to the seat record at 0.12, with N = 3: v1 (last written by
   0.7) and v3 (0.8) are already past the window, v2 was never dispatched,
   v4 (0.9) is kept through 0.12, and v5 (0.11) through 0.14. The build
   gives every row already past the window a `Reader until` of its own
   line, 0.12 today, so the first release after it removes them with v4.
   The dialect pins' last lines are measured by the build, not here.

   **Enforcement binding:** the table test from ruling 4 computes each
   journal `reader` or `retired` row's deadline and never reads it from
   the row. The
   deadline is the row's `Last written` line plus N, or the line the
   build lands in where that is later; the test holds that landing line
   as one constant, so it does not move with later releases. `Reader
   until` must equal the deadline. A cell shorter or longer than it
   refuses, naming the row and both lines. The one longer form admitted
   is `<line> (decision NNNN)`, matched whole, where decision NNNN's
   `Status:` reads `accepted`, nothing supersedes it, and it carries a
   header line reading exactly `Extends the reader of: contracts/<file>
   to <line>`, with the row's file and line. Any other form, a decision
   that does not carry the line, or one superseded, refuses. An
   `Extends the reader of:` line no row names is refused as unused
   (decision 0071 ruling 6), and the build adds the line to the
   decisions README's header vocabulary. `Last written` is bound to a
   typed source too, so moving it cannot lengthen a window:
   - a seat-record row's line is one `SeatRecordVersion::of_engine`
     dispatches to that row's version, and the next minor line it does
     not, so the line is the last that version was written under. v2's
     row reads `never`: `of_engine` selects v2 for no line from 0.0 to
     the workspace line, and its deadline is the landing line. A retired
     seat-record version keeps its variant and its `of_engine` arm, which
     the store answers with the refusal above, so a retired row's line
     stays bound to the same dispatch;
   - any other journal `reader` or `retired` row, today the dialect pins
     v1 and v2, has no loader that holds its line. The test admits such a
     row's `Last written` cell only from a closed list it holds, by exact
     file and cell, as ruling 2's list is held, and an entry stays when
     its row retires. The build measures those lines and names in its
     pull request the commit each was read from.

   The test then compares each `reader` row's `Reader until` with the
   workspace version in `Cargo.toml`, and fails at the version bump that
   passes it while the row is still `reader`. For seat-record rows the
   dispatch check above keeps the table and the store from disagreeing.

   A `retired (0.N)` row is bound to the same authority, never to its own
   cells alone.
   - A journal row keeps its `Last written` and `Reader until`, bound as
     above, and `Reader until` still equals the computed deadline. 0.N
     must be later than that deadline, so a reader removed inside its
     window refuses, naming the row, 0.N and the deadline. 0.N must not
     be later than the workspace version, so a row cannot record a
     removal the tree has not made.
   - An input row is retired only by an accepted decision that nothing
     supersedes and that carries a header line reading exactly
     `Retires the reader of: contracts/<file> in <0.N>, warned from
     <0.M>`, with 0.M earlier than 0.N. Such a line no row names is
     refused as unused (decision 0071 ruling 6), and the build adds the
     line to the decisions README's header vocabulary. While the
     workspace version is earlier than 0.N, the row stays `reader` and
     its `Reader until` reads `0.N (decision NNNN)`, matched whole
     against that line. From 0.M on, the test drives `brokkr doctor`
     against an input under the row's label and asserts its exact
     warning, naming the label to move to. From 0.N on, the row reads
     `retired (0.N)` with the same `Reader until`, and the version bump
     that reaches 0.N fails while it still reads `reader`. A decision
     not in the ledger, not accepted, or superseded, a line whose file
     or release differs from the row's, a 0.M not earlier than 0.N, or
     any other `Reader until` form, refuses, naming the row.
   - For both classes, no reader source admits a retired row's version:
     a `set` family's array omits it, the seat-record dispatch answers
     its lines with the refusal, and any other kind's loader, driven
     with an instance of it, refuses it.

   One tree cannot show that the 0.M release was cut after the retiring
   decision landed. The operator, accepting the decision, rules a 0.M
   not yet released, and the test reads no history.

   `crates/brokkr-store` gains a test that plants a `run/started` naming
   a retired line and asserts the exact refusal text. In
   `crates/brokkr-runtime/tests/frozen_contracts.rs`, the `contracts/`
   entries of `FROZEN` are held equal to the set of
   `contracts/*.schema.json`, so a file landed without its digest fails.
   `FROZEN`'s one other entry, `fixtures/evaluator/corpus.ndjson`
   (`frozen_contracts.rs:112`), stays pinned there and is outside the
   comparison, as is the policy table's digest (`:170`). The build folds
   in the two contract digests pinned beside it
   (`frozen_contracts.rs:593-603`) and pins the 14 files unpinned today.
   Removal controls: bump a planted row's line past its window and see
   the test fail naming the row; change `seat-record.v4`'s `Reader until`
   from 0.12 to 0.99 with no decision named and see it fail naming the
   row and the deadline 0.12; change it to `0.13 (decision NNNN)` naming
   an accepted decision without the `Extends the reader of:` line and see
   it refused, then add the line and see it pass; move
   `seat-record.v4`'s `Last written` from 0.9 to 0.10 and see it fail
   naming the row, since `of_engine` dispatches 0.10 to v5; change a
   dialect pin's `Last written` cell and see it refused as outside the
   list; delete one `FROZEN` entry and see it fail naming the file.
   Retired rows, each with the workspace version planted:
   - at 0.13, remove `seat-record.v5`'s reader and write its row
     `retired (0.13)`, and see it fail naming the row, 0.13 and the
     deadline 0.14, which shows an early retirement is refused;
   - at 0.13, write `seat-record.v4`'s row `retired (0.14)`, and see it
     fail as a removal the tree has not made;
   - empty a retired row's `Last written` or `Reader until`, and see it
     refused as unreadable;
   - at 0.13, drop `SCHEMA_V1` from `brokkr_core::realms::SCHEMAS` and
     write `realms.v1`'s row `retired (0.13)` with `Reader until`
     `0.13 (decision NNNN)`: with no such decision in the ledger, see it
     refused; plant the decision `proposed`, and see it refused; plant
     it accepted with `in 0.14, warned from 0.12`, and see it refused as
     the wrong release; plant it with `in 0.13, warned from 0.13`, and
     see it refused as a warning shorter than one line;
   - at 0.12, with `realms.v1` still `reader` under an accepted decision
     reading `in 0.13, warned from 0.12`, remove the doctor's warning for
     `forge.realms/v1`, and see the test fail naming the label.

4. **One lifecycle table, in `versioning.md`, covers every file in
   `contracts/` and is checked against the tree.** It takes the place of
   the guide's "other contracts" table. It has one row per file, plus the
   one label without a file that ruling 5 records, with these columns:
   `Version` (the label, or `family/vN` for a family without one),
   `File`, `Class` (`journal` or `input`), `Minted under` (ruling 1),
   `Writer`, `Reader`, `State`, `Last written` and `Reader until`.
   - `Writer` is `engine <path>`, `drivers`, `operator`, `Looper`,
     `shipped <path>` or `none`.
   - `Reader` is `none`, or the support source the check reads from the
     loader, one of a closed set of kinds. Each is a typed fact read as
     data or a loader driven, never text found in a file.
     - `set <item>`: a public array of the labels a loader admits by
       membership. `brokkr_core::realms::SCHEMAS`
       (`crates/brokkr-core/src/realms.rs:106`, read by the loader at
       `:548`) and
       `brokkr_runtime::dialect::SCHEMAS`
       (`crates/brokkr-runtime/src/dialect.rs:34`, read by `parse_pinned`
       at `:276`) are the two today. The family's `current` and `reader`
       rows equal the array, both ways.
     - `dispatch brokkr_store::SeatRecordVersion`: the row names its
       variant, that variant's `contract()` is the row's file, and
       `of_engine` selects it for the row's last-written line. v2's row is
       named directly and selected by no line, as
       `crates/brokkr-store/src/seat_record.rs:121-128` says.
     - `label <item>`: a public label constant the loader compares by
       equality: `brokkr_protocol::PROTO` (compared at
       `crates/brokkr-protocol/src/process.rs:376`),
       `brokkr_core::dispatch::DISPATCH_SCHEMA_V2` (`dispatch.rs:246`) and
       `RUN_MANIFEST_SCHEMA_V2` (`:478`), and
       `brokkr_core::policy::TABLE_SCHEMA_V2` (`policy.rs:229`). The
       constant equals the row's label.
     - `load <item>`: a public loader fed an instance of the row's
       version, which must load. The file-less phase-machine v1 row is
       one (ruling 5), and so is any version whose label is held in no
       public constant, such as the host file's serde rename
       (`crates/brokkr-runtime/src/admission/host.rs:50`).
     - `schema <item>`: an embedded validator at runtime whose copy is held
       byte-equal to the published file, today the tool dialect
       (`crates/brokkr-runtime/src/capabilities/dialect.rs:20`).

     Where a source is private to its crate, as the host reader
     (`host.rs:282`, `pub(crate)`) and the tool dialect's embedded copy
     (`pub(super)`) are, that row's check runs in the owning crate's
     tests, which read the table from the workspace root at run time. The
     build adds no `pub` item for the check (decision 0071 ruling 6). It
     fills each current row with the kind its loader supports, and a row
     no kind can bind is a finding it names, never a row bound by text.
   - `State` is `current`, `reader`, `record` or `retired (0.N)`, where
     0.N is the release that removed the reader. Exactly one row per
     family or lineage is `current`. Ruling 3 binds a `retired` row, and
     the check below binds a `record` row.
   - `Last written` is an engine line, or `never`, for a journal
     `reader` or `retired` row, and empty otherwise. Ruling 3 binds it.
   - `Reader until` is an engine line, or ruling 3's extended form, for
     a journal `reader` or `retired` row. For an input row it is
     `decision` while no retirement is ruled, and `0.N (decision NNNN)`
     for a `reader` or `retired` row whose retirement is ruled. It is
     empty otherwise. Ruling 3 computes or binds it; the row only states
     it.

   Rows are written by hand and checked. The guide's three stale
   sentences are corrected in the same slice.

   **Enforcement binding:** a new test module,
   `crates/brokkr-cli/tests/contract_lifecycle.rs`, registered in
   `tests/it.rs`, with these checks.
   - The set of `File` cells equals the set of `contracts/*.schema.json`.
     A new file with no row fails, naming it, and so does a row with no
     file. The one file-less row is admitted only as ruling 5 records it.
   - Every cell is read from its closed form, and a form the reader does
     not recognise is refused.
   - Every `Reader` cell other than `none` names a source of the closed
     set, and the check reads or drives it as its kind says. A path whose
     text merely names the version binds nothing. Every `engine <path>` or
     `shipped <path>` in `Writer` exists.
   - Exactly one `current` row per family or lineage.
   - A `record` row is admitted only for a file
     `contracts/run-manifest.v<N>.schema.json` of the local lineage, with
     N below that of the lineage's `current` row. The test holds the
     Looper lineage's file, `run-manifest.v2`, as one constant outside
     it. The row's `Class` is `journal`, its `Writer` and `Reader` are
     `none`, and its `Last written` and `Reader until` are empty. As ruling
     3 drives a retired row's version, the lineage's `current` row's reader
     source, driven with an instance of the record version, refuses it.
     Any other `record` row refuses, naming the row.
   - Ruling 3's window and retirement checks.

   Removal controls: delete a row and see the test fail naming the file.
   Relabel `seat-record.v5`'s row `record`, with `Reader` `none` and
   `Last written` and `Reader until` emptied, and see it refused naming
   the row, so its V5 dispatch cannot leave the 0.14 deadline by its
   `State` cell. At 0.13, drop `SCHEMA_V1` from
   `brokkr_core::realms::SCHEMAS` and relabel `realms.v1`'s row `record`
   the same way: the set check passes both ways, and the row is still
   refused naming it, so an input reader is never removed without ruling
   3's `Retires the reader of:` decision. Relabel `run-manifest.v2`'s row
   `record` and see it refused as the Looper lineage's file. Make the
   current run-manifest reader admit a `run-manifest.v12` instance and
   see the `run-manifest.v12` row fail.
   The behavioural ones each take a version out of a loader's accepted set
   and leave its label constant in place: drop `SCHEMA_V7` from
   `brokkr_core::realms::SCHEMAS` and see the check fail naming
   `realms.v7`; drop `SCHEMA_V1` from `brokkr_runtime::dialect::SCHEMAS`
   and see it fail naming `dialect.v1`; remove `of_engine`'s `V5_ENGINE`
   arm and see it fail naming `seat-record.v5`; change `PROTO`'s label and
   see the `driver-protocol.v1` row fail.

5. **`forge.phase-machine/v1` has no schema file, on purpose.** Its only
   instance, `policy/phase-machine.json`, is frozen. Its bytes are pinned,
   and the frozen evaluator corpus pins its behaviour through
   `crates/brokkr-core/tests/differential.rs`. Together these are a
   stronger contract than a schema. Its rule level is open by design, so
   a schema would either prove nothing or contradict the loader. Nothing
   writes v1, so a schema file would be a version with no writer (decision
   0071 ruling 6). The lifecycle table records the label in one file-less
   row: `File` `none (decision 0082 ruling 5)`, `Class` `input`, `Writer`
   `none`, `Reader` `load brokkr_core::policy::Machine::from_table`,
   `State` `reader`, `Last written` empty, `Reader until` `decision`. The kind is `load`, not
   `label`, because no production line compares a table with
   `TABLE_SCHEMA_V1` (research 0022, row 6): the loader reads v1 as every
   label other than v2.

   **Enforcement binding:** the file-less-row admission in ruling 4's
   test. The row's label must equal `TABLE_SCHEMA_V1` in
   `crates/brokkr-core/src/policy.rs`, and the frozen
   `policy/phase-machine.json`, whose label it is, must load through
   `Machine::from_table`. Removal controls: rename the label in a planted
   row and see the row refused; make `Machine::from_table` refuse any
   label other than v2 and see the row fail.

## Consequences

- Minting is slower by exactly one step. A slice that needs a new version
  waits for the ruling that mints it. A proposal may still carry its
  version on a branch.
- Seven files' authority, and the Looper pair's, becomes visible on the
  page and in the tree, and can only move towards a recorded ruling.
- At N = 3, if the build lands in the 0.12 line, the 0.13 release stops
  reading journals under seat-record v1 to v4. The operator's oldest journals are then
  verified by the engine that wrote them. Which of those journals still
  need verifying was not measured.
- An input label is never aged by a release. A realms map or host file
  under an old label keeps loading until a decision says otherwise, and
  then the doctor warns first.
- The double definition #361 measured stands. A contract is still a
  schema plus a hand-written loader, and seat records and tool dialects
  are still the only families validated by a schema at runtime. The
  table's `Reader` column shows which is which. Collapsing the two is not
  ruled here.
- Not ruled here: the phase-machine loader reads an unknown label under
  v1's rules (research 0022, row 6), and `brokkr init` writes
  `forge.realms/v3` or `v4` while v8 is current (row 11). The research
  names both as findings, classifies both `not-planned`, and leaves them
  unruled. Nothing here plans either.
- No contract, fixture, policy table or reference byte moves under any
  ruling here.

## Ruled (operator, in chat, 2026-10-12)

Each item below was asked of the operator, and each was ruled as
recommended.

1. **N, the window.** Recommended 3 minor lines, which keeps a version
   read for about a month at this year's cadence. The alternatives are 2,
   or no window before 1.0, which leaves every reader in place until then.
   **Ruled:** 3 minor lines.
2. **The table: written by hand and checked, or generated.** Recommended
   by hand and checked, the form of the decisions and research indexes.
   Generating it would need a typed source for each row's writer and
   reader that the tree does not have.
   **Ruled:** by hand, and checked.
3. **The test home for the minting rule.** Recommended in
   `decisions_index.rs`, which owns the ledger the rule reads, with the
   table check in `contract_lifecycle.rs`, and the rows whose reader is
   private checked in the owning crate. The alternative is one module
   for both, with the ledger reader shared through `tests/support/`.
   **Ruled:** the minting rule in `decisions_index.rs`; the table check in
   `contract_lifecycle.rs`; private readers checked in the owning crate.
4. **The nine pending rows.** For each, accept the decision, withdraw it,
   or give it a home in the tree: 0075 or an erratum to 0065 for
   `realms.v7`, and an erratum to 0068's 2026-10-10 addendum for
   `run-manifest.v13`. Recommended: give `realms.v7` the erratum to 0065,
   where PR #526 ported its tier, so it no longer waits on draft PR
   #486; give `run-manifest.v13` the erratum to 0068's addendum, its one
   home; rule the five rows under proposed decisions with each decision,
   not here; and leave the Looper pair listed, since it predates any
   minting rule. The alternative is one blanket acceptance of all nine
   in this decision, which would accept five proposals' content unread,
   or withdrawing a decision, so that its rows read `withdrawn` and its
   versions take ruling 3's window.
   **Ruled:** as recommended, with one row overtaken. Decision 0075 was
   accepted on 2026-10-12 and is on `main` (#486), so `realms.v7` is
   minted under 0075 and needs no erratum. `run-manifest.v13` takes the
   erratum to 0068's 2026-10-10 addendum. The five rows under proposed
   decisions are ruled with each decision. The Looper pair stays listed.
5. **Whether `enacted, pending acceptance` stays open for new versions.**
   Recommended closed, as ruling 1 rules. Leaving it open is option (b)
   above.
   **Ruled:** closed.
6. **The unknown phase-machine label.** Whether ruling 5 should also
   refuse a table whose label is neither v1 nor v2, or leave it unruled.
   Recommended unruled here, as research 0022 leaves it (row 6,
   `not-planned`), until a commission asks for it: ruling 5 records a
   label, and refusing an unknown one changes what the loader admits,
   with its own refusal text and tests. The
   alternative rules it here, so ruling 5's build also makes
   `Machine::from_table` refuse a label that is neither v1 nor v2.
   **Ruled:** unruled here, until a commission asks for it.
7. **What was not measured.** Which journals the operator still verifies,
   by engine line. Whether each older tag builds and which carry release
   archives. Whether any third-party driver or Looper deployment reads a
   specific version. Each bears on N. Recommended: rule N now, and have
   the build count the first, the journals still verified by engine
   line, before the first release that removes a reader, so the operator
   can confirm N or extend a row's window by ruling 3's form then. The
   other two stay unmeasured, because a refused journal names the
   engine to verify it with. The alternative holds N until all three
   are measured, which leaves every reader in place meanwhile.
   **Ruled:** N is ruled now; the build counts the journals still verified
   by engine line before the first release that removes a reader.
8. **The four addendum status lines and 0045's five keep lines.** The
   build also gives 0045 one `Keeps the minting of:` header line for each
   file minted under 0041 or 0043, the decisions it supersedes in part,
   and the operator confirms those five as well. The alternative is to
   read a supersession in part as leaving the minting standing, which
   reopens ruling 1's gap for any ruling a part supersedes.
   Ruling 1 reads an addendum's own
   `Addendum status:` line, not its heading. The build writes one line
   into each of the four minting addenda, transcribed from its heading:
   accepted for 0042's 2026-09-04 and 2026-09-06 addenda and 0068's
   2026-10-10 addendum, proposed for 0006's 2026-09-27 addendum. Each
   line is an erratum to a ruled text, so the operator confirms the four
   or rules otherwise. The alternative, a closed map in the test from
   each existing heading to its status, keeps the decisions untouched but
   leaves a second grammar for every later addendum.
   The same pull request lists every initial `Minted under` cell beside
   the research 0022 row it is transcribed from, and the operator
   confirms them too, since ruling 1's test reads a cell's ruling and not
   that ruling's text. Recommended: confirm all three sets in that pull
   request.
   **Ruled:** the three sets are confirmed in the build's pull request.
9. **A pointer that names several decisions.** Ruling 1 walks each
   decision a `Superseded by:` or `Superseded in part by:` pointer names,
   and admits a row when exactly one of them keeps its file. The
   decisions README makes a comma-separated list the pointer's legal
   form, and 0045 already carries one. The alternative refuses any
   pointer naming more than one decision as unreadable. Under it, the
   first time 0041 or 0043 is superseded in part again, every row it
   minted refuses, and only splitting the pointer gets through.
   **Ruled:** each named decision is walked; exactly one keeps the file.
10. **When a supersession's back-pointer lands.** The walk ends only at
   an accepted decision, and the decisions README adds the
   `Superseded by:` back-pointer in the superseding pull request. So a
   decision that supersedes a minting decision cannot land on `main` as
   a proposal, though every other decision, this one included, can. The
   recommendation is to keep that, because a proposal that strands a
   minted version should be ruled before it lands. The alternative adds
   the back-pointer only when the successor is accepted, and changes the
   README's rule to match.
   **Ruled:** kept: a decision that supersedes a minting decision lands
   accepted.
