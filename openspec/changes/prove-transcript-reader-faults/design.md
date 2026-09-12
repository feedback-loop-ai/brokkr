## Context

This design adopts `prove-transcript-reader-faults` at `b0ea627` on
`fire/222-transcript-readers`. That means proposal decisions S1–S14 and the
`transcript-reading` delta, as answered by three clarify passes (`aa864a6`,
`4d131f0`, `b0ea627`). Clarification is `clear`. See
[proposal.md](proposal.md) (Why, What Changes) for motivation and the spec
delta for the requirements. The last code commit is `5738889`, and
`git diff 5738889 b0ea627 -- crates` is empty, so every line reference below
holds on both commits.

The settled change `2026-09-10-read-every-transcript-kind`, its three living
specifications and proposed decision 0055 stand. Chief review 7505 ruled
`spec_defect=false`, and its 22 findings have repairs at `5738889`. This
design reopens none of them. It numbers its own decisions D1–D11. A
reference to the settled change's design says "archived D*n*".

### The reader boundary today

- `crates/brokkr-cli/src/ui/safe_fs.rs` is the handle-based helper (0055
  ruling 2, archived D3). Platform-independent wrappers `Dir::entries_bounded`
  (`:70`), `Dir::child` (`:75`), `OpenedFile::identity` (`:91`) and
  `OpenedFile::read_bounded` (`:103`) delegate to one `imp` module per
  target: `#[cfg(unix)]` over rustix `openat`/`fstat` (`:109`), and
  `#[cfg(windows)]` over `NtCreateFile` and `GetFileInformationByHandle[Ex]`
  (`:231`). There is no pathname fallback implementation. A target that is
  neither does not compile the reader.
- Each `imp::Dir::child` makes a directory attempt. On an absent answer it
  returns `Absent`. On any other failure it falls through to a file attempt,
  which classifies `File`, `Unsafe` or `Absent` (unix `:175-199`, Windows
  `:452-489`). Windows caches a file's identity at open (`:491`), and it opens
  every handle with `FILE_SHARE_DELETE` (`:364`, `:394`).
- `crates/brokkr-cli/src/ui.rs` owns discovery (`discover` `:339`, the three
  kind walks `:385-680`, `Lookup::resolve` `:260`) and the read boundary
  (`read_with_home` `:694`): admit, re-walk (`acquisition_is_current` `:373`),
  recheck the retained handle's identity, make one bounded read, then run the
  pure projector.
- `tui.rs` and the browser consume the shared `TranscriptRead`. `tui.rs` has
  no filesystem boundary.

### Re-measured misses

`target/coverage/local-lcov.info` is the box measurement at `5738889`. Its
reader-owned uncovered records are:

| File | Lines | Branch lines |
|---|---|---|
| `brokkr-cli/src/ui.rs` | 301, 302, 393, 420, 489, 511, 521, 552, 615, 637, 668, 744, 751, 758, 828, 999, 1003 | 261, 377, 392, 443, 445, 488, 516, 613, 614, 636, 743, 750, 827, 990, 998 (×2) |
| `brokkr-cli/src/tui.rs` | 3330–3334 | 3156 |
| `brokkr-cli/src/ui/safe_fs.rs` | — | 193 |
| `brokkr-view/src/transcript.rs` | — | 1974 |

That is 22 lines and 19 branch records, matching controller evidence 1305.
The `tui.rs` misses the framing asked to itemize are the `None` arm of the
open-door recompose in `drive` (`:3330-3334`) and the `None` side of
`if let Some(reason) = read.unavailable` in `refused_lines` (`:3156`). The
non-reader misses and their box causes are the proposal's S11 table. This
design does not repeat them.

### Constraints that shape the approach

- The seam sits behind the boundary guarantees and never around them. Those
  guarantees are the canonical root, handle-relative opening, no-follow,
  non-blocking and regular-file checks, the retained-handle recheck and the
  fail-closed re-walk.
- Tests run with `RUST_TEST_THREADS=2`. libtest runs each test on its own
  spawned thread, and runs it on the calling thread only when a spawn would
  block. `HOME` is process-global. The unit tests that mutate it take
  `crate::tests::HOME` (`src/tests.rs:14`).
- `scripts/coverage-exact.sh` stays byte-identical. It leaves out only
  `tests.rs`, `*_tests.rs` and `tests/`, refuses `coverage(off)`, and requires
  literal 100% of lines, branches and functions. An inline
  `#[cfg(test)] mod tests` inside a production file is counted.
- The minimum supported Rust version is 1.88. Clippy runs with `-D warnings`
  over `--all-targets --all-features`.
- There is no new dependency, no feature and no `Cargo.lock` change. The
  frozen surfaces stay byte-identical.

## Goals / Non-Goals

**Goals:**

- Give the spec's six seam targets one concrete shape: a lifecycle that
  cannot leak, cannot be disarmed and cannot abort the test binary, and that
  compiles only into the `brokkr-cli` unit-test build.
- Route every re-measured miss to exactly one of four things: an ordinary
  fixture, a real change timed by the seam, a scripted error, or a
  restructure whose unreachability proof is recorded. Each route asserts the
  specified outcome.
- Make the privacy tests assert S12–S14 exactly, and prove the
  missing-journal rule on the command, the browser and the TUI.
- Say how the rest of #222 closes on this branch: the repair verification
  record, the security residual, the integration fixture ports, every gate,
  the final review and the archive fold.

**Non-Goals:**

- Any new transcript behavior, vocabulary, reason token, route or output
  field. Every outcome asserted here is already specified.
