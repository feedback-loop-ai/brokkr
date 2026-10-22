# 0089 — Every readout reads a page of what the writer keeps: one summary row per run, a row per seat, and an index of every event

Status: accepted — operator ruled 2026-10-19 (PR #769), every choice asked taken as recommended
Date: 2026-10-18

Built: partial (#753) — slice 1, the fleet, is built but Muninn's queue and the many-hearth `brokkr runs`, which still read the kept way, and the console's last page by the reverse order; of slice 2, `event_index` is built, its trigger, backfill, keyed reads and ruling 4's check, but `seat_summary` and the run and seat views that read them are unbuilt; slices 3 and 4 are unbuilt
Supersedes in part: 0088

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
(`fleet.rs:125`). The store offers no summary and no page. Its keyed
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
(`crates/brokkr-store/src/import.rs:267`). A run row reaches `runs` at
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

**SQLite's own memory, measured after the third review.** dhat counts
Rust's global allocator alone (`crates/brokkr-cli/tests/heap/support.rs:14`
to `15`), and SQLite allocates beside it. A third scratch program
(`rusqlite` 0.32.1 bundled, which printed SQLite 3.46.0) read SQLite's
high-water by `sqlite3_status64(SQLITE_STATUS_MEMORY_USED)`, reset before
each statement, on a reader at `PRAGMA cache_size = -256` over 4 KiB pages:

| Read | SQLite's high-water above the reader's base |
|---|---|
| An acknowledgement's operator and reason cut by `substr` in the select, 1 MiB each | 2,114,384 B |
| The same, 64 and 256 characters stored | 0 B |
| The cut pair stored in a row whose 1 MiB value sits after it, selected alone | 8,736 B |
| `GROUP BY` a 64-byte key over an index range: 24,000 rows in 9,000 groups; 240,000 in 9,000; 240,000 rows of a 4-byte key in 4 | 1,273,184 B; 1,129,240 B; 1,050,320 B |
| The same three with `SQLITE_CONFIG_PMASZ` set to 8 pages before the first open | 537,168 B; 635,200 B; 371,608 B |
| A median by `ORDER BY` the duration `LIMIT 2 OFFSET` the middle, over 20,000 and 200,000 runs, either PMA size | 176,032 B; 1,440,072 B |

Both plans were an index range and `USE TEMP B-TREE FOR GROUP BY` (or
`ORDER BY`). So a cut made in a select still reads the whole value inside
SQLite. A cut made where the value is stored bounds it. `GROUP BY`'s
sorter held about a megabyte however few its groups, from 24,000 rows to
240,000. In the pinned
`libsqlite3-sys-0.30.1/sqlite3/sqlite3.c`, as the third review read it,
`SQLITE_SORTER_PMASZ` is 250 pages (`:22868`), the floor is that many
pages (`:103795`), and the cache cap only raises it (`:103806`). The
median's sort holds every row before its offset, so it grows with the
window. The scratch was removed after.

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
     reference, its start, and its first and last checkpoint's seq and
     time;
   - `event_index`: one row per event, keyed `(run_id, seq)`: its type,
     attempt, member and time, with an index `(run_id, attempt_id,
     member, seq)` for a seat's checkpoint stream and a partial index on
     `(run_id, seq)` over every type but `effect/checkpointed` for a run's
     trail; a checkpoint's and a park's row also carry the dated facts the
     aggregates read (ruling 7).

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
   | `folded_seq`, `folded_hash` | the seq column; ruling 5's check; the `behind` test (ruling 6) | the seq and hash of the last event the writer judged, whether its fold stood or refused: `RunState.seq`, `RunState.last_hash` (`fold.rs:139` to `140`) after a fold, the run's last sealed event after a refusal; null until a writer has judged the run |
   | `head_seq`, `head_hash`, `last_recorded_at` | the age as text | the last event's seq, hash and `recorded_at` (`crates/brokkr-cli/src/fleet.rs:55`), kept by ruling 3's trigger |
   | `last_at_ms`, `order_at` | the order, the section and staleness (`fleet.rs:272` to `287`, `440` to `447`), which today parse `last_recorded_at`, or `created_at` while the run has no event, at every read | `last_at_ms` is that stamp's milliseconds by the one parse (`js::parse_millis`, `crates/brokkr-view/src/js.rs:179`), and while the run has no event, `created_at`'s, as `section_of` falls back to it (`fleet.rs:282`); written by the writer with the folded columns, null when the stamp does not parse; `order_at` is generated, `COALESCE(last_at_ms, 9223372036854775807)` (ruling 6) |
   | `acknowledged`, `ack_operator`, `ack_reason`, `ack_at` | whether a quarantined run's acknowledgement stands at its head (`need`, `fleet.rs:418` to `424`), and its line's who, why and when (`Acknowledgement::line`, `fleet.rs:123` to `128`) | an acknowledgement made at the row's head (`acknowledgement.rs:51` to `58`); ruling 3's triggers keep it, its operator, reason and time copied cut |
   | `lane` | nothing: the index key ruling 6 reads a range by | generated from the columns above (ruling 6) |
   | `feature`, `residuals`, and the whole value of every cut column, last in the row | `--json`'s and `brokkr runs`' lossless values, and nothing on a page | the `runs` row; the residual findings as `residual_findings` derives them; the fold's values before their cut |

   **A row is bounded, so a page is.** Every column a page selects has a
   stated bound, and the writer holds it:

   | Column | Bound | Past it |
   |---|---|---|
   | `run_id` | 128 bytes. The engine mints at most 41 characters, a slug of 32 and eight hex (`crates/brokkr-runtime/src/engine.rs:515` to `525`); `import` refuses an id past 128 ASCII characters (`RUN_ID_MAX`, `crates/brokkr-store/src/import.rs:81`, `:144`). `create_run` checks no length (`lib.rs:429`), so slice 1 gives it `import`'s check | nothing a Brokkr writer writes. The page selects an id's first 129 bytes, and a reader that reads 129 refuses the row at the edge as a quarantined row naming an id no writer admits; it never shows a cut id as a run |
   | `status`, `residual_open` | closed words: `Status::as_str` (`fold.rs:54`), 17 bytes at most; `Severity` | a word the reader does not know is refused at the edge (below) |
   | `phase`, `rule_id`, `rule_from`, `hire_seat` | 64 bytes each, cut at a character boundary. Each is the recipe's or the table's own name, which the run's data states and no contract bounds | the whole value, last in the row |
   | `park_reason` | 256 bytes. `ParkReason::Stated` is free text (`crates/brokkr-core/src/fold/run.rs:66` to `70`) | the same |
   | `refusal_words` | 512 bytes | the same |
   | `ack_operator`, `ack_reason`, `ack_at` | 64, 256 and 40 characters, cut by the trigger as SQLite's `substr` counts characters in text: at most 256, 1,024 and 160 bytes | the whole acknowledgement, in its own table |
   | `feature_line` | 1,024 bytes | the whole feature |
   | `created_at`, `last_recorded_at` | 40 bytes. A stamp the engine writes is about 30; `import` keeps a stamp as the export carries it | the whole stamp. No order, section or bound reads the text: they read `last_at_ms`, which the writer parsed from the whole stamp |
   | integers and hashes | fixed: eight bytes an integer, and `folded_hash`, 64 hex | none |

   The acknowledgement's `operator` and `reason`
   (`crates/brokkr-store/src/acknowledgement.rs:26` to `32`, `TEXT NOT
   NULL`, unbounded) are bounded as stored: ruling 3's trigger copies them
   onto the row cut, and the page reads the copy, never the
   acknowledgement's table. A cut in the page's select would bound
   nothing, since `substr` reads the whole value inside SQLite first
   (2,114,384 B for 1 MiB fields, against 8,736 B for the stored cut;
   Context, SQLite's own memory). The whole pair stays in its table, which
   `brokkr runs` and the run view read. So a refused row's
   selected bytes are at most about 3.3 KiB, a folded row's about 1.9 KiB
   (the two exclude each other: a refusal has no fold), and a page and the
   one ahead, 100 rows, at most about 330 KiB of row text whatever a run
   holds. The whole values are the row's last columns and no page selects
   them, so SQLite reads none of them, nor their overflow pages: one
   parked run with a 1 MiB feature and 10,000 findings added 981 B to its
   page and no time (Context, re-measured). `brokkr runs`, `--json` and the
   run view read them by key, one row at a time as each is written out, so
   their memory is a page of bounded rows and the largest row's whole
   values: bounded by the run, never by the journal.

   A cut value is shown on a page as its bound leaves it, ended with the
   ellipsis a title takes (`fleet.rs:473`). So the console is unchanged
   while every value fits its bound, as every fixture journal's names and
   reasons do: the
   longest `rule_id`, `phase`, `from`, `seat` or `next` field there,
   key and quotes included, is 41 characters
   (`"rule_id":"REVIEW-REFORGE-EXHAUSTED-DEBT"`), and the longest `reason`
   169 (`grep -rhoE` over `fixtures/journals`, `sort -u`, `wc -L`). One
   fixture's feature has a first line of 1,993 bytes, past its 1,024
   (`tui-graph-the-selection-box-gets-80f98deb.ndjson`, by `jq`'s
   `utf8bytelength`), and its title is unchanged by the agreement below. Past
   its bound, the console shows a value's first bytes and an ellipsis
   where today it shows more. That is an output change, put to the
   operator (Asked 5).

   `title` is derived from `feature_line` as it is today from the feature.
   The two agree whenever the first line is at most 1,024 bytes, or its
   first 1,024 bytes are at least as wide as the title asked for. A line
   with no zero-width character is at least 256 columns wide in 1,024
   bytes (a character takes at most four bytes and at least one column),
   so they agree at every width up to 256 columns; a line made mostly of
   zero-width marks is the one case a title comes out shorter. Slice 1's
   `a_cut_first_line_titles_as_its_feature`, in `brokkr-view`'s fleet
   tests (`crates/brokkr-view/src/fleet/tests.rs`, beside
   `a_title_takes_the_columns_it_is_given` at `:762`), binds it over a
   table of features: one under 1,024 bytes, a 1 MiB ASCII line, a 1 MiB
   line of three-byte characters cut inside one, a first line behind
   blank lines, and a line of 2,000 zero-width joiners before its words.
   It asserts `title_within(cut, w)` equal to `title_within(feature, w)`
   for the first four at `TITLE_COLUMNS` (`fleet.rs:134`, 60) and at 256
   columns, and pins the exact shorter title the fifth gives. Its removal
   control is a cut at a byte rather than a character boundary, which
   fails on the third. `residual_open` is the highest open severity, as
   `open_severity` derives it today. Decision 0071 ruling 7 asks for one
   derivation, so slice 1 moves that function, with the operator's
   closing marks it reads (`brokkr-view/src/lib.rs:803`), into one pure
   function in `brokkr-core` that the writer and `brokkr-view` both call.

   `status` holds `Status::as_str()`'s word, or nothing for a refusal; a
   reader parses it into `Status` at the edge and refuses a word it does
   not know, and parses `residual_open` into `Severity` alike.
   `residuals` is the canonical JSON of a closed list, read into its type
   with `deny_unknown_fields` at the edge (decision 0071 ruling 3). The
   acknowledgement itself, who and why, stays whole in its own table
   (`crates/brokkr-store/src/acknowledgement.rs:26`), read by key by
   `brokkr runs`, `--json` and the run view: it is bookkeeping beside the
   chain and lapses when the journal moves (`fleet.rs:108` to `111`). The
   row carries whether one stands at its head, which is what places it,
   and the cut copy its line prints.
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
     to 0 and clears the acknowledgement's copy, and adds its
     `event_index` row, reading the type, attempt,
     member and time from the envelope by `json_extract`, as the store
     already reads a manifest's engine in SQL (`ENGINE_OF_RUN`,
     `lib.rs:280`). A trigger after each insert into
     `quarantine_acknowledgements` sets `acknowledged` to 1 on the run's
     row when the acknowledgement's head is the row's head, the same test
     its standing query makes (`acknowledgement.rs:51` to `58`), and
     copies its operator, reason and time onto the row cut to ruling 2's
     bounds. That `substr` reads each whole value once, in the
     acknowledging writer's transaction, never in a reader's. So an
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
     rows from brokkr-core's pure projections of the result. Those
     include `last_at_ms`, parsed from the head event's whole stamp, or
     `created_at` while the run has none, by the one parse, which slice 1 moves from `brokkr-view` (`js.rs:179`) into
     `brokkr-core` so the writer and `brokkr-view` call one function, and
     `folded_seq` and `folded_hash`, the head it judged. A run whose fold
     refuses (the backfill and the catch-up below can meet one) gets its
     refusal columns, nothing in the fold's, and `folded_seq` and
     `folded_hash` at the head it judged, so a refused row is at its head
     until the run moves. A caller with no fold, or a fold at another
     head, writes nothing folded: the row falls behind its head, its
     `lane` becomes `behind`, and ruling 6 reads that run whole before
     placing it. The projections are pure functions in `brokkr-core`, so
     the writer and every reader derive them one way.
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

   **Every readout of one run opens it.** The run view (the console's run
   level and detail pane, `inspect`, `seats`, `watch`, `/api/view/<run>`),
   the seat view, `costs`, `compare` and `export` each read that run's
   whole chain, verified, before anything else they read of it. That read
   is bounded by the run, never by the journal, and refuses a corrupt run
   exactly as today: each calls `Store::load` now (`inspect` and `seats`
   through `run_view_of`, `crates/brokkr-cli/src/verbs/readouts.rs:101`;
   the console through the fleet's read, `crates/brokkr-cli/src/fleet.rs:105`;
   `crates/brokkr-cli/src/compare.rs:25`; `ui.rs:123`;
   `crates/brokkr-cli/src/run_view.rs:231`; `lib.rs:1438`; `export`
   at `crates/brokkr-store/src/lib.rs:644`), which verifies the chain
   (`lib.rs:626` to `640`), and each keeps that refusal and its error. So
   the weakening above is the fleet's and the aggregates' alone: an
   aggregate counts a run by its rows, where today's whole read counts a
   corrupt run as unreadable. The binding is
   `a_corrupt_run_is_refused_wherever_it_is_opened`, in `brokkr-cli`: a
   fixture journal with one event below the head edited, refused by each
   of those readouts with the `StoreError` variant it returns today. Its
   removal control is a run view that reads the trail without the
   opening, which then shows the run.

6. **The fleet reads a page by keyset, and every page costs the same.**
   The summary is indexed `(lane, order_at DESC, run_id DESC)`. Both
   `lane` and `order_at` are generated columns, so SQLite keeps them from
   the row's own columns whoever writes them. `lane` is exactly:

   ```sql
   CASE
     WHEN folded_seq IS NULL OR folded_seq <> head_seq THEN 'behind'
     WHEN refusal_words IS NOT NULL AND acknowledged = 0 THEN 'refused'
     WHEN refusal_words IS NOT NULL THEN 'acknowledged'
     WHEN status = 'awaiting_operator' THEN 'parked'
     WHEN status = 'running' THEN 'running'
     ELSE 'finished'
   END
   ```

   `head_seq` is never null: the trigger on `runs` writes 0, and
   `head_hash` the zero hash, as `head_of` reads a run with no event
   (`lib.rs:225` to `239`). `folded_seq` is null only until a writer has
   judged the run, and a refused row's `folded_seq` is the head it was
   refused at (ruling 3), so a refused row is `refused` or `acknowledged`
   while it stands at its head and `behind` once the run moves. The order
   of the cases is the order of the test: a row behind its head is
   `behind` whatever it last folded to. A lane is not a displayed fact.
   It only says which range a row is read from; `brokkr-view` still places
   every row it is handed by `section_of` (`fleet.rs:272`), from the row's
   facts.

   **The key is a time the writer parsed, never text.** `section_of`
   and `silence` parse the stamp at every read (`fleet.rs:282` to `284`,
   `440` to `447`) by `js::parse_millis`, which admits an offset as well
   as `Z` (`js.rs:179` to `220`), and `import` keeps an export's stamps as
   they are. A text range would order `+02:00` stamps against `Z` ones by
   their characters, not their times. So the index and every bound read
   `order_at`: the writer's `last_at_ms`, the milliseconds of the same
   parse, or `9223372036854775807` when the stamp does not parse. That
   value is what `section_of` does with an unreadable time: it is newer
   than every bound, so a finished run with one lists in "last 24h"
   (`fleet.rs:283` to `285`) and a running one is never stale
   (`fleet.rs:442`), and it sorts first in its section. The Context's
   tables measured the earlier `last_recorded_at` key, a text column; the
   plans and their costs have the same shape on the integer key, and
   slice 1's plan test (ruling 9) pins the ruled one.

   A page is the sections in `Section::ORDER` (`fleet.rs:241`). Each
   section reads one range per lane it holds, with `now` the reader's
   clock in milliseconds, `stale` the staleness bound (`fleet.rs:158`)
   and `day` the 24 hours of `RECENT_MILLIS` (`fleet.rs:141`):

   | Section | Its ranges |
   |---|---|
   | "needs you" | `refused`; `parked`; `running` with `order_at < now - stale`; and the `behind` runs `section_of` places here (below) |
   | "running" | `running` with `order_at >= now - stale` |
   | "last 24h" | `finished` with `order_at > now - day` |
   | "older" | `finished` with `order_at <= now - day`; `acknowledged`, whatever its time, as `section_of` lists an acknowledged run there however recent (`fleet.rs:279` to `281`) |

   **A section is its ranges merged into one order.** Each range is read
   in the section's order, `(order_at DESC, run_id DESC)`, below the
   cursor, and its statement is stepped only as far as the merge consumes
   it, holding one row ahead. The reader merges the ranges by that key,
   as a k-way merge does, and fills the page from the merge in section
   order. So a section's rows are in one total order whichever lane each
   came from: a park newer than every refusal lists first in "needs you",
   however many refusals there are, and no lane's rows push another
   lane's newer ones off the page. A page of n rows visits at most n rows
   and one more per range it read. Every range is an equality on `lane`
   and at most one bound on `order_at` with the cursor's, so it visits
   only the rows it returns: 912 VM steps for 50 refusals behind 900 or
   9,000 acknowledged ones, where reading the acknowledgement per row
   visited 21,015 and 191,115 (Context, re-measured). A cursor is the last
   row's `(section, order_at, run_id)`, typed and parsed at the edge, and
   applies as `(order_at, run_id) < (?, ?)` to every range of its section,
   so page N reads what page 1 reads.

   **A row behind its head is folded before it is placed.** Its folded
   columns are from an older head, so its lane says nothing of where it
   belongs: a run parked at seq 11 by an older binary, summarized running
   at seq 10, is `behind`, never `running`. The reader reads the `behind`
   range first on every page, reads each of its runs whole, chain
   verified, as #744's kept read does and keeps it between refreshes,
   and hands `brokkr-view` the fold's row, which `section_of` places like
   any other; a run whose read refuses is quarantined, as today. The
   `behind` range is read first, never shown first: each folded run is
   merged into the section `section_of` gives it, at its key computed from
   the read's last event by the same parse, and the cursor applies to it
   as to a range. At most
   a page and one ahead of `behind` runs (100) are read on one refresh, by
   last activity; any beyond them are counted by the index, and "needs
   you" prints them as one line, so a lagging run is never placed by its
   stale status and never dropped silently. The `behind` lane is empty
   whenever every writer is current and the backfill has finished
   (rulings 3 and 8); when it is not, a page costs the runs it reads
   whole, bounded by those runs and never by the journal, and ruling 9's
   budgets are stated for an empty `behind` lane.

   - Within a section the order becomes `(order_at DESC, run_id DESC)`:
     last activity, newest first, an unreadable time first, then run id.
     Today it is creation, newest first (`fleet.rs:693` to `697` over
     `lib.rs:528`). The whole read's order (`run_rows`, `fleet.rs:694`;
     `fleet_rows`, `:720`) moves to the same key from the same function,
     so the paged read and the whole read are compared like with like
     (ruling 9).
   - The console reads the visible page and one ahead, scrolls by keyset,
     and reads the last page by the reverse order. The older runs' count
     it prints (`crates/brokkr-cli/src/tui/panes.rs:224` to `225`) is an
     index-only count, the one fleet cost that grows with the number of
     runs (2.624 ms at 18,800, measured), never with events.
   - `brokkr runs` with no flag prints every run as today, read and
     written a page at a time, so its memory is one page's; its JSON shape
     is today's. `--limit N` prints one page and `--after CURSOR` the page
     after a cursor, and their JSON gains `next` (the cursor, or null at
     the end), which moves `VIEW_VERSION` up by one
     (`brokkr-view/src/lib.rs:128`). `/api/runs` takes the same two.
     *Amended, 2026-10-21:* the move is from 24 to 25, not 23 to 24,
     because #568's run view took 24 first.

