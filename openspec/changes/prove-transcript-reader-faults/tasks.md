# Tasks: Prove the transcript reader's failure handling (#222)

Groups are the design's landing order (the Migration Plan): the proposed
0055 addendum filed before any production edit, then the seam module
itself, then the seven hook statements with their scripted-error and
real-change tests, then the restructures and the `claude_source` home
parameter, then the remaining ordinary fixtures, then the journal-inertness
tests, then the read-only integration fixture ports, then coverage
re-trace and every gate, then the contributor note, the archive fold and
the final commit, with the independent final review as post-commit evidence
of the head that commit fixes. This is the order another smith executes
them in: a later
group's tests exercise hooks and restructures the earlier groups land, and
the closing gates and archive must see every earlier group's tests green
before they run.

Every task names the requirement it serves as `transcript-reading /
<Requirement>`. Group 1, group 7 and group 9 serve every requirement of
this change, because a filing prerequisite, the read-only integration
inspection and the closing gates, fold and commit are not requirements of
their own.

Conventions binding on every task below, restated once rather than per
task:

- No frozen byte is edited: `contracts/`, `policy/phase-machine.json`,
  `policy/schemas/`, `reference/` and `fixtures/` keep their bytes.
- No accepted decision is edited. The addendum to `docs/decisions/0055-…`
  stays `Status: proposed`; no other ruling changes and no other decision
  number is taken. The archived `design.md` and `tasks.md` at
  `openspec/changes/archive/2026-09-10-read-every-transcript-kind/` stay
  byte-identical.
- The seam (`crates/brokkr-cli/src/ui/safe_fs.rs`, `mod fault`) is the only
  new production code. It is declared and called only under `#[cfg(test)]`,
  builds no `Child`, `Identity`, handle, path, name or file-type value, and
  reads no environment variable, argument, configuration key, file or
  journal value. A change closure runs only inside the test's own tempdir.
- Tests are written with the code they prove, in `ui/tests.rs`,
  `tui/tests.rs`, `crates/brokkr-view/src/transcript.rs`'s own test module,
  or the `tests/transcript_*.rs` file the surface already uses. Related
  arms are grouped into table-driven tests rather than one bespoke function
  per row.
- Every fixture keeps one entry per enumerated directory whose count a
  scripted occurrence depends on, so enumeration order cannot make an
  occurrence drift silently (S4).
- Cargo commands run with `CARGO_BUILD_JOBS=2` and `RUST_TEST_THREADS=2`.
- Nothing in `crates/brokkr-protocol/src/adapters.rs`, the engine's
  resumption argv or the sandbox boundary is changed: #226 owns those.
- `../brokkr-222-integration` and `main` are never written.

## 1. The proposed 0055 addendum, filed before any production edit (D10)

- [x] 1.1 Append an addendum to `docs/decisions/0055-read-every-transcript-kind.md`,
      dated on filing (the addendum commit's own date, as addenda 0035 and
      0042 carry theirs; not guessed ahead of it and not re-derived if a
      later commit moves), `Status: proposed` throughout: it records the
      journal-inertness clarification (a read-only open of a quiescent
      journal may create an empty `-wal` and a `-shm`, an existing `-wal`
      keeps its digest, a missing journal gains no file) superseding
      archived `design.md:699-700`'s sidecar sentence, and it adds the
      unit-test-only reader fault seam (bounded as this change's D2-D4 and
      D7) to ruling 5's enforcement binding. No other ruling or the
      registry row's status changes — every requirement of this change.
- [x] 1.2 Run `cargo test -p brokkr-cli --test decisions_index` and leave it
      green — every requirement of this change.
- [x] 1.3 Land 1.1-1.2 before the first production edit of group 2, and do
      not revisit 0055's rulings later in this change to match an
      implementation that drifted — every requirement of this change.

## 2. The fault seam module and its own tests (D2, D4, D8)

