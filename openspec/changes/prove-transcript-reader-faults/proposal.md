# Change: Prove the transcript reader's failure handling (#222)

## Why

Issue #222's reader is specified, archived and repaired. Chief review 7505
ruled `spec_defect=false`, and all 22 of its findings have repairs at
`5738889`. Two things still keep the issue open.

First, the exact-coverage gate is not closed. The local measurement at
`5738889` (`target/coverage/local-lcov.info`) leaves 22 lines and 19
branches uncovered in reader-owned files. Most of them are the reader's
handling of real filesystem failures and races: an enumeration that fails,
a candidate identity that cannot be read, a bounded read that fails, an
entry that vanishes between enumeration and open, an acquisition that
changes between discovery and the read. The settled requirements already
fix the outcome of each case (`unreadable`, `not-found`, fail-closed
acquisition). What is missing is a way for a test to reach that handling.
The predecessor stopped at IMPL-BROKEN-TWICE because it could close these
lines only by removing the handling, which the gate forbids, or by adding a
fault-injection seam, which no artifact describes. A seam landed without a
contract risks becoming an unreviewed fault switch in production, so this
change writes the contract first.

Second, the archived design says a transcript read "must not create WAL
sidecars" (`design.md:699-700`). That is not true and cannot be made true
safely. Result `5fd1387e` measured it on the Rust store, and this
specification re-probed it with SQLite 3.46.1: a read-only open of a
quiescent write-ahead-log journal that has no sidecars creates a zero-byte
`-wal` and a 32,768-byte `-shm`. The database bytes and hash stay unchanged.
The tracked privacy tests
(`crates/brokkr-cli/tests/transcript_privacy.rs`, `journal_and_wal`,
`assert_journal_inert`) assert the weaker invariant that actually holds:
the database is unchanged, a new `-wal` holds no frame, and an existing
`-wal` is left alone. The living requirement "Transcript prose stays local
and inert" never mentions sidecars. The text and the tests have to agree,
and this change makes them agree in the open.

## What Changes

- Add one narrow, test-only fault seam at the handle-based reader boundary
  in `crates/brokkr-cli/src/ui/safe_fs.rs` and its callers in `ui.rs`. It
  can target these operations and points:
  - directory enumeration (`Dir::entries_bounded`);
  - child open (`Dir::child`);
  - handle identity (`OpenedFile::identity`);
  - bounded read (`OpenedFile::read_bounded`);
  - the point inside a child open between its directory attempt and its
    file attempt, present in both the unix and the Windows implementation;
  - the point before the read-boundary acquisition re-walk.

  A scripted entry names one target and the occurrence at which it fires,
  counted per target on the installing thread from one. Each target accepts
  exactly one kind of action:
  - enumeration, identity and bounded read accept only an injected I/O
    error of kind `Other`, returned in place of the operation's result;
  - child open and both points accept only a test-owned filesystem change
    inside the test's synthetic home, after which the real operation runs.

  Absence, replacement and a changed identity are produced only by real
  changes, so the production code classifies a genuine OS answer. No
  injected error reaches the code that classifies absence or a file type.
  The seam never hands the reader a handle, path, name, file type or
  identity.
- Compile the seam only in the `brokkr-cli` unit-test configuration. It
  reads no environment variable, argument, configuration key, file or
  journal value. A reference to it outside that configuration fails to
  compile, which the release binary, clippy's non-test targets and the
  integration-test builds all exercise. A plan is scoped to the thread that
  installed it. A scripted entry that never fires fails its test, and no
  entry can be disarmed or exempted from that check.
- Drive every listed miss through the route that really reaches it, and
  assert the specified outcome, not merely the line. Re-measure first; the
  table below is the local measurement at `5738889` with each arm's route as
  read from the code.
- Pass `HOME` into the Claude browser lookup as a parameter, following the
  existing `read_local` / `read_with_home` split. The missing-home arm is
  then tested without mutating the process environment, which tests running
  on two threads share.
