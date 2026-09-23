# Acceptance ledger — tasks 8.8 and 8.10

Issue #226, change `2026-09-09-226-session-resumption`. One row per acceptance
clause of whole-task **8.8** and whole-change **8.10**, against evidence this
ledger's author OPENED. It ends with the exact remaining work and three
one-line answers.

Task 8.8's acceptance lives in **three** places, and all three are graded here:

| Where | Lines | Rows |
|---|---|---|
| The numbered repair tasks **8.8.1.1–8.8.8.4** (R1–R4, the composite group) | 1232–1548 | `N1`–`N14`, with `N5` and `N6` split into six (§1) |
| Whole-task 8.8's own prose | 4735–5074 | `A1`–`A68` (§2) |
| The commissioned subgroups **8.8.9–8.8.15** (D7, Pass C) | 133–345 | `S1`–`S13` (§3) |

Whole-change **8.10** (5089–5452) is `B1`–`B104` (§4).

§1 is new in this revision. The first cut of this ledger omitted all fourteen
numbered tasks on the reading that they sit under a heading called
"Historical composite decisions and execution clauses". That heading does not
discharge them: 1153–1156 says those clauses are "retained under their original
owners" and that the current plan "does not reopen composite work **or claim its
pending evidence passed**", and 1209–1212 says "Broad task owners remain
unchecked for inherited pending evidence even after their current repair clauses
are delivered… **do not narrow full acceptance to earn a tick**". Eight of the
fourteen are unticked. They are part of 8.8's acceptance and they change the
answer to "can 8.8 be ticked".

**This ledger ticks nothing.** It edits no checkbox, no `tasks.md`, no
`design.md` and no spec. It is a reconciliation, not a delivery claim.

## The candidate it reconciles

Branch `slice-dsh-pass-d` at `d73d94d1`, PR #326. `git diff origin/main
--name-only` names exactly four files — `crates/brokkr-protocol/src/adapters/
composite/tests.rs`, `crates/brokkr-protocol/src/adapters/tests.rs`, this
change's `tasks.md` and this ledger — so every production byte on this branch is
`e50020ac`'s, which already carries PR #311 (8.8 a–c, 10.7) and PR #313 (8.8(d)
Passes B and C). Pass D's three runs added test evidence and delivery records
only, and this ledger and its correction add documentation only.

The commission reports PR #326 green. That report is context. Nothing below
rests on it: where a row says a suite passed, a named seat ran it on a named
revision, recorded below.

This file has now answered **five** returned reviews, and each was answered
by opening evidence rather than by trusting the finding.

*The first return* (nine findings against the first cut): §1 came into being,
eleven rows moved from `discharged` to `partially discharged`, one citation was
replaced, two clause-classes were found to have no evidence at any level, the
work list grew, and the first answer below changed from yes to no.

*The second return* (nine findings, C1–C9, against that revision) is answered
here. Every one was checked against the owning `tasks.md` clause and the actual
source before being accepted; **all nine held**, and three of them reversed
claims this ledger had made in its own voice. What changed:

| | Change | Finding |
|---|---|---|
| 1 | `N5` became four rows and `N6` two, because 8.8.3.1 and 8.8.4.1 each carry sub-clauses this ledger had collapsed — and the evidence for them was in the tree, uncited | C1 |
| 2 | `N7`–`N10` and `S3`–`S7` moved to `partially discharged`: they name removals in their own words, exactly as the rows already graded partial do | C2 |
| 3 | `A13`, `A64` and `B13` stopped citing partial rows as though complete | C2 |
| 4 | Unit 2a grew from one shape to **six**, and gained the offered-panel positive; `B36`'s positive matrix is now graded as the six cells it is | C3 |
| 5 | Unit 4a now commissions a **bound-route** execution reading the emitted launch row and journal — the surfaces the clause names — not only private-carrier keys | C4 |
| 6 | Unit 6 gained a precondition, unit 9 exists, and **the 8.10 answer is no longer an unconditional yes** | C5 |
| 7 | `B59` moved to partial: four version-output classes, two vectors | C6 |
| 8 | `N11`'s test **exists** and was wrongly reported absent; the blanket execution claim is withdrawn, because `doctor_dsh_selection.rs` was never executed by any recorded command | C7 |
| 9 | `F6` and `F7` were **factually wrong** about this change's own history: strict OpenSpec had passed twice before, and `coverage-exact.sh` did run and did **fail** | C8 |
| 10 | The unit count, unit 4/4a order and `B88`'s name count are corrected | C9 |

Rows 8 and 9 are the ones worth reading twice: both are cases of this ledger
asserting a *negative* — "no test exists", "the gate has never run" — without
opening the artefact that would have refuted it. That is the same failure F12
records against the first cut, committed again in the revision that recorded it.

*The third return* (the chief review of run
`issue-226-the-8-8-and-8-10-accep-b5a676bf` over `af89511a`, recorded at
`.forge/tasks/ledger-chief-review-3-b5a676bf.md`) left a medium residual. The
operator accepted it on 2026-09-23 and ruled that run
`issue-226-acceptance-ledger-reme-d530d5d2` remediates it. That run first
adopted `af89511a`'s text on `slice-dsh-8810` beside the record of every unit
that had landed there (1, 2a, 2b, 2c-fix, 2c, 3, 4a, 2d, 3b-fix, 3b), then
answered findings 1, 2, 3, 5 and 6. Finding 4 had already been folded into unit
4a's commission; its answer is 4a's own record, and the A53, B42 and B30 rows now
say which half 4a did not prove.

| | Change | Finding |
|---|---|---|
| 1 | The Apple and env source pins are **blocked retrieval**, not impossible. Every seat that tried was refused the network (`tasks.md` 2389–2394, 2425–2431). An externally owned retrieval-and-verification unit now exists (§6, unit 13), separate from any operator scope change | 1 |
| 2 | N6a and N6b move to partial: their removals are records. N2 and N4 are graded assertion by assertion. N11's positive **and** its two removals are graded as never performed, which is not the same as a record. Unit 17 now lists every recorded-removal row, and N11 is not one of them | 2 |
| 3 | 8.10 ticks only on a ruling that covers every removal predicate in its scope, B5's every-test clause, B16's four and B20's one included. After a negative ruling, units 18–21 enumerate the bounded replays | 3 |
| 4 | Four claims were false and are corrected. The R3 raw-span matrix is not exclusive to the doctor suite. `doctor_dsh_selection.rs` has historical recorded executions. `69cac25d` is D3's returned-review head, not Pass C's. N13 has a failed gate run, so it is not "nothing" | 5 |
| 5 | §1 is eighteen rows, not sixteen. Unit 17's list is counted. §6 is one numbered list in dependency order, and §7 gives the three one-line answers first | 6 |

*The fourth return* (the chief review of this run over `e84fe794` and
`b0d17e67`) confirmed the remediation and left one medium and three low
findings, all author correctness. This revision answers them. It still changes
no code, no test and no checkbox.

| | Change | Finding |
|---|---|---|
| 1 | B5's "every new test" had no fallback for THE PROOFS' tests (`tasks.md` 5095–5099). B5 now carries a test-by-test inventory of them. B4 now names the two reachable refusal tests the commission meant. Controls with **no record found** are a third class, which a ruling cannot accept. Unit 21 now has a half, 21(a), that performs them whatever unit 17 rules, so 8.10 waits on it in every case | C1 |
| 2 | F5's S rows are fifteen, not sixteen. N5d names six mutations (M1–M3, M7–M9), not five | C2 |
| 3 | B5 closes on unit 17 or units 18, 19 and 21(b), with 21(a) in both cases. It no longer says "18–21 all", because unit 20 is 8.8's alone | C3 |
| 4 | 9.6's detail counts two required rulings, 17 and 23. The scope change stays optional, as unit 23 and the 8.8 answer already say | C4 |

*The fifth return* (the chief review of this run over `339c93da`) left two
medium findings, both author correctness in B5's inventory. Both were
confirmed against the opened records and tests before being answered. Its
low finding I1 concerned the panel's inputs, not this ledger, and needs no
change here. This revision still changes no code, no test and no checkbox.

| | Change | Finding |
|---|---|---|
| 1 | D10's absence binding is **recorded**, not unrecorded. Task 10.5's M8/M8a/M9/M9a (`tasks.md` 10230–10233) observed it at `adapters/tests.rs::a_supported_assessment_without_both_affirmative_markers_declines`' decision arm. That record is historical, and neither the guard nor the test has changed since. It goes to unit 17, or to 21(b) after a no. The conformance cases carry known markers and cannot exercise it. What stays unrecorded is the unwrapped-shape predicate (9455–9456), and it is now in 21(a) | C1 |
| 2 | B5's inventory gains `4daaa7d2`'s other three new tests, the literal-owner, overlapping-drain and selected-single-marker cases. No control was recorded for any of them, so unit 21(a) performs all three, each against a named assertion | C2 |

## How to read a row

| Field | Meaning |
|---|---|
| **Row** | Stable identifier. `N…` = 8.8.1.1–8.8.8.4, `S…` = 8.8.9–8.8.15, `A…` = 8.8's whole-task prose, `B…` = 8.10. |
| **Clause** | Exact words, abbreviated with `…`. Where a single word decides the status it is quoted whole. |
| **Line** | `tasks.md` line or range. |
| **Evidence** | What was opened. `file:line` = source read. `suite::name` = a test whose BODY and assertions were read. |
| **Status** | `discharged`, `partially discharged` (with the gap), `not started`, `not this slice's` (with the owning text). |

**Evidence kinds are kept apart.** *Existence* (a test of that name is in the
tree), *assertion coverage* (its body asserts what the clause requires — read,
not inferred from its name), *recorded execution* (it ran, here, green) and
*recorded narrative* (a delivery section says a thing was done, and nothing in
the tree re-derives it). A delivery note, a checkbox or a green PR label
discharges nothing.

**The narrative rule, applied.** The first cut of this ledger stated that rule
and then broke it: rows whose clause *itself demands a mutation* — A61, S1, S10,
B5, B16, B20, B77 — were graded `discharged` on delivery sections that describe
compile-and-revert cycles leaving nothing in the tree. A recorded narrative is
not assertion coverage and is not recorded execution. Those rows now read
**partially discharged — evidence-verification gap**, naming exactly the
predicate that rests on narrative. This is a documentation correction: no clause
asks this commission to replay the mutations, and F5 below says what replaying
them would and would not add.

### Recorded execution

Dated, per-revision, and never merged into one claim. `936c04b6` and `d73d94d1`
differ by this ledger's own bytes alone, so the Rust results below carry to the
head; they are not restated as fresh runs on it.

