# 0089 — Every readout reads a page of what the writer keeps: one summary row per run, a row per seat, and an index of every event

Status: proposed
Date: 2026-10-18

Built: unbuilt (#753)

## Context

Issue #753. The fleet (`brokkr tui` at its runs level, `brokkr runs`) reads
every event of every run to draw one screen. The operator's journal on
2026-10-18 (`the-forge/.forge/forge.db`, measured for the issue) was 1.1 GB,
745,281 events and 1,164 runs, with 95% of the events (672 of 754 MiB)
`effect/checkpointed`. An open `brokkr tui` (0.12.0) held 2.6 GB, and
`brokkr runs --json` took 6.7 s and 2.65 GB. The process floor is 5.5 MB
max RSS for `brokkr --help` and 6.5 MB for `brokkr runs` on an empty
journal. `sqlite3` read each run's last event over the `(run_id, seq)`
key in 0.06 s and 6 MB, for all 1,164 runs.

The operator, 2026-10-18, in chat, in order:

1. "yes, draft the summary-table decision, and add pagination to it. Then
   it can grow as much as it wants, we are still in the milliseconds."
2. "Also, 81mb memory for one page TUI is [nonsense], don't you think?"
   (one spelling corrected in brackets)
3. "not to mention 2.5gb+"
4. "ok, but let me challenge you a bit, think as if you live in 1980s -
   that TUI is something that could be rendered then. What was the top
   memory for a machine back then?"
5. "ok, about that decision, the backend data that with synthesize and
   design should be usable for the other views as well, so we make all the
   views blazing fast"

**The budget, from the screen.** A fleet page is a terminal screen. An
80×24 screen is 1,920 cells, and 30 to 50 summary rows of about 200 bytes
are about 10 KB. A VT100 drew such a screen from about 3 KB of memory, and
a 1980s PC had at most 640 KB. So what a page costs beyond the process
floor must be kilobytes, whatever the journal's size. The floor itself is
mostly mapped code and libraries. This decision therefore budgets **live
heap per page**, not process RSS, and holds RSS within about 1 MB of the
floor.

**What #744 bought, and what it leaves.** #744's open pull request writes
decision 0088, the console reads only what landed, off its input loop. It
keeps each run's fold between refreshes (its ruling 1) and reads on a
thread of its own (its ruling 4). Its commit f5a0800a measured a
generated journal of 734,484 events and 1,164 runs: the whole read took
3.48 s at 2.11 GB VmHWM, the first kept refresh 0.48 s at 81 MB, and an
idle refresh 17 ms reading 0 events. The first read's time went to text
read 0.72 s, envelope parse 1.27 s and chain hashing 1.80 s; the fold took
0.07 s. Its ruling 5 keeps every body verified on that first read,
because the chain's hash covers each body. So the first read still grows
with the journal. This decision's ruling 5 asks the operator to change
that at fleet level only.

**What every readout reads today.** Every listing reads the same way,
through `fleet::read_hearth` (`crates/brokkr-cli/src/fleet.rs:97`): it
lists the `runs` table (`Store::list_runs`,
`crates/brokkr-store/src/lib.rs:524`, unpaged, `ORDER BY created_at`),
then loads every run whole (`Store::load`, `lib.rs:626`, which verifies the
whole chain at `lib.rs:638`), folds it, and derives its residual findings
(`fleet.rs:124`). The store offers no summary and no page. Its keyed
reads each return one fact of one run: its head (`head_of`, `lib.rs:228`;
`Store::head_hash`, `:655`), its manifest (`:509`), whether it started here
(`:484`), its standing acknowledgement (`acknowledgement.rs:178`), and the
events after a seq (`load_after`, `crates/brokkr-store/src/read.rs:21`),
which only the engine's replay calls
(`crates/brokkr-runtime/src/engine/replay.rs:42`). None of them serves a
listing.

| Readout | Entry | What it reads |
|---|---|---|
| The fleet: `brokkr tui` runs level, `brokkr runs`, the many-hearth fleet | `tui_views` (`crates/brokkr-cli/src/lib.rs:1032`, `read_hearth` at `:1102`); `runs` (`crates/brokkr-cli/src/verbs/readouts.rs:239`) | every run whole, every 8 ticks of 250 ms (`crates/brokkr-cli/src/tui.rs:76`, `:81`) |
| The run view: TUI run level and detail pane, `inspect`, `seats`, `watch` | `run_view_of` (`readouts.rs:92`); `brokkr_view::run_view` (`crates/brokkr-view/src/lib.rs:2696`) | the run whole; its phase rail, decision trail, journal rows, notices, dashboard |
| The seat view: TUI participant level, `brokkr transcript` | `scan_participants` (`brokkr-view/src/lib.rs:1015`), `transcript_command` (`crates/brokkr-cli/src/lib.rs:1423`) | the run whole; each checkpoint's model, usage, cost and transcript reference; the transcript file on disk |
| `brokkr costs`, `brokkr compare` | `seat_costs` (`crates/brokkr-view/src/compare.rs:299`) | one or two runs whole |
| `brokkr analytics`, `brokkr graph` | `read_hearths` (`crates/brokkr-cli/src/verbs/analytics.rs:113`) | every run of every hearth whole |
| `brokkr ui` | `handle` (`crates/brokkr-cli/src/ui.rs:81`); `/api/runs` at `:100` | `/api/runs` every run whole; `/api/view/<run>` and the seat routes one run whole |
| Muninn | `dossier_of` (`crates/brokkr-cli/src/muninn.rs:230`) | every run whole (`:248`), and a full run view for every healthy run (`:292`) |
| `export`, `anchor`, `import`, `replay`, `ledger`, `resume` | `export_ndjson` (`lib.rs:643`) and the rest | one run whole, chain verified: this is their job |

The DSH console plugin (decision 0085, #568, another session's work)
reads `brokkr runs --json` and `brokkr inspect --json`
(`extensions/dsh/brokkr-console/lib/host/brokkr.js:41`, `:42`). This
decision changes none of its code, and ruling 6 keeps the shape it reads.

**Where the fold already holds what the fleet shows.** A fleet row
(`RunRow`, `crates/brokkr-view/src/fleet.rs:18`) is built by `run_row`
(`fleet.rs:634`) from a `RunEntry` (`brokkr-view/src/lib.rs:164`): the
`runs` row's feature and creation time, the last event's time, the fold's
`RunState` (`crates/brokkr-core/src/fold.rs:137`), the refusal when the
journal does not load or fold, the residual findings, and the
quarantine acknowledgement. Every value the row shows is a field of one of
those, listed in ruling 2. The engine holds the fold at each turn already:
its replay keeps the state at the head and folds only the suffix
(`replay.rs:27` to `55`, `fold_onto` at `fold.rs:419`). A checkpoint moves
no fleet fact: the fold's checkpoint arm only checks that the checkpoint
names the open effect (`fold.rs:549` to `551`).

**Who writes the journal.** Every row reaches `events` through one of two
inserts in `brokkr-store`: `seal_and_insert`
(`crates/brokkr-store/src/chain.rs:50`), which both the single append
(`append_once`, `lib.rs:303`) and the chained one (`chain_once`,
`chain.rs:108`) call, and `import_run`
(`crates/brokkr-store/src/import.rs:118`). A run row reaches `runs` at
`lib.rs:259` and `import.rs:191`. Their callers are the engine
(`crates/brokkr-runtime/src/engine.rs:598`, `:642`; its fenced append at
`crates/brokkr-runtime/src/engine/fence.rs:68` and `:80`; its checkpoint
flush at `crates/brokkr-runtime/src/engine/checkpoints.rs:344` to `409`),
the operator verbs (`crates/brokkr-runtime/src/engine/operator.rs:73`,
`:217`, `:365`) and `import` (`crates/brokkr-cli/src/verbs/exchange.rs:135`).
The seat-record fence already reads typed content inside the append's
transaction (`chain.rs:14` to `45`), so the transaction is where a derived
row belongs.

**What was measured for it, this session.** The operator's journal is
outside this seat's sandbox, so a scratch program under `.forge/`
(`rusqlite` 0.32.1, the workspace's) generated three journals in the
schema `MIGRATION_V1` writes (`crates/brokkr-store/src/schema.rs:19`),
with a candidate `run_summary` table of about 300 bytes a row and one index
`(standing, last_recorded_at DESC, run_id DESC)`, an earlier draft's key
that ruling 6 replaces with `lane` (re-measured below): 2,000 runs of 5 events
(10,000 events, 27 MB); 2,000 runs of 525 (1,050,000 events, 2.2 GB); and
20,000 runs of 50 (1,000,000 events, 2.2 GB). Checkpoints were 1,000,000 of
the second journal's 1,050,000 events and 1,407,600,000 of its
1,457,374,890 envelope bytes, heavier than the operator's. Release build,
on the workstation at load average 25 (`uptime`: 25.22, 16.14, 13.45).
What a page read cost, the visible page and one ahead (100 rows) plus a
head check of each row against `events`:

| Journal | Plan | Page and heads, warm, median of 9 | Rust heap peak | SQLite memory high-water | Process VmHWM |
|---|---|---|---|---|---|
| 2,000 runs, 10,000 events | `SEARCH run_summary USING INDEX run_summary_page (standing=? AND (last_recorded_at,run_id)<(?,?))` | 0.388 ms | 92,112 B | 342,488 B | 4,816 kB |
| 2,000 runs, 1,050,000 events | the same | 0.552 ms | 92,112 B | 342,488 B | 4,864 kB |
| 20,000 runs, 1,000,000 events | the same | 0.643 ms | 92,212 B | 342,488 B | 4,880 kB |

The reader's page cache was capped at 256 KiB (`PRAGMA cache_size =
-256`). The heap, high-water and VmHWM columns are each journal's first
measurement; the warm column is a later one's nine fresh connections. The
first read in a fresh process, with the operating system's cache cold for
the head rows, took 0.76 ms to 42.7 ms in seven of nine tries; two took
3.3 s and 3.9 s, one of them on the smallest journal, so they were the
loaded disk and not the journal's size. Walking every page by keyset, the
last page took 0.18 to 13.6 ms over nine walks, within the first page's
cold range, with a heap peak of 24,815 B for a 30-row page and 45,651 B
for a 50-row one. Counting the
older runs by the index alone took 0.115 ms for 1,880 rows and 2.624 ms
for 18,800 (`SEARCH ... USING COVERING INDEX`). The run with the most
events (525) read its 25 control events whole in 9.81 ms, 728,685 bytes;
through a partial index that skips checkpoints, in 0.11 ms, 24,885 bytes.
Building that index over 1,050,000 events took 21.43 s. Per append, one
transaction each and fsync off so the commit's own sync does not hide it,
over 20,000 appends: the event alone cost 16.0 to 18.0 µs; with a summary
row's `UPDATE` beside it, 19.7 to 23.7 µs; with a trigger that reads the
envelope's time by `json_extract`, 22.3 to 34.4 µs. With the store's
default sync, each commit took milliseconds and the three were noise
(653 to 12,328 µs over 2,000). The scratch was removed after.

**Re-measured under the ruled key, after review.** A second scratch program
(`rusqlite` 0.32.1, release build, load average 4.60) built the summary
table as ruling 2 states it, with ruling 6's generated `lane` column and
its index, ruling 3's acknowledgement and append triggers, and a seat
table of 12 rows a run, in four journals: 2,000 and 20,000 runs, each with
and without one wide row. Each held 50 refused runs unacknowledged, the
oldest by activity, behind the 45% of runs that were refused and
acknowledged (900 and 9,000); one wide row was a parked run with a
1 MiB feature and 10,000 residual findings (about 1.5 MB). A reader capped
its page cache at 256 KiB. VM steps are SQLite's own count for the
statement (`sqlite3_stmt_status`, `SQLITE_STMTSTATUS_VM_STEP`, through
`rusqlite`'s `Statement::get_status`), which grows with every row the
statement visits, returned or not.

| Read | 2,000 runs | 20,000 runs | Plan |
|---|---|---|---|
| 50 unacknowledged refusals, the acknowledgement read by key per row (the earlier draft) | 21,015 VM steps, 0.74 ms | 191,115 VM steps, 4.84 ms | `SEARCH s USING INDEX old_status_page (status_key=?)` and a correlated subquery per row |
| The same 50 from the `refused` lane | 912 VM steps, 0.088 ms | 912 VM steps, 0.059 ms | `SEARCH run_summary USING INDEX run_summary_page (lane=?)` |
| A first page, sections in order to 100 rows | 1,822 VM steps, 0.25 ms; Rust heap peak 42,235 B; SQLite high-water 363,288 B | 1,810 VM steps, 0.13 ms; 42,490 B; 359,584 B | each range `(lane=?)` or `(lane=? AND last_recorded_at<?)` |
| The parked page, 50 rows, the wide row among them | 912 VM steps, 0.070 ms; 34,996 B | 912 VM steps, 0.070 ms; 34,996 B | `(lane=?)` |
| The same page with no wide row | 34,015 B | 34,015 B | the same |
| A page deep in "older" by cursor | 912 VM steps | 912 VM steps | `SEARCH run_summary USING INDEX run_summary_page (lane=? AND (last_recorded_at,run_id)<(?,?))` |
| Every run's lane and residual digest, streamed and counted | 0.79 ms; heap 732 B | 4.7 ms; heap 732 B | a scan, by design |
| Every seat row's tokens and cost, streamed and summed by phase and model | 24,000 rows, 9.1 ms; heap 2,769 B | 240,000 rows, 57 ms; heap 2,769 B | a scan, by design |

So the post-filter's cost grew with the acknowledged refusals ahead of the
page, nine-fold for ten-fold as many, while the lane's stayed at 912 steps.
The wide row added 981 B to its page, its 1,024-byte first line against a
43-byte one, and no time: the page selects no lossless column, and the
lossless columns sit after the bounded ones, so SQLite does not read the
row's overflow pages. An append to an acknowledged refused run moved its
row to the `behind` lane with `acknowledged` 0, through the triggers alone.
The scratch was removed after.

**The alternatives weighed.** (1) #744's kept read alone: the idle
refresh is already free, but the first read and its memory still grow
with the journal (81 MB, 0.48 s on 734,484 events). (2) A cache file
beside the journal: a second file to keep in step, outside the
transaction. (3) A service that holds the fleet in memory: decision 0003
rules one binary, no services. (4) SQL views over `events`: a view
recomputes over the events it selects, so it is a whole read written in
SQL. (5) One projection per readout: seven tables for one set of facts,
against decision 0071 ruling 5. This decision takes a small, derived read
model the writer keeps in the journal's own file and transaction.

**What this would touch, once accepted.**

- **0001 and 0003.** They stand: the journal's events stay the record. The
  read model is derived, never exported, never hashed, and rebuilt from
  the events whenever it disagrees with them.
- **0013 and 0071 ruling 7.** They stand: a projection stores the fold's
  facts, and `brokkr-view` keeps deriving every displayed fact from them
  (standing, verdict text, title, need, section).
- **0014 and 0026 ruling 5.** They stand: a reader opens the journal
  read-only and writes nothing, and each hearth keeps its own model.
- **0029.** It stands: the projection is written in the fenced append's
  own transaction, so a fence that fails writes no row of it either
  (`lib.rs:316` to `317`).
- **0088**, in #744's open pull request. Its kept read becomes the
  fallback (ruling 8), and its ruling 5 is what ruling 5 here puts to the
  operator. When both are accepted the pointer pair is added in the pull
  request that accepts this one, as the index requires
  (`crates/brokkr-cli/tests/decisions_index.rs`, `pointers_are_paired`).
- **0082 (#361).** No contract version is minted: no event, export or
  manifest changes (ruling 8).

## Rulings

1. **The read model is three projections, derived from the events and
   kept in the journal's file.** Each is a table the writer keeps in the
   transaction that writes the events it derives from:

   - `run_summary`: one row per run, the fleet's whole account of it
     (ruling 2);
   - `seat_summary`: one row per participant of an attempt, a seat, a
     panel member or a sequence step, keyed `(run_id, attempt_id,
     member)`: its seat, phase, outcome, model, effort, provider, boundary,
     token counts, reported cost, item and turn counts, transcript
     reference, and its first and last checkpoint's seq and time;
   - `event_index`: one row per event, keyed `(run_id, seq)`: its type,
     attempt, member and time, with an index `(run_id, attempt_id,
     member, seq)` for a seat's checkpoint stream and a partial index on
     `(run_id, seq)` over every type but `effect/checkpointed` for a run's
     trail.

   Which readout reads which is ruling 7. Nothing else reads a
   projection's row as a fact: the engine decides on its own fold, never
   on a row (decision 0071 ruling 7). Binding: a layering test in the
   build's first slice that no `brokkr-runtime` decision path names a
   projection's reader.

2. **The summary row holds exactly what the fleet shows, as the fold
   states it.** Each column, with what reads it and where the value comes
   from today:

   | Column | What the fleet shows from it | Its source |
   |---|---|---|
   | `run_id` | the id; the key | `runs.run_id` |
   | `feature_line`, `created_at` | the title, which `title` derives at read (`fleet.rs:457`) from the feature's first line that says anything, and the age | the `runs` row (`lib.rs:528`): that first line, cut at a character boundary to at most 1,024 bytes |
   | `status` | the status, and through `standing_of` the standing and the section (`fleet.rs:585` to `593`) | `RunState.run.status()` (`fold.rs:142`) |
   | `phase` | the phase; a stop at the table's stop phase against an operator's (`fleet.rs:590`) | `RunState.phase` (`fold.rs:143`) |
   | `park_reason` | the verdict's reason (`fleet.rs:577` to `580`) | `RunState.run.park_reason()` |
   | `rule_id`, `rule_from`, `rule_routes` | the verdict's rule and text, and whether a ruling parked the run (`fleet.rs:597` to `607`) | `RunState.last_decision` (`fold.rs:163`): its rule id, its phase, whether `next` is present |
   | `hire_seat`, `hire_failed_attempts` | the seat at work and its attempt (`fleet.rs:669` to `691`) | `RunState.run.cursor()` at `ExecuteEffect` or `EffectInFlight` |
   | `residual_open`, `residual_count` | the residual cell, the highest open severity (`open_severity`, `fleet.rs:623`), and how many findings stand open | the run's residual findings with their supersede marks (`brokkr_core::residual::residuals`, `crates/brokkr-core/src/residual.rs:111`; `residual_findings`, `brokkr-view/src/lib.rs:855`) |
   | `refusal_seq`, `refusal_words` | a quarantined row's `?` and its detail (`fleet.rs:31` to `37`) | the `FoldError` and its seq (`fold.rs:169` to `205`), only when the fold refuses; the words cut to at most 512 bytes |
   | `folded_seq`, `folded_hash` | the seq column; ruling 5's check | `RunState.seq`, `RunState.last_hash` (`fold.rs:139` to `140`) |
   | `head_seq`, `head_hash`, `last_recorded_at` | the age, the section and staleness (`fleet.rs:272` to `287`, `440` to `447`) | the last event's seq, hash and `recorded_at` (`crates/brokkr-cli/src/fleet.rs:55`) |
   | `acknowledged` | whether a quarantined run's acknowledgement stands at its head (`need`, `fleet.rs:418` to `424`) | an acknowledgement made at the row's head (`acknowledgement.rs:51` to `58`); ruling 3's triggers keep it |
   | `lane` | nothing: the index key ruling 6 reads a range by | generated from the columns above (ruling 6) |
   | `feature`, `residuals`, last in the row | `--json`'s lossless feature and list, and nothing on a page | the `runs` row; the residual findings as `residual_findings` derives them |

   **A row is bounded, so a page is.** Every column a page selects is
   bounded: identifiers, words, integers and hashes, the 1,024-byte first
   line, the 512-byte refusal words, and two numbers for the residuals. A
   bounded row is at most about 2 KiB, so a page of 100 rows is at most
   about 200 KiB whatever a run holds. The lossless `feature` and
   `residuals` are the row's last columns and no page selects them, so
   SQLite reads neither, nor their overflow pages: one parked run with a
   1 MiB feature and 10,000 findings added 981 B to its page and no time
   (Context, re-measured). `brokkr runs --json` and the run view read them
   by key, one row at a time as each is written out, so their memory is a
   page of bounded rows and the largest row's lossless fields: bounded by
   the run, never by the journal. `title` is derived from `feature_line` as
   it is today from the feature; the two agree whenever the first line is
   at most 1,024 bytes, or its first 1,024 bytes are at least as wide as
   the widest title a surface asks for. A line made mostly of zero-width
   marks is the one case a title would come out shorter, and the test of
   ruling 4 pins it. `residual_open` is the highest open severity, as
   `open_severity` derives it today. Decision 0071 ruling 7 asks for one
   derivation, so slice 1 moves that function, with the operator's
   closing marks it reads (`brokkr-view/src/lib.rs:803`), into one pure
   function in `brokkr-core` that the writer and `brokkr-view` both call.

   `status` holds `Status::as_str()`'s word, or nothing for a refusal; a
   reader parses it into `Status` at the edge and refuses a word it does
   not know, and parses `residual_open` into `Severity` alike.
   `residuals` is the canonical JSON of a closed list, read into its type
   with `deny_unknown_fields` at the edge (decision 0071 ruling 3). The
   acknowledgement itself, who and why, stays in its own table
   (`crates/brokkr-store/src/acknowledgement.rs:26`), read by key for an
   acknowledged row on the page: it is bookkeeping beside the chain and
   lapses when the journal moves (`fleet.rs:108` to `111`). The row
   carries only whether one stands at its head, which is what places it.
   A store refusal (`DoesNotLoad`) is never a row's fact: a writer wrote a
   chain that loads, and a reader that meets one says so live (ruling 5).

3. **The writer keeps the model, in the transaction of every insert.**
   Every writer is the engine's code, the driving engine, the operator
   verbs in `brokkr-runtime` and `import`, and every insert passes through
   the four sites above. Two parts, split by what each needs:

   - **What the row itself states, SQLite keeps.** A trigger after each
     insert into `runs` adds the run's summary row with nothing folded,
     and a trigger after each insert into `events` moves its
     `head_seq`, `head_hash` and `last_recorded_at`, sets `acknowledged`
     to 0, and adds its `event_index` row, reading the type, attempt,
     member and time from the envelope by `json_extract`, as the store
     already reads a manifest's engine in SQL (`ENGINE_OF_RUN`,
     `lib.rs:280`). A trigger after each insert into
     `quarantine_acknowledgements` sets `acknowledged` to 1 on the run's
     row when the acknowledgement's head is the row's head, the same test
     its standing query makes (`acknowledgement.rs:51` to `58`). So an
     acknowledgement stands on the row exactly while it stands in its
     table, and lapses with the next append, as today. A trigger is part
     of the journal file, so every writer keeps these, an older binary's
     included, and `brokkr acknowledge` (`Store::acknowledge_quarantine`,
     `acknowledgement.rs:162`) gains no code.
   - **What the fold states, the writer keeps.** An append takes the fold
     its caller holds: the engine's replay state, the state an operator
     verb decided on (`operator.rs:81`, `:87`), the verified run `import`
     folds (`verify_export`, `import.rs:351`). Inside the transaction the
     store checks that the fold stands at the head it found, carries it
     over the sealed events
     with `fold_onto`, and writes the run's folded columns and its seat
     rows from brokkr-core's pure projections of the result. A caller
     with no fold, or a fold at another head, writes nothing folded: the
     row falls behind its head, its `lane` becomes `behind`, and ruling 6
     reads that run whole before placing it. The projections are pure
     functions in `brokkr-core`, so the writer and every reader derive
     them one way.
   - **A writer catches up what fell behind.** Every writable open by a
     current binary refolds the `behind` lane, one run per transaction,
     in the backfill's form (ruling 8), so a lagging row lasts until the
     next current writer opens the journal.

4. **The summary equals a fresh fold of every run, and a test says so.**
   The binding is `every_run_reads_as_its_fold`: after every engine
   scenario test in `brokkr-runtime`, and over every exported journal under
   `fixtures/journals/` imported into a fresh store (read, never
   regenerated), it folds each run whole and asserts its summary row,
   seat rows and index rows equal the projections of that fold, column for
   column. In production, `brokkr doctor` gains the same comparison over a
   journal, one run at a time so its memory is the largest run's, and
   names each run whose row stands at its head yet disagrees with the
   fold, with the first column that differs, as a failure. Such a row is
   a bug or an edit, never a lag; a lagging row is ruling 6's.

5. **Integrity without a full read.** For each row on the page and the
   one ahead, a reader reads that run's last event by its key (`head_of`,
   `lib.rs:228`, `ORDER BY seq DESC LIMIT 1` on `(run_id, seq)`) and
   compares its seq and hash with `folded_seq` and `folded_hash`. When
   they agree the row is shown. When they do not, the reader reads that
   run whole, chain verified, folds it and shows the fold's answer, never
   the row's: a run read whole that fails is quarantined as today.

   What this catches: a row behind its head (an append by an older binary,
   a raw insert, a writer with no fold); a row whose seq and hash name no
   event or another event; a head event replaced, whose stored hash then
   differs. What it **no longer catches at fleet level**: an edit to a
   row's folded columns that leaves its seq and hash standing, and a
   corrupt or edited event below the head, which today's fleet read
   quarantines. That is a weaker refusal at fleet level. The argument for
   it: the fleet admits nothing, and every verb that acts on a run reads
   it whole with its chain first (`resume`, the operator verbs, `export`,
   `anchor`, `import`; `Store::load`, `lib.rs:626`), as does opening the
   run (ruling 7); the engine's own replay already trusts a prefix it
   verified (`replay.rs:21` to `25`), and #744's kept read never re-reads
   one; the chain is an unkeyed hash, so whoever can edit a row can
   recompute it, and what the chain refuses is corruption, which the
   doctor's sweep and every opened run still find. The full check moves to
   where a run is opened and to `brokkr doctor`; it does not leave.

6. **The fleet reads a page by keyset, and every page costs the same.**
   The summary is indexed `(lane, last_recorded_at DESC, run_id DESC)`.
   `lane` is a generated column, so SQLite keeps it from the row's own
   columns whoever writes them:

   | `lane` | When |
   |---|---|
   | `behind` | `folded_seq` is not `head_seq`, or nothing is folded and nothing refused |
   | `refused` | a refusal, and `acknowledged` is 0 |
   | `acknowledged` | a refusal, and `acknowledged` is 1 |
   | `parked` | `status` is `awaiting_operator` |
   | `running` | `status` is `running` |
   | `finished` | any other status |

   The order of the table is the order of the test: a row behind its head
   is `behind` whatever it last folded to. A lane is not a displayed fact.
   It only says which range a row is read from; `brokkr-view` still places
   every row it is handed by `section_of` (`fleet.rs:272`), from the row's
   facts.

   A page is the sections in `Section::ORDER` (`fleet.rs:241`), each one
   or more ranges of that index with `LIMIT`, read in order until the page
   and the one ahead are full:

   - **"needs you"**: first the `behind` range (below), then the
     `refused` range, the `parked` range, and the `running` range older
     than the staleness bound (`fleet.rs:158`);
   - **"running"**: the rest of the `running` range;
   - **"last 24h"** and **"older"**: the `finished` range either side of a
     day ago (`fleet.rs:141`), with the `acknowledged` range merged into
     "older" by the reader, as `section_of` lists an acknowledged run
     there however recent (`fleet.rs:279` to `281`).

   Every range is an equality on `lane` and at most one bound on
   `last_recorded_at`, so it visits only the rows it returns: 912 VM steps
   for 50 refusals behind 900 or 9,000 acknowledged ones, where reading the
   acknowledgement per row visited 21,015 and 191,115 (Context,
   re-measured). `now` is the reader's argument, as today. A cursor is the
   last row's `(range, last_recorded_at, run_id)`, typed and parsed at the
   edge, so page N reads what page 1 reads.

   **A row behind its head is folded before it is placed.** Its folded
   columns are from an older head, so its lane says nothing of where it
   belongs: a run parked at seq 11 by an older binary, summarized running
   at seq 10, is `behind`, never `running`. The reader reads the `behind`
   range first on every page, reads each of its runs whole, chain
   verified, as #744's kept read does and keeps it between refreshes,
   and hands `brokkr-view` the fold's row, which `section_of` places like
   any other; a run whose read refuses is quarantined, as today. At most
   a page and one ahead of `behind` runs (100) are read on one refresh, by
   last activity; any beyond them are counted by the index, and "needs
   you" prints them as one line, so a lagging run is never placed by its
   stale status and never dropped silently. The `behind` lane is empty
   whenever every writer is current and the backfill has finished
   (rulings 3 and 8); when it is not, a page costs the runs it reads
   whole, bounded by those runs and never by the journal, and ruling 9's
   budgets are stated for an empty `behind` lane.

   - Within a section the order becomes last activity, newest first, then
     run id. Today it is creation, newest first (`fleet.rs:693` to `697`
     over `lib.rs:528`).
   - The console reads the visible page and one ahead, scrolls by keyset,
     and reads the last page by the reverse order. The older runs' count
     it prints (`crates/brokkr-cli/src/tui/panes.rs:224` to `225`) is an
     index-only count, the one fleet cost that grows with the number of
     runs (2.624 ms at 18,800, measured), never with events.
   - `brokkr runs` with no flag prints every run as today, read and
     written a page at a time, so its memory is one page's; its JSON shape
     is today's. `--limit N` prints one page and `--after CURSOR` the page
     after a cursor, and their JSON gains `next` (the cursor, or null at
     the end), which moves `VIEW_VERSION` from 23 to 24
     (`brokkr-view/src/lib.rs:128`). `/api/runs` takes the same two.

7. **Every readout reads a projection, and states its page and its
   bound.** Every page costs milliseconds and kilobytes of live heap,
   whatever the journal's size; a readout that needs a run's whole history
   says so, and is bounded by the run.

   | Readout | Reads | Keys | Page | Bound |
   |---|---|---|---|---|
   | The fleet: runs level, `brokkr runs`, `/api/runs`, the many-hearth fleet, Muninn's queue | `run_summary`; the acknowledgement by key | ruling 6's index; `head_of` per row | the visible rows and one page ahead, per hearth | ruling 9's numbers |
   | The run view: run level and detail pane, `inspect`, `seats`, `watch`, `/api/view/<run>` | the run's summary row, its trail through `event_index`'s partial index and the events by key, its seat rows | `(run_id, seq)` | the trail whole; journal rows a page at a time by seq | the run's control events, about 5% of its events: 25 of 525 read in 0.11 ms, measured |
   | Opening a run | the run's chain, verified streaming, one envelope held at a time | `(run_id, seq)` | none: a whole read | the run: at #744's rate, about 2.5 µs an event to hash, so about 18 ms for the 7,510-event run the benches size by (`crates/brokkr-cli/benches/budgets_lib.rs:23`) |
   | The seat view: participant level, `brokkr transcript`, `/api/presentation`, `/api/transcript` | the seat row; its checkpoints through `event_index`'s stream index | `(run_id, attempt_id, member, seq)` | 50 checkpoints, newest first | a page of checkpoints; the transcript file keeps its own caps and heap budgets (`quality/heap-bytes.json`) |
   | `brokkr costs`, `brokkr compare` | the seat rows of one or two runs; compare's trails | `(run_id, ...)` | the run's seats | the run's attempts |
   | `brokkr analytics`, `brokkr graph`, Muninn's dossier | `run_summary` and `seat_summary`, streamed, one row held at a time | the keys above | an aggregate, not a page | heap: the aggregate's accumulators, at most 256 KiB, independent of the runs; time: the number of runs and seats, never events (ruling 9's exception); a dated window reads one range |
   | `export`, `anchor`, `import`, `replay`, `ledger`, `resume`, the engine's first turn | the run whole, chain verified | `(run_id, seq)` | none | the run |
   | `brokkr doctor`'s sweep | every run whole, one at a time | `(run_id, seq)` | none | the largest run in memory; the journal in time |

   So three projections serve seven readouts: the fleet, Muninn's queue
   and the aggregates share `run_summary`; the run view, the seat view,
   costs, compare and the aggregates share `seat_summary`; the run and
   seat views share `event_index`.

8. **Migration and fallback; no contract moves.**

   - **The schema version stays 1** (`DATABASE_SCHEMA`, `schema.rs:12`).
     The precedent is the store's own: additive tables and columns did
     not move it, because an older binary reads a migrated journal as
     before (`schema.rs:70` to `74`, `:181`). Here an older binary would
     also write past the model; the triggers of ruling 3 keep its
     presence, head and index rows, and ruling 5 reads its folded columns
     whole. Moving the version would refuse such a binary the journal
     outright, reading included (`schema_supported`, `schema.rs:320`;
     `open_read_only`, `lib.rs:406`).
   - **A writer migrates.** The open's repair path (`schema.rs:165`) gains
     a fifth additive repair: the tables, indexes and triggers under one
     short immediate transaction, then a backfill that reads one run at a
     time outside the lock and writes its rows in a transaction of its
     own, so the write lock is held for one run's rows at most, never the
     journal's. A meta key records the backfill finished, and a backfill
     cut short resumes where it stopped. Its memory is the largest run's;
     its time is about the whole read once (3.48 s on #744's generated
     journal).
   - **A read-only reader falls back.** On a journal with no model, or one
     whose backfill has not finished, a reader that cannot write
     (decision 0014; `open_read_only`, `lib.rs:394` to `412`) reads the
     fleet with #744's kept read and writes nothing. On a migrated
     journal, a row behind its head is read whole for that run alone.
   - **No contract or manifest moves.** The model is store bookkeeping,
     like the sidecar columns (`schema.rs:53` to `57`): no event carries
     it, no fold reads it, an export is events only (`lib.rs:643` to
     `652`), and an import rebuilds it. `event-envelope.v1`, the run
     manifest and the seat record are unchanged, so decision 0082's
     lifecycle mints nothing. `VIEW_VERSION` is a view model's version,
     and only ruling 6's `next` moves it.

9. **The budgets, and the tests that hold them.** On a generated journal
   of at least 1,000,000 events and 2,000 runs, and the same between
   10,000 and 1,000,000 events:

   - **Time.** The first fleet page, open to painted frame, within 50 ms
     on a warm cache: measured 0.39 to 0.64 ms for its read. Wall clock is
     the machine's, so `scripts/measure-budgets.sh` records it and no
     required check gates it, like the bootstrap budgets
     (`.github/workflows/ci.yml:551`). The gate on time is the Callgrind
     form (`ci.yml:715` to `723`): a `fleet_page` bench beside the fold's
     in `budgets_lib.rs`, at both sizes, against the base.
   - **Live heap.** At most 256 KiB of Brokkr's heap to read, derive and
     paint one page and the one ahead: the read alone measured 92,112 B
     at both sizes, and 42,235 to 42,490 B under the ruled key. The
     binding is a dhat test of its own,
     `crates/brokkr-cli/tests/heap_fleet.rs`, in the form of
     `heap_claude.rs` (dhat allows one profiler per process), with its
     budget in `quality/heap-bytes.json` at the measured peak plus a
     tenth, never above 256 KiB, and an assertion that the peaks at
     10,000 and 1,000,000 events are equal to the byte. Its journals carry
     the rows that stretch a page: one run on the page has a 1 MiB
     feature and another 10,000 residual findings, and the test asserts
     the page's peak equal to the byte to the same page with those runs'
     first lines cut to exactly 1,024 bytes and one finding each, so the
     row's bound (ruling 2) is what holds the budget.
   - **SQLite's share.** A fleet reader caps its page cache at 256 KiB,
     so SQLite's memory stays within about 350 KB (342,488 B measured).
   - **RSS.** Within 1 MiB of the floor, `brokkr runs` on an empty journal:
     at most 7.5 MB max RSS on the generated journal.
   - **Independence.** Each range's plan is pinned to the exact text
     `EXPLAIN QUERY PLAN` gives for it, `SEARCH run_summary USING INDEX
     run_summary_page (lane=?)` or with `AND last_recorded_at<?`, `>?` or
     the cursor's `(last_recorded_at,run_id)<(?,?)`, with no `SCAN` and no
     temporary B-tree, in a `brokkr-store` test. A plan is not enough on
     its own: a range with a residual filter keeps the same plan text and
     still visits every row it filters out, as the acknowledgement read
     per row did. So the same test binds **visited rows**: it reads the
     first page and a page deep by cursor at 2,000 and at 20,000 runs,
     45% of them acknowledged refusals ahead of 50 unacknowledged ones,
     and asserts each statement's VM-step count
     (`Statement::get_status(StatementStatus::VmStep)`) equal at both
     sizes, to the step. Its removal control is a page that reads the
     acknowledgement per row again, whose count grows (21,015 to 191,115,
     measured).
   - **The ranges are the sections.** `every_page_is_its_sections`, in
     `brokkr-cli`: over every fixture journal and the generated one, at a
     `now` on each side of every staleness and 24-hour bound, the rows the
     paged read returns for each section, in order, equal the rows
     `brokkr_view::sections` gives over the same runs read whole. This
     holds the lane table to `section_of`, so the two never derive a
     section differently.
   - **A lagging row is placed by its fold.** `a_run_behind_its_summary_is_placed_by_its_fold`,
     in `brokkr-cli`: a run summarized running at seq 10 and recently
     active, then parked at seq 11 by an append that writes nothing
     folded (an older writer's path), lists under "needs you" as parked;
     and with 150 runs behind, the page reads 100 of them whole and
     prints the rest as one counted line. Its removal control is a page
     that skips the `behind` range, where the parked run lists under
     "running".

   The generated journal is one builder in `brokkr-store`'s test support,
   shared by these tests, with short bodies, since a page reads no body;
   the measurement script grows its bodies to about 1 KiB.

   **The other readouts' budgets**, each bound in its slice (ruling 11) in
   the same forms, a dhat test with its budget in
   `quality/heap-bytes.json`, a pinned plan and a Callgrind bench:

   | Readout | Time | Live heap | Bound by | Binding |
   |---|---|---|---|---|
   | The run view, its first screen | 50 ms | 256 KiB and about 1 KiB per control event of the run (24,885 B for 25, measured) | the run's control events | `heap_run_view.rs`: the same run in a 10,000- and a 1,000,000-event journal, peaks equal to the byte; the trail's plan on the partial index |
   | The seat view, a page of 50 checkpoints | 50 ms | 256 KiB and the largest checkpoint body on the page, each parsed into its row and dropped | the page's checkpoints | `heap_seat_view.rs`, the same two journals, peaks equal; the stream index's plan |
   | `costs`, `compare` | 50 ms | 256 KiB for two runs of 100 seat rows each, about 200 B a row estimated | the runs' seats | `heap_costs.rs`, the same two journals, peaks equal |
   | Opening a run, chain verified | about 2.5 µs an event | one envelope at a time, at most 256 KiB and the largest envelope | the run's events | `heap_open.rs`: a 7,510-event run, peaks equal in both journals |
   | The aggregates: `analytics`, `graph`, Muninn's dossier | 250 ms at 20,000 runs and 240,000 seat rows (measured 4.7 and 57 ms) | 256 KiB of accumulators, independent of the runs (732 B and 2,769 B measured at both sizes) | the runs and seats, never events | `heap_aggregates.rs` at 2,000 and 20,000 runs, peaks equal to the byte; a `fleet_aggregate` Callgrind bench at both sizes, whose instruction count grows with the rows and never with events |

   **The aggregates are the one exception, and it is put to the
   operator.** Each answers a question about every run: what the fleet
   cost, which models ran, which crossings Muninn's queue holds. A page of
   runs cannot answer it, so its time grows with the number of runs and
   seats. Its heap does not: rows are streamed and dropped, and only
   accumulators keyed by a bounded group (a phase, a model, a realm) are
   held. Its time never grows with events, and a dated window
   (analytics' day, week or month, `Range`,
   `crates/brokkr-view/src/analytics.rs:39`) reads one range of the index.

10. **Checkpoint bodies stay where they are.** They are 95% of the
    operator's events (672 of 754 MiB) and 96.6% of the generated
    journal's envelope bytes. Three options:

    - **Stay inline** (recommended). The read model never reads a body
      save on a seat's page of checkpoints and a whole read; ruling 7's
      trail read 24,885 of a run's 728,685 bytes. What stays is the file's
      size, and a whole read, which hashes every body wherever it lives.
    - **A side table the chain still covers.** Each checkpoint's row in
      `events` would hold its envelope without the body, and the body a
      row of its own, rejoined to hash, verify and export. `events` is
      append-only by trigger (`schema.rs:38` to `43`), so an existing
      journal keeps both forms or is rewritten into a new file; export,
      import and verification gain a rejoin. It would speed what the read
      model has already made cold.
    - **Coalesced.** Fewer, larger checkpoint events: a change to what is
      journaled, against decision 0034's per-turn record, and a new
      contract version.

    A move is a ruling of its own, on a measured need such as the file's
    size or an export's time.

11. **The build, in slices, each with its acceptance.**

    - **Slice 1, the fleet** (the first). `run_summary`, its triggers and
      its index; the writer's fold through the append; the backfill;
      the `lane` key and the acknowledgement facet; the `behind` range
      read first and its catch-up at a writer's open; ruling 5's check;
      ruling 6's paging in the console, `brokkr runs`, `/api/runs` and
      Muninn's queue; ruling 4's test and doctor check; ruling 9's fleet
      heap test with its wide rows, its plan and visited-row test, its
      sections test, its lagging-row test and its bench. Acceptance: the
      fleet's budgets hold on the generated journal; `brokkr runs --json`
      lists every fixture journal's runs with the same rows as today,
      ordered as ruled; an unmigrated journal reads through the kept
      read. It may land as two pull requests, the store's half first.
    - **Slice 2, the run and seat views.** `seat_summary` and
      `event_index`; the run view from the trail and the seat rows; the
      seat view's checkpoint pages; a run's chain verified streaming at
      open. Acceptance: every run view and seat view equals today's on
      every fixture journal, and ruling 9's run view, seat view and
      opening budgets hold, bound by `heap_run_view.rs`,
      `heap_seat_view.rs` and `heap_open.rs`.
    - **Slice 3, `inspect`, `seats`, `watch`, `transcript`, the console's
      view routes, `costs` and `compare`** from the same projections.
      Acceptance: their output equals today's on every fixture journal,
      and ruling 9's `costs` budget holds, bound by `heap_costs.rs`.
    - **Slice 4, the aggregates:** `analytics`, `graph` and Muninn's
      dossier from the rows, and the run selectors (decision 0015) by a
      key range on `runs`. Acceptance: their output equals today's on
      every fixture journal, and ruling 9's aggregate budget holds, bound
      by `heap_aggregates.rs` and the `fleet_aggregate` bench.

## Asked of the operator

Each choice this decision makes, recommendation first:

1. **The read model.** Three projections, `run_summary`, `seat_summary`
   and `event_index` (ruling 1). Or: `run_summary` alone now, and the
   other two ruled when slice 2 begins.
2. **The writer.** The append carries its caller's fold and writes the
   folded columns only when that fold stands at the head (ruling 3). Or:
   the store keeps its own fold per run and reads a run whole on a miss,
   with no change to the append's callers. Or: a summarizer that catches
   up after the append, which lags.
3. **Older writers and the schema version.** Triggers keep presence, the
   head and the event index for every writer, and `DATABASE_SCHEMA` stays
   1 (rulings 3 and 8). Or: the version moves to 2 and an older binary
   refuses the journal, for reading as well.
4. **Integrity at fleet level.** The head check alone, with the full chain
   check at a run's opening, at every verb that acts on a run, and in
   `brokkr doctor` (ruling 5). Or: the console also verifies runs whole
   off its input loop, one an idle tick, marking each row with when it was
   last verified. Or: every body verified at the first fleet read, as
   0088 ruling 5 has it, which keeps the first read growing with the
   journal.
5. **The row's bound.** Every column a page selects is bounded: the
   feature's first line to 1,024 bytes, the refusal's words to 512, the
   residuals as their highest open severity and a count; the lossless
   feature and list are the row's last columns, read by key for `--json`
   and the run view (ruling 2). Or: no lossless column in the row, and
   `--json` derives the list from the run's ruling events, which waits
   for slice 2's `event_index`.
6. **The page key.** A generated `lane` column, with `acknowledged` kept
   by triggers on the events and the acknowledgements, so every range is
   an index range that visits only what it returns (rulings 3 and 6). Or:
   a partial index per section, six indexes for one key. Or: the
   acknowledgement read per row, as the earlier draft had it, whose cost
   grows with the acknowledged runs (21,015 to 191,115 VM steps,
   measured).
7. **Rows behind their head.** Read first on every page and folded before
   they are placed, at most a page and one ahead per refresh with the rest
   counted, and caught up at every current writer's open (rulings 3 and
   6). Or: listed unfolded in "needs you" as behind, with no verdict until
   a writer catches them up.
8. **The order within a section.** Last activity, newest first, then run
   id (ruling 6). Or: creation, newest first, as today, which costs a sort
   of a day's finished runs per page.
9. **`brokkr runs`.** Every run by default, a page at a time, today's
   JSON shape, and `--limit` and `--after` with `next` and
   `VIEW_VERSION` 24 (ruling 6). Or: one page by default, which changes
   what the console plugin reads.
10. **The older runs' count.** Kept, counted by the index alone (ruling
    6). Or: "more older runs" with no number, which costs nothing.
11. **The budgets.** 50 ms to the first fleet page, warm and measured;
    256 KiB of live heap per page, held by dhat at two sizes with wide
    rows on the page; a 256 KiB page cache; RSS within 1 MiB of the
    floor; each range's plan pinned and its visited rows bound; and the
    run view, seat view, `costs` and opening budgets of ruling 9's table,
    each bound in its slice. Or: other numbers.
12. **The aggregates' exception.** `analytics`, `graph` and Muninn's
    dossier hold 256 KiB of accumulators whatever the fleet's size, and
    take time that grows with the runs and seats, never events: 250 ms at
    20,000 runs and 240,000 seat rows, 57 ms measured (ruling 9). Or: a
    kept aggregate per day, written at append, which bounds their time
    too and adds a fourth projection.
13. **Migration.** The first writer backfills a run per transaction, and a
    read-only reader falls back to the kept read until it has finished
    (ruling 8).
14. **Checkpoint bodies.** They stay inline, and a move is a later ruling
    on a measured need (ruling 10). Or: a side table the chain covers, in
    this build.
15. **The first slice.** The fleet, slice 1 (ruling 11).

## Consequences

- A fleet page reads about a hundred summary rows and a hundred head rows,
  whatever the journal's size or a run's feature and findings: kilobytes,
  not megabytes or gigabytes.
- A run whose summary fell behind its head costs its own whole read on
  the page that shows it, until a current writer catches it up; it is
  never placed by what it last folded to.
- The aggregates are the one readout whose time grows with the fleet,
  with its runs and seats and never its events; their memory does not.
- The journal file grows by a row per run, a row per participant and a
  row per event, the last about a tenth the size of a checkpoint's row,
  estimated from its columns and measured in slice 2.
- Each append costs a few microseconds more for its rows (measured with
  fsync off), beside a commit's sync of milliseconds.
- An older binary writing to a migrated journal keeps the model's
  presence, order, acknowledgement facet and index current through the
  triggers; its runs read whole until a current writer opens the journal.
- A corrupt event below a run's head is no longer refused in the fleet. It
  is refused when the run is opened, exported, resumed or swept by
  `brokkr doctor`.
- The kept read of decision 0088 stays as the fallback for a journal
  without the model, and so for as long as a read-only reader can meet
  one.
- Nothing under `contracts/`, `fixtures/`, `policy/` or `reference/` moves,
  and decision 0014 holds: every reader opens read-only and writes
  nothing.
