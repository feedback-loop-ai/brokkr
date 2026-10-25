# Design — the dispatch door reads this run's reservation under the admission lock (#835)

The proposal (S1 to S10) records the mechanism the operator ruled on
2026-10-24 and the clarify history that cleared it (`f7183e97`); the delta
(`capability-writer-lease`, three added requirements) holds the behaviour.
This design carries what the code does, the evidence for each choice, and
the council's two positions reconciled. A decision is stated once here and
cited from there. Where this design reversed a record of the proposal (S1,
S5, S8, and the proof S10 named) the proposal now says so at that record
and points here; everything else of it stands. Every line number is this
tree's (`999c820b`; the code is unchanged since `efa9af75`), re-verified
by this seat on each of its two visits, the second after the council's
second positions and the clarify that cleared `999c820b`
(`.forge/results/ffd18e5e-…-judge.json`).

## Context

**The race.** Every spawn door calls `admission::recheck(door.held)`
(`dispatch.rs:377`) before `verify_record` (`:382`), the charter's two checks
(`:400-420`), the re-walk (`:425-428`) and `settlement::spawning` (`:431`).
Panel members dispatch on scoped threads over one shared `Held`
(`panel.rs:270-291`, `:452-453`), so one member's recheck runs beside a
sibling's `spawning` in the same process. `recheck` reads the reservation
with no lock (`admission.rs:694-699`); `spawning` goes through `amend`, which
takes `root::locked` first (`:757`), reads (`:759`) and seals (`:761`). The
seal commits by `linkat(name, name.old)` then `renameat(staged, name)`
(`commit`, `root.rs:408-420`), syncs, and retires by `unlinkat(name.old)`
(`retired`, `:376-388`). `standing` (`root.rs:281-298`) opens and stats
`name` first, then looks for `name.old`; a stat taken between the seal's
link and its rename reads two links, and a look taken after the seal's
unlink finds no prior, so the `(None, named)` arm filters the stale stat on
`st_nlink == 1`, `bytes` refuses `Unknown::Participant`, and `recheck` maps
it to `Unknown::Lost`: `member driver did not spawn: dispatch refused: … the
attempt's reservation no longer stands`. That is the signature of both #835
failures, printed by `efa9af75`'s `effects` and reproduced in scratch on
this host (3 unlocked readers beside 3,000 seals: 201 refusals of 26,604
reads, then 865 of 33,612; `.forge/scratch/demo-widened.txt` carries both
resume tests failing with the signature once the window was widened).

**The operator's ruling (2026-10-24).** (1) Decision 0065 slice two's fence
is lifted for this one mechanism: `root.rs` and `admission.rs` may change;
nothing else of #853 is in scope. (2) The mechanism is (a): `recheck` reads
the reservation under the admission lock, as `spawning` does; `standing`'s
M2 read rule stays untouched; every other reader of the reservation is
audited and named in the commit with why it is safe or now locked. (3) An
engine change, heaviest council. Acceptance: a test, or the scratch stress
made a test, shows the wrong refusal before the fix and none after; the two
resume tests hold in-suite on this host, with counts; no assertion weakened.

**What the branch holds.** `efa9af75` (the diagnostic print; the resume
tests' assertions unchanged); the change's artifacts, `fba1d60e` through
`999c820b`, the last cleared by clarify with no finding; the council's two
positions under `.forge/design/positions/`, each written twice, the second
time at `999c820b` against D1 to D11 as committed; the earlier attempts'
measurements under `.forge/scratch/` (alone 100/100 each; in-suite 30 runs
of 1,378 tests, 0 failed, all with production code unchanged). No fix is
on the branch yet.

**The pieces the design builds on, as read in this tree.**

- `recheck` (`admission.rs:688-718`): `reserved_at` then `read`, then the
  attempt's membership, `redirected` under the fence rule, and the artifact
  root's identity. Its 22 direct test callers (10 in `admission/tests.rs`,
  3 in `fold/tests/routes.rs`, 9 in `fold/tests/restart.rs`) and every
  `spawn_site` caller hold no admission lock on their thread (every
  `taken()` use checked: `tests.rs:613`, `:817`, `sources.rs:1052`,
  `:1285`).
- `amend` (`:752-762`): lock, `reserved_at`, `read`, seal, in one function;
  the guard lives to the seal. `reserved_at` (`:729-731`) is
  `reserved_in(&held.coordination, [repo, run], true)`
  (`settlement.rs:469-478`).
- `locked` (`root.rs:221-235`): opens `LOCK` with `CREATE | NOFOLLOW`,
  refuses `Unknown::Lock` unless it is this user's single regular file
  (`single`, `:238-242`), then tries a non-blocking exclusive `flock` before
  it reads the deadline, sleeping at most `POLL` (10 ms, `:41`) between
  tries, and refuses `Unknown::Expired` once the deadline is spent. So an
  expired deadline on a free lock is admitted: a proof of `Expired` must
  hold the lock, as `an_unprotected_lock_refuses` (`admission/tests.rs:807-
  822`) holds it with `taken` (`:351`).
- `startup(Instant::now(), Duration::MAX)` is MB3's 30-second bound
  (`broker.rs:100`, `:171-173`), computed where `spawning` (`settlement.rs:
  386`), `witnessed` (`:336`), `settled_driver` (`:425`) and `attested`
  (`attestation.rs:42`) compute theirs, in the effect crate.
