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

  A scripted entry names one operation or point and the occurrence at which
  it fires. It does one of two things. Either it returns an I/O error in
  place of that operation's result, or it runs a test-owned filesystem change
  inside the test's synthetic home before the real operation proceeds.
  Absence, replacement and a changed identity are produced only by real
  changes, so the production code classifies a genuine OS answer. The seam
  never hands the reader a handle, path, name, file type or identity.
- Compile the seam only in the `brokkr-cli` unit-test configuration. It
  reads no environment variable, argument, configuration key, file or
  journal value. A reference to it outside that configuration fails to
  compile, which the release binary, clippy's non-test targets and the
  integration-test builds all exercise. A plan is scoped to the thread that
  installed it. A scripted entry that never fires fails its test.
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

The `lib.rs` hands-route lines (35 lines, 3 functions) are skipped only
because this nested box sets `BROKKR_HANDS_BOX`. They stay pending host
proof. They are not a seam matter.

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
  recorded as done in tracked checkboxes.

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
held.

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
keeps one entry in each enumerated directory, or restricts a child-open
selector to a name. The guard fails the test if any entry never fired, so an
ordinal that drifts cannot pass silently.

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