- [ ] 2.1 Add `#[cfg(test)] pub(crate) mod fault` to
      `crates/brokkr-cli/src/ui/safe_fs.rs`: the disjoint `FailAt` (`Entries`,
      `Identity`, `Read`) and `ChangeAt` (`Child`, `ChildFileAttempt`,
      `BeforeRewalk`) enums; `Plan` with private fields and the consuming
      builders `new`, `fail(FailAt, occurrence: usize)`,
      `change(ChangeAt, occurrence: usize, impl FnOnce() + 'static)`, with
      no `clear`, `skip`, `disarm` or exemption method; `Plan::install`
      storing the plan in a `thread_local!` `RefCell<Option<_>>` with a
      per-target occurrence counter starting at zero, panicking if a plan is
      already stored on that thread — transcript-reading / Reader failure
      handling is proved through a test-only fault seam.
- [ ] 2.2 Add the opaque `Guard` (`PhantomData<*const ()>`, so `!Send`) with
      a `Drop` that always takes the plan out of the thread-local first,
      returns without asserting when `std::thread::panicking()` is true, and
      otherwise asserts every entry fired, naming each unfired target and
      occurrence in the panic message — same requirement.
- [ ] 2.3 Add the two hooks: `fault::fail(FailAt) -> Option<io::Error>`,
      returning `Some(io::Error::other("scripted reader fault"))` exactly
      when the plan holds an unfired entry at that target's incremented
      occurrence and `None` otherwise (including with no plan installed);
      `fault::point(ChangeAt)`, taking the matching entry's closure out of
      the `RefCell`, releasing the borrow, then running it, and being a
      no-op when no entry matches — same requirement.
- [ ] 2.4 Add the seam's own tests to `ui/tests.rs` (their `#[should_panic]`
      tails are not counted by the coverage gate): a plan whose entry never
      fires fails with the unfired message; a nested `install` on one thread
      fails with the refusal, and the same test's unwinding proves the
      unwinding arm clears the plan without aborting the test binary; a test
      that spawns a second thread proves that thread's read at the scripted
      occurrence sees only real results, and that the installing thread's
      same occurrence is unaffected once its guard drops — same requirement.
- [ ] 2.5 Confirm from this group's tests that every arm in the design's D8
      table is reached: no plan installed; plan installed with no match at
      this count; a `fail` match; a `point` match; the guard's all-fired,
      unfired and unwinding paths; the `install` refusal. Record any arm
      this group alone does not reach, so group 3's hook tests close it —
      same requirement.
- [ ] 2.6 Add `#![deny(clippy::mem_forget)]` to
      `crates/brokkr-cli/src/lib.rs`, so a `Guard` passed to `mem::forget`
      anywhere in the `brokkr_cli` library crate — `ui/safe_fs.rs` and
      `ui/tests.rs` included — fails the clippy gate, for every test
      written against the seam after this change lands (D4.5). That crate
      root is the whole scope the lint needs: the bin target is a separate
      crate root, and the seam is `cfg(test)` in the library. Confirm
      first that the pinned toolchain's clippy carries the lint under that
      name; if it does not, record the name it does carry for this pattern,
      or record that the D7 inspection's no-leak clause stands alone — as a
      finding to repair, never as a silent gap. Confirm from a workspace
      grep for `mem::forget`, `ManuallyDrop` and `Box::leak` over `crates/`
      that the lint denies a pattern no code uses, so it needs no
      suppression, no dependency and no `Cargo.lock` change — same
      requirement.

## 3. The seven hook statements, driven by scripted errors, real changes and one ordinary fixture (D3, D5, D6)

- [ ] 3.1 Add the `Entries`, `Identity` and `Read` hook statements to the
      platform-independent wrappers `Dir::entries_bounded` (`safe_fs.rs:70`),
      `OpenedFile::identity` (`:91`) and `OpenedFile::read_bounded` (`:103`):
      on `Some(error)` from the matching `fault::fail` call, return
      `Err(error)` before the real operation runs — transcript-reading /
      Reader failure handling is proved through a test-only fault seam.
- [ ] 3.2 Add the `Child` hook statement in `Dir::child` (`:75`), before
      `self.inner.child`, calling `fault::point(ChangeAt::Child)`; add the
      `ChildFileAttempt` hook statement in the unix `imp::Dir::child`
      (`:184`) and the Windows `imp::Dir::child` (`:469`), each after the
      directory attempt's non-absent failure and before the file attempt,
      calling the same `fault::point(ChangeAt::ChildFileAttempt)`; add the
      `BeforeRewalk` hook statement as the first statement of
      `acquisition_is_current` (`ui.rs:373`), calling
      `safe_fs::fault::point(ChangeAt::BeforeRewalk)` — same requirement.