- A seam in `brokkr-view`, `tui.rs`, `lib.rs` or any non-reader file.
- Faking a namespace, `bwrap` or `HANDS_BOX_ENV` to close the non-reader
  box misses. Those stay pending host proof under S11.
- A no-sidecar SQLite open, or any change to `Store::open_read_only`.
- Repairing the existing `HOME` save/restore pattern in `ui/tests.rs`. D1
  records it as a residual.
- Controller work: host exact coverage, remote CI, integration, publication
  and closing the issue.

## Decisions

### D1 — The council is reconciled claim by claim

Two positions were written for the first sitting: simplicity and robustness.
Both pass the settled contract. Where they differ, each claim is settled on
evidence, not by averaging. A second sitting, on the return from analyze,
is reconciled after this table.

| Claim | Seat | Ruling | Evidence and result |
|---|---|---|---|
| One `#[cfg(test)]` module inside `safe_fs.rs`, one thread-local plan, six closed targets, two constructors over disjoint target types | simplicity | **Adopted** | This is S3, S4 and S8 exactly. D2 fixes the shape. |
| "Five hook sites" | simplicity | **Corrected** | The seat's own list names six targets. The spec requires the file-attempt point in both platform modules, so there are seven statement sites for six targets (D3). |
| "Keep the guard to a single `assert!` so it adds no branch" | simplicity | **Rejected as stated** | `assert!` is itself a branch pair, and D4 adds the unwinding check. Both are covered by the seam's own tests (D8), which is the pressure S7 intends. |
| Reach `tui.rs:3330-3334` through `drive`'s `source` parameter | simplicity | **Rejected** | The arm is reachable only from a `Tui` that a test builds with `reading_transcript` set and no readable view. Production never computes that state (D5 proves it). Reaching it would prove how a forged state is handled, which S2 rejects for the filesystem and this design rejects for the state machine. The arm is removed by restructure. |
| Restructure `ui.rs:613` to remove the post-walk `else` | simplicity | **Adopted, with a form** | The restructure must not fall back to the root. D5 fixes a form that has no `Option` left unset. |
| Only text and tests for the WAL, no no-sidecar open | simplicity | **Adopted** | S5 and S14. D9. |
| No negative-compile test; the non-test builds are the absence proof | simplicity | **Adopted** | A `trybuild` test needs a new dev-dependency, and doctests never see `pub(crate)` `cfg(test)` items. D7. |
| The Windows hook is native CI proof, not a Linux coverage number | simplicity | **Adopted and extended** | The whole Windows `imp` module is already outside the Linux report. The hook adds nothing to that class. D6 adds a placement witness that runs on all three OSes. |
| RAII drop guard | robustness | **Adopted** | An explicit "assert all fired" call stops checking at the first early return. D4. |
| Skip the unfired check while unwinding | robustness | **Adopted** | A panic inside `Drop` during unwinding aborts the whole test binary. D4 clears the plan and returns when `std::thread::panicking()` is true. |
| Refuse to install over a live plan, because worker threads are reused | robustness | **Adopted; premise corrected** | libtest spawns a thread per test, so reuse across tests is not the normal case. Two real hazards remain: a nested install within one test or helper, and libtest's run-on-caller fallback. The refusal covers both. D4 also makes the guard `!Send`, so it always drops on the installing thread. |
| Race checkpoints are permanent call-outs through one platform-independent helper | robustness | **Adopted** | Both `imp::Dir::child` bodies call the one `fault::point`, and neither inlines seam logic. D3. |
| A grep-based structural check that ties each checkpoint to its scenario | robustness | **Rejected, replaced** | A grep proves the text is present, not where it is. Removing a point leaves its entry unfired on every OS (D4). A displaced file-attempt point is caught by the placement witness on every OS (D6) and by the exact-coverage branch at `safe_fs.rs:193` on Linux. |
| State a filesystem assumption or a flake policy for the real changes | robustness | **Adopted, narrowed** | The changes are scripted interleavings on the reader's own thread, not timing races. D6 states the one filesystem assumption and adopts no retry. |
| Write the triage aid for the boundary-in-force test into the design | robustness | **Adopted** | D5 requires a message on each asserted read, and the entry's occurrence written as a sum of named per-read counts. |
| An append-only builder and an opaque guard, so "disarm" cannot be written | robustness | **Adopted** | D2. |
| Name the existing `HOME` pattern as a residual | robustness | **Adopted; count corrected** | Eight `ui/tests.rs` and `tui/tests.rs` sites take `crate::tests::HOME`, so mutation among lock holders is serialized. The residual is that restoring `HOME` is not unwind-safe, and readers that do not take the lock can see the mutation. It is out of scope and not copied: no test in this change mutates any environment variable. |

#### Second sitting: the return from analyze (F1-F4)

Analyze returned `drift` with four low findings, three owned by this design
and one by the tasks. Two positions were written again. They agree on the
direction of all four and differ on one thing: F1's enforcement. Simplicity
holds that a named review rule in D7 is the whole answer and refuses any
mechanism; robustness holds that a promise this design states as
unconditional cannot rest on review alone when a mechanical check is
available for nothing.