| Command | Revision | Who | Result |
|---|---|---|---|
| `cargo fmt --all -- --check` | `936c04b6` | ledger seat | exit 0, clean |
| `cargo test -p brokkr-protocol --all-features --locked --lib` | `936c04b6` | ledger seat | **426 passed, 0 failed**, exit 0 |
| `cargo test -p brokkr-runtime --all-features --locked --lib` | `936c04b6` | ledger seat | **456 passed, 0 failed**, exit 0 |
| `cargo test -p brokkr-cli --all-features --locked --lib` | `936c04b6` | ledger seat | **467 passed, 0 failed**, exit 0 |
| `cargo test -p brokkr-cli --all-features --locked --test driver_conformance` | `936c04b6` | ledger seat | **24 passed, 0 failed**, exit 0 |
| `openspec validate --all --strict` | `936c04b6` | ledger seat | **NOT RUN** — refused by that seat's permission grant before execution. An unavailable tool is not a pass. |
| `bash scripts/coverage-exact.sh` | `936c04b6` | ledger seat | **NOT RUN** — same refusal, and by design external. |
| `openspec validate --all --strict` | `4d6b15f3` (D1 candidate) | D1 review seat | **exit 0 — 17 passed** (`.forge/results/8d9f4a9e-…`) |
| `bash scripts/coverage-exact.sh` | `4d6b15f3` | D1 review seat | **exit 1 — FAILED.** 32,626/32,802 lines, 5,494/5,508 branches, 3,181/3,191 functions (`.forge/results/8d9f4a9e-…`) |
| `cargo clippy --workspace --all-targets --all-features --locked` | `4d6b15f3` | D1 review seat | exit 0 |
| every gate of the D3 delivery, run in sequence: `cargo fmt`, workspace `clippy`, `cargo test -p` for all seven crates (`brokkr-cli` "467 lib and every integration binary"), `compile --bundle bundles/self` | the D3 candidate | D3 implement seat | PASS, by that seat's delivery record (`tasks.md` 1010–1018). `openspec` and the coverage script were refused (1020–1021); `bundles/verify` is not in the list |
| the same list, re-run | `1d1d9f17` | D3's returned-review seat | PASS, by its record (`tasks.md` 1093–1099); one `hands::` probe flake recorded as environmental |
| `openspec validate --all --strict` | `69cac25d` (**D3's returned-review head**, Pass D — the previous revisions said "Pass C head", which was wrong) | the review seat of that head | **exit 0 — 17/17** (`.forge/results/5c0333bc-…`) |
| `cargo clippy` (workspace, all targets/features, locked) + self-bundle compile | `69cac25d` (Pass D) | the same seat | exit 0 (`.forge/results/5c0333bc-…`) |
| `openspec validate --all --strict` | **`d73d94d1`** | **review seat** | **exit 0 — 17 passed, 0 failed** (informational notices) |
| `git diff --check 936c04b6..HEAD` | `d73d94d1` | review seat | clean |
| `openspec validate --all --strict` | `1e1c2f63` (the second revision) | its correctness reviewer | exit 0, as the third chief reports from the `004fcb8b` correctness result. That artefact is in the b5a676bf run's worktree, not this one, so this seat did not open it |
| `cargo fmt --all -- --check` | `1e1c2f63` + `af89511a` | the `af89511a` ledger seat | exit 0, clean |
| `git diff --check` | `af89511a` | the `af89511a` ledger seat | clean |
| `git diff --check 936c04b6..HEAD`, `cargo fmt --all -- --check`, `openspec validate --all --strict` | `af89511a` | the third chief | all passed; strict validation 17 items, 0 failures, by the chief's own record (`.forge/tasks/ledger-chief-review-3-b5a676bf.md:17`, opened here) |
| `cargo test --workspace`, `compile --bundle bundles/self` | heads on `slice-dsh-8810` after the units | the engine's per-visit checks | pass, e.g. `.forge/results/e8e0e1cf-…-checks.json`: 75 suite summaries, 0 failed. One earlier record, `394405f6-…-checks.json`, is a failure: the `hands::` network-prefix probe, the #255 shape. The records name no revision, so they are reports on this branch and are not tied to a head |
| `git diff --check`, `cargo fmt --all -- --check` | the remediation revision (this one) | the remediation seat | clean, exit 0 |
| `openspec validate --all --strict` | the remediation revision | the remediation seat | **NOT RUN.** This seat's permission grant refused it too. The report kept below is the `af89511a` seat's, unchanged |
| `openspec validate --all --strict` | `af89511a`'s own revision | the `af89511a` ledger seat | **NOT RUN** — refused by that seat's permission grant. `which openspec` resolves to `/home/vyanakiev/.volta/bin/openspec`, so the binary is present and the grant, not the tool, is the blocker. This is the **sixth** dated unavailable-tool report on this change, beside the D1/D2/D3 seats' (`tasks.md` 650–657, 833–848, 1020–1037), the Pass C seat's (507–511) and the previous ledger seat's. Per 8.8.14.2, "an unavailable tool is not a pass" — and per this revision's own F6, it is not a claim that the gate is unrun either: **three review seats have run it green**, most recently on `d73d94d1`, whose tree differs from this one by this file alone. *(Remediation note: the count was five by `af89511a`'s own chief — `4d6b15f3`, `69cac25d`, `d73d94d1`, `1e1c2f63` and `af89511a` — and the remediation seat's refusal is the seventh unavailable-tool report.)* |

**The blanket execution claim is withdrawn (C7).** The previous revision said
"every test cited below is in one of the four suites that ran green", and that
is **false**. The four ledger-seat commands above are three `--lib` runs and one
named integration target. `crates/brokkr-cli/tests/doctor_dsh_selection.rs` is
a *separate* integration target, one of thirty files in
`crates/brokkr-cli/tests/`, and none of those four commands built it.

**Its correction was itself too strong, and is corrected again (third return,
finding 5).** The previous revision went on to say "no recorded command in this
change ran it". That is false. The suite has **historical recorded executions**,
and each is a seat's report on its own revision:

- D3's delivery ran `brokkr-cli` with "467 lib and every integration binary"
  (`tasks.md` 1015–1016). Its returned review re-ran it on `1d1d9f17` (1098).
- The composite visits' gate rows name the suite and count it:
  "`doctor_dsh_selection` 14" passed at `tasks.md` 1940, 2076, 2377, 2596 and
  2921.
- The third chief reports that `.forge/results/183c4dda-…` records every CLI
  integration binary executing during D3. That file is not in this worktree.
- This branch's engine check records report workspace passes (the table above).

None of these reports is on the candidate that will be ticked, and this ledger
re-derives none of them. Rows N2, N4, N5b, N5c and N9 therefore carry
**existence, assertion coverage and historical recorded execution**, the last
attributed as above. Unit 14 is where they gain recorded execution **on the
final candidate**, because `cargo test -p brokkr-cli --all-features --locked`
builds every target in the crate.

So the rule is narrower than the old sentence. A `discharged` row citing
`composite/tests.rs`, `adapters/tests.rs`, `agents/tests.rs`, `doctor/tests.rs`,
`route_overlay.rs`, `engine/resume.rs`, `engine/resume_tests.rs`,
`engine/boundary_tests.rs`, `bundle/tests.rs` or `driver_conformance.rs`
carries all three evidence kinds on the ledger seat's revision. A row citing
`doctor_dsh_selection.rs` carries the third only as a historical report.

### Prior recorded validation of this change (C8)

The previous revision reported that `openspec validate --all --strict` had never
run on a candidate before the review seat's run, and that `coverage-exact.sh`
had never been run as a gate at all. **Both claims were wrong**, and the
artefacts that refute them were in this worktree the whole time. Opened here
(by the revision seats of run `b5a676bf`, in that run's worktree; the
remediation seat's worktree does not hold these files, so their rows below are
that seat's reading, kept as it was):

| Artefact | Revision | What it actually records |
|---|---|---|
| `.forge/results/8d9f4a9e-…-positions-correctness.json` | `e50020ac..4d6b15f3` (the **D1** candidate) | `cargo fmt`; workspace `clippy`; `cargo test -p brokkr-protocol` (412 unit, 99 integration, 1 doc; 2 native macOS probes ignored); **`openspec validate --all --strict` — 17 passed**; `git diff --check` |
| the same artefact, finding C2 | the same | **`bash scripts/coverage-exact.sh` was executed and exited 1** — lines 32,626/32,802, branches 5,494/5,508, functions 3,181/3,191. "All uncovered lines are in unchanged files; no `composite.rs` line is uncovered." |
| `.forge/results/5c0333bc-…-positions-correctness.json` | `69cac25d` (**D3's returned-review head**, Pass D. This row said "the Pass C head", and the third chief found that false against `.forge/ledger/dsh-pass-d-part-three-of-three-d-144c7c79.md`. `git log -1 69cac25d` reads "tasks: record the returned review and what answering it changed", a descendant of D1's `4d6b15f3` through D2 and D3. Pass C is `b0ec5517`, PR #313) | `git diff --check`; `cargo fmt`; workspace all-target/all-feature locked `clippy`; `cargo test -p brokkr-protocol` (426 unit, 99 integration, 1 doctest); **self-bundle compilation**; **`openspec validate --all --strict` — 17/17**; and it notes the verify artifact records `cargo test --workspace` passing |

Three consequences, all of which change rows below:

1. **Strict OpenSpec has passed three times on this change's own candidates** —
   `4d6b15f3`, `69cac25d`, `d73d94d1` — not once. F6 is rewritten. *(Third
   return: five, counting `1e1c2f63`'s correctness result and the third
   chief's run on `af89511a`, both as that chief reports them.)*
2. **An exact-coverage gate run exists and it FAILED**, on this change's D1
   candidate. The previous revision's "unmeasured by any gate run" was not a
   cautious statement, it was an incorrect one, and it understated the debt: the
   honest position is not "no result" but "the last actual gate run was red".
   F7 is rewritten.
3. **Clippy and the self-bundle compile have passing results on Pass D heads.**
   Clippy passed on `4d6b15f3` (D1) and clippy with the self bundle on
   `69cac25d` (D3's returned review). The D3 seat and its returned review each
   recorded clippy, all seven crate suites and the self bundle passing, in
   sequence (`tasks.md` 1010–1018, 1093–1099). This point first said "not on a
   Pass D head", which was false: both revisions are Pass D. These are real
   results on earlier revisions. Unit 14 must repeat them on the candidate that
   will be ticked.

What survives all of this: **no seat has run 8.8.14.2's or 8.8.8.2's
*complete* list in order on one candidate into a delivery record.** D3 came
closest. Its record runs everything up to the self bundle, in order, but
`openspec validate` was refused and `bundles/verify` is absent (1020–1021).
Those rows are open for the missing commands and for the ordering on one
candidate, so they stay open. Correcting the history does not close a gate. It
stops this ledger misreporting which gates have ever been green.

**What the review seat's `openspec` run does and does not close.** It is one
command of 8.8.14.2's ordered list and of 8.8.8.2's, run outside their order and
outside a delivery record, so it closes neither row. It is also not a result
this seat may represent as its own.

### Decision 0063 and Windows

Supported hosts are **Linux and macOS** (`docs/decisions/0063-windows-is-not-a-host.md`
ruling 1). Ruling 2: "No Windows effort. No new Windows-conditional code, test,
fixture, evidence or review obligation is written, owed or accepted." Ruling 3:
"Every pending native-Windows obligation in an open change is struck… A ledger
says *withdrawn by decision 0063*, never *pending*."

Ruling 5 retains host-agnostic validation of Windows-shaped **data**: "Host-
agnostic validation of *data* stays". Rows B24 and B26 below are that case and
are discharged on Linux; rows naming native Windows execution are marked
**withdrawn by decision 0063** and are neither debt nor executed proof.

Ruling 3 strikes issue #226's obligations **by name**: "the Windows port of the
lookup oracle, `GetBinaryTypeW` evidence, the native Windows matrix and
MSRV-on-Windows execution".

**The complete list of withdrawn obligations in 8.8 and 8.10**, so no later
reader restores one: B26's "native Windows proof"; and N4's (8.8.2.2) "Native
Windows matrix, doctor/GetBinaryTypeW and Windows MSRV" — all three of ruling 3's
named items land on that one row, whose own text already records them as
withdrawn at 1299–1300, and the lookup-oracle port is the Windows half of the
`native_executable_resolution_matches_command_matrix` that same row preserves.
Nothing else in either task's acceptance asks for Windows evidence. `crates/brokkr-cli/tests/
doctor_dsh_selection_windows.rs` exists in the tree as pre-0063 code; ruling 2
forbids *new* Windows-conditional work, and 0063 leaves existing such code alone
until it is touched, so its presence is neither a debt this ledger tracks nor a
clause it discharges.

---

## 1. Task 8.8's numbered tasks, 8.8.1.1–8.8.8.4 (1232–1548)

Fourteen numbered tasks, the R1–R4 composite repair group. Six are `[x]`,
**eight are `[ ]`**. They are graded in **eighteen rows** (twelve single rows
plus `N5a`–`N5d` and `N6a`/`N6b`; the previous revision said sixteen, a
miscount): 8.8.3.1 carries four
numbered sub-clauses and 8.8.4.1 two extension paragraphs added on return, so
`N5` became `N5a`–`N5d` and `N6` became `N6a`/`N6b` (C1). A single row could not
hold them without hiding which half had evidence — and in 8.8.3.1's case the
evidence for three of the four sub-clauses was in the tree, uncited.
Their own execution-order table is at 1214–1228 and binds each to an
AS1 scenario. This slice commissioned none of them (A55: "Current AU/D7 repair
clauses 8.8.9.1–8.8.15.1 alone are commissioned here") — but 8.8 cannot be
ticked over an open one, so each is graded, and each row says who owns it.

Where a row reads *partially discharged — external*, the **code is in the tree
and opened here**. What stays open is an obligation the task's own words assign
to evidence that no seat of this change has been able to reach: immutable
Apple/env source pins and native macOS platform runs. That is **blocked
retrieval, not impossibility**. The pins are public upstream source. Every seat
that tried to fetch them was refused the network: `curl`, `WebFetch`, `gh api`
and the GitHub MCP (`tasks.md` 2389–2394, 2425–2431). `composite.rs:2628–2631`
says the same of its own port: "this seat could reach neither the source nor a
Darwin host". A seat or host whose grant reaches the network can retrieve them,
and §6 unit 13 is that work. Native macOS runs likewise need a host, not a
waiver (unit 15).

| Row | Task | Box | Clause | Line | Evidence opened | Status |
|---|---|---|---|---|---|---|
| N1 | 8.8.1.1 | `[ ]` | keep "candidate spelling and native argv[0] beside canonical file identity"; `classify_in`/`selected_from` changed so doctor "launches the already-selected invocation"; searched `dsh -> /usr/bin/env` and an absolute alias "refuse before either probe". Closing words: "Immutable Apple source pins and native platform evidence remain this owner's pending inherited acceptance, outside the repair" | 1232–1252 | `composite.rs:2217–2267` — `DshInvocation` is a distinct type carrying the invocation "EXECUTION: a launcher reads the path it was run by"; `:1815–1850` `DshPrepared` holds `invocation` beside the admitted home; `:1956` `selected_from`; `:3276` `classify_in`, whose refusals at `:3327` and `:3330` are spelled "the selected invocation {why}" | **partially discharged — external.** The R2 carriage is implemented and opened. The row itself calls the Apple source pins and native platform evidence (1248–1250) "pending inherited acceptance". The pins are **blocked retrieval**: seats were refused the network (2389–2394). Unit 13 retrieves and verifies them, and unit 15 supplies the native macOS evidence. "Adopt resolver removals without replay" (1248) covers its removals |
| N2 | 8.8.1.2 | `[ ]` | extend protocol selection tests, doctor unit seams and `crates/brokkr-cli/tests/doctor_dsh_selection.rs` with "both R2 alias forms and direct env"; "Independently remove selected-env qualification, then independently restore canonical execution"; "native macOS remains pending, not newly commissioned" | 1253–1271 | Opened assertion by assertion (third return, finding 2). **Built doctor**, `doctor_dsh_selection.rs::a_dsh_alias_of_env_is_refused_and_an_admitted_alias_runs_as_selected` (`:2771–2895`): *direct-env control*, where native `/usr/bin/env --version` is read (`:2788–2798`) and doctor's line is asserted to be that native result. Where env answers, that is `ok … {env_version} · serves` plus `· composite unreadable:` (`:2865–2869`), so the line is **never a readable composite**. Where it does not, the line is the not-found warning (`:2871–2874`). *Both alias forms*, `searched` and `absolute` (`:2810–2850`): zero doctor markers (`:2837`), the line opens with the exact selection cause, "the selected invocation is the platform's env utility invoked under the name 'dsh'…" (`:2804–2808`, `:2838–2843`), and env's version is absent from it (`:2844–2849`). Each native alias result is **recorded, never counted** (`eprintln!` at `:2822–2826`; the uutils result "exit 1… empty stdout" is recorded at `tasks.md` 1650–1651). *No-probe*: the markers are zero, but a silent env leaves none, so the no-probe half rests, as the case says (`:2760–2765`), on `doctor/tests.rs::a_failed_selection_probes_nothing_and_carries_its_cause` (`:2765`, a probe closure that panics, `:2769`). *Admitted launcher control* (`:2877–2894`): the launcher prints `$0`, native runs it by the alias (`:2882–2886`), and doctor's line opens with the alias path, not the launcher's (`:2890–2894`). **Protocol**, `composite/tests.rs::the_selected_invocation_is_not_replaced_by_its_canonical_target` (`:2940`): both alias forms refuse `select_in` and `selected_from` with the same cause (`:2974–2988`). Direct env is selected as its canonical file and invoked as spelled, with status, stdout and stderr equal to native's (`:2999–3016`). The admitted launcher's `path` is the canonical target (`:3026`) and its invocation is `{program: alias, argv0: "dsh"}` (`:3027–3033`), which gives **separate canonical identity** and the retained spelling. **Removals**: M5 "selected-env qualification removed" and M6 "canonical path substituted as invocation" (`tasks.md` 1669–1670) | **partially discharged — external, with an evidence-verification gap and one unasserted predicate.** Assertion coverage holds for direct env, both alias forms, the cause, zero markers and the retained invocation. The cases also have historical recorded execution (see *Recorded execution*). **Recorded but unverified:** the two independent removals (M5, M6) are delivery records (F5), so unit 17 rules them. **Unasserted:** "no second search". The invocation holds the absolute candidate the search found (`:3029–3032`), so the launch should not search again, but no case runs it with the search taken away. Unit 12 adds that. **Never performed:** native macOS, pending by the row's own words (1267–1268). Unit 15 |
| N3 | 8.8.2.1 | `[ ]` | change `env_program`'s "successful absence for an empty or ASCII-space/tab-only argument tail to a cause-bearing refusal"; flip the bare `#!/usr/bin/env` row in `the_candidate_classifier_stops_where_the_child_stops_and_refuses_the_unprovable`. Closing words: "Immutable env source pins and missing native platform proof remain inherited debts" | 1273–1287 | `composite.rs:3675–3685` — a blank-tail scan, then `Err("is the platform's env utility given no nonblank program, so the program it would run is the launcher itself")`, with the comment citing run `124cca78`, R4; the same sentence is asserted at `composite/tests.rs:5650` | **partially discharged — external.** The R4 refusal is implemented and its exact cause is asserted. The env source pins and native platform proof at 1284–1286 are the row's own "inherited debts". The pins are **blocked retrieval**, not impossible. The env dispatch rule cites GNU `src/env.c`, Apple `usr.bin/env/env.c`, uutils `src/bin/coreutils.rs`, busybox `libbb/appletlib.c` and GNU `src/coreutils.c` by name, but pins none of them (`composite.rs:3598–3608`, beside the kernel's `fs/binfmt_script.c` for `argv[0]`). Unit 13 retrieves and pins them, and unit 15 supplies native macOS |
| N4 | 8.8.2.2 | `[ ]` | test bare and blank env shebangs "through protocol selection/producer, injected doctor probes and the built doctor"; "Native bare/blank reproductions use separate markers and an external process-group timeout"; "**Native Windows matrix, doctor/GetBinaryTypeW and Windows MSRV are withdrawn by decision 0063.** Native macOS matrix and applicable source-pin cells retain their original pending acceptance" | 1288–1308 | Opened assertion by assertion (third return, finding 2). **Protocol selection**, `composite/tests.rs:5655–5689`: five bare, unterminated and blank env bodies each refuse `select_in` with the exact missing-program cause, naming the launcher, the env interpreter and "no nonblank program" (`:5670–5680`). They also refuse through `DshSeams::selected_from`, so no selection exists and nothing can be probed (`:5683–5689`). **Built doctor**, `doctor_dsh_selection.rs::an_env_launcher_without_a_program_is_refused_before_any_probe` (`:2909–3002`): the *bounded native* reproductions of `bare` and `blank` run under their own `oracle-{tag}` markers and a 2-second external process-group deadline, with kill and reap (`:2916`, `:2926–2943`), and their outcome is **recorded, never asserted** (`eprintln!`, `:2937–2943`). Doctor runs under its own `doctor-{tag}` markers and a 120-second bound that **fails** on expiry ("doctor did not return: it started the loop", `:2946–2955`). It leaves zero markers (`:2956`) and names the exact cause (`:2958–2966`). The *`env sh` control* terminates natively with `v9.9.9-sh` (`:2973–2984`), and doctor probes it once, with markers `["dsh"]` and an `ok` line (`:2985–3000`). **Injected doctor probes:** the failed-selection seam is proved in general by `doctor/tests.rs::a_failed_selection_probes_nothing_and_carries_its_cause` (`:2765`, panicking probe), but over an absent-`PATH` cause (`:2773`). No doctor unit case injects the bare or blank env selection. **Removal:** M4, "blank tail `Ok(None)` restored → classifier: 'expected a refusal' for the bare-env row; nothing spawned" (`tasks.md` 1668) | **partially discharged, with an evidence-verification gap.** The Windows half is **withdrawn by decision 0063**; the row already says so in its own words. Assertion coverage holds for protocol selection, the built doctor, the bounded native evidence and the `env sh` control, and the cases have historical recorded execution. **Unasserted:** the injected-doctor-probe cell for the missing-program cause. Unit 12 adds it. **Recorded but unverified:** M4 (F5), unit 17. M4 records only the selection test failing. The clause also names the "callback tests", and no record opened here shows a callback test failing under that mutation. **Never performed:** the native macOS matrix and the applicable source-pin cells (1304–1305). Units 15 and 13 |
| N5a | 8.8.3.1(1) | `[x]` | **Shared preparation.** Prepare profile and bounded pnpm observation "before authority for either DSH or Node version probing"; retain declarations, patchReload, raw anchor, canonical boundary and dependencies "in a small privately constructed value"; "Composition consumes them once, without profile/lock rereads"; "Preparation computes no digest"; a located pnpm admission failure "blocks both probes" | 1314–1329 | `composite/tests.rs:3074` `::the_pnpm_lock_is_admitted_before_any_probe_and_composed_as_retained` — `DshPrepared::admit` returns the retained value, `prepared.invocation()` and `prepared.seams()` are asserted, its `Debug` carries the admitted dependency, and the test's own doc records the repair: "The lock was parsed last — after doctor's DSH probe and the producer's Node probe… Admission is now the selection's, and the producer's own first step" | discharged |
| N5b | 8.8.3.1(2) | `[x]` | **R1 separation.** At `split_flow_entry` consume "the entire admitted ASCII-space separator run before `flow_scalar` inspects opening syntax"; test `{node:  *missing}`, `{node:  &}`, `{node:  %bad}` "with one, two and additional separator spaces" through the sole producer and built doctor; "One-space and padded numeric controls (`22`) and valid quoted controls remain readable with the same control composite" | 1331–1345 | Producer: `composite/tests.rs:3128–3130` drives `engines: {node:  *missing}` to the exact member cause naming the `'*'` indicator; `::missing_pnpm_field_separation_and_unsupported_flow_syntax_refuse_by_reason` (`:4155`) holds the padded controls at `:4497–4499` — `{node:    22}`, `{node:  '>=18',   npm:     "9"}`, `{node:   '  *kept  '}` — each readable. Built doctor: `doctor_dsh_selection.rs::ignored_pnpm_values_are_admitted_as_syntax_through_the_built_doctor` (`:2256`) drives `{node:      %bad}` to its named cause | discharged for assertion coverage. The built-doctor half has **historical recorded execution only**, reported by earlier seats and never on the final candidate (see *Recorded execution*; unit 14). An earlier revision called that suite "unrun", which was false |
| N5c | 8.8.3.1(3) | `[x]` | **R3 raw span.** Bound the original implicit block-key slice before trimming; "Count Unicode characters including quotes and pre-colon spaces"; maximum 1,024; cover "1,024/1,025 ASCII characters; 1,023/1,024 plus one space; and quoted keys with 1,022/1,023 content characters"; "Include otherwise admitted multibyte keys to prove character rather than byte counting, and long scalar values to rule out a blanket line/value cap" | 1346–1359 | `doctor_dsh_selection.rs` — the refusal side at `:2545–2560` (`"k".repeat(1025)` plain, then `.repeat(1024)` plus a pre-colon space, each to `IMPLICIT_KEY_REFUSAL`, whose text at `:2241–2243` names "YAML's implicit-key lookahead limit of 1,024 characters"); the **admitted** side at `:2660–2673`, which is the whole matrix in one list: `repeat(1024)` plain, `repeat(1023)` plus a space, `'{repeat(1022)}'` quoted, **`"\u{e9}".repeat(1024)`** (the multibyte character-not-byte control), a long scalar value, and a 1,024-key *with* a long quoted value — each asserted to keep the control composite. **Producer:** `composite/tests.rs::missing_pnpm_field_separation_and_unsupported_flow_syntax_refuse_by_reason`, `:4553–4619`, carries the same matrix through the sole producer. It has fifteen cells: 1,024/1,025 plain; 1,023/1,024 plus one space; a padded-over-limit key; single- and double-quoted 1,022/1,023; multibyte `é` at 1,024/1,025, plus one space, and quoted. Each admitted cell is asserted equal to the body control's composite, and each refused cell to the exact lookahead-limit reason (`:4588–4601`). Three 4,096-byte long values follow, each keeping the control composite (`:4603–4619`) | discharged for assertion coverage at **both** levels. The matrix is complete and was opened cell by cell. **Correction (third return, finding 5):** the previous revisions said the doctor suite "is the only place the raw-span matrix lives". That was false: the producer suite holds it with real equality and refusal assertions, and ran on the ledger seat's revision (`--lib`, 426 passed). The built-doctor half has historical recorded execution only (see *Recorded execution*) |
| N5d | 8.8.3.1(4) | `[x]` | **Independent proof.** "Restore one-space consumption for R1; remove the raw-span guard for R3; separately move R3's guard after trimming"; "Separately bypass admission before DSH and before Node"; "Separately reopen retained pnpm/profile input after a version probe rewrites it: the retain-and-reuse assertion fails"; "Record each guard/wiring path and focused command, restore one mutation at a time" | 1360–1371 | Nothing in the tree. Six distinct compiling mutations are named: one-space consumption, the removed raw-span guard, the guard moved after trimming, admission bypassed before DSH, admission bypassed before Node, and reopened retained input. The R1–R4 record gives them as M1–M3 and M7–M9 (`tasks.md` 1665–1673). None leaves an artefact. *(Corrected, fourth return: this cell said "Five", which undercounted the clause's two admission bypasses as one)* | **partially discharged — evidence-verification gap (F5).** The guards these mutations target are opened at N5a–N5c and pass. The six removals are recorded, not re-derived |
| N6a | 8.8.4.1 | `[x]` | track admitted decoded package headings separately; reject repetitions and conflicting records "with `repeated package key` and the decoded key"; preserve legitimate equal-triple deduplication | 1376–1390 | `composite.rs:1573` emits `a repeated package key '{key}'`; `composite/tests.rs` 5394, 5408, 5414 assert that exact sentence with the decoded key, including the excluded local-tarball key; dedup retention is `::npm_three_group_and_dedup_vectors_retain_distinct_triples`. **Removal** (third return, finding 2): the clause goes on, "Remove the heading-set rejection and observe those named assertions fail; restore it and rerun positive and negative controls" (1383–1386). The one record of it is M8 at `tasks.md` 12377: "`pnpm_dependencies`: `seen_packages.insert` no longer refuses a repeat" parts `duplicate_decoded_pnpm_package_keys_refuse_before_triple_normalization` (`composite/tests.rs:5337`) with "`identical` was accepted" | **partially discharged — evidence-verification gap.** The rejection, its exact sentence, the decoded key and the dedup retention are opened and asserted. The removal and restoration are **recorded but unverified** (F5), so unit 17 rules them. The previous revisions graded this row `discharged` over a clause whose last two sentences are a removal, which is the grading error F5 exists to prevent |
| N6b | 8.8.4.1, both returns | `[x]` | the two extensions the row acquired on return: "the same singleton rule holds in **every mapping scope** the grammar admits without reading — a flow map, an ignored block body at every depth, a package child spelled twice — **by DECODED key and per block**, so one key in two sibling blocks stays two keys"; then "keys are compared as YAML compares them — the padding before a plain key's colon is the separator's, and **a plain key spelling a typed scalar is refused by its cause before any comparison**, in every admitted scope" | 1391–1401 | All of it in `composite/tests.rs::missing_pnpm_field_separation_and_unsupported_flow_syntax_refuse_by_reason` (`:4155–5052`), which is where this ledger had not looked. **Per scope:** `a repeated package child 'cpu'` (`:4397`) and `'peerDependencies'` (`:4401`); a repeat inside an ignored block body, "the entry 'react' at 6 spaces, which repeats a key of its block" (`:4413–4415`). **Padding is the separator's:** that same vector spells the repeat `react : '>=17'`. **Typed scalars refused by cause:** four vectors at `:4419–4439` — `'11', which is a number and not a string`, and `'true', which is a boolean and not a string` — matching `composite.rs:750–757` `mapping_key`, whose doc records the reason ("This grammar resolves no scalar type"); `::pnpm_identity_strings_preserve_the_distinction_from_typed_scalars` (`:5127`) asserts the same shape at `:5184`. **Keys YAML keeps apart:** `:4450–4456` — plain `react` beside quoted `'react '`, and the quoted typed scalars `{'true': a, 'True': b}` and `{'11': 1, '0xB': 2}`, which are strings and therefore two keys each. **Sibling blocks stay two keys:** `:4656` composes a two-importer, two-record lock and asserts `two_records.canonical` equals a lock with no importers at all — "the sibling blocks are admitted and ignored" | **partially discharged — evidence-verification gap.** Assertion coverage is complete, on evidence the first revision did not cite and the second nearly recorded as absent. But the clause's own words make the removals part of it: "Each scope's guard is **proved by its own removal** (D1–D3)… proved by removals D4–D5" (1395–1401). Those five are **recorded but unverified**: D1–D3 at `tasks.md` 1920–1922, D4–D5 at 2057–2058. Each names the protocol and built-doctor assertion that parted (F5), so unit 17 rules them. The previous revision graded this row `discharged` while saying in the same cell that the removals were narrative. That was incoherent, and the grade has moved |
| N7 | 8.8.5.1 | `[x]` | replace the loop/hash oracle in `the_plugin_component_is_bytewise_path_order_and_fails_closed` "with a literal recorded from the existing sole production producer"; "no prose/helper/generator computes another component or canonical serialization" | 1405–1417 | That test and `::the_worked_plugin_vector_pins_the_bytewise_path_order_of_the_component`, both opened at A18/B90; `::no_test_reassembles_the_component_stream` is the guard | **partially discharged — evidence-verification gap.** The literal and the guard are opened. Its "alter production path/line ordering in a compiling mutation" half is recorded, not re-derived (F5) |
| N8 | 8.8.5.2 | `[x]` | "a focused source-conformance assertion… detects restoration of the known competing serialization and concatenation-hash block" | 1418–1432 | `composite/tests.rs::no_test_reassembles_the_component_stream` (A23, A29) | **partially discharged — evidence-verification gap.** The source-conformance assertion is opened. Its restoration mutation is recorded, not re-derived (F5) |
| N9 | 8.8.6.1 | `[x]` | apply `Safe` at final rendering of the unavailable-binary and retained-selection cause in `doctor.rs`; a built-doctor test with "a newline and ANSI clear-screen sequence"; "assert the recognizable escaped spelling… with no raw injected sequence" | 1433–1447 | `doctor_dsh_selection.rs:1999–2000` — "S2. A nonexistent override carrying a newline and an ANSI clear-screen sequence reaches stdout ESCAPED"; the unit-side sibling at `doctor/tests.rs:3017` | **partially discharged — evidence-verification gap.** The built-doctor assertion is opened. It lives in `doctor_dsh_selection.rs`, which has historical recorded execution only, never on the final candidate (see *Recorded execution*; an earlier revision said "no recorded command ran" it, which was false). The unit-side sibling at `doctor/tests.rs:3017` ran on the ledger seat's revision. Its "remove safe rendering in a compiling control" half is recorded, not re-derived (F5) |
| N10 | 8.8.7.1 | `[x]` | enrich exhausted `resolve_bundle` with `bundle '…' does not resolve: no package.json found`; "Preserve true-absence continuation to a legitimate later hit; unreadable/canonicalization-error/outside first hits still stop" | 1448–1464 | `composite/tests.rs::removing_only_the_plugin_manifest_names_the_drifted_file` (A18); `::a_bundle_candidate_that_cannot_be_inspected_stops_the_search` and `::an_outside_first_bundle_hit_is_not_skipped_for_a_later_inside_one` (B93) | **partially discharged — evidence-verification gap.** The cause and the continuation rule are opened. Its filename-context mutation is recorded, not re-derived (F5) |
| N11 | 8.8.8.1 | `[ ]` | consolidate R1–R4 tests and removal records from 8.8.1–8.8.3; "Verify all four built-doctor reproductions now end in named refusals"; "**The actual absent-PATH retained-Node positive and its two removals remain pending under this address until their native prerequisite exists; shell failure or NotFound is no positive**" | 1465–1483 | `composite/tests.rs::an_absent_path_is_a_named_refusal_and_never_the_working_directory` and `::the_default_search_path_is_the_c_librarys_own_answer` (A24) are the refusal side. **The positive exists**: `::absent_path_node_identity_is_retained_by_the_composite` at `composite/tests.rs:3489`, opened here in full. It runs `node -p process.execPath` with `PATH` removed and branches on the host. Its `Ok` arm (`:3524–3568`) is the real positive — the retained `node` is the runtime the native default search ran, `composite.node` is that runtime's own `--version` output, and a *different* runtime retained in its place moves the canonical digest (`assert_ne!` at `:3567`). Its `Err` arm (`:3501–3522`) asserts the named refusal instead and prints `PENDING: no node on this host's default search path` | **partially discharged — external; the positive and both removals were never performed.** Correction: the first revision said "no test in the tree is the absent-PATH retained-Node positive". That was **wrong**, because the test exists and its `Ok` arm asserts what the clause requires. *(Line references in this row are `af89511a`'s. Unit 1 has since moved the test to `composite/tests.rs:3511`, its `Err` arm to `:3523–3545` and its `Ok` arm to `:3546–`.)* **Third return, finding 2: the removals are not records.** The previous revision filed them as "narrative (F5)" and listed N11 for the unit-9 ruling (now unit 17). Opened, they are not narrative. The test's own doc says the positive "with its two removal controls is recorded PENDING, never a passing skip" (`:3503–3505`), and its `Err` arm prints "the absent-PATH retained-Node positive and its two removal controls were not established here" (`:3541–3544`). The only execution account opened says the same: both absent-PATH tests "printed `PENDING: no node on this host's default search path /bin:/usr/bin`" (`tasks.md` 2395–2399). The clause keeps the positive "and its two removals… pending under this address until their native prerequisite exists; shell failure or NotFound is no positive" (1477–1480). So: the positive is written and has **never run its `Ok` arm on any recorded host**; the two removals (unconditional absent-PATH refusal restored, and a distinct wrong Node retained, `:3505–3508`) have **never been performed**, because they need that arm. A ruling can accept a record; there is no record here to accept. Unit 16 performs all three on a capable host. The consolidation record the clause also asks for belongs to unit 23 |
| N12 | 8.8.8.2 | `[ ]` | "On the restored candidate run `cargo fmt --all -- --check` and `cargo clippy…`. Run `cargo test -p <crate>… sequentially, in order`" for all seven crates; plus both `compile --bundle` runs and `openspec validate --all --strict`; "Unavailable tools or failures leave this row pending" | 1484–1503 | The recorded-execution table above. The ledger seats ran fmt on both revisions and four of seven crate suites on `936c04b6`. The D3 seat and its returned review each report fmt, clippy, all seven crate suites **sequentially** and the self bundle passing on their Pass D heads (`tasks.md` 1010–1018, 1093–1099), but `openspec` was refused there (1020–1021) and `bundles/verify` is absent. `openspec` itself passed five times under review and chief grants, outside the list (see F6). The previous revision said "clippy, three crate suites and both bundles unrun by any seat on a Pass D candidate", which was false | **open as the complete ordered list.** No single candidate has every command of the list recorded green in order: `openspec validate` and `bundles/verify` are missing from the one sequential record. The clause's last sentence keeps it pending: "Unavailable tools or failures leave this row pending". It duplicates S11's gate list, and one execution closes both. Unit 14 |
| N13 | 8.8.8.3 | `[ ]` | "Collect fresh coverage on the committed restored candidate"; the controller "runs unchanged `TMPDIR=/tmp bash scripts/coverage-exact.sh` on a capable host/CI"; "Require nonzero exact equality for **source lines, branches and functions**"; "Keep this row and final-head remote results pending until actual evidence exists" | 1504–1526 | **One literal gate run, and it failed.** The D1 review seat ran `bash scripts/coverage-exact.sh` on `4d6b15f3` and it exited 1: lines 32,626/32,802, branches 5,494/5,508, functions 3,181/3,191 (`.forge/results/8d9f4a9e-…`, opened by the `af89511a` seat, F7). D2 and D3 also reproduced the script's substance by hand (F7), and the row's own text forbids reading that as the gate: "The box cannot execute namespace boundary tests; this is preparation, not a literal gate pass". The previous revisions put "Nothing" here and graded the row "not started", which F7 itself contradicted | **open — externally owned, from a red result.** The last literal run on this change failed, so the obligation is to clear a known failure on the final candidate, not to start from nothing. This, not 8.8.14.3, is where the coverage obligation lives. Unit 15 |
| N14 | 8.8.8.4 | `[ ]` | reconcile R1–R4 delivery; "Tick a task only when its **entire** acceptance is met; broad owners with inherited pending predicates stay open"; "Preserve change-wide states, unchecked 8.8 and proposed 0056"; commit, never push | 1528–1548 | Not begun: 8.8 is `[ ]` at 4735 and 0056 is `proposed`, which is this row's *preservation* requirement, not its delivery | **not started.** It is also the clause that decides N1–N4 and N11: a row whose inherited predicate is pending "stays open with current repair delivery recorded in prose". Unit 23 |

**What §1 changes.** Eight numbered tasks are open. Five of them (N1–N4, N11)
are open on evidence that **no seat of this change has been granted the reach
to produce**: Apple/env immutable source pins, native macOS runs and the
absent-PATH retained-Node positive. Their own rows declare them inherited debt.
None of it is impossible. The pins are blocked retrieval (unit 13), and the
macOS legs and N11's positive need a host (units 15 and 16). N11's positive is
written, compiled and waiting, but its two removals have never been performed
and have no record for a ruling to accept (unit 16). N5b, N5c and N9 need
`doctor_dsh_selection.rs` run **on the final candidate**, because it has only
historical recorded execution (unit 14). N2 and N4 each keep one small
unasserted cell (unit 12). N12 and N13 are gate executions, and N13's last
literal run was red. N14 is the reconciliation that records all of it (unit
23). Among the six ticked rows, N5d, N6a, N6b and N7–N10 rest their removals
on records, so unit 17 rules them. None of this can be closed by editing this
ledger. 8.8 cannot be ticked while any of it is open, unless the operator rules
that N1–N4 and N11 may be ticked on their delivered-repair half with their
inherited predicates recorded, and 1209–1212 forbids a seat from deciding that
for itself. That ruling would be a **scope change**, not satisfaction of the
acceptance, and unit 23 keeps it separate.

## 2. Task 8.8 — whole-task prose (4735–5074)

| Row | Clause | Line | Evidence opened | Status |
|---|---|---|---|---|
| A1 | "Execute local groups 8.8.9–8.8.15 above" | 4736 | Rows S1–S13 below | partially discharged — S11 (8.8.14.2) and S12 (8.8.15.1) remain open. This clause names groups 9–15 only; it does not narrow 8.8's acceptance to them, and §1's fourteen numbered tasks stay in force (1209–1212) |
| A2 | "adopt the prior composite, Pass B and Pass C commits and proofs" | 4736–4737 | `git log origin/main..HEAD` (17 Pass D commits, no revert); `e78c1da1` (#311) and `b0ec5517` (#313) are ancestors of `HEAD`; `git diff origin/main --name-only` = 3 files | discharged |
| A3 | "Historical closure claims neither discharge these three findings nor erase inherited debt" | 4737–4739 | R1–R3 are proved independently at A61/S6–S8, not by any prior claim | discharged (reading rule; honoured) |
| A4 | "it does not commission live qualification, Pass D or enablement here" | 4739–4740 | The branch touches no `adapters/dsh.json`, no `extensions/`, no decision; 11.3 stays unticked (5956) | discharged (scope rule) |
| A5 | "This whole-task checkbox remains open" | 4741 | `tasks.md:4735` reads `- [ ] 8.8` | discharged — and unchanged by this ledger |
| A6 | "After 1.1, 6.4, 11.5 and 13.1 establish the corrected disabled truth and 10.7's live half commits the adaptation" | 4743–4744 | **The states themselves, not the boxes.** The corrected disabled truth: `adapters/dsh.json:59–65` declares `"resume": { "headless-work": { "status": "unmeasured"` under identity `0.1.5-rc.1` — the route is measured-as-not-enabled, which is the truth those four tasks were to establish. 10.7's committed adaptation: the six files and `PROVENANCE.md` opened at A7. The ticks at 4072, 4505, 6008, 6075 and 5732 are recorded beside this, not relied on | discharged — on the opened states. The first cut cited five checkboxes, which by this ledger's own rule discharge nothing |
| A7 | the forward-pinned core `@deepseek-ai/dsh` 0.1.5-rc.2 with integrity `sha512-8Xc8…`, the six-file adaptation of plugin 0.2.0 at `0f487e74…` "committed as bytes under `extensions/dsh/plugin-cli-session/`" with the sibling `PROVENANCE.md` | 4744–4756 | `extensions/dsh/plugin-cli-session/` holds exactly `package.json`, `lib/index.js`, `lib/startup.js`, `cordis.patch.yml`, `README.md`, `LICENSE`; `extensions/dsh/PROVENANCE.md` beside it; `.forge/tasks/dsh-pair-qualification-015rc2.json` present | discharged |
| A8 | "changes exactly one expression, `lib/index.js` line 253… every other byte is upstream-identical" | 4756–4760 | `lib/index.js:253` reads `\tconst events = agent.session.snapshotEvents(firstSeq);`; `composite/tests.rs::the_committed_plugin_set_is_the_six_files_and_the_one_expression_delta` asserts one occurrence and that substituting the upstream expression back hashes to `a40b52b3…`; PROVENANCE's upstream and adapted per-file blocks differ in `lib/index.js` alone | discharged |
| A9 | "adopts these bytes, the never-publish note and the settled second-selection reader seam unchanged" | 4761–4765 | `git diff origin/main --name-only` names no `extensions/` path; the second selection is `adapters.rs:3760` (`select_file` re-selected before the boundary read) | discharged |
| A10 | "use the plugin's explicit `--new --output-format stream-json` cold form and `--session <owned-id> --output-format stream-json` warm form" | 4766–4769 | `adapters.rs:3705–3720`; `adapters/tests.rs::a_qualified_dsh_launch_uses_the_stream_json_forms_and_records_observed_identity` (cold: `--new`, `--output-format stream-json`, no `--session`), `::a_warm_dsh_offer_names_the_owned_root_and_folds_past_its_sequence` | discharged |
| A11 | carry `transcript.home` "from the same confirmed checkpoint as the root and locator" as `owned_target.persistence_home` "at both existing callers in `engine.rs`" | 4769–4775 | `engine/resume.rs::the_private_context_carries_the_owned_target_and_originating_digest` (five coordinates); `engine/resume_tests.rs::an_offered_dsh_start_carries_the_recorded_home_at_the_single_site` and `::…_at_the_panel_member` read the home off the `Start.input` the driver actually received | discharged |
| A12 | "Never borrow a missing field from another checkpoint, site or older owner; missing evidence stays missing" | 4776–4777 | `resume_tests.rs::a_stamped_row_is_offered_only_to_its_own_site_owner_and_persistent_root` 1272–1372: a newer row without a transcript carries `persistence_home: None`; distinct old/new coordinates; a mistyped newest version/digest reads `None`, never the older row's | discharged |
| A13 | "Verify the runtime unit/integration cases assigned in 8.10 after repairing R2's serialized checkpoint transport" | 4777–4780 | Rows B23–B28 | **partially discharged.** It can be no stronger than the rows it points at, and **B27 is partial** (its mistyped home and locator are undriven) and **B30 is partial** (launch-evidence exclusion and the retained-fields predicate). B23–B26 and B28–B29 are discharged. Units 6 and 7 (recorded as 3 and 4a) have landed. B27 keeps its pre-unit grade until unit 22 regrades it on opened evidence. B30 keeps the half unit 7 did not prove, which is unit 11's |
| A14 | "Leave `Body::Resume` and driver protocol v1 unchanged, and publish `root_session` plus the complete `transcript` locator atomically on the same stamped launch checkpoint" | 4783–4785 | No protocol file in the branch diff; `adapters/tests.rs::the_dsh_launch_hold_needs_every_confirmation_before_it_publishes`, `::a_launch_is_published_on_confirmation_and_a_mismatch_publishes_nothing` | discharged |
| A15 | "Resolve that locator beneath the admitted originating DSH home… decline truncation, ambiguity, `..` or symlink escape without scanning for or creating a substitute root" | 4786–4789 | `adapters/tests.rs::dsh_owned_locators_resolve_only_beneath_the_home_and_name_the_offered_root` (empty, absolute, traversal, leading `./`, separator, unresolved, unknown id, file-not-directory, ambiguity, symlink escape + contained control, each by its exact production reason); `::a_dsh_overlong_locator_is_never_truncated_into_another_valid_root` | discharged |
| A16 | "Land the digest work first, in design D10's order" | 4789–4790 | PR #311 (`e78c1da1`, a–c) precedes PR #313 (`b0ec5517`, d) in history | discharged |
| A17 | **(a)** `ResumeIdentity::Measured` gains an optional `wrapper_digest`, "the measured branch's closed key list admits it with seat record v5's 64-lowercase-hex grammar checked at load", unknown still admits `unknown` alone, the member travels into the private start context | 4790–4797 | `agents/load.rs:1112–1136` (closed key list `["version","applies_to","wrapper_digest"]`, `is_lower_hex_64` at load, `only_keys(["unknown"])`); `agents/tests.rs::the_optional_wrapper_digest_member_loads_carries_and_is_refused_by_name`; `resume_tests.rs::a_declared_wrapper_digest_reaches_the_private_start_context` | discharged |
| A18 | **(b)** the plugin component = SHA-256 of six `<relative path>\0<file SHA-256>\n` lines "in bytewise path order"; a missing file or an extra entry "is unreadable and names the drifted file" | 4797–4802 | `composite.rs:257–266` (`component_digest` over a `BTreeMap`, path `\0` digest `\n`); `composite/tests.rs::the_plugin_component_is_bytewise_path_order_and_fails_closed`, `::removing_only_the_plugin_manifest_names_the_drifted_file`, `::the_worked_plugin_vector_pins_the_bytewise_path_order_of_the_component` | discharged |
| A19 | **(b)** the canonical composite over D6's fixed `<component>\0<value>\n` lines; the complete npm path grammar; optional `name` ignored; malformed paths and missing/mistyped/invalid versions rejected | 4802–4813 | `composite.rs:1678–1719` (`canonical_composite`, exact line order), `:286` (`npm_name`, every group parsed, terminal spelling taken); `composite/tests.rs::the_npm_key_rule_takes_only_the_terminal_package_spelling`, `::the_worked_npm_vector_binds_every_key_spelling_to_its_terminal_package`, `::npm_locks_reject_unparseable_and_incomplete_entries` | discharged |
| A20 | **(b)** "one shared source-scalar rule… reject empty, NUL, space, tab, CR and LF without trimming"; "The inherited npm-version predicate is only one use of this rule; the complete 8.10 rejection-vector ledger remains pending" | 4813–4817 | `composite.rs:110–135` (`scalar_reason`/`scalar`); `composite/tests.rs::the_scalar_rule_names_empty_nul_and_every_whitespace`, `::the_component_gate_refuses_each_forbidden_byte`, `::npm_versions_with_any_whitespace_are_unreadable`, `::a_pnpm_integrity_with_whitespace_is_unreadable` | discharged — and the forward reference it makes is row B89, also discharged |
| A21 | **(b)** "Deduplicate only equal complete triples, retain different versions/integrities, then sort their value bytes"; exclude the core's own entry and the plugin's local-tarball entry | 4818–4821 | `composite.rs:1691–1706` (`BTreeSet` over npm ∪ pnpm); `composite/tests.rs::npm_three_group_and_dedup_vectors_retain_distinct_triples`, `::npm_exclusions_name_exact_records_and_keep_same_named_registry_ones`, `::the_plugin_s_own_tarball_record_leaves_and_its_registry_namesake_stays` | discharged |
| A22 | **(b)** the tail: `plugin`, `plugin-patch`, `profile-patch`, one `profile-bundle` per entry "in declared order", `profile-patch-reload`, `home-patch`, "and, only if D6's conditional extension is named in the profile's bundles, `extension`" | 4821–4826 | `composite.rs:1707–1717`; `composite/tests.rs::the_worked_vectors_pin_the_canonical_composite_byte_form` (frozen stream literal), `::the_conditional_extension_is_absent_or_composed_from_its_own_four_files` (absence emits no line) | discharged |
| A23 | **(b)** the conditional extension package, "authored only after a demonstrated missing hook", resolved through the same bundle lookup, required inside the profile, hashed with the same file-line function; "No repository provenance is read at runtime and no second digest producer is added" | 4827–4835 | No `extensions/dsh/resume-policy/` exists (none is demonstrated necessary); `composite.rs:1805` lists `EXTENSION_BUNDLE` only when the profile declares it; `composite/tests.rs::the_extension_walk_refuses_a_missing_extra_or_symlinked_member`, `::the_plugin_and_extension_must_resolve_inside_the_profile`, `::no_test_reassembles_the_component_stream` | discharged |
| A24 | **(b)** D6's locators: `bin.dsh` with the `#!/usr/bin/env node` first line; the hidden lock as the only core lock "with no root-lock fallback"; `node` by AO/D10's native child lookup including the absent-PATH default search; only `<home>/profiles/headless/package.json` for the manifest | 4835–4847 | `composite/tests.rs::the_core_executable_must_be_lib_bin_js_with_the_exact_shebang`, `::the_hidden_npm_lock_is_the_sole_source_and_missing_fields_are_refused`, `::the_root_package_lock_is_never_read_beside_or_instead_of_the_hidden_one`, `::the_default_search_path_is_the_c_librarys_own_answer`, `::an_absent_path_is_a_named_refusal_and_never_the_working_directory`, `::read_profile_refuses_a_missing_manifest_bundles_and_reload`, `::the_composite_reuses_the_launcher_head_selection_inspected` | discharged |
| A25 | **(b)** canonicalize the profile once for containment, keep the raw anchor "so that order does not change"; compare each first-hit canonical bundle directory; "no raw-path fallback, string-prefix check or search past an outside first hit is permitted" | 4847–4854 | `composite/tests.rs::containment_compares_canonical_components_not_string_prefixes`, `::an_outside_first_bundle_hit_is_not_skipped_for_a_later_inside_one`, `::a_bundle_directory_that_is_a_symlink_is_judged_where_it_lands`, `::the_bundle_search_order_is_core_ancestors_then_globals_then_the_profile` | discharged |
| A26 | **(b)** "Preserve the inherited two-path canonical-boundary correction and prove the symlinked-home case in this slice" | 4854–4857 | `composite/tests.rs::the_dsh_composite_accepts_a_symlinked_home_ancestor`; `::one_pair_in_two_homes_under_two_seat_overlays_is_one_identity` | discharged — the forward reference ("the complete 8.10 containment ledger remains pending") is rows B92–B93, also discharged |
| A27 | **(b)** the pnpm lock read "before allocation through D6's inclusive 8 MiB bounded, fail-closed line reader with no YAML crate" | 4857–4859 | `composite.rs:463–610` (`read_pnpm_from`, hand-written line reader); `composite/tests.rs::the_pnpm_reader_is_bounded_inclusively_at_the_limit`, `::the_pnpm_reader_consumes_at_most_one_byte_past_the_limit`, `::the_pnpm_reader_refuses_an_absent_unreadable_or_non_utf8_lock`; no YAML crate in `brokkr-protocol`'s manifest | discharged |
| A28 | **(b)** `cordis.yml`, the raw `package.json`, `pnpm-workspace.yaml`, `.env` layers, persisted state and the per-seat overlay "never enter it"; an entry without a registry integrity, a NUL/newline value, an outside layout or an unreadable component makes the identity unreadable | 4859–4863 | `composite/tests.rs::a_rewritten_generated_cordis_yml_leaves_the_composite_untouched`, `::one_pair_in_two_homes_under_two_seat_overlays_is_one_identity` (per-seat overlay leaves `home-patch` at `absent`), `::the_dsh_composite_refuses_a_layout_outside_the_locators`, `::an_unreadable_npm_lock_and_extension_are_named_by_their_own_component` | discharged |
| A29 | **(b)** "Read every identity-bearing source once per call… and expose only `dsh_composite` as the production producer" | 4863–4867 | `composite.rs:4755` (`pub fn dsh_composite`) and `:4763` (`dsh_composite_prepared`); `adapters.rs:3541` is the only production call; `composite/tests.rs::the_core_manifest_and_hidden_lock_are_read_once_and_retained`, `::the_plugin_patch_cannot_be_a_second_read_of_a_changed_file`, `::no_test_reassembles_the_component_stream` | discharged |
| A30 | **(c)** doctor's `dsh` line reports "the composite's digest or its unreadable component and whether it equals, differs from or has no declared `wrapper_digest`"; "informational until a `supported` shape declares one, then a warning"; reads no credential or settings file; the guide's sample follows it; spawns only the two version probes | 4867–4874 | `doctor/tests.rs::the_dsh_composite_detail_reports_each_disposition` (all four dispositions, warning only under `supported`, escaped reason), `::doctor_appends_the_dsh_composite_detail_to_the_provider_line`, `::doctor_warns_when_the_dsh_composite_differs_from_a_declared_digest`, `::dsh_provider_line_reports_an_unknown_identity_and_a_readable_composite`, `::the_dsh_version_and_composite_come_from_one_resolved_installation`, `::the_guide_documents_the_wording_the_classifier_emits` | discharged |
| A31 | **(c)** "The report describes the composite as read for launch, not lifetime integrity" — no continuous verifier, no exemption | 4874–4878 | `doctor.rs` holds one probe path and no watcher; no background task exists in the crate | discharged |
| A32 | **(d)** complete the planner "in this order, with the corresponding 8.10 cases beside each repair"; R3's staging observation is "a private `#[cfg(test)]` thread-local counter at entry to `dsh_seat_overlay_in`, before either settings or patch creation… Keep the production staging location and planner signature" | 4878–4886 | `adapters.rs:5293–5303` — `DSH_STAGING_CALLS` increments at entry, before `dsh_transcript_row`, the settings branch and `create()`; the signature is unchanged from the inherited one | discharged |
| A33 | "Pass B adopts (a)–(c); it adds no digest producer, public planner seam, dependency, generic argument grammar or live-provider qualification" | 4887–4888 | One producer (A29); `dsh_launch_with` is private; `Cargo.toml` untouched on this branch | discharged |
| A34 | **(d)** extract "exactly one separate-value `--model <id>`", the shared effort splitter's forms, "one exact `--patch <value>`"; refuse `--model=<id>`, duplicate/malformed/valueless model or effort, effort without a model, "before any route read, version probe, composite call or staging" | 4889–4896 | `adapters.rs:3558–3600` (boundaries → model → effort → patch → residual → validation, all before `route_overlay::claim` at 3603); `adapters/tests.rs::dsh_residual_and_joined_controls_refuse_before_any_observation` (marker `zzz-9f31c7-marker`, three planner paths, zero staging, no probe), `::both_dsh_effort_spellings_are_admitted_and_stage_one_overlay`, `::the_dsh_model_and_patch_splitters_refuse_their_malformed_shapes` | discharged |
| A35 | **(d)** "refuse every residual argument… the inherited selector-only deny-list is not the admission rule" | 4896–4900 | `adapters.rs:3563–3572` (`dsh_control_conflict` over ALL residual argv); the residual ledger covers selectors, profile/settings/output overrides, positional text, `--`, aliases, joined and clustered forms, `--from-default-profile`, `--verbose`, unknown names | discharged |
| A36 | **(d)** fixed field/category diagnostics; "never echo an unknown option name, joined value, model, ID or path"; proved on cold, offered and disabled paths with private markers | 4900–4904 | Same test: every rejected spelling carries `MARK` in an option name, a joined value, the model, a selector value, a patch value or positional text, and no diagnostic contains it, on all three paths | discharged |
| A37 | **(d)** bind the seat's single `--patch` "at every model-site start, cold, offered or `unmeasured`, independent of the resume gate, **at both `start_context` call sites**", withholding it for an absolute path, `..`, symlink escape or a non-member | 4905–4917 | Positive: `resume_tests.rs::a_valid_route_overlay_binds_at_the_single_site` (`:1687`), `::…_at_the_panel_member` (`:1731`), `::a_valid_route_overlay_binds_on_an_offered_start_too` (`:2267`, single site), `::a_valid_route_overlay_binds_on_an_offered_panel_member_start_too` (`:2331`). Withholding: `::a_non_binding_route_overlay_withholds_the_member_at_both_call_sites` (`:1784`, panel member), `::…_at_the_single_site` (`:1883`), `::every_remaining_non_binding_shape_is_withheld_at_the_single_site` (`:1940`), `::an_escaping_symlink_member_is_withheld_at_the_single_site` and `…_at_the_panel_member` (`:2061`, `:2108`) | discharged (corrected by unit 2d and its return) — the positive binds at both call sites on cold `unmeasured` starts and on an offered start at both: the single site's is a retry, the panel member's a re-entry offered `alpha-1`, each start carrying the argv value and the manifest digest. Before 2d the offered positive rode the single site alone. Every withholding shape the clause names — absolute, `..`, compiled-member symlink escape, non-member — is withheld at both call sites, each by its own rule: every vector names bytes a lookup could find, and a compiling mutation of each rule parts that shape at each site (see B38). The first 2d grading claimed this while the panel's `..` value named no file; the return closed that. Units 2a, 2b, 2d |
| A38 | **(d)** "The binding is the engine's alone… from the compiled manifest's `files` entry and never from a hash of the file the value resolves to" | 4917–4924 | `resume_tests.rs::a_changed_route_overlay_member_carries_the_manifest_digest` (asserts the manifest digest AND `assert_ne!` against a hash of the changed bytes) | discharged |
| A39 | **(d)** the adapter reads the file once, "require SHA-256 equality with the bound digest before any shape check", then AS3's closed data-only reader and closed six-field set with `apiKeyEnv` required and the `https` endpoint grammar | 4925–4933 | `route_overlay.rs:81–…` (`claim_with`), `::claim_reads_the_bound_file_and_requires_the_digest_before_the_shape`, `::the_route_row_and_provider_are_closed`, `::a_credential_value_or_a_field_outside_the_set_is_refused`, `::the_endpoint_grammar_decides_the_positive_and_every_refusal`; `adapters/tests.rs::a_dsh_route_overlay_planner_checks_the_digest_before_the_shape_and_before_staging` | discharged |
| A40 | **(d)** require the binding and argv agreement before reading; "Every other `--patch` refuses through the existing pre-work failure path, never forwarded or dropped" | 4933–4937 | `route_overlay.rs:89–115` (absent binding with a `--patch`, binding without a `--patch`, disagreement — each a refusal before any read); `adapters/tests.rs::dsh_route_binding_matrix_refuses_before_staging_on_every_planner_path` | discharged |
| A41 | **(d)** "close every disabled assessment gate before the version probe or composite producer, with or without an offer"; a missing/malformed declared digest also prevents either observation; only a matching `applies_to` reaches the sole producer | 4938–4943 | `adapters.rs:3606–3626`; `adapters/tests.rs::a_closed_dsh_gate_reaches_neither_probe_nor_producer_and_keeps_the_cold_route` (ten dispositions × offer/no-offer, counted producer, recording shim marker), `::a_dsh_identity_mismatch_declines_the_offer_and_keeps_the_cold_route` (absent and uppercase-malformed declared digest reach neither) | discharged |
| A42 | **(d)** compare the recomputed composite with the declared; on an offer compare both observations with `originating_harness_version` and `originating_wrapper_digest` "from the same confirmed root"; missing/mistyped/malformed/unreadable/mismatched declines as `unverified-harness` | 4943–4949 | `adapters.rs:3617–3635`; `::a_dsh_identity_mismatch_declines_the_offer_and_keeps_the_cold_route` 10527–10556 | partially discharged — the originating comparison is driven with *different* values for both fields and *missing* for the digest alone; *missing version*, and *mistyped* and *malformed* for either field, are not driven independently (see B60, F3). Unit 6 (recorded as 3) has landed. The row keeps this grade until unit 22 regrades it on opened evidence |
| A43 | **(d)** preserve `root_session.wrapper_digest` → `resume_context.originating_wrapper_digest`; carry the observed version as `harness_version` and the composite as `wrapper_digest` "separately from desired pins" | 4949–4953 | `resume_tests.rs::a_stamped_row_is_offered_only_to_its_own_site_owner_and_persistent_root` 1236–1255 (`originating_root` reads both off the same row); `adapters/tests.rs::a_qualified_dsh_launch_uses_the_stream_json_forms_and_records_observed_identity` (drift records the shim's `9.9.9`, never the requested pin) | discharged |
| A44 | **(d)** "An origin mismatch follows the current observations; it does not imply zero producer calls" | 4953–4955 | `::a_dsh_identity_mismatch_declines_the_offer_and_keeps_the_cold_route` 10527–10556 — the three origin declines run with a producer that RETURNS a matching composite, so one call precedes them | discharged |
| A45 | **(d)** admit the complete owned target only after identity agreement; canonicalize both homes and require equality "before retained-session reads"; "Never switch homes to honour an offer, even when both homes contain the same ID and locator" | 4956–4962 | `adapters.rs:3766–3800` (`owned_dsh_root`: id grammar, provider id equality, locator, home, canonicalize both, compare); `adapters/tests.rs::a_dsh_retained_root_refusal_names_its_field_and_never_the_home`, `::owned_dsh_root_refuses_a_persistence_home_that_does_not_resolve`, `::a_dsh_offer_requires_the_complete_recorded_address_and_a_bounded_locator` | discharged |
| A46 | **(d)** enforce the 80-character bound "using Rust `chars`, not UTF-8 bytes"; validate the planned locator "before the shared `Transcript::record` clamp and before staging too, keeping that clamp unchanged for other producers" | 4962–4967 | `adapters.rs:3670–3676` (`planned_dsh_locator` before the overlay is staged); `adapters/tests.rs::an_eighty_character_multibyte_dsh_locator_round_trips_through_warm_planning` (80 chars / 144 bytes admitted, exact locator, original root, one staging call, clamp leaves it unchanged), `::the_planned_locator_is_bounded_before_anything_is_staged` | discharged |
| A47 | **(d)** canonical containment of each project/session directory "before enumerating it" and of the selected regular `session.v3.jsonl` "before opening it"; a contained alias may pass, an escape into another root inside the same home must fail | 4968–4972 | `::dsh_unsafe_stored_candidates_decline_instead_of_being_skipped`; `::dsh_owned_locators_resolve_only_beneath_the_home_and_name_the_offered_root` (symlinked escape by reason, with the contained control) | discharged |
| A48 | **(d)** "Complete the exact-one valid depth-zero header check for the offered ID, refusing missing, malformed/truncated, unreadable or ambiguous evidence instead of skipping unsafe candidates or finding a substitute" | 4972–4974 | `::dsh_owned_locators_…` (two depth-zero headers naming one id select neither); `::a_session_file_whose_first_row_is_not_the_header_is_unreadable`; `::owned_dsh_root_refuses_a_session_id_outside_the_grammar` | discharged |
| A49 | **(d)** "explicit finite DSH-local budgets and bounded IO; a size check after an unbounded allocation is insufficient. A truncated, invalid or unreadable boundary must decline, never become zero or a partial-prefix maximum" | 4974–4979 | `::dsh_admission_reads_are_complete_within_their_bounds_or_decline`; `::dsh_stored_sequences_decline_instead_of_reporting_a_partial_maximum`; `::a_retained_project_that_cannot_be_read_is_a_bounded_refusal` and its two entry-level siblings | discharged |
| A50 | **(d)** "Preserve the inherited `firstSeq` convention and all current-event folds; B repairs admission reads only" | 4979–4981 | `::a_cold_dsh_launch_folds_its_first_event_and_a_warm_one_folds_past_the_boundary`; `::the_planned_dsh_fold_boundary_reaches_the_transcript_drain` | discharged |
| A51 | **(d)** "fold the validated route ahead of the transcript/model/settings rows in `dsh_seat_overlay_in` and stage exactly one overlay"; both forms retain the admitted `headless` profile, current workdir and Rust-owned model/effort/settings | 4982–4988 | `adapters.rs:5330–5343` (route bytes, then the model row, then the transcript/settings/sandbox rows); `adapters/tests.rs::the_shipped_route_overlay_folds_ahead_of_the_rust_owned_rows` (index ordering asserted), `::dsh_positive_planner_paths_fold_the_shipped_route_ahead_of_rust_owned_rows` | discharged |
| A52 | **(d)** a disabled gate or identity/storage mismatch builds "the shipped `dsh --profile headless --patch <overlay>` cold command under the current home, with no `--new`, `--session` or `--output-format`, no rejoining target and no offerable root. Only a declined offer carries a refusal token; no-offer cold has none" | 4988–4992 | `::a_closed_dsh_gate_reaches_neither_probe_nor_producer_and_keeps_the_cold_route` asserts the whole argv and `launch.refusal == session.is_some().then_some(reason)` | discharged |
| A53 | **(d)** "The bound current route folds on qualified cold, warm and disabled/mismatched cold alike. No route byte or binding enters the composite, launch row or journal. Retain `confirms_from_locator: false`" | 4993–4997 | `::dsh_positive_planner_paths_fold_the_shipped_route_ahead_of_rust_owned_rows` (all four planner outcomes); `adapters.rs:3500` (`confirms_from_locator: false`) | **partially discharged.** The fold and the flag are proved. The **exclusion** is not: `assert_dsh_planner_overlay` (`adapters/tests.rs:12361–12400`) reads the planned argv and the written overlay file, and `dsh_launch_with` returns a plan — no case in either suite opens an emitted launch row, a journal envelope or a computed composite and asserts it free of route bytes. See F8 and unit 4. **Unit 4a (§6, `3a80b1d8`, `a41ae69e`, `9a3c0746`):** the exclusion is now proved at the composite, and at the launch row and the journal, stderr tail and non-protocol stdout included, for the shipped route, which the real adapter runs under the engine. The journal half rests on `9a3c0746`: at `a41ae69e` the case filtered the adapter's stdout to JSON-looking lines, so a disclosure printed outside the protocol never reached the journal it searched (review of `f5895001`, SEC-2). For the gated shapes it is proved at the wire and the engine separately, not in one process, so it **stays partial** there. **What unit 7 (recorded as 4a) proved, against the third chief's finding 4.** The chief found that a `model_driver` shim emitting a fixed claude-session checkpoint, whose route is only `route: offered`, cannot exercise production DSH publication (`adapters.rs:843–883`). Unit 7's record answers that for the **shipped (closed-gate) route**. That case runs production's `adapters::serve(Dsh)` under the engine on a seat whose `--patch` binds, reads the journal and the raw stdout, and a content-copy mutation (`displayName` copied into `effort`) parts it. The engine's offered cases use the DSH-shaped `dsh_model_driver`, which has a `dsh-session` root and a `transcript`. The **gated shapes** (qualified cold, confirmed rejoin, declined offer) are the half it did not prove in one process, because the real adapter's open gate never ran under the engine. **Still partially discharged**, and unit 11 owns that half. The earlier "See … unit 4" is unit 14, the gate run |
| A54 | "Do not move the roster assertion, `bundle.json`, `research-web.yml`, compiled staffing or research-dsh witness digest" | 4998–5002 | `git diff origin/main --name-only` names none of them | discharged |
| A55 | "Current AU/D7 repair clauses 8.8.9.1–8.8.15.1 **alone are commissioned here**" | 5002–5005 | Rows S1–S13 | discharged (scope rule) — and read narrowly. It says which clauses *this* repair executes; it does not say 8.8's acceptance is those clauses. §1's fourteen numbered tasks are not commissioned by this slice and are still 8.8's to satisfy (1209–1212) |
| A56 | "Confirm the launched root before publishing": a valid prior depth-zero header at the resolved locator, the plugin's post-`await agents.resume` init event read from the stream-json child, no fresh sibling root/session, and new sequence activity past `firstSeq`, "before the launch hold releases" | 5005–5013 | `adapters/tests.rs::the_dsh_launch_hold_needs_every_confirmation_before_it_publishes`, `::a_dsh_init_event_alone_is_never_the_root_confirmation`, `::a_qualified_dsh_child_confirms_the_root_and_folds_current_only`, `::dsh_a_consistent_pending_rejoin_still_confirms_through_the_terminal_body` | discharged |
| A57 | "Same-root nonce continuity is 10.7's probe-only model-recall device… never planted in a prompt or read at run time" | 5013–5017 | `git grep` finds no nonce in `adapters.rs` or any prompt template; the confirmation reads only header, init event, sibling-root and sequence facts (`DshObservation`) | discharged |
| A58 | "Retain the immutable pre-spawn census of admitted `(header ID, canonical file)` occurrences, exactly one baseline offered header and its address; require current counted occurrences to fit the baseline without collapsing IDs, addresses or multiplicity" | 5017–5020 | `::dsh_census_identity_counts_occurrences_and_addresses_not_distinct_ids`; `::dsh_a_rejoin_without_its_pre_spawn_baseline_is_refused_before_the_child_runs`; `::dsh_a_fresh_entry_reusing_a_sibling_id_at_a_new_address_refuses_the_rejoin` (two admitted addresses behind one id) | discharged |
| A59 | one observation rule on both sides of init; the listed failures "permanently refuse. Only readable consistent waiting is retryable; later init, repaired storage, EOF or delivery never cures refusal" | 5020–5026 | `::dsh_contradictions_after_the_init_event_are_never_restored_away`, `::dsh_an_unreadable_stream_line_refuses_a_pending_rejoin_before_the_stream_ends`, `::dsh_uncertainty_before_the_init_event_is_never_cured_by_a_later_reading`, `::dsh_cold_noise_and_a_root_mismatch_keep_their_terminal_behaviour` | discharged |
| A60 | "R1–R3 end failed with LE3's exact unconfirmed-session reason on both endings, no accepted result, root, locator, launch or work publication and no cold replacement; retain delivery on disk" | 5026–5030 | The shared contract `adapters/tests.rs:5903` (`assert_refused`): terminal reason `"provider never confirmed the offered session; refusing to accept the invocation"` — byte-identical to `specs/adapter-launch-evidence/spec.md:98`; no `root_session`/`transcript`/`launch` row; one child; a delivered file retained, never accepted. Driven on BOTH endings by `refused_on_both_endings` (`:6064`) | discharged |
| A61 | "Verify the six headline child proofs, completed-observation witnesses and all D7 rule controls/removals through the real terminal body" | 5030–5032 | `run_dsh_latch` (`:5999`) drives a real child through production `run_seat_with` + `invoke_dsh_launch_observed`; the six headline proofs are R1/R2/R3 × two endings in `::dsh_malformed_output_before_the_init_event_…`, `::dsh_an_observed_fresh_sibling_…`, `::dsh_a_fresh_entry_reusing_a_sibling_id_…` | **partially discharged — evidence-verification gap.** The six headline proofs and the witnesses carry existence, assertion coverage and execution. The clause's "**and all D7 rule controls/removals**" rests on the fourteen-row ledger at `tasks.md` 465–480 (*this said "twelve-row", corrected 2026-09-23*), which this seat opened and found to be a delivery narrative: it names mutations, endings and a restored SHA-256, and leaves nothing in the tree to re-derive (F5). The first cut graded this row `discharged` while saying the same thing in its own note |
| A62 | "Never forward the launcher's TUI example, treat the retained directory as a provider handle, alter the live global pin/profile, add an SDK runner or admit hands" | 5033–5035 | `::a_retained_dsh_directory_alone_never_supplies_a_provider_handle`; `::a_dsh_offer_is_declined_and_its_retained_directory_is_not_a_handle`; no SDK path in `adapters.rs`; `adapters/dsh.json` unchanged | discharged |
| A63 | "If 10.7 demonstrates that the documented setup or pre-work observation hook is still insufficient… add only the narrow Cordis extension" | 5035–5044 | `tasks.md:5732` — 10.7 is `[x]` and demonstrated no insufficiency; no `extensions/dsh/resume-policy/` was created | discharged — conditional, condition not met |
| A64 | "Verify with the DSH planner/storage shim cases in 8.10 and 9.6; the route-overlay binding's engine cases in the runtime crate… beside `the_private_context_carries_the_owned_target_and_originating_digest` and in its pattern… and in `engine/resume_tests.rs`, the engine integration cases 8.10 assigns to that suite, read off the `Start.input`… at both call sites, a single site and a panel member" | 5044–5058 | `resume.rs::the_private_context_carries_a_supplied_route_overlay_binding` (sits directly beside the named case, asserts value, digest and absence); `resume_tests.rs` route rows A37/A38 | partially discharged, **on two counts now**. (1) 9.6's shim cases do not exist (row not owned here; see the 9.6 answer). (2) The claim that "every case 8.10 assigns exists and asserts" is withdrawn: it was asserted over rows that are themselves partial. The engine route cases it names ride one call site each (B38, B39), the offered positive has no panel-member twin (B36), and the private-carrier case does not reach launch evidence (B30). Units 2a and 4a, now units 2 and 7 with 3 and 8 beside them, have landed and re-graded B36 and B38. B30 keeps unit 11's half, and B39 awaits unit 22's regrade |
| A65 | the four loader cases in `agents/tests.rs` | 5059–5063 | `::the_optional_wrapper_digest_member_loads_carries_and_is_refused_by_name` — all four in one case: measured without the member loads; a well-formed member loads and is carried; malformed or beside `unknown` is refused naming the field; `assert_ne!` on the adapter content digest | discharged |
| A66 | "doctor cases… for a matching, differing, undeclared and unreadable composite" | 5063–5064 | `doctor/tests.rs::the_dsh_composite_detail_reports_each_disposition` — all four, plus the warning rule | discharged |
| A67 | the committed-bytes test: exactly six files; the function's per-file lines carry "the provenance block's path and SHA-256 pairs"; the adapted expression "occurs exactly once"; "the recomputed delta digest equals the note's"; substituting the upstream expression back reproduces `a40b52b3…` | 5064–5071 | `composite/tests.rs::the_committed_plugin_set_is_the_six_files_and_the_one_expression_delta` — asserts the six names, the full digest map against `COMMITTED_PLUGIN_DIGESTS` (`:12039`, which this seat compared line-by-line with `PROVENANCE.md:67–72`), one occurrence, and the upstream SHA-256 | **partially discharged** — four of five. **No test recomputes the delta digest.** `78256d2e…` appears only at `PROVENANCE.md:50`. This seat recomputed it by hand (SHA-256 of `lib/index.js:253\n` + `-` upstream line + `+` adapted line, each newline-terminated) and it reproduces exactly — so the note is true, and what is missing is the assertion, not the fact. See F1 and unit 1, which landed at `8f60f06c`. The row keeps this grade until unit 22 regrades it on opened evidence |
| A68 | "Full 8.8 remains pending through C/D; completing B alone never ticks it" | 5071–5074 | Pass C (`b0ec5517`) and Pass D (D1–D3 on this branch) are both delivered; 8.8 stays `[ ]` | discharged (reading rule) |

## 3. Task 8.8's commissioned subgroups, 8.8.9–8.8.15 (133–345)

| Row | Clause | Line | Evidence opened | Status |
|---|---|---|---|---|
| S1 | 8.8.9.1 `[x]` — D7's private `run_seat` seam, helpers returning "the actual wire results and checkpoints from one real child"; warm and qualified-cold controls through the shared terminal body; "Prove the controls' publication/terminal assertions by applicable removal and restored rerun" | 135–143 | `adapters.rs::run_seat_with` + `invoke_dsh_launch_observed`; `adapters/tests.rs:5999` drives one real child; `::a_qualified_dsh_child_confirms_the_root_and_folds_current_only`; `::dsh_cold_noise_and_a_root_mismatch_keep_their_terminal_behaviour` | **partially discharged — evidence-verification gap.** The seam, the real child and the two controls are opened and green. "Prove the controls' publication/terminal assertions **by applicable removal and restored rerun**" rests on rows M10 and M12 of the narrative ledger (F5) |
| S2 | 8.8.9.2 `[x]` — the per-invocation completed-observation seam; "notify on all completed outcomes, not only in the refusal branch"; "Record exact observed facts, not a separate walk or child-created proof marker" | 144–152 | The observer at `:6036` receives every `DshObservation`; `assert_one_child` asserts `acknowledged == acks` and that the child passed each await; `::dsh_a_consistent_pending_rejoin_still_confirms_through_the_terminal_body` | discharged |
| S3 | 8.8.10.1 `[x]` — one absorbing refusal transition, initialized when an offered launch lacks `first_seq`, a successful baseline census or exactly one baseline offered header; "never replace missing history later" | 156–163 | `::dsh_a_rejoin_without_its_pre_spawn_baseline_is_refused_before_the_child_runs` (its five baseline cases); `::dsh_contradictions_after_the_init_event_are_never_restored_away` | **partially discharged — evidence-verification gap.** The transition and its baseline cases are opened and green. The clause's closing sentence — "Verify focused state transitions and baseline-repair controls **with applicable compiling removals**, restoring the repair green" (161–163) — is narrative (F5) |
| S4 | 8.8.10.2 `[x]` — counted pair containment over `(ID, PathBuf)` occurrences; cardinality, then retained address, then counts; "an unrelated old sibling's disappearance alone does not" refuse | 164–174 | `::dsh_census_identity_counts_occurrences_and_addresses_not_distinct_ids`; `::dsh_cold_noise_and_a_root_mismatch_keep_their_terminal_behaviour`; the positive "an unrelated baseline sibling is gone" case | **partially discharged — evidence-verification gap.** Counted pair containment and all four controls are opened. "verify focused new-address, equal-pair multiplicity, equal-total replacement and old-sibling-removal controls **with applicable removals/restoration**" (170–172) is narrative (F5) |
| S5 | 8.8.10.3 `[x]` — one store rule on both sides of init; "Before init, `last > first_seq` refuses; after init only readable consistent `last <= first_seq` waits, and `last > first_seq` confirms"; the weaker pre-init predicate retired | 175–184 | `adapters.rs` `dsh_read_offered_store` (one rule, both sides); `::dsh_work_before_the_init_event_is_never_adopted_by_it`; `::dsh_a_consistent_pending_rejoin_still_confirms_through_the_terminal_body` | **partially discharged — evidence-verification gap.** The unified rule and the retirement of the weaker predicate are opened. "Verify each former early return against D7's disposition table with focused **controls/removals**" (181–182) is narrative (F5) |
| S6 | 8.8.11.1 `[x]` — refuse malformed pre-init output "even with no observed sequence advance"; refuse pending line-read errors before `break`; "each removal must fail the intended terminal assertion, not time out" | 188–196 | `::dsh_malformed_output_before_the_init_event_refuses_the_rejoin_for_good` — two cases, the second with the store UNMOVED when the line is read, `observations[0] == {Malformed, census: None, last_seq: None, Refused}`; `::dsh_an_unreadable_stream_line_refuses_a_pending_rejoin_before_the_stream_ends` | **partially discharged — evidence-verification gap.** Both refusals and the no-advance case are opened. "**each removal must fail the intended terminal assertion, not time out**" (194–195) is narrative (F5) — and note that this clause does not merely ask for a removal, it asks for a property *of* the observed failure, which a record cannot supply |
| S7 | 8.8.11.2 `[x]` — apply the common store observation on a malformed post-init line "without blanket JSON refusal"; consistent noise can still confirm; EOF/drain cannot reverse an absorbing outcome | 197–204 | `::a_qualified_stream_json_launch_skips_a_malformed_line_and_still_confirms`; `::dsh_contradictions_after_the_init_event_are_never_restored_away` (the sibling read on a malformed line) | **partially discharged — evidence-verification gap.** The common observation, the confirming noise case and the EOF/drain rule are opened. "**Remove only** the observation-before-skip protection while keeping witness acknowledgment active; require the terminal assertion to fail, then restore green" (201–203) is narrative (F5) |
| S8a | 8.8.12.1 `[x]` — prove **R1**, baseline `session-1`/27 with malformed pre-init output, on both LE1 endings; "Remove malformed pre-init refusal and observe each terminal/publication assertion fail"; "Keep 8.8.11.1's no-advance case distinct" | 208–216 | `::dsh_malformed_output_before_the_init_event_refuses_the_rejoin_for_good` through `refused_on_both_endings` (`adapters/tests.rs:6064`); its second case keeps the store UNMOVED, which is the distinct no-advance case S6 also cites | partially discharged — evidence-verification gap: the removal (ledger row M1a) is narrative (F5) |
| S8b | 8.8.12.2 `[x]` — prove **R2**: matching init with a fresh `session-9`; "Witness the completed contradictory production census before permitting deletion"; remove only the permanent contradiction transition | 217–226 | `::dsh_an_observed_fresh_sibling_…` through `refused_on_both_endings`; the case witnesses the exact contradictory census it consumed before the sibling is removed | partially discharged — evidence-verification gap: the removal (M3) is narrative (F5) |
| S8c | 8.8.12.3 `[x]` — prove **R3** "separately": the same ID at `--old--` and `--new--`, one offered `session-1`; "Mutate counted containment to the former ID-only comparison"; "no transient R2 case or Pass D storage-admission matrix substitutes" | 227–236 | `::dsh_a_fresh_entry_reusing_a_sibling_id_at_a_new_address_refuses_the_rejoin`, witnessing two admitted addresses behind one id, through `refused_on_both_endings` | partially discharged — evidence-verification gap: the removal (M5) is narrative (F5) |
| S9a | 8.8.13.1 `[x]` — "Post-init offered-header contradictions cannot be restored away": missing and ambiguous offered headers after init, then restored uniqueness; "isolate/disclose overlapping cardinality/identity protection in removals" | 240–248 | `::dsh_contradictions_after_the_init_event_are_never_restored_away` | partially discharged — evidence-verification gap. The disclosure the clause demands is made, at `tasks.md` 481–487 ("Not separately removable, and said so: the CURRENT offered-header cardinality check after init"), which is itself narrative (F5) |
| S9b | 8.8.13.2 `[x]` — "Required post-init evidence cannot become readable later to cure refusal": independently fail census and offered-sequence reads after init, witness each, then repair the store | 249–258 | `::dsh_contradictions_after_the_init_event_are_never_restored_away` and `::dsh_uncertainty_before_the_init_event_is_never_cured_by_a_later_reading`; the baseline half is `::dsh_a_rejoin_without_its_pre_spawn_baseline_is_refused_before_the_child_runs` | partially discharged — evidence-verification gap: removals M3 and M6/M6′ are narrative (F5) |
| S9c | 8.8.13.3 `[x]` — "A fresh sibling observed before init also latches refusal" on valid non-init JSON; "replace sleep-only causal evidence for any transient case newly credited here" | 259–266 | `::dsh_a_fresh_sibling_observed_before_the_init_event_refuses_the_rejoin_for_good`; the transient cases wait on the observer seam (S2), not on sleeps | partially discharged — evidence-verification gap: removal M4 is narrative (F5) |
| S9d | 8.8.13.4 `[x]` — the remaining identity comparisons: an admitted alias repeating the exact canonical `(ID, file)` occurrence, and a replacement changing a retained address with unique IDs and total count unchanged | 267–276 | `::dsh_census_identity_counts_occurrences_and_addresses_not_distinct_ids` | partially discharged — evidence-verification gap: removals M5, M5b and the disclosed joint M5+M7 are narrative (F5) |
| S9e | 8.8.13.5 `[x]` — "A consistent pending rejoin can still confirm" and "Cold stream noise keeps its existing behavior"; "run applicable removals on new assertions to exclude unconditional refusal or whole-census equality" | 277–287 | `::dsh_a_consistent_pending_rejoin_still_confirms_through_the_terminal_body`; `::dsh_cold_noise_and_a_root_mismatch_keep_their_terminal_behaviour` | partially discharged — evidence-verification gap: removals M9, M10 and M12 are narrative (F5) |
| S10 | 8.8.14.1 `[x]` — the transition/return audit against a "task -> scenario -> actual test/ending -> mutation -> failed assertion -> restored pass ledger"; "Inspect the diff to ensure no mutation… survives" | 291–299 | `tasks.md` 465–489 is that ledger, with fourteen mutation rows (*this said "twelve", corrected 2026-09-23*) and their endings; `git diff origin/main` shows no mutation in the tree, and production is `e50020ac`'s | **partially discharged — evidence-verification gap.** "Inspect the diff to ensure no mutation… survives" is discharged, on the opened diff. The audit's own substance — that each transition and return was exercised, per the "task -> scenario -> actual test/ending -> mutation -> failed assertion -> restored pass" ledger — is narrative (F5) |
| S11 | 8.8.14.2 **`[ ]`** — "run each D11 gate separately, in order: `cargo fmt`…; `openspec validate --all --strict`; `compile --bundle bundles/self`; `… bundles/verify`. Record command, revision and result; tick only when all these checks execute green. **An unavailable tool is not a pass.**" | 300–313 | The recorded-execution table above. Two ledger seats ran fmt and one ran four of seven suites green; both were **refused `openspec`**, as were the D1/D2/D3 seats (`tasks.md` 650–657, 833–848, 1020–1037) and the Pass C seat (507–511). The **review seat** ran `openspec validate --all --strict` green on `d73d94d1`. *(Corrected, third return: the previous text said clippy, four crate suites and both bundle compiles had "no recorded result on a Pass D candidate". That was false. The D3 seat and its returned review report fmt, clippy, all seven crate suites in sequence and the self bundle passing (`tasks.md` 1010–1018, 1093–1099). `bundles/verify` and `openspec` are absent from those runs)* | **not started** as a complete ordered execution. One gate of the list now has a dated green result on the head, out of order and outside a delivery record; the clause asks for each gate run "separately, in order" on one candidate, and ticking "only when **all** these checks execute green". Duplicated by N12 (8.8.8.2), which adds both bundle compiles. See unit 14 |
| S13 | 8.8.14.3 **`[x]`** — "Prepare the unchanged external exact-coverage handoff"; verify no pin, gate, exclusion, denominator or test-selection change; "Record revision/environment and actual covered/total lines, branches and functions **when supplied, otherwise explicitly pending/unavailable**"; "This task verifies **preparation and truthful handoff only**; its tick is not a green coverage or remote gate" | 314–326 | `scripts/coverage-exact.sh` and both workflows consume `rust-nightly-version.txt` (the release configuration's own toolchain-agreement extension); the pending external results are recorded as pending at F7 and in the D2/D3 delivery sections | **discharged, and it has no pending half.** The first cut had no row for 8.8.14.3 at all and then, in its remaining work, invented one — assigning the external coverage run to "8.8.14.3's pending half". Its closing sentence forecloses that: the tick is preparation and truthful handoff, already given. The coverage obligation is **N13 (8.8.8.3)**, and recording external evidence as pending is **S12 (8.8.15.1)** |
| S12 | 8.8.15.1 **`[ ]`** — record the delivery, "Tick each finished scoped task beside its evidence", verify `git diff --check`, strict active-change validation, the requirement/checkbox inventory, "then commit the exact staged repair and ledger"; "Do not report delivery before that commit or represent pending external gates as passed" | 327–345 | The three Pass D delivery records exist (`tasks.md` 535, 703, 879) and the clause's own gate — "After the scoped implementation/proofs **and local gates above pass**" — is S11, which has not | **not started** — blocked on S11 by its own first words. Note what it does and does not require of external evidence: it asks that "external pending evidence" be **recorded**, not completed, so it can close over a pending N13. See unit 22 |

## 4. Task 8.10 (5089–5452)

| Row | Clause | Line | Evidence opened | Status |
|---|---|---|---|---|
| B1 | "This whole-change acceptance remains pending." | 5089 | `tasks.md:5089` reads `- [ ] 8.10` | discharged — and unchanged here |
| B2 | "execute only 8.8.9.1–8.8.15.1: R1–R3 plus the bounded watcher-rule controls, both terminal endings, completed production observations, independent compiling removals and positive/cold preservation" | 5089–5093 | Rows S1–S13 | partially discharged — S11 and S12 are open, and every removal-bearing row among S1–S9e carries an evidence-verification gap (F5). Note the scope word: 8.10 says "execute **only** 8.8.9.1–8.8.15.1", so §1's numbered tasks are not 8.10's work — they bear on 8.8's tick, not this one's |
| B3 | "Do not repeat adopted Codex or Pass B work, open Pass D's unrelated matrix or tick this whole task" | 5093–5094 | Pass C's commit touched `adapters.rs` and `adapters/tests.rs` only; 8.10 is `[ ]` | discharged (scope rule) |
| B4 | "six wrapped/unwrapped compiled Codex gate decisions and supported exact-root exchanges, then `bundle.rs`'s two reachable refusal tests and unreachable census-arm consolidation" | 5095–5098 | `driver_conformance.rs::the_compiled_live_inline_codex_shapes_rejoin_their_provider_confirmed_root` (four shapes: single/no-hands-member × wrapped/unwrapped, each asserting the recorded root, the `resumed` row, no refusal and the exact resume argv) and `::the_compiled_hands_inline_codex_shapes_refuse_unavailable_confinement` (two) = six; `bundle/tests.rs::a_dialect_wrapped_verify_select_reaches_the_single_or_panel_refusal` (`:220`, asserting "dialect verify currently requires a single or panel verify seat") and `::a_selected_agent_case_keeps_its_empty_reference_cause` (`:254`, asserting "seat 'work:engine' agent must be a non-empty string"). The consolidated arm is `owner_index`'s one existing-label refusal (`bundle.rs:1741`, message "addresses two different sites as"), which `::a_literal_phase_that_aliases_a_selected_case_is_refused_globally` (`:1271`) drives | discharged. *(Corrected, fourth return: this row named `::a_raw_phase_that_aliases_a_wrapped_panel_member_is_refused` (`:1349`) and `::a_literal_phase_that_aliases_the_injected_validator_is_refused` (`:1387`) as "the two reachable refusal tests", with `claim_address` (`bundle.rs:2949`) as the arm. The commission itself says otherwise. THE PROOFS item 4 names `assemble`'s second verify refusal and `parse_selected_body`'s empty agent reference as the reachable pair, and `owner_index`'s same-owner tolerance as the arm to remove (`tasks.md` 9465–9502). `4a3854ca`'s message agrees: "new compiler tests reach the second verify refusal… and the selected agent resolver error". The two tests this row cited are D10 F2 authoring-census refusals, added by `4daaa7d2`. They are new tests of this change too, so B5 inventories them. All five exist and assert what is claimed, so the grade stands)* |
| B5 | "Every new test needs an observed compiling mutation failure at its claimed assertion and a restored pass." | 5098–5099 | Recorded mutation ledgers: Pass C's fourteen rows (465–480: M1a, M1b, M2, M3, M4, M5, M5b, M5+M7, M6, M6′, M8, M9, M10, M12; *this cell said "twelve", corrected 2026-09-23*), D1's two (616–623), D2's seven (803–816), D3's six plus one discarded (992–1008), and the returned review's two (1058–1074), and task 10.5's M8, M8a, M9 and M9a (10230–10233). Each names the case that parted | **partially discharged — evidence-verification gap.** This is the clause the narrative rule bites hardest: "**Every** new test needs an **observed** compiling mutation failure at its claimed assertion and a restored pass." Every such observation in this change is a delivery record. What is opened here is that no mutation survives (`git diff origin/main`) and that the cases those mutations aimed at exist, assert what is claimed, and pass (F5). **Scope, third return (finding 3):** "Every new test" covers every suite this change added tests to, not only the terminal body. That includes the Codex bridge and conformance suites, and the tests units 1–10 landed on `slice-dsh-8810`, each of which recorded its own compiling mutations under §6. **Fourth return (finding 1): THE PROOFS' tests, one by one.** The clause's own sentence covers them: its "Every new test" follows "six wrapped/unwrapped compiled Codex gate decisions… then `bundle.rs`'s two reachable refusal tests and unreachable census-arm consolidation" (5095–5099), and the commission asks for "exact mutation diffs/failures/restored passes for both new refusal tests and the census control" (9504–9505). Searched: `tasks.md` (each test name occurs only in its commissioning clause), the landing messages of `4daaa7d2` and `4a3854ca`, the squash `5ef4a842` and `.forge/`. (i) D10's four removal bindings (9451–9455), all **recorded but unverified**. Three are against the six compiled Codex cases (`driver_conformance.rs:3194`, `:3297`, B4): `4a3854ca` says "disabling whole relocation, dropping the no-hands marker, and falsely marking a hands member as no-hands each broke the named test", naming no assertion. The fourth, "delete the independent absence refusal to break otherwise supported unknown-marker controls" (9454–9455), is not a control over those cases. Their inputs carry known markers: `:3228` asserts `"not applicable"` for the no-hands shapes, and the hands shapes are namespace/boxed. So they never reach a missing-marker refusal. The matrix that does is `adapters/tests.rs::a_supported_assessment_without_both_affirmative_markers_declines` (`:1987`). Its decision arm (`:2086–2089`) refuses absent, null, non-string and unknown markers with "`{case}: must not enable`". Task 10.5's visit recorded that test's controls (`tasks.md` 10230–10233): M8, "remove the boundary membership term", failed with "`boundary unknown: must not enable`"; M8a, "bypass the boundary absence/type guard with a `"harness"` default", failed with "`boundary absent: must not enable`"; M9 and M9a did the same for hands. Each restored to "1 passed". That record is historical. It was made at `9da5ff92`, where its `tests.rs:2030:43` is the same `panic!` arm. `git log -L` finds no later commit touching `resume_gate` (`adapters.rs:931`) or the test's body. *(Fifth return, C1: this said "no record found" for the fourth binding, and unit 21(a) sent it to the conformance cases, which cannot exercise it.)* One predicate of the same commission has no record: "Common marker/assessment mutations must also fail unwrapped controls" (9455–9456). Both conformance loops run the wrapped shape first (`:3197–3198`, `:3300–3301`), and 9460 says "masked mutations do not count". So "broke the named test" cannot show that an unwrapped shape parted. That holds for the hands case too: `:3297`'s loop runs the wrapped `HandsMember` first (`:3300`), so `4a3854ca`'s "falsely marking a hands member as no-hands… broke the named test" establishes at most the wrapped shape's failure, and the unwrapped `HandsMember` failure at the direct refusal-token assertion (`:3352–3355`) is unit 21(a)'s. *(Residual C1, 2026-09-23: this cell graded the three records without saying so.)* (ii) `bundle/tests.rs:220` and `:254`, the two reachable refusals: **no record found**. (iii) The census control, `owner_index`'s refusal disabled in both census invocations against `:1271` (9497–9502): **no record found**. (iv) `bundle/tests.rs:1349` and `:1387`, the D10 F2 collision refusals from `4daaa7d2`: **no record found**. (v)–(vii) `4daaa7d2`'s other three new tests: `bundle/tests.rs::a_wrapped_verify_panel_leaves_an_unrelated_literal_phase_untouched` (`:1425`, asserting "the unrelated literal phase keeps its own hands" at `:1448` and "no prefix sweep manufactured a wrapper coordinate for the literal phase" at `:1455`), `::a_wrapped_panel_drains_overlapping_member_addresses_without_overwrite` (`:1469`, asserting "member `{member}` keeps its own network-enabled hands" at `:1491` and "member `checks:{member}` keeps its own default hands" at `:1501`), and `engine/tests.rs::a_selected_single_publishes_its_own_confinement_at_dispatch` (`:188`, asserting "the selected single publishes its own boundary" at `:266` and `hands` equal to `"boxed"` at `:270`): **no record found**. Searched as for (ii)–(iv). None of the three names occurs in `tasks.md`, and `4daaa7d2`'s message lists gates, not removals. `design.md:4972` names `:188` only to say it "asserts markers only". *(Fifth return, C2: this inventory had missed all three.)* A ruling can accept a record. It cannot accept an observation nobody recorded. So (i)'s unwrapped predicate and all of (ii)–(vii) belong to unit 21(a) in every case. (i)'s four recorded bindings go with the class to unit 17, or to unit 21(b) after a no. B5 closes when unit 21(a) has landed **and** either a unit-17 ruling names B5 or units 18, 19 and 21(b) have landed. Unit 20 is 8.8's alone, so B5 does not wait on it. *(Fourth return, findings 1 and 3: this cell said "after units 18–21 all land", which both over-required unit 20 and left (i)–(iv) with no work)* *2026-09-23: unit 21(a) has landed (§6, entry 21; `removal-controls-2026-09-23.md`). (i)'s unwrapped predicate and (ii)–(vii) are performed: six controls parted their named assertions, and four (F1, and F2–F4 for (iii) and (iv)) passed under the operator's ruling that for exactly those four the first assertion depending on the removal, quoted verbatim, stands in for the named one. B5 keeps its grade, and now waits only on a unit-17 ruling that names it, or on units 18, 19 and 21(b)* |
| B6 | "Extend the existing runtime/protocol/CLI suites using the test-only seam; do not substitute fabricated roots, repaired markers or map assertions." | 5099–5101 | All new cases live in the four existing suites; the seams are `run_seat_with`, `invoke_dsh_launch_observed`, `dsh_launch_with` and `DSH_STAGING_CALLS`, all private | discharged |
| B7 | "A no-offer refusal is judged by the private production gate with actual composed facts." | 5102–5103 | `::a_closed_dsh_gate_…` drives `dsh_launch_with` with composed `resume_context` inputs and reads `launch.refusal` | discharged |
| B8 | "Namespace/boxed remains refused; preserve shipping harness/none and no-hands live controls." | 5103–5104 | `::the_compiled_hands_inline_codex_shapes_refuse_unavailable_confinement` (`restrictions-unavailable`, then a fresh cold launch); `boundary_tests.rs::the_seat_input_names_the_boundary_and_the_marker_only_under_a_box` | discharged |
| B9 | "Directly executed shebang tests are Unix-only or use a real target-platform executable." | 5104–5105 | Every shebang-driving case reads `#[cfg(unix)]` (`adapters/tests.rs` 4068, 6110, 10236, 12563, 12736…); `composite/tests.rs::each_platforms_absent_path_search_is_its_own_loaders_rule` is `#[cfg(unix)]` | discharged |
| B10 | "Clauses 5–6 own fresh gates, coverage and the unsigned evidence commit." | 5105–5106 | Rows S11, S12 | partially discharged — the gates and the commit are open |
| B11 | "Keep this checkbox pending for inherited C/D work" | 5106–5108 | 8.10 is `[ ]` | discharged |
| B12 | "The following provider-planner and operator-ruling breakdown is inherited acceptance/history, not additional work commissioned by this slice." | 5109–5111 | Reading rule; rows B13–B22 are graded as acceptance all the same | discharged |
| B13 | "Complete each provider-local planner guard and its tests in `adapters/tests.rs` from the captured grammar, and the engine's private-target and route-binding cases in the runtime suites" | 5111–5114 | Rows A34–A53, B36–B55 | **partially discharged.** Like A13, it is only as strong as the rows it points at, and several are partial: A37, A42 and A53 in the planner group; B36, B38, B39, B42, B47, B49, B50 and B55 in the route group. The guards themselves are opened and green throughout; what is partial is their proof coverage. Units 2a, 2b, 2c, 3 and 4a (now 2, 3, 5, 6 and 7, with 4, 8, 9 and 10 beside them) have all landed. A37, B36 and B38 were re-graded by their units. The rest keep their pre-unit grade until unit 22 regrades them on opened evidence, except the half of A53, B42 and B30 that unit 11 owns |
| B14 | "Keep exact arity, duplicate and precedence checks for every authoritative restriction on cold and resume paths without a generic provider grammar" | 5114–5116 | `::claude_refuses_a_duplicate_or_valueless_authoritative_restriction` (eleven spellings × cold and offered); `::only_the_flags_a_resume_can_safely_carry_travel_with_it`; `::a_second_bare_or_odd_patch_is_refused_by_arity` | discharged |
| B15 | "Preserve the completed Claude cases that independently refuse a second or last-wins permission mode, tools list, strictness/MCP document, allowed/disallowed-tools list (including aliases), model or effort control" | 5116–5119 | `::claude_refuses_a_duplicate_or_valueless_authoritative_restriction` 4074–4123 — permission mode twice and joined+separate, tools twice, `--strict-mcp-config` twice, `--mcp-config` twice, `--allowedTools`/`--allowed-tools`, model twice, effort twice, and three valueless spellings; each on `None` and an offered session | discharged |
| B16 | the operator-ruling breakdown: "first the composition bridge in `engine/boundary_tests.rs` and the existing agent suites, then the shipped assessment/production-composed argv exchange in CLI `tests/driver_conformance.rs`, with separately observed disabled-status, boxed-hands, harness-fragment and boundary-mark mutation failures" | 5119–5125 | `boundary_tests.rs::the_shipped_codex_harness_work_seat_composes_the_preserved_rejoin`; `driver_conformance.rs::the_shipped_codex_harness_work_seat_rejoins_its_retry`, `::the_shipped_inline_codex_work_seat_rejoins_its_retry` | **partially discharged — evidence-verification gap.** The bridge and the exchange are opened and green. The clause's "**separately observed** disabled-status, boxed-hands, harness-fragment and boundary-mark mutation failures" are four narrative records (F5). They close on a unit-17 ruling that names them, or by unit 19 |
| B17 | "The full resolved boxed MCP argv stays cold." | 5125 | `::the_compiled_hands_inline_codex_shapes_refuse_unavailable_confinement` — the declined offer falls back to a fresh cold launch | discharged |
| B18 | run `the_seat_input_names_the_boundary_and_the_marker_only_under_a_box` and `no_gate_topology_is_ever_offered_a_session` beside that bridge | 5125–5129 | Both exist (`boundary_tests.rs:1562`, `resume_tests.rs:720`) and both ran green in this seat's `brokkr-runtime` suite (456 passed) | discharged, with recorded execution |
| B19 | "Then the actual DSH cold planner boundary, Codex refusal cause, Codex cold selector and six-file digest controls, in that order." | 5130–5131 | `::a_closed_dsh_gate_…`; `::a_codex_seat_argv_that_selects_a_session_is_refused_on_the_cold_path_too`; `::a_refused_resume_is_a_cold_spawn_with_the_refusal_journaled`; `::the_committed_plugin_set_…` | discharged |
| B20 | "pair its selector mutation with the existing protocol adapter test `a_refused_resume_is_a_cold_spawn_with_the_refusal_journaled`, verifying two children, no selector and the retained sandbox in the replacement argv, and one cold launch row" | 5131–5135 | That test exists, asserts two children, no selector, the retained sandbox and one cold launch row, and ran green | **partially discharged — evidence-verification gap.** The test is opened. "**pair its selector mutation with**" that test is a narrative record (F5). It closes on a unit-17 ruling that names it, or by unit 19 |
| B21 | "Each clause names its tests, requirement and observed failure/pass; retain all inherited fixes and leave 8.10 unchecked." | 5136–5138 | The delivery sections name tests and outcomes throughout; 8.10 is `[ ]` | discharged |
| B22 | "Only AU's Pass C repair is scheduled by the current clauses; Pass D remains pending and unscheduled. Verify each current case alongside 8.8(d); full 8.10 remains unchecked." | 5140–5144 | Pass D was later commissioned in three runs (`tasks.md` 535, 703, 879), which is the schedule this sentence anticipated | discharged |
| B23 | R2's transport repair: "use `serde_json` to serialize one complete checkpoint-data JSONL row per declared invocation, including that invocation's ID"; the shim emits its indexed row "as a `%s` argument in a fixed format"; "A missing row fails explicitly, never repeats the last checkpoint. Declare enough rows for panel re-entry" | 5145–5152 | `resume_tests.rs::a_missing_dsh_checkpoint_row_fails_the_shim_without_repeating` (second invocation emits no checkpoint and says `no checkpoint row` on stderr); `::an_offered_dsh_start_carries_the_recorded_home_at_the_panel_member` declares two rows and the panel is re-entered once | discharged |
| B24 | exercise the emitted bytes "with a representative Windows home and quotes, percent signs, backslashes and an embedded newline; decode JSON and assert exact fields and one JSONL frame per checkpoint. Treat that home as data, without creating a Windows-shaped directory on POSIX" | 5152–5157 | `::a_windows_shaped_dsh_home_survives_the_checkpoint_transport` — the home is `C:\Users\seat\AppData\%TEMP%\"quoted"\nnext` (backslashes, percent, quotes, embedded newline), exactly one `checkpoint` frame, and `data["transcript"]["home"]` decodes back unchanged. No directory is created | discharged — this is decision **0063 ruling 5**'s retained host-agnostic validation of *data*, run on Linux |
| B25 | "Preserve and run both existing tests `an_offered_dsh_start_carries_the_recorded_home_at_the_single_site` and `…_at_the_panel_member` on their actual temporary homes" | 5157–5160 | Both read, both assert the three owned-target coordinates plus both originating members off the real `Start.input`; both ran green here | discharged, with recorded execution |
| B26 | "native macOS evidence stays pending controller CI; native Windows proof is withdrawn by decision 0063" | 5160–5163 | `docs/decisions/0063-windows-is-not-a-host.md` rulings 2–3 | **not this slice's** — the Windows half is **withdrawn by decision 0063** (rulings 2 and 3), neither debt nor executed proof. The macOS half is pending "controller CI" by the clause's own words, so it is externally owned and is not a commissionable unit; it is tracked at F7 |
| B27 | prove the originating-home carrier with "distinct old/new ID, locator, home, version and digest values plus another site's row, then **absent/mistyped** latest fields"; "A **missing/mistyped home or locator** never borrows from an older checkpoint or another site; an incompatible newest owner supplies no offer" | 5164–5174 | `resume_tests.rs::a_stamped_row_is_offered_only_to_its_own_site_owner_and_persistent_root` — `SITE_B` (another site), `OTHER_OWNER` (incompatible newest owner), the `coord` helper's distinct old/new five coordinates at 1295–1337, the newest-row-without-transcript case at 1285–1290 (`persistence_home: None`, `persistence_locator: None`), and the mistyped case at 1340–1372 | **partially discharged.** *Absent* home and locator are proved, and both read `None` rather than the older row's. *Mistyped* home and locator are **not**: in the mistyped journal only `harness_version` and `wrapper_digest` become `json!(7)` and `json!(8)` (`:1354–1355`); `locator` stays `"sessions/brokkr/newer"` and `home` stays `"/new/home"`, both well-typed strings. The clause names home and locator explicitly, so the mistyped half of its own sentence is undriven. See F9 and unit 3 (now 6), whose record at `3a2a6785` drives a mistyped `locator` (`9`) and `home` (`10`). The row keeps this grade until unit 22 regrades it on opened evidence |
| B28 | read actual `Start.input` at both production callers: "an offered start carries the exact five coordinates from its selected checkpoint… and a no-offer start carries no owned target. Directly constructing both context objects in a unit test does not establish their journal association." | 5175–5182 | The two `an_offered_dsh_start_carries_the_recorded_home_at_…` cases drive a real engine run and read what the logging driver received; the first attempt carries no `owned_target` | discharged |
| B29 | "Retain the two scans unless a failing case requires a B correction." | 5182–5183 | `eligible_offer` and `originating_root` both still exist and are exercised separately at `resume_tests.rs` 1238–1255 | discharged |
| B30 | assert the private carrier "is absent from rendered prompt/context **and is not copied as `resume_context` or `owned_target` into launch evidence**; retain the existing confirmed `root_session` and `transcript` fields" | 5183–5186 | `resume_tests.rs::an_offered_dsh_start_carries_the_recorded_home_at_the_single_site` 2026–2033 — `starts[1]["context"].get("owned_target").is_none()` and the same for `assessment` | **partially discharged.** The clause has three predicates. *Absent from rendered context* is proved, on the real `Start.input`. *Not copied into launch evidence* is **not**: both assertions read `starts[…]["context"]`, a start message, and nothing in the suite opens an emitted launch row. *Retain `root_session`/`transcript`* is not asserted in these two cases either. See F8 and unit 4 (now 7 and 14). **Unit 4a (§6, `3a80b1d8`):** both halves are now asserted. The journaled launch row of the offered attempt, on a DSH-shaped checkpoint at both call sites, keeps `root_session` and `transcript` exactly and carries no `resume_context`, `owned_target` or `assessment`. The adapter's emitted row does the same on its confirmed shapes. The assertions are landed; running them in the gate is unit 4's job (now unit 14). **Remediation grading (third return, finding 4):** still **partially discharged**. Unit 7 (4a) asserts all three predicates on real surfaces, but for the offered start in two places. The engine journals the launch row that the DSH-shaped `dsh_model_driver` shim emitted. The real adapter's confirmed and declined shapes are checked on its own wire in the protocol suite. No single process runs the production adapter's offered publication under the engine, and the one real-adapter engine case is closed-gate cold, which carries no `owned_target`. That half is unit 11's |
| B31 | "Preserve gate/no-offer behavior and Pass A's binding cases independently." | 5186–5188 | `::no_gate_topology_is_ever_offered_a_session`; `::every_work_topology_is_offered_its_own_session_and_no_other` | discharged |
| B32 | D10's counter at `dsh_seat_overlay_in` entry, "Reset it for every synchronous `dsh_launch_with` call and require one on positive plans to calibrate it; every pre-observation control/route refusal requires zero… Use no process-global counter, directory scan or new planner signature. Origin/storage declines can legitimately observe identities and stage one safe cold plan" | 5189–5197 | `adapters.rs:5301–5302` (`#[cfg(test)]` thread-local at entry); `reset_dsh_staging_calls()`/`dsh_staging_calls()` used in the residual, grammar, binding and path matrices — zero on refusals, one on positives, one on the origin/mismatch declines | discharged |
| B33 | exercise `dsh_launch_with` for exact authorized inputs "and every competing residual category" (the full named list); "Retain both existing effort spellings as positive cases, with no guessed aliases. Exercise cold, offered and disabled paths." | 5198–5206 | `::dsh_residual_and_joined_controls_refuse_before_any_observation` — the ledger covers duplicate/missing/invalid model and effort, equals-joined model, mixed effort spellings, effort without a model, bare/duplicate/joined/odd patch, session/new/resume/list/profile/workdir/output/settings controls, `--from-default-profile`, `--verbose`, unknown names, `--`, positional text and short/joined/clustered forms, driven over `("disabled", …), ("offered", …), ("cold", …)`; `::both_dsh_effort_spellings_are_admitted_and_stage_one_overlay` | discharged |
| B34 | "Pair bad controls with a route that would fail to read and require the control refusal first; record zero version calls, a never-called composite closure and no staged overlay or retained-root allocation on these failures." | 5206–5210 | The same ledger pins `--patch does-not-exist.yml` beside the rejected control and asserts `dsh_staging_calls() == 0`, a marker-free version shim and a panicking/counted producer | discharged |
| B35 | "Include synthetic private markers in rejected option names, equals-joined values, malformed model/path/selector-control values and odd patch spellings; assert no diagnostic echoes them." | 5209–5212 | `MARK = "zzz-9f31c7-marker"` (a valid model id, so no control refusal may name it) carried in every vector; the loop asserts no diagnostic contains it | discharged |
| B36 | the engine-side route cases belong to `resume_tests.rs` "in the pattern of the private-context unit case", built through that suite's `bundle` helper, with a dsh site carrying one `--patch`, the logging driver, the planted file, read off `Start.input` "at both production `start_context` call sites… and for the positive case on a cold start, an offered start and a start under an `unmeasured` assessment alike" | 5213–5229 | `resume_tests.rs` 1640–2379: `overlay_digest`, `patched`, `route_start`, `route_binding`; the cold positives at 1687 (single site) and 1731 (panel member), inline seats whose assessment is `unmeasured`; the offered positives at 2267 (single site, a retry) and 2331 (panel member, a re-entry — unit 2d); the withholding cases at 1784, 1883, 1940, 2061, 2108; the changed-bytes cases at 2165 and 2216 | discharged (corrected by unit 2d). The first grading read `discharged` while the only offered positive went through `single(...)`, so no case showed a binding on an offered **panel** start; `a_valid_route_overlay_binds_on_an_offered_panel_member_start_too` reads both of the member's starts off the real `Start.input` and asserts the same `{value, digest}` on each |
| B37 | "(i) A valid leaf-manifest member carries the actual argv value and that member's compiled manifest digest." | 5229–5231 | `::a_valid_route_overlay_binds_at_the_single_site` and `::…_at_the_panel_member` — `binding["value"] == "recipe/route.yml"`, `binding["digest"] == overlay_digest(bytes)` | discharged |
| B38 | "(ii) A same-shaped nonmember outside the layer, a working-directory shadow…, an ancestor-layer file, a `..` component and an in-layer symlink whose target resolves outside the layer directory while remaining inside the working directory (**present at compilation and therefore a `files` member**), and the absolute path produced by `./` expansion each receive no binding… **at either call site**." | 5231–5237 | **Panel member:** `::a_non_binding_route_overlay_withholds_the_member_at_both_call_sites` (`resume_tests.rs:1784`) — six shapes (`nonmember`, `shadow`, `ancestor`, `traversal`, `absolute`, nonmember `symlink`); `::an_escaping_symlink_member_is_withheld_at_the_panel_member` (`:2108`, unit 2b). **Single site:** `::a_non_binding_route_overlay_withholds_the_member_at_the_single_site` (`:1883`, nonmember, unit 2a); `::every_remaining_non_binding_shape_is_withheld_at_the_single_site` (`:1940`, shadow, ancestor, traversal, absolute, unit 2d); `::an_escaping_symlink_member_is_withheld_at_the_single_site` (`:2061`, unit 2b). The `./` half is `bundle.rs:3689`, where a `./`-spelled command part expands to `dir.join(rel)` — an absolute path, which is the `absolute` shape | discharged (corrected by units 2a, 2b and 2d, and 2d's return) — every named shape receives no binding at each call site. Each case reads the real `Start.input`, asserts the context is present, and collects any shape whose context carries a `route_overlay` key, null included; the collection is empty. The escaping symlink is a compiled `files` member at both sites (2b). At both sites every vector names bytes a lookup could find, on a canonical root: the shadow and the ancestor file are spelled `route.yml` beside a member of that name, and the ancestor records its file in its own `files`. The `..` value (`../work/recipe/route.yml`) and the absolute expansion both resolve to the member. A compiling mutation of each rule parts that shape at each site. The first 2d grading read `discharged` while the panel's `..` value (`../escape.yml`) named no file, its ancestor's `files` was empty, and its assertion accepted a null or value-less binding; the `..` and ancestor-fallback mutations left that case green. 2d's return closed all three |
| B39 | "(iii) A member whose bytes changed after compilation cannot authorize its new bytes: the carried digest is the manifest's recorded value, not a hash of the resolved file, **at either call site**." | 5237–5240 | `::a_changed_route_overlay_member_carries_the_manifest_digest` — asserts the manifest digest and `assert_ne!` against a hash of the changed bytes | **partially discharged** — its seat is `single(…)`, so the case is proved at the **single site** only; the panel member's changed-bytes variant is not driven. The mirror of B38's gap. See unit 2a (now 2), whose record at `17b5b4d2` adds `a_changed_route_overlay_member_carries_the_manifest_digest_at_the_panel_member`. The row keeps this grade until unit 22 regrades it on opened evidence |
| B40 | the adapter suite owns the rest: `dsh_launch_with` "reusing the existing `route_overlay.rs` reader vectors and the shipped `recipes/research-dsh` overlay as the positive vector on qualified cold, qualified warm and disabled cold, plus identity-mismatch cold… asserting each planned command, exactly one `--patch`, one staging call, unchanged reasoning-level rows and the route rows before every Rust-owned… row" | 5240–5248 | `adapters/tests.rs:12348` reads the shipped `recipes/research-dsh/drivers/research-web.yml`; `::dsh_positive_planner_paths_fold_the_shipped_route_ahead_of_rust_owned_rows` asserts one `--patch`, `dsh_staging_calls() == 1`, one `reasoningEfforts:` and the row ordering on all four outcomes | discharged |
| B41 | "Include declared-composite mismatch without an offer (no refusal token) and originating-identity mismatch with an offer (`unverified-harness`), with the version/producer counts appropriate to each comparison." | 5248–5252 | Same test 12504–12551 — the mismatch case asserts `refusal.is_none()` and one staging call; the origin case asserts `Some("unverified-harness")`, cold, one staging call | discharged |
| B42 | "The resulting overlay is the observation; the existing helper-only ordering test and synthetic `contains` assertions do not discharge it. **Keep the composite, launch row and journal free of route bytes.**" | 5251–5254 | `assert_dsh_planner_overlay` (`adapters/tests.rs:12381–12400`) reads `launch.overlay.path()` off disk and compares row indices; `::the_shipped_route_overlay_folds_ahead_of_the_rust_owned_rows` is retained beside it, not in place of it | **partially discharged.** The first sentence is discharged — the observation really is the written overlay. The second is not: no opened case asserts the composite, an emitted launch row or a journal envelope free of route bytes. See F8 and unit 4. **Unit 4a (§6, `3a80b1d8`, `a41ae69e`, `9a3c0746`):** the second sentence is now proved at the composite, and at the launch row and the journal, stderr tail and non-protocol stdout included, for the shipped route under the real adapter; the journal half rests on `9a3c0746`, as in A53. For the gated shapes the journal half **stays partial**, as in A53. That half is unit 11's, and the earlier "unit 4" is unit 14 |
| B43 | "Run each negative on cold, offered and disabled planning, requiring a pre-staging refusal naming a depth, field or URL part and never a value." | 5254–5258 | `::dsh_route_grammar_matrix_refuses_before_staging_on_every_planner_path`, `::dsh_route_binding_matrix_…`, `::dsh_route_overlay_path_refusals_precede_any_probe_or_staging` — each loops `("disabled", …), ("offered", …), ("enabled", …)` and asserts zero staging, zero producer calls, no version probe and no echoed value | discharged as the rule; its per-class coverage is graded at B44–B50 |
| B44 | "a second or bare `--patch`" | 5258–5259 | The residual ledger's `duplicate patch`, `bare patch`, `joined patch`, `odd patch spelling`, `flag-shaped patch value` and three "patch claiming a later control" vectors, all three planner paths, `Field("--patch")`; plus the splitter-level `::a_second_bare_or_odd_patch_is_refused_by_arity` | discharged |
| B45 | "an absolute, `..`, symlink-escaping, non-regular, oversized or non-UTF-8 path" | 5259–5260 | `::dsh_route_overlay_path_refusals_precede_any_probe_or_staging` drives symlink-escape, non-regular, oversized and non-UTF-8 on all three paths. Absolute and `..` values receive no engine binding (B38), so at the planner they arrive as "a `--patch` with no bound route overlay", which `::dsh_route_binding_matrix_…` drives on all three paths; their direct spelling refusal is `route_overlay.rs::claim_refuses_absolute_traversal_and_escaping_values` | discharged — jointly, as the clause's own "proven end to end by the two suites together" allows |
| B46 | "an absent binding beside a present `--patch`, a binding without `--patch`, a disagreeing binding, and a bound digest the bytes read do not hash to… each of those five refusals is proven end to end by the two suites together and never by adapter cases in place of engine ones" | 5259–5266 | `::dsh_route_binding_matrix_refuses_before_staging_on_every_planner_path` — its `Case` list carries `digest before shape`, the absent binding, the binding without `--patch` and the disagreeing binding, each on three paths; the engine half is B38/B39 | discharged |
| B47 | "a Rust-owned or foreign row ID, **a second entry or provider**, a provider the seat did not pin, an absent model pin or one without a provider segment" | 5266–5269 | The planner matrix's vector labels are exactly `foreign row` (`adapters/tests.rs:12593`), `provider not pinned` (`:12598`), `model not pinned` (`:12603`) and `second provider` (`:12608`). **Absent model pin** and **a model without a provider segment** are proved only at the reader (`route_overlay.rs::a_route_needs_a_model_pin_and_model_item`, `::a_bound_route_needs_a_pin_a_nonempty_value_and_a_readable_workdir`), and `route_overlay.rs:111–115` is the production refusal. A **second entry** vector exists at neither the reader nor the planner | **partially discharged.** Six classes are named. Four run the three planner paths. Two (absent model pin, segment-less model) are reader-only, which line 5285 says is not evidence for the launch paths. One — **a second entry** — has no vector anywhere; the first cut read the clause's "a second entry or provider" as one class and credited `second provider` for both. See F2 and unit 2c (now 5, with 2c-fix as 4), whose records at `289d9c5b` and `7b26d186` claim every class on all three paths. The row keeps this grade until unit 22 regrades it on opened evidence |
| B48 | "each field outside the closed six-field set, a literal authentication header beside a valid `apiKeyEnv`, and a missing or non-name-shaped `apiKeyEnv`" | 5269–5272 | The grammar matrix's `field outside the set`, `literal auth header`, `missing apiKeyEnv`, `malformed apiKeyEnv` — all three paths | discharged |
| B49 | "each `baseURL` grammar breach (userinfo, query, fragment, percent-escape, backslash, whitespace, brackets, **non-ASCII**, empty segment, invalid host label or port, `http`, uppercase scheme, schemeless)" | 5272–5274 | `route_overlay.rs::the_endpoint_grammar_decides_the_positive_and_every_refusal`, `:692–718` — its thirteen vectors read, in order: `user:pass@`, `?api_key=1`, `#frag`, `%2f`, `host\x`, `/ space`, `[::1]`, `//x`, `http://`, `HTTPS://`, `host/x`, `-host`, `:123456`. The planner matrix drives **three** (`http endpoint`, `query endpoint`, `backslash endpoint`) | **partially discharged, worse than first recorded.** Ten of the thirteen reader vectors never run the three planner paths, and line 5285 forbids reading helper success as evidence for them. Beyond that, the clause names **fourteen** breaches and the reader enumerates thirteen: **non-ASCII is absent from the reader's own list**, so that breach has no evidence at either level. The first cut said "all thirteen" and matched the count instead of the names. See F2 and unit 2c (now 5), whose record at `7b26d186` claims the ten and a non-ASCII host at both levels. The row keeps this grade until unit 22 regrades it on opened evidence |
| B50 | "tabs, control characters, document markers and executable or unrecognized syntax at any depth in either representation (a `!!js` or other tagged scalar, a `__jsExpr` mapping, a flow collection, anchor, alias, merge key or block/quoted scalar)" | 5274–5277 | The planner matrix drives `tagged scalar`, `flow collection`, `anchor` and `merge key`. `route_overlay.rs::executable_or_unrecognized_syntax_is_refused_at_any_depth` and `::the_reader_refuses_every_lexical_shape_it_did_not_recognize` cover `__jsExpr`, alias, block scalar, quoted scalar, tabs, control characters and document markers — at the reader only | **partially discharged** — seven of eleven lexical shapes are reader-only, the same gap as B49. See unit 2c (now 5), whose record at `7b26d186` claims all seven on the three paths. The row keeps this grade until unit 22 regrades it on opened evidence |
| B51 | "Prove the binding and digest check run before any shape check: bytes invalid on both digest and shape axes must yield the digest refusal first, while a bound, digest-matching member with an invalid `baseURL` yields its grammar refusal." | 5277–5281 | `::dsh_route_binding_matrix_…`'s first vector (`digest before shape`: an `apiKeyEnv` of `9LIVE` bound to the VALID bytes' digest) and its `shaped` vector (a digest-matching member with `baseURL: http://host/x`); `::a_dsh_route_overlay_planner_checks_the_digest_before_the_shape_and_before_staging` | discharged |
| B52 | "assert zero entries to the calibrated stager and zero version/producer calls, with a fixed depth/field/URL-part reason and no synthetic private marker echoed. An error or absent retained directory alone does not establish that nothing was staged." | 5281–5285 | All three matrices assert `dsh_staging_calls() == 0`, `calls.get() == 0` and `!marker.exists()` per vector per path | discharged |
| B53 | "Helper-reader success is not evidence for these three launch paths." | 5285–5286 | This sentence is what makes B47, B49 and B50 partial rather than discharged | discharged as the grading rule this ledger applies |
| B54 | "Keep one resulting patch, unchanged reasoning levels and current route rows before Rust-owned rows on each positive path, including identity-mismatch cold." | 5286–5289 | `::dsh_positive_planner_paths_fold_the_shipped_route_ahead_of_rust_owned_rows` — qualified cold, qualified warm, disabled cold, declared mismatch and origin mismatch, each with one `--patch`, one `reasoningEfforts:` and the asserted row order | discharged |
| B55 | "Retain the existing engine privacy assertions; B's plan is not an emitted launch." | 5288–5290 | B30 (retained, and re-graded there); `dsh_launch_with` returns a `DshLaunch` value, so the planner suite has no launch row to inspect | **partially discharged.** "Retain the existing engine privacy assertions" rides B30, now partial. The first cut also claimed "the tests assert no launch row" — that is unsupported and withdrawn: the tests do not assert the *absence* of a launch row, they simply never emit one, which is a property of the seam, not an assertion. **Unit 4a (§6, `3a80b1d8`, `a41ae69e`):** the existing engine privacy assertions are kept and extended, and emitted launch evidence is now read, both at the wire and as journaled. The assertions are landed; running them in the gate is unit 4's job (now unit 14). *(Its grade still reads partial because "Retain the existing engine privacy assertions" rides B30, and B30 keeps unit 11's half.)* |
| B56 | the gate-before-probe matrix: "missing/unmeasured/unsupported assessments and missing accounting evidence (`unsupported-resume`), incompatible boundary/hands (`restrictions-unavailable`), missing/mistyped applicable identity and absent/mistyped/malformed declared digest (`unverified-harness`)" | 5291–5295 | `::a_closed_dsh_gate_reaches_neither_probe_nor_producer_and_keeps_the_cold_route` — `absent`, `unmeasured`, `unsupported`, `missing-accounting`, `restrictions`, `hands-mismatch`, `no-identity`, `mistyped-identity`, `no-declared-digest`, `mistyped-declared-digest`, each × offer and no-offer, each asserting its exact token; the *malformed* declared digest (`"A"×64`) is driven in `::a_dsh_identity_mismatch_…` 10492–10508 | discharged |
| B57 | "Use a recording version shim and a panicking or counted composite closure; assert zero calls to both. A nonexistent executable alone does not prove no version attempt." | 5295–5298 | `dsh_recording_version_shim` (`:10218`, touches a marker file on every invocation) and `Cell`-counted or `panic!`ing closures, in every gate case | discharged |
| B58 | "Assert the complete shipped cold argv and overlay, not merely `stream_json == false`: no `--new`, `--session` or `--output-format`, no rejoining target or offerable-root claim, and a refusal token only when an offer was declined." | 5298–5301 | Same test 10351–10383 — `command[0]`, `command[1..4] == ["--profile","headless","--patch"]`, exactly one `--patch`, none of the three selectors, and the overlay's transcript/compression/model rows read off disk | discharged |
| B59 | "independently exercise matching observations, absent/malformed/unreadable version output, version-command failure and version drift, producer error and canonical-composite mismatch. A matching version invokes the sole producer once; earlier failures invoke it zero times." | 5302–5306 | `::a_dsh_identity_mismatch_declines_the_offer_and_keeps_the_cold_route` 10406–10473 — producer error, composite mismatch, cold `exit 3` and banner, drift with `calls == 0`; on the offer (units 3b-fix, 3b) absent, malformed, unreadable and matching-then-`exit 3` output, each declining `unverified-harness` with `calls == 0`; and `::a_qualified_dsh_launch_…` for the matching case | discharged |
| B60 | "**independently** vary originating **version and digest** through missing, mistyped, malformed and different values; these declines may follow one producer call" | 5306–5308 | Same test, `adapters/tests.rs:10527–10556` — the loop's three labels are exactly `("originating version", "version")`, `("originating digest", "digest")` and `("missing originating digest", "null")`, mutating to `"0.1.4-rc.1"`, `"d"×64` and `Value::Null` | **partially discharged.** Two fields × four values = eight vectors required; three exist. Driven: different version, different digest, **missing digest**. Undriven: **missing version**, mistyped version, malformed version, mistyped digest, malformed digest. The first cut listed the gap as two (mistyped, malformed) by treating `Value::Null` as covering both fields' missing case; it covers the digest's only. `adapters.rs:1097–1110` reads both through `Value::as_str`, so the undriven five collapse onto the driven two in behaviour — the independent variation the clause asks for is what is missing. See F3 and unit 3 (now 6), whose record at `3a2a6785` varies each field through absent, null, mistyped, malformed and different. The row keeps this grade until unit 22 regrades it on opened evidence |
| B61 | "Require `unverified-harness`, the exact shipped cold route under the current home and no offerable root on any failed qualification. No-offer mismatch has no refusal token. Observed version/digest must not be replaced by requested pins." | 5308–5311 | The same declines assert `refusal`, `!stream_json`, `rejoining.is_none()`; `assert_shipped_cold_command` in `::every_dsh_component_drift_…`; `drifted.observed == Some("9.9.9")` | discharged |
| B62 | "Retain the producer's existing component-drift suites; extending the full composite/doctor/adaptation matrix below belongs to D." | 5311–5314 | `composite/tests.rs::the_measured_composite_moves_with_every_component_it_names` retained; D's matrix is rows B82–B102 | discharged |
| B63 | owned storage: "missing/mistyped provider ID, locator or home, ID disagreement, a missing/unresolvable/different home, and two homes holding the identical ID/locator must decline to cold in the current home without reading the other store" | 5315–5319 | `::a_dsh_offer_requires_the_complete_recorded_address_and_a_bounded_locator`; `::dsh_storage_refusals_name_their_field_and_never_the_path_they_tried`; `::a_dsh_retained_root_refusal_names_its_field_and_never_the_home`; `::owned_dsh_root_refuses_a_persistence_home_that_does_not_resolve`; `adapters.rs:3766–3800` | discharged |
| B64 | "A symlinked spelling of the same canonical home is a **positive** case." | 5319–5320 | `adapters/tests.rs::a_dsh_offer_requires_the_complete_recorded_address_and_a_bounded_locator`, `:11277–11293` — "A symlinked spelling of the same canonical home is equivalent": a `home-link` symlink to `dir.path()` is passed as `persistence_home`, and the plan asserts `linked.stream_json && linked.rejoining.as_deref() == Some("session-1")`; `adapters.rs:3794–3800` canonicalizes both before comparing | discharged — **on a corrected citation.** The first cut cited `::resolve_dsh_root_refuses_an_unreadable_admitted_home`, which is an absent-home *negative* (`adapters/tests.rs:13245–13250`), and the locator alias control, which compares no homes at all. Neither proves a positive. The correct positive was in the tree all along |
| B65 | "Cover empty, absolute, traversal, non-directory, missing and escaping locators; offered and planned locator round-trip at 80/81 Rust characters, including R4's eligible 80-character locator whose UTF-8 encoding exceeds 80 bytes, counting its path prefix." | 5320–5323 | `::dsh_owned_locators_resolve_only_beneath_the_home_and_name_the_offered_root` (each by its own production reason); `::an_eighty_character_multibyte_dsh_locator_round_trips_through_warm_planning` (`sessions/brokkr/` + 64 `é` = 80 chars, 144 bytes); `::a_dsh_overlong_locator_is_never_truncated_into_another_valid_root` (80 admitted, 81 refused by the bound's own reason) | discharged |
| B66 | "Drive that offer through warm planning and assert the exact original output locator, original root, warm argv and no replacement allocation; pass the planned value through the existing `Transcript::record` clamp and assert it remains unchanged, without a provider launch." | 5323–5327 | Same multibyte test — `launch.locator == locator`, `launch.root == dir.join(&locator)`, one `--session`, no `--new`, one staging call, and `transcript.record(&locator, …)` leaves `meta["transcript"]["locator"]` equal | discharged |
| B67 | "Retain the 81-character refusals and an overlong value whose 80-character prefix names another valid root, which must never be selected by truncation. This positive case must fail if the admission bound is changed from `chars().count()` to byte length" | 5327–5331 | `::a_dsh_overlong_locator_…` — the 80-character prefix IS a planted valid root, the 81st character refuses by the bound's reason, and the overlong address names nothing on disk; the multibyte positive is the `chars()`-vs-bytes control | discharged |
| B68 | "No lossy conversion or separator rewrite repairs an address." | 5331–5332 | `::dsh_owned_locators_…` drives a component carrying the separator the shared clamp rewrites and asserts its own refusal | discharged |
| B69 | "Include project-directory, session-directory and `session.v3.jsonl` symlink escapes, including an escape to another root within the same home; contained aliases remain admissible." | 5332–5335 | `::dsh_unsafe_stored_candidates_decline_instead_of_being_skipped`; `::dsh_owned_locators_…`'s symlinked escape with its contained control | discharged |
| B70 | "Require one matching valid depth-zero header; missing, nonregular, unreadable, malformed/truncated, delegated or ambiguous stored evidence cannot qualify." | 5335–5337 | `::a_session_file_whose_first_row_is_not_the_header_is_unreadable`; `::a_delegated_sub_session_never_becomes_the_one_the_seat_reports`; the ambiguity case in `::dsh_owned_locators_…` | discharged |
| B71 | "Exercise each finite enumeration/header/sequence budget **at its admitted limit and beyond**, read/iteration failure, malformed or truncated boundary data and a valid prefix followed by invalid data: decline rather than skip an unsafe candidate, use a partial maximum or default to zero." | 5337–5342 | `::dsh_admission_reads_are_complete_within_their_bounds_or_decline` (`adapters/tests.rs:11673–11790`) and `::dsh_stored_sequences_decline_instead_of_reporting_a_partial_maximum` (`:11793–11858`), both read line by line; plus `::owned_dsh_root_refuses_a_store_whose_sequence_cannot_be_read` and `::a_retained_project_entry_the_reader_cannot_yield_is_a_bounded_refusal` and its session-level sibling | **partially discharged.** Budget by budget: **header** — both sides, a 16-byte budget declines and `DSH_HEADER_LIMIT` admits, with an exact-at-limit positive whose newline lands on the last byte (`:11692–11701`); **enumeration** — both sides, `dsh_session_file_with(&root, &id, 1).is_err()` beside `(…, 2).is_ok()` (`:11773–11774`); **session-file cap** — refusal only, `set_len(DSH_SESSION_FILE_LIMIT + 1)` (`:11780`), with no file at exactly the cap admitted; **per-row sequence** — refusal only, `dsh_session_last_seq_with(…, 4)` returning `None` (`:11829`), with no row admitted at exactly its injected budget. The failure, malformed, truncated and valid-prefix halves are fully driven. Two of four budgets lack the clause's positive side. See F10 and unit 3 (now 6), whose record at `3a2a6785` adds both admitted-limit positives. The row keeps this grade until unit 22 regrades it on opened evidence |
| B72 | "Assert no other store changes, history copies or substitute-root search; the sole safe cold plan may allocate its own fresh root, never reuse the refused one." | 5341–5343 | `::a_retained_dsh_directory_alone_never_supplies_a_provider_handle` — a complete readable retained root with nine stored sequences still launches `--new` with a freshly allocated root when the offer names nothing, and an id with no recorded address is declined the same way; the control rejoins at sequence nine | discharged |
| B73 | "Put synthetic private markers in invalid offered IDs and stored addresses and assert bounded storage diagnostics never echo them." | 5343–5345 | `::dsh_storage_refusals_name_their_field_and_never_the_path_they_tried`; `::a_dsh_transcript_root_refusal_names_its_field_and_never_the_root` | discharged |
| B74 | "These are pre-spawn admission cases; current-event folding and accounting remain D." | 5345–5347 | Reading rule. Folding and accounting are **9.6**'s (5508–5520) and Pass D's fold work; this row assigns them away | discharged (scope rule) — the folding debt it points at is F4 |
| B75 | "Finish with exact qualified cold/warm argv and overlay assertions… with warm selecting the original root rather than allocating a replacement. Preserve current-directory conformance and `confirms_from_locator: false`." | 5348–5353 | `::a_qualified_dsh_launch_uses_the_stream_json_forms_and_records_observed_identity`; `::a_warm_dsh_offer_names_the_owned_root_and_folds_past_its_sequence`; `::an_eighty_character_multibyte_…` (original root); `adapters.rs:3500` | discharged |
| B76 | "A built-driver conformance case in `crates/brokkr-cli/tests/driver_conformance.rs` is added only if it proves a distinct planner observation; no child-confirmation acceptance is added in B." | 5353–5356 | `driver_conformance.rs` holds 24 cases; the DSH ones (`::the_dsh_admission_rule_reads_the_whole_payload_the_command_line_carried`, `::a_residual_terminator_refuses_on_every_dsh_path_wherever_it_stands`, `::dsh_overlay_staging_is_observed_without_any_child_execution`, `::a_dsh_deadline_kill_flushes_no_held_launch_row_and_starts_no_replacement`) each observe the built driver, not a child confirmation | discharged |
| B77 | "The current proofs require actual synthetic children through production terminal handling, the immutable address/occurrence census, permanent refusal, both endings with exact LE3 reasons and no publication/replacement, completed-observation acknowledgment for transient evidence and each regression/ending's compiling removal/restoration." | 5357–5364 | Rows A60, A61, S1–S9 | **partially discharged — evidence-verification gap.** Children, census, permanent refusal, both endings, exact LE3 reasons, no publication or replacement, and completed-observation acknowledgment are all opened and green. "each regression/ending's **compiling removal/restoration**" is narrative (F5) |
| B78 | "D7's additional watcher controls do not open Pass D." | 5364–5365 | Pass D was separately commissioned and separately delivered (D1–D3) | discharged |
| B79 | the seven inherited DSH-arm cases (confirming child; exit before any init; mismatch + clean exit and mismatch + delivered result; nonzero exit with prose but no machine-readable rejection shape; cancellation or deadline expiry with the hold open; the one local-decline path permitting exactly one safe cold launch) | 5365–5384 | `::a_qualified_dsh_child_confirms_the_root_and_folds_current_only`; `::a_qualified_stream_json_launch_finishes_its_held_row_without_a_confirmation`; `::a_dsh_root_mismatch_that_delivers_a_result_is_still_a_mismatch`; `::dsh_stderr_prose_and_a_nonzero_exit_start_no_cold_replacement`; `::a_dsh_deadline_kill_inside_the_open_launch_hold_fabricates_nothing`; `::a_cancel_reaching_the_dsh_driver_publishes_nothing_and_launches_nothing`; `::an_unsupported_dsh_offer_takes_exactly_one_independently_safe_cold_launch` | discharged |
| B80 | "Label these the DSH arm of LE1/LE3/AS4/D7 rather than a restatement of 7.9's or 9.7's generic cross-adapter coverage" | 5384–5386 | Those tests drive a DSH child's init event; `::conformance_across_all_builtin_adapters` (9.7's) drives no DSH child-process init event | discharged |
| B81 | "Pass D owns completion of the remaining composite/containment/doctor/adaptation-bytes/retained-storage matrix; keep its passing cases without extending that matrix in B." | 5387–5389 | D1/D2/D3 delivered it (`tasks.md` 535, 703, 879); rows B82–B102 grade it against source | discharged |
| B82 | "Prove core, Node, dependency, plugin, patch, composed-profile or optional extension drift yields `unverified-harness` before provider work." | 5389–5391 | `adapters/tests.rs::every_dsh_component_drift_declines_the_offer_before_any_provider_work` — a readable synthetic install whose offer is HONOURED first, then all seven drifts one per fresh install, each through the production `dsh_composite`, each `unverified-harness`, cold, no rejoin, no declared identity recorded, and the shim's log holding exactly `["--version"]` | discharged |
| B83 | "Pin the canonical composite's byte form with one worked vector per lock dialect (the npm lockfile-3 hidden lock and pnpm lockfile 9.0, as committed synthetic excerpts in the measured grammar)." | 5391–5394 | `composite/tests.rs::the_worked_vectors_pin_the_canonical_composite_byte_form` — `WORKED_CANONICAL` (the producer's own output) and `WORKED_CANONICAL_STREAM` (a frozen literal of the `<component>\0<value>\n` stream, written out rather than assembled, guarded by `::no_test_reassembles_the_component_stream`) | discharged |
| B84 | the npm vector's key spellings: "top-level and nested unscoped names, scoped names under unscoped and scoped parents, scoped-parent/unscoped-terminal keys, no `name` fields and a conflicting optional `name` field" | 5394–5397 | `::the_worked_npm_vector_binds_every_key_spelling_to_its_terminal_package` — every spelling, with the census of `name` fields asserted so the conflicting one cannot be quietly lost | discharged |
| B85 | "at least three package groups, for example `node_modules/a/node_modules/@parent/b/node_modules/@scope/child`, with version and integrity distinct from shallower `@scope/child` entries" | 5397–5403 | Same test — the three-group key with its own version and integrity, distinct from both shallower entries | discharged |
| B86 | "Also refuse `node_modules/a/extra/node_modules/@scope/child`, whose valid terminal package cannot excuse its malformed intermediate group." | 5403–5405 | `::a_malformed_intermediate_group_refuses_the_whole_worked_lock` | discharged |
| B87 | "The hidden lock is the sole npm source: a missing hidden lock is unreadable even if a valid root `package-lock.json` is present." | 5405–5407 | `::the_root_package_lock_is_never_read_beside_or_instead_of_the_hidden_one` — the refusal carries the hidden lock's component, locator and the host's own `io::Error` wording | discharged |
| B88 | "Assert exact normalized value bytes, equality with equivalent pnpm entries, complete-triple deduplication and retention of same-name entries with different versions or integrities." | 5407–5409 | `WORKED_DEPENDENCIES` (`composite/tests.rs:10071–10085`), opened and counted: **fourteen values over eight distinct names** — `@pnpm/only`, `@scope/child`, `debug`, `dsh-plugin-cli-session`, `dup`, `plain`, `split`, `versions`; `::equivalent_npm_and_pnpm_entries_compose_to_one_dependency`; `::npm_three_group_and_dedup_vectors_retain_distinct_triples` | discharged — **on a corrected count.** Both previous cuts said "fourteen values over ten names". Two earlier reviews (`8d9f4a9e`, `5c0333bc`) had already reported eight, against `tasks.md:572–578` and the tests' own comments, and this ledger repeated the wrong figure anyway. The literals and the assertions are sound; only the prose was wrong, here and in the three places those reviews named |
| B89 | "Empty, absolute, incomplete, traversal, backslash or trailing-separator keys and missing, non-string, empty or whitespace-bearing versions are unreadable." | 5410–5412 | `::malformed_npm_keys_are_refused`; `::npm_keys_with_an_empty_or_dotted_component_are_refused`; `::npm_locks_reject_unparseable_and_incomplete_entries`; `::npm_versions_with_any_whitespace_are_unreadable` | discharged |
| B90 | "one vector for the plugin component's bytewise path order, the exclusion of the plugin's own tarball entry, and an equal composite for the same pair staged in two homes at different absolute paths and under different per-seat overlays" | 5412–5415 | `::the_worked_plugin_vector_pins_the_bytewise_path_order_of_the_component` (six files staged in REVERSE declared order, key sequence asserted); `::the_plugin_s_own_tarball_record_leaves_and_its_registry_namesake_stays`; `::one_pair_in_two_homes_under_two_seat_overlays_is_one_identity` (overlays staged through the production `dsh_seat_overlay_with`) | discharged |
| B91 | "also reach the same home through a symlinked ancestor and require equal plugin/composite values when the same bundles resolve" | 5415–5417 | `::the_dsh_composite_accepts_a_symlinked_home_ancestor` | discharged |
| B92 | "Exercise general bundle, plugin and synthetic conditional-extension containment against the canonical profile, and prove the original lookup anchor/order is retained." | 5417–5420 | `::the_bundle_search_order_is_core_ancestors_then_globals_then_the_profile` (four steps, each position holding a candidate outside both canonical roots); `::the_plugin_and_extension_must_resolve_inside_the_profile` | discharged |
| B93 | "An unresolvable profile boundary, a symlink target outside the allowed canonical roots, a near-prefix sibling such as `headless-extra`, and an outside first hit with a later inside candidate must each stay unreadable; no fallback or weakened comparison cures the false refusal." | 5420–5424 | `::containment_compares_canonical_components_not_string_prefixes` (the `headless-extra` sibling); `::a_bundle_directory_that_is_a_symlink_is_judged_where_it_lands`; `::an_outside_first_bundle_hit_is_not_skipped_for_a_later_inside_one`; `::a_bundle_candidate_that_cannot_be_inspected_stops_the_search` | discharged |
| B94 | "These cases add no provider support or real extension." | 5424–5425 | No `extensions/dsh/resume-policy/`; `adapters/dsh.json` untouched | discharged |
| B95 | "A profile bundle added, dropped or reordered, a changed `patchReload` and an added home-level `cordis.patch.yml` each move the composite, and a rewritten `cordis.yml` does not." | 5425–5428 | `::a_profile_bundle_added_dropped_or_reordered_moves_the_composite` — six cases, six LITERAL recorded canonical digests, none colliding (the test-side stream oracle the returned review struck is gone, `tasks.md` 1044–1057); `::a_rewritten_generated_cordis_yml_leaves_the_composite_untouched`, with the profile's own `cordis.patch.yml` as the moving control | discharged |
| B96 | "A listed bundle resolving outside the core root and the profile…, an executable that is not the core's `env node` script, a missing or malformed `bundles` or `patchReload`, and each unrecognized pnpm construct (tab, comment, document marker, block-form, missing or repeated resolution, key without a version separator) make the identity unreadable." | 5428–5434 | `::the_bundle_search_order_…` steps 1, 2 and 4; `::the_core_executable_must_be_lib_bin_js_with_the_exact_shebang`; `::the_profile_manifest_names_a_missing_mistyped_and_invalid_member_apart` (absent/string/object/null/number `bundles`; absent/number/null/empty/array `patchReload`; a well-formed value outside the closed pair); `::the_unrecognized_pnpm_constructs_refuse_through_the_producer` | discharged |
| B97 | "Every case builds its homes in temporary directories; no test reads `.forge/` or needs an installed provider." | 5434–5435 | `FixtureRoot` (`composite/tests.rs:140–157`) canonicalizes the temporary root once — the macOS `/var`→`/private/var` correction the returned review required (`tasks.md` 1075–1087); no `.forge/` path appears in either suite | discharged |
| B98 | "Cover bounded locator round-trip and refusal of truncation, ambiguity, traversal and symlink escape; a retained directory is never a handle." | 5435–5437 | Rows B65–B72 | discharged |
| B99 | "keep 8.8's committed-bytes test pinning the repository-owned adaptation's exact six-file set against its provenance block" | 5437–5439 | Row A67 | partially discharged — see A67 and F1. Unit 1 has landed. The row keeps this grade until unit 22 regrades it on opened evidence |
| B100 | "Cover the conditional extension with synthetic absent and present sets, exact four-file path order, changed bytes, missing or extra files, symlinks and resolution outside the profile; absence emits no extension line." | 5439–5442 | `::the_conditional_extension_is_absent_or_composed_from_its_own_four_files` (`WORKED_PAIR_STREAM` with no `extension` line, `WORKED_TRIO_STREAM` with one); `::the_extension_walk_refuses_a_missing_extra_or_symlinked_member`; `::the_extension_s_local_record_leaves_only_when_the_extension_resolves`; `::the_plugin_and_extension_must_resolve_inside_the_profile` | discharged |
| B101 | "**If the extension is required**, also compare its committed set to its own provenance block through the same function." | 5442–5444 | No extension is required (row A63: 10.7 demonstrated no missing hook), none is commissioned, and B102 forbids creating one | **not this slice's** — conditional with no subject. It cannot be discharged and is not debt; it stays conditional on 8.8's 5035–5044 clause ever firing |
| B102 | "No speculative extension is created merely to exercise these cases." | 5444 | No `extensions/dsh/resume-policy/` in the tree | discharged |
| B103 | "Retain the exact resume argv and complete current class/model/effort cases for every other adapter, generated-fragment versus passthrough distinction, no ambient cold/gate continuation, nonpersistent refusal, identifier injection, unsupported hands and cold/resume inability to honour the class." | 5445–5449 | Claude: `::a_claude_resume_is_the_cold_argv_plus_exactly_one_owned_selector`, `::a_claude_argument_that_selects_a_conversation_refuses_before_any_provider_work`. Codex: `::a_codex_resume_carries_the_thread_the_class_and_the_prompt`, `::a_codex_resume_re_expresses_the_effort_pin_as_a_config_override`, `::a_codex_seat_argv_that_selects_a_session_is_refused_on_the_cold_path_too`, `::a_class_that_cannot_travel_spawns_cold_with_the_reason_journaled`. LaneTally: `::the_lanetally_wrapper_resumes_on_its_own_shape_and_its_own_binary` — the gap D3 closed: exact cold and warm argv with the wrapper at `argv[0]`, one owned selector, the class/model/effort fragment unaltered, a claude measurement that does NOT enable the wrapper's shape, unsupported hands, a forged identifier absent from the cold argv, and an ambient `--continue` refused by the COMPLETE expected reason cold and warm | discharged |
| B104 | "Label these deterministic planner/storage shims rather than live DSH compatibility or enforcement evidence." | 5449–5452 | Every Pass D delivery section says so in its own words (`tasks.md` 554–561, 725–731, 905–909); the sources carry the same labels | discharged |

## 5. Verification gaps and findings

These are the places where this ledger could not turn a clause into opened
evidence, or opened evidence and found less than the clause asks. **F13 is new
in this revision; F6 and F7 were rewritten because they were factually wrong,
and F2b and F5 were widened.** F6 and F7 are the two worth dwelling on: both
asserted that something had *never happened*, and the artefacts refuting both
were sitting in `.forge/results/` in this worktree while they were written.
*Third return:* F5, F6, F8, F11 and F13 are corrected again, and F12 records
why.

- **F1 — the delta digest is recorded, never recomputed (A67, B99).** `PROVENANCE.md:50`
  names `78256d2e114f7ae8caec22987c5793b7398018cd59cd24cf36e79d7be011a585`.
  No test recomputes it. This seat did, by hand, over
  `lib/index.js:253\n` + `-\tconst events = agent.session.events;\n` +
  `+\tconst events = agent.session.snapshotEvents(firstSeq);\n`, and it
  reproduces exactly. **Missing evidence, not missing behaviour.**
- **F2 — reader-only, and in two places no-level, coverage for the route-overlay
  classes (B47, B49, B50).** Ten of thirteen `baseURL` breaches and seven of
  eleven lexical shapes are proved at `route_overlay.rs`'s unit tests and never
  through the cold, offered and disabled planner paths; line 5285 says in as
  many words that this is not evidence for those paths. Two further classes have
  **no vector at either level**: a **second entry** (5267) and a **non-ASCII
  `baseURL`** (5273). The reader's thirteen endpoint vectors (`:692–718`) were
  miscounted against the clause's fourteen in the first cut — matching the
  count, not the names. For the reader-only ten and the model-pin pair this is
  **missing evidence, not missing behaviour**. For the second entry and the
  non-ASCII endpoint, nothing opened establishes the behaviour either way: the
  refusals are expected to exist, and the units that add the vectors are told to
  report a production finding rather than relax a vector if one passes.
- **F2b — the engine route shapes ride one call site, and one required shape is
  absent (A37, B36, B38, B39).** The positive binding is proved at both
  `start_context` call sites, but not on every start kind at each: the offered
  positive is a `single(…)` seat (`resume_tests.rs:1891`), so the
  offered-at-panel-member cell of B36's 2 × 3 matrix is empty.
  The six non-binding shapes are all panel members
  (`resume_tests.rs:1777`, despite the test being named
  `…_withholds_the_member_at_both_call_sites`), and the changed-bytes case is
  a single site. Each clause says "at either call site". `engine.rs:1372`
  computes the binding through one `route_overlay_binding` helper per site in
  a single loop, and both `start_context` callers (`:1667` the single site,
  `:1917` the panel member) consume that one value — so the call-site half is
  **missing evidence, not missing behaviour**. The symlink half is different in
  kind. The clause's vector is an in-layer symlink escaping the layer and
  "**present at compilation and therefore a `files` member**"; at
  `resume_tests.rs:1797` the manifest lists `route.yml` alone, so `link.yml` is
  a nonmember and is refused for being a nonmember. The member-that-escapes
  vector does not exist, and what it would prove — that membership alone never
  authorizes a path whose resolution leaves the layer — is the one shape in (ii)
  a nonmember cannot stand in for.
- **F3 — five of eight originating-identity vectors undriven (A42, B60).** The
  clause asks for *missing, mistyped, malformed and different* independently for
  *version* and *digest*. The suite drives different-version, different-digest
  and missing-digest. Missing-**version** is undriven, as are mistyped and
  malformed on both fields. `adapters.rs:1097–1110` reads both fields through
  `Value::as_str`, so the undriven five collapse onto the driven behaviour.
  **Missing evidence, not missing behaviour.**
- **F4 — a 9.6 behaviour gap, verified in production.**
  `adapters.rs:2044` `find_dsh_transcript` returns the FIRST depth-zero
  transcript the enumeration reaches; `names_the_seats_own_session` (`:2069`)
  tests only `type == "session"` and `delegationDepth == 0`, never the offered
  id. Beside an unrelated retained sibling a confirmed rejoin can tail the
  sibling's file and fold no current work. Recorded at `tasks.md` 519–524 and
  confirmed here by reading the function. **Missing behaviour**, owned by 9.6
  (5508–5513, warm retained-store integration).
- **F5 — removal proofs are records, not artefacts, and several clauses ask for
  the artefact.** Every mutation ledger (Pass C's fourteen at 465–480 — this
  said "twelve", corrected 2026-09-23 — D1's two,
  D2's seven, D3's six, the review's two) is a narrative of a compile-and-revert
  that leaves nothing in the tree. This ledger confirms no mutation survives
  (`git diff origin/main`), and that the cases those mutations aimed at exist,
  assert what is claimed and pass. It does **not** re-derive the failures.
  The rows that name a mutation in their own words therefore read **partially
  discharged — evidence-verification gap** rather than `discharged`. The list is
  larger than the previous cut's, because that cut applied the rule to the rows
  it had noticed and not to their neighbours (C2): **N5d, N7, N8, N9, N10, N11,
  A61, S1, S3, S4, S5, S6, S7, S8a–c, S9a–e, S10, B5, B16, B20 and B77**.

  **Third return (finding 2): two kinds of absence, kept apart.** A removal
  can be *recorded but unverified*: a delivery record names the mutation, the
  assertion that parted and the restored pass, and nothing in the tree
  re-derives it. Or it can be *never performed*: no record exists, because the
  control could not run. A ruling can accept the first kind. It cannot accept
  the second, because there is no observation for it to accept. Applied to the
  list above:
  - **N11 leaves the list.** Its positive and its two removals were never
    performed (§1, N11): every execution of the test took the `Err` arm and
    printed PENDING. Unit 16 owns them.
  - **N2, N4, N6a and N6b join it.** N2's two removals are recorded as M5/M6
    and N4's as M4 (`tasks.md` 1668–1670). N6a's heading-set removal is M8
    (12377), and N6b's five scope removals are D1–D5 (1920–1922, 2057–2058).
    The previous revision graded N6a and N6b `discharged` over them.

  The recorded-but-unverified class is therefore **N2, N4, N5d, N6a, N6b,
  N7–N10, A61, S1, S3–S7, S8a–c, S9a–e, S10, B5, B16, B20 and B77**:
  twenty-nine rows. Counted: nine N rows (N2, N4, N5d, N6a, N6b, N7, N8, N9,
  N10), one A row, fifteen S rows (S1, S3, S4, S5, S6, S7, S8a, S8b, S8c, S9a,
  S9b, S9c, S9d, S9e, S10, and no S2) and four B rows. The previous revision
  called its list "twenty-five" while it listed twenty-six. *(Fourth return:
  this sentence said "sixteen S rows" over the same fifteen names. The total
  of twenty-nine was right.)*

  **Fourth return (finding 1): a third kind, no record found.** B5 reaches
  tests whose controls no opened record describes (B5's inventory in §4). For
  those there is nothing for a ruling to accept, just as with N11. So they sit
  outside this class, and unit 21(a) performs them in every case. *(Fifth
  return: D10's absence binding left this third kind, because task 10.5's
  visit recorded it at `tasks.md` 10230–10233 (C1). Three more `4daaa7d2`
  tests joined it (C2).)*
  S6 deserves a note — its clause does not merely require a removal, it requires
  a *property of the observed failure* ("must fail the intended terminal
  assertion, **not time out**"), which no record can supply after the fact.

  **How these rows close (C5).** The previous cut named the gap and then
  excluded it from the work list, while still answering "yes" to 8.10 — which
  cannot both be true, since B5 is one of the rows. Three things are true at
  once and the ledger now says all three:

  1. **No clause asks this commission to replay the mutations.** `tasks.md` 1248
     says "**Adopt resolver removals without replay**", but that sentence
     belongs to 8.8.1.1 and covers the *resolver's* removals alone. The
     previous revision leaned on it for the whole class, and the third chief
     rightly rejected that: it resolves no other row's obligation.
  2. **The records are not nothing.** Each names a seat, a revision, a mutation,
     the assertion that parted and the restored pass. What they are not is
     evidence this ledger opened.
  3. Therefore the gap closes first by **decision**, and that decision is the
     operator's. Unit 17 puts it to them as one question over the whole class,
     so twenty-nine rows are not left permanently ambiguous. It is a
     precondition of unit 22, not a suggestion.

  If the operator rules the records sufficient **for every row the ruling
  names**, those rows become `discharged` with no code written. A ruling that
  names only some rows leaves the rest open. **If the ruling is no, labour
  follows, and it is bounded but not small** (third return, finding 3). The
  previous revision offered S6 and B5's terminal-body cases as the whole
  fallback. That cannot reach B16's four Codex bridge and conformance removals
  (`tasks.md` 5121–5125), B20's selector mutation (5131–5135), or B5's
  **every** new test (5098–5099) in the suites outside the terminal body.
  Units 18–21 enumerate the replays by suite, and each is a single visit.
  *(Fourth return, finding 1: the enumeration still missed THE PROOFS' tests,
  and some of their controls have no record at all. Unit 21(a) performs
  those in every case. Unit 21(b) replays the rest after a no.)*
- **F6 — strict OpenSpec has passed three times on this change's candidates, and
  both previous cuts misreported that (C8).** The first cut said the gate "has
  never run on a Pass C or Pass D candidate". The second cut corrected that to
  "the first strict validation ever recorded on a candidate", crediting the
  review seat's run on `d73d94d1`. **That correction was itself wrong.** Opened
  above: `openspec validate --all --strict` passed **17/17 on `4d6b15f3`** (the
  D1 candidate) and **17/17 on `69cac25d`** (D3's returned-review head; this
  line first said "the Pass C head", which was wrong), both recorded in
  `.forge/results/` in the b5a676bf run's worktree, before either claim was
  written. With `d73d94d1` that is three passes. *Third return:* the third
  chief reports two more, on `1e1c2f63` (the `004fcb8b` correctness result)
  and its own on `af89511a`, which makes five. Every dated seat refusal still
  stands, the remediation seat's included. Those reports were never the
  problem. The problem was reasoning from "this seat could not run it" to
  "nobody has". What remains true, and what actually keeps N12 and S11 open:
  **no seat has run the *complete* ordered list, `openspec` and
  `bundles/verify` included, on one candidate into a delivery record.**
- **F7 — the exact-coverage gate has run, and it FAILED (C8).** Both previous
  cuts described this obligation as unmeasured by any gate run. The D1 review
  seat executed `bash scripts/coverage-exact.sh` and it **exited 1** — lines
  32,626/32,802, branches 5,494/5,508, functions 3,181/3,191 — recorded in
  `.forge/results/8d9f4a9e-…-positions-correctness.json`, which adds that "all
  uncovered lines are in unchanged files; no `composite.rs` line is uncovered".
  This matters in the direction that hurts: the honest status of N13 is not "no
  result yet" but **"the last real gate run on this change was red"**, and a
  ledger that reports the weaker statement understates what unit 15 must clear.
  Separately, the hand reproductions D2 and D3 recorded on `1d1d9f17` —
  32,802/32,802 `DA` and 5,508/5,508 `BRDA` — are consistent with the export,
  but their function figure is not the gate's: `tasks.md:1110` quotes the
  report's own column, **3,237 of 3,237**, while review `5c0333bc` applied the
  script's file-plus-start-line rule to both retained exports and got
  **3,191/3,191**. A report's symbol count and the gate's source-function count
  are different numbers and this ledger should not have merged them.
  `coverage-exact.sh` on the final candidate, the native macOS leg (B26, N2, N4)
  and final-head remote CI all remain pending. The owner is **N13 (8.8.8.3)**,
  whose text keeps it "pending until actual evidence exists", together with its
  denominator reconciliation against chief `124cca78`'s baseline (32,148/32,324
  lines, 5,434/5,448 branches, 3,125/3,135 functions, failed equality).
  **8.8.14.3 (S13) is not the owner** and has no pending half.
- **F13 — a suite this ledger leans on has no execution on the final
  candidate (C7, corrected on the third return).** Rows N2, N4, N5b, N5c and
  N9 cite `crates/brokkr-cli/tests/doctor_dsh_selection.rs`. It is a separate
  integration target, and none of the four ledger-seat commands builds it.
  **This finding's first wording was itself two false negatives**, the class
  F12 names:
  - "never executed" is false. D3's delivery ran every `brokkr-cli`
    integration binary (`tasks.md` 1015–1016), and so did its returned review
    on `1d1d9f17` (1098). The composite visits count "`doctor_dsh_selection`
    14" passed at 1940, 2076, 2377, 2596 and 2921. The third chief also
    reports the D3 result `183c4dda-…`.
  - "the entire R3 raw-span matrix… lives only there" is false. The producer
    suite holds the same matrix with real equality and refusal assertions
    (`composite/tests.rs:4553–4619`; N5c).

  What stands: those executions are **historical reports by other seats, on
  other revisions**, and none is on the candidate that will be ticked. Unit
  14's crate-scoped `cargo test -p brokkr-cli` gives the suite its execution
  on that candidate, which is one more reason unit 14 is not a formality.
- **F8 — the privacy exclusions are asserted one level too early (A53, B30,
  B42, B55).** Three clauses ask that the private carrier and the route bytes be
  absent from **emitted launch evidence, the journal and the composite**.
  `resume_tests.rs:2026–2033` inspects `starts[1]["context"]` — a start message
  — and `assert_dsh_planner_overlay` (`adapters/tests.rs:12361–12400`) inspects
  planned argv and the written overlay file. `dsh_launch_with` returns a plan
  and emits no launch row at all, so the planner suite has nothing to inspect;
  the engine suite emits real launch rows and never reads them for these fields.
  B30's third predicate — "retain the existing confirmed `root_session` and
  `transcript` fields" — is likewise unasserted in those two cases. **This is an
  evidence gap, not an observed leak**: nothing opened here shows a private
  field or a route byte reaching a launch row, and `adapters.rs:3500`'s
  `confirms_from_locator: false` and the plan-shaped seam are consistent with
  the intended exclusion. The remedy is an assertion, and if it fails, a Pass-B
  production finding.
  *Since then:* unit 7 (recorded as 4a) asserted the exclusions on current
  production bytes with no finding, at the composite and the wire. For the
  shipped route it did so at the journal too, through the real adapter under
  the engine. A53, B42 and B30 keep one half open: the gated shapes and the
  offered publication, proved in two processes rather than one. B55 stays
  partial only because it rides B30. Unit 11 closes that half.
- **F9 — B27's mistyped half covers two fields of four.** 5170–5172 names
  "a missing/mistyped **home or locator**". `resume_tests.rs:1285–1290` proves
  both *missing* cases read `None`. The mistyped journal at `:1340–1372`
  mistypes `harness_version` and `wrapper_digest` only (`:1354–1355`);
  `locator` and `home`
  stay well-typed strings. **Missing evidence, not missing behaviour** —
  `originating_root` reads every field the same way — but the clause names the
  two fields whose mistyped case is undriven.
- **F10 — two of four admission budgets have no admitted-limit positive
  (B71).** 5337–5338 asks for each budget "at its admitted limit **and
  beyond**". The header budget and the enumeration budget have both sides, the
  header's positive landing its newline exactly on the last admitted byte. The
  session-file cap is driven only at `DSH_SESSION_FILE_LIMIT + 1` and the
  per-row sequence budget only beyond its injected value of 4. A refusal-only
  budget cannot distinguish a correct bound from one set too low, which is what
  the positive side is for. **Missing evidence, not missing behaviour.**
- **F11 — eight numbered tasks under 8.8 are open, and they are open for three
  different reasons (N1–N4, N11–N14).** §1 grades them, and the reasons should
  not be run together, because only one of them is work:
  **(a) blocked retrieval.** N1's and N3's immutable Apple and env source pins,
  which their own clauses call pending inherited acceptance "outside the
  repair". *(Third return, finding 1: this item first read "unproducible… which
  no seat, host or visit can mint". That was wrong. The sources are public
  upstream files, and what has been missing is a grant that reaches the
  network. Every seat that tried was refused, at `tasks.md` 2389–2394 and
  2425–2431. Unit 13 retrieves and verifies them.)* **(b) a host away.**
  N2's and N4's macOS legs and N11's retained-Node positive. The code and the
  test are written and only execution is missing, and N11's two removals have
  never been performed (units 15 and 16). **(c) gates and reconciliation.**
  N12, N13, N14. The previous cut filed (a) and (b) together as "evidence
  nobody here can produce", which was misleading about both. N11 in particular
  was reported as a missing *test*, and it is not one.
- **F12 — this ledger is the worked example of its own rule, twice.** The first
  cut set out three evidence kinds, said a checkbox and a delivery note
  discharge nothing, and then discharged A6 on five checkboxes, eleven
  removal-bearing rows on delivery narratives, and B64 on a test that proves the
  opposite of the clause; it also omitted a fifth of 8.8's acceptance because
  the heading above it read "Historical".

  The revision that recorded all of that then made a **different and worse**
  class of error, and it is worth naming precisely because it survived a pass
  written specifically to catch the first one. Where the first cut *over*-graded
  on evidence it had not opened, the second cut **under**-graded on evidence it
  had not looked for, and stated the absences as findings:
  "no test in the tree is the absent-PATH retained-Node positive" (the test is
  at `composite/tests.rs:3489`); "the first strict validation ever recorded on a
  candidate" (there were two before it); "exact coverage… unmeasured by any gate
  run" (it ran, and failed); N5's and N6's sub-clauses graded on one line of
  evidence each while three tests covering them sat uncited; `B88`'s name count
  repeated after two prior reviews had corrected it in writing.

  The two failures share one root: **grading a clause against the evidence that
  happened to be in front of the grader** — offered, in the first case; absent,
  in the second. A negative claim needs an opened artefact exactly as much as a
  positive one does, and "this seat could not run it" is not "nobody has run
  it". That is the rule this ledger exists to apply, and the reason both cuts
  are recorded here rather than quietly overwritten.

  The third cut repeated the second failure, in the corrections themselves.
  C7's correction said `doctor_dsh_selection.rs` had never run and that the
  raw-span matrix lived only there. C8's said `69cac25d` was the Pass C head,
  and that neither clippy nor the bundle compile had run on a Pass D head. N13
  read "Nothing" beside the F7 that recorded its failed gate. And "no seat…
  can mint" the source pins turned a refused network into an impossibility.
  Each was a negative, or a label, stated without opening the artefact that
  decides it: `tasks.md`'s own gate tables, `git log`, the producer suite.
  The third chief found every one. The remediation pass corrected them in
  place and left the wrong words beside the correction.

## 6. The exact remaining work

**Twenty-four numbered entries, 1–24, and the numbering is the dependency
order.** No entry depends on a later one. Where two entries are independent,
the lower number is only a convenient place in the order, not a prerequisite.
Each entry is one visit unless it says otherwise. Entries 1–10 have **landed**
on `slice-dsh-8810`. They keep their dated records verbatim, and their
headings carry the label each was commissioned and recorded under.
*(Third return, finding 6: the previous cut put unit 4 before its
prerequisite 4a, and unit 9 after units 6 and 7, which depend on it. It then
sent readers to a separate "execution order" paragraph for the real order.
That paragraph is gone, because the list is now the order.)*

**Labels.** Text written before this renumbering keeps the labels in force
when it was written. That means the commissions and dated records under
entries 1–10 and the notes those units added to rows. There, "unit 4" is
entry 14 and "unit 3" is entry 6. Everywhere else this ledger uses the new
numbers. The map:

| New | Recorded as | State |
|---|---|---|
| 1 | 1 | landed `8f60f06c` |
| 2 | 2a | landed `17b5b4d2` |
| 3 | 2b | landed `a4f7a5e0` |
| 4 | 2c-fix | landed `289d9c5b` |
| 5 | 2c | landed `7b26d186` (after a stop) |
| 6 | 3 | landed `3a2a6785` |
| 7 | 4a | landed `3a80b1d8`, `a41ae69e`, `9a3c0746` |
| 8 | 2d | landed `97624edd`, `7900c182` |
| 9 | 3b-fix | landed `6a5f1bc9` |
| 10 | 3b | landed `cb5bbd9a` (after a stop) |
| 11 | — (new: unit 7's unproved half) | landed `6e1e7066`, `471bc740` |
| 12 | — (new: N2/N4 unasserted cells) | landed `36b16922` |
| 13-fix | — (new: finding P1, entry 13's stop) | landed `1d2763cf` |
| 13 | — (new: source retrieval) | landed `fcb91ad2` (after a stop; host-retrieved sources) |
| 14 | 4 | open |
| 15 | 5 | open, externally owned |
| 16 | — (new: N11 on a capable host) | open, externally owned |
| 17 | 9 | open, an operator ruling |
| 18–20 | — (new: replays after a negative ruling) | conditional on 17 |
| 21 | — (new: (a) THE PROOFS' unrecorded controls; (b) the remaining replays) | (a) landed: performed at `90b548e3`, closed under the operator's 2026-09-23 ruling on F1–F4; (b) conditional on 17 |
| 22 | 6 | open |
| 23 | 7 | open, prepares an operator ruling |
| 24 | 8 | open, 9.6's, not this change's |

| Kind | Entries |
|---|---|
| Missing **evidence**, a test to write | 1–10 (landed), 11, 12 |
| **Externally owned evidence**: a grant or host no seat of this change has had | 13, 15, 16 |
| **Gate execution**, nothing to write | 14 |
| **An operator ruling**, not labour | 17 (and 23 prepares one) |
| **Controls never recorded**, performed whatever 17 rules | 21(a) (landed) |
| **Replays**, only if 17 rules no | 18, 19, 20, 21(b) |
| **Record and reconcile** | 22, 23 |
| Missing **behaviour**, not this change's | 24 |

**What gates what.** Entry 14 runs on the head that will be ticked, so every
entry that can move a byte comes before it: 11 and 12 write tests, and 13 may
correct the port's source citations or find a divergence. 17 is ruled against
that gated head. 22 ticks 8.10 only when 21(a) has landed and either 17's
ruling covers every 8.10 removal predicate or 18, 19 and 21(b) have landed.
21(a) runs on 14's gated candidate and moves no byte that survives. If one
of its controls does not part its case, that is a finding, and a repair
re-opens 14. *(2026-09-23: 21(a) has landed. It ran on `be1ecf77`'s
bytes, after 11, 12 and 13 had landed, and its four findings closed under
the operator's ruling with no repair, so 14 was not re-opened.)* 23 needs 13, 15, 16 and
17 (or 20). **15 is not a predecessor of 22.** S12 asks that external
evidence be *recorded*, "when supplied, otherwise explicitly
pending/unavailable" (314–326). So 22 may close over a pending 15 if it
reports it as pending. It may not report it as passed, and given F7 it may
not report it as unrun. 24 follows both ticks.

The first cut's single unit 2 was split three ways (entries 2, 3 and 5),
because one visit could not add six call-site variants, a compiled-member
symlink fixture and twenty planner vectors.

1. **Assert the adaptation's delta digest.** Closes A67 and B99 (the one of
   five sub-obligations now open). Touches
   `crates/brokkr-protocol/src/adapters/composite/tests.rs` only, inside
   `the_committed_plugin_set_is_the_six_files_and_the_one_expression_delta`.
   Proof: recompute SHA-256 over `lib/index.js:253\n`, the upstream line
   prefixed `-`, the adapted line prefixed `+`, each newline-terminated, and
   assert equality with `PROVENANCE.md`'s `78256d2e…`; show the assertion
   binds by changing either line in the expectation and observing it part.
   No production change; `extensions/` stays byte-identical.

   **Landed 2026-09-23 at `8f60f06c`.** The digest is recomputed inside
   `the_committed_plugin_set_is_the_six_files_and_the_one_expression_delta`
   from the two lines the upstream-digest substitution already proves, with
   the location read off the committed bytes (`index.lines().position(…) + 1`,
   asserted `== 253`) rather than copied from the note; it reproduces
   `78256d2e…`. Three mutations, compiled, run red and reverted: the adapted
   line spelled with `0` for `firstSeq` parts at `tests.rs:958`
   (`fac67fa4…`), the upstream line without its leading tab parts at `:957`
   (`26b8abfa…`), the line index taken one past its position parts at `:954`
   (254 against 253). `cargo fmt --all -- --check`, `cargo clippy --workspace
   --all-targets --all-features --locked -- -D warnings` and `cargo test -p
   brokkr-protocol --all-features --locked` (426 + 99 + 1 passed, 0 failed)
   are green. Tests only; `extensions/` unchanged; no checkbox moved.

2. *(recorded as 2a)* **Give each engine route shape its missing call site.** Closes B38's
   single-site half, B39's panel-member half, B36's empty matrix cell and
   A37's call-site half. Touches
   `crates/brokkr-runtime/src/engine/resume_tests.rs` only. The previous cut
   asked for "one non-binding shape" at the single site; that does not satisfy
   the clause, which says **each** listed shape receives no binding "at either
   call site" (5231–5237). The full list, all five currently panel-only
   (`:1745–1797`):
   **(1) `nonmember`** — `recipe/other.yml`, in the layer, not in `files`;
   **(2) `shadow`** — `route.yml` at the working-directory path, shadowing the
   bundled one; **(3) `ancestor`** — `base/ancestor.yml`, bindable only through
   the ancestor's aggregate digest; **(4) `traversal`** — `../escape.yml`;
   **(5) `absolute`** — the layer file named by its absolute path, which is
   what `./` expansion produces at `bundle.rs:3689`.
   (The sixth, the symlink, is unit 2b's — it needs a fixture that does not yet
   exist, and adding it here would hide that.)
   Also in this unit, two positives: **the changed-bytes member at the panel
   site** (B39's mirror — today `::a_changed_route_overlay_member_carries_the_manifest_digest`
   is `single(…)` only), and **the offered start at the panel member** (B36's
   missing cell — today `::a_valid_route_overlay_binds_on_an_offered_start_too`
   uses `seat(single(argv, vec![candidate]), …)` at `:1891`).
   Proof, in every case read off the real `Start.input`: a non-binding shape's
   `resume_context` carries no `route_overlay`; the changed-bytes member carries
   the manifest's recorded digest with `assert_ne!` against a hash of the
   resolved file; the offered panel member carries value and digest on both its
   cold attempt and its retry.
   Expected to be evidence-only — `engine.rs:1372` computes one
   `route_overlay_binding` per site in a single loop and both callers (`:1667`
   single, `:1917` panel) consume that one value — so a *divergence* between
   the sites would be a production finding, and the unit reports it as one.

   **Landed 2026-09-23 at `17b5b4d2`.** Two cases in `resume_tests.rs`, both
   reading the real `Start.input`:
   `a_non_binding_route_overlay_withholds_the_member_at_the_single_site`
   drives the nonmember shape through `run_driver` and asserts the context is
   an object carrying no `route_overlay` key at all, and
   `a_changed_route_overlay_member_carries_the_manifest_digest_at_the_panel_member`
   drives the changed-bytes member through `MemberRun`, asserting the
   manifest's recorded digest and `assert_ne!` against a hash of the file as
   it now stands on disk. Two mutations, compiled, run red and reverted:
   `None` for the member's binding at `engine.rs:1917` parts the panel case at
   `resume_tests.rs:1958` (`Null` against `"recipe/route.yml"`) while the
   single-site case stays green — which is what makes the new case evidence
   for that call site; and a `files` lookup falling back to a literal digest
   instead of refusing a nonmember parts the single-site case at
   `resume_tests.rs:1859`
   (`Some({"digest":"unrecorded","value":"recipe/other.yml"})` against
   `None`). `cargo fmt --all -- --check`, `cargo clippy --workspace
   --all-targets --all-features --locked -- -D warnings` and `cargo test -p
   brokkr-runtime --all-features --locked` (458 lib plus 85 integration
   passed, 0 failed) are green. Tests only; no production line moved;
   B38(ii)'s compiled-member escaping symlink stays with unit 2b; no checkbox
   moved.

   *Adoption note (remediation, 2026-09-23).* The commission above is
   `af89511a`'s, which grew to five single-site shapes and the offered-panel
   positive. This branch's unit 2a ran on the older one-shape text. The
   record above lands the nonmember and the panel changed-bytes member. The
   other four single-site shapes and the offered-panel positive landed with
   unit 8 (recorded as 2d) and its return.

3. *(recorded as 2b)* **Build the compiled-member escaping-symlink vector.** Closes the shape of
   B38(ii) that no fixture in the tree expresses (F2b). Touches
   `crates/brokkr-runtime/src/engine/resume_tests.rs` only. The fixture the
   clause requires: an in-layer symlink whose target resolves outside the layer
   but inside the working directory, **listed in `bundle.manifest["files"]`
   with a digest**, so that it is a compiled member and its refusal cannot be
   the nonmember rule. Retain the existing nonmember-symlink case beside it.
   Proof: the member receives no binding, at both call sites. If it *does* bind,
   that is a Pass-B production finding — membership authorizing a path that
   escapes its layer — and the unit reports it rather than relaxing the vector.
   This unit is named separately because its outcome is not predictable from
   anything opened here.

   **Landed 2026-09-23 at `a4f7a5e0`; the member does not bind — no Pass-B
   finding.** `escaping_member_layer` lists `recipe/link.yml` in `files`
   with the digest of `work/outside.yml`, the bytes it reaches, and asserts
   the fixture's own shape first (a symlink, canonicalising outside the
   layer). `an_escaping_symlink_member_is_withheld_at_the_single_site` and
   `an_escaping_symlink_member_is_withheld_at_the_panel_member` (beside a
   sibling whose real member binds) each find the private context present
   and no `route_overlay` key in it; the nonmember-symlink panel case is
   retained. One mutation, compiled, run red and reverted: canonicalising
   only the joined path's parent in `route_overlay_binding`, so the final
   symlink is not followed, parts both new cases — `resume_tests.rs:1941`
   and `:1998`, `Some({"digest":"fbf191b4…","value":"recipe/link.yml"})`
   against `None` — while both existing nonmember cases stay green, which is
   what the nonmember could not stand in for. `cargo fmt --all -- --check`,
   `cargo clippy --workspace --all-targets --all-features --locked -- -D
   warnings` and `cargo test -p brokkr-runtime --all-features --locked` (460
   lib plus 94 integration passed, 0 failed) are green. Tests only; no
   production line moved; no checkbox moved.

4. *(recorded as 2c-fix)* **Refuse a route beside a model pin with no provider segment.** The
   production remedy for unit 2c's Pass-B finding below, ordered before 2c's
   re-run. Touches `crates/brokkr-protocol/src/adapters/route_overlay.rs`
   (the `claim` boundary and its test module) and
   `crates/brokkr-protocol/src/adapters/tests.rs` (the segment-less vector in
   `dsh_route_grammar_matrix_refuses_before_staging_on_every_planner_path`).
   A pin without a `/` provider segment refuses with the absent pin's bounded
   reason, before any file is read or staged, echoing no value. Proof: a
   route keyed `deepseek-official`, model item `deepseek-v4-flash` with a
   `reasoningEfforts` block, beside `--model deepseek-v4-flash`, refuses on
   disabled, offered and cold with `dsh_staging_calls() == 0`, zero producer
   calls and the exact reason; removing the check in a compiling mutation
   parts the new vectors; a pinned-segment positive still passes unchanged.

   **Landed 2026-09-23 at `289d9c5b`.** `claim_with` now reads the pin through
   `model.filter(|model| model.contains('/'))`, so a segment-less pin takes
   the absent-pin arm (`` refusing to invoke the dsh driver: a route overlay
   needs a pinned `--model` with a provider segment ``) before the path checks,
   the `stat` or the read. The matrix carries the vector above as `model pin
   without a provider segment`, asserting the reason whole and no echo of
   `deepseek-v4-flash` or `deepseek-official`, and
   `route_overlay.rs::a_bound_route_beside_a_segment_less_pin_refuses_before_any_read`
   proves the same bytes pass `validate` for that pin, refuse at `claim_with`
   with injected readers that panic if reached, and admit when the route and
   pin name `deepseek`. One mutation, compiled, run red and reverted: the
   filter removed parts the matrix at `tests.rs:12725` (`model pin without a
   provider segment/disabled must refuse`; a diagnostic run under the same
   mutation saw each of disabled, offered and cold admit with
   `dsh_staging_calls() == 1`) and the reader test at `route_overlay.rs:935`
   (the injected `stat` reached). The existing positives
   `claim_reads_the_bound_file_and_requires_the_digest_before_the_shape` and
   `a_dsh_route_overlay_planner_folds_on_the_offered_and_unmeasured_paths`
   pass unchanged. `cargo fmt --all -- --check`, `cargo clippy --workspace
   --all-targets --all-features --locked -- -D warnings` and `cargo test -p
   brokkr-protocol --all-features --locked` (427 + 99 + 1 passed, 0 failed)
   are green. The DSH route stays disabled; unit 2c's other classes are
   untouched and 2c re-runs whole; no checkbox moved.

5. *(recorded as 2c)* **Drive the reader's remaining classes through the three planner paths, and
   add the two classes that exist nowhere.** Closes B47, B49 and B50. Touches
   `crates/brokkr-protocol/src/adapters/tests.rs` (extend
   `dsh_route_grammar_matrix_refuses_before_staging_on_every_planner_path`) and
   `crates/brokkr-protocol/src/adapters/route_overlay.rs`'s test module. The
   ten reader-only `baseURL` breaches — userinfo, fragment, percent-escape,
   whitespace, brackets, empty segment, invalid host label, invalid port,
   uppercase scheme, schemeless; the seven reader-only lexical shapes —
   `__jsExpr`, alias, block scalar, quoted scalar, tab, control character,
   document marker; an absent `--model` pin and a provider-segment-less model
   beside a bound route. **New at both levels:** a **non-ASCII `baseURL`**
   (5273) added to `the_endpoint_grammar_decides_the_positive_and_every_refusal`
   and to the planner matrix, and a **second entry** (5267) — a route document
   carrying two top-level entries — likewise at both. Proof: each vector on
   disabled, offered and cold with `dsh_staging_calls() == 0`, zero producer
   calls, no version-probe marker, a fixed reason naming a depth, field or URL
   part and no echoed value. If the non-ASCII or second-entry vector is admitted
   rather than refused, that is a Pass-B production finding and the unit reports
   it. If 2c proves too large for one visit, split it at the `baseURL`/lexical
   boundary; the two new classes ride with whichever half lands first.

   **Stopped 2026-09-23 on a Pass-B production finding; no test landed.** A
   route beside a `--model` pin with **no provider segment** is admitted and
   staged, against `specs/adapter-resume-safety/spec.md:1478` and `:1560` and
   `tasks.md` 5268. `route_overlay.rs:111–115` refuses only an *absent* pin;
   a segment-less pin reaches `validate`, where `parse_dsh_model` supplies
   the default provider `deepseek-official` (`adapters.rs:4814`, `:4843`), so
   a route whose one provider key is `deepseek-official`, model item
   `deepseek-v4-flash` with a `reasoningEfforts` block, beside `--model
   deepseek-v4-flash` validates `Ok(())` and plans with `dsh_staging_calls()
   == 1` on the disabled, offered and cold paths alike. The reader's existing
   vector (`a_route_needs_a_model_pin_and_model_item`) hides this: its route
   names `dashscope`, so the segment-less pin is refused for naming a
   provider the seat did not pin, not for lacking a segment. F2's grading of
   the model-pin pair as "missing evidence, not missing behaviour" is
   withdrawn for the segment-less half. The two new classes were probed on
   the same bytes and are **not** findings: a second top-level entry refuses
   `route overlay must hold exactly one top-level entry` and a non-ASCII host
   (`https://hóst/x`) refuses `baseURL leaves the closed endpoint grammar`,
   each at the reader and on the cold planner path with zero staging calls.
   The remedy is a production refusal at the `claim` boundary for a pin
   without `/`, then this unit re-run whole; the probes were reverted.

   **Re-run landed 2026-09-23 at `7b26d186`; both new classes refuse — no
   Pass-B finding.** The matrix now breaches a route it first proves
   `validate` admits for `deepseek/deepseek-v4-flash`, one class per vector,
   and asserts each reason whole on disabled, offered and cold with
   `dsh_staging_calls() == 0`, zero producer calls, no probe marker and no
   echo of a `leaked` sentinel carried in every breaching value: the ten
   reader-only `baseURL` breaches and a non-ASCII host (`baseURL leaves the
   closed endpoint grammar`); `__jsExpr` (line 6 key not a plain
   identifier), alias, block and quoted scalar (line 6 reserved character),
   tab and control character (line 6), a document marker opening a second
   document (line 10); a second top-level entry (`route overlay must hold
   exactly one top-level entry`); and an absent `--model` pin (the claim
   boundary's pin reason). 2c-fix's segment-less vector is kept, not
   duplicated. At the reader,
   `the_endpoint_grammar_decides_the_positive_and_every_refusal` gains a
   non-ASCII host and path segment and asserts every refusal whole, and
   `a_route_document_carrying_a_second_entry_is_refused` refuses the shipped
   route repeated and followed by a `settings` row. Five mutations, compiled,
   run red and reverted: a sequence of any length admitted parts the reader
   case at `route_overlay.rs:743` (`Ok(())`) and the matrix at
   `tests.rs:12830` (`second top-level entry/disabled must refuse`); Unicode
   host letters part `route_overlay.rs:721` (`"https://hóst/x"`, `Ok(())`)
   and `non-ASCII endpoint/disabled`; `%` admitted in a segment parts `:721`
   (`percent%2fescape`) and `percent-escape endpoint/disabled`; the lexer's
   control-character half removed parts `control character/disabled`; the
   planner defaulting a missing pin before `claim` parts `absent model
   pin/disabled`. `cargo fmt --all -- --check`, `cargo clippy --workspace
   --all-targets --all-features --locked -- -D warnings` and `cargo test -p
   brokkr-protocol --all-features --locked` (428 + 99 + 1 passed, 0 failed)
   are green. Tests only; the DSH route stays disabled; no checkbox moved.

6. *(recorded as 3)* **Complete the undriven variation matrices.** Closes A42, B60, **B59**,
   B27's mistyped half and B71's two missing positives. Touches
   `crates/brokkr-protocol/src/adapters/tests.rs` and
   `crates/brokkr-runtime/src/engine/resume_tests.rs`. Three groups:
   (i) in `a_dsh_identity_mismatch_declines_the_offer_and_keeps_the_cold_route`,
   add **missing version**, and mistyped (a number) and malformed (a non-hex
   string) for both originating fields, each declining `unverified-harness`,
   cold under the current home, with the producer call count the clause permits;
   (ii) in `a_stamped_row_is_offered_only_to_its_own_site_owner_and_persistent_root`,
   mistype the newest row's `locator` and `home` and assert each reads `None`
   rather than the older row's value; (iii) in
   `dsh_admission_reads_are_complete_within_their_bounds_or_decline` and
   `dsh_stored_sequences_decline_instead_of_reporting_a_partial_maximum`, add an
   admitted-at-limit positive for the session-file cap (a file of exactly
   `DSH_SESSION_FILE_LIMIT` bytes read successfully) and for the per-row
   sequence budget (a row whose newline lands exactly on the injected budget);
   and **(iv)** in that same identity test, the version-output classes 5302–5306
   asks for "independently" and which today share one banner vector (B59): a
   probe that **succeeds with empty stdout** (absent output) and one whose
   stdout is **not readable as a version for a different reason** than the
   existing `no-version-here` banner — non-UTF-8 bytes — each asserting
   `observed == None`, the cold route and zero producer calls, so that absent,
   malformed and unreadable are three vectors rather than one.
   No production change expected.

   **Landed 2026-09-23 at `3a2a6785`; every new vector passes on current
   production bytes.** (i) Beside a control where the unvaried offer rejoins
   its own root after one producer call, each originating field is varied
   through absent, null, mistyped (`7`/`8`), malformed (`zz-not-hex`,
   `"z"×64`) and different values; each declines `unverified-harness` after
   exactly one producer call, with no rejoin, no fold boundary, no
   `--session` and a fresh root whose parent is the canonicalised current
   home's `sessions/brokkr`. (ii) The newest row's `locator` (`9`) and then
   `home` (`10`) are mistyped; each reads `None` while the other coordinate
   stands. (iii) A stored session of exactly `DSH_SESSION_FILE_LIMIT` bytes
   reads `Some(5)`; a `{"seq":7}` row admitted at an injected budget of its
   own length reads `Some(7)`, one byte less `None`. Twelve mutations,
   compiled, run red and reverted: the originating comparison skipped for an
   absent, null, mistyped or malformed version, and likewise for the digest,
   each part `tests.rs:10577` at its own case (`None` against
   `Some("unverified-harness")`); the cap tightened to `>=` parts `:11828`
   (`None` against `Some(5)`); the event budget one byte short parts `:11889`
   (`None` against `Some(7)`); a mistyped locator or home stringified parts
   `resume_tests.rs:1405` (`Some("9")`) and `:1410` (`Some("10")`).
   `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets
   --all-features --locked -- -D warnings`, `cargo test -p brokkr-protocol
   --all-features --locked` (428 + 99 + 1 passed) and `cargo test -p
   brokkr-runtime --all-features --locked` (460 lib plus 94 integration
   passed) are green. Tests only; no checkbox moved.

   *Adoption note (remediation, 2026-09-23).* Group (iv) and the **B59** in
   this unit's heading are `af89511a`'s text. This branch's unit 3 was
   commissioned from the older text without them, and its record above covers
   (i)–(iii) only. Group (iv) was delivered by units 9 and 10 (recorded as
   3b-fix and 3b), and B59's row cites them.

7. *(recorded as 4a)* **Assert the privacy exclusions where the clauses put them.** Closes A53's
   exclusion half, B30's launch-evidence and retained-fields halves, B42's
   second sentence and B55.

   The previous cut got this unit's *subject* wrong (C4). It proposed checking
   the private carrier's keys in launch evidence plus a composite equality —
   but the clause at 4993–4995 is about **route bytes**: "No **route byte or
   binding** enters the composite, launch row or journal", repeated at
   5253–5254 as "Keep the composite, launch row and journal free of route
   bytes." The two cases it named,
   `an_offered_dsh_start_carries_the_recorded_home_at_…`
   (`resume_tests.rs:1937–2113`), were opened again here and **carry no bound
   `--patch` at all** — no `patched(…)` argv, no `manifest["files"]` entry — so
   no assertion added to them can say anything about route exclusion. And
   `assert_dsh_planner_overlay` (`adapters/tests.rs:12361–12398`), opened line
   by line, reads `launch.command` and the written overlay file only.

   So the unit is: **drive a start that actually has a bound route, and read the
   surfaces the clause names.** Touches
   `crates/brokkr-runtime/src/engine/resume_tests.rs` — extend
   `::a_valid_route_overlay_binds_on_an_offered_start_too`, which already plants
   the file, binds the member and runs a real engine, and assert of the
   **emitted launch evidence and the journal envelope** for that attempt: no
   `route_overlay` key, and the overlay's path and digest appear as no value
   anywhere in either — while the confirmed `root_session` and `transcript`
   fields are retained, and no `resume_context`, `owned_target` or `assessment`
   is copied across (B30's other two predicates) — and
   `crates/brokkr-protocol/src/adapters/tests.rs` — the same synthetic install
   composed once under a bound route and once unbound yields one identity
   (A53's composite half).

   Proof: each new assertion parts under a compiling mutation that lets the
   field through. **If any of them fails on current production bytes, stop and
   report it as a Pass-B security finding rather than adjusting the assertion.**
   Nothing opened shows a leak — `adapters.rs:3500` holds
   `confirms_from_locator: false`, `dsh_launch_with` returns a plan and emits no
   launch row, and `engine.rs:3962–3998` retains canonical containment — but
   nothing opened establishes the exclusion either (F8), and that is the whole
   reason this unit exists. Runs before unit 4 so its result is inside the gated
   candidate.

   **Landed 2026-09-23 at `3a80b1d8`; every exclusion holds on current
   production bytes — no Pass-B finding.** Built to the unit and both review
   addenda. Engine: both `an_offered_dsh_start_carries_the_recorded_home_at_…`
   cases run on a canonicalised root, on the DSH-shaped checkpoint fixture
   (`dsh_model_driver`: `dsh-session` root plus `transcript`), with a bound
   `--patch` route whose display name, key variable and endpoint carry a
   marker. The offered start's private context holds the binding, the owned
   target and the assessment; the journaled launch row of that attempt keeps
   `root_session` and `transcript` exactly and carries none of them; every
   journaled event is searched for the marker, `recipe/route.yml`, the
   file's path and digest and every private-carrier key, the pinned
   manifest's `files` entry (spec: the provenance the bundle digest already
   covers) asserted and removed first. Adapter:
   `a_bound_dsh_route_reaches_neither_the_composite_nor_the_launch_row_nor_the_journal`
   validates the marked route, then over a synthetic install and the real
   producer composes one identity under a bound and an unbound route
   (recomputed while the bound overlay is staged), and drives the route
   through `run_seat_with` → `dsh_launch_with` → `invoke_dsh_launch` on
   qualified cold, confirmed rejoin, closed-gate shipped cold and a declined
   offer: the child receives the route, the launch row holds exactly its own
   vocabulary (retained root and address where confirmed), and no wire
   message carries the route's content, path, digest, binding or a private
   carrier. Nine mutations, compiled, run red and reverted: the binding
   written into the launch row's `model` at the single site parts
   `resume_tests.rs:2279` with the panel case green, and at the member site
   parts the panel case alone at `:2279`; the offered attempt's `transcript`
   dropped parts `:2338`; the private context in the stderr tail parts `:2279`
   at an unfenced `EffectFailed`; the staged overlay copied into the launch
   row as `sandbox` parts `tests.rs:14265` (key set), into `effort` parts
   `:14321` on `cold`, and on the shipped route only parts `:14321` on
   `disabled`; the route folded into the compared composite parts `:14099`
   (`None` against the declared digest); the owned target in the finishing
   record parts `:14321` on `resumed` alone. `resume_context` inserted into a
   journaled checkpoint is refused by the closed v5 seat-record schema — a
   second layer, recorded, not the proof. `cargo fmt --all -- --check`,
   `cargo clippy --workspace --all-targets --all-features --locked -- -D
   warnings`, `cargo test -p brokkr-protocol --all-features --locked` (429 +
   99 + 1 passed) and `cargo test -p brokkr-runtime --all-features --locked`
   (460 lib plus 94 integration passed) are green. **Closes** B30's
   launch-evidence and retained-fields halves and B55. It also claimed A53's
   exclusion half and B42's second sentence; the review of `e428ad23` (C1 +
   SEC-1) withdrew that claim, because the driver re-emits its child's stderr
   outside every wire message and the engine journals that tail on a failed
   or indeterminate attempt, so clean wire messages did not prove a clean
   journal. See the return below. Tests only; the DSH route stays disabled;
   no checkbox moved.

   **Returned and landed 2026-09-23 at `a41ae69e`; the real adapter's
   journal is clean on current production bytes — no Pass-B finding.**
   `the_real_dsh_driver_journals_no_route_byte_and_no_carrier`
   (`resume_tests.rs`) runs its own test binary as the seat's driver, serving
   production's `adapters::serve(Dsh)`, on a seat whose `--patch` route binds
   and whose declaration is the shipped `unmeasured` one read by the
   production loader. That is the closed-gate cold route production runs.
   The first attempt's child exits 3 with one stderr line and the retry
   succeeds. Both starts hold the binding and the assessment privately and
   both children receive the marked route. The case first shows that each
   launch row, the failed attempt's stderr tail and the success exist. It
   then finds no journaled event carrying the route's content, path, digest,
   binding or a private carrier, and asserts the tail equals the child's own
   line and each launch row is exactly the shipped vocabulary plus the
   engine's stamps. The adapter case now also captures each invocation's
   stderr on all four shapes and asserts it free of the same needles and
   equal to the child's own line. Three mutations, compiled, run red and
   reverted. First, the staged overlay's content appended to the
   invocation's stderr parts `tests.rs:14342` (`cold`) and
   `resume_tests.rs:2279` on the journaled `EffectFailed`. Second, the
   overlay's `displayName` value copied into the launch row's `effort` parts
   `tests.rs:14328` (`cold`) and `resume_tests.rs:2279` on the journaled
   launch checkpoint; copying the whole line was refused first by the closed
   v5 fence, which is a second layer (recorded, not the proof). Third, the
   bound route file `eprint!`ed beside the driver's tail leaves the adapter
   case green and parts the engine case alone at `resume_tests.rs:2279`. The
   gates `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets
   --all-features --locked -- -D warnings`, `cargo test -p brokkr-protocol
   --all-features --locked` (429 + 99 + 1 passed) and `cargo test -p
   brokkr-runtime --all-features --locked` (461 lib plus every integration
   binary, 0 failed) are green. **Closes**, for the shipped route: A53's
   exclusion half and B42's second sentence at the composite, the launch
   row and the journal, stderr tail included. **Stays partial:** A53 and
   B42 at the journal for the gated shapes (qualified cold, confirmed
   rejoin, declined offer). Their wire messages and stderr are proved clean
   by the adapter case. The engine's journaling of an offered DSH row is
   proved to add nothing by the two originating-home cases. But no case runs
   the real adapter's open gate under the engine, because that needs the
   synthetic composite install that exists only in the protocol suite. Tests
   only; the DSH route stays disabled; no checkbox moved.

   **Corrected 2026-09-23: the journal closure above did not hold at
   `a41ae69e` and holds from `9a3c0746`.** The review of `f5895001` (SEC-2)
   found the case piping the adapter's stdout through `grep '^{'`, which
   dropped every non-JSON-looking line before the engine read it. Production
   keeps an unreadable driver line in its error (`process.rs:183–184`) and
   the engine journals that error as `EffectFailed`, so a route byte printed
   outside the protocol would have reached the journal in production and
   vanished in the test. **Returned and landed 2026-09-23 at `9a3c0746`; the
   real adapter's whole stdout is clean on current production bytes — no
   Pass-B finding.** The driver's stdout now reaches the engine whole but
   for libtest's one announcement line, matched exactly, and is also kept
   raw. The case reads the journal first, then asserts the raw stream's
   non-protocol lines are exactly `["", "running 1 test"]` per invocation,
   every other line a `forge-driver/v1` message, and the whole stream free
   of the same needles as the journal. One mutation, compiled, run red and
   reverted: the bound route's bytes `println!`ed on the adapter's stdout
   after the claim in `adapters.rs` part `resume_tests.rs:2265` on the
   journaled `EffectFailed` (seq 5, error carrying `route4amarker`); with
   the journal call skipped they part the raw-stream assertion at
   `resume_tests.rs:2732`; and under the old `grep '^{'` filter the journal
   search stays green and only `:2732` parts — the gap SEC-2 named. The
   gates `cargo fmt --all -- --check`, `cargo clippy --workspace
   --all-targets --all-features --locked -- -D warnings` and `cargo test -p
   brokkr-runtime --all-features --locked` (461 lib plus every integration
   binary, 0 failed) are green; `brokkr-protocol` is unchanged. **Closes**,
   for the shipped route, A53's exclusion half and B42's second sentence at
   the journal, now including non-protocol stdout. **Stays partial:** A53
   and B42 at the journal for the gated shapes, as above. Tests only; the
   DSH route stays disabled; no checkbox moved.

8. *(recorded as 2d)* **Close the route-binding matrix at the single site.** Added from the
   chief review of run `issue-226-the-8-8-and-8-10-accep-b5a676bf` (C3 +
   SEC-2, 2026-09-23); ordered after 4a. Closes B38 and corrects B36 and
   A37. `tasks.md` 5231–5237 requires **each** named non-binding shape at
   **either** call site. The panel case drives every negative as a panel
   member; units 2a and 2b added only the nonmember and the compiled escaping
   symlink at the single site. Touches
   `crates/brokkr-runtime/src/engine/resume_tests.rs` only. Drive the
   shadow, ancestor-layer file, traversal and absolute expansion through the
   single site. Also add the offered-**panel** positive: the panel positive
   is a cold, unmeasured start, and the offered positive goes through
   `single(...)`. Proof: each negative start's `resume_context` carries no
   `route_overlay`, and the positive carries the manifest's recorded
   digest, both read off the real `Start.input`. `engine.rs:3962–3998`
   keeps canonical containment, so a shape that binds is a Pass-B production
   finding and the unit reports it.

   **Landed 2026-09-23 at `97624edd`; no shape binds — no Pass-B finding.**
   `every_remaining_non_binding_shape_is_withheld_at_the_single_site` runs
   each shape in its own run on a canonical root and collects any context
   carrying a `route_overlay`; the collection is empty. Every vector names
   bytes a lookup could find. The shadow and the ancestor file are spelled
   `route.yml` beside a member of that name, and the ancestor records its
   file in its own `files`. The `..` value (`../work/recipe/route.yml`) and
   the absolute expansion both resolve to the member.
   `a_valid_route_overlay_binds_on_an_offered_panel_member_start_too` sends
   a panel member back in through the phase machine's re-entry, where it is
   offered `alpha-1`, and asserts both starts carry exactly
   `{value: "recipe/route.yml", digest}`. Five mutations, compiled, run red
   and reverted:
   - the absolute-path check removed parts `resume_tests.rs:1993`
     (`absolute` bound with the member digest) and the panel case;
   - the `..` check removed parts `:1993` alone (`traversal`);
   - containment falling back to the argv value parts `:1993` (`shadow`) and
     the panel case;
   - a path outside the layer looked up in its chain ancestor's `files`
     parts `:1993` alone (`ancestor`);
   - the member site withholding the binding on an offered start parts
     `:2357` (`[binding, Null]` against two bindings), every other route
     case green.

   The panel's `traversal` and `ancestor` vectors stayed green under their
   mutations; the first grading recorded this as a residual. `cargo fmt
   --all -- --check`, `cargo clippy --workspace --all-targets --all-features
   --locked -- -D warnings` and `cargo test -p brokkr-runtime --all-features
   --locked` (463 lib plus 94 integration passed, 0 failed) are green. Tests
   only; no production line moved; the DSH route stays disabled; no
   checkbox moved.

   **Returned 2026-09-23 by review C1; answered at `7900c182`.** The panel
   case `a_non_binding_route_overlay_withholds_the_member_at_both_call_sites`
   now runs on a canonical root, with vectors that name findable bytes like
   the single site's: `base/route.yml` recorded in the ancestor's own
   `files`, and `../work/recipe/route.yml`, which resolves to the member. It
   asserts every context is present and that no shape's context has a
   `route_overlay` key, null included. Green on production bytes, so there
   is no Pass-B finding. Three mutations, compiled, run red and reverted:
   the `..` check removed parts `resume_tests.rs:1868` alone (`traversal`);
   the ancestor-`files` fallback parts `:1868` alone (`ancestor`); writing
   `route_overlay: null` when nothing binds parts `:1868` for all six
   shapes, which the old assertion accepted. A37 and B38 are now discharged
   on this evidence, with line references moved to the current file. Same
   gates green (463 lib plus 94 integration, 0 failed). Tests only; no
   checkbox moved.

9. *(recorded as 3b-fix)* **An unreadable version output observes no version.** The production
   remedy for unit 3b's finding below (run
   `issue-226-acceptance-ledger-unit-86b33424`, recorded at `7fb456d2`),
   ordered before 3b's re-run. `tasks.md` 5302–5306 requires unreadable
   version output to be exercised on its own, and `qualify`'s doc comment
   (`adapters.rs:1049–1051`) disables resume on a version "missing,
   unreadable or different". `observed_version` decoded stdout lossily, so
   invalid bytes beside the matching version qualified. Touches
   `crates/brokkr-protocol/src/adapters.rs` (decode strictly; stdout that
   is not valid UTF-8 observes `None`) and
   `crates/brokkr-protocol/src/adapters/tests.rs` (the unreadable vector
   only). The vector: invalid bytes with the matching version, on exit 0,
   on the offered path. It declines `unverified-harness` with `observed`
   `None`, calls the producer zero times (a panicking closure) and plans a
   fresh cold root under the current home. Proof: restoring
   `from_utf8_lossy` in a compiling mutation parts the new vector. The
   valid-UTF-8 positives for codex, claude and dsh stay green unchanged.
   Unit 3b's other vectors stay with 3b, which re-runs whole.

   **The second caller.** `dsh_launch_with` calls `observed_version`
   directly (`adapters.rs:3617`), and only once the gate is open and a
   declared digest is recordable. Strict decoding changes one thing there:
   invalid bytes now leave `observed` `None`. The composite producer is
   then never called, `qualified` stays false, an offer declines
   `unverified-harness` onto a fresh root, and a cold seat plans the
   shipped route with no version or digest recorded. That is the
   fail-closed path the exit-3 and banner vectors already take, so nothing
   beyond strict decoding was needed there.

   **Landed 2026-09-23 at `6a5f1bc9`.** `observed_version` now reads
   `std::str::from_utf8(&output.stdout).ok()?`.
   `a_dsh_identity_mismatch_declines_the_offer_and_keeps_the_cold_route`
   drives `0.1.5-rc.1\n\377\376\n` and `0.1.5-rc.1 \377\n` on the warm
   offer whose unvaried control rejoins. It first checks each shim's exact
   bytes and that they fail a UTF-8 decode. Each shape declines
   `unverified-harness` with `observed` and `wrapper_digest` `None` and
   never reaches the producer. It gets no stream-json, no rejoin, no fold
   boundary and no `--session`, and a root whose parent is the
   canonicalised home's `sessions/brokkr`. One mutation, compiled, run red
   and reverted: `from_utf8_lossy` restored parts `tests.rs:10626`, the
   panicking producer reached on `invalid line after the version`. A
   diagnostic under the same mutation, with the producer answering, parts
   `:10630` on `invalid byte beside the version` (`None` against
   `Some("unverified-harness")`). The first shape qualified the same way.
   `the_version_probe_reads_the_number_out_of_each_measured_banner` (codex,
   claude, dsh) passes unchanged. `cargo fmt --all -- --check`, `cargo
   clippy --workspace --all-targets --all-features --locked -- -D warnings`
   and `cargo test -p brokkr-protocol --all-features --locked` (429 + 99 +
   1 passed, 0 failed) are green. The DSH route stays disabled; no
   checkbox moved.

10. *(recorded as 3b)* **Complete B59's version-output matrix.** Added from the chief review of
   run `issue-226-the-8-8-and-8-10-accep-b5a676bf` (C6, 2026-09-23); ordered
   after 2d. `tasks.md` 5302–5306 asks for absent, malformed and unreadable
   version output and a version-command failure, each exercised
   independently. `a_dsh_identity_mismatch_declines_the_offer_and_keeps_the_cold_route`
   has `exit 3` and one successful `no-version-here` banner, and the matching
   positive does not supply the rest. Touches
   `crates/brokkr-protocol/src/adapters/tests.rs` only. Add distinct cases,
   each on a successful exit: empty output (absent), output that is not a
   version (malformed), and output that is not valid UTF-8 (unreadable). Keep
   the `exit 3` case. Assert the exact reason each declines with. A vector
   ACCEPTED on current production bytes is a production finding: stop and
   report it. B59 is discharged only when all four are distinct and bound.

   **Stopped 2026-09-23 on a production finding; no test landed.** Version
   output that is **not valid UTF-8 is admitted** when it also carries the
   matching version. `observed_version` (`adapters.rs:1005–1031`) reads the
   probe's stdout through `String::from_utf8_lossy` (`:1010`), so the bytes
   `0.1.5-rc.1\n\377\376\n` on exit 0 observe `0.1.5-rc.1`. So does
   `0.1.5-rc.1 \377\n`. On the offered path of the test above, each
   qualifies after one producer call. Each carries no refusal, plans
   `stream_json`, rejoins `session-1` at its own root with `first_seq`
   `Some(3)` and records the digest. This is against `tasks.md` 5302–5306
   and `qualify`'s own contract at `adapters.rs:1049–1051` ("missing,
   unreadable or different … disables resume"). A vector of invalid bytes
   alone (`\377\376\n`) declines, but only because it carries no digit, so
   it collapses onto the malformed case and cannot stand for "unreadable".
   The other three were probed on the same offer, each isolated to one
   property. Each is **not** a finding: empty output on exit 0,
   `no-version-here` on exit 0, and the matching version printed before
   `exit 3` each observe nothing, never reach the producer (a panicking
   closure), and decline `unverified-harness` onto a fresh root under the
   current home. B59's `discharged` grade is withdrawn for its unreadable
   half. The remedy is a strict UTF-8 read of the probe's stdout, where
   invalid bytes observe no version. Then this unit re-runs whole. The
   probes were reverted.

   **Re-run landed 2026-09-23 at `cb5bbd9a`; each vector declines on
   current production bytes — no production finding.** On the warm offer
   whose control rejoins, beside 3b-fix's unreadable vector (not
   duplicated): nothing on exit 0 (absent), a valid-UTF-8 `no-version-here`
   on exit 0 (malformed), the matching version then `exit 3` (failure),
   each shim's status and bytes checked first; each declines
   `unverified-harness`, `observed` and digest `None`, producer unreached,
   fresh root under the current home. Mutations, run red and reverted:
   exit status ignored parts `tests.rs:10632` (`version command failure`);
   digit filter dropped parts `:10636` (`malformed version output`,
   `Some("no-version-here")`), the cold banner at `:10452` silenced as a
   diagnostic; empty output read as the version parts `:10632` (`absent
   version output`). fmt, clippy and `cargo test -p brokkr-protocol`
   (429 + 99 + 1) green. B59 discharged; tests only; no checkbox moved.

11. **Prove the gated shapes' exclusions in one process.** Added by the
   remediation (third return, findings 2 and 4). Closes the half of **A53**,
   **B42** and **B30** that entry 7 (recorded as 4a) did not prove. Entry 7
   proved route-byte and carrier exclusion at the composite and on the
   adapter's wire for all four shapes. It proved them at the journal only for
   the **shipped (closed-gate) route**, through the real adapter under the
   engine. The engine's offered cases journal what the DSH-shaped
   `dsh_model_driver` shim emits. So for **qualified cold, confirmed rejoin
   and declined offer**, the production adapter's publication and the
   engine's journaling are proved in two processes and never in one, and
   B30's "not copied as `resume_context` or `owned_target` into launch
   evidence" has no offered start through the real adapter under the engine.

   Touches `crates/brokkr-runtime/src/engine/resume_tests.rs`, extending
   `the_real_dsh_driver_journals_no_route_byte_and_no_carrier`'s real-adapter
   harness. Tests only. It needs a synthetic DSH install whose composite the
   production producer computes, the protocol suite's `Synthetic` rebuilt as
   a runtime test fixture, and a test-local declaration that measures it
   (`supported`, its `wrapper_digest`), so the adapter's gate really opens.
   On a seat whose `--patch` binds distinguishable valid route content (a
   marked `displayName`, key variable and endpoint), drive through the engine:
   - a qualified cold start;
   - an offered start that rejoins a prior DSH checkpoint (`dsh-session` root
     plus `transcript`) and confirms;
   - an offered start the adapter declines.

   Proof, for each: read every journaled event, launch row, stderr tail and
   the raw driver stdout, and find no route content, path, digest, binding,
   `resume_context`, `owned_target` or `assessment`. The confirmed rejoin's
   launch row keeps `root_session` and `transcript` exactly. A compiling
   mutation that copies route content into a launch-row field under
   **another** name parts the case, and so does one that copies
   `owned_target`. If the gate cannot be opened from the runtime suite without
   a production seam, the unit stops and reports `oversized` rather than
   adding one. A leak on current bytes is a Pass-B security finding, reported
   and not adjusted.

   **Landed 2026-09-23 at `6e1e7066`; every exclusion holds on current
   production bytes, so there is no Pass-B finding and no production seam.**
   `the_real_dsh_driver_journals_no_route_byte_on_the_gated_shapes` drives
   the served production adapter over a synthetic install. Production's
   resolver and producer select and measure it, and the declaration is
   `supported` at that digest. It runs a qualified cold start (`--new`,
   `s-1`), a confirmed `--session s-1` rejoin, and an offer declined
   `unverified-harness` after a version drift, all in one run. The journal,
   the raw stdout, three exact launch rows and two exact stderr tails carry
   nothing of the route or the carriers. The rejoin keeps the cold start's
   root and address. (Corrected below: at `6e1e7066` the declined offer
   succeeded, so its stderr was never journaled and the decline's stderr
   exclusion was unproved.) Mutations, compiled, run red and reverted:
   - `displayName` into `effort` on stream-json parts `resume_tests.rs:2434`
     (seq 6), with the shipped-route case green. Rejoin-only, it parts at
     seq 11. Refusal-only, it parts at seq 17.
   - `owned_target` copied under its own name parts `:3364` (raw stdout).
     The engine's v5 fence refuses those rows, a second layer.
   - Its provider id copied into `effort` parts `:3507` ("the confirmed
     rejoin").

   fmt, clippy and `cargo test -p brokkr-runtime` (464 lib plus 94
   integration) are green. Tests only; no checkbox moved.

   **Review return, 2026-09-23, landed at `471bc740`: a failed declined
   offer journals its stderr, and the exclusion holds on current bytes.**
   Review found the gap above (`Engine::conclude_single` journals no stderr
   for an accepted success). The seat now takes four attempts. The third is
   the declined offer, whose child exits 3, so its tail is journaled and
   asserted exactly (`dsh child 3 wrote this`). The fourth declines the
   same way and succeeds. The failed decline's launch row is asserted
   exactly too. The journal search now also reads payloads with escaped
   quotes undone, because a carrier copied into a stderr tail serialises
   as `\"owned_target\"` and the quoted needles never matched it.
   Decline-only mutations in `adapters.rs`, compiled, run red and reverted
   (an `eprintln!` in the open gate's decline branch; the shipped-route
   case stays green):
   - The overlay value on stderr parts `resume_tests.rs:2439` (seq 19
     carries `recipe/route.yml`). The pre-return test passed under it.
   - `owned_target` on stderr parts the exact tail at `:3477` ("attempt
     3") before the search was hardened, and `:2439` after it (seq 19
     carries `"persistence_home"`).

   fmt, workspace clippy and `cargo test -p brokkr-runtime --all-features`
   (464 lib plus 94 integration) are green. Tests only; no checkbox moved.

12. **Assert N2's and N4's two unasserted cells.** Added by the remediation
   (third return, finding 2). Touches
   `crates/brokkr-protocol/src/adapters/composite/tests.rs` and
   `crates/brokkr-cli/src/doctor/tests.rs`. Tests only.
   - **N2, "no second search"** (`tasks.md` 1261–1262). In
     `the_selected_invocation_is_not_replaced_by_its_canonical_target`, run
     the admitted launcher's `selected.invocation.command()` with `PATH`
     emptied and assert it still prints the alias path. The invocation then
     needs no search to run. A compiling mutation that makes the invocation
     carry the bare searched name parts it.
   - **N4, "injected doctor probes"** (1289–1290). A `doctor/tests.rs` case
     feeds `dsh_provider_line_with` the real `select_in` over staged bare and
     blank env launchers, with probe and producer closures that panic, and
     asserts the line carries the exact missing-program cause. Restoring
     blank-tail `Ok(None)` in a compiling mutation parts it. That also gives
     M4 the "callback test" failure its record lacks.

   Expected to be evidence only. A pass is not a finding. A search, a probe
   or an admission on current bytes is a production finding, and the unit
   reports it.

   **Landed 2026-09-23 at `36b16922`; both cells hold on current production
   bytes, so there is no production finding.** N2: the admitted launcher's
   invocation runs with `PATH` emptied from a cwd holding no `dsh` and prints
   the alias; `invocation.program` set to the bare name in `lookup_in`, and
   separately `command()` running `argv0`, each part `composite/tests.rs:3040`
   (`Err(NotFound)`). N4:
   `an_env_launcher_without_a_program_reaches_neither_doctor_callback` drives
   the real selection (`DshSeams::selected`, in a re-executed child, since
   `select_in` is private to the protocol suite) over bare and blank env
   launchers with panicking callbacks and asserts the exact missing-program
   cause; blank-tail `Ok(None)` parts the child's probe at
   `doctor/tests.rs:2836` with nothing spawned, M4's callback failure. Tests
   only; fmt, clippy, `-p brokkr-protocol` and `-p brokkr-cli` green; no
   checkbox moved.

13-fix. **The Apple arm remembers only what native remembers.** Added for
   finding P1, which entry 13 stopped on (run
   `issue-226-acceptance-ledger-entr-61b7a860`). It is a production fix and
   comes before 13, which still owes its pins. At Libc-1752.120.2
   (`4e34d055`), `sys/posix_spawn.c` 178–193 and `gen/FreeBSD/exec.c`
   273–289 read `if (stat(bp, &sb) != 0) break;`. A candidate whose
   METADATA cannot be read (for example a `PATH` directory without search
   permission, where `stat` fails with EACCES) is walked past and NOT
   remembered. EACCES is remembered only when `stat` SUCCEEDS and the later
   access or regular-file check fails. An otherwise empty search then ends
   in ENOENT (posix_spawn.c 195–204, exec.c 293–306). The port remembered
   every Apple EACCES: `step` in `composite.rs`, the doc comments on
   `Library::Apple` and on `step`'s Apple arm, and the assertion in
   `the_lookup_rule_is_each_librarys_own_switch_arm_by_arm`. It reached
   this through `classify_in`'s metadata EACCES → `Passed { denied: true }`
   → `Search::find`'s "is not executable by this process", where native
   macOS reports NotFound.

   **Fix.** Make the Apple arm depend on WHICH question failed, not on the
   errno alone. A metadata failure is walked past unremembered. An access
   or not-regular failure on a file whose metadata was read is remembered
   as the denial. The Linux/glibc and musl arms do not change. Correct the
   doc comments to cite the pinned lines.

   **Tests.** `composite/tests.rs` (correct the assertion to native's
   reading) and `composite/tests/native_matrix.rs`. Add the cell no test
   has: a sealed directory (no search permission) as the ONLY Apple
   candidate, which must end NotFound, and as the LAST candidate after a
   readable non-executable file, which must end with that file's denial.
   Fixtures build on a canonicalised temporary root, and the sealed
   directory has its permission restored before cleanup.

   **Proof.** Restore errno-only remembering in a compiling mutation; the
   new cells fail; record the test and assertion; restore. Selection is
   unchanged: every existing positive stays green. If the Linux or musl
   arms would ALSO have to change to pass, the unit stops and reports.

   **Files.** `crates/brokkr-protocol/src/adapters/composite.rs`
   (production: the Apple arm and its comments only),
   `composite/tests.rs`, `composite/tests/native_matrix.rs`.

   **Landed 2026-09-23 at `1d2763cf`; the glibc and musl arms did not
   move.** `step` takes the failed question. Apple's EACCES is remembered
   only for `access`, and `metadata` walks on unremembered. The
   not-regular denial never reaches `step`. The signature change reaches
   both call sites, the second of which passes `metadata` to
   `refuse_working_directory` (stop-only, unchanged). `classify_in`'s
   comment is corrected with the other two.
   `apple_walks_past_a_sealed_directory_without_remembering_it` covers
   three libraries × two operations. On Apple, a sealed directory alone
   ends "is not on PATH". Sealed last after a 0644 file ends in that
   file's denial, and a direct name gets one answer on every arm.
   Mutations, compiled, run red and reverted:
   - errno-only remembering parts `composite/tests.rs:2458` (Apple under
     Exec, sealed alone) and `:7328` (the step table).
   - errno-only forgetting parts `:2458` (sealed last), `:7332` and
     `:8234`.

   The native matrix's new `sealed-directory-alone` and
   `non-executable-then-sealed-directory` controls pass on glibc. Their
   Apple expectation is **pending the macOS leg**: Linux runs the glibc
   arm, so neither mutation reaches them here. The direct-name audit row
   for a metadata EACCES now records the remembered flag per library, with
   the words unchanged. fmt, workspace clippy and `cargo test -p
   brokkr-protocol --all-features --locked` (430 + 99 + 1) are green. No
   checkbox moved.

   **Review return answered 2026-09-23 at `4ce6eba2`** (run
   `issue-226-acceptance-ledger-entr-bda73e1f`, C1: the two sealed
   controls ran only in the explicit form). Both controls now also run in
   the INHERITED form. The parent stages the fixtures and each sealed
   `PATH` in a child of its own, which calls `Command::new("dsh")`
   (`posix_spawnp`) beside `resolve_executable`. They are declared as
   `control:<what>-inherited`, and the explicit controls are retained. One
   `Sealed::control` asserts native's exact errno and the whole refusal
   for both forms. An `Unseal` guard restores the permission before
   cleanup. Mutations, compiled, run red and reverted, all on Linux's
   glibc arm:
   - glibc given Apple's metadata rule fails both sealed-directory-alone
     forms at `native_matrix.rs:272` ("not on PATH" != "not executable").
   - glibc's access denial suppressed fails both
     non-executable-then-sealed-directory forms at `:272` (the sealed
     candidate's denial != `controls-readable/dsh`'s).
   - running only the first sealed child fails the inventory, missing
     `control:non-executable-then-sealed-directory-inherited`.

   The Apple rows of both forms remain **pending the macOS leg**. fmt,
   workspace clippy and `-p brokkr-protocol` (430 + 99 + 1) are green. No
   production file moved and no checkbox moved.

13. **Retrieve, pin and verify the Apple and env sources.** Added by the
   remediation (third return, finding 1). **Externally owned**: it needs a
   seat or host whose grant reaches the network. Every seat of this change so
   far was refused `curl`, `WebFetch`, `gh api` and the GitHub MCP (`tasks.md`
   2389–2394, 2425–2431). The sources are **retrievable, not impossible**.
   Closes N1's and N3's source-pin predicates (1248–1250, 1284–1286) and
   N4's "applicable source-pin cells" (1304–1305).

   **Sources**, each at an immutable revision (a release tag with its commit
   SHA), with the file's SHA-256 recorded:
   - Apple Libc `gen/FreeBSD/exec.c` and `sys/posix_spawn.c`
     (`apple-oss-distributions/Libc`). The port cites design D10's
     moving-`main` ranges 178–218 and 262–297, and 97–143 and 170–195
     (`composite.rs:2617–2631`; design.md 2953–2960).
   - The env dispatch sources the resolver's rule names
     (`composite.rs:3598–3608`): GNU coreutils `src/env.c` and
     `src/coreutils.c`, Apple's `env/env.c`, uutils `src/bin/coreutils.rs`
     and busybox `libbb/appletlib.c`. The code says `usr.bin/env/env.c`;
     proposal.md 3842 links `apple-oss-distributions/shell_cmds`
     `env/env.c`. The retrieval records which path the pinned revision holds.
   - The Linux kernel's `fs/binfmt_script.c`, which the same doc comment
     cites for `argv[0]`.

   Hosts are Linux and macOS only (decision 0063), so every source above
   applies and no Windows source is owed. glibc's `posix/execvpe.c` is cited
   to the fixed 2.42 release and N1/N3 do not name it, so it is outside this
   unit.

   **Files it updates.** The pins go "beside the implementation" (design.md
   2958–2959): the doc comments at `composite.rs` 2329–2331, 2367–2368, 2417,
   2617–2631, 2728, 2817, 3104, 3154–3155 and 3598–3608; the source comments
   at `composite/tests.rs` 2360, 7157, 7476, 7703 and 7799–7800; and
   `composite/tests/native_matrix.rs` 210, 607–608 and 861. A dated evidence
   file goes beside `controller-evidence-2026-09-10.md` in this change's
   directory, with each source, tag, commit, file digest and the exact line
   ranges re-read against the port. A delivery record in `tasks.md` under
   8.8.1.1 and 8.8.2.1 cites it.

   **Proof.** Every ported block's cited range is re-read at the pinned
   revision. Where the port's reading differs from the pinned source, that is
   a production finding, and the unit stops and reports it rather than
   editing the port to match. It comes before entry 14 because it moves
   bytes.

   **Kept separate: an operator scope change.** The operator may instead rule
   that N1 and N3 tick without the pins, as inherited debt recorded in prose.
   That would be a change to 8.8's acceptance, not satisfaction of it. It is
   prepared in entry 23, and it does not replace this entry unless the
   operator says so.

   **Landed 2026-09-23 at `fcb91ad2`, from host-retrieved sources (run
   `issue-226-acceptance-ledger-entr-a5dcdf46`).** All sixteen digests
   were re-hashed locally, and every cited range was re-read against the
   port. `source-pins-2026-09-23.md` records each source, tag, commit,
   SHA-256 and range. The first visit stopped on P1, which was repaired
   by 13-fix. The second visit found P2, which is not a production
   finding: the renamed-symlink refusal comes from Ubuntu's AppArmor
   patch on uutils 0.2.2, and upstream has no such check. The refusal is
   kept, and only its attribution was corrected. Only comments moved:
   - `strchrnul` replaces `strsep`;
   - the pinned ranges replace D10's moving-`main` ones;
   - the default search is attributed to `_execvpe`;
   - `paths.h` is cited at line 65;
   - the xnu `PATH_MAX` is cited;
   - Apple's env is cited at `env/env.c`.

   `tasks.md` records the delivery under 8.8.1.1 and 8.8.2.1. design.md's
   matching prose (2954–2957, 2988, 2996–3006 and 3036–3045) is recorded
   for entry 22. Native macOS is still pending. fmt, workspace clippy
   and `-p brokkr-protocol` (430 + 99 + 1) are green. No checkbox moved.

14. *(recorded as 4)* **Run the gate list, in order, on the final
   candidate.** Closes S11 (8.8.14.2), **N12 (8.8.8.2)** and B10's gate half.
   It also runs every assertion entries 1–12 added. Touches nothing. It is an
   execution, recorded in a delivery section by entry 22. Commands, in the
   clause's order:
   1. `cargo fmt --all -- --check`
   2. `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
   3. `cargo test -p <crate> --all-features --locked` for `brokkr-core`,
      `brokkr-store`, `brokkr-protocol`, `brokkr-runtime`, `brokkr-view`,
      `brokkr-bridge` and `brokkr-cli`, each completing before the next
   4. `openspec validate --all --strict`
   5. `cargo run --locked -p brokkr-cli -- compile --bundle bundles/self`
   6. `… bundles/verify`

   **Externally owned prerequisite:** a seat or host whose permission grant
   reaches `openspec` (F6). Review and chief grants have reached it five
   times. Implement, ledger and remediation seats have not. An unavailable
   tool is not a pass. It must follow entries 1–13 so it runs on the head
   that will be ticked. If 13 is still pending when the operator wants this
   run, it may run first, but any byte 13 later moves requires this entry
   again.

   *(Corrected, third return, finding 5.)* This entry first said the
   crate-scoped `cargo test -p brokkr-cli` gives `doctor_dsh_selection.rs`
   "its first execution" in this change. It also said this would be "the
   first time clippy and both bundle compiles are run on a Pass D head".
   Both were false. The suite has historical recorded executions, clippy
   passed on D1's `4d6b15f3` and D3's `69cac25d`, and D3's seat recorded the
   self bundle (see *Recorded execution*). What this entry adds is the
   **complete ordered list on the one candidate that will be ticked**, which
   no seat has yet recorded. That list has `openspec` and `bundles/verify` in
   it, and it runs `doctor_dsh_selection.rs` on that candidate, which is what
   N2, N4, N5b, N5c and N9 wait on.

15. *(recorded as 5)* **Obtain the external evidence.** Closes **N13
   (8.8.8.3)** and supplies B26's macOS half and N2's and N4's macOS legs.
   Touches nothing. Run `TMPDIR=/tmp bash scripts/coverage-exact.sh` on CI or
   a host that can create the boundary namespace. Record the committed
   revision, the environment and the actual covered/total lines, branches and
   functions, with the denominator reconciled against chief `124cca78`'s
   fresh baseline. Then the native macOS leg, and remote CI on the final
   candidate head. **Externally owned** throughout. Record each as pending
   until its own result exists, and never as passed.

   Three corrections to how the first cut framed this (C5, C8):
   - **(a) It is not a prerequisite of entry 22.** S12 asks that external
     evidence be *recorded*, "when supplied, otherwise explicitly
     **pending/unavailable**" (314–326). So entry 22 closes over a pending
     entry 15 by reporting it pending.
   - **(b) The coverage leg starts from a red result, not from nothing.** The
     last literal gate run on this change exited 1 (F7, N13). Whoever runs it
     should expect to reconcile 32,626/32,802, not to confirm a formality.
   - **(c) The macOS leg closes N2's, N4's and B26's macOS halves.**

   N11's positive and its two removals moved to entry 16. They need a host
   with `node` on its default search path, which is a different requirement
   from macOS, and the removals are work, not a run. The first cut's "What no
   unit can reach" paragraph on the source pins is withdrawn: entry 13 reaches
   them.

16. **Run N11's positive and perform its two removals on a capable host.**
   Added by the remediation (third return, finding 2). **Externally owned**:
   it needs a Linux or macOS host with a `node` executable on the platform's
   default search path, which is `/bin:/usr/bin` on the hosts recorded so far
   (`tasks.md` 2398). Closes N11's positive and both removals (1477–1480).
   Before this unit they were **never performed**, not merely unverified.
   Touches nothing in the tree.
   - Run `composite/tests.rs::absent_path_node_identity_is_retained_by_the_composite`
     and `doctor_dsh_selection.rs::absent_path_default_search_matches_native_dsh_and_node`,
     each `--exact`. Require the `Ok` arm, with no `PENDING` line in the
     output.
   - Then perform, one at a time, the two compiling removals the test's doc
     names (`composite/tests.rs:3505–3508`). Restore unconditional absent-PATH
     refusal: the positive fails while the native child still succeeds. Retain
     a distinct wrong Node: the retained-identity assertion fails. Restore each
     exactly and rerun green.

   Record the host, `node`'s path and version, each command and each failing
   assertion verbatim in a retained log that entry 23 can open. "Shell failure
   or NotFound is no positive" (1479–1480). A positive that fails on current
   bytes is a production finding.

17. *(recorded as 9)* **Put the recorded-removal class to the operator as
   one question.** Closes the evidence-verification gap on **N2, N4, N5d,
   N6a, N6b, N7–N10, A61, S1, S3–S7, S8a–c, S9a–e, S10, B5, B16, B20 and
   B77**. That is twenty-nine rows (counted in F5). It also covers the
   recorded mutations of entries 1–10, which B5's "every new test" reaches.
   Touches `tasks.md` only, and only to record the ruling once it is given.
   It comes after entry 14, so the ruling is taken against a fully gated
   candidate, and before entries 22 and 23, which cannot tick over it.

   *(Third return, finding 2.)* N11 is **not** in this class. Its positive
   and removals were never performed, so there is no record to rule
   sufficient (entry 16). N2, N4, N6a and N6b joined the class: their
   removals are records (§1). The previous revision's count, "twenty-five",
   was wrong for the twenty-six rows it listed.

   The question, stated so it can be answered yes or no: *a compiling
   mutation run by a named seat on a named revision, recorded with the
   assertion that parted and the restored pass but leaving no artefact in the
   tree — is that acceptance for a clause that asks for an "observed"
   removal?* The operator may answer per row, and a ruling covers only the
   rows it names. For 8.10 it must name B5, B16, B20 and B77, and through B2,
   S1 and S3–S10. Anything short of that leaves 8.10 on entries 18, 19 and
   21(b). No ruling covers the controls B5's inventory found no record of.
   Those are entry 21(a)'s whatever this entry rules.

   The unit prepares the answer and does not take it. It presents:
   - the mutation ledgers: Pass C's fourteen (465–480; this line said
     "twelve", corrected 2026-09-23); D1's two; D2's seven;
     D3's six plus one discarded; the returned review's two; the R1–R4 group's
     M1–M9 (1665–1673), D1–D5 (1920–1922, 2057–2058) and the earlier M1–M12
     (12366–12383); entries 1–10's own records under §6; `4a3854ca`'s
     three recorded Codex removals, which name "the named test" but no
     assertion; and task 10.5's M8, M8a, M9 and M9a (10230–10233), which
     answer D10's absence binding at a named assertion (B5, inventory (i));
   - the confirmation that **no mutation survives** in the tree
     (`git diff origin/main`), and that every case the mutations aimed at
     exists, asserts what is claimed and passes.

   It does not rest on `tasks.md:1248`'s "Adopt resolver removals without
   replay". That sentence governs 8.8.1.1's resolver alone (F5).

18. **If 17 rules no: replay the terminal-body removals.** Added by the
   remediation (third return, finding 3). Covers S1, S3–S7, S8a–c, S9a–e,
   S10, A61 and B77, and B5's share for the Pass C tests. Touches nothing that
   survives. On the final candidate, apply each of Pass C's fourteen recorded
   mutations (`tasks.md` 465–480: M1a, M1b, M2, M3, M4, M5, M5b, M5+M7, M6,
   M6′, M8, M9, M10, M12) to `adapters.rs`, one at a time. *(Corrected
   2026-09-23: this said "twelve"; the table has fourteen rows.)* Run its
   named `adapters/tests.rs` case, capture the failing assertion verbatim,
   restore and rerun green. For S6, record that the failure is the intended
   terminal assertion and **not a timeout**, with the elapsed time. The logs
   are retained in a dated evidence file in this change's directory, where
   the next grader can open them. A mutation that no longer parts its case is
   a finding, not a skip.

19. **If 17 rules no: replay the Codex bridge and conformance removals.**
   Added by the remediation (third return, finding 3). Covers **B16's four**:
   separately observed disabled-status, boxed-hands, harness-fragment and
   boundary-mark mutation failures, against
   `engine/boundary_tests.rs::the_shipped_codex_harness_work_seat_composes_the_preserved_rejoin`
   and `tests/driver_conformance.rs`'s two shipped-Codex rejoin cases
   (`tasks.md` 5121–5125). It also covers **B20's** selector mutation, paired
   with `a_refused_resume_is_a_cold_spawn_with_the_refusal_journaled`, which
   must show two children, no selector, the retained sandbox and one cold
   launch row (5131–5135). This is B5's share for those suites. Same method
   and evidence file as entry 18.

20. **If 17 rules no: replay the R1–R4 group's removals (8.8 only).** Added
   by the remediation. Covers N2 (M5, M6), N4 (M4, with entry 12's
   callback-test half), N5d (M1–M3, M7–M9), N6a (M8), N6b (D1–D5), N7 (the
   component-ordering mutation), N8 (the restored-oracle mutation), N9 (the
   `Safe` removal) and N10 (the filename-context removal). The records are at
   `tasks.md` 1665–1673, 1920–1922, 2057–2058 and 12366–12383. Same method.
   8.10 does not wait on this entry; 8.8 does.

21. **Perform THE PROOFS' unrecorded controls, and, if 17 rules no, replay
   the remaining new tests' removals.** Added by the remediation (third
   return, finding 3). Split in two by the fourth return (finding 1), because
   B5's inventory found controls with no record, and a ruling cannot accept
   those. Touches nothing that survives. Same method and evidence file as
   entry 18. A control that does not part its named assertion is a finding,
   not a skip.

   **(a) In every case, whatever 17 rules.** Covers the B5 controls for which
   no record was found (B5's inventory (i)–(vii); `tasks.md` 5095–5099,
   9449–9505):
   - apply one common marker mutation, dropping the no-hands marker
     (`mark_hands`'s `NoHands` arm, `engine.rs:1237–1240`), and observe an
     **unwrapped** shape of
     `driver_conformance.rs::the_compiled_live_inline_codex_shapes_rejoin_their_provider_confirmed_root`
     (`:3194`) fail its decision assertion (9455–9456). The loop reaches an
     unwrapped shape only after the wrapped one passes, so run the unwrapped
     shape alone by a temporary, restored narrowing that weakens no
     assertion, and record the narrowing verbatim. *(Fifth return, C1: this
     bullet sent the absence refusal to these cases. Task 10.5 recorded that
     control against the protocol matrix, so it moved to 21(b).)*
   - in `driver_conformance.rs::the_compiled_hands_inline_codex_shapes_refuse_unavailable_confinement`
     (`crates/brokkr-cli/tests/driver_conformance.rs:3297`), isolate the
     **unwrapped** `HandsMember` iteration (`:3300–3301`) by a temporary,
     restored narrowing that weakens no assertion, and record the narrowing
     verbatim. Apply the hands-to-no-hands mutation: a hands-bearing member
     falsely marked no-hands (`mark_hands` has separate `Hands` and `NoHands`
     arms, `engine.rs:1226–1240`). Observe the direct
     `resume_refusal == "restrictions-unavailable"` assertion (`:3352–3355`)
     fail, then restore and record the passing rerun (`tasks.md` 9452–9456).
     This bullet is unconditional. B5 and entry 22 keep it as a prerequisite
     through 21(a). `4a3854ca`'s record does **not** establish this failure:
     the loop runs the wrapped shape first, so "broke the named test" is at
     most the wrapped shape's. *(Residual C1 of run
     `issue-226-acceptance-ledger-reme-d530d5d2`, operator-accepted
     2026-09-23.)*
   - mutate `assemble`'s second verify diagnostic, and observe
     `bundle/tests.rs::a_dialect_wrapped_verify_select_reaches_the_single_or_panel_refusal`
     (`:220`) fail its "requires a single or panel verify seat" assertion;
   - mutate `parse_selected_body`'s empty-agent diagnostic or its
     propagation, and observe `::a_selected_agent_case_keeps_its_empty_reference_cause`
     (`:254`) fail its "agent must be a non-empty string" assertion;
   - disable `owner_index`'s existing-label refusal in **both** census
     invocations, observe `::a_literal_phase_that_aliases_a_selected_case_is_refused_globally`
     (`:1271`) fail its collision assertion while its rename-only
     construction stays valid (9497–9502);
   - for `::a_raw_phase_that_aliases_a_wrapped_panel_member_is_refused`
     (`:1349`) and `::a_literal_phase_that_aliases_the_injected_validator_is_refused`
     (`:1387`), remove the refusal each test names, one at a time. For
     `:1349` that is the authoring census `4daaa7d2` runs before the wrapper
     renames an address. For `:1387` it is the claim on the injected
     validator's address. Disable any backstop that would mask either, as
     9497–9502 does for the census control, and observe each test's
     "addresses two different sites as" assertion fail;
   - for `bundle/tests.rs::a_wrapped_verify_panel_leaves_an_unrelated_literal_phase_untouched`
     (`:1425`), replace `relocate_verify_facts`' exact source/destination
     pairs (`bundle.rs:2927`) with a sweep over every label under the wrapped
     seat's `verify:` prefix, and observe `:1448` or `:1455` fail. A census
     refusal, or the `unwrap` panic at `:1447`, is a masked failure (9460),
     not the claimed assertion;
   - for `::a_wrapped_panel_drains_overlapping_member_addresses_without_overwrite`
     (`:1469`), insert each destination as soon as its source is removed,
     instead of staging every source first (`bundle.rs:2931–2941`), and
     observe `:1491` or `:1501` fail;
   - for `engine/tests.rs::a_selected_single_publishes_its_own_confinement_at_dispatch`
     (`:188`), remove the dispatch-time re-mark
     `self.mark_hands(&site_name, &mut input)` (`engine.rs:1111`), and observe
     `:266`'s boundary assertion fail. *(Fifth return, C2: these three
     bullets are new.)*

   Restore each exactly and rerun green. 8.10 waits on this half in every
   case.

   **(b) If 17 rules no.** B5's remainder: D1's two, D2's seven, D3's six and
   the returned review's two recorded mutations (616–623, 803–816, 992–1008,
   1058–1074); the compiling mutations entries 1–10 recorded under §6; and
   `4a3854ca`'s three recorded Codex removals (whole relocation disabled, the
   no-hands marker dropped, a hands member falsely marked no-hands), each
   against its named assertion in the two `driver_conformance.rs` cases
   above; and task 10.5's M8, M8a, M9 and M9a (10230–10233), each against
   `adapters/tests.rs::a_supported_assessment_without_both_affirmative_markers_declines`'
   decision arm (`:2086–2089`), with the failing case named. If one visit cannot hold it, it splits at the protocol/runtime
   crate boundary, and the ledger records the split.

   **(a) performed 2026-09-23 at `90b548e3`, on `be1ecf77`'s bytes (run
   `issue-226-acceptance-ledger-entr-4b36a6f8`); no byte survives. Not
   landed: stopped on four findings.** `removal-controls-2026-09-23.md`
   records all ten controls. Each has its diff, any narrowing or backstop
   removal, the verbatim failure and the restored green rerun. Six part
   their named assertions:
   - hands→no-hands on the unwrapped `HandsMember` at `:3352`, where the
     boxed member rejoined;
   - `bundle/tests.rs` `:241`, `:270`, `:1448` and `:1491`, and
     `engine/tests.rs:266`.

   Four do not, and each is a finding (the evidence file's **Findings**):
   - **F1**, control 1. The dropped no-hands marker parts both unwrapped
     live shapes at the cold `root_session` assertion (`:3223`), before
     the retry decision assertion (`:3238`). A `Disabled` gate runs no
     version probe, so no root is recorded and the engine makes no offer.
     That explains the failure, but it is not the commissioned decision
     assertion, and entry 22 cannot accept a substituted one. *(Exception,
     2026-09-23: the operator's ruling accepts the substituted assertion
     for exactly F1–F4, and for no other control. See the record below.)*
   - **F2–F4**, controls 5, 6 and 7 (`:1271`, `:1349`, `:1387`). Each
     removal lets the alias compile. So each fails in the shared `error()`
     helper's "expected compilation to fail" (`bundle/tests.rs:7`), and
     the collision assertion (`:1292`, `:1377`, `:1414`) never runs.
     `:1271`'s rename-only construction passed under the mutation as a
     temporary verbatim copy. `:1387`'s claim was masked by the final
     walk alone, which was then disabled and recorded.

   No test was weakened. Closing F1–F4 needs an operator ruling on what
   these controls must show, or a test repair that re-opens 14. Gates on
   the restored bytes: fmt, workspace clippy, `-p brokkr-runtime`
   (464 + 94), `-p brokkr-cli` (468 + 318) and `git diff --check`. No
   checkbox moved.

   - 2026-09-23, review return of the same run (C1–C3): the first cut's
     "nine part" became six, with F1–F4 recorded as findings, and 21(a)
     reopened. Controls 2 and 10's failures are now verbatim in the
     evidence file, no longer abridged. Docs only, at `7e3454ce`.

   **(a) closed 2026-09-23 under the operator's ruling (run
   `issue-226-acceptance-ledger-entr-a18ca61d`). 21(a) has landed.** The
   operator ruled "accept the substituted assertions". In substance: for
   F1–F4, a control is proof when its test fails at the FIRST assertion
   that depends on the removed refusal or marker, with that failure quoted
   verbatim, in place of the assertion this ledger named, because the named
   assertion checks behaviour the removal prevents from happening.
   `removal-controls-2026-09-23.md` records the ruling (**Operator
   ruling**) and reclassifies F1–F4 as controls passed under it, each with
   its verbatim failure and restored pass:
   - control 1 at the cold `root_session` assertion (`:3223`), both
     unwrapped shapes;
   - controls 5, 6 and 7 at `error()`'s "expected compilation to fail"
     (`bundle/tests.rs:7`), called at `:1291`, `:1371` and `:1408`.

   All ten controls are closed. The ruling covers those four and no other
   control: for entries 18–21(b), a control that does not part its named
   assertion is still a finding, and entry 22 still accepts no substituted
   assertion outside F1–F4. No test, production or frozen byte moved, and
   no checkbox. 8.10 stays unticked: entry 22 still waits on entry 17 (or
   18, 19 and 21(b)), entry 14 and the rest of its list.

   - 2026-09-23, 21(a)-close: the ruling recorded and F1–F4 reclassified
     as controls passed under it, in the evidence file and this ledger.
     Docs only, at `a60631dd`. Gates: `git diff --check` and
     `cargo fmt --all -- --check` pass; `openspec validate --all
     --strict` was refused by this seat's grant and is unrun here.

22. *(recorded as 6)* **Regrade, record, tick and commit the D7/Pass-C/Pass-D
   account.** Closes S12 (8.8.15.1), S11 (8.8.14.2) beside entry 14's record,
   and **8.10**, but not 8.8. Touches
   `openspec/changes/2026-09-09-226-session-resumption/tasks.md` and this
   ledger.

   **Before any tick**, regrade on opened evidence every row that still
   carries its pre-unit grade while a landed entry claims to close it: A42,
   A67, B27, B39, B47, B49, B50, B60, B71 and B99, and A13, A64 and B13,
   which point at them. It ticks 8.10 only if **all** of these hold:
   - entry 14 is recorded green;
   - entry 11 has landed, closing B42's and B30's open half;
   - entry 21(a) has landed, both unwrapped Codex controls included (the
     no-hands marker at `:3194` and the hands-to-no-hands marking at
     `:3297`); *met 2026-09-23: the `:3297` control parted its named
     assertion, and the `:3194` control (F1) passed under the operator's
     ruling on F1–F4, which is this list's only accepted substitution;*
   - entry 17's ruling names every 8.10 removal predicate — B5 over every
     suite, B16, B20, B77, and S1 and S3–S10 through B2 — or, after a no,
     entries 18, 19 and 21(b) have landed;
   - every 8.10 row reads discharged, withdrawn by decision 0063, or not this
     slice's by its own words.

   *(Third return, finding 3: the previous cut let 8.10 tick after a negative
   ruling's two-row fallback, while other clauses stayed open.)* The unit
   then writes a delivery section recording entries 11–21 with their actual
   commands and results. It ticks 8.8.14.2, 8.8.15.1 and 8.10 beside their
   evidence, and leaves 8.8, 9.6, 11.x, §1's numbered rows and groups 14 and
   15 untouched. Proof: `git diff --check`, `openspec validate --all
   --strict`, the requirement/checkbox inventory, frozen surfaces unchanged,
   0056 still `proposed` and the DSH route still `unmeasured`. Then one
   commit, never pushed.

23. *(recorded as 7)* **Reconcile the R1–R4 numbered group and prepare its
   ruling.** Closes **N14 (8.8.8.4)** and, with it, whichever of N1–N4 and
   N11 are closable. Touches `tasks.md` only. It is a separate commission
   from entry 22 because it answers a different question. 8.8.8.4 says "Tick
   a task only when its **entire** acceptance is met; broad owners with
   inherited pending predicates stay open with current repair delivery
   recorded in prose". 1209–1212 forbids a seat from narrowing full
   acceptance to earn a tick. It follows entries 13, 15 and 16, and 17 (or
   20).

   For each of N1–N4 and N11 it records the delivered repair (§1) and the
   evidence those entries produced: the pinned sources, the native macOS
   results, N11's positive and removals. It ticks a row only where the whole
   of that row's acceptance is now met. N11 also needs the consolidation
   record its clause asks for.

   **Kept separate: the scope change.** For any row still waiting on
   evidence that could not be obtained, it puts an operator question: tick on
   the delivered half with the predicate recorded as inherited debt, or stay
   open. It says plainly that a yes changes 8.8's acceptance. A yes does not
   satisfy it.

24. *(recorded as 8)* **9.6's warm retained-store integration, starting with
   the fold.** Not part of 8.8 or 8.10. It is listed because it is the debt
   those two hand on, and it comes last because it follows both ticks. Its
   own first words are "After 8.8 and 8.10" (5499). Touches
   `crates/brokkr-protocol/src/adapters.rs` and `adapters/tests.rs`:
   `find_dsh_transcript` and `names_the_seats_own_session` must select the
   OFFERED root's transcript, not the first depth-zero one (F4). Proof: a
   confirmed rejoin beside an unrelated retained sibling folds the offered
   root's current work, and a compiling mutation restoring first-match
   behaviour parts that case. It carries a `proposed` decision only if it
   changes semantics beyond the repair.

## 7. The three answers

- **8.8 — No.** It ticks only after entries 11–16 land, entry 17 rules every
  recorded removal sufficient (or entries 18–21 replay them), entry 22
  regrades and records, and entry 23 closes N1–N4, N11 and N14 on the
  retrieved pins and native results. Ticking N1 and N3 without the pins would
  be an operator scope change, not satisfaction.
- **8.10 — Yes, conditionally.** It ticks only after entries 11, 14 and
  21(a) land and entry 17's ruling covers every 8.10 removal predicate: B5
  over every suite, B16's four, B20's one, B77, and S1 and S3–S10 through B2.
  After a negative ruling, it waits for entries 18, 19 and 21(b) instead.
  In either case entry 22 regrades the stale rows and ticks it. No seat may
  tick it otherwise. *(2026-09-23: entries 11 and 21(a) have landed, 21(a)
  under the operator's ruling on F1–F4. Entries 14 and 17 remain.)*
- **9.6 — It waits on both ticks**, its own "After 8.8 and 8.10" (5499). So it
  waits on everything above, entry 13's source pins and entry 16's host run
  included, and then on its own unstarted work, starting with entry 24.

**8.8, in detail.** 8.8's acceptance includes the fourteen numbered tasks at
1232–1548, graded in eighteen rows, of which **eight tasks are open** (§1).
Entries 14 and 22 close 8.8.14.2 and 8.8.15.1. Entry 11 closes A53's open
half. Entry 12 closes N2's and N4's unasserted cells. Entry 17 (or 20) rules
the recorded removals of N2, N4, N5d, N6a, N6b and N7–N10. What then remains
is external, not impossible. **N1's and N3's Apple and env source pins are
blocked retrieval** (entry 13): the previous revisions called them "cannot be
produced by anyone here", and that is withdrawn (finding 1). **N2's and N4's
macOS legs need a host** (entry 15). **N11's positive and its two removals
have never been performed** (entry 16). Entry 23 then reconciles N14. The
operator may instead rule N1/N3 closable on their delivered half, but that is
a change to the acceptance, and 1209–1212 and 8.8.8.4 reserve it to them.
The first cut answered "yes" because it had not inventoried the numbered rows
at all.

**8.10, in detail.** 8.10 scopes itself to "execute **only**
8.8.9.1–8.8.15.1" (5089–5093), so §1's open numbered tasks are not its
acceptance. Its behavioural matrices were reduced to evidence entries, and
all of 1–10 have landed. Entry 11 remains for B42 and B30. Several B rows
await entry 22's regrade against those landings. Its gate half (B10) rides
entry 14. Its one undischargeable clause (B101) is conditional, by its own
words, on an extension nobody is permitted to create. What is not settled is
the removal class. B5 says "**Every** new test needs an **observed**
compiling mutation failure at its claimed assertion and a restored pass",
beside B16's four separately observed failures, B20's paired selector
mutation, B77, and the S rows B2 brings in. Every observation for them in
this change is a record (F5). The previous cut's fallback, S6 plus B5 on the
terminal body, could not reach B16, B20 or B5 in the other suites, and it
promised a tick over clauses it left open (finding 3). The third cut's
fallback still missed THE PROOFS' tests, some of whose controls were never
recorded at all (fourth return, finding 1). The fourth cut's inventory then
missed three of `4daaa7d2`'s tests, and called one recorded control
unrecorded (fifth return). The unrecorded controls are entry 21(a)'s whatever
the ruling says. *(2026-09-23: 21(a) has performed and closed them, four
under the operator's ruling on F1–F4.)* So the answer is the conditional
one above.

**9.6, in detail.** Its precondition at 5499 is "After 8.8 and 8.10", so it
waits on both ticks. That means **two** operator rulings: entry 17 for the
removal class and entry 23's reconciliation of N1–N4/N11. A scope change at
entry 23 is a third only if the operator chooses one. It is never required,
because entries 13, 15 and 16 can satisfy those rows instead. *(Fourth
return, finding 4: this said "three", counting the optional scope change as
mandatory.)* It also means the externally owned entries 13, 15 and 16, and
entry 21(a) (landed 2026-09-23).
Then comes its own accounting and compatibility acceptance, which is **not
started**:
- cold/warm retained-store integration, including the verified first-match
  fold gap (F4, entry 24);
- historical/current multi-message, tool and retry intervals;
- output, tool and target filtering;
- usage deduplication;
- per-message versus cumulative accounting;
- omission of unattributable totals;
- legacy compatibility (5499–5524).

Satisfying 9.6's prerequisites completes none of that. Entries 22 and 23 open
9.6's door and do not walk through it.