- [ ] 3.3 Scripted-error tests: an `Entries` error on the Claude projects
      root (`ui.rs:301-302`, `392-393`), the Codex `sessions` walk
      (`488-489`), and the DSH seat directory (`614-615`) and its one
      project directory (`636-637`), each asserting discovery-stage
      `unreadable` with null path, no turns, zero counts, no truncation, no
      notices, the kind's unresolved `full_session` hint, and that browser
      presentation for the same reference reports `admitted: false` with
      reason `unreadable` — same requirement.
- [ ] 3.4 Scripted-error tests: an `Identity` error on each kind's sole
      matching candidate (`ui.rs:420`, `511`, `668`), run once alone and once
      with a second qualifying sibling present, asserting discovery-stage
      `unreadable` with no candidate admitted in either case — same
      requirement.
- [ ] 3.5 Scripted-error test: a `Read` error on the only DSH candidate's
      opening-header read (`ui.rs:552`), asserting discovery-stage
      `unreadable` with null path, null DSH hint, no turns and zero counts —
      same requirement.
- [ ] 3.6 Scripted-error tests, after discovery has admitted a source: an
      `Identity` error at the retained-handle recheck (`ui.rs:750-751`), run
      for each of the three kinds; a `Read` error at the bounded source read
      (`ui.rs:758`), run for Claude/Codex and for DSH (past its two header
      reads). Each asserts body-stage `unreadable` with no turns, zero
      counts, no truncation and the retained file's bytes unchanged — same
      requirement.
- [ ] 3.7 Real-change test: a `Child` entry removes the only matching Codex
      rollout, sitting directly under `sessions`, after enumeration and
      before its open (`ui.rs:521`), asserting `not-found` with null path,
      never `unsafe-path`, and no content read — same requirement.
- [ ] 3.8 Real-change tests at `ChildFileAttempt` on a Claude lookup: removing
      the session file after its directory attempt fails (`safe_fs.rs:193`)
      asserts `not-found` with null path and no content read; the placement
      witness (D6) instead replaces the file with a same-named directory,
      asserting `unsafe-path` on unix and Windows, proving the hook fires
      between the directory and file attempts rather than before either —
      same requirement.
- [ ] 3.9 Real-change tests at `BeforeRewalk`, after discovery admits a
      unique Claude source: renaming that file into a second qualifying
      project directory (`ui.rs:377` branch, `743-744`); moving the admitted
      file to a non-qualifying holding name and then renaming a new file
      onto its freed path (`743-744`), never a rename directly onto an open
      name. Both assert body-stage `unreadable`, no turns, zero counts, no
      truncation and no prose from either file on any surface — same
      requirement.
- [ ] 3.10 Boundary-in-force scenario (S9): one test whose only scripted
      entry is an `Entries` error at the occurrence falling in a separate,
      otherwise valid lookup made after the asserted reads. The same test
      first reads a symlinked project, a symlinked candidate, a FIFO
      candidate and a regular candidate, asserting each keeps its canonical
      root, handle-relative opening, no-follow, non-blocking and
      regular-file checks and its existing `unsafe-path`/readable outcome;
      then asserts the later lookup's discovery-stage `unreadable`, so the
      entry fired and the unfired-entry check passes unchanged. Write the
      entry's occurrence as the sum of each named read's enumeration count
      plus one, with a comment naming the read it points at, so a drift is
      traceable to a named read — same requirement.
- [ ] 3.11 Add the release-build proof: confirm that clippy's
      `--all-targets`, the workspace tests, both bundle compiles and the
      release build already exercise a build without the unit-test
      configuration, in which any unguarded `fault::` reference is a
      compile error. The source inspection that confirms where each
      `fault::` reference sits is task 8.5, which owns and records it —
      same requirement.

## 4. Restructures with recorded unreachability proofs, and the `claude_source` home parameter (D5)