| Claim | Seat | Ruling | Evidence and result |
|---|---|---|---|
| F1: close the `mem::forget` hole with a review rule in D7 and nothing else; a drop-watcher, a global live-plan registry, a thread-exit hook or a `trybuild` negative test are all larger than the seam they protect | simplicity | **Adopted in part** | The refusals are adopted whole: each of those mechanisms needs its own code, its own tests and its own shared state, and none can see a guard already moved into `forget`. The "and nothing else" is rejected — see the next row. |
| F1: add `clippy::mem_forget` as a mechanical backstop, and extend D7's inspection to cover `Guard` leak patterns | robustness | **Adopted** | The seat's own workspace grep (`mem::forget`, `ManuallyDrop` over `crates/`) returns nothing, re-run here with `Box::leak` added and still empty. So the lint denies a pattern no code uses: no noise, no suppression, no dependency, no lockfile change, one attribute line. It is the same shape of guarantee D7 already gives the seam's absence from release builds, and unlike review it holds for tests nobody has written yet. D4.5 adopts it. |
| F1: the lint's scope, left open between "the `ui` module tree" and "crate-wide in `brokkr-cli`" | robustness | **Fixed** | An open scope is the drift F3 was raised about. The scope is one crate-root attribute, `#![deny(clippy::mem_forget)]` in `crates/brokkr-cli/src/lib.rs`: it covers `ui/safe_fs.rs` and `ui/tests.rs`, the only places a `Guard` can exist, and needs no manifest edit (each crate's `[lints]` already inherits the workspace table). |
| F1: the lint alone is not the whole closure | robustness | **Adopted, and stated** | `clippy::mem_forget` fires on `mem::forget` of a `Drop` type. It does not see `ManuallyDrop`, `Box::leak`, or a helper that takes a `Guard` by value and drops nothing. D4.5 names what the lint closes and what only review closes, rather than implying one mechanism covers both. |
| F1: the lint name must be checked against the pinned toolchain, not assumed | robustness | **Adopted** | This box has no `cargo` (`which cargo` fails), and the pinned toolchain is `nightly-2026-09-05`, so the check cannot run here. D7 clause 4 makes it an implementation check with a recorded result, and a missing lint is a finding, not a silent gap. |
| F4: the inspection needs an owning group-8 task whose record quotes the grep output, not a checkbox asserting a conclusion | robustness | **Adopted** | An unfalsifiable checkbox is the same defect analyze found in the dangling cross-reference. D7 now requires the command and its output; task 8.5 owns it and 3.11 cites it by number. |
| F4: no new tool, script or task group for the inspection | simplicity | **Adopted** | It is four text clauses over `grep` and the clippy result. It needed an owner, not machinery. |
| F2: date the addendum on filing | both | **Adopted** | Matches addenda 0035 and 0042. Robustness's condition is adopted: the date is fixed at the commit that lands it, and is not re-derived if a later commit moves. |
| F3: land the note in `docs/guides/contributing-by-hand.md`, not `CONTRIBUTING.md`, and do not duplicate it | both | **Adopted** | `CONTRIBUTING.md` has no coverage guidance; `:107` is a pointer. |
| F3: the section is "Exact coverage" (`:190`) | both | **Corrected** | `:190` is the section that gives the command. The note is guidance for a contributor holding an uncovered error arm, and that guidance already lives in "The four refusal shapes" (`:407`), whose first shape reads "usually an error arm. The fix is the test case that takes the arm — not deleting the arm." The note extends that sentence, so it lands there. Both seats named the weaker anchor; the evidence overrides both. |
| F3: proposal Impact, D10 and task 9.1 must name the same place in the same words | robustness | **Adopted** | All three are repaired together in this visit. |
| The two `tui.rs` restructures are the largest code motion; prefer the smallest form that removes the arm | simplicity | **Adopted as a caution** | D5 already fixes the form and requires the existing `transcript_invalidates` expectations to move unchanged. No reopening of the settled reach-or-remove ruling. |
| Neither seat reopens D2-D9's seam shape, the S1-S14 clarify record, or the six targets | both | **Held** | Nothing in F1-F4 bears on them, and no new evidence was offered. |

### D2 — One test-only module, a builder plan and an opaque guard

`crates/brokkr-cli/src/ui/safe_fs.rs` gains
`#[cfg(test)] pub(crate) mod fault`. It is the seam's only definition. It
has no `cfg(not(test))` twin, no feature, no environment read, no file read
and no `pub` item.

- **Targets.** There are two disjoint enums.
  - `FailAt` is `Entries`, `Identity` or `Read`.
  - `ChangeAt` is `Child`, `ChildFileAttempt` or `BeforeRewalk`.

  No value is in both, so S8's pairing is a property of the types.
- **Plan.** `Plan` has private fields and three consuming builder methods:
  - `Plan::new()`;
  - `fail(self, FailAt, occurrence: usize) -> Plan`;
  - `change(self, ChangeAt, occurrence: usize, impl FnOnce() + 'static) -> Plan`.

  A plan can be added to, never trimmed. There is no `clear`, `skip`,
  `disarm`, `optional` or `allow_unfired`.
- **Install.** `Plan::install(self) -> Guard`. It stores the plan in a
  `thread_local!` `RefCell<Option<…>>`, with one occurrence counter per
  target starting at zero. If a plan is already stored on that thread, it
  panics.
- **Guard.** `Guard` has no methods. It carries a
  `PhantomData<*const ()>`, which makes it `!Send`, and it implements
  `Drop` (D4).
- **Hooks.** There are two hooks.
  - `fault::fail(FailAt) -> Option<io::Error>` returns
    `Some(io::Error::other("scripted reader fault"))` exactly when the plan
    holds an unfired entry for that target at the incremented occurrence.
    The error's kind is `ErrorKind::Other`, and `io::Error::other` keeps
    clippy's `io_other_error` lint quiet.
  - `fault::point(ChangeAt)` takes the matching entry's closure out of the
    `RefCell`, releases the borrow, and then runs the closure.

  With no plan installed, either hook is a counter-free no-op.