- Carry the proven journal invariant into the living "stays local and inert"
  requirement: a read writes no journal content. The design for this change
  supersedes the archived wording at `design.md:699-700`, and the privacy
  tests compare an existing `-wal` by digest, not only by length. A missing
  journal still gains no file of any kind. A proposed addendum to decision
  0055 records the clarification and adds the seam to ruling 5's enforcement
  binding. It stays `proposed`.
- Finish the rest of #222 on this branch:
  - name and re-run the tracked regression test for each of M1–M11 and
    L1–L11 on the final head;
  - carry the security residual flag until a review clears it on evidence;
  - bring over the integration branch's inode-fixture and portable-fixture
    repairs (`controller-inode-fixture-repair.json`, `c7ff0fa`, `10908b6`,
    `82c802a`, `05ab44b`) where they still apply to source files, without
    touching `../brokkr-222-integration` or `main`;
  - run every local gate and the Rust 1.88 all-targets check;
  - end with an independent clean final review.

### Measured misses and their routes

| Site at `5738889` | Arm | Route |
|---|---|---|
| `ui.rs:261` branch | discovery limit hit with two or more candidates | ordinary fixture: two valid DSH roots plus an oversized header gives `ambiguous-source` |
| `ui.rs:301-302`, `392-393`, `488-489`, `614-615`, `636-637` | `bounded_entries` I/O error at the Claude root, the Codex walk and the DSH seat and project directories | scripted enumeration error |
| `ui.rs:377` branch | re-walk finds the same identity at a different path | real rename at the re-walk point |
| `ui.rs:420`, `511`, `668` | candidate identity error during discovery | scripted identity error |
| `ui.rs:443`, `445` branches | Codex token at filename start or end | ordinary fixture: ids `rollout-…` and `jsonl` are in the Codex language, so the whole-token rule applies as specified |
| `ui.rs:516` branch | directory deeper than six Codex levels | ordinary fixture |
| `ui.rs:521` | Codex entry absent at open | real removal between enumeration and open |
| `ui.rs:552` | DSH opening-header read error | scripted read error |
| `ui.rs:613` branch | DSH walk ends with no directory | unreachable: `split('/')` yields at least one component, and every loop pass either returns or sets the directory. Restructure and record the proof. |
| `ui.rs:743-744` | acquisition no longer current | real rename or replacement at the re-walk point |
| `ui.rs:750-751` | retained-handle identity recheck fails | scripted identity error on the retained handle |
| `ui.rs:758` | bounded source read fails | scripted read error |
| `ui.rs:827-828` | no `HOME` for the Claude browser lookup | parameterized home |
| `ui.rs:990` branch | empty run component before decoding | ordinary request `/api/presentation//<key>` |
| `ui.rs:998-999` | empty component after decoding | unreachable: `decode_component` turns each non-empty component into a non-empty string or refuses it, and the pre-decode check already refused empty components. Remove and record the proof. |
| `ui.rs:1003` | journal load error | ordinary request for an unknown run: `Store::load` returns `RunNotFound` |
| `ui/safe_fs.rs:193` branch | file attempt finds the entry absent | real removal between the directory attempt and the file attempt |
| `tui.rs:3156` branch | `refused_lines` with no reason | unreachable at its only call site, which follows the readable guard. Carry the reason in the signature and record the proof. |
| `tui.rs:3330-3334` | open transcript door when the fresh read is not readable and invalidation did not fire | a state transition, not I/O. Reach it through `drive`'s existing `source` parameter, or prove it unreachable and restructure. |
| `brokkr-view/src/transcript.rs:1974` branch | quiet DSH row without `seq` | ordinary fixture |

### Local misses outside the reader

The same local measurement at `5738889` leaves 189 lines, 31 branches and 9
functions uncovered in total. The reader rows above account for 22 lines and
19 branches. The other 167 lines, 12 branches and 9 functions are in files
this change does not touch, and none of them is a seam matter:

| Site at `5738889` | Lines / branches / functions | Why this box misses it |
|---|---|---|
| `brokkr-cli/src/lib.rs:630-675`, `2560` | 35 / 0 / 3 (`hands` and its two closures) | the `brokkr hands serve` and `exec` routes; their tests return early under `HANDS_BOX_ENV` (`tests/hands.rs:34`, `src/tests.rs:3386`) |
| `brokkr-protocol/src/hands.rs:778-779`, `926-1062` | 122 / 6 / 6 | `require_bwrap_for`, `execute`, `execute_in` and `run_boxed`; their tests need a new namespace and skip under `HANDS_BOX_ENV` (`hands/tests.rs:14`, `tests/hands.rs:34`) |
| `brokkr-protocol/src/hands.rs:264` | 0 / 1 / 0 | the no-identity arm of `git_facts`, which depends on the git configuration the test process sees |
| `brokkr-cli/src/doctor.rs:94-103` | 2 / 1 / 0 | `probe_in_box`; its test skips under `HANDS_BOX_ENV` (`doctor/tests.rs:458`) |
| `brokkr-runtime/src/engine.rs:3745-3749` | 4 / 1 / 0 | the unboxed-dispatch layer re-walk at spawn; its tests skip under `HANDS_BOX_ENV` (`engine/boundary_tests.rs:933`, `1752`, `1854`) |
| `brokkr-runtime/src/engine.rs:2281-2282`, `2266`, `2369` | 2 / 2 / 0 | the sequence fence for a malformed `change`. `engine/tests.rs:493` looks as though it reaches it and has no box guard, so the cause here is not established |
| `brokkr-runtime/src/bundle.rs:2372` | 1 / 0 / 0 | a `walk_files` error inside `layer_drift`; the cause here is not established |
| `brokkr-runtime/src/realms.rs:296-297` | 1 / 1 / 0 | `house_for` with no selected realm; the cause here is not established |

Host evidence: the host exact-coverage gaps at `9191336`
(`controller-coverage-gaps.json`) list no line or branch gap in `hands.rs`,
`engine.rs`, `doctor.rs`, `bundle.rs`, `realms.rs` or the `lib.rs` hands
route. Each of those files, and the `lib.rs` region, is byte-identical
between `9191336` and `5738889`. That artifact does not itemize functions,
so no host evidence reachable here covers the 9 functions. All of these
stay pending host proof on the final head. None of them is recorded as
proved by a box run that skipped its test.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `transcript-reading`: add the requirement that the reader's failure
  handling is proved through a unit-test-only seam that holds no production
  fault switch and never bypasses the boundary. Restate "Transcript prose
  stays local and inert" with the measured journal invariant and two
  scenarios. No other settled requirement or scenario changes.

## Impact

- Code: `crates/brokkr-cli/src/ui/safe_fs.rs`, `ui.rs`, `tui.rs` and their
  unit-test modules; `crates/brokkr-view/src/transcript.rs` tests;
  `crates/brokkr-cli/tests/transcript_privacy.rs` for the digest check;
  other `tests/transcript_*.rs` files only to bring over fixture repairs.
- No new production or registry dependency, no Cargo feature and no
  `Cargo.lock` change. `scripts/coverage-exact.sh` stays byte-identical.
- Frozen surfaces stay byte-identical: `contracts/`,
  `policy/phase-machine.json`, `policy/schemas/`, `reference/`,
  `fixtures/`.
- Docs: the proposed addendum to decision 0055. A `CONTRIBUTING.md`
  coverage note tells contributors how to reach a reader error arm through
  the seam.
- Out of scope: issue #226's engine and adapter resumption, launch
  evidence, sandbox re-imposition and provider credentials; browser body
  routes for Codex or DSH; running any hint or provider.
- Pending outside this branch: host exact coverage with `TMPDIR=/var/tmp`
  and `BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1`, remote CI on the final head,
  integration and publication. These stay controller handoff and are never
  recorded as done in tracked checkboxes. The host run is also the proof
  still owed for every non-reader miss listed above.