7. **Every readout states what it reads, its page and its bound.** Every
   page costs milliseconds and kilobytes of live heap, whatever the
   journal's size; a readout that needs a run's whole history says so, and
   is bounded by the run. Every readout but `graph` reads a projection.

   | Readout | Reads | Keys | Page | Bound |
   |---|---|---|---|---|
   | The fleet: runs level, `brokkr runs`, `/api/runs`, the many-hearth fleet, Muninn's queue | `run_summary`, the acknowledgement's cut copy included | ruling 6's index; `head_of` per row | the visible rows and one page ahead, per hearth | ruling 9's numbers |
   | Opening a run, first, by every readout of one run: the run view, `inspect`, `seats`, `watch`, `/api/view/<run>`, the seat view, `costs`, `compare` (ruling 5) | the run's chain, verified streaming, one envelope held at a time; a corrupt run refused as `Store::load` refuses it today | `(run_id, seq)` | none: a whole read | the run: at #744's rate, about 2.5 µs an event to hash, so about 18 ms for the 7,510-event run the benches size by (`crates/brokkr-cli/benches/budgets_lib.rs:23`) |
   | The run view: run level and detail pane, `inspect`, `seats`, `watch`, `/api/view/<run>` | after its opening, the run's summary row, its trail through `event_index`'s partial index and the events by key, its seat rows | `(run_id, seq)` | the trail whole; journal rows a page at a time by seq | its opening, then the run's control events, about 5% of its events: 25 of 525 read in 0.11 ms, measured |
   | The seat view: participant level, `brokkr transcript`, `/api/presentation`, `/api/transcript` | after its opening, the seat row; its checkpoints through `event_index`'s stream index | `(run_id, attempt_id, member, seq)` | 50 checkpoints, newest first | its opening, then a page of checkpoints; the transcript file keeps its own caps and heap budgets (`quality/heap-bytes.json`) |
   | `brokkr costs`, `brokkr compare` | after their opening, the seat rows of one or two runs; compare's trails | `(run_id, ...)` | the run's seats | their opening, then the run's attempts |
   | `brokkr analytics`, Muninn's dossier | `run_summary`'s aggregate columns, `seat_summary`, and `event_index`'s dated rows, streamed, one row held at a time (below) | `run_summary (finished_ms)`; `seat_summary (started_ms)`; `event_index`'s checkpoint index and park index | an aggregate, not a page | Brokkr's heap: at most 64 groups a figure, each key at its bound, and the medians' counts, 256 KiB; SQLite's: the page cache and `GROUP BY`'s sorter (ruling 9); time: the runs open or finished inside the window, their seats and the dated facts inside it, the runs finished in the equal window before it, counted, and the parks that ended within their own length of its start (ruling 9's exception) |
   | `brokkr graph` | as today: every run of every hearth whole (`read_hearths`) | none | none | the journal, until a decision of its own (below) |
   | `export`, `anchor`, `import`, `replay`, `ledger`, `resume`, the engine's first turn | the run whole, chain verified | `(run_id, seq)` | none | the run |
   | `brokkr doctor`'s sweep | every run whole, one at a time | `(run_id, seq)` | none | the largest run in memory; the journal in time |

   So three projections serve every readout but `graph`: the fleet,
   Muninn's queue and the aggregates share `run_summary`; the run view,
   the seat view, costs, compare and the aggregates share `seat_summary`;
   the run view, the seat view and the aggregates share `event_index`.

   **`graph` is a follow-up.** It reads what no projection here keeps:
   each run's stays in its states (`stays`,
   `crates/brokkr-view/src/graph/facts.rs:55` to `78`); its rulings, with
   rule, phase, next state, result and severity (`rulings`, `:82` to
   `99`); its manifest's pins, which drift compares for every run in the
   window (`:143`; `drift`, `crates/brokkr-view/src/graph/figures.rs:251`
   to `274`); and each spend's phase (`figures.rs:389` to `394`). A state's
   entered count, median, p90, tokens and cost are derived from those
   (`state`, `:376` to `413`). Its output is not bounded by count either:
   it lists every open run of the recipe (`live`, `:499` to `505`) and,
   when the table is not found, every state and rule its runs named
   (`:50` to `69`, `:165` to `178`). Until a decision of its own rules
   graph's projection, its indexes and its output's bound, `graph` reads
   as today, and nothing here claims a budget for it.

   **What the aggregates read, and where it is kept.** Today's figures
   filter every fact by its own time, so a run's totals cannot answer
   them: a spend counts when its checkpoint falls in the window
   (`reported_in`, `crates/brokkr-view/src/analytics/figures.rs:129` to
   `136`; `Spend`, `crates/brokkr-view/src/attempts.rs:112` to `120`), a
   seat when it started or metered there (`Site::ran`, `attempts.rs:45`),
   a park by its overlap with the window (`waited`, `figures.rs:68` to
   `76`), and a run by when it ended. Each fact gets a home, written by
   the writer with the folded columns from `brokkr-core`'s pure
   projections (slice 4 moves the readings `facts.rs` and `attempts.rs`
   make into `brokkr-core`, as slice 1 moves `open_severity`):

   | Fact | Read today | Kept as |
   |---|---|---|
   | a run's realm, recipe, start, end, ending and its rule, implement visits | `facts` (`crates/brokkr-view/src/analytics/facts.rs:190`) | `run_summary`'s aggregate columns, which no page selects: `realm`, `recipe`, `started_ms`, `finished_ms`, `ending`, `ending_rule`, `implement_visits`, the three names cut as group keys (below); indexed `(finished_ms)` |
   | a run's review routes | `reviews` (`facts.rs:168`); `rework` reads the first, the second and whether any went back (`figures.rs:236` to `272`) | `review_first`, `review_second`, `review_back` on the same row |
   | a park: its phase, start, answer and end | `parks` (`facts.rs:127`) | its `run/parked` row in `event_index`, with `phase`, `answered_ms` and `ended_ms` written when the operator next commands and when the run next moves, null while open and read as `now`, and `span_class`, written with `ended_ms`: the bit length of the park's reach, `max(answered_ms, ended_ms) - at_ms`, 0 when that is not positive (0 to 63), or 64 while the park is open; a partial index `(span_class, at_ms)` over parks (below) |
   | a metered checkpoint: its time, cost, tokens, and whether it reported a cost or closed a session | `attempts` (`attempts.rs:112` to `140`) | its `effect/checkpointed` row in `event_index`, with `at_ms`, `usd`, `reported`, the token counts and `session`; a partial index `(at_ms)` over checkpoints |
   | a seat's start, phase, provider and model | `Site` (`attempts.rs:28` to `36`) | `seat_summary`'s `started_ms`, `phase`, `provider`, `model`, the three names cut as group keys (below); indexed `(started_ms)` |

   So a window `[from, to)` reads one range per index: the runs that
   ended in it and the runs still open (the `running`, `parked` and
   `behind` lanes), the seats that started in it, the checkpoints in it
   and the parks that overlap it. A checkpoint is an event, so a window's
   time grows with the checkpoints inside it. Three reads reach before the
   window, and each is bounded:

   - **The preceding window.** The headline's `previous` and `change`
     count the runs shipped in the equal window before it,
     `[from - span, from)` (`figures.rs:328` to `336`). That is one count
     over the same `(finished_ms)` index, holding one integer, and its
     time grows with the runs finished in that window alone.
   - **The parks that overlap it.** A park overlaps the window when it
     started before `to` and ended after `from` (`Window::overlap`,
     `facts.rs:31` to `33`), an open one ending at `now`. One range on
     either time alone visits every park on the far side of the other.
     A park's reach is the later of its answer and its end, since today
     waiting and parked time are each overlapped on their own (`parks`,
     `facts.rs:133` to `142`; `waited`, `figures.rs:68` to `76`) and
     `recorded_at` is evidence, not a monotonic clock
     (`crates/brokkr-core/src/envelope.rs:5` to `6`): a command stamped
     after the run's next event, across a clock stepped back, keeps
     waiting time past the park's end. So the window reads one range per
     `span_class`: for each class `c` from 1 to 63, `span_class = c AND
     at_ms >= from - 2^c AND at_ms < to`, checking `max(answered_ms,
     ended_ms) > from` on each row it visits, and for the open parks,
     `span_class = 64 AND at_ms < to`. A park of class `c` reaches less
     than `2^c` past its start, so one that overlaps the window started
     within `2^c` before it, and its class's range holds it. A park that
     starts after the window is in no range. A park of class 0, zero-length
     or ended before it started across a clock stepped back, is in no
     range either: it reaches no later than its start, so its overlap with
     any window is nothing (`Window::overlap` floors at zero, `facts.rs:31`
     to `33`), and every figure that reads a park adds only its overlap
     (`waited`, `figures.rs:68` to `76`; `waiting_by_step`, `:214` to
     `216`, counts a park only past zero).
   - **The parks just before it.** A park of class `c` whose reach ends
     before the window is visited, and rejected, only while it started
     within `2^c` of `from`. Since `c` is at least 1, its reach is at
     least `2^(c-1)`, so it ended within
     `2^(c-1)` before `from`, within its own length of the window. That
     band is the one read before the window that is not a count: at most
     two parks a run in each class while its clock runs forward, since
     then a run's parks never overlap and each started inside the same
     `2^(c-1)`; across a clock stepped back, at most the run's parks. A
     park that ended further back than its own length is in no range,
     however many there are. An open park is at most one a parked run.

   Two figures need more than a stream.

   - **The median is selected in Brokkr, never sorted.** It is the
     headline's and each realm row's (`median`, `figures.rs:59` to `65`;
     `realm_row`, `:309`). SQLite's `ORDER BY ... LIMIT 2 OFFSET` the
     middle holds every row before its offset, 176,032 B at 20,000 runs
     and 1,440,072 B at 200,000 (Context). A first pass over the window's
     finished runs reads their count and their least and greatest
     duration. Each later pass counts the durations inside the interval
     that holds each middle rank into 64 buckets and narrows to the
     bucket that holds it, until each middle rank is one value: at most
     eleven passes after the first for any span a 64-bit duration takes.
     A median holds two histograms of 64 counts, 1 KiB, and every realm's
     median is selected in the same passes, at most 64 realms, 64 KiB.
     `middle` (`figures.rs:42`) then takes the two values as today. The
     passes are a pure function in `brokkr-view` handed each pass's
     stream, a closure seam (decision 0071 ruling 2).
   - **A grouped figure holds at most 64 groups, each by a bounded
     key.** A grouped figure holds one entry per distinct group
     (`models`, `figures.rs:275` to `294`; `waiting_by_step`, `:210`;
     `stopped_by_rule`, `:93`; the providers of `cost_figure`, `:161` to
     `164`; the realm and recipe rows,
     `crates/brokkr-view/src/analytics/recipes.rs:15`), and its groups
     are open sets whose names no contract bounds as read: a model is
     whatever a record's `model` field says (`attempts.rs:125` to `127`),
     and a phase, rule, realm, recipe or provider whatever the run's data
     named (ruling 2). Today `waiting_by_step` copies a whole phase into
     its row (`figures.rs:223` to `226`), so one 1 MiB phase would pass
     the heap budget with one group. So every key is stored cut where it
     is written, never cut in a query, which reads the whole value first
     (Context): a model to 80 bytes, the seat record's `maxLength` for
     it (`contracts/seat-record.v5.schema.json:28` to `32`), and a park's
     phase, a seat's phase and provider, and a run's realm, recipe and
     ending rule to 64 bytes at a character boundary, as ruling 2's names
     are. Two names that agree to their bound are one group, printed cut
     as ruling 2 prints a cut value. Each figure counts its groups in
     SQLite, `GROUP BY` the stored key over the window's range, keeps the
     64 its order ranks first, and prints the rest as one row, "N more".
     A figure that counts runs per group (`waiting_by_step`'s `runs`)
     groups by the key and then the run, and counts a change of run in
     that order, never by a `DISTINCT` table. Both the "N more" row and a
     cut key are output changes, only past 64 distinct groups in one
     window or past a key's bound. The fixture journals hold 2 distinct
     models, `opus` and `sonnet`, and 6 distinct phases (`grep -rhoE`
     over `fixtures/journals`, `sort -u`), and their longest name field
     is 41 characters (ruling 2).

   The group counts are the second figure SQLite derives beside the lane,
   a bend of decision 0071 ruling 7 for the same reason (a whole read is
   the only other way). Ruling 9's `every_figure_is_its_whole_read` holds
   them to `brokkr-view`'s derivation. `GROUP BY` runs in SQLite's sorter,
   whose memory grows to 250 pages, about a megabyte, before it spills,
   however few its groups (Context). Ruling 9 budgets it as SQLite's
   share. Past that it spills to a temporary file SQLite opens and deletes, outside
   the journal, so the reader still writes nothing to the journal
   (decision 0014).

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
   - **Live heap, Brokkr's and SQLite's.** A page's live heap is the sum
     of two shares, each read by its own counter, since dhat sees Rust's
     global allocator alone (Context). Brokkr's is at most 256 KiB to
     read, derive and paint one page and the one ahead: the read alone
     measured 92,112 B at both sizes, and 42,235 to 42,490 B under the
     ruled key. SQLite's is its own high-water,
     `sqlite3_status64(SQLITE_STATUS_MEMORY_USED)` through `rusqlite::ffi`,
     from the reader's open to the painted page, at most 400 KiB: the
     256 KiB page cache and the connection's statements, 363,288 B
     measured under the ruled key (342,488 B under the earlier one). So a
     page costs at most 656 KiB of live heap. The binding is a dhat test
     of its own, `crates/brokkr-cli/tests/heap_fleet.rs`, in the form of
     `heap_claude.rs` (dhat allows one profiler per process), which reads
     SQLite's high-water beside dhat's peak. Both budgets sit in
     `quality/heap-bytes.json` at the measured peak plus a tenth, never
     above 256 and 400 KiB. The test asserts Brokkr's peaks at 10,000 and
     1,000,000 events equal to the byte, and SQLite's within its budget at
     both. Its journals carry the rows that stretch a page, each value
     1 MiB: a parked run's feature, park reason, phase, rule id, ruling
     phase and seat; a refused run's refusal words and its
     acknowledgement's operator and reason; and 10,000 residual findings
     on a third. The test asserts Brokkr's peak equal to the byte to the
     same page with each of those values cut to exactly its bound (ruling
     2's table) and one finding, so the row's bound is what holds the
     budget. It holds SQLite's within its budget in both, not to the byte,
     since a wide row's overflow changes how many leaf pages the page's
     rows span. Its removal controls are a page that selects the whole
     `park_reason`, whose Brokkr peak grows by the megabyte, and a page
     that cuts the acknowledgement by `substr` from its own table, whose
     SQLite high-water grows by about two megabytes (2,114,384 B measured,
     Context).

     *Amended, 2026-10-21 (operator, on PR #808, recorded on #753):*
     SQLite's share is held to 500 KiB on every host, not 400, and its
     budget is that cap itself (512,000 B), not the measured peak plus a
     tenth. The same page reads 448,912 B on CI's `test (macos-latest)`
     leg against 399,176 B on Linux; that SQLite on Apple platforms counts
     each block's malloc size class is the likely cause, inferred and not
     measured on a Mac. Each table the journal gains adds to the schema
     SQLite reads at open (slice 2's `event_index` took Linux to 405,064
     B), so the cap is set to hold slice 2 on both hosts. Brokkr's own
     shares, and every other cap here, are unchanged.
   - **A page at every bound.** The 256 KiB is a page of ordinary rows.
     A page whose every row fills every bound holds about 330 KiB of row
     text before it is derived and painted (ruling 2), so it has a budget
     of its own: `heap_fleet.rs` reads a page of 100 such rows and holds
     Brokkr's peak at the measured peak plus a tenth, never above
     640 KiB, and SQLite's within its 400 KiB, since no selected value
     passes 1,024 bytes. Those numbers are proposed, not measured.
   - **RSS.** Within 1 MiB of the floor, `brokkr runs` on an empty journal:
     at most 7.5 MB max RSS on the generated journal.
   - **Independence.** Each range's plan is pinned to the exact text
     `EXPLAIN QUERY PLAN` gives for it, `SEARCH run_summary USING INDEX
     run_summary_page (lane=?)` or with `AND order_at<?`, `>?`, `<=?`,
     `>=?` or the cursor's `(order_at,run_id)<(?,?)`, with no `SCAN` and
     no temporary B-tree, in a `brokkr-store` test. A plan is not enough
     on its own: a range with a residual filter keeps the same plan text
     and still visits every row it filters out, as the acknowledgement
     read per row did. So the same test binds **visited rows**: it reads
     the first page and a page deep by cursor at 2,000 and at 20,000
     runs, 45% of them acknowledged refusals ahead of 50 unacknowledged
     ones, and asserts each statement's VM-step count
     (`Statement::get_status(StatementStatus::VmStep)`) equal at both
     sizes, to the step. Equality to the step holds only where the two
     journals put the same rows within reach of each statement, so the
     builder pins it: both journals hold the same runs, with the same
     times, in every range down to a page and two ahead of the deepest
     page read, and differ only below that and in the acknowledged
     refusals. Nothing held the Context's two journals so, and its first
     page, several ranges deep, read 1,822 and 1,810 steps; its single
     ranges, each returning 50 rows at both sizes, read 912 and 912. Its
     removal control is a page that reads the acknowledgement per row again,
     whose count grows (21,015 to 191,115, measured).
   - **The ranges are the sections.** `every_page_is_its_sections`, in
     `brokkr-cli`: over every fixture journal and the generated one, at a
     `now` on each side of every staleness and 24-hour bound, the rows the
     paged read returns for each section, in order, equal the rows
     `brokkr_view::sections` gives over the same runs read whole and
     ordered by ruling 6's key. The generated journal carries the rows
     that would split the two: a page of "needs you" that mixes lanes,
     an old refusal and enough older refusals to fill the page behind a
     newer park and a newer stale run, which must list first; a run
     whose last stamp is `+02:00` and one whose stamp does not parse;
     and an acknowledged run active within the day. This holds the lane
     table and the merge to `section_of`, so the two never derive a
     section differently. Its removal controls are a section read lane
     after lane without the merge, where the newer park falls off the
     page, and an `order_at` taken from the stamp's text, where the
     `+02:00` run is placed on the wrong side of a bound.
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
   `quality/heap-bytes.json`, a pinned plan and a Callgrind bench. Each
   live heap below names Brokkr's share and SQLite's, and each heap test
   reads SQLite's as `heap_fleet.rs` does. Reading a text column holds
   its whole value inside SQLite, and an envelope is read whole (today's
   keyed reader takes each as a `String`,
   `crates/brokkr-store/src/read.rs:65` to `66`). So a readout that reads
   envelopes, opening a run, the run view's trail and the seat view's
   checkpoints, has a SQLite share of the 256 KiB page cache, its
   statements and the largest envelope it reads: bounded by the run,
   never by the journal, and past the fleet's 400 KiB when one envelope
   is. Each of their tests stretches one envelope to 1 MiB and holds both
   shares within their budget and that envelope:

   | Readout | Time | Live heap | Bound by | Binding |
   |---|---|---|---|---|
   | The run view, its first screen | 50 ms, its opening included | Brokkr's: 256 KiB and the run's control envelopes as parsed (24,885 B for 25, measured), after an opening that holds one envelope at a time. SQLite's: the page cache, its statements and the largest of its control envelopes, its summary row's whole last columns and its acknowledgement's whole pair, each read by key (ruling 2) | its opening's events, then the run's control events | `heap_run_view.rs`: the same run in a 10,000- and a 1,000,000-event journal, peaks equal to the byte, and with one park's reason stretched to 1 MiB; the trail's plan on the partial index |
   | The seat view, a page of 50 checkpoints | 50 ms, its opening included | Brokkr's: 256 KiB and the largest checkpoint body on the page, each parsed into its row and dropped, after its opening. SQLite's: the page cache, its statements and the largest checkpoint envelope on the page | its opening's events, then the page's checkpoints | `heap_seat_view.rs`, the same two journals, peaks equal, and with one checkpoint on the page stretched to 1 MiB; the stream index's plan |
   | `costs`, `compare` | 50 ms, their openings included | Brokkr's: 256 KiB for two runs of 100 seat rows each, about 200 B a row estimated, after their openings. SQLite's: their openings' | their openings' events, then the runs' seats | `heap_costs.rs`, the same two journals, peaks equal |
   | Opening a run, chain verified | about 2.5 µs an event | Brokkr's: one envelope at a time, at most 256 KiB and the largest envelope. SQLite's: the page cache, its statements and the largest envelope | the run's events | `heap_open.rs`: a 7,510-event run, peaks equal in both journals, and with one envelope stretched to 1 MiB |
   | The aggregates: `analytics`, Muninn's dossier | 250 ms for a window of 20,000 runs, 240,000 seat rows and 240,000 checkpoints: the streamed scans measured 4.7 and 57 ms for the first two; the checkpoints, the parks' 64 ranges and the medians' passes are unmeasured, and slice 4 measures them before the number is final | Brokkr's 256 KiB: the accumulators of at most 64 groups a figure, each key at its bound, and the medians' histograms, 64 KiB at most (732 B and 2,769 B measured for the scans at both sizes). SQLite's about 1.5 MiB: the 256 KiB page cache and `GROUP BY`'s sorter, 1,050,320 to 1,273,184 B measured (Context); proposed, and slice 4 fixes it on its own measurement | the window's runs, seats and dated facts, the parks that overlap it or ended within their own length of its start, and the preceding window's finished runs, counted | `heap_aggregates.rs` at 2,000 and 20,000 runs, Brokkr's peaks equal to the byte and SQLite's within its share, over a distribution of 2,000 and of 9,000 distinct models, phases and realms in the window (each past 64 and the same number of digits in its "N more"), and over one whose every group key is 1 MiB beside one whose keys are cut to their bounds, Brokkr's peaks equal to the byte, its removal control a figure grouped by the whole phase, whose peak grows by the megabyte; `a_window_visits_only_the_parks_near_it`, in `brokkr-store`: one window held fixed while a year of later parks, and a year of older ones each shorter than half its distance from the window, are added, its park ranges' VM-step counts equal to the step, and then k parks added inside ruling 7's band, each ended before `from` within its own length, which grow the count by exactly k times what one adds, so the band is every earlier park the window visits, and then 10,000 parks of class 0 at `from - 1` and inside the window, zero-length and ended a year before they started, which leave the count equal to the step; its removal controls the one range on `COALESCE(ended_ms, 9223372036854775807) >= from`, whose count grows with the later parks, and a class-0 range on `at_ms >= from - 1`, whose count grows with the class-0 parks; a `fleet_aggregate` Callgrind bench over two journals whose windows hold the same facts and one of which adds a year of runs older than the preceding window, instruction counts equal; and `every_figure_is_its_whole_read` |

   **The aggregates are the one exception, and it is put to the
   operator.** Each answers a question about every run in a window: what
   the fleet cost, which models ran, which crossings Muninn's queue holds.
   A page of runs cannot answer it, so its time grows with what the
   window holds: the runs open or finished in it, their seats, and the
   checkpoints and parks inside it, as ruling 7 lists them, each read by
   one range of an index (analytics' day, week or month, `Range`,
   `crates/brokkr-view/src/analytics.rs:39`). Before the window it reads
   only the preceding window's finished runs, counted, and the parks that
   ended within their own length of its start (ruling 7's band). So its
   time grows with the journal's age only through parks long enough to
   reach that near the window; a park that ended further back than its
   own length, or reaches no later than its start, is never visited. Its heap does not grow with the
   window: rows are streamed and dropped, a grouped figure holds at most
   64 groups of bounded keys, a median holds two histograms of 64 counts,
   and SQLite's sorter spills past its floor. The binding of its output is
   `every_figure_is_its_whole_read`, in `brokkr-cli`: over every fixture
   journal and the generated one, at each `Range`, every figure from the
   projections equals today's `analytics` over the same runs read whole.
   The generated journal carries a park that starts and ends before a
   window's start, with its command stamped inside the window, after the
   run's next event, across a clock stepped back.
   Past 64 groups, the 64 kept and the "N more" row equal the first 64 of
   today's figure and the count of the rest. Past a key's bound, the
   figure equals today's over the same names cut to that bound. Its
   removal controls are a spend filtered by its seat's start rather than
   its checkpoint's time, a median taken from the window's first half,
   a `previous` counted over the window itself rather than the one
   before it, and a park classed and filtered by `ended_ms` alone, which
   loses that park's waiting time.

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
      the `lane` and `order_at` key and the acknowledgement facet; ruling
      2's bounds, `create_run`'s run id check and the cut first line with
      `a_cut_first_line_titles_as_its_feature`; the acknowledgement's cut
      copy on the row; the parse moved into
      `brokkr-core`; the `behind` range read first and its catch-up at a
      writer's open; ruling 5's check; ruling 6's merged paging in the
      console, `brokkr runs`, `/api/runs` and Muninn's queue; ruling 4's
      test and doctor check; ruling 9's fleet heap test with its wide rows,
      its page at every bound and SQLite's share read beside dhat's, its
      plan and visited-row test, its sections test, its lagging-row test
      and its bench. Acceptance: the
      fleet's budgets hold on the generated journal; `brokkr runs --json`
      lists every fixture journal's runs with the same rows as today,
      ordered as ruled; an unmigrated journal reads through the kept
      read. It may land as two pull requests, the store's half first.
    - **Slice 2, the run and seat views.** `seat_summary` and
      `event_index`; the run view from the trail and the seat rows; the
      seat view's checkpoint pages; a run's chain verified streaming at
      its opening, which each view makes first (ruling 5). Acceptance:
      every run view and seat view equals today's on every fixture
      journal, `a_corrupt_run_is_refused_wherever_it_is_opened` holds for
      them, and ruling 9's run view, seat view and opening budgets hold,
      bound by `heap_run_view.rs`, `heap_seat_view.rs` and `heap_open.rs`.
    - **Slice 3, `inspect`, `seats`, `watch`, `transcript`, the console's
      view routes, `costs` and `compare`** from the same projections, each
      after its opening. Acceptance: their output equals today's on every
      fixture journal, each refuses a corrupt run as today by
      `a_corrupt_run_is_refused_wherever_it_is_opened`, and ruling 9's
      `costs` budget holds, bound by `heap_costs.rs`.
    - **Slice 4, the aggregates:** `run_summary`'s aggregate columns and
      `event_index`'s dated facts (ruling 7), with the readings of
      `facts.rs` and `attempts.rs` moved into `brokkr-core`; `analytics`
      and Muninn's dossier from the rows, with the group keys cut where
      they are stored, the medians selected in passes, the 64-group bound
      counted in SQLite, the parks read by span class and the preceding
      window counted; and the run selectors (decision 0015) by a key range
      on `runs`. `graph` is not in it (ruling 7). Acceptance: their output
      equals today's on every fixture journal,
      `every_figure_is_its_whole_read` holds past 64 groups and past a
      key's bound, `a_window_visits_only_the_parks_near_it` holds, and
      ruling 9's aggregate budgets hold, Brokkr's and SQLite's, bound by
      `heap_aggregates.rs` and the `fleet_aggregate` bench, with SQLite's
      share fixed on its measurement.

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
4. **Integrity at fleet level and in the aggregates.** The head check
   alone, with the full chain
   check at a run's opening, which every readout of one run makes first
   (`inspect`, the run view, the seat view, `costs` and `compare`, each
   refusing a corrupt run as today), at every verb that acts on a run,
   and in `brokkr doctor` (ruling 5). Or: the console also verifies runs whole
   off its input loop, one an idle tick, marking each row with when it was
   last verified. Or: every body verified at the first fleet read, as
   0088 ruling 5 has it, which keeps the first read growing with the
   journal.
5. **The row's bound.** Every column a page selects is bounded (ruling
   2's table): the run id to 128 bytes, which `create_run` gains as
   `import` holds it, and a row past it refused at the edge; names to 64
   bytes, the park reason to 256, the refusal's words to 512, the
   feature's first line to 1,024, stamps to 40, the acknowledgement's
   operator and reason to 64 and 256 characters, copied cut onto the row
   by its trigger; the residuals
   as their highest open severity and a count. The whole values are the
   row's last columns, read by key for `brokkr runs`, `--json` and the
   run view. A value past its bound shows cut, with an ellipsis, in the
   console: an output change no fixture journal reaches. Or: no lossless
   column in the row, and `--json` derives the list from the run's ruling
   events, which waits for slice 2's `event_index`. Or: other bounds.
6. **The page key.** A generated `lane` column, with `acknowledged` kept
   by triggers on the events and the acknowledgements, and a generated
   `order_at` from the time the writer parsed, so every range is an index
   range that visits only what it returns and compares times, never text
   (rulings 3 and 6). The lane's `CASE` is a second, store-side reading of
   which section a row's range is, beside `section_of`: a bend of
   decision 0071 ruling 7, confined to choosing ranges and held to
   `section_of` by `every_page_is_its_sections`. Or:
   a partial index per section, six indexes for one key. Or: the
   acknowledgement read per row, as the earlier draft had it, whose cost
   grows with the acknowledged runs (21,015 to 191,115 VM steps,
   measured).
7. **Rows behind their head.** Read first on every page and folded before
   they are placed, at most a page and one ahead per refresh with the rest
   counted, and caught up at every current writer's open (rulings 3 and
   6). Or: listed unfolded in "needs you" as behind, with no verdict until
   a writer catches them up.
8. **The order within a section.** Last activity, newest first, an
   unreadable time first, then run id, with a section's lanes merged
   into that one order (ruling 6). Or: creation, newest first, as today,
   which costs a sort of a day's finished runs per page.
9. **`brokkr runs`.** Every run by default, a page at a time, today's
   JSON shape, and `--limit` and `--after` with `next` and
   `VIEW_VERSION` 24 (ruling 6). Or: one page by default, which changes
   what the console plugin reads.
10. **The older runs' count.** Kept, counted by the index alone (ruling
    6). Or: "more older runs" with no number, which costs nothing.
11. **The budgets.** 50 ms to the first fleet page, warm and measured;
    live heap per page as Brokkr's share and SQLite's, 256 KiB held by
    dhat and 400 KiB by SQLite's own high-water, at two sizes with wide
    rows on the page, and 640 KiB of Brokkr's for a page whose every row
    fills every bound, proposed and unmeasured; a 256 KiB page cache;
    RSS within 1 MiB of the floor; each range's plan pinned and its visited rows
    bound; and the
    run view, seat view, `costs` and opening budgets of ruling 9's table,
    each bound in its slice, whose SQLite share holds the largest
    envelope it reads, bounded by the run. Or: other numbers.
12. **The aggregates' exception.** `analytics` and Muninn's dossier read
    the dated facts ruling 7 names, so their output stays today's. They
    hold 256 KiB of Brokkr's heap whatever the fleet's size, with at most
    64 groups a figure, each key cut where it is stored, and the rest as
    one "N more" row: an output change only past 64 distinct groups in a
    window or past a key's bound. The medians are selected in Brokkr by
    counted passes. The group counts are read in SQLite, a bend of
    decision 0071 ruling 7 held by `every_figure_is_its_whole_read`, and
    SQLite's share is the page cache and `GROUP BY`'s sorter, about
    1.5 MiB, proposed and fixed in slice 4. Their time grows with what
    the window holds, its checkpoints included, with the preceding
    window's finished runs, counted, and with the parks that ended within
    their own length of its start, never with anything else before it: 250 ms
    for a window of 20,000 runs, 240,000 seats and 240,000 checkpoints,
    proposed, its scans measured at 57 ms and its checkpoints, parks and
    passes to be measured in slice 4 (rulings 7 and 9). `graph` waits for
    a decision of its own. Or: an explicit memory exception in place of
    the 64-group bound, about one accumulator per distinct group, which
    keeps every figure's output today's. Or: the sorter's floor lowered
    by `SQLITE_CONFIG_PMASZ`, which measured 371,608 to 635,200 B at 8
    pages but grew with the window's rows (Context). Or: a kept aggregate
    per hour, written at append, a fourth projection that bounds their
    time by the window's hours, at the price of windows whose edges fall
    on the hour, an output change. Or: `graph`'s projection ruled here,
    before slice 4.
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
- The aggregates are the one readout whose time grows with what it
  counts: the runs, seats, checkpoints and parks inside its window, the
  preceding window's finished runs, and the parks that ended within their
  own length of its start, nothing else before it. Their
  memory does not, beside the sorter's floor of about a megabyte in
  SQLite, and a figure past 64 distinct groups prints the rest as one
  row.
- `brokkr graph` still reads every run whole, until a decision of its own
  rules its projection.
- A value past its column's bound shows cut in the console; `brokkr runs`
  and `--json` print it whole.
- The journal file grows by a row per run, a row per participant and a
  row per event, the last about a tenth the size of a checkpoint's row,
  estimated from its columns and measured in slice 2.
- Each append costs a few microseconds more for its rows (measured with
  fsync off), beside a commit's sync of milliseconds.
- An older binary writing to a migrated journal keeps the model's
  presence, order, acknowledgement facet and index current through the
  triggers; its runs read whole until a current writer opens the journal.
- A corrupt event below a run's head is no longer refused in the fleet. It
  is refused when the run is opened, by any readout of that run with
  `inspect` among them, and when it is exported, resumed or swept by
  `brokkr doctor`.
- The kept read of decision 0088 stays as the fallback for a journal
  without the model, and so for as long as a read-only reader can meet
  one.
- Nothing under `contracts/`, `fixtures/`, `policy/` or `reference/` moves,
  and decision 0014 holds: every reader opens read-only and writes
  nothing to the journal.