- **Invalid entries.** An entry at occurrence zero, or a second entry for a
  `(target, occurrence)` pair that is already scripted, can never fire. The
  unfired check fails it, so neither needs its own refusal branch.

The seam never builds a `Child`, `Identity`, handle, path or name. A change
closure is test code in `ui/tests.rs`, and it touches only the test's
tempdir. That is a review rule for test code, because the seam cannot see
what a closure does.

**Alternatives rejected.** These are settled in S3 and not re-argued:
- an environment variable, configuration key or runtime switch;
- a Cargo feature;
- a generic filesystem trait;
- a `cfg(not(test))` twin;
- a failpoint crate.

Two more are rejected here. A process-global plan would leak into the
second test thread's reads. A plan keyed by file name would give the seam a
name to match, and S4 rules that out.

### D3 — Seven statement sites carry six targets

Each site is one statement under `#[cfg(test)]`, and each runs before the
real operation it precedes.

| Target | Site | Form |
|---|---|---|
| `Entries` | `Dir::entries_bounded` wrapper (`safe_fs.rs:70`) | on `Some(error)` from `fault::fail(FailAt::Entries)`, return `Err(error)` |
| `Identity` | `OpenedFile::identity` wrapper (`:91`) | the same with `FailAt::Identity` |
| `Read` | `OpenedFile::read_bounded` wrapper (`:103`) | the same with `FailAt::Read` |
| `Child` | `Dir::child` wrapper (`:75`), before `self.inner.child` | `fault::point(ChangeAt::Child)` |
| `ChildFileAttempt` | unix `imp::Dir::child`, after the directory attempt's non-absent failure and before the file `openat` (`:184`) | `super::fault::point(ChangeAt::ChildFileAttempt)` |
| `ChildFileAttempt` | Windows `imp::Dir::child`, after the directory attempt's non-absent failure and before the file `nt_open` (`:469`) | the same call |
| `BeforeRewalk` | first statement of `ui.rs::acquisition_is_current` (`:373`) | `safe_fs::fault::point(ChangeAt::BeforeRewalk)` |

The three `fail` targets sit in the platform-independent wrappers. So an
identity error can be scripted on Windows too, even though the Windows
identity is cached. `Dir::open_root`, `OpenedFile::len` and
`std::fs::canonicalize` are not targets. No listed miss needs them, and
their error arms are already reached by real conditions.

### D4 — The lifecycle: install, count, fire, tear down

1. `install` refuses a second live plan on the thread (D2).
2. Each hook call on the installing thread increments that target's
   counter, then looks for the first unfired entry whose
   `(target, occurrence)` equals `(target, counter)`. A match is marked as
   fired before its action runs. Hook calls on any other thread see no plan.
3. `Guard::drop` always takes the plan out of the thread-local, so no plan
   outlives its guard. If `std::thread::panicking()` is true, drop returns:
   the test is already failing, and a second panic would abort the binary.
   Otherwise its last statement asserts that no entry is unfired, and the
   message names each unfired target and occurrence.
4. A guard discarded with `let _ =` drops at once. Its entries are then
   unfired and the test fails. That failure is loud, not silent.
5. A guard that is never dropped leaves its plan stored with entries
   unfired, and nothing fails. No `install`-time refusal saves it: D1's
   own premise is that libtest spawns a thread per test, so in the normal
   case there is no next `install` on that thread, and the thread-local
   dies with the thread. This is the one route by which a test could
   exempt itself from the unfired check that `spec.md:51-53` forbids
   exempting any test from, so the design closes it outside the guard,
   and says which part is mechanical and which is review.
   - `crates/brokkr-cli/src/lib.rs` carries
     `#![deny(clippy::mem_forget)]`. `mem::forget` of a value that
     implements `Drop` — a `Guard` — then fails the clippy gate anywhere
     in the `brokkr_cli` library crate, `ui/safe_fs.rs` and `ui/tests.rs`
     included, for every test written against the seam after this change
     lands. That crate root is the whole scope that needs the lint: the
     bin target is a separate crate root, and the seam is `cfg(test)` in
     the library, so no `Guard` value can exist outside it. A
     workspace-wide grep over `crates/` for `mem::forget`, `ManuallyDrop`
     and `Box::leak` returns nothing at `5738889`, so the lint denies a
     pattern no code uses: no noise, no suppression, no lockfile or
     dependency change. Implementation confirms the lint's name against
     the pinned toolchain before relying on it (D7).
   - The lint does not see `ManuallyDrop`, `Box::leak`, or a helper that
     takes a `Guard` by value and never drops it. Those are closed by the
     no-leak clause of the D7 source inspection, which is review. The
     design records that plainly instead of claiming a mechanism it does
     not have.
   The seam still offers no way to clear a plan other than dropping its
   guard.

The seam's own tests live in `ui/tests.rs`, whose `#[should_panic]` tails
are not counted:
- a plan whose entry never fires fails with the unfired message;
- a nested `install` fails with the refusal. Its unwinding drops the outer
  guard, which holds an unfired entry, so the same test proves that the
  unwinding arm clears the plan without aborting;
- a test that spawns a second thread proves thread scoping. Its reader call
  at the scripted occurrence sees only real results. After the guard drops,
  the same occurrence on the installing thread is unaffected too.

### D5 — Every miss has one route and one asserted outcome

A test drives the reader on the installing thread through `read_with_home`,
`discover`, `claude_source` or `ui::handle`. It never mutates `HOME`. Each
fixture keeps one entry in each directory whose enumeration a selected
occurrence depends on (S4).

**Scripted errors** (S2: no portable real fault exists):