## Decisions

### S1 — A new change, not an edit of the archived one

The archived change and its three living specs are settled. The chief found
no specification defect, so none is reopened to avoid a repair. This change
adds one requirement and restates one requirement with a clarification the
measurement forces. The archived `design.md` and `tasks.md` stay byte-identical.
This change's design supersedes their sidecar wording by reference, and the
superseding is stated, not silent.

### S2 — Real changes where they exist, injected errors only where they do not

A forged `Absent` or a forged identity would prove handling of a state the
production code never computes. Where a portable, deterministic filesystem
change reaches an arm, the seam runs that change and lets the real OS answer
flow through the production classification. That covers:

- removal between enumeration and open;
- removal between the directory and file attempts;
- rename or replacement before the re-walk.

These work on unix, macOS and Windows: the reader's handles are opened with
`FILE_SHARE_DELETE` on Windows, so renames succeed while the handles are
held. Each of these changes has to fall between two reader operations, so the
seam times it. The seam does not replace it.

Errors from enumeration, identity and bounded read are injected because no
portable deterministic real fault exists:

- On an open unix descriptor `fstat` has no reachable failure, and on
  Windows the file identity is cached at open.
- A read of an open regular file fails only on device error.
- Linux's `ENOENT` from enumerating a removed directory does not reproduce on
  macOS or Windows.

### S3 — Absence at compile time, not a runtime switch

Rejected alternatives:

- An environment variable or configuration key. Any caller could set it in
  production.
- A Cargo feature. A packager can enable a feature in a release build, and
  the gate's `--all-features` builds would carry it outside tests.
- A generic filesystem trait threaded through discovery. That puts an
  alternative filesystem implementation into production signatures, which
  could hand the reader paths or handles around the boundary.
- A `cfg(not(test))` no-op twin. It is a carve-out-shaped double definition
  that the gate rules out.

A module declared only under `cfg(test)` does not exist in any other build,
so a leaked reference is a compile error.

### S4 — Thread-scoped plans, occurrence selectors, a guard that fails on an unfired entry

Tests run on two threads (`RUST_TEST_THREADS=2`), so a process-wide plan
would leak into a concurrently running read. The reader is sequential on its
calling thread, so a selector made of an operation or point and its
occurrence on that thread is deterministic. Enumeration order is set by the
filesystem, so a fixture whose selected occurrence would depend on that order
keeps one entry in each enumerated directory. An entry names no file, so the
seam holds no name to match. The guard fails the test if any entry never
fired, so an ordinal that drifts cannot pass silently.

### S5 — The journal invariant is amended to what is proved

A read-only open that creates no sidecar would need one of these, and each is
rejected:

- `immutable=1`: SQLite then assumes the file never changes and takes no
  locks, which is unsafe for a live journal that a run may be writing.
- Exclusive locking mode: it would block the writer of a live run.
- A VFS that refuses to create the files: the open then fails for every
  quiescent journal, which breaks every read surface.
- Deleting the sidecars after the read: that races a writer that has just
  opened them.
- Reading a copy: a copy of a live database made without its locks is
  inconsistent.

`Store::open_read_only` is also the shipped open that the `ui` server, the
TUI, `ledger` and Muninn already use. The transcript reader adds no new
sidecar behavior. The honest invariant is the one the tests assert: no journal
content changes. The restated requirement says that and pins it with two
scenarios. The existing `-wal` comparison moves from length to digest.

### S6 — The seam stays at the filesystem boundary

`tui.rs` has no filesystem boundary of its own. It consumes the shared
`TranscriptRead`, and its two misses are state-machine arms. They are closed
through `drive`'s existing `source` parameter, or removed with a proof. The
missing-`HOME` arm is closed by passing the home as a parameter, the same
split `read_with_home` already uses. It is not scripted. Keeping the seam at
the handle operations keeps its surface small enough to review.

### S7 — The coverage gate is unchanged and counts the seam

