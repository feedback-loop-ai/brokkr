# Acceptance ledger — tasks 8.8 and 8.10

Issue #226, change `2026-09-09-226-session-resumption`. One row per acceptance
clause of whole-task **8.8** and whole-change **8.10**, against evidence this
ledger's author OPENED. It ends with the exact remaining work and three
one-line answers.

Task 8.8's acceptance lives in **three** places, and all three are graded here:

| Where | Lines | Rows |
|---|---|---|
| The numbered repair tasks **8.8.1.1–8.8.8.4** (R1–R4, the composite group) | 1232–1548 | `N1`–`N14` (§1) |
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

This revision answers a returned review of the ledger's first cut. Its nine
findings are all documentation findings against this file; none asked for a
production or test change, and none was made. What changed: §1 exists, eleven
rows moved from `discharged` to `partially discharged`, one citation was
replaced, two clause-classes were found to have no evidence at any level, the
remaining-work list grew from seven units to nine, and the first answer below
changed from yes to no.

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
| `openspec validate --all --strict` | **`d73d94d1`** | **review seat** | **exit 0 — 17 passed, 0 failed** (informational notices). Its grant reached the binary; this seat's did not. |
| `git diff --check 936c04b6..HEAD` | `d73d94d1` | review seat | clean |
| `cargo fmt --all -- --check` | `d73d94d1` | this seat | exit 0, clean |
| `git diff --check 936c04b6..HEAD` | `d73d94d1` | this seat | clean |
| `openspec validate --all --strict` | `d73d94d1` | this seat | **NOT RUN** — refused by this seat's permission grant, a third dated unavailable-tool report beside the D1/D2/D3 seats' (`tasks.md` 650–657, 833–848, 1020–1037) and the Pass C seat's (507–511). |

Every test cited below is in one of the four suites that ran green on
`936c04b6`, so every `discharged` row carries existence, assertion coverage and
recorded execution unless it says otherwise.

**What the review seat's `openspec` run does and does not close.** It is the
first strict validation ever recorded on a Pass C or Pass D *candidate* (the
earlier 15/0 was over the Pass C tasks tree), so F6's older claim that the gate
"has never run" is withdrawn for that one gate at that one revision. It is one
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