| Site | Entry | Asserted outcome |
|---|---|---|
| `ui.rs:392-393`, `301-302` | `Entries` 1 on a Claude lookup | discovery-stage `unreadable`: null path, no turns, zero counts, no truncation, no notices, and the kind's unresolved hint `full_session(&valid, None)`; browser presentation for the same reference reports `admitted: false` with reason `unreadable` |
| `ui.rs:488-489` | `Entries` 1 on a Codex lookup (the `sessions` walk) | the same |
| `ui.rs:614-615` | `Entries` 1 on a DSH lookup (the seat directory) | the same |
| `ui.rs:636-637` | `Entries` 2 on a DSH lookup (its one project directory) | the same |
| `ui.rs:420`, `511`, `668` | `Identity` 1 on each kind's sole matching candidate, and also with a second, qualifying sibling | discovery-stage `unreadable`, no candidate admitted, including with the survivor present |
| `ui.rs:552` | `Read` 1 on the only DSH candidate | discovery-stage `unreadable`, null path, null DSH hint, no turns, zero counts |
| `ui.rs:750-751` | `Identity` at the retained-handle recheck: 3 for every kind (discovery, re-walk, recheck) | body-stage `unreadable`, no turns, zero counts, no truncation, retained bytes unchanged |
| `ui.rs:758` | `Read` at the source read: 1 for Claude or Codex, 3 for DSH (two header reads first) | the same |

**Real changes timed by the seam** (D6 fixes the staging):

| Site | Entry | Asserted outcome |
|---|---|---|
| `ui.rs:521` | `Child` 2 on a Codex lookup whose only rollout sits directly under `sessions` (1 is `sessions`): remove that rollout | `not-found`, null path, not `unsafe-path`, no content read |
| `safe_fs.rs:193` | `ChildFileAttempt` 1 on a Claude lookup: remove the session file after its directory attempt failed | `not-found`, null path, no content read |
| `ui.rs:377` branch, `743-744` | `BeforeRewalk` 1: rename the admitted Claude file into a second project directory | body-stage `unreadable`, no turns, zero counts, no truncation, no prose from either file |
| `ui.rs:743-744` | `BeforeRewalk` 1: move the admitted file to a non-qualifying holding name, then rename a new file onto its path | the same |

**Ordinary fixtures:**

| Site | Input | Asserted outcome |
|---|---|---|
| `ui.rs:261` branch | two valid DSH sessions plus a third whose header exceeds the cap | `ambiguous-source`, not `discovery-limit` |
| `ui.rs:443`, `445` branches | Codex ids `rollout-…` and `jsonl`, which lie in the Codex language, so the id meets the filename's start or end | the whole-token rule admits or refuses as specified |
| `ui.rs:516` branch | a matching rollout seven directory levels below `sessions` | `not-found` |
| `ui.rs:990` branch | `GET /api/presentation//<key>` | 404 `participant` |
| `ui.rs:1003` | `GET /api/presentation/<unknown-run>/<key>` over a real journal (`Store::load` → `RunNotFound`) | 404 `participant` |
| `transcript.rs:1974` branch | a quiet DSH row without `seq` | the row contributes no position, and the projection is otherwise unchanged |

**Parameter** (S6): `claude_source` takes the projects home as a parameter,
mirroring `read_local`/`read_with_home`. The SSE route passes
`local_projects_home()` on each poll, as it reads it today. A unit test
passes `None` and asserts `Refused(MissingHome)`, which covers
`ui.rs:827-828` without touching the environment.

**Restructures, each with its proof recorded in a code comment and in the
verification record:**

- **`ui.rs:998-999`.** This is the post-decode emptiness check. Proof:
  `decode_component` appends exactly one byte per input byte or `%XX`
  triple, so a non-empty component decodes to a non-empty byte string.
  `String::from_utf8` of a non-empty vector is non-empty or `None`, and the
  pre-decode check at `:990` already refused an empty component. The check
  is removed. The `:990` check and the decode refusal keep their tests.
- **`ui.rs:613`.** This is the DSH walk ending with no directory. Proof:
  `str::split` yields at least one component, and each loop pass either
  returns or stores a directory. The walk is restructured so the base is
  always the last directory a step opened, and nothing is left unset:
  - `split_once('/')` separates the first component, and that component
    opens from the root;
  - each later component opens from the previous step's directory;
  - one step helper keeps today's `Dir`, `Unsafe`, other and `Err` arms.

  Both `split_once` arms are reached by ordinary one-component and
  multi-component locators, and `valid_dsh_locator` admits both. The form
  must not fall back to the root, and must not use `expect` or `unwrap`.
- **`tui.rs:3156`.** `refused_lines` receives the reason as a parameter. The
  transcript pane matches `read.unavailable` inside `Some(read)`: `None`
  renders the turns and `Some(reason)` renders the refusal. The
  `is_readable` guard and the reason-less arm both disappear, and the
  existing `refused_lines` tests pass a reason.
- **`tui.rs:3330-3334`.** Invariant: in `drive`, `tui.reading_transcript`
  implies that `views.transcript` is `Some` and readable.
  - It starts false (`:295`).
  - It is set true only at `:977` and `:982`, inside the `is_readable`
    filter on `views.transcript`.
  - Every other write sets it false.
  - `views` changes only at `views = fresh`, and the recompose keeps the
    door open only over a readable fresh read.
  - Key handling never changes `views`.

  Given the invariant, when the fresh read is `None` or unavailable,
  `transcript_invalidates` returns true (`:500`, `:502`) and closes the
  door before the recompose runs, so the `None` arm cannot run. The
  restructure classifies the `(displayed, fresh)` pair once into three
  outcomes:
  - both absent: unchanged;
  - continuing: the fresh read, which is both present and not invalidated,
    so it is readable;
  - invalidated.

  The door recomposes only in the continuing arm, from the read that arm
  carries. The existing `transcript_invalidates` expectations move to the
  classification unchanged. Every reachable state keeps today's outcome.