- [ ] 4.1 Remove the post-decode emptiness check at `ui.rs:998-999`. Record
      the unreachability proof in a code comment next to the surviving
      `:990` check: a non-empty component decodes to a non-empty byte
      string, `String::from_utf8` of a non-empty vector is non-empty or
      `None`, and the pre-decode check already refused an empty component.
      Keep the `:990` check's existing test and the decode-refusal test —
      transcript-reading / Reader failure handling is proved through a
      test-only fault seam.
- [ ] 4.2 Restructure the DSH walk (`ui.rs:613`) into a step-based form
      where the base is always the last directory a step opened and nothing
      is left unset: `split_once('/')` separates the first component, which
      opens from the root; each later component opens from the previous
      step's directory; one step helper keeps today's `Dir`/`Unsafe`/other/
      `Err` arms. No fallback to the root, no `unwrap`/`expect`. Record the
      unreachability proof for the removed post-walk `else` in a code
      comment. Add ordinary fixtures reaching both `split_once` arms
      (single-component and multi-component locators) — same requirement.
- [ ] 4.3 Restructure `tui.rs::refused_lines` to take the refusal reason as
      a parameter, matching it against `read.unavailable` inside `Some` at
      its only call site (which already follows the `is_readable` guard);
      remove the reason-less arm. Update the existing `refused_lines` tests
      to pass a reason — same requirement.
- [ ] 4.4 Restructure `tui.rs::drive` around the invariant that
      `tui.reading_transcript` implies `views.transcript` is `Some` and
      readable (established by its three write sites and by
      `transcript_invalidates` closing the door before any recompose runs on
      a `None`-or-unavailable fresh read): classify the `(displayed, fresh)`
      pair once into "both absent", "continuing" (fresh is present and not
      invalidated) and "invalidated", and recompose the door only in the
      continuing arm, from that arm's fresh read. Move the existing
      `transcript_invalidates` expectations onto the new classification
      without changing any reachable outcome. Record the invariant's proof
      in a code comment — same requirement.
- [ ] 4.5 Give `claude_source` (`ui.rs:820-834`) a `home: Option<&str>`
      parameter, mirroring the existing `read_local`/`read_with_home` split;
      have its current caller(s) pass `local_projects_home()` as they read it
      today, with no behavior change on a present home. Add a unit test
      passing `None` and asserting `Discovery::Refused(Unavailable::MissingHome, None)`,
      closing `ui.rs:827-828` without mutating the process environment —
      same requirement.

## 5. The remaining ordinary-fixture branches (D5)

- [ ] 5.1 `ui.rs:261` branch: a fixture with two valid DSH session
      candidates plus a third whose opening header exceeds the 65,536-byte
      cap, asserting `ambiguous-source` and not `discovery-limit` —
      transcript-reading / Reader failure handling is proved through a
      test-only fault seam.
- [ ] 5.2 `ui.rs:443`, `445` branches: Codex ids `rollout-…` and `jsonl`
      (both in the Codex filename-token language), asserting the whole-token
      rule admits or refuses each as specified at the filename's start and
      end — same requirement.
- [ ] 5.3 `ui.rs:516` branch: a matching rollout seven directory levels
      below `sessions`, asserting `not-found` — same requirement.
- [ ] 5.4 `ui.rs:990` branch: `GET /api/presentation//<key>`, asserting 404
      `participant` — same requirement.
- [ ] 5.5 `ui.rs:1003`: `GET /api/presentation/<unknown-run>/<key>` over a
      real journal so `Store::load` returns `RunNotFound`, asserting 404
      `participant` — same requirement.
- [ ] 5.6 `brokkr-view/src/transcript.rs:1974` branch: a quiet DSH row
      without `seq`, asserting it contributes no position and the rest of
      the projection is unchanged — same requirement.

## 6. Journal inertness: text and tests agree (D9)

- [ ] 6.1 In `crates/brokkr-cli/tests/transcript_privacy.rs`, change
      `journal_and_wal` to record the database digest and the `-wal`
      digest, and stop recording the `-wal`'s length — transcript-reading /
      Transcript prose stays local and inert.