Fourteen rows, the R1–R4 composite repair group. Six are `[x]`, **eight are
`[ ]`**. Their own execution-order table is at 1214–1228 and binds each to an
AS1 scenario. This slice commissioned none of them (A55: "Current AU/D7 repair
clauses 8.8.9.1–8.8.15.1 alone are commissioned here") — but 8.8 cannot be
ticked over an open one, so each is graded, and each row says who owns it.

Where a row reads *partially discharged — external*, the **code is in the tree
and opened here**; what stays open is an obligation the task's own words assign
to evidence nobody in this repository can produce: immutable Apple/env source
pins and native macOS platform runs.

| Row | Task | Box | Clause | Line | Evidence opened | Status |
|---|---|---|---|---|---|---|
| N1 | 8.8.1.1 | `[ ]` | keep "candidate spelling and native argv[0] beside canonical file identity"; `classify_in`/`selected_from` changed so doctor "launches the already-selected invocation"; searched `dsh -> /usr/bin/env` and an absolute alias "refuse before either probe". Closing words: "Immutable Apple source pins and native platform evidence remain this owner's pending inherited acceptance, outside the repair" | 1232–1252 | `composite.rs:2217–2267` — `DshInvocation` is a distinct type carrying the invocation "EXECUTION: a launcher reads the path it was run by"; `:1815–1850` `DshPrepared` holds `invocation` beside the admitted home; `:1956` `selected_from`; `:3276` `classify_in`, whose refusals at `:3327` and `:3330` are spelled "the selected invocation {why}" | **partially discharged — external.** The R2 carriage is implemented and opened. The Apple source pins and native platform evidence the row names at 1248–1250 are unopenable here and are the row's own declared inherited debt |
| N2 | 8.8.1.2 | `[ ]` | extend protocol selection tests, doctor unit seams and `crates/brokkr-cli/tests/doctor_dsh_selection.rs` with "both R2 alias forms and direct env"; "Independently remove selected-env qualification, then independently restore canonical execution"; "native macOS remains pending, not newly commissioned" | 1253–1271 | `crates/brokkr-cli/tests/doctor_dsh_selection.rs` exists in the tree | **partially discharged — external, with an evidence-verification gap.** The suite exists; its two required removals are recorded, not re-derived (F5). macOS is pending by the row's own words (1269) |
| N3 | 8.8.2.1 | `[ ]` | change `env_program`'s "successful absence for an empty or ASCII-space/tab-only argument tail to a cause-bearing refusal"; flip the bare `#!/usr/bin/env` row in `the_candidate_classifier_stops_where_the_child_stops_and_refuses_the_unprovable`. Closing words: "Immutable env source pins and missing native platform proof remain inherited debts" | 1273–1287 | `composite.rs:3675–3685` — a blank-tail scan, then `Err("is the platform's env utility given no nonblank program, so the program it would run is the launcher itself")`, with the comment citing run `124cca78`, R4; the same sentence is asserted at `composite/tests.rs:5650` | **partially discharged — external.** The R4 refusal is implemented and its exact cause asserted. The env source pins and native platform proof at 1285–1286 are the row's own inherited debt |
| N4 | 8.8.2.2 | `[ ]` | test bare and blank env shebangs "through protocol selection/producer, injected doctor probes and the built doctor"; "Native bare/blank reproductions use separate markers and an external process-group timeout"; "**Native Windows matrix, doctor/GetBinaryTypeW and Windows MSRV are withdrawn by decision 0063.** Native macOS matrix and applicable source-pin cells retain their original pending acceptance" | 1288–1308 | `composite/tests.rs:5650` (the producer-side cause); `doctor_dsh_selection.rs` (the doctor side) | **partially discharged.** Its Windows half is **withdrawn by decision 0063** — the row already says so in its own words, so no ledger correction is owed there. Its macOS half and source-pin cells are pending external. Its blank-tail removal is recorded, not re-derived (F5) |
| N5 | 8.8.3.1 | `[x]` | R1/R3 pnpm syntax admitted before both probes: structural ASCII separation before scalar admission; raw implicit block-key spelling bounded before trimming | 1310–1375 | `composite.rs:463–610` (the hand-written bounded pnpm line reader, no YAML crate); `composite/tests.rs::the_unrecognized_pnpm_constructs_refuse_through_the_producer`, `::the_pnpm_reader_is_bounded_inclusively_at_the_limit` | discharged (also graded at A27, B96) |
| N6 | 8.8.4.1 | `[x]` | track admitted decoded package headings separately; reject repetitions and conflicting records "with `repeated package key` and the decoded key"; preserve legitimate equal-triple deduplication | 1376–1404 | `composite.rs:1573` emits `a repeated package key '{key}'`; `composite/tests.rs` 5394, 5408, 5414 assert that exact sentence with the decoded key, including the excluded local-tarball key; dedup retention is `::npm_three_group_and_dedup_vectors_retain_distinct_triples` | discharged |
| N7 | 8.8.5.1 | `[x]` | replace the loop/hash oracle in `the_plugin_component_is_bytewise_path_order_and_fails_closed` "with a literal recorded from the existing sole production producer"; "no prose/helper/generator computes another component or canonical serialization" | 1405–1417 | That test and `::the_worked_plugin_vector_pins_the_bytewise_path_order_of_the_component`, both opened at A18/B90; `::no_test_reassembles_the_component_stream` is the guard | discharged for the literal; its "alter production path/line ordering in a compiling mutation" half is recorded, not re-derived (F5) |
| N8 | 8.8.5.2 | `[x]` | "a focused source-conformance assertion… detects restoration of the known competing serialization and concatenation-hash block" | 1418–1432 | `composite/tests.rs::no_test_reassembles_the_component_stream` (A23, A29) | discharged for the assertion; its restoration mutation is recorded, not re-derived (F5) |
| N9 | 8.8.6.1 | `[x]` | apply `Safe` at final rendering of the unavailable-binary and retained-selection cause in `doctor.rs`; a built-doctor test with "a newline and ANSI clear-screen sequence"; "assert the recognizable escaped spelling… with no raw injected sequence" | 1433–1447 | `doctor_dsh_selection.rs:1999–2000` — "S2. A nonexistent override carrying a newline and an ANSI clear-screen sequence reaches stdout ESCAPED"; the unit-side sibling at `doctor/tests.rs:3017` | discharged for the built-doctor assertion; its "remove safe rendering in a compiling control" half is recorded, not re-derived (F5) |
| N10 | 8.8.7.1 | `[x]` | enrich exhausted `resolve_bundle` with `bundle '…' does not resolve: no package.json found`; "Preserve true-absence continuation to a legitimate later hit; unreadable/canonicalization-error/outside first hits still stop" | 1448–1464 | `composite/tests.rs::removing_only_the_plugin_manifest_names_the_drifted_file` (A18); `::a_bundle_candidate_that_cannot_be_inspected_stops_the_search` and `::an_outside_first_bundle_hit_is_not_skipped_for_a_later_inside_one` (B93) | discharged for the cause and the continuation rule; its filename-context mutation is recorded, not re-derived (F5) |
| N11 | 8.8.8.1 | `[ ]` | consolidate R1–R4 tests and removal records from 8.8.1–8.8.3; "Verify all four built-doctor reproductions now end in named refusals"; "**The actual absent-PATH retained-Node positive and its two removals remain pending under this address until their native prerequisite exists; shell failure or NotFound is no positive**" | 1465–1483 | `composite/tests.rs::an_absent_path_is_a_named_refusal_and_never_the_working_directory` and `::the_default_search_path_is_the_c_librarys_own_answer` (A24) are the *refusal* side; no test in the tree is the absent-PATH **retained-Node positive** | **partially discharged — external.** The refusal side is opened. The positive and its two removals are pending a native prerequisite by the row's own words (1477–1480); the consolidation record itself is narrative (F5) |
| N12 | 8.8.8.2 | `[ ]` | "On the restored candidate run `cargo fmt --all -- --check` and `cargo clippy…`. Run `cargo test -p <crate>… sequentially, in order`" for all seven crates; plus both `compile --bundle` runs and `openspec validate --all --strict`; "Unavailable tools or failures leave this row pending" | 1484–1503 | The recorded-execution table above: fmt green on both revisions; four of seven crate suites green on `936c04b6`; `openspec` green on `d73d94d1` but only under the **review seat's** grant and outside the ordered list; clippy, three crate suites and both bundles unrun by any seat on a Pass D candidate | **not started as the ordered list.** Its own last sentence keeps it pending. Duplicates S11's gate list; one execution closes both. Unit 4 |
| N13 | 8.8.8.3 | `[ ]` | "Collect fresh coverage on the committed restored candidate"; the controller "runs unchanged `TMPDIR=/tmp bash scripts/coverage-exact.sh` on a capable host/CI"; "Require nonzero exact equality for **source lines, branches and functions**"; "Keep this row and final-head remote results pending until actual evidence exists" | 1504–1526 | Nothing. D2/D3 reproduced the script's substance by hand (F7); the row's own text forbids reading that as the gate — "The box cannot execute namespace boundary tests; this is preparation, not a literal gate pass" | **not started — externally owned.** This, not 8.8.14.3, is where the coverage obligation lives. Unit 5 |
| N14 | 8.8.8.4 | `[ ]` | reconcile R1–R4 delivery; "Tick a task only when its **entire** acceptance is met; broad owners with inherited pending predicates stay open"; "Preserve change-wide states, unchecked 8.8 and proposed 0056"; commit, never push | 1528–1548 | Not begun: 8.8 is `[ ]` at 4735 and 0056 is `proposed`, which is this row's *preservation* requirement, not its delivery | **not started.** It is also the clause that decides N1–N4 and N11: a row whose inherited predicate is pending "stays open with current repair delivery recorded in prose". Unit 7 |

**What §1 changes.** Eight numbered tasks are open. Five of them (N1–N4, N11)
are open on evidence — Apple/env immutable source pins, native macOS runs, the
absent-PATH retained-Node positive — that **no local commission can produce**,
and whose own rows declare them inherited debt rather than this change's work.
N12 and N13 are gate executions. N14 is the reconciliation that records all of
it. None can be closed by editing this ledger, and 8.8 cannot be ticked while
any of them is open unless the operator rules that N1–N4 and N11 may be ticked
on their delivered-repair half with their inherited predicates recorded — which
1209–1212 forbids a seat from deciding for itself.

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
| A13 | "Verify the runtime unit/integration cases assigned in 8.10 after repairing R2's serialized checkpoint transport" | 4777–4780 | Rows B23–B28 | discharged |
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
| A37 | **(d)** bind the seat's single `--patch` "at every model-site start, cold, offered or `unmeasured`, independent of the resume gate, **at both `start_context` call sites**", withholding it for an absolute path, `..`, symlink escape or a non-member | 4905–4917 | `resume_tests.rs::a_valid_route_overlay_binds_at_the_single_site`, `::…_at_the_panel_member`, `::a_valid_route_overlay_binds_on_an_offered_start_too`, `::a_non_binding_route_overlay_withholds_the_member_at_both_call_sites` | partially discharged — the positive binding is proved at both call sites and on cold, offered and `unmeasured` starts; the **withholding** half rides the panel-member site only, and its symlink vector is a nonmember rather than a compiled member (see B38). Units 2a, 2b |
| A38 | **(d)** "The binding is the engine's alone… from the compiled manifest's `files` entry and never from a hash of the file the value resolves to" | 4917–4924 | `resume_tests.rs::a_changed_route_overlay_member_carries_the_manifest_digest` (asserts the manifest digest AND `assert_ne!` against a hash of the changed bytes) | discharged |
| A39 | **(d)** the adapter reads the file once, "require SHA-256 equality with the bound digest before any shape check", then AS3's closed data-only reader and closed six-field set with `apiKeyEnv` required and the `https` endpoint grammar | 4925–4933 | `route_overlay.rs:81–…` (`claim_with`), `::claim_reads_the_bound_file_and_requires_the_digest_before_the_shape`, `::the_route_row_and_provider_are_closed`, `::a_credential_value_or_a_field_outside_the_set_is_refused`, `::the_endpoint_grammar_decides_the_positive_and_every_refusal`; `adapters/tests.rs::a_dsh_route_overlay_planner_checks_the_digest_before_the_shape_and_before_staging` | discharged |
| A40 | **(d)** require the binding and argv agreement before reading; "Every other `--patch` refuses through the existing pre-work failure path, never forwarded or dropped" | 4933–4937 | `route_overlay.rs:89–115` (absent binding with a `--patch`, binding without a `--patch`, disagreement — each a refusal before any read); `adapters/tests.rs::dsh_route_binding_matrix_refuses_before_staging_on_every_planner_path` | discharged |
| A41 | **(d)** "close every disabled assessment gate before the version probe or composite producer, with or without an offer"; a missing/malformed declared digest also prevents either observation; only a matching `applies_to` reaches the sole producer | 4938–4943 | `adapters.rs:3606–3626`; `adapters/tests.rs::a_closed_dsh_gate_reaches_neither_probe_nor_producer_and_keeps_the_cold_route` (ten dispositions × offer/no-offer, counted producer, recording shim marker), `::a_dsh_identity_mismatch_declines_the_offer_and_keeps_the_cold_route` (absent and uppercase-malformed declared digest reach neither) | discharged |
| A42 | **(d)** compare the recomputed composite with the declared; on an offer compare both observations with `originating_harness_version` and `originating_wrapper_digest` "from the same confirmed root"; missing/mistyped/malformed/unreadable/mismatched declines as `unverified-harness` | 4943–4949 | `adapters.rs:3617–3635`; `::a_dsh_identity_mismatch_declines_the_offer_and_keeps_the_cold_route` 10527–10556 | partially discharged — the originating comparison is driven with *different* values for both fields and *missing* for the digest alone; *missing version*, and *mistyped* and *malformed* for either field, are not driven independently (see B60, F3) |
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
| A53 | **(d)** "The bound current route folds on qualified cold, warm and disabled/mismatched cold alike. No route byte or binding enters the composite, launch row or journal. Retain `confirms_from_locator: false`" | 4993–4997 | `::dsh_positive_planner_paths_fold_the_shipped_route_ahead_of_rust_owned_rows` (all four planner outcomes); `adapters.rs:3500` (`confirms_from_locator: false`) | **partially discharged.** The fold and the flag are proved. The **exclusion** is not: `assert_dsh_planner_overlay` (`adapters/tests.rs:12361–12400`) reads the planned argv and the written overlay file, and `dsh_launch_with` returns a plan — no case in either suite opens an emitted launch row, a journal envelope or a computed composite and asserts it free of route bytes. See F8 and unit 4 |
| A54 | "Do not move the roster assertion, `bundle.json`, `research-web.yml`, compiled staffing or research-dsh witness digest" | 4998–5002 | `git diff origin/main --name-only` names none of them | discharged |
| A55 | "Current AU/D7 repair clauses 8.8.9.1–8.8.15.1 **alone are commissioned here**" | 5002–5005 | Rows S1–S13 | discharged (scope rule) — and read narrowly. It says which clauses *this* repair executes; it does not say 8.8's acceptance is those clauses. §1's fourteen numbered tasks are not commissioned by this slice and are still 8.8's to satisfy (1209–1212) |
| A56 | "Confirm the launched root before publishing": a valid prior depth-zero header at the resolved locator, the plugin's post-`await agents.resume` init event read from the stream-json child, no fresh sibling root/session, and new sequence activity past `firstSeq`, "before the launch hold releases" | 5005–5013 | `adapters/tests.rs::the_dsh_launch_hold_needs_every_confirmation_before_it_publishes`, `::a_dsh_init_event_alone_is_never_the_root_confirmation`, `::a_qualified_dsh_child_confirms_the_root_and_folds_current_only`, `::dsh_a_consistent_pending_rejoin_still_confirms_through_the_terminal_body` | discharged |
| A57 | "Same-root nonce continuity is 10.7's probe-only model-recall device… never planted in a prompt or read at run time" | 5013–5017 | `git grep` finds no nonce in `adapters.rs` or any prompt template; the confirmation reads only header, init event, sibling-root and sequence facts (`DshObservation`) | discharged |
| A58 | "Retain the immutable pre-spawn census of admitted `(header ID, canonical file)` occurrences, exactly one baseline offered header and its address; require current counted occurrences to fit the baseline without collapsing IDs, addresses or multiplicity" | 5017–5020 | `::dsh_census_identity_counts_occurrences_and_addresses_not_distinct_ids`; `::dsh_a_rejoin_without_its_pre_spawn_baseline_is_refused_before_the_child_runs`; `::dsh_a_fresh_entry_reusing_a_sibling_id_at_a_new_address_refuses_the_rejoin` (two admitted addresses behind one id) | discharged |
| A59 | one observation rule on both sides of init; the listed failures "permanently refuse. Only readable consistent waiting is retryable; later init, repaired storage, EOF or delivery never cures refusal" | 5020–5026 | `::dsh_contradictions_after_the_init_event_are_never_restored_away`, `::dsh_an_unreadable_stream_line_refuses_a_pending_rejoin_before_the_stream_ends`, `::dsh_uncertainty_before_the_init_event_is_never_cured_by_a_later_reading`, `::dsh_cold_noise_and_a_root_mismatch_keep_their_terminal_behaviour` | discharged |
| A60 | "R1–R3 end failed with LE3's exact unconfirmed-session reason on both endings, no accepted result, root, locator, launch or work publication and no cold replacement; retain delivery on disk" | 5026–5030 | The shared contract `adapters/tests.rs:5903` (`assert_refused`): terminal reason `"provider never confirmed the offered session; refusing to accept the invocation"` — byte-identical to `specs/adapter-launch-evidence/spec.md:98`; no `root_session`/`transcript`/`launch` row; one child; a delivered file retained, never accepted. Driven on BOTH endings by `refused_on_both_endings` (`:6064`) | discharged |
| A61 | "Verify the six headline child proofs, completed-observation witnesses and all D7 rule controls/removals through the real terminal body" | 5030–5032 | `run_dsh_latch` (`:5999`) drives a real child through production `run_seat_with` + `invoke_dsh_launch_observed`; the six headline proofs are R1/R2/R3 × two endings in `::dsh_malformed_output_before_the_init_event_…`, `::dsh_an_observed_fresh_sibling_…`, `::dsh_a_fresh_entry_reusing_a_sibling_id_…` | **partially discharged — evidence-verification gap.** The six headline proofs and the witnesses carry existence, assertion coverage and execution. The clause's "**and all D7 rule controls/removals**" rests on the twelve-row ledger at `tasks.md` 465–480, which this seat opened and found to be a delivery narrative: it names mutations, endings and a restored SHA-256, and leaves nothing in the tree to re-derive (F5). The first cut graded this row `discharged` while saying the same thing in its own note |
| A62 | "Never forward the launcher's TUI example, treat the retained directory as a provider handle, alter the live global pin/profile, add an SDK runner or admit hands" | 5033–5035 | `::a_retained_dsh_directory_alone_never_supplies_a_provider_handle`; `::a_dsh_offer_is_declined_and_its_retained_directory_is_not_a_handle`; no SDK path in `adapters.rs`; `adapters/dsh.json` unchanged | discharged |
| A63 | "If 10.7 demonstrates that the documented setup or pre-work observation hook is still insufficient… add only the narrow Cordis extension" | 5035–5044 | `tasks.md:5732` — 10.7 is `[x]` and demonstrated no insufficiency; no `extensions/dsh/resume-policy/` was created | discharged — conditional, condition not met |
| A64 | "Verify with the DSH planner/storage shim cases in 8.10 and 9.6; the route-overlay binding's engine cases in the runtime crate… beside `the_private_context_carries_the_owned_target_and_originating_digest` and in its pattern… and in `engine/resume_tests.rs`, the engine integration cases 8.10 assigns to that suite, read off the `Start.input`… at both call sites, a single site and a panel member" | 5044–5058 | `resume.rs::the_private_context_carries_a_supplied_route_overlay_binding` (sits directly beside the named case, asserts value, digest and absence); `resume_tests.rs` route rows A37/A38 | partially discharged — every case 8.10 assigns exists and asserts; 9.6's shim cases do not (row not owned here; see the 9.6 answer) |
| A65 | the four loader cases in `agents/tests.rs` | 5059–5063 | `::the_optional_wrapper_digest_member_loads_carries_and_is_refused_by_name` — all four in one case: measured without the member loads; a well-formed member loads and is carried; malformed or beside `unknown` is refused naming the field; `assert_ne!` on the adapter content digest | discharged |
| A66 | "doctor cases… for a matching, differing, undeclared and unreadable composite" | 5063–5064 | `doctor/tests.rs::the_dsh_composite_detail_reports_each_disposition` — all four, plus the warning rule | discharged |
| A67 | the committed-bytes test: exactly six files; the function's per-file lines carry "the provenance block's path and SHA-256 pairs"; the adapted expression "occurs exactly once"; "the recomputed delta digest equals the note's"; substituting the upstream expression back reproduces `a40b52b3…` | 5064–5071 | `composite/tests.rs::the_committed_plugin_set_is_the_six_files_and_the_one_expression_delta` — asserts the six names, the full digest map against `COMMITTED_PLUGIN_DIGESTS` (`:12039`, which this seat compared line-by-line with `PROVENANCE.md:67–72`), one occurrence, and the upstream SHA-256 | **partially discharged** — four of five. **No test recomputes the delta digest.** `78256d2e…` appears only at `PROVENANCE.md:50`. This seat recomputed it by hand (SHA-256 of `lib/index.js:253\n` + `-` upstream line + `+` adapted line, each newline-terminated) and it reproduces exactly — so the note is true, and what is missing is the assertion, not the fact. See F1 and unit 1 |
| A68 | "Full 8.8 remains pending through C/D; completing B alone never ticks it" | 5071–5074 | Pass C (`b0ec5517`) and Pass D (D1–D3 on this branch) are both delivered; 8.8 stays `[ ]` | discharged (reading rule) |

## 3. Task 8.8's commissioned subgroups, 8.8.9–8.8.15 (133–345)

| Row | Clause | Line | Evidence opened | Status |
|---|---|---|---|---|
| S1 | 8.8.9.1 `[x]` — D7's private `run_seat` seam, helpers returning "the actual wire results and checkpoints from one real child"; warm and qualified-cold controls through the shared terminal body; "Prove the controls' publication/terminal assertions by applicable removal and restored rerun" | 135–143 | `adapters.rs::run_seat_with` + `invoke_dsh_launch_observed`; `adapters/tests.rs:5999` drives one real child; `::a_qualified_dsh_child_confirms_the_root_and_folds_current_only`; `::dsh_cold_noise_and_a_root_mismatch_keep_their_terminal_behaviour` | **partially discharged — evidence-verification gap.** The seam, the real child and the two controls are opened and green. "Prove the controls' publication/terminal assertions **by applicable removal and restored rerun**" rests on rows M10 and M12 of the narrative ledger (F5) |
| S2 | 8.8.9.2 `[x]` — the per-invocation completed-observation seam; "notify on all completed outcomes, not only in the refusal branch"; "Record exact observed facts, not a separate walk or child-created proof marker" | 144–152 | The observer at `:6036` receives every `DshObservation`; `assert_one_child` asserts `acknowledged == acks` and that the child passed each await; `::dsh_a_consistent_pending_rejoin_still_confirms_through_the_terminal_body` | discharged |
| S3 | 8.8.10.1 `[x]` — one absorbing refusal transition, initialized when an offered launch lacks `first_seq`, a successful baseline census or exactly one baseline offered header; "never replace missing history later" | 156–163 | `::dsh_a_rejoin_without_its_pre_spawn_baseline_is_refused_before_the_child_runs` (its five baseline cases); `::dsh_contradictions_after_the_init_event_are_never_restored_away` | discharged |
| S4 | 8.8.10.2 `[x]` — counted pair containment over `(ID, PathBuf)` occurrences; cardinality, then retained address, then counts; "an unrelated old sibling's disappearance alone does not" refuse | 164–174 | `::dsh_census_identity_counts_occurrences_and_addresses_not_distinct_ids`; `::dsh_cold_noise_and_a_root_mismatch_keep_their_terminal_behaviour`; the positive "an unrelated baseline sibling is gone" case | discharged |
| S5 | 8.8.10.3 `[x]` — one store rule on both sides of init; "Before init, `last > first_seq` refuses; after init only readable consistent `last <= first_seq` waits, and `last > first_seq` confirms"; the weaker pre-init predicate retired | 175–184 | `adapters.rs` `dsh_read_offered_store` (one rule, both sides); `::dsh_work_before_the_init_event_is_never_adopted_by_it`; `::dsh_a_consistent_pending_rejoin_still_confirms_through_the_terminal_body` | discharged |
| S6 | 8.8.11.1 `[x]` — refuse malformed pre-init output "even with no observed sequence advance"; refuse pending line-read errors before `break`; "each removal must fail the intended terminal assertion, not time out" | 188–196 | `::dsh_malformed_output_before_the_init_event_refuses_the_rejoin_for_good` — two cases, the second with the store UNMOVED when the line is read, `observations[0] == {Malformed, census: None, last_seq: None, Refused}`; `::dsh_an_unreadable_stream_line_refuses_a_pending_rejoin_before_the_stream_ends` | discharged |
| S7 | 8.8.11.2 `[x]` — apply the common store observation on a malformed post-init line "without blanket JSON refusal"; consistent noise can still confirm; EOF/drain cannot reverse an absorbing outcome | 197–204 | `::a_qualified_stream_json_launch_skips_a_malformed_line_and_still_confirms`; `::dsh_contradictions_after_the_init_event_are_never_restored_away` (the sibling read on a malformed line) | discharged |
| S8a | 8.8.12.1 `[x]` — prove **R1**, baseline `session-1`/27 with malformed pre-init output, on both LE1 endings; "Remove malformed pre-init refusal and observe each terminal/publication assertion fail"; "Keep 8.8.11.1's no-advance case distinct" | 208–216 | `::dsh_malformed_output_before_the_init_event_refuses_the_rejoin_for_good` through `refused_on_both_endings` (`adapters/tests.rs:6064`); its second case keeps the store UNMOVED, which is the distinct no-advance case S6 also cites | partially discharged — evidence-verification gap: the removal (ledger row M1a) is narrative (F5) |
| S8b | 8.8.12.2 `[x]` — prove **R2**: matching init with a fresh `session-9`; "Witness the completed contradictory production census before permitting deletion"; remove only the permanent contradiction transition | 217–226 | `::dsh_an_observed_fresh_sibling_…` through `refused_on_both_endings`; the case witnesses the exact contradictory census it consumed before the sibling is removed | partially discharged — evidence-verification gap: the removal (M3) is narrative (F5) |
| S8c | 8.8.12.3 `[x]` — prove **R3** "separately": the same ID at `--old--` and `--new--`, one offered `session-1`; "Mutate counted containment to the former ID-only comparison"; "no transient R2 case or Pass D storage-admission matrix substitutes" | 227–236 | `::dsh_a_fresh_entry_reusing_a_sibling_id_at_a_new_address_refuses_the_rejoin`, witnessing two admitted addresses behind one id, through `refused_on_both_endings` | partially discharged — evidence-verification gap: the removal (M5) is narrative (F5) |
| S9a | 8.8.13.1 `[x]` — "Post-init offered-header contradictions cannot be restored away": missing and ambiguous offered headers after init, then restored uniqueness; "isolate/disclose overlapping cardinality/identity protection in removals" | 240–248 | `::dsh_contradictions_after_the_init_event_are_never_restored_away` | partially discharged — evidence-verification gap. The disclosure the clause demands is made, at `tasks.md` 481–487 ("Not separately removable, and said so: the CURRENT offered-header cardinality check after init"), which is itself narrative (F5) |
| S9b | 8.8.13.2 `[x]` — "Required post-init evidence cannot become readable later to cure refusal": independently fail census and offered-sequence reads after init, witness each, then repair the store | 249–258 | `::dsh_contradictions_after_the_init_event_are_never_restored_away` and `::dsh_uncertainty_before_the_init_event_is_never_cured_by_a_later_reading`; the baseline half is `::dsh_a_rejoin_without_its_pre_spawn_baseline_is_refused_before_the_child_runs` | partially discharged — evidence-verification gap: removals M3 and M6/M6′ are narrative (F5) |
| S9c | 8.8.13.3 `[x]` — "A fresh sibling observed before init also latches refusal" on valid non-init JSON; "replace sleep-only causal evidence for any transient case newly credited here" | 259–266 | `::dsh_a_fresh_sibling_observed_before_the_init_event_refuses_the_rejoin_for_good`; the transient cases wait on the observer seam (S2), not on sleeps | partially discharged — evidence-verification gap: removal M4 is narrative (F5) |
| S9d | 8.8.13.4 `[x]` — the remaining identity comparisons: an admitted alias repeating the exact canonical `(ID, file)` occurrence, and a replacement changing a retained address with unique IDs and total count unchanged | 267–276 | `::dsh_census_identity_counts_occurrences_and_addresses_not_distinct_ids` | partially discharged — evidence-verification gap: removals M5, M5b and the disclosed joint M5+M7 are narrative (F5) |
| S9e | 8.8.13.5 `[x]` — "A consistent pending rejoin can still confirm" and "Cold stream noise keeps its existing behavior"; "run applicable removals on new assertions to exclude unconditional refusal or whole-census equality" | 277–287 | `::dsh_a_consistent_pending_rejoin_still_confirms_through_the_terminal_body`; `::dsh_cold_noise_and_a_root_mismatch_keep_their_terminal_behaviour` | partially discharged — evidence-verification gap: removals M9, M10 and M12 are narrative (F5) |
| S10 | 8.8.14.1 `[x]` — the transition/return audit against a "task -> scenario -> actual test/ending -> mutation -> failed assertion -> restored pass ledger"; "Inspect the diff to ensure no mutation… survives" | 291–299 | `tasks.md` 465–489 is that ledger, with twelve mutation rows and their endings; `git diff origin/main` shows no mutation in the tree, and production is `e50020ac`'s | **partially discharged — evidence-verification gap.** "Inspect the diff to ensure no mutation… survives" is discharged, on the opened diff. The audit's own substance — that each transition and return was exercised, per the "task -> scenario -> actual test/ending -> mutation -> failed assertion -> restored pass" ledger — is narrative (F5) |
| S11 | 8.8.14.2 **`[ ]`** — "run each D11 gate separately, in order: `cargo fmt`…; `openspec validate --all --strict`; `compile --bundle bundles/self`; `… bundles/verify`. Record command, revision and result; tick only when all these checks execute green. **An unavailable tool is not a pass.**" | 300–313 | The recorded-execution table above. Two ledger seats ran fmt and one ran four of seven suites green; both were **refused `openspec`**, as were the D1/D2/D3 seats (`tasks.md` 650–657, 833–848, 1020–1037) and the Pass C seat (507–511). The **review seat** ran `openspec validate --all --strict` green on `d73d94d1`. Clippy, `brokkr-core`, `brokkr-store`, `brokkr-view`, `brokkr-bridge` and both `compile --bundle` runs have no recorded result on a Pass D candidate | **not started** as a complete ordered execution. One gate of the list now has a dated green result on the head, out of order and outside a delivery record; the clause asks for each gate run "separately, in order" on one candidate, and ticking "only when **all** these checks execute green". Duplicated by N12 (8.8.8.2), which adds both bundle compiles. See unit 4 |
| S13 | 8.8.14.3 **`[x]`** — "Prepare the unchanged external exact-coverage handoff"; verify no pin, gate, exclusion, denominator or test-selection change; "Record revision/environment and actual covered/total lines, branches and functions **when supplied, otherwise explicitly pending/unavailable**"; "This task verifies **preparation and truthful handoff only**; its tick is not a green coverage or remote gate" | 314–326 | `scripts/coverage-exact.sh` and both workflows consume `rust-nightly-version.txt` (the release configuration's own toolchain-agreement extension); the pending external results are recorded as pending at F7 and in the D2/D3 delivery sections | **discharged, and it has no pending half.** The first cut had no row for 8.8.14.3 at all and then, in its remaining work, invented one — assigning the external coverage run to "8.8.14.3's pending half". Its closing sentence forecloses that: the tick is preparation and truthful handoff, already given. The coverage obligation is **N13 (8.8.8.3)**, and recording external evidence as pending is **S12 (8.8.15.1)** |
| S12 | 8.8.15.1 **`[ ]`** — record the delivery, "Tick each finished scoped task beside its evidence", verify `git diff --check`, strict active-change validation, the requirement/checkbox inventory, "then commit the exact staged repair and ledger"; "Do not report delivery before that commit or represent pending external gates as passed" | 327–345 | The three Pass D delivery records exist (`tasks.md` 535, 703, 879) and the clause's own gate — "After the scoped implementation/proofs **and local gates above pass**" — is S11, which has not | **not started** — blocked on S11 by its own first words. Note what it does and does not require of external evidence: it asks that "external pending evidence" be **recorded**, not completed, so it can close over a pending N13. See unit 6 |

## 4. Task 8.10 (5089–5452)

| Row | Clause | Line | Evidence opened | Status |
|---|---|---|---|---|
| B1 | "This whole-change acceptance remains pending." | 5089 | `tasks.md:5089` reads `- [ ] 8.10` | discharged — and unchanged here |
| B2 | "execute only 8.8.9.1–8.8.15.1: R1–R3 plus the bounded watcher-rule controls, both terminal endings, completed production observations, independent compiling removals and positive/cold preservation" | 5089–5093 | Rows S1–S13 | partially discharged — S11 and S12 are open, and every removal-bearing row among S1–S9e carries an evidence-verification gap (F5). Note the scope word: 8.10 says "execute **only** 8.8.9.1–8.8.15.1", so §1's numbered tasks are not 8.10's work — they bear on 8.8's tick, not this one's |
| B3 | "Do not repeat adopted Codex or Pass B work, open Pass D's unrelated matrix or tick this whole task" | 5093–5094 | Pass C's commit touched `adapters.rs` and `adapters/tests.rs` only; 8.10 is `[ ]` | discharged (scope rule) |
| B4 | "six wrapped/unwrapped compiled Codex gate decisions and supported exact-root exchanges, then `bundle.rs`'s two reachable refusal tests and unreachable census-arm consolidation" | 5095–5098 | `driver_conformance.rs::the_compiled_live_inline_codex_shapes_rejoin_their_provider_confirmed_root` (four shapes: single/no-hands-member × wrapped/unwrapped, each asserting the recorded root, the `resumed` row, no refusal and the exact resume argv) and `::the_compiled_hands_inline_codex_shapes_refuse_unavailable_confinement` (two) = six; `bundle/tests.rs::a_raw_phase_that_aliases_a_wrapped_panel_member_is_refused` and `::a_literal_phase_that_aliases_the_injected_validator_is_refused`, with `bundle.rs:2949` `claim_address` as the consolidated arm worded like the final walk's | discharged |
| B5 | "Every new test needs an observed compiling mutation failure at its claimed assertion and a restored pass." | 5098–5099 | Recorded mutation ledgers: Pass C's twelve rows (465–480), D1's two (616–623), D2's seven (803–816), D3's six plus one discarded (992–1008), and the returned review's two (1058–1074). Each names the case that parted | **partially discharged — evidence-verification gap.** This is the clause the narrative rule bites hardest: "**Every** new test needs an **observed** compiling mutation failure at its claimed assertion and a restored pass." Every such observation in this change is a delivery record. What is opened here is that no mutation survives (`git diff origin/main`) and that the cases those mutations aimed at exist, assert what is claimed, and pass (F5) |
| B6 | "Extend the existing runtime/protocol/CLI suites using the test-only seam; do not substitute fabricated roots, repaired markers or map assertions." | 5099–5101 | All new cases live in the four existing suites; the seams are `run_seat_with`, `invoke_dsh_launch_observed`, `dsh_launch_with` and `DSH_STAGING_CALLS`, all private | discharged |
| B7 | "A no-offer refusal is judged by the private production gate with actual composed facts." | 5102–5103 | `::a_closed_dsh_gate_…` drives `dsh_launch_with` with composed `resume_context` inputs and reads `launch.refusal` | discharged |
| B8 | "Namespace/boxed remains refused; preserve shipping harness/none and no-hands live controls." | 5103–5104 | `::the_compiled_hands_inline_codex_shapes_refuse_unavailable_confinement` (`restrictions-unavailable`, then a fresh cold launch); `boundary_tests.rs::the_seat_input_names_the_boundary_and_the_marker_only_under_a_box` | discharged |
| B9 | "Directly executed shebang tests are Unix-only or use a real target-platform executable." | 5104–5105 | Every shebang-driving case reads `#[cfg(unix)]` (`adapters/tests.rs` 4068, 6110, 10236, 12563, 12736…); `composite/tests.rs::each_platforms_absent_path_search_is_its_own_loaders_rule` is `#[cfg(unix)]` | discharged |
| B10 | "Clauses 5–6 own fresh gates, coverage and the unsigned evidence commit." | 5105–5106 | Rows S11, S12 | partially discharged — the gates and the commit are open |
| B11 | "Keep this checkbox pending for inherited C/D work" | 5106–5108 | 8.10 is `[ ]` | discharged |
| B12 | "The following provider-planner and operator-ruling breakdown is inherited acceptance/history, not additional work commissioned by this slice." | 5109–5111 | Reading rule; rows B13–B22 are graded as acceptance all the same | discharged |
| B13 | "Complete each provider-local planner guard and its tests in `adapters/tests.rs` from the captured grammar, and the engine's private-target and route-binding cases in the runtime suites" | 5111–5114 | Rows A34–A53, B36–B55 | discharged |
| B14 | "Keep exact arity, duplicate and precedence checks for every authoritative restriction on cold and resume paths without a generic provider grammar" | 5114–5116 | `::claude_refuses_a_duplicate_or_valueless_authoritative_restriction` (eleven spellings × cold and offered); `::only_the_flags_a_resume_can_safely_carry_travel_with_it`; `::a_second_bare_or_odd_patch_is_refused_by_arity` | discharged |
| B15 | "Preserve the completed Claude cases that independently refuse a second or last-wins permission mode, tools list, strictness/MCP document, allowed/disallowed-tools list (including aliases), model or effort control" | 5116–5119 | `::claude_refuses_a_duplicate_or_valueless_authoritative_restriction` 4074–4123 — permission mode twice and joined+separate, tools twice, `--strict-mcp-config` twice, `--mcp-config` twice, `--allowedTools`/`--allowed-tools`, model twice, effort twice, and three valueless spellings; each on `None` and an offered session | discharged |
| B16 | the operator-ruling breakdown: "first the composition bridge in `engine/boundary_tests.rs` and the existing agent suites, then the shipped assessment/production-composed argv exchange in CLI `tests/driver_conformance.rs`, with separately observed disabled-status, boxed-hands, harness-fragment and boundary-mark mutation failures" | 5119–5125 | `boundary_tests.rs::the_shipped_codex_harness_work_seat_composes_the_preserved_rejoin`; `driver_conformance.rs::the_shipped_codex_harness_work_seat_rejoins_its_retry`, `::the_shipped_inline_codex_work_seat_rejoins_its_retry` | **partially discharged — evidence-verification gap.** The bridge and the exchange are opened and green. The clause's "**separately observed** disabled-status, boxed-hands, harness-fragment and boundary-mark mutation failures" are four narrative records (F5) |
| B17 | "The full resolved boxed MCP argv stays cold." | 5125 | `::the_compiled_hands_inline_codex_shapes_refuse_unavailable_confinement` — the declined offer falls back to a fresh cold launch | discharged |
| B18 | run `the_seat_input_names_the_boundary_and_the_marker_only_under_a_box` and `no_gate_topology_is_ever_offered_a_session` beside that bridge | 5125–5129 | Both exist (`boundary_tests.rs:1562`, `resume_tests.rs:720`) and both ran green in this seat's `brokkr-runtime` suite (456 passed) | discharged, with recorded execution |
| B19 | "Then the actual DSH cold planner boundary, Codex refusal cause, Codex cold selector and six-file digest controls, in that order." | 5130–5131 | `::a_closed_dsh_gate_…`; `::a_codex_seat_argv_that_selects_a_session_is_refused_on_the_cold_path_too`; `::a_refused_resume_is_a_cold_spawn_with_the_refusal_journaled`; `::the_committed_plugin_set_…` | discharged |
| B20 | "pair its selector mutation with the existing protocol adapter test `a_refused_resume_is_a_cold_spawn_with_the_refusal_journaled`, verifying two children, no selector and the retained sandbox in the replacement argv, and one cold launch row" | 5131–5135 | That test exists, asserts two children, no selector, the retained sandbox and one cold launch row, and ran green | **partially discharged — evidence-verification gap.** The test is opened. "**pair its selector mutation with**" that test is a narrative record (F5) |
| B21 | "Each clause names its tests, requirement and observed failure/pass; retain all inherited fixes and leave 8.10 unchecked." | 5136–5138 | The delivery sections name tests and outcomes throughout; 8.10 is `[ ]` | discharged |
| B22 | "Only AU's Pass C repair is scheduled by the current clauses; Pass D remains pending and unscheduled. Verify each current case alongside 8.8(d); full 8.10 remains unchecked." | 5140–5144 | Pass D was later commissioned in three runs (`tasks.md` 535, 703, 879), which is the schedule this sentence anticipated | discharged |
| B23 | R2's transport repair: "use `serde_json` to serialize one complete checkpoint-data JSONL row per declared invocation, including that invocation's ID"; the shim emits its indexed row "as a `%s` argument in a fixed format"; "A missing row fails explicitly, never repeats the last checkpoint. Declare enough rows for panel re-entry" | 5145–5152 | `resume_tests.rs::a_missing_dsh_checkpoint_row_fails_the_shim_without_repeating` (second invocation emits no checkpoint and says `no checkpoint row` on stderr); `::an_offered_dsh_start_carries_the_recorded_home_at_the_panel_member` declares two rows and the panel is re-entered once | discharged |
| B24 | exercise the emitted bytes "with a representative Windows home and quotes, percent signs, backslashes and an embedded newline; decode JSON and assert exact fields and one JSONL frame per checkpoint. Treat that home as data, without creating a Windows-shaped directory on POSIX" | 5152–5157 | `::a_windows_shaped_dsh_home_survives_the_checkpoint_transport` — the home is `C:\Users\seat\AppData\%TEMP%\"quoted"\nnext` (backslashes, percent, quotes, embedded newline), exactly one `checkpoint` frame, and `data["transcript"]["home"]` decodes back unchanged. No directory is created | discharged — this is decision **0063 ruling 5**'s retained host-agnostic validation of *data*, run on Linux |
| B25 | "Preserve and run both existing tests `an_offered_dsh_start_carries_the_recorded_home_at_the_single_site` and `…_at_the_panel_member` on their actual temporary homes" | 5157–5160 | Both read, both assert the three owned-target coordinates plus both originating members off the real `Start.input`; both ran green here | discharged, with recorded execution |
| B26 | "native macOS evidence stays pending controller CI; native Windows proof is withdrawn by decision 0063" | 5160–5163 | `docs/decisions/0063-windows-is-not-a-host.md` rulings 2–3 | **not this slice's** — the Windows half is **withdrawn by decision 0063** (rulings 2 and 3), neither debt nor executed proof. The macOS half is pending "controller CI" by the clause's own words, so it is externally owned and is not a commissionable unit; it is tracked at F7 |
| B27 | prove the originating-home carrier with "distinct old/new ID, locator, home, version and digest values plus another site's row, then **absent/mistyped** latest fields"; "A **missing/mistyped home or locator** never borrows from an older checkpoint or another site; an incompatible newest owner supplies no offer" | 5164–5174 | `resume_tests.rs::a_stamped_row_is_offered_only_to_its_own_site_owner_and_persistent_root` — `SITE_B` (another site), `OTHER_OWNER` (incompatible newest owner), the `coord` helper's distinct old/new five coordinates at 1295–1337, the newest-row-without-transcript case at 1285–1290 (`persistence_home: None`, `persistence_locator: None`), and the mistyped case at 1340–1372 | **partially discharged.** *Absent* home and locator are proved, and both read `None` rather than the older row's. *Mistyped* home and locator are **not**: in the mistyped journal only `harness_version` and `wrapper_digest` become `json!(7)` and `json!(8)` (`:1354–1355`); `locator` stays `"sessions/brokkr/newer"` and `home` stays `"/new/home"`, both well-typed strings. The clause names home and locator explicitly, so the mistyped half of its own sentence is undriven. See F9 and unit 3 |
| B28 | read actual `Start.input` at both production callers: "an offered start carries the exact five coordinates from its selected checkpoint… and a no-offer start carries no owned target. Directly constructing both context objects in a unit test does not establish their journal association." | 5175–5182 | The two `an_offered_dsh_start_carries_the_recorded_home_at_…` cases drive a real engine run and read what the logging driver received; the first attempt carries no `owned_target` | discharged |
| B29 | "Retain the two scans unless a failing case requires a B correction." | 5182–5183 | `eligible_offer` and `originating_root` both still exist and are exercised separately at `resume_tests.rs` 1238–1255 | discharged |
| B30 | assert the private carrier "is absent from rendered prompt/context **and is not copied as `resume_context` or `owned_target` into launch evidence**; retain the existing confirmed `root_session` and `transcript` fields" | 5183–5186 | `resume_tests.rs::an_offered_dsh_start_carries_the_recorded_home_at_the_single_site` 2026–2033 — `starts[1]["context"].get("owned_target").is_none()` and the same for `assessment` | **partially discharged.** The clause has three predicates. *Absent from rendered context* is proved, on the real `Start.input`. *Not copied into launch evidence* is **not**: both assertions read `starts[…]["context"]`, a start message, and nothing in the suite opens an emitted launch row. *Retain `root_session`/`transcript`* is not asserted in these two cases either. See F8 and unit 4 |
| B31 | "Preserve gate/no-offer behavior and Pass A's binding cases independently." | 5186–5188 | `::no_gate_topology_is_ever_offered_a_session`; `::every_work_topology_is_offered_its_own_session_and_no_other` | discharged |
| B32 | D10's counter at `dsh_seat_overlay_in` entry, "Reset it for every synchronous `dsh_launch_with` call and require one on positive plans to calibrate it; every pre-observation control/route refusal requires zero… Use no process-global counter, directory scan or new planner signature. Origin/storage declines can legitimately observe identities and stage one safe cold plan" | 5189–5197 | `adapters.rs:5301–5302` (`#[cfg(test)]` thread-local at entry); `reset_dsh_staging_calls()`/`dsh_staging_calls()` used in the residual, grammar, binding and path matrices — zero on refusals, one on positives, one on the origin/mismatch declines | discharged |
| B33 | exercise `dsh_launch_with` for exact authorized inputs "and every competing residual category" (the full named list); "Retain both existing effort spellings as positive cases, with no guessed aliases. Exercise cold, offered and disabled paths." | 5198–5206 | `::dsh_residual_and_joined_controls_refuse_before_any_observation` — the ledger covers duplicate/missing/invalid model and effort, equals-joined model, mixed effort spellings, effort without a model, bare/duplicate/joined/odd patch, session/new/resume/list/profile/workdir/output/settings controls, `--from-default-profile`, `--verbose`, unknown names, `--`, positional text and short/joined/clustered forms, driven over `("disabled", …), ("offered", …), ("cold", …)`; `::both_dsh_effort_spellings_are_admitted_and_stage_one_overlay` | discharged |
| B34 | "Pair bad controls with a route that would fail to read and require the control refusal first; record zero version calls, a never-called composite closure and no staged overlay or retained-root allocation on these failures." | 5206–5210 | The same ledger pins `--patch does-not-exist.yml` beside the rejected control and asserts `dsh_staging_calls() == 0`, a marker-free version shim and a panicking/counted producer | discharged |
| B35 | "Include synthetic private markers in rejected option names, equals-joined values, malformed model/path/selector-control values and odd patch spellings; assert no diagnostic echoes them." | 5209–5212 | `MARK = "zzz-9f31c7-marker"` (a valid model id, so no control refusal may name it) carried in every vector; the loop asserts no diagnostic contains it | discharged |
| B36 | the engine-side route cases belong to `resume_tests.rs` "in the pattern of the private-context unit case", built through that suite's `bundle` helper, with a dsh site carrying one `--patch`, the logging driver, the planted file, read off `Start.input` "at both production `start_context` call sites… and for the positive case on a cold start, an offered start and a start under an `unmeasured` assessment alike" | 5213–5229 | `resume_tests.rs` 1609–1930: `overlay_digest`, `patched`, `route_start`, `route_binding`; the four cases at 1644, 1688, 1734, 1814 and the offered case at 1866; the single-site and panel-member cases are inline seats whose assessment is `unmeasured` | discharged |
| B37 | "(i) A valid leaf-manifest member carries the actual argv value and that member's compiled manifest digest." | 5229–5231 | `::a_valid_route_overlay_binds_at_the_single_site` and `::…_at_the_panel_member` — `binding["value"] == "recipe/route.yml"`, `binding["digest"] == overlay_digest(bytes)` | discharged |
| B38 | "(ii) A same-shaped nonmember outside the layer, a working-directory shadow…, an ancestor-layer file, a `..` component and an in-layer symlink whose target resolves outside the layer directory while remaining inside the working directory (**present at compilation and therefore a `files` member**), and the absolute path produced by `./` expansion each receive no binding… **at either call site**." | 5231–5237 | `::a_non_binding_route_overlay_withholds_the_member_at_both_call_sites`, `resume_tests.rs:1745–1797` — six shapes (`nonmember`, `shadow`, `ancestor`, `traversal`, `absolute`, `symlink`), each asserted to carry no `route_overlay`. The `./` half is `bundle.rs:3689`, where a `./`-spelled command part expands to `dir.join(rel)` — an absolute path, which is the `absolute` shape | **partially discharged, on two axes.** (1) **Call site.** The seat is `seat(panel(members), …)` at `:1777`, so all six shapes ride the **panel member** only; despite the test's name no shape is driven through the single site. (2) **The symlink shape is the wrong one.** At `:1797` the manifest is `bundle.manifest["files"] = json!({ "route.yml": … })` — `recipe/link.yml` is **not** a `files` member, so the case proves a *nonmember that happens to be a symlink*, not the clause's escaping symlink "present at compilation and therefore a `files` member". The parenthesis is the whole point of the vector: a member the compiler admitted, escaping at resolution. That vector does not exist. See F2b and units 2a/2b |
| B39 | "(iii) A member whose bytes changed after compilation cannot authorize its new bytes: the carried digest is the manifest's recorded value, not a hash of the resolved file, **at either call site**." | 5237–5240 | `::a_changed_route_overlay_member_carries_the_manifest_digest` — asserts the manifest digest and `assert_ne!` against a hash of the changed bytes | **partially discharged** — its seat is `single(…)`, so the case is proved at the **single site** only; the panel member's changed-bytes variant is not driven. The mirror of B38's gap. See unit 2a |
| B40 | the adapter suite owns the rest: `dsh_launch_with` "reusing the existing `route_overlay.rs` reader vectors and the shipped `recipes/research-dsh` overlay as the positive vector on qualified cold, qualified warm and disabled cold, plus identity-mismatch cold… asserting each planned command, exactly one `--patch`, one staging call, unchanged reasoning-level rows and the route rows before every Rust-owned… row" | 5240–5248 | `adapters/tests.rs:12348` reads the shipped `recipes/research-dsh/drivers/research-web.yml`; `::dsh_positive_planner_paths_fold_the_shipped_route_ahead_of_rust_owned_rows` asserts one `--patch`, `dsh_staging_calls() == 1`, one `reasoningEfforts:` and the row ordering on all four outcomes | discharged |
| B41 | "Include declared-composite mismatch without an offer (no refusal token) and originating-identity mismatch with an offer (`unverified-harness`), with the version/producer counts appropriate to each comparison." | 5248–5252 | Same test 12504–12551 — the mismatch case asserts `refusal.is_none()` and one staging call; the origin case asserts `Some("unverified-harness")`, cold, one staging call | discharged |
| B42 | "The resulting overlay is the observation; the existing helper-only ordering test and synthetic `contains` assertions do not discharge it. **Keep the composite, launch row and journal free of route bytes.**" | 5251–5254 | `assert_dsh_planner_overlay` (`adapters/tests.rs:12381–12400`) reads `launch.overlay.path()` off disk and compares row indices; `::the_shipped_route_overlay_folds_ahead_of_the_rust_owned_rows` is retained beside it, not in place of it | **partially discharged.** The first sentence is discharged — the observation really is the written overlay. The second is not: no opened case asserts the composite, an emitted launch row or a journal envelope free of route bytes. See F8 and unit 4 |
| B43 | "Run each negative on cold, offered and disabled planning, requiring a pre-staging refusal naming a depth, field or URL part and never a value." | 5254–5258 | `::dsh_route_grammar_matrix_refuses_before_staging_on_every_planner_path`, `::dsh_route_binding_matrix_…`, `::dsh_route_overlay_path_refusals_precede_any_probe_or_staging` — each loops `("disabled", …), ("offered", …), ("enabled", …)` and asserts zero staging, zero producer calls, no version probe and no echoed value | discharged as the rule; its per-class coverage is graded at B44–B50 |
| B44 | "a second or bare `--patch`" | 5258–5259 | The residual ledger's `duplicate patch`, `bare patch`, `joined patch`, `odd patch spelling`, `flag-shaped patch value` and three "patch claiming a later control" vectors, all three planner paths, `Field("--patch")`; plus the splitter-level `::a_second_bare_or_odd_patch_is_refused_by_arity` | discharged |
| B45 | "an absolute, `..`, symlink-escaping, non-regular, oversized or non-UTF-8 path" | 5259–5260 | `::dsh_route_overlay_path_refusals_precede_any_probe_or_staging` drives symlink-escape, non-regular, oversized and non-UTF-8 on all three paths. Absolute and `..` values receive no engine binding (B38), so at the planner they arrive as "a `--patch` with no bound route overlay", which `::dsh_route_binding_matrix_…` drives on all three paths; their direct spelling refusal is `route_overlay.rs::claim_refuses_absolute_traversal_and_escaping_values` | discharged — jointly, as the clause's own "proven end to end by the two suites together" allows |
| B46 | "an absent binding beside a present `--patch`, a binding without `--patch`, a disagreeing binding, and a bound digest the bytes read do not hash to… each of those five refusals is proven end to end by the two suites together and never by adapter cases in place of engine ones" | 5259–5266 | `::dsh_route_binding_matrix_refuses_before_staging_on_every_planner_path` — its `Case` list carries `digest before shape`, the absent binding, the binding without `--patch` and the disagreeing binding, each on three paths; the engine half is B38/B39 | discharged |
| B47 | "a Rust-owned or foreign row ID, **a second entry or provider**, a provider the seat did not pin, an absent model pin or one without a provider segment" | 5266–5269 | The planner matrix's vector labels are exactly `foreign row` (`adapters/tests.rs:12593`), `provider not pinned` (`:12598`), `model not pinned` (`:12603`) and `second provider` (`:12608`). **Absent model pin** and **a model without a provider segment** are proved only at the reader (`route_overlay.rs::a_route_needs_a_model_pin_and_model_item`, `::a_bound_route_needs_a_pin_a_nonempty_value_and_a_readable_workdir`), and `route_overlay.rs:111–115` is the production refusal. A **second entry** vector exists at neither the reader nor the planner | **partially discharged.** Six classes are named. Four run the three planner paths. Two (absent model pin, segment-less model) are reader-only, which line 5285 says is not evidence for the launch paths. One — **a second entry** — has no vector anywhere; the first cut read the clause's "a second entry or provider" as one class and credited `second provider` for both. See F2 and unit 2c |
| B48 | "each field outside the closed six-field set, a literal authentication header beside a valid `apiKeyEnv`, and a missing or non-name-shaped `apiKeyEnv`" | 5269–5272 | The grammar matrix's `field outside the set`, `literal auth header`, `missing apiKeyEnv`, `malformed apiKeyEnv` — all three paths | discharged |
| B49 | "each `baseURL` grammar breach (userinfo, query, fragment, percent-escape, backslash, whitespace, brackets, **non-ASCII**, empty segment, invalid host label or port, `http`, uppercase scheme, schemeless)" | 5272–5274 | `route_overlay.rs::the_endpoint_grammar_decides_the_positive_and_every_refusal`, `:692–718` — its thirteen vectors read, in order: `user:pass@`, `?api_key=1`, `#frag`, `%2f`, `host\x`, `/ space`, `[::1]`, `//x`, `http://`, `HTTPS://`, `host/x`, `-host`, `:123456`. The planner matrix drives **three** (`http endpoint`, `query endpoint`, `backslash endpoint`) | **partially discharged, worse than first recorded.** Ten of the thirteen reader vectors never run the three planner paths, and line 5285 forbids reading helper success as evidence for them. Beyond that, the clause names **fourteen** breaches and the reader enumerates thirteen: **non-ASCII is absent from the reader's own list**, so that breach has no evidence at either level. The first cut said "all thirteen" and matched the count instead of the names. See F2 and unit 2c |
| B50 | "tabs, control characters, document markers and executable or unrecognized syntax at any depth in either representation (a `!!js` or other tagged scalar, a `__jsExpr` mapping, a flow collection, anchor, alias, merge key or block/quoted scalar)" | 5274–5277 | The planner matrix drives `tagged scalar`, `flow collection`, `anchor` and `merge key`. `route_overlay.rs::executable_or_unrecognized_syntax_is_refused_at_any_depth` and `::the_reader_refuses_every_lexical_shape_it_did_not_recognize` cover `__jsExpr`, alias, block scalar, quoted scalar, tabs, control characters and document markers — at the reader only | **partially discharged** — seven of eleven lexical shapes are reader-only, the same gap as B49. See unit 2c |
| B51 | "Prove the binding and digest check run before any shape check: bytes invalid on both digest and shape axes must yield the digest refusal first, while a bound, digest-matching member with an invalid `baseURL` yields its grammar refusal." | 5277–5281 | `::dsh_route_binding_matrix_…`'s first vector (`digest before shape`: an `apiKeyEnv` of `9LIVE` bound to the VALID bytes' digest) and its `shaped` vector (a digest-matching member with `baseURL: http://host/x`); `::a_dsh_route_overlay_planner_checks_the_digest_before_the_shape_and_before_staging` | discharged |
| B52 | "assert zero entries to the calibrated stager and zero version/producer calls, with a fixed depth/field/URL-part reason and no synthetic private marker echoed. An error or absent retained directory alone does not establish that nothing was staged." | 5281–5285 | All three matrices assert `dsh_staging_calls() == 0`, `calls.get() == 0` and `!marker.exists()` per vector per path | discharged |
| B53 | "Helper-reader success is not evidence for these three launch paths." | 5285–5286 | This sentence is what makes B47, B49 and B50 partial rather than discharged | discharged as the grading rule this ledger applies |
| B54 | "Keep one resulting patch, unchanged reasoning levels and current route rows before Rust-owned rows on each positive path, including identity-mismatch cold." | 5286–5289 | `::dsh_positive_planner_paths_fold_the_shipped_route_ahead_of_rust_owned_rows` — qualified cold, qualified warm, disabled cold, declared mismatch and origin mismatch, each with one `--patch`, one `reasoningEfforts:` and the asserted row order | discharged |
| B55 | "Retain the existing engine privacy assertions; B's plan is not an emitted launch." | 5288–5290 | B30 (retained, and re-graded there); `dsh_launch_with` returns a `DshLaunch` value, so the planner suite has no launch row to inspect | **partially discharged.** "Retain the existing engine privacy assertions" rides B30, now partial. The first cut also claimed "the tests assert no launch row" — that is unsupported and withdrawn: the tests do not assert the *absence* of a launch row, they simply never emit one, which is a property of the seam, not an assertion |
| B56 | the gate-before-probe matrix: "missing/unmeasured/unsupported assessments and missing accounting evidence (`unsupported-resume`), incompatible boundary/hands (`restrictions-unavailable`), missing/mistyped applicable identity and absent/mistyped/malformed declared digest (`unverified-harness`)" | 5291–5295 | `::a_closed_dsh_gate_reaches_neither_probe_nor_producer_and_keeps_the_cold_route` — `absent`, `unmeasured`, `unsupported`, `missing-accounting`, `restrictions`, `hands-mismatch`, `no-identity`, `mistyped-identity`, `no-declared-digest`, `mistyped-declared-digest`, each × offer and no-offer, each asserting its exact token; the *malformed* declared digest (`"A"×64`) is driven in `::a_dsh_identity_mismatch_…` 10492–10508 | discharged |
| B57 | "Use a recording version shim and a panicking or counted composite closure; assert zero calls to both. A nonexistent executable alone does not prove no version attempt." | 5295–5298 | `dsh_recording_version_shim` (`:10218`, touches a marker file on every invocation) and `Cell`-counted or `panic!`ing closures, in every gate case | discharged |
| B58 | "Assert the complete shipped cold argv and overlay, not merely `stream_json == false`: no `--new`, `--session` or `--output-format`, no rejoining target or offerable-root claim, and a refusal token only when an offer was declined." | 5298–5301 | Same test 10351–10383 — `command[0]`, `command[1..4] == ["--profile","headless","--patch"]`, exactly one `--patch`, none of the three selectors, and the overlay's transcript/compression/model rows read off disk | discharged |
| B59 | "independently exercise matching observations, absent/malformed/unreadable version output, version-command failure and version drift, producer error and canonical-composite mismatch. A matching version invokes the sole producer once; earlier failures invoke it zero times." | 5302–5306 | `::a_dsh_identity_mismatch_declines_the_offer_and_keeps_the_cold_route` 10406–10471 — producer error, composite mismatch, `exit 3` version failure, unreadable banner, drift with `calls == 0`; and `::a_qualified_dsh_launch_…` for the matching case | discharged |
| B60 | "**independently** vary originating **version and digest** through missing, mistyped, malformed and different values; these declines may follow one producer call" | 5306–5308 | Same test, `adapters/tests.rs:10527–10556` — the loop's three labels are exactly `("originating version", "version")`, `("originating digest", "digest")` and `("missing originating digest", "null")`, mutating to `"0.1.4-rc.1"`, `"d"×64` and `Value::Null` | **partially discharged.** Two fields × four values = eight vectors required; three exist. Driven: different version, different digest, **missing digest**. Undriven: **missing version**, mistyped version, malformed version, mistyped digest, malformed digest. The first cut listed the gap as two (mistyped, malformed) by treating `Value::Null` as covering both fields' missing case; it covers the digest's only. `adapters.rs:1097–1110` reads both through `Value::as_str`, so the undriven five collapse onto the driven two in behaviour — the independent variation the clause asks for is what is missing. See F3 and unit 3 |
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
| B71 | "Exercise each finite enumeration/header/sequence budget **at its admitted limit and beyond**, read/iteration failure, malformed or truncated boundary data and a valid prefix followed by invalid data: decline rather than skip an unsafe candidate, use a partial maximum or default to zero." | 5337–5342 | `::dsh_admission_reads_are_complete_within_their_bounds_or_decline` (`adapters/tests.rs:11673–11790`) and `::dsh_stored_sequences_decline_instead_of_reporting_a_partial_maximum` (`:11793–11858`), both read line by line; plus `::owned_dsh_root_refuses_a_store_whose_sequence_cannot_be_read` and `::a_retained_project_entry_the_reader_cannot_yield_is_a_bounded_refusal` and its session-level sibling | **partially discharged.** Budget by budget: **header** — both sides, a 16-byte budget declines and `DSH_HEADER_LIMIT` admits, with an exact-at-limit positive whose newline lands on the last byte (`:11692–11701`); **enumeration** — both sides, `dsh_session_file_with(&root, &id, 1).is_err()` beside `(…, 2).is_ok()` (`:11773–11774`); **session-file cap** — refusal only, `set_len(DSH_SESSION_FILE_LIMIT + 1)` (`:11780`), with no file at exactly the cap admitted; **per-row sequence** — refusal only, `dsh_session_last_seq_with(…, 4)` returning `None` (`:11829`), with no row admitted at exactly its injected budget. The failure, malformed, truncated and valid-prefix halves are fully driven. Two of four budgets lack the clause's positive side. See F10 and unit 3 |
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
| B88 | "Assert exact normalized value bytes, equality with equivalent pnpm entries, complete-triple deduplication and retention of same-name entries with different versions or integrities." | 5407–5409 | `WORKED_DEPENDENCIES` (fourteen values over ten names, asserted as literals, each value's shape read off itself); `::equivalent_npm_and_pnpm_entries_compose_to_one_dependency`; `::npm_three_group_and_dedup_vectors_retain_distinct_triples` | discharged |
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
| B99 | "keep 8.8's committed-bytes test pinning the repository-owned adaptation's exact six-file set against its provenance block" | 5437–5439 | Row A67 | partially discharged — see A67 and F1 |
| B100 | "Cover the conditional extension with synthetic absent and present sets, exact four-file path order, changed bytes, missing or extra files, symlinks and resolution outside the profile; absence emits no extension line." | 5439–5442 | `::the_conditional_extension_is_absent_or_composed_from_its_own_four_files` (`WORKED_PAIR_STREAM` with no `extension` line, `WORKED_TRIO_STREAM` with one); `::the_extension_walk_refuses_a_missing_extra_or_symlinked_member`; `::the_extension_s_local_record_leaves_only_when_the_extension_resolves`; `::the_plugin_and_extension_must_resolve_inside_the_profile` | discharged |
| B101 | "**If the extension is required**, also compare its committed set to its own provenance block through the same function." | 5442–5444 | No extension is required (row A63: 10.7 demonstrated no missing hook), none is commissioned, and B102 forbids creating one | **not this slice's** — conditional with no subject. It cannot be discharged and is not debt; it stays conditional on 8.8's 5035–5044 clause ever firing |
| B102 | "No speculative extension is created merely to exercise these cases." | 5444 | No `extensions/dsh/resume-policy/` in the tree | discharged |
| B103 | "Retain the exact resume argv and complete current class/model/effort cases for every other adapter, generated-fragment versus passthrough distinction, no ambient cold/gate continuation, nonpersistent refusal, identifier injection, unsupported hands and cold/resume inability to honour the class." | 5445–5449 | Claude: `::a_claude_resume_is_the_cold_argv_plus_exactly_one_owned_selector`, `::a_claude_argument_that_selects_a_conversation_refuses_before_any_provider_work`. Codex: `::a_codex_resume_carries_the_thread_the_class_and_the_prompt`, `::a_codex_resume_re_expresses_the_effort_pin_as_a_config_override`, `::a_codex_seat_argv_that_selects_a_session_is_refused_on_the_cold_path_too`, `::a_class_that_cannot_travel_spawns_cold_with_the_reason_journaled`. LaneTally: `::the_lanetally_wrapper_resumes_on_its_own_shape_and_its_own_binary` — the gap D3 closed: exact cold and warm argv with the wrapper at `argv[0]`, one owned selector, the class/model/effort fragment unaltered, a claude measurement that does NOT enable the wrapper's shape, unsupported hands, a forged identifier absent from the cold argv, and an ambient `--continue` refused by the COMPLETE expected reason cold and warm | discharged |
| B104 | "Label these deterministic planner/storage shims rather than live DSH compatibility or enforcement evidence." | 5449–5452 | Every Pass D delivery section says so in its own words (`tasks.md` 554–561, 725–731, 905–909); the sources carry the same labels | discharged |

## 5. Verification gaps and findings

These are the places where this ledger could not turn a clause into opened
evidence, or opened evidence and found less than the clause asks. F8–F12 are
new in this revision; F2, F2b, F3, F6 and F7 are corrected.

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
  absent (A37, B38, B39).** The positive binding is proved at both
  `start_context` call sites. The six non-binding shapes are all panel members
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
  the artefact.** Every mutation ledger (Pass C's twelve at 465–480, D1's two,
  D2's seven, D3's six, the review's two) is a narrative of a compile-and-revert
  that leaves nothing in the tree. This ledger confirms no mutation survives
  (`git diff origin/main`), and that the cases those mutations aimed at exist,
  assert what is claimed and pass. It does **not** re-derive the failures.
  What changed in this revision is the *grading*, not the facts: A61, S1, S8a–c,
  S9a–e, S10, B5, B16, B20 and B77 name a mutation in their own words, so they
  read **partially discharged — evidence-verification gap** rather than
  `discharged`. Reproducing the mutations is still not required by any clause
  and is still not proposed as remaining work; naming which predicates rest on
  narrative is what a reconciliation owes. The operator is the one who decides
  whether a recorded removal, run by a named seat on a named revision, is
  acceptance — this ledger only stops asserting that it is verified evidence.
- **F6 — `openspec validate --all --strict` has now run once on a Pass D
  candidate, under another seat's grant.** The review seat ran it on `d73d94d1`:
  exit 0, 17 passed, 0 failed. The first cut's claim that the gate "has never
  run on a Pass C or Pass D candidate" is withdrawn. Four ledger/implement seats
  (this one, D1, D2, D3) and the Pass C implement seat (`tasks.md` 507–511) were
  each refused the binary by permission grant, and those dated reports stand.
  What remains true: no seat has run 8.8.14.2's or 8.8.8.2's list in order, on
  one candidate, into a delivery record, so the gate rows stay open. The
  externally owned prerequisite is unchanged — a seat or host whose grant
  reaches `openspec`.
- **F7 — exact coverage is external and unmeasured by any gate run, and it
  belongs to 8.8.8.3.** D2 and D3 reproduced the script's substance by hand with
  the pinned toolchain (32,802/32,802 `DA`, 5,508/5,508 `BRDA`, 3,237/3,237
  functions on `1d1d9f17`) and reported it as a reproduction, not the gate;
  8.8.8.3's own words agree — "this is preparation, not a literal gate pass".
  `bash scripts/coverage-exact.sh`, the native macOS leg (B26, N2, N4) and
  final-head remote CI all remain pending their actual results. The owner is
  **N13 (8.8.8.3)**, whose text keeps it "pending until actual evidence exists",
  together with 8.8.8.3's own denominator reconciliation against chief
  `124cca78`'s fresh baseline (32148/32324 lines, 5434/5448 branches,
  3125/3135 functions, failed equality). **8.8.14.3 (S13) is not the owner** and
  has no pending half; the first cut assigned the work there.
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
- **F11 — eight numbered tasks under 8.8 are open, five of them on evidence
  nobody here can produce (N1–N4, N11–N14).** §1 grades them. Five turn on
  immutable Apple and env source pins, native macOS platform runs and the
  absent-PATH retained-Node positive, each declared inherited debt by the row
  that carries it. Two are gate executions (N12, N13). One is the
  reconciliation and commit (N14). This is the single largest correction in
  this revision and the one that changes the tickability answers.
- **F12 — this ledger's own first cut is the worked example of its rule.** It
  set out three evidence kinds, said a checkbox and a delivery note discharge
  nothing, and then discharged A6 on five checkboxes, eleven removal-bearing
  rows on delivery narratives, and B64 on a test that proves the opposite of the
  clause. It also omitted a fifth of 8.8's acceptance because the heading above
  it read "Historical". Recording that here is not self-flagellation: the
  failure mode — grading a clause by the shape of the evidence offered rather
  than by opening it — is exactly what a ledger exists to catch, and a reader
  should know which rows were re-opened under it.

## 6. The exact remaining work

Numbered, dependency-ordered, one visit each. Units 1–3 are missing
**evidence**; unit 8 is missing **behaviour**; units 4–5 are **gate
execution**; units 6–7 are **record and reconcile**. Externally owned
prerequisites are named where they bind, and the two commissions nobody in this
repository can discharge are stated as such rather than listed as units.

Unit 2 is split into three. The first cut's single unit 2 could not close the
rows it claimed: one visit cannot add a call-site variant, a compiled-member
symlink fixture and twenty planner vectors, and the fixture it needed did not
exist.

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

2a. **Give each engine route shape its missing call site.** Closes B38's
   single-site half and B39's panel-member half, and A37's call-site half.
   Touches `crates/brokkr-runtime/src/engine/resume_tests.rs` only: one
   non-binding shape driven through the single site, and a panel member whose
   compiled bytes changed after compilation. Proof: each start's
   `resume_context` carries no `route_overlay` (non-binding) or the manifest's
   recorded digest with `assert_ne!` against a hash of the resolved file
   (changed bytes), read off the real `Start.input`.

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

2b. **Build the compiled-member escaping-symlink vector.** Closes the shape of
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

2c-fix. **Refuse a route beside a model pin with no provider segment.** The
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

2c. **Drive the reader's remaining classes through the three planner paths, and
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

3. **Complete the undriven variation matrices.** Closes A42, B60, B27's
   mistyped half and B71's two missing positives. Touches
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
   sequence budget (a row whose newline lands exactly on the injected budget).
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

4. **Run the gate list, in order, on the final candidate.** Closes S11
   (8.8.14.2), **N12 (8.8.8.2)** and B10's gate half, and supplies A53/B30/B42's
   execution once unit 4a below has added their assertions. Touches nothing — it
   is an execution, recorded in a delivery section by unit 6. Commands, in the
   clause's order: `cargo fmt --all -- --check`; `cargo clippy --workspace
   --all-targets --all-features --locked -- -D warnings`; `cargo test -p <crate>
   --all-features --locked` for `brokkr-core`, `brokkr-store`,
   `brokkr-protocol`, `brokkr-runtime`, `brokkr-view`, `brokkr-bridge`,
   `brokkr-cli`, each completing before the next; `openspec validate --all
   --strict`; `cargo run --locked -p brokkr-cli -- compile --bundle
   bundles/self`; then `… bundles/verify`. **Externally owned prerequisite:** a
   seat or host whose permission grant reaches `openspec` (F6) — the review
   seat's grant did; five implement seats' did not. An unavailable tool is not a
   pass. Must follow units 1–3 and 4a so it runs on the head that will be
   ticked.

4a. **Assert the privacy exclusions where the clauses put them.** Closes A53's
   exclusion half, B30's launch-evidence and retained-fields halves, B42's
   second sentence and B55. Touches
   `crates/brokkr-runtime/src/engine/resume_tests.rs` (in the two
   `an_offered_dsh_start_carries_the_recorded_home_at_…` cases, read the emitted
   launch evidence for that attempt and assert it carries no `resume_context`,
   no `owned_target` and no `assessment`, and that its confirmed `root_session`
   and `transcript` fields are retained) and
   `crates/brokkr-protocol/src/adapters/tests.rs` (assert the computed composite
   is unchanged by a bound route — the same install under a bound and an unbound
   route composes to one identity). Proof: the new assertions fail when the
   exclusion is removed in a compiling mutation. **If any of these assertions
   fails on the current production bytes, stop and report it as a Pass-B
   security finding rather than adjusting the assertion** — this unit exists
   because nothing opened establishes the exclusion either way (F8). Ordered
   before unit 4 so its result is inside the gated candidate.

5. **Obtain the external evidence.** Closes **N13 (8.8.8.3)** and supplies
   B26's macOS half, N2's and N4's macOS legs and N11's native prerequisite.
   Touches nothing. `TMPDIR=/tmp bash scripts/coverage-exact.sh` on CI or a host
   that can create the boundary namespace, recording committed revision,
   environment and actual covered/total lines, branches and functions, with the
   denominator reconciled against chief `124cca78`'s fresh baseline; the native
   macOS leg; remote CI on the final candidate head. **Externally owned**
   throughout; record each as pending until its own result exists and never as
   passed. Runs in parallel with unit 4; both must land before unit 6.
   *Not commissionable here* — no seat's grant reaches a capable host — which is
   why it is a unit only in the sense that someone must be asked.

6. **Record, tick and commit the D7/Pass-C/Pass-D account.** Closes S12
   (8.8.15.1) **and with it 8.10** — not 8.8. Touches
   `openspec/changes/2026-09-09-226-session-resumption/tasks.md` only: a
   delivery section recording units 1–5 with their actual commands and results,
   then the checkbox for 8.8.14.2, 8.8.15.1 and 8.10 beside their evidence, with
   8.8, 9.6, 11.x, §1's numbered rows and group 14/15 untouched. Proof:
   `git diff --check`, `openspec validate --all --strict`, the
   requirement/checkbox inventory, frozen surfaces unchanged, 0056 still
   `proposed`, the DSH route still `unmeasured` — then one commit, never pushed.

7. **Reconcile and close the R1–R4 numbered group.** Closes **N14 (8.8.8.4)**
   and, with it, whatever N1–N4 and N11 the operator rules closable. Touches
   `tasks.md` only. It is a separate commission from unit 6 because it answers a
   different question: 8.8.8.4 says "Tick a task only when its **entire**
   acceptance is met; broad owners with inherited pending predicates stay open
   with current repair delivery recorded in prose", and 1209–1212 forbids a seat
   from narrowing full acceptance to earn a tick. So this unit **prepares a
   ruling, it does not take one**: for each of N1–N4 and N11 it records the
   delivered repair (opened at §1), names the inherited predicate still pending
   (Apple/env source pins, native macOS, the absent-PATH retained-Node
   positive), and asks the operator to rule whether the row ticks on its
   delivered half or stays open. Blocked on unit 5 for anything the native legs
   would close.

8. **9.6's warm retained-store integration, starting with the fold.** Not part
   of 8.8 or 8.10; listed because it is the debt those two hand on. Touches
   `crates/brokkr-protocol/src/adapters.rs` (`find_dsh_transcript` /
   `names_the_seats_own_session` must select the OFFERED root's transcript,
   not the first depth-zero one — F4) and `adapters/tests.rs`. Proof: a
   confirmed rejoin beside an unrelated retained sibling folds the offered
   root's current work; a compiling mutation restoring first-match behaviour
   parts that case. It carries a `proposed` decision only if it changes
   semantics beyond the repair. Blocked by 9.6's own first words, "After 8.8
   and 8.10" (5499).

## 7. The three answers

- **Can 8.8 be ticked once these units land?** **No** — not on units 1–7 alone,
  and not by any seat. 8.8's acceptance includes the fourteen numbered tasks at
  1232–1548, of which **eight are open** (§1). Five of those — N1, N2, N3, N4,
  N11 — are open on immutable Apple and env source pins, native macOS platform
  evidence and the absent-PATH retained-Node positive, which their own rows call
  inherited pending acceptance and which nothing in this repository can produce;
  unit 5 is the only path to the native ones and it is not locally
  commissionable. Units 1–6 close 8.8.14.2 and 8.8.15.1; unit 7 prepares N14.
  After all of them, 8.8 ticks only if the operator rules that N1–N4 and N11 may
  close on their delivered-repair half with their inherited predicates recorded
  — a ruling 1209–1212 and 8.8.8.4 reserve to the operator. The first cut
  answered "yes" because it had not inventoried those fourteen rows.
- **Can 8.10 be ticked once they land?** **Yes**, on units 1–4a and 6. 8.10
  scopes itself to "execute **only** 8.8.9.1–8.8.15.1" (5089–5093), so §1's open
  numbered tasks are not its acceptance. Every behavioural clause of its Pass B,
  Pass C and Pass D matrices is discharged or reduced to a named evidence unit
  above; its one undischargeable clause (B101, the real extension's provenance
  comparison) is conditional by its own words on an extension nobody is
  permitted to create. Two qualifications the operator should see: 8.10's B5
  requires an observed mutation for **every** new test, and every such
  observation in this change is a recorded narrative (F5); and B10's gate half
  rides unit 4, which needs a grant that reaches `openspec`.
- **What does 9.6 still wait on?** Its stated precondition at 5499 is "After 8.8
  and 8.10", so it waits on both ticks — and therefore, through 8.8, on the
  operator ruling above and on the external evidence of unit 5. Then its own
  accounting and compatibility acceptance, which is **not started**: cold/warm
  retained-store integration including the verified first-match fold gap (F4),
  historical/current multi-message/tool/retry intervals, output/tool/target
  filtering, usage deduplication, per-message versus cumulative accounting,
  omission of unattributable totals, and legacy compatibility (5499–5524).
  Satisfying 9.6's prerequisites completes none of that; units 6 and 7 open
  9.6's door and do not walk through it.