The **boundary-in-force** scenario (S9) runs on `cfg(unix)`, like the
existing FIFO and symlink fixtures. It reads a symlinked project, a
symlinked candidate, a FIFO candidate and a regular candidate, and then
makes one more valid lookup. Each assertion names its fixture. The entry's
occurrence is written as the sum of each named read's enumeration count
plus one, so a drift points at a named read.

The **release-build** and **unreachable-arm** scenarios are proved by D7 and
by the proofs above.

### D6 — Real changes are staged portably and fail loudly

A change runs synchronously on the reader's own thread at a fixed point,
inside the test's tempdir. Nothing runs concurrently, so the only
nondeterminism is whether the filesystem accepts the operation.

- **Assumption.** The tempdir is on a local filesystem that allows removing
  and renaming a name while another handle holds its file open. On
  Windows, the reader's handles allow this because they are opened with
  `FILE_SHARE_DELETE` (`safe_fs.rs:364`, `:394`). No change in this design
  has run on native macOS or Windows yet. Those results are pending
  controller CI and are not claimed here.
- **No rename onto an open name.** Whether a rename can replace a name
  whose file another handle holds open depends on the Windows rename
  semantics. The existing `5738889` M7 fixture does exactly that, but it is
  `cfg(unix)` (`ui/tests.rs:1889`, rename at `:1921`). The new replacement fixture runs on
  all three OSes. So it first moves the original to a non-qualifying
  holding name, which `FILE_SHARE_DELETE` permits, and then creates the
  replacement at the freed path. That is the move-away shape integration
  commit `bfd677b` gave its TUI fixture.
- **No retry and no quarantine.** Each closure unwraps its own filesystem
  result. A refused change fails the test with its OS error, to be read.
- **Placement witness.** This test runs on all three OSes. At
  `ChildFileAttempt` 1 on a Claude lookup, the change replaces the session
  file with a directory of the same name. With the point placed correctly,
  the directory attempt has already failed on the file, and the file
  attempt then meets a directory: on unix `fstat` reports a non-regular
  file, and on Windows the attempt returns `STATUS_FILE_IS_A_DIRECTORY`.
  Either way the result is `unsafe-path`. A point displaced before the
  directory attempt would open the directory, giving `not-found`. So the
  outcome proves where the point is on Windows, where the Linux coverage
  report cannot see it.

### D7 — The seam is absent from every non-test build, by construction

The module declaration carries `#[cfg(test)]`, the module is `pub(crate)`,
and every call site carries `#[cfg(test)]`. So:
- the release binary, packages, the non-test library that integration
  tests link, and the `--all-targets` lib and bin targets compile none of
  it;
- any unguarded reference is a compile error in those builds.

The gates already build all of them: clippy `--all-targets`, the workspace
tests, both bundle compiles, the release build and the Rust 1.88
all-targets check.

Verification adds one source inspection, owned by task 8.5, with four
clauses. The verification record quotes the command it ran and the output
it read, not the conclusion it drew, so a later reader can re-run it.

1. Every `fault::` reference under `crates/brokkr-cli/src` sits in the
   `fault` module, under a `#[cfg(test)]` statement, or in `ui/tests.rs`.
2. The `fault` module reads no environment variable, argument,
   configuration key, file or journal value.
3. No `Guard` value anywhere in `crates/brokkr-cli` is passed to
   `mem::forget`, wrapped in `ManuallyDrop`, leaked, or handed to a helper
   that takes it by value and does not drop it. Every `install` site binds
   its guard as a plain local, or as a field of a value that drops in the
   ordinary path (D4.5).
4. `#![deny(clippy::mem_forget)]` is present in
   `crates/brokkr-cli/src/lib.rs`, the pinned toolchain's clippy carries
   that lint under that name, and the clippy gate ran clean with it. If the
   pinned clippy does not carry it, the record names the lint it does
   carry for this pattern, or records that clause 3 stands alone as review
   — as a finding to repair, never as a silent gap.

Clause 3 is review, and the design says so (D4.5). Clauses 1, 2 and 4 are
mechanical facts the inspection reads off the source and the gate.

**Alternatives rejected:**
- a `trybuild` compile-fail test, which needs a new dev-dependency;
- a doctest, which never sees `cfg(test)` or `pub(crate)` items;
- a runtime assertion, which would itself be a switch.

### D8 — The unchanged gate counts the seam, and decision 0050's principle is met

The seam is in `safe_fs.rs`, a production file, so the gate counts every
line and branch of the `fault` module and of the seven hook statements.
Their arms are reached as follows.

| Arm | Reached by |
|---|---|
| no plan installed | every other reader test |
| plan installed, no match at this count | an entry at a later occurrence, such as `Child` 2 |
| `fail` match | the scripted-error tests |
| `point` match | the real-change tests |
| guard, all fired | every seam route test |
| guard, unfired | D4's unfired test |
| guard, unwinding | D4's nested-install test |
| `install` refusal | D4's nested-install test |

The seam's own tests live in `ui/tests.rs`. That is the established test
file, not a harness-shaped name for production code.

The Windows hook statement sits inside `#[cfg(windows)] mod imp`, which the
Linux report has never contained. It is proved by the Windows CI run of the
placement witness and the file-attempt removal test, and it is recorded
with the other native platform evidence. That is native proof, not a Linux
coverage number, and not an exclusion.