- [ ] 6.2 Take the before-state once, before the first read, in both
      `reading_leaves_the_journal_and_the_retained_file_unchanged` and
      `growth_reads_keep_the_tree_config_and_journal_inert`; assert the
      database digest equals that before-state after every read in each
      test — same requirement.
- [ ] 6.3 For each read: when it began with no `-wal`, assert the `-wal`
      afterward is absent, empty, or frame-free by `wal_has_frames`; when it
      began with an existing `-wal`, record that `-wal`'s digest immediately
      before the read and assert the same digest afterward. Apply this to
      the refusal in the first test and to every read after the first in the
      growth test — same requirement.
- [ ] 6.4 Add a frame-bearing fixture: keep the writer `Store` that appended
      the seat's transcript reference open and idle across the three
      command reads (success, refusal, growth), staying well below SQLite's
      1000-page autocheckpoint. Before the first read, assert
      `wal_has_frames`. After each read, assert the JSON resolved that
      reference (proving the frames were read), the `-wal` digest is
      unchanged, and the database digest is unchanged. Do not compare the
      `-shm`. Drop the writer only after the assertions — same requirement.
- [ ] 6.5 Add the missing-journal proofs on every surface, each asserting the
      database path, its `-wal` and its `-shm` still do not exist afterward:
      a new test in `tests/transcript_command.rs` with `--db` naming a
      nonexistent path; a new test in `ui/tests.rs` calling `ui::handle` for
      `/api/runs`, `/api/run/<run>`, `/api/view/<run>` and
      `/api/presentation/<run>/<key>` over that path (asserting 404 for
      each) and one `serve_io` poll of `/sse/<run>` with `sse_limit = Some(1)`
      (asserting head sequence zero); re-run the existing
      `tests/machine_proof.rs:3429-3440` TUI assertion on the final head —
      same requirement.

## 7. Read-only integration fixture ports (D11)

- [ ] 7.1 Compare this branch's files against each of `c2a7da4`, `fcd3b93`,
      `49ef15d`, `8d2211d`, `c7ff0fa`, `10908b6`, `82c802a`, `05ab44b` and
      `bfd677b` in `../brokkr-222-integration`, reading that worktree only.
      Check `c2a7da4`'s production change against the lossless `i128`
      identity `widen` this branch already carries — every requirement of
      this change.
- [ ] 7.2 Check whether `bfd677b`'s
      `a_working_seats_transcript_is_re_resolved_without_a_journal_move`
      inode-reuse purpose is already covered by this branch's
      stage-then-rename-over-the-unopened-target fixture
      (`src/tests.rs:2222-2229`); port the commit's test content if it is
      not — every requirement of this change.
- [ ] 7.3 Port, by content, any other test-only repair from the inspected
      set whose unrepaired form is still present in this branch's files.
      Record each commit's disposition (ported, already subsumed, or not
      applicable) in the verification record built in group 8. Do not write
      to `../brokkr-222-integration` or `main` — every requirement of this
      change.

## 8. Coverage re-trace, review-finding regression map, and every gate (D8, D11)

- [ ] 8.1 Re-measure local exact coverage and confirm the 22 reader-owned
      lines and 19 branch records from the design's table are now covered,
      and that every arm in the D8 seam-coverage table is reached by a
      named test — transcript-reading / Reader failure handling is proved
      through a test-only fault seam.
- [ ] 8.2 Re-trace S11 against the newest host measurement whose source is
      byte-identical for each non-reader missed region, using the same two
      conditions the proposal states (host coverage of the region, and
      every test whose executed regions differ returns early under
      `HANDS_BOX_ENV` or a failed namespace probe). Repair, like a reader
      miss, any non-reader miss this re-trace does not establish a box
      cause for, or that the final-head host run leaves uncovered — every
      requirement of this change.
- [ ] 8.3 For each of chief review 7505's M1-M11 and L1-L11, name or add the
      tracked regression test proving its repair, and record its result on
      the final head in the verification record; where a repair has no
      tracked test, add one rather than recording an absence — every
      requirement of this change.
- [ ] 8.4 Carry `has_security_residual` (review 7505, medium) in every
      result until an independent review clears it on evidence — every
      requirement of this change.
