# 0082 — A contract version has a lifecycle: it is minted under an accepted ruling, tabled with its writer and reader, and read for a stated window

Status: proposed (implementer, 2026-10-11)
Date: 2026-10-11

Built: unbuilt (#361)

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
only for the versions minted before this decision.

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
   A file added to `contracts/` names the ruling it was minted under.
   That is a decision whose `Status:` is `accepted`, or an addendum the
   operator has ruled inside an accepted decision. A proposed addendum to
   an accepted decision is proposed, and does not mint. A decision whose
   proposal is still open may describe a version, and its slice may build
   the version on a branch. The version lands on `main` only once the
   ruling that mints it is accepted.

   **Enforcement binding:** `crates/brokkr-cli/tests/decisions_index.rs`
   gains `every_contract_version_is_minted_under_an_accepted_ruling`,
   reading the `Minted under` cell of every row of ruling 4's table. The
   cell is one decision number, optionally followed by the date of the
   addendum that minted it (`0042 (2026-09-06 addendum)`). The decision
   must be in the ledger with status `accepted`. A named addendum must
   have a heading in that file opening `## Addendum — <date>`. A heading
   containing `proposed` refuses. One containing `ruled`, `ruling`,
   `accepted` or `confirmed by the operator` admits, and any other heading
   is refused as unreadable, never admitted. Removal control for the
   build: plant a row minted under a proposed decision, see the test fail
   naming the file and the decision, then restore.

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
   is then accepted fails the test until it does. A decision that minted
   a contract version never leaves this directory. If the operator
   declines it, it stays with status `withdrawn` and a dated note, and the
   withdrawal rules its versions' reader state. A version that lost its
   authority is read for ruling 3's window like any older version, or is
   retired at once where nothing ever wrote it. Two rows need a home in
   the tree before they can leave the list. `realms.v7` needs either 0075
   landing or a dated erratum to 0065, where PR #526 ported the
   provisional tier. `run-manifest.v13` needs a dated erratum to 0068's
   2026-10-10 addendum recording the version beside `forge.host/v1`. Both
   errata are the operator's to make. `forge.host/v1` stands as it is,
   under 0068's addendum of 2026-10-10, whose heading says `ruled`. A
   later Looper-lineage version needs an accepted ruling like any other.

   **Enforcement binding:** the closed list inside the test from ruling
   1. The build adds this exception to the decisions README's sentence on
   declined proposals. Removal control: add a tenth file with the marker, see it refused as
   outside the list, then restore.

3. **An older version's reader is kept for a stated window, and its bytes
   are never touched.** Every family is one of two classes.
   - A **journal** family is one the engine or its drivers write into a
     journal: the run manifests, the seat record, the event envelope and
     its payload extensions, the operator supersede, and the dialect pin.
     The reader of an older version is kept for N engine minor lines
     after the last line that wrote it (N is asked below, recommended 3).
     In the first release past the window the reader is removed, unless
     the operator rules a longer window for that row, and the row then
     names the ruling. After that, export, import verification and offline
     verification refuse a journal whose engine line no kept reader
     covers. The refusal names the engine line that reads it, so the
     operator verifies it with that engine. It is never read under a newer
     contract and never skipped.
   - An **input** family is one an operator or a counterpart writes and
     the engine reads live: the realms map, the host file, the dialect
     file, the tool dialect, the phase-machine table, the dispatch
     envelope and the driver protocol. No release ages it. An input
     label's reader is removed only under an accepted decision that names
     the release (decision 0071 ruling 6), and `brokkr doctor` names the label to move
     to for at least one minor line before that release.
   - A **record** row, an older local run manifest, has no reader to
     remove. Its bytes stay as the description of what older journals
     hold.

   Whatever reader is removed, every file under `contracts/` keeps its bytes, and
   is pinned by digest from the day it lands, not from the day its
   successor arrives.

   Applied to the seat record at 0.12, with N = 3: v1 (last written by
   0.7) and v3 (0.8) are already past the window, v2 was never dispatched,
   v4 (0.9) is kept through 0.12, and v5 (0.11) through 0.14. The build
   gives every row already past the window a `Reader until` of its own
   line, 0.12 today, so the first release after it removes them with v4.
   The dialect pins' last lines are measured by the build, not here.

   **Enforcement binding:** the table test from ruling 4 compares each
   `reader` row's `Reader until` line with the workspace version in
   `Cargo.toml`, and fails at the version bump that passes it while the
   row is still `reader`. For seat-record rows it also asserts that
   `SeatRecordVersion::of_engine` dispatches each row's last-written line
   to that row's version, so the table and the store cannot disagree.
   `crates/brokkr-store` gains a test that plants a `run/started` naming
   a retired line and asserts the exact refusal text. In
   `crates/brokkr-runtime/tests/frozen_contracts.rs`, `FROZEN` is held
   equal to the set of `contracts/*.schema.json`, so a file landed
   without its digest fails. The build pins the 14 files unpinned today.
   Removal controls: bump a planted row's line past its window and see
   the test fail naming the row; delete one `FROZEN` entry and see it
   fail naming the file.

4. **One lifecycle table, in `versioning.md`, covers every file in
   `contracts/` and is checked against the tree.** It takes the place of
   the guide's "other contracts" table. It has one row per file, plus the
   one label without a file that ruling 5 records, with these columns:
   `Version` (the label, or `family/vN` for a family without one),
   `File`, `Class` (`journal` or `input`), `Minted under` (ruling 1),
   `Writer`, `Reader`, `State` and `Reader until`.
   - `Writer` is `engine <path>`, `drivers`, `operator`, `Looper`,
     `shipped <path>` or `none`.
   - `Reader` is `loader <path>` (hand-written Rust), `schema <path>` (an
     embedded validator at runtime) or `none`.
   - `State` is `current`, `reader`, `record` or `retired (0.N)`. Exactly
     one row per family or lineage is `current`.
   - `Reader until` is an engine line for a journal `reader` row,
     `decision` for an input `reader` row, and empty otherwise.

   Rows are written by hand and checked. The guide's three stale
   sentences are corrected in the same slice.

   **Enforcement binding:** a new test module,
   `crates/brokkr-cli/tests/contract_lifecycle.rs`, registered in
   `tests/it.rs`, with these checks.
   - The set of `File` cells equals the set of `contracts/*.schema.json`.
     A new file with no row fails, naming it, and so does a row with no
     file. The one file-less row is admitted only for a label a reader
     constant names.
   - Every cell is read from its closed form, and a form the reader does
     not recognise is refused.
   - Every path in `Writer` and `Reader` exists, and its text names the
     row's label, its file name or its `family/vN` spelling, so a reader
     cited for a version is code that names that version.
   - Exactly one `current` row per family or lineage.
   - Ruling 3's window check.

   Removal controls: delete a row and see the test fail naming the file;
   point a reader at a path that does not name the version and see it
   refused.

5. **`forge.phase-machine/v1` has no schema file, on purpose.** Its only
   instance, `policy/phase-machine.json`, is frozen. Its bytes are pinned,
   and the frozen evaluator corpus pins its behaviour through
   `crates/brokkr-core/tests/differential.rs`. Together these are a
   stronger contract than a schema. Its rule level is open by design, so
   a schema would either prove nothing or contradict the loader. Nothing
   writes v1, so a schema file would be a version with no writer (decision
   0071 ruling 6). The lifecycle table records the label in one file-less
   row: `File` `none (decision 0082 ruling 5)`, `Class` `input`, `Writer`
   `none`, `Reader` `loader crates/brokkr-core/src/policy.rs`, `State`
   `reader`, `Reader until` `decision`.

   **Enforcement binding:** the file-less-row admission in ruling 4's
   test, with `TABLE_SCHEMA_V1` in `crates/brokkr-core/src/policy.rs` as
   the constant that names the label. Removal control: rename the
   constant's label in a planted row and see the row refused.

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
- Not ruled here, and named as follow-ups in research 0022: the
  phase-machine loader reads an unknown label under v1's rules (row 6),
  and `brokkr init` writes `forge.realms/v3` or `v4` while v8 is current
  (row 11).
- No contract, fixture, policy table or reference byte moves under any
  ruling here.

## Asked

1. **N, the window.** Recommended 3 minor lines, which keeps a version
   read for about a month at this year's cadence. The alternatives are 2,
   or no window before 1.0, which leaves every reader in place until then.
2. **The table: written by hand and checked, or generated.** Recommended
   by hand and checked, the form of the decisions and research indexes.
   Generating it would need a typed source for each row's writer and
   reader that the tree does not have.
3. **The test home for the minting rule.** Recommended in
   `decisions_index.rs`, which owns the ledger the rule reads, with the
   table check in `contract_lifecycle.rs`. The alternative is one module
   for both, with the ledger reader shared through `tests/support/`.
4. **The nine pending rows.** For each, accept the decision, withdraw it,
   or give it a home in the tree: 0075 or an erratum to 0065 for
   `realms.v7`, and an erratum to 0068's 2026-10-10 addendum for
   `run-manifest.v13`.
5. **Whether `enacted, pending acceptance` stays open for new versions.**
   Recommended closed, as ruling 1 rules. Leaving it open is option (b)
   above.
6. **The unknown phase-machine label.** Whether ruling 5 should also
   refuse a table whose label is neither v1 nor v2, or leave that to a
   follow-up.
7. **What was not measured.** Which journals the operator still verifies,
   by engine line. Whether each older tag builds and which carry release
   archives. Whether any third-party driver or Looper deployment reads a
   specific version. Each bears on N.