Proposed decision 0050 is cited for its principle only, that the machine
proves what it promises (S7). The reader promises to fail closed on I/O
errors and races, and after this change each promise has a test that runs
the production handling.

### D9 — Journal inertness: text and tests agree (S5, S12–S14)

The spec delta already restates the requirement. This design supersedes the
archived D7 sentence "a read must not create WAL sidecars"
(`archive/2026-09-10-read-every-transcript-kind/design.md:699-700`), and
leaves that file's bytes unchanged (S1). The tests change in
`crates/brokkr-cli/tests/transcript_privacy.rs`.

- **State helper.** `journal_and_wal` records the database digest and the
  `-wal` digest. It no longer records the `-wal` length.
- **Before-state, taken once.** The before-state is the database digest
  plus the `-wal` state, and it is taken before the first read.
- **What every read owes.** After every read, the database digest equals
  the before-state.
- **The `-wal` check, by read.**
  - When a read began with no `-wal`, the `-wal` after it is absent, or
    empty, or frame-free by `wal_has_frames`.
  - When a read began with a `-wal`, the test records that `-wal`'s digest
    immediately before the read and asserts the same digest after it.
  - This applies to the refusal in
    `reading_leaves_the_journal_and_the_retained_file_unchanged`, and to
    each read after the first in
    `growth_reads_keep_the_tree_config_and_journal_inert`.
- **Frame-bearing fixture.** A new fixture keeps the writer `Store` that
  appended the transcript reference open and idle across the three command
  reads (success, refusal, growth). The fixture stays far below SQLite's
  1000-page autocheckpoint.
  - Before the first read, the test asserts `wal_has_frames`.
  - Each read's JSON resolves the reference, which proves the frames were
    read.
  - After each read the `-wal` digest and the database digest are
    unchanged.
  - The `-shm` is not compared.
  - The writer drops only after the assertions.
- **Missing journal.** The same helper asserts absence: the database path,
  its `-wal` and its `-shm` still do not exist.
  - The command: a new test in `tests/transcript_command.rs`, with `--db`
    naming a path that does not exist.
  - The browser: a new test in `ui/tests.rs` that calls `ui::handle` for
    `/api/runs`, `/api/run/<run>`, `/api/view/<run>` and
    `/api/presentation/<run>/<key>`, and makes one `serve_io` poll of
    `/sse/<run>` with `sse_limit = Some(1)`. It asserts 404 for each route
    and head sequence zero for the stream.
  - The TUI: the existing `tests/machine_proof.rs:3429-3440` assertion,
    re-run on the final head.

The store links rusqlite's bundled SQLite (`libsqlite3-sys` 0.30.1), and the
lockfile pins it. The 3.46.1 probe in S14 is evidence that the rule can
hold. The tracked tests are the proof.

### D10 — A proposed addendum to 0055, and a contributor note

`docs/decisions/0055-read-every-transcript-kind.md` gains an addendum with
status `proposed`, filed before any production edit and dated on filing:
the date is the addendum commit's own date, matching how addenda 0035 and
0042 carry theirs. It is not guessed ahead of the commit, and it is not
re-derived afterwards if a later commit moves. It records two things:

1. The journal-inertness clarification. The read-only open writes no
   journal content. SQLite's own empty `-wal` and its `-shm` may appear. A
   `-wal` that already exists keeps its digest, and a missing journal gains
   no file. This supersedes archived D7's sidecar sentence.
2. Ruling 5's enforcement binding gains the unit-test-only reader fault
   seam, bounded as in D2–D4 and D7.

No other ruling changes. Only the operator accepts the addendum.

The contributor note lands in `docs/guides/contributing-by-hand.md`, in the
existing section **"The four refusal shapes"** under "The coverage gate,
practically" (`:407`), as one short paragraph after that section's first
shape. That shape already carries the guidance this note extends: "A new
`if` or `match` arm no test reaches. The most common one, and usually an
error arm. The fix is the test case that takes the arm — not deleting the
arm." `CONTRIBUTING.md` holds no coverage guidance of its own — its single
coverage mention (`:107`) is a pointer to this guide — so its pointer line
stays as it is, and the note is not duplicated there.

The note says how to reach a reader error arm, in order: prefer an ordinary
input, then a real change timed by `safe_fs::fault`, then a scripted error.
It says that entries must fire, and that the seam exists only in the
`brokkr-cli` unit-test build. It also says that the `fault` module is not a
test module: it is counted code in a production file, reached by tests, so
the same section's test-module placement rule (`:432-438`, sibling file or
`crates/<crate>/tests/`) does not apply to it and does not exempt it from
the counter. `safe_fs.rs` already carries an inline, counted
`#[cfg(test)] mod tests` (`:514`), so the `fault` module sets no precedent
in that file.

The proposal's Impact line, this decision and task 9.1 name that file and
that section in the same words. F3 was raised because two artifacts
described one landing place differently and one of them pointed nowhere;
repairing them separately would recreate it.

### D11 — The rest of #222 closes on this branch

- **Review repairs.** For each of M1–M11 and L1–L11 in chief review 7505,
  the verification record names the tracked regression test and its result
  on the final head. Where a repair has no tracked test, that absence is a
  finding to repair, not a residual.
- **Security residual.** `has_security_residual` (review 7505) is carried in
  every result until an independent review clears it on evidence.