- `From<Unknown> for Unadmitted` (`admission.rs:192-196`) is the one
  conversion, so `Lock` and `Expired` carry one text at the door and at
  admission. `Reservation` is closed (`deny_unknown_fields`, `:203`).
- The admission suite (`admission/tests.rs`, 1,820 lines): `Host` (`:272-
  281`) makes a HOME in `TMPDIR` with its root opened; `open` (`:451`) is a
  claim that protects nothing with one unboxed writer, so its lease has no
  authored boxes and no artifact root; `Host::admit` (`:344`),
  `Host::root()` (`:321`), `Host::reservation()` (`:327`), `taken` (`:351`),
  `soon` (`:362`), `mpsc` imported (`:16`), `LOCK` in scope through
  `use super::*`. `admission.rs` is 796 lines against a ceiling of 800;
  `quality/ratchet.sh` lets a file grow to its ceiling or its baseline,
  whichever is higher. jscpd reports a clone from 50 tokens over 5 lines.

## Goals and non-goals

Goals:

1. A panel member's spawn is never refused because a sibling's seal of the
   same reservation is in flight: the door waits for the seal and reads the
   record it committed.
2. Every refusal the door makes keeps its variant and its text; a lock not
   had is refused by the lock's name, never as a loss (ruling 8).
3. A test in the suite fails on the unfixed tree with the race's own
   refusal whenever the unlocked door reaches its read within the test's
   hold, and passes on the fixed one, which no time can fail short of the
   broker's own deadline; and a second binds the lock's own refusal at the
   door, with no bound at all (ruling 9).
4. The two resume tests hold alone and in-suite on this host, with counts,
   their assertions as `efa9af75` left them.
5. Every reader of a reservation is named in the commit with its lock.
6. The smallest footprint that holds those: one production file, no new
   module, no new seam, no helper moved.