`scripts/coverage-exact.sh` leaves only `tests.rs`, `*_tests.rs` and `tests/`
out of the report, and it refuses exclusion attributes. The seam lives in
production source under `cfg(test)`, so the gate counts its lines and branches,
and they must reach 100% like everything else. Giving the seam a
harness-shaped file name to keep it out of the count would be an exclusion by
another name, and it is rejected. This follows the principle of proposed
decision 0050, that the machine proves what it promises: the reader promises
to fail closed on I/O errors and races, and after this change a test proves
each promise. Decision 0050 is still proposed and rules on the phase machine.
It is cited here for its principle, not as authority.

### S8 — Each target accepts one action (clarify Q1)

The re-walk point has no operation result to replace:
`acquisition_is_current` returns a `bool`. The operation after the point
inside a child open is the file attempt. The platform code classifies that
attempt's raw error into `Absent` (`ENOENT`, the absent NT statuses) or
`Unsafe` (`ELOOP`, `EMLINK`, `EISDIR` and the matching NT statuses)
(`safe_fs.rs:190-197`, `476-483`). An error injected there would forge the
very absence or file type that S2 forbids. So both points and child open take
only a real change. Enumeration, identity and bounded read take only an
error. That is exactly the set S2 argues has no portable real fault. No
listed miss needs an error at child open: the measurement already covers
its non-absent error arms. The injected kind is `Other` because no reader
caller maps it to absence. `root_error` maps `NotFound`, and the root open is
not a seam target anyway. The seam's constructors carry the pairing, so a
wrong pairing cannot be written at all. It is not checked when the test
runs. The spec's opening paragraph now orders the routes as a preference:
ordinary input, then real change, then injected error. It no longer calls the
seam a fallback, because the race changes need the seam to time them.

### S9 — The boundary-in-force proof fires its entry after the asserted reads (clarify Q2)

The scenario could not be met as first written. An entry that never fires
fails its test. An entry that fires inside an asserted read changes that
read's outcome: a fired identity error sets `io_seen`, which outranks the
sole candidate (`ui.rs:269-271`). The scenario now puts the entry's
occurrence in a separate, otherwise valid lookup made after the asserted
reads, and asserts that lookup's `unreadable`. The unfired-entry check runs
unchanged. A drift in either direction fails the test. The requirement also
states that no entry can be disarmed or exempted, so the proof cannot be
bought with a guard exemption.

### S10 — The WAL scenario names the command's reads, not a growth watch (clarify Q3)

The browser's growth watch is the Claude `/sse/session/<id>` stream. It polls
`claude_source_size` and never opens the store (`ui.rs:1100-1139`), so a
journal scenario over it would prove nothing. The surface meant, and the one
the tracked proofs exercise, is the transcript command:

- a successful read and a refusal:
  `reading_leaves_the_journal_and_the_retained_file_unchanged`;
- a repeated read after the retained source grows:
  `growth_reads_keep_the_tree_config_and_journal_inert`.

The scenario now names those three reads. The invariant belongs to
`Store::open_read_only`. The browser server's routes open the journal
through that same call (`ui.rs:90`, `1023`), and so do the TUI's views
(`lib.rs:953`) and the command (`lib.rs:1388`). Proving it once per surface
would re-prove one function, so the scenario does not multiply surfaces.

### S11 — Every local miss is accounted for, and host proof is still owed (clarify Q4)

The first draft named only `lib.rs`. The table above now lists every
non-reader local miss, with the guard named wherever the box cause was
traced. Three sites have no established box cause (`engine.rs` sequence
fence, `bundle.rs:2372`, `realms.rs:296-297`). Their files match `9191336`,
whose host gaps list none of them. The implementation's first act is still a
fresh measurement.

Verification applies one rule to the fresh measurement:

- A non-reader miss is recorded as pending host proof, with its box cause.
- A non-reader miss with no box cause is owned by this change. So is one that
  the final-head host exact-coverage run leaves uncovered. An owned miss is
  repaired like a reader miss, and it is never deferred as a residual.