- **Integration fixture ports.** The ports run read-only against the
  integration commits. The inspection set is `c2a7da4`, `fcd3b93`,
  `49ef15d`, `8d2211d`, `c7ff0fa`, `10908b6`, `82c802a`, `05ab44b` and
  `bfd677b`. Each is compared with this branch's files. A test-only repair
  whose unrepaired form is still present is ported by content.
  - `c2a7da4` is the one production change in the set. It is checked
    against the lossless `i128` identity `widen` already carries.
  - `bfd677b` repairs `a_working_seats_transcript_is_re_resolved_without_a_journal_move`
    (`src/tests.rs`). This branch already stages a copy there and renames
    it over the unopened target (`src/tests.rs:2222-2229`), so the check
    confirms whether that already covers `bfd677b`'s inode-reuse
    purpose.
  - The verification record gives each commit a disposition: ported,
    already subsumed, or not applicable.
  - `../brokkr-222-integration` and `main` are never written.
- **Coverage re-trace.** Verification re-measures and re-traces S11 against
  the newest byte-identical host measurement. A non-reader miss without a
  box cause is owned and repaired.
- **Seam boundary inspection.** D7's four-clause inspection runs on the
  final head as task 8.5, and the verification record quotes the command
  and the output, not the conclusion.
- **Gates.** The gates are:
  - `cargo fmt --check`;
  - clippy with `-D warnings`;
  - the workspace tests;
  - the `bundles/self` and `bundles/verify` compiles;
  - the release build;
  - `openspec validate --all --strict`;
  - the frozen-byte comparison against `5bc8cf3`;
  - the Rust 1.88 `--all-targets` check, recorded as pending if the
    toolchain is absent;
  - exact coverage through the unchanged script, run under
    `CARGO_BUILD_JOBS=2` and `RUST_TEST_THREADS=2`.

  Host coverage with `TMPDIR=/var/tmp` and
  `BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1`, remote CI and publication stay
  controller handoff. They are recorded as pending outside the tracked
  checkboxes.
- **Archive.** When every task is checked, the change folds into
  `openspec/specs/transcript-reading/spec.md` with one normal archive: the
  modified requirement is replaced and the new one appended. Existing
  provenance lines are append-only (decision 0042).
- **Final review.** A fresh, independent final review closes the issue's
  engineering. Its run-integrity observations are recorded as observations,
  never as directions to override a gate.

## Risks / Trade-offs

- [An occurrence ordinal drifts after a reader refactor or a change in
  enumeration order] → Fixtures keep one entry per enumerated directory, and
  an unfired entry fails its test. A drift inside an asserted read changes
  that read's outcome (S9).
- [`#[cfg(test)]` statements inside production functions cost readability]
  → There are exactly seven one-line statements, each with a comment naming
  the seam. No hook takes or returns reader data.
- [The Windows hook is invisible to the Linux gate] → The placement witness
  and the removal test run on Windows CI (D6, D8). If Windows cannot express
  a change, that is recorded as a design finding. It never leads to a
  pathname fallback.
- [A restructure changes behavior] → Each proof shows that the removed arm
  is unreachable. Every reachable state keeps its outcome and its existing
  tests, and the new ordinary fixtures reach both sides of each new branch.
- [The frame-bearing fixture checkpoints early, or hits `SQLITE_BUSY`] → The
  test asserts the frames rather than assuming them, and stays far below
  autocheckpoint. A read-only WAL reader takes no write lock, and
  `patiently` absorbs transient busy.
- [A change closure touches files outside its tempdir] → Closures live in
  `ui/tests.rs` and are built from the fixture's own paths. Review checks
  this, because the seam cannot.
- [A leaked guard — `mem::forget`, `ManuallyDrop`, `Box::leak`, or a helper
  that swallows ownership — leaves a plan whose entries never fire, and
  nothing fails] → `#![deny(clippy::mem_forget)]` on `brokkr-cli` closes the
  mechanical pattern at the clippy gate; the D7 inspection's no-leak clause
  closes the rest by review; D4.5 says which is which and claims no
  `install`-time refusal for a leaked guard. A plan never affects another
  thread, and libtest's run-on-caller fallback does meet the refusal.
- [The box cannot run the namespace-guarded tests] → Those misses stay
  pending host proof with their S11 causes, and a skipped test is never
  recorded as proof.
- [The pre-existing `HOME` pattern in `ui/tests.rs` is not unwind-safe] →
  Recorded residual, out of scope. No test in this change mutates the
  environment.

## Migration Plan

Each step is a commit tagged `(#222)`, unsigned and never pushed.

1. File the proposed 0055 addendum (D10).
2. Add the `fault` module and its self-tests (D2, D4, D8).
3. Add the seven hook statements (D3), with the scripted-error and
   real-change tests and the placement witness (D5, D6).
4. Make the three restructures and the `claude_source` home parameter, each
   with its proof comment (D5).
5. Add the ordinary fixtures (D5).
6. Update the privacy tests and add the missing-journal tests (D9).
7. Port the integration fixtures and record each disposition (D11).
8. Re-measure, re-trace S11 and run every gate. Write the verification
   record with the M/L regression map and the security residual.
9. Add the contributor note to `docs/guides/contributing-by-hand.md`'s
   "The four refusal shapes" section. Get the final independent review.
   Fold the archive.

There is no data or contract migration: the journal format, frozen
surfaces, CLI and routes are unchanged. Rollback is reverting the commits.
The seam exists only in unit-test builds, so a revert changes no shipped
behavior. Reverting a restructure restores an arm that cannot run, and
brings its coverage miss back.

## Open Questions

None that would change the specs, the approach or the task breakdown. Three
things are pending evidence, not open decisions: each integration port's
disposition, established by diff during implementation; the Windows and
macOS results of the new tests; and the final-head host coverage run. The
last two are controller handoff.