Non-goals: `standing`'s read rule (M2), the seal's steps, `amend`'s order,
the lock's wait, mechanism (b); decision 0065 slice two's other open work
(#853); the `Unadmitted::Identity` prefix that misnames a lost reservation
at the door (`46fbcbab`, a text follow-up); any retry of a refused spawn
(decision 0006); contracts, `policy/`, `reference/`, fixtures, recipes,
agents, flags; a stress test in the suite; a root-bound lock token type; a
`Hook` step between a seal's link and its rename; a dispatch-level door
test; the sweep's other flakes (#857).

## The council, reconciled

Each claim is adopted, rejected or combined on evidence, with the ruling of
decision 0071 it turns on. The council sat twice: at `c6e557cf`, on the
proposal, and at `999c820b`, on D1 to D11 as committed. The second visit
moved no mechanism. Simplicity asked that the design not be reopened and
offered two cuts of text; robustness withdrew its stress mandate and its
helper move on the scope evidence, concurred with D1, D5 and D6, and asked
that the design claim of its wait test only what a bounded observation
proves. Each is answered below where its claim lands. The clarify of
`999c820b` found nothing against the artifacts; the two citation slips
and the notes on D8 it considered and did not count are taken up in D1
and D8 all the same.

**Adopted from both positions.** Mechanism (a), the operator's: the read
under `root::locked`, taken the one way (ruling 5), before `reserved_at`
enters the run's directory (D1). The lock scoped to the read and released
before `redirected`, the identity comparison and `spawning`, which takes its
own; two takes, no transaction across them (D4). Each refusal its own
variant, `Lost` mapped from the read alone (D3; ruling 8). The reader audit
in the commit, as ruled (D5). No sleep, retry or widened timeout; the resume
assertions exact (D8). Commit the fix first, then measure within the budget
(D8). M2, the seal steps and `amend` untouched.

**Adopted from simplicity alone.** The lock take as a block inside
`recheck`, six lines for two, and no `recheck_by` (D1, D2; ruling 6).
Nothing moves: `root.rs` and `settlement.rs` are untouched, `admission.rs`
ends at 800, its ceiling (D9). The tests in `admission/tests.rs` beside the
door's existing tests, no `door.rs` (D9; ruling 5). The binding test models
a seal in flight by its two invariants, the lock held and the record as a
descheduled reader composes it, which `standing` refuses: the race's own
refusal before the fix, with no wait and no load-dependent rate, and
admission after (D6). The budgeted stress is not a suite test (D7). The
`Expired`-at-the-door scenarios go: `Expired` is `locked`'s refusal, proven
with the lock held at admission, and reaches the door by the one conversion
the lock-file test proves there (D3). From its second visit, two cuts of
text: `recheck`'s doc comment gains four words, `read under the admission
lock`, and is otherwise reflowed within its six lines, the reach clause
and the ruling citation unchanged and no issue number added, for the
number's home is the commit and the tests' docs (ruling 5; D1); and the
open question of a shared constant for a test's bounded hold is dropped,
for it named a consumer that does not exist (ruling 6).

**Adopted from robustness alone.** Its invariants become the implementer's
checklist and the tests' shape: the guard is bound as `let _lock`, never
`let _`, which drops at once (D1); the guard and the run directory's
descriptor live through the typed read, in one block (D1); the lock guards
`held.coordination.dir()`, the root the lease was admitted under (D1);
`locked` tries the flock before the deadline, so no spent-deadline test is
specified at the door and the `Expired` proof is the one that holds the
lock (D3); the precedence change is named, not hidden: a doubly-refused
lease now reports the lock's refusal, and one held to the deadline is
refused when it is spent (D4); the two takes are no enduring guarantee, and
`spawning`'s re-read and refusals stand (D4); a test thread must not be
stranded by a panic, so the lock's holder lives inside the scope (D6); a
message sent before the call proves nothing about reaching the lock, so the
negative half of the wait test is a bounded observation and is called one
(D6); "zero refusals with zero useful work is not a pass", which is one
reason the stress leaves the suite rather than gaining a floor (D7). The
Lock and wait behaviours are proven by two tests, one per mutation, so each
mutation names the test it fails (ruling 9), where simplicity drafted one.
From its second visit: the words "deterministically" and "every time",
which goal 3, this section, D6 and D7 and the proposal's S5 and proof
used of the unfixed tree, are replaced by what the observation proves:
the unlocked door makes no wait, so the test fails whenever that door
reaches its read within `HOLD`, and a thread descheduled past `HOLD`
before its read is the one way the mutation survives a run, which the
recorded run is read for and the lock-file test closes (D6, D7). The
fixed test's one time-bound failure is named too: a door kept from the
released lock for the broker's 30 seconds answers `Expired`, the
deadline's own refusal and not the test's (D6, Risks). The residual on
`Expired`, that no door test binds a remapping of `Expired` alone, is
named as robustness asks (D2, Risks), as is the reader audit's standing
as today's substitute for a type-level guarantee (D5; open question 2).

**Rejected from robustness.** Retaining the stress and adding progress
tracking or coordinated rounds to show exercised contention (D7): more
machinery for evidence its own author calls inconclusive, a timing test
inside a timing-flake sweep, and a floor that fails by name on a slow host.
`recheck_by` as the operation under a bounded wrapper (D2): its only
consumer beyond `recheck` is a test whose facts are proven elsewhere, and
it costs the `identity` move into two more files. Robustness's second visit
withdraws both on the scope evidence, so they stand rejected on the record
and in no contention. A root-bound lock token (named by robustness as
separately authorized): outside this mechanism's fence, recorded under open
questions. From its second visit, the `let _` mutation offered "if needed"
to show the wait test binds the guard's lifetime: traced against D6's
test, it does not bind, so it is not recorded as a mutation that fails.
`let _ = locked(…)?` takes the lock and drops it at the statement's end
(`locked` returns the `OwnedFd` whose drop releases it, `root.rs:218-221`),
so that door waits in `locked`'s loop while the holder holds, as the fixed
door does, then reads after the heal and admits; and the lock-file test
refuses `Lock` through the same `?`. Both pass. The lifetime is named as a
residual the tests do not prove (D6), owned by review, with D1 naming the
binding as `amend` binds its own (`admission.rs:757`). No test binds it
without a seam between the door's take and its read, a new `Hook` in a
fenced change.

**Rejected from simplicity.** One test for both behaviours: split in two
(D6). The assertion that the reservation's bytes are unchanged after a
`Lock` refusal: `recheck` has no write path, so the assertion cannot fail
and would be the judges' medium row; dropped from the test and the
scenario. Cutting the second requirement's enumeration of the door's later
checks to one sentence: the clarify of `781b5c30` asked for those checks to
be named and `f7183e97` cleared the text as written; it is true, marked
"today", and is the behaviour the door now has (D4). Dropping the
doubly-refused scenario: it encodes an answered ambiguity, which the
dialect's return rule says a scenario does; it is kept, with the proof it
rests on named (D4). Re-quoting the scratch stress as the only before side:
the modelled test is the before side the acceptance names, and the
scratch is quoted beside it (D7, D8).

**Combined.** The operator's acceptance is read with both: "a test … shows
the wrong refusal before the fix and none after" is met by D6's first test,
run on the unfixed tree before the fix lands (tests first, as the commission
says) and recorded; the scratch's real-race counts and the resume tests'
loops are the evidence around it (D8).

**Reversed in the proposal.** S1 (no `recheck_by`: D2), S5 (the stress and
the `Expired` door test: D6, D7), S8 (nothing moves: D9), and the proof S10
named (D4, D6). The proposal's What Changes, Impact and the delta's
scenarios are amended to match, in the same commit as this design.

## Decisions

### D1. The fix: the read under the lock, in one block, in `admission.rs` alone

`recheck` keeps its signature and its place; two of its lines become six:

```rust
pub(in crate::engine) fn recheck(held: &Held) -> Result<(), Unadmitted> {
    let standing: Reservation = {
        let deadline = startup(Instant::now(), Duration::MAX);
        let _lock = locked(held.coordination.dir(), deadline)?;
        let (run, at) = reserved_at(held)?;
        read(&run, RESERVATION, &at).or(Err(Unknown::Lost))?
    };
    let ours = (standing.repo == held.repo) & (standing.run == held.run);
    // … unchanged from here
```

- `locked`, `read`, `startup`, `Duration` and `Instant` are already imported
  (`admission.rs:41`, `:51`, `:54`); no import line is added.
- The guard is `_lock`, a named binding, as `amend` binds its own (`:757`),
  so it lives to the block's end; `let _ = locked(…)` would drop the
  descriptor at the statement's end and read unlocked, which no test of D6
  can tell from the fixed door (robustness's first invariant, owned by
  review; D6). The run directory's descriptor `run` and
  the guard drop together at the brace, before `ours` is judged, before
  `redirected` observes hands and Git, before the artifact root is compared,
  and before `spawning` takes its own lock (D4).
- The lock is the one `amend` takes, on the one root,
  `held.coordination.dir()`, with the one deadline
  (`startup(Instant::now(), Duration::MAX)`; D2), bound on its own line as
  every broker caller binds it (`settlement.rs:242`, `:386`, `:425`, `:457`;
  `attestation.rs:42`) save `witnessed` (`:336`), whose call fits. With the
  deadline inside the call the arguments are 63 columns, and the tree is
  formatted to rustfmt's defaults (no `rustfmt.toml`): a call whose
  arguments pass `fn_call_width`'s 60 is laid one argument per line, as
  `fold/tests.rs:1010-1014` (62 columns) and
  `admission/tests/sealing.rs:123-127` (66) are, while one of exactly 60
  keeps its line (`admission/tests.rs:760`). So the inline form would be
  four lines, the block eight and the file 802, which the files gate refuses
  (the clarify of `e6623cbf`, finding 1). The clock is read in the effect
  crate, which ruling 1 allows.
- `?` on `locked` and on `reserved_at` converts `Unknown` to
  `Unadmitted::Identity` through the one `From` impl, as `amend`'s do; only
  the read's failure is mapped to `Lost`, as today (D3).
- The block observes, the lines after it decide (ruling 10).
- `recheck`'s doc comment (`:688-693`) gains four words, `read under the
  admission lock`, and is reflowed within its six lines, each at most 77
  columns where the file keeps its doc lines within 80; nothing else of it
  changes, not the reach clause nor the ruling it cites, and no issue
  number is added, for the number's home is the commit and the tests' docs
  (simplicity's second visit; ruling 5):

  ```
  /// Whether the lease `held` still stands at the dispatch door: its
  /// reservation, read under the admission lock, still names its attempt
  /// standing, every path each boxed writer's box is built from, observed
  /// again from its authored hands and from Git, still resolves as sealed (R1;
  /// operator ruling 2026-10-18, after U8a2a's third run), under the fence
  /// rule, and the artifact root it protects is still the directory admitted.
  ```
- `admission.rs`: 796 + 4 = 800 lines, its ceiling, which the files gate
  admits (D9). `cargo fmt --all -- --check` is run on the block before the
  count is read, so the count the gate reads is the formatted one.
  `recheck`'s cyclomatic complexity gains one `?`; the ratchet's allowance
  is the higher of 15 and its baseline, and the gate is the authority.

Alternatives. A shared helper for the three lines `amend` and `recheck` now
both have (lock, `reserved_at`, `read`): refused, because `amend` must keep
the guard to its seal while `recheck` must drop it at the read, so the
helper would hand back the guard, the descriptor, the path and the record
together, more surface than the three lines it saves, and three lines are
under jscpd's five-line floor. `recheck_by`: D2. Moving `identity` to make
room: D9. Mechanism (b), a second look in `standing`: ruled out by the
operator.

### D2. The deadline is computed in the block; there is no `recheck_by`

The proposal's S1 made `recheck` a wrapper computing the deadline and
`recheck_by(held, deadline)` the operation, after `witnessed` /
`witnessed_by`, so a test could spend the deadline and prove `Expired` at
the door, `Expired` over `Redirected` and `Expired` over `Lost`. This
design computes the deadline in `recheck`'s own block, bound on its own
line (D1), and adds no second entry (ruling 6):

- `Expired` is `locked`'s refusal, and a proof of it must hold the lock
  (`locked` tries the flock before it reads the deadline). It is proven that
  way at admission, `an_unprotected_lock_refuses`, and reaches the door
  through the same `?` and the same `From<Unknown>` that `Lock` takes; the
  lock-file test (D6) proves that conversion at the door. A mutation that
  folds the lock's refusals into `Lost` folds both and fails that test.
- `Expired` over `Redirected` would prove the lock is taken before the
  reach is observed. Both orders are fail-closed; what the order decides is
  only which of two refusals a doubly-refused lease reports, and the block
  shows it on the page. It is named in the delta (D4), not re-proven at the
  cost of an entry point.
- `Expired` over `Lost` for a reservation removed while the lock is held
  would prove the lock is taken before the read. D6's first test proves the
  same by its last assertion: a door whose lock were taken after its read
  reads the seal in flight and answers `Lost` after the release, not
  `Ok(())`.
- The cost avoided is not the function alone: about nine lines where the
  file has four of headroom, which is what forced S8's `identity` move
  into `root.rs` and its import edit in `settlement.rs`.

Handing the deadline in at the door is refused as S1 refused it: it changes
the door's call and 22 test callers for nothing, and the door's two takes
are separate waits with the bound each observes.

### D3. Each refusal keeps its own variant; a lock not had is not a loss

A lock not had by the deadline refuses
`Unadmitted::Identity(Unknown::Expired)`; a lock file that is not this
user's single regular file refuses `Identity(Unknown::Lock)`; a run
directory the door cannot enter keeps `Identity(Unknown::Root)`; each by
`?`, as `amend` refuses them. Only the read's own failure maps to `Lost`,
which keeps the read-failed and attempt-missing meaning
(`admission.rs:162-163`). No `Unadmitted` text changes and no variant is
added or removed. A reader that could not tell "the lock was busy" from
"the reservation is gone" would hide a fail-open path behind a retry or a
fail-closed one behind a wrong name: ruling 8, the judges' high row. The
lock-file test binds the conversion at the door; the
`.or(Err(Unknown::Lost))` mutation on the lock take fails it (D6).

### D4. The lock's scope, the door's order, and what moves

The lock is scoped to the read (D1). After the brace the door runs as
today, without it: the membership check, `redirected` under the fence rule
and the artifact root's identity inside `recheck`; then `verify_record`, the
charter text carried, the charter moved, the re-walk and `spawning`
(`dispatch.rs:382-431`), which takes its own lock. U8a2a keeps the host
admission lock short-lived, and the re-walk and `redirected` observe hands
and Git. Between the two takes a sibling may seal; `spawning` re-reads under
its lock and refuses `Lost` only where the attempt is truly gone (H1 of
U8e2's first run). The two takes are not a transaction and the design makes
none.

What moves, named as the delta's second requirement says it: today the
door's one lock take is `spawning`'s, its last check; now the lock is the
door's first take, once a spawn composed refused is turned away
(`dispatch.rs:373-375`). A lease the lock refuses that any later check would
also refuse, the recheck's `Root`, `Lost`, `Redirected` or `Replaced`, or
the door's launch record, charter or re-walk, reports the lock's refusal
where today it reports the other and reaches no lock; one whose lock is
held to the deadline is refused when the deadline is spent, not at once. In
every such case both refuse, nothing is observed and nothing spawns:
fail-closed. `recheck` keeps its place at the door (S10's two refused
alternatives stand: moving it behind the re-walk reorders a door the
2026-10-18 ruling placed, and folding the takes holds the lock across the
re-walk). What proves it: the lock-file test proves the take inside
`recheck` refuses before anything is read; the seal-in-flight test proves
the take precedes the read; `recheck` running first at the door is today's
code, unchanged, not re-proven. No dispatch-level test is added (S10 names
the cheap form should a judge want it).

Confirmed, not changed: no production caller of `spawn_site` holds the
admission lock when it reaches the door. `admit_with`, `extended_with`,
`adopted` and `attested` release theirs on return, before any door; a lock
held on the calling thread would contend with the door's own descriptor
(`flock` contends between two descriptors of one process) and wait out the
30-second deadline to `Expired`, fail-closed. The suite would show it: every
door test would stall.

### D5. The readers audited, and the invariant's authority

S4 stands: admission makes no refusal of a dispatch, an amendment, an
adoption or an attestation from a read taken outside the lock, because
`standing` finds no record while a sibling's seal stands between its link
and its unlink. Its authority is the operator's ruling 2, U8a2a (every seal
under the lock) and U8e2 (the settlement record sealed under it, H1). Every
read of `RESERVATION` goes through `root::read` or `root::bytes`, both
through `standing`. The production readers in this tree, each with its
lock, for the commit to confirm:

| Reader | Lock | Finding |
| --- | --- | --- |
| `admission::recheck`, `admission.rs:696` | none today | the race; locked by D1 |
| `admission::amend`, `:759` | `locked` at `:757`, held to the seal | safe; its five callers inherit it: `proved` (`settlement.rs:317`), `witnessed_by` (`:345`), `spawning` (`:387`), `settled_driver` (`:426`), `release` (`admission.rs:743`) |
| `settlement::standing`, `settlement.rs:553` | its only caller `adopted` locks at `:512` | safe |
| `inventory/standing.rs:210`, through `inventory()` (`inventory.rs:168`) | `admit_with` at `admission.rs:623`; `sources::extended_with` at `sources.rs:590`; `attestation::attested` at `attestation.rs:43-44`, before `reserved_for` (`:76`), the guard alive through its seals | safe |

`settlement.rs:309` reads the fold's evidence record, not the reservation;
`attestation.rs:83` builds a path for a refusal's text and reads nothing.
No crate outside the runtime's engine reads a reservation: beyond the
broker, `reservation.json` and `RESERVATION` appear in test files only. The
commit names each reader this way, and any this audit missed the same way.

### D6. The tests that bind: a seal in flight, and a lock file refused by its name

Why the race is modelled and not raced in the suite: the `Hook` seam
(`root.rs:86`) is handed a seal at `Staged`, `Committed`, `Retiring`,
`Final` and `Undone`, and `commit` (`:408-420`) calls none between its link
and its rename, so the window cannot be held open without a new `Step`,
which changes the seal's steps, out of scope; and the refusal needs the
reader descheduled between its stat and its look for the prior, which no
test schedules. A seal in flight is therefore modelled by its two
invariants: the lock, which every production seal holds from its first step
to its last sync (D5), held by another thread through `taken`; and the
record as a reader descheduled between the seal's link and its unlink
composes it, `name` of two links and no `name.old`, which `standing`
refuses: `named` has `st_nlink == 2`, so `linked` is true and the prior is
looked for twice, both looks find none, and the `(None, named)` arm filters
on `st_nlink == 1` and returns none; `bytes` refuses `Participant`; `read`
propagates it; `recheck` maps it to `Lost`. The test builds that record by
hard-linking `reservation.json` to a name of its own outside the run's
directory (`st_nlink` is the inode's, wherever the second name lies), and
heals it by removing that link before the lock is released, as a seal's
unlink precedes its release.

Both tests go in `admission/tests.rs` after the existing `recheck` tests,
on the suite's builders, with one constant beside `soon`:

```rust
/// How long the seal-in-flight test holds the lock after the door's call,
/// so a door that reads without the lock (the binding mutation) shows
/// itself by answering. It bounds only that: the fixed door waits on the
/// lock to its own startup deadline, and never on this (#835).
const HOLD: Duration = Duration::from_millis(500);
```

```rust
/// #835: the door reads the reservation under the admission lock, so a
/// sibling's seal in flight is waited for and never read as a loss. The
/// seal in flight is modelled by what every seal holds, the lock, and what
/// a reader descheduled between its link and its unlink finds, the record
/// of two links and no prior, which `standing` refuses. Without the lock
/// the door answers that refusal at once; with it, nothing until the record
/// is whole and the lock released, and then admission.
#[test]
fn the_door_waits_for_a_seal_in_flight() {
    let host = Host::new();
    let work = host.worktree("work");
    let held = host.admit(&open(&work, ["run-a", "a1"])).unwrap();
    let mid_seal = host.dir.path().join("mid-seal");
    std::fs::hard_link(host.reservation(&work, "run-a"), &mid_seal).unwrap();
    std::thread::scope(|scope| {
        let holder = taken(&host.root().join(LOCK));
        let (door, answered) = mpsc::channel();
        scope.spawn(move || {
            let _ = door.send(recheck(&held));
        });
        let early = answered.recv_timeout(HOLD);
        assert_eq!(
            early,
            Err(mpsc::RecvTimeoutError::Timeout),
            "read through the held lock"
        );
        std::fs::remove_file(&mid_seal).unwrap();
        drop(holder);
        assert_eq!(answered.recv().unwrap(), Ok(()));
    });
}
```

- The holder lives inside the scope, so a panic in the closure drops it
  during the unwind and the door's thread is released before the scope
  joins; the door's thread ignores a send to a receiver gone the same way.
  On the fixed code the door's own deadline is 30 s, so no failure path
  waits longer than that in any case.
- `early` is `Timeout` on the fixed code by construction, not by timing:
  the door is inside `locked`'s loop until the holder drops, which happens
  after this assertion, and nothing but the broker's 30-second deadline
  takes it out of that loop sooner. The last assertion follows from the
  heal: after it the record has one link, the door's next poll (within
  10 ms) takes the lock, `standing` returns the record, the attempt stands
  in it, `open`'s lease has no authored boxes and no artifact root, so
  `recheck` is `Ok(())`. The one way time fails the fixed test is a door
  kept from the released lock for the 30 seconds its deadline allows, which
  answers `Expired`, the deadline's own refusal: the broker's bound, not
  the test's, and a host that stalls a thread so long fails every door test
  alike.
- The binding mutation, recorded in the commit: the `locked` line removed.
  That door makes no wait, so whenever its thread reaches the read within
  `HOLD`, `early` is `Ok(Err(Unadmitted::Identity(Unknown::Lost)))`, the
  race's own refusal, and the first assertion fails naming it. That is the
  operator's "wrong refusal before the fix"; the passing run after is "none
  after". The claim is a bounded observation, as robustness asks it be
  called: a thread descheduled past `HOLD` before its read is the one way
  the mutation survives a run, so the recorded run is read for the refusal
  it names, never for a pass, and the lock-file test closes the same
  mutation with no bound. A door whose lock were taken after its read would
  also fail the last assertion with `Lost`, so the test binds the take's
  place before the read.
- `HOLD` bounds only how long the mutation is given to show itself. The
  fixed door cannot fail this test by time, which is why the bound is the
  test's and not the code's; the operator's rule on a bound that moves (a
  fail-closed limit and a comment saying why) is met by the constant's
  comment. Under a load that deschedules the mutated door for over 500 ms
  the mutation could survive one run; the lock-file test catches the same
  mutation with no bound, and the implementer's recorded mutation run
  is read for the refusal it names, not for a pass.

```rust
/// #835: the door's take of the lock is `locked`'s, so a lock file that is
/// not this user's single regular file refuses the dispatch by the lock's
/// own name, as it refuses admission, and nothing is read; with the second
/// link gone the same lease is admitted.
#[test]
fn the_door_refuses_a_lock_file_by_its_own_name() {
    let host = Host::new();
    let work = host.worktree("work");
    let held = host.admit(&open(&work, ["run-a", "a1"])).unwrap();
    let second = host.dir.path().join("second-link");
    std::fs::hard_link(host.root().join(LOCK), &second).unwrap();
    assert_eq!(recheck(&held), Err(Unadmitted::Identity(Unknown::Lock)));
    std::fs::remove_file(&second).unwrap();
    assert_eq!(recheck(&held), Ok(()));
}
```

- Mutations, recorded in the commit: the `locked` line removed, and the
  first assertion gets `Ok(())`; the lock's refusals folded into the read's
  (`locked(…).or(Err(Unknown::Lost))?`), and it gets `Lost` where `Lock` is
  asserted (D3).
- A directory, a link or another user's file at the lock's name is refused
  by the same open and check (`root.rs:222-224`, `:238-242`), proven once
  at the lock; the door test proves the second link, as the admission test
  does.

Tests first, as the commission says: the implementer writes both, runs
them on the unfixed tree and records the two failures (the first with
`Lost` at `early`, the second with `Ok(())` at the `Lock` assertion), then
lands D1 and runs them again, then commits before any measuring (D8). Both
tests reuse `Host`, `open`, `taken` and `LOCK`; the shared prologue is
three lines, under jscpd's floor. About 55 lines in all as rustfmt lays
them (the `early` assertion's three arguments are 74 columns, so they stand
one per line), `tests.rs` at about 1,875 of 2,000 (D9). The existing
`recheck` tests, the redirected tests and every `sealing.rs` and
`coordination.rs` test stay green and unchanged. `resume_tests.rs` is not
touched beyond `efa9af75`.

What these tests do not prove, said plainly: that the guard lives through
the read (a `let _` door waits for the holder as the fixed door does, drops
the lock at its statement's end, reads the healed record and admits, and
refuses the second link through the same `?`, so it passes both; the
binding's name is D1's and review's, as `amend`'s is); that the guard is
dropped before `redirected` (an efficiency rule of U8a2a, read on the page;
a wider scope changes no result here); and contention under real seals,
which the resume tests in-suite and the scratch carry (D7, D8). Binding the
first would need a seam between the door's take and its read, a new `Hook`
in a fenced change, for a mutation that leaves a narrower window than
today's and that review reads on one line.

### D7. The stress is not a suite test

The proposal's S5 added a budgeted stress: three readers of 400 reads
beside a sealer under a 2-second budget, a floor of 10 and a cap of 3,000
seals, one shared 20-second deadline, per-variant tallies, run on tmpfs and
on disk. It is cut, for these reasons together:

- The operator's acceptance is disjunctive, "a test, or the scratch stress
  made a test", and D6's first test meets its first clause strictly better:
  on the unfixed tree it fails whenever the unlocked door reaches its read
  within `HOLD`, with no rate below that to wait for, where the stress's
  mutated form refused at 0.76 % and 2.6 % of reads under this host's load
  and, by S5's own words, is "load-sensitive evidence, not a proof".
- Robustness shows what a passing stress proves without more machinery:
  little. "Zero `Lost` with zero useful work is not a pass", and a
  simultaneous start with final totals does not show the readers ran while
  the sealer sealed. Curing that needs progress tracking or coordinated
  rounds, more test structure for evidence that stays non-proof.
- It is a timing test inside the sweep that removes timing tests (#857).
  Its floor of 10 seals in 2 seconds is fail-closed, which means it fails
  by name on a host whose fsync is slow, a new flake candidate in the gate
  this change exists to steady.
- Three clarify rounds were spent on its bounds and none on the fix.

The real-race evidence stays on record and is quoted in the commit: before,
the scratch stress of `efa9af75` (201 of 26,604; 865 of 33,612) and the
widened reproduction (`demo-widened.txt`, both tests failing with the
signature); after, the two resume tests looped alone and in-suite, which is
the operator's own second acceptance item. `.forge/scratch/seal-timing.py`
and the seal rates `781b5c30`'s message quotes remain as measurements and
are no longer load-bearing. The delta's "Readers beside many seals"
scenario is replaced by the seal-in-flight scenario D6 proves.

### D8. Evidence and the budget (S6 stands)

No sleep, retry or widened timeout is the fix; `recheck` waits on the lock
only, to its deadline, then refuses. The implementer commits as soon as the
fix builds and D6's tests pass, before the workspace validation and before
any measuring: the attempt is killed at two hours, and the earlier attempts'
workspace runs on this host took about 1,700 s and 1,500 s of test time
before build time (`.forge/scratch/workspace-full.txt`,
`workspace-rest.txt`). What runs before that commit is what its message
carries and nothing that measures: `cargo fmt --all -- --check`, which
compiles nothing and gives the files gate its formatted count (D1, D9);
the two mutation runs, each on the one test binary the fix already built,
whose recorded failures ruling 9 asks for; the door's neighbours, filtered
in that binary; and the reader audit's grep (tasks 2.1 to 2.4; the clarify
of `999c820b`, note b). Clippy, `cargo test --workspace` split by crate
around the
seat's 20-minute command limit, both bundles and the ratchets run after the
commit and amend it with their results, unsigned (the clarify of
`e6623cbf`, finding 2). Then, in 45 minutes at most: up to 20 runs of each
resume test alone, filtered to itself as its before side was
(`before-alone-seq.txt` and `before-alone-fresh.txt` under
`.forge/scratch/`, where every earlier attempt's measurement is, the
commission's `.forge/loops/` never having existed on this worktree; each
reads `running 1 test … 1377 filtered out`, 100 runs each): `cargo test -p
brokkr-runtime
--lib -- engine::resume_tests::a_sequence_offers`, then the same for
`engine::resume_tests::a_fresh_process_derives`, 40 invocations at most,
each test's count set beside its own before side; and up to 3 in-suite runs
(`cargo test -p brokkr-runtime --lib`), beside the before side's 30 runs of
1,378 tests passed, 0 failed (`suite-a.txt` to `suite-c.txt`; in 21 of the
30 the binary held a 1,379th test, an earlier attempt's scratch test,
ignored, so a run's `1378 passed` is the count to set beside; the clarify
of `999c820b`, note e). The commission's
budget names one test and #835 has two, so each test has the budget of one,
and the one 45 minutes bounds both alone loops and the in-suite runs
together (finding 3). The counts in hand when the budget ends go in the
report and an amended commit message; the fix is never reverted to measure.
The commit message carries: the reader audit (D5), the mutations and the
tests each fails (D6), the two recorded failures on the unfixed tree, the
gates' results and the counts. A tempdir test failing with `root: "/tmp"`
is another sandbox's `/tmp/.git`, reported by signature; other timing tests
that fail under load and pass alone and in-suite are #857's, reported and
not fixed.

### D9. Ceilings: `admission.rs` ends at 800, its ceiling, and nothing moves (S8 reversed)

D1 adds four lines to a file of 796 against a ceiling of 800, and the doc
comment is reworded within its six lines (D1). `quality/ratchet.sh files`
lets a file grow to its ceiling or its baseline, whichever is higher, and
refuses only a count above it (`if ($1 > allowed)`, `ratchet.sh:196`); the
count is `wc -l`'s (`rs_lines`, `quality/lib.sh:122`, which `file_lines` at
`:123` runs over every Rust file `git ls-files` names). So 800 passes, with
no line to spare, and the count the implementer reads with `wc -l` is the
gate's. S8's move of `identity` into `root.rs`, with its `MetadataExt` and
`Unrooted` imports and the import edit in `settlement.rs`, existed to make
room for `recheck_by` (D2) and is not made: a third file in a fenced change
for no behaviour, and `settlement.rs` is outside the operator's lift.
`root.rs` is untouched, which honours ruling 1 of the operator's by the
plainest reading.

No fallback stands (the clarify of `e6623cbf`, finding 1). The earlier
records kept S8's move as the one fallback "if the gate's own count
disagrees with this arithmetic"; the gate's rule and count are read above,
so no disagreement is possible, and the move's import edit reaches
`settlement.rs`, which the lift does not name, so it is not a move this
change may make, and the tasks' file check would refuse the commit that
made it. Should the files gate refuse 800 even so, the implementer stops
and reports the count as a finding, moving nothing. The next change that
grows `admission.rs` pays for its room first, by that move or another,
under its own scope.

`admission/tests.rs` grows by about 55 lines to about 1,875 of its 2,000; no
new test module (ruling 5: the door's tests have one home, beside
`recheck`'s existing tests). `dispatch.rs`, `panel.rs`, `root.rs`,
`settlement.rs`: unchanged.

### D10. No decision document, no new seam, no new type (S7 stands)

The change alters no rule: D6's admission operation reads and seals under
the lock, U8e2 put the door's settlement record under it, and the door's
read was the one read of the reservation that could refuse outside it.
Mechanism (a) brings it under the lock the others hold, changing no
variant, text, read rule, seal step or lock rule. The operator's ruling of
2026-10-24 is recorded in the commission, the proposal and here. No trait is
added (`ProducerTransport` stays the one seam), no `_with` closure seam, no
new type; `HOLD` is a test constant whose comment says what it bounds.

### D11. Deliberately left out

`recheck_by`; the `identity` move and the `settlement.rs` import edit; the
`door.rs` module; the budgeted stress and its disk-and-tmpfs runs; a `Hook`
`Step` between a seal's link and its rename; a second look in `standing`
(mechanism (b)); a root-bound lock token or any retrofit of `root::read`'s
API; a dispatch-level test holding the lock against `spawn_site`; the
`Unadmitted::Identity` prefix text at the door; any change to
`resume_tests.rs` beyond `efa9af75`; any item of #853.

## Risks

- **The mutated wait test can pass once under extreme load** (a mutated
  door descheduled for over `HOLD` before its read), so its catch is a
  bounded observation and is never called deterministic. One-way but for
  the broker's own bound: the fixed door cannot fail this test by time
  short of the 30 seconds its deadline allows, after which it answers
  `Expired`, the deadline's refusal and not the test's. The lock-file test
  catches the same mutation with no bound, and the recorded mutation run is
  read for the refusal it names (D6).
- **The race is modelled, not raced, in the suite.** The model is exact for
  what `standing` computes (traced in D6) and rests on D5's audit that every
  production seal holds the lock. The real race's reproduction is the
  scratch's, quoted; the resume tests in-suite are the integration evidence
  (D7).
- **`Expired` is not asserted at the door.** It is `locked`'s refusal,
  proven with the lock held at admission; the door's conversion is proven by
  `Lock`. A mutation that folds only `Expired` into `Lost` while keeping
  `Lock` is not one anyone writes; it is named here rather than bought with
  `recheck_by` (D2).
- **The lock's scope and the guard's lifetime have no binding test.** A
  guard held across `redirected` changes no result, only contention; a
  `let _` guard dropped before the read passes both tests and leaves a
  narrower window than today's. Code review owns both, the block shows them
  on two lines, and D1 names the binding as `amend` names its own (D4, D6).
- **`admission.rs` at 800 of 800.** The next change that grows it moves
  something out first, under its own scope; this change keeps no fallback
  move, and a gate refusal of 800 is reported, not moved around (D9).
- **Every door now waits on a sibling's seal**, a few milliseconds on disk
  under load, and takes the lock twice per door. U8a2a accepted the
  serialization at admission; the door's two takes are each short.
- **A doubly-refused lease reports the lock's refusal** where today it
  reports the later check's, and one whose lock is held to the deadline is
  refused at the deadline rather than at once. Fail-closed both ways;
  named in the delta (D4). Intended.
- **A refusal text follow-up stands open**: the `Unadmitted::Identity`
  prefix misnames a lost reservation at the door (`46fbcbab`). Out of
  scope; recorded under open questions.
- **The host's load at measuring time** (load average over 30 on 8 cores
  as this design was written) may leave the implementer with fewer runs
  than the budget allows. The counts in hand are quoted as they stand (D8).

## Migration

None. No record format, contract, fixture, recipe, agent, flag or knob
changes; no journal written before this change reads differently; a lease
admitted by an older engine is read at the door the same way, only under
the lock. Nothing to publish beyond the commit; the operator seals the
branch.

## Open questions

1. **The `Unadmitted::Identity` prefix at the door.** `MCP server box
   filesystem identity is not protected: … the attempt's reservation no
   longer stands` misnames a lost reservation (`46fbcbab`). A text change
   outside this run, for a follow-up on the branch's parent issue.
2. **A root-bound lock token.** Robustness notes `OwnedFd` does not encode
   "admission lock for this root". A type carrying that invariant would
   strengthen every reader's API; it is #853's territory under decision
   0065 slice two, not this mechanism's, and is left for that work.