- [ ] 8.5 Run D7's four-clause seam boundary inspection on the final head
      and write it into the verification record, quoting each command and
      the output it produced, not the conclusion drawn from it: (1) every
      `fault::` reference under `crates/brokkr-cli/src` sits in the `fault`
      module, under a `#[cfg(test)]` statement, or in `ui/tests.rs`; (2) the
      `fault` module reads no environment variable, argument, configuration
      key, file or journal value; (3) no `Guard` value in `brokkr-cli` is
      passed to `mem::forget`, wrapped in `ManuallyDrop`, leaked, or handed
      to a helper that takes it by value and does not drop it, and every
      `install` site binds its guard as a plain local or as a field that
      drops in the ordinary path; (4) `#![deny(clippy::mem_forget)]` is
      present in `crates/brokkr-cli/src/lib.rs`, the pinned clippy carries
      that lint, and the clippy gate ran clean with it. Record clause 3 as
      review and clauses 1, 2 and 4 as mechanical, as D7 and D4.5 state —
      every requirement of this change.
- [ ] 8.6 Run and leave green: `cargo fmt --all -- --check`; `cargo clippy
      --workspace --all-targets --all-features --locked -- -D warnings`;
      `cargo test --workspace --all-features --locked`; `cargo run -p
      brokkr-cli -- compile --bundle bundles/self`; `cargo run -p
      brokkr-cli -- compile --bundle bundles/verify`; `cargo build --release
      --locked -p brokkr-cli`; `openspec validate --all --strict`; a frozen-
      byte comparison against `5bc8cf3` for `contracts/`,
      `policy/phase-machine.json`, `policy/schemas/`, `reference/` and
      `fixtures/`; the Rust 1.88 `--all-targets --all-features` check
      (recorded as pending if the toolchain is absent); the unchanged exact
      coverage gate under `CARGO_BUILD_JOBS=2` and `RUST_TEST_THREADS=2` —
      every requirement of this change.

## 9. Contributor note, archive fold, commit (D10, D11)

- [ ] 9.1 Add the short coverage note to
      `docs/guides/contributing-by-hand.md`, as one paragraph in the
      existing "The four refusal shapes" section (`:407`) after that
      section's first shape, which already says an unreached arm is
      "usually an error arm" whose fix "is the test case that takes the arm
      — not deleting the arm". The note says how to reach a reader error
      arm, in order: prefer an ordinary input, then a real change timed by
      `safe_fs::fault`, then a scripted error; entries must fire; the seam
      exists only in the `brokkr-cli` unit-test build; and the `fault`
      module is not a test module but counted code in a production file, so
      that section's test-module placement rule (`:432-438`) neither applies
      to it nor exempts it from the counter. Leave `CONTRIBUTING.md`'s
      pointer line (`:107`) as it is and do not duplicate the note there.
      Check that this task, design D10 and the proposal's Impact line name
      the same file and section in the same words — every requirement of
      this change.
- [ ] 9.2 Fold the change into `openspec/specs/transcript-reading/spec.md`
      with one normal archive: replace the modified "Transcript prose stays
      local and inert" requirement and append the new "Reader failure
      handling is proved through a test-only fault seam" requirement,
      keeping existing provenance lines append-only (decision 0042). Move
      this change's directory under `openspec/changes/archive/`. Then re-run
      `openspec validate --all --strict` on the folded tree and confirm both
      the living `transcript-reading` specification and the archived change
      pass: 8.6's run validated the pre-fold tree and does not speak for
      this one — every requirement of this change.
- [ ] 9.3 Commit the completed, ticked task list and its code together,
      tagged `(#222)`, unsigned, and never pushed — every requirement of
      this change.

The fresh, independent final review is deliberately outside the tracked
checkboxes above. After 9.3 fixes the delivered head, that review is taken
of the unchanged commit 9.3 produced, and it causes no further tracked edit.
Its run-integrity observations are recorded as observations, never as
directions to override a gate. If it finds a defect, the finding is repaired
in its owning artifact through the run's ordinary drift machinery, the
repair is a new commit, and that new head gets its own clean review — the
reviewed head is always the delivered head. Host exact coverage with
`TMPDIR=/var/tmp` and `BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1`, remote CI,
integration and publication stay controller handoff and are recorded as
pending until their results exist.
