# Decision 0065, slice one — evidence after the operator ruling

## Current status — unit 2 council design, 2026-09-23

The rerun adopts all work through `368bc34e`. Design D5.5 reconciles both
positions, retains D5 and assigns the demonstrated sandbox admission defect
to 2.1.5 alongside the reopened proof work. Task 2.1 and all seven substeps
remain open. See “Unit 2 — rerun chief council disposition” below for this
visit's evidence and validation limits; earlier results retain their dates
and scope. No implementation or security hold is closed by this design.

## Whole-rebuild documentation status — historical, 2026-09-23

Rebuild unit 1 has since rebased the branch onto origin/main 072cdd9b. Its
mapping, conflicts, pins and gates are under "Unit 1" at the end of this file.

Adopted branch `slice-0065-capabilities` at **44430402**, specification draft
**a84197cd** and design revision **3c5402be**, retaining every commit. This
sole tasks seat orders the existing rebuild ledger and updates its dependent
documentation. The earlier seats supplied the four-part proposal/specification
and proposed 0066 amendments; this visit preserves them. No production, test,
shipped-data, frozen-byte, rebase, provider measurement or digest change occurs.
The security hold remains unresolved. Task 28.1 stays open; no archive or push.

The operator's [four-part ruling](operator-ruling-2026-09-23.md) supersedes
all evidence of authored-list merging/reconciliation, pinned outward-link
admission, intermediate-only launch proof and per-capability doctor delivery.
Those observations establish only what the old code did; they close no rebuilt
requirement. Design Decisions reconciles every completed council position.
There is no returned_from in this run context; the explicitly named ruling and
positions supply the return findings.

## Sources read by the adopted design visit

- Complete operator ruling first; complete third security/correctness/compliance
  positions in `.forge/tasks/council-positions-80bfd784.md`, including blocked
  adversarial status and distinct validation outcomes.
- Both current design positions, robustness and simplicity, in
  `.forge/design/positions/`; their claims are explicitly adopted, combined or
  rejected with source reasons in design D1. Their reported gates are separate
  observations, not this chief's results.
- First/second chief rulings `.forge/tasks/council-ruling-3c72a18a.md` and
  `.forge/tasks/council-ruling-25d222e6.md`.
- README, decisions 0004/0005/0009/0063/0065, proposed 0066, adopted artifacts,
  provider grammar/composer and declaration/migration surfaces.
- Dialect-owned `dialects/openspec.json`, specify/return/design/tasks instructions,
  plus rendered OpenSpec artifact instructions. No workflow runner invoked.

## Adopted foundations

Checked tasks retain their original evidence and narrow scope. Detailed prior
records remain retrievable from git object
`44430402:openspec/changes/decision-0065-capabilities-slice-one/evidence.md`
and the matching tasks.md; this revision does not rewrite old commits.
The following uses current task IDs; each checkbox retains its previous ID.
This is the retained completion index, not a new test run.

| Tasks | Retained implementation/evidence scope |
| --- | --- |
| 0.2–0.5 | Additive tool-dialect.v1, realms.v6 and run-manifest.v11 contracts and frozen-contract checks. Existing versions remain frozen. |
| 0.6–0.9 | Legacy empty grants, explicit operator context, definition/dialect strict loaders, schema validation and same-byte authority pins; original evidence sections What this visit found and did / Second visit. |
| 0.10, 0.11 | Original-source strict JSON and typed request inheritance/subtraction; source-specific R1/R2 removals below. No authored harness-list exception survives. |
| 0.12, 0.13 | Shipped native declarations/uncertainty and researcher abstract wants/DATA charter. These declarations still owe the new both-halves parsing and exact final-state proof. |
| 0.14 | Whole-loaded-library lint and consulted unseated definitions; R3/R4 removals below. |
| 0.15 | Captured hands-fragment boundary and reassembly integrity only; R-H2b below. New template/typed-local origins remain 3.1/4.1/4.2, including engine dispatch, not claimed done. |
| 0.16, 0.17, 0.18 | Grant validation, pure request/grant intersection and known-native denial floor; first-chief repair and R-H1 proofs below. The total launch invariant is newly open. |
| 0.19 | Independent realm/inventory/MCP-unbuilt reporting with no model or capability-server launch, as recorded in the first-hold implementation doctor evidence. Whole-plan assessment remains open. |
| 0.20 | Separately runnable wants-only compatibility-removal assertions A/B below. They prove notices, not final command delivery. |

### Retained historical removal observations

These rows are copied from the adopted first-repair record, including original
assertions and outcomes. They are not remeasured on this documentation head.

| # | Mutation | Intended assertion | Historical observation |
| --- | --- | --- | --- |
| R1 | unmapped root reverts to the workspace | `an_unmapped_run_reads_and_keeps_the_operated_repository_…` | FAILED at the run that must refuse; the other verb tests green |
| R2 | an unmapped RESUME reads the workspace | same test | FAILED at the resume, with the mismatch refusal — reachable only after the decoy was removed |
| R3 | resume reads today's map, not the run's pin | the map-only-edit test and the unmapped test | both FAILED — and even mutated the engine REFUSED (`pinned grants, dialects, sites no longer match`) rather than borrowing the grant: the manifest comparison is a second line behind the pinned world |
| R4 | the mismatch-door mapping dropped | `a_resume_that_cannot_reproduce_its_pinned_inputs_…` | FAILED at the dialect-gone case, showing the bare compile error the door replaces |
| A | provider compatibility itself removed (`if false && bound != provider`), notice recording and OFF composition intact | `provider_compatibility_drops_a_want_with_its_exact_notice`, whole-vector notice equality | FAILED at `capabilities/tests.rs:1171`, its FIRST substantive assertion: left `[]`, right the one `dropped … because provider 'claude' cannot carry a binding to provider 'test-native'; native capability remains OFF` notice. Restored. |
| B | restriction compatibility itself removed (`if false && !grant.restrictions.is_empty()`) | `cq1_an_inexpressible_restriction_drops_a_want_with_its_exact_notice` | FAILED at `:1459`, first substantive assertion: left `[]`, right the `cannot express restriction 'allow.hosts'` notice. Restored; 32 capabilities tests green; `git diff --stat capabilities.rs` empty. |
| R-H1a | floor removed at its single source (`known_powers` arms renamed) | `a_known_native_power_with_no_valid_denial_refuses_the_seat` | FAILED at `capability_launch.rs:1577`: left the launched argv with NO `web_search` OFF (the delivered H1 defect), right the full refusal with the load failure as cause. |
| R-H1b | the compiler's floor lookup alone removed, the driver's kept | same | FAILED at `:1577`: the shared composer still refuses, but with `declares the provider's inventory unmeasured` — the original cause is lost, and the exact-cause equality detects it. |
| R-H1c | malformed-plan rejection made lenient (required arrays) | `a_plan_is_read_whole_and_a_malformed_one_refuses_naming_its_fault` | FAILED at `native_controls/tests.rs:237` on `'on' is missing`: left `Ok(Some(Controls{held: [] …}))`, right the full refusal. |
| R-H2b | provenance reassembly check disabled | `provenance_is_a_recorded_fact_that_must_reassemble_the_argv` | FAILED at `:1378`: left `Ok((["--sandbox","read-only"], []))`, right the full refusal. |
| R1 | `agents/load.rs` strict parse removed | `a_request_key_written_twice_in_an_agent_source…` | ONLY that test FAILED, at the equality (`tests.rs:400`): left `loaded {"web-search": Wants}`. |
| R2 | `compose.rs::read_layers` strict parse removed | `a_request_key_written_twice_in_a_recipe_layer…` | ONLY that test FAILED (`compose_tests.rs:1413`): left the composed map with both wants. |
| R3 | compile lint disabled, CLI lint untouched | `an_unseated_loaded_agent_with_an_undefined_request_refuses_the_compile` | FAILED at `agent_tests.rs:754`: left `compiled 'fixture'`. |
| R4 | consulted-unseated-definition contribution removed (`library_asks.take(0)`) | `a_definition_only_an_unseated_agent_names_is_pinned…` | FAILED at `:806`: left `[]`, right `["web-search"]`. |
| lexical | fixture root used lexically (the macOS habit) | `a_native_declaration_with_a_repeated_key…` | FAILED at its first full-equality assertion with the `real`/`alias` mismatch; canonical root restores it. |

The original duplicate-adapter, selected sequence-fallback and unmapped-resume
with decoy-removed regressions also remain retained, as recorded directly after
that historical removal table. Their scope is preserved; no broader final
command or owner-target proof is inferred.

## Reopened claims and why

| Old completion basis | Current status / evidence against it |
| --- | --- |
| Authored Claude/LaneTally local-list preservation, list folding and MCP subtraction | Superseded by ruling 1. S1/S2/C1/R1 reproduced malformed/widened lists; R6 found wildcard/hands conflict. All authored capability options now refuse; former positive tests must change. |
| Managed controls parsed by an intermediate composer | Incomplete. S3/C4/R4/R8 reproduce unchecked Codex argv and final prefix/selector disagreement; C2 proves separator serialization changes denial meaning. Both declared halves and final actual command now need validation. |
| Read/empty final positives and restriction-resume transport test | Narrower evidence only. C6/R10 identify hand-built plans and missing production compilation; 21.1–21.3 and 23.1 reopen. Generic --settings syntax does not qualify every restriction object; 9.1 reopens. |
| Outward links admitted because walk hashes their content | Revoked by ruling 3, an upstream artifact defect (C7/R3). Never a valid substitute for containment. The adopted specification/design revisions corrected the owning artifacts and 0066. |
| Every in-tree policy is necessarily pinned | Disproved by C3/R2 FIFO policy reproduction. 16.1/16.2 require regular-file refusal and binding the parsed buffer to file-map identity. |
| Library digest verification closes owner/target obligation | Incomplete. C8/R5 equal-byte retarget bypasses owner containment; 17.1, 18.1–18.2 and 19.1 reopen while verified-buffer rendering remains useful narrower evidence. |
| Per-capability provider-aware doctor is sufficient | Disproved by S4/C5/R7 combined OFF conflicts. 22.1/22.2 reopen for whole-plan composer admission in both report paths. |
| Completed matrix and removal audit | R10 names holdings-only/is_ok/manual-plan cases and absent per-case baseline records; 25.1/21.3/25.3 reopen. R12 reopens the complete canonical-root audit. |
| Historical exact-coverage pass closes V1 | False for a new candidate. Preserve distinct measurements below; 27.4 stays open until fresh final-head external equality. |

No old removal showing that list merging survived, that a pinned outward link
compiled, or that a single capability composed is cited as positive evidence
for the operator's new rule. Old full histories remain available through the
adopted git object for audit, with no current completion claim attached.

## Inventory and rebase observations

Recursive reads covered adapters/, recipes/, agents/ and extensions/, then
bundles/ to catch the shipped verify witness. The exact file dispositions and
typed migrations are in design Migration Plan. Concrete inline flag sites:
fast (implement/review), node (implement/review), preflight (review), verify
(review), standby (implement/review), review-first (Codex review) and
wager-harness (Codex implement). Adapter templates/generated fragments are
engine data with an explicit-origin obligation. No agents/*.json authors an
inline flag; researcher already uses typed requests. Extension profile strings
are CLI definitions/examples; no extension-code migration is planned. The
research-dsh patch contains route metadata only and retains validation.

`git log HEAD..origin/main` read seven commits: b0ec5517 (#313), efbb035c
(#315), 314e8786 (#320), e5921db6 (#321), 3d978bfd (#322), e50020ac (#323),
072cdd9b (#326). This is a local-ref observation, not a remote freshness claim.
Unit 1 must capture current origin/main at execution, retain all adopted slice
work and measure roster/version-sensitive identities. No rebase happened here.

## Historical measurements — not current passes

| Source / candidate | Recorded result and limit |
| --- | --- |
| Second chief, 3b31c5de | Exact coverage failed: 34897/35073 source lines, 5730/5744 branches, 3443/3453 logical functions, exit 1. |
| Second implementation record, through b6dbb057 | Reported 35366/35366 lines, 5764/5764 branches, 3505/3505 functions. Historical report only; third council did not reproduce this pass. |
| Third correctness rerun, b6dbb057 | Fresh failed exact gate: 35190/35366 lines, 5750/5764 branches, 3495/3505 functions, exit 1. |
| Third security/compliance attempts | Coverage/workspace attempts stopped at bridge socket PermissionDenied, exit 101; fresh counts unavailable. Existing reports are not measurements from those attempts. |
| Third council self/verify compile | Self 09ff39b7d3f38faf1beb6283b5ca15cea1f7252cbf4062162d509715d155924e; verify fcbfe0ec7281992d946e983808e30a9eded91cbb96a92e35749d39dc3155bd2f. No new pins measured here. |
| Third validation | Formatting/clippy/OpenSpec/diff and compiles were reported green; suite outcomes varied by seat/environment. Correctness recorded one intermittent workspace test then a passing rerun. No root cause or blanket pass is inferred. |

Exact coverage on the rebuilt final head must run outside the nested workspace
box on a capable host/CI, unchanged pinned compiler and threshold, with all
three nonzero covered/total measures equal. Linux and macOS results and remote
CI remain separate pending evidence. No native Windows host is required (0063).
No publication/channel/profile action is part of this documentation visit.

## Measurement limits that survive

The supplied Codex 0.154.0 measurement covers cold exec only, using
`-c web_search="disabled"` for OFF and measured default ON. Resumed OFF/ON,
explicit ON values and other versions remain unmeasured. Claude live controls,
DSH/LaneTally native inventory and controls, and ambient Codex MCP isolation
remain unmeasured. Deterministic command evidence establishes composition only.
MCP runtime, slice-two gate policy/retention and slice-three comparisons remain
out of scope. Only the operator can grant capabilities or accept proposed 0066.

## Specification visit gates at a84197cd — historical

- `git diff --check`: passed, exit 0.
- `openspec validate --all --strict --no-interactive`: passed, 16 items, zero
  failures. Existing long-requirement and unrelated issue-226 archive notices
  are informational; no unrelated artifact was changed.
- `cargo fmt --all -- --check`: could not start, exit 127, `cargo: command not
  found` in the workspace hands. Formatting is unavailable, not a pass.
- No production/tests/data were changed. This narrow commission runs the three
  requested documentation gates; it claims no fresh workspace suite, clippy,
  bundle compilation, exact coverage, macOS, remote CI or provider result.

That specification visit reported formatting unavailable. The two design seats
subsequently reported three passing documentation gates; those remain their
observations, not a reason to rewrite the earlier failure or infer this chief's
result. The rebuild ledger stays open wherever implementation proof is owed.

## Chief design reconciliation at 3c5402be — historical

Read both positions whole and checked their boundary claims against the current
source. Candidate::parts and SiteSpawn::launch_arguments carry only the old
hands split; grammar Problem displays raw tokens and config_key strips quotes;
Codex composition still returns controls.argv verbatim. active_input returns a
joined path, own_table reads policy separately from the walk, CharterPin lacks
owner/target, and doctor uses denial_on in both reporting paths. These are
source observations, not new behavioral probes or repaired code.

Amended the proposal, six deltas, design, proposed 0066 and ledger in dependency
order. Decisions now specify end-to-end private origins, a checked final command
without subsequent argv edits, handle-bound contained input reads or refusal,
exact local migration and complete doctor positives. Added scenarios at the
owning deltas, including omission/empty local lists and diagnostic bounds.
Re-read the migration inventory and local main log: the seven bundle files and
seven main commits above still apply; --add-dir and Codex --include-plan-tool
are additionally explicit authored refusals because the existing grammar
admits those permission/tool options. No new remote freshness claim is made.

The single rebuild order has 27 visits. Task substeps 3.1/4.1/13.1
expose previously bundled work; they are unchecked. Unit 1 remains rebase and
measured pins, migration precedes refusal, and final assessment/cold/resume,
charter dispatch/start-resume and compiled matrix/restriction proofs have
separate visits. No feature unit exceeds three named production files.
Qualification of the existing supported nonempty-restriction positive remains
open in 9.1: neither panel supplied a provider-backed transport proof.

This chief changed no production, test, shipped data, frozen bytes, witness
pins or history. No behavioral finding, security hold, external gate, live
measurement or task 28.1 is closed by this design revision.

## Chief documentation gates at 3c5402be — historical

Results are recorded below after running the commissioned commands through the
workspace tool. This narrow visit does not run implementation suites or claim
fresh clippy, bundle, coverage, macOS or remote CI results.

- `git diff --check`: the first attempt caught one new blank line at EOF in
  this file; it was removed. The final check passed, exit 0.
- `cargo fmt --all -- --check`: could not start, exit 127, `cargo: command not
  found` in this chief's workspace tool. No formatting pass is claimed; the
  earlier positions' reported passes do not discharge this blocked attempt.
- `openspec validate --all --strict`: passed, exit 0, 16 items and zero
  failures. Existing long-requirement and unrelated archive notices were
  informational. No workflow runner was invoked.
- Scope audit: exactly the requested eleven Markdown artifacts are modified;
  proposed 0066 stays proposed and every implementation task remains open or
  retains only its explicitly indexed historical foundation evidence.

## Tasks visit, 2026-09-23

Read the operator ruling first and whole, then all completed third-council
positions and both earlier chief rulings. Reviewed the adopted proposal, design,
six deltas, 0066 and evidence; checked the local provider grammar, dialect tasks/
return instructions and file inventory. Used recursive grep because rg is absent.
The seven shipped bundle migration sites and seven local origin/main commits
above remain unchanged at 3c5402be. No remote freshness or runtime proof is claimed.

Reordered all 72 checkboxes into retained foundations, the 27 design units in
execution order, and the separately gated final archive task. Renumbered current
references while preserving each previous ID and the 20 checked artifact/
foundation rows. Every implementation/validation obligation remains open.
Every task now names its served requirements; the omitted NC4 coverage is explicit
in authored refusal and resumed/fallback control work. All 45 requirements have
scenarios and task citations. Each unit closes tasks in its own group, with
cross-unit aggregate completion deferred until its last dependency. Unit 1 remains
rebase/re-pin/gates; the separate recipe migrations precede refusal.

No earlier artifact must change to order this work honestly. The existing unit 9
qualification is an explicit evidence task: if a bounded supported nonempty
restriction cannot be established, its smith returns upstream before dependent
load/final-delivery work. This draft does not invent that transport or mark it done.

### Tasks documentation gates

- `git diff --check`: passed, exit 0.
- `cargo fmt --all -- --check`: unavailable, exit 127, `cargo: command not
  found`. The workspace PATH names a cargo directory that is not present;
  no host-shell substitute or formatting pass is claimed. This gate remains
  blocked by the missing workspace toolchain, not by an upstream artifact.
- `openspec validate --all --strict`: passed, exit 0, all 16 items. Existing
  long-requirement and unrelated archive notices remain informational.
- Task audit: 72 unique sequential checkboxes, 20 retained checks; every one
  of the 45 requirements has scenarios and a named citation. Task groups and
  design unit closures agree; prior IDs remain traceable.
- Scope: six Markdown files only; no production/test/data/frozen-path changes.
  The inherited 0066 amendment remains proposed. No fresh suite, clippy,
  compilation, exact coverage, macOS, remote CI or provider pass is claimed.

The phase result is a drafted breakdown, not a fully green implementation or
a documentation-gate completion claim. The unavailable formatting command is
preserved explicitly for the next workspace/host validation visit.

## Implement visit, 2026-09-23

This commission is documentation only; its one implementation task is 0.1.
Audited the adopted artifacts at 955f588c against the commission's four
deliverables and changed none of them:

- The ruling's four parts own their requirements: authored refusal and the
  exhaustive per-harness catalogue in the realm delta, including split, `=`,
  attached `-xVALUE` and alias forms and all five Codex config spellings;
  load-time ON/OFF parsing and final-command proof in the native delta;
  containment refusal with the pinned-outward-link permission revoked in the
  manifest delta; and whole-plan doctor submission in the doctor delta.
  Proposed 0066's ruling 2 states the total principle and it stays proposed.
- Re-ran the recursive inventory over adapters/, recipes/, agents/, bundles/
  and extensions/. It matches design's Migration Plan file for file: seven
  bundle files author inline flags (fast, node, preflight, verify, standby,
  review-first, wager-harness), no agents/*.json does, and the DSH extension
  hits are CLI examples only.
- `git log HEAD..origin/main` on the local ref still lists the seven
  commits unit 1 names (#313, #315, #320, #321, #322, #323, #326). No fetch
  was made, so remote freshness is still for unit 1 to record.
- Rebuild units: unit 1 is the rebase. Migration units 6–8 come before
  refusal in unit 12. No feature unit names more than three production files.

Gates on this visit's candidate:

- `git diff --check`: passed, exit 0.
- `cargo fmt --all -- --check`: passed, exit 0. This supersedes the two
  earlier unavailable attempts for this documentation head only.
- `openspec validate --all --strict`: not run. The seat's command
  permissions refused the invocation before it started. No pass is claimed.
  The last observed result is the tasks visit's pass (16 items). This visit
  changes only this Markdown file, so no delta or task structure moved.

No production, test, shipped data, frozen or witness bytes changed. Every
implementation obligation from unit 1 onward remains open.

## Unit 1 — rebase onto origin/main, 2026-09-23

Run `build-decision-0065-slice-one-re-08fc67a8`. `git fetch origin` captured
origin/main at **072cdd9b**. The old merge base was 5347c667 (engine 0.10.0).
Main's seven-commit advance matched the inventory: b0ec5517 (#313), efbb035c
(#315), 314e8786 (#320), e5921db6 (#321), 3d978bfd (#322), e50020ac (#323),
072cdd9b (#326). `git rebase origin/main` replayed all 45 adopted commits
(028871a2..6cac60bb), including a84197cd, 3c5402be and the tasks amendment,
with the steward signature. None was dropped or squashed. No feature repair
was made.

### Replay mapping

`git range-diff 5347c667..6cac60bb origin/main..HEAD` reports `=` (same
patch) for 39 commits. It reports `!` for six: five had conflicts resolved,
and commit 34 had only a context shift. It picked up main's `gpt-6-sol` roster
line in the surrounding test context and did not change the patch.

| # | old | new | # | old | new | # | old | new |
|---|---|---|---|---|---|---|---|---|
| 1 | 028871a2 | ac665a9d | 16 | 6ec71432 | 2509b22f | 31 | 49fda5e6 | 787afcc5 |
| 2 | 205eeb3e | a36fb0a1 | 17 | 12641369 | 75c41f9f | 32 | 90bf9d44 | eb5d59ec |
| 3 | 303455fb | a3f70f77 | 18 | 9c709e22 | 203086e4 | 33 | a8950ded | e239acb7 |
| 4 | ba7708ab | 1cb1d8d0 | 19 | 07c0653c | 7c65a3c2 | 34 ! | c68b0e34 | dd7a33ad |
| 5 ! | bc3e1a3d | 53cfd0a0 | 20 | ecbdd68c | b4705c3a | 35 | 6103af88 | 1cdc3f1f |
| 6 | 9fb2f3d9 | 7f20a7ef | 21 | f0264a9b | ad7940ce | 36 | c2e6950d | 5e12fff7 |
| 7 | 69d67672 | c9f4cbb2 | 22 | 5c53a30f | 5cb59d81 | 37 | 2ad20be9 | 879d7c2a |
| 8 ! | 65ddc65a | dfadcc9c | 23 | 6d47c120 | 3d2ef50d | 38 | 703a8897 | 7c636342 |
| 9 ! | 4dc772b0 | 74460ce1 | 24 | e5408669 | 19ff3d2e | 39 | 27a28c90 | c6e7e71e |
| 10 | 1ea9e6d0 | 101b13bd | 25 | cc0a9c31 | 688d4d65 | 40 | b6dbb057 | 89e46464 |
| 11 | b2aff549 | 813e71f6 | 26 | b34e9c9a | 21714d8c | 41 | 44430402 | 1703fbdf |
| 12 | 7b5601bd | af96d75e | 27 ! | 575915c8 | 23f57ea2 | 42 | a84197cd | 3119fd03 |
| 13 | 4fab9683 | 61b19e43 | 28 | 59c3aa9c | db83c85c | 43 | 3c5402be | feb5af98 |
| 14 ! | 97991a25 | 4f24af60 | 29 | f97b7e77 | ec8b96f3 | 44 | 955f588c | 4010bdb2 |
| 15 | 3215b38d | cadbc98f | 30 | 3b31c5de | 809be145 | 45 | 6cac60bb | 784a1759 |

The unit's own commit sits on 784a1759. It carries the measured pins, the two
replay adaptations below and this record.

### Conflicts, against the inventory

- **Inventoried production files:** `adapters/claude.json`, `adapters/codex.json`
  and `adapters/lanetally.json` auto-merged with no textual conflict. The merged
  tree keeps main's roster lines: `opus: claude-opus-5-5`, `sol: gpt-6-sol`,
  `luna: gpt-6-luna` and `opus-tallied: claude-opus-5-5`. Next to them are the
  slice's `native_capabilities`. `Cargo.toml` and `Cargo.lock` equal main's
  (v0.11.0, `git diff origin/main HEAD` empty for both).
- **Production conflict not in the inventory, a fourth production file:**
  `crates/brokkr-protocol/src/adapters.rs`, `dsh_launch_with`, at
  replayed commits 14 and 27. (The first record called this "below the ceiling
  of three" and within scope. That was wrong. The unit requires an inventoried
  split *before* a fourth production conflict is resolved, and it was resolved
  without one. See "Review return" below.) #313 put `dsh_input_boundaries(extra)?` at the
  head of the function. The slice put its native-control guard there too:
  `native_controls::managed` at commit 14, and at 27
  `composed_launch("dsh", extra, input)` with `extra` rebound to the composed
  argv. The resolution keeps both, the slice's first, so its "refused before
  the seat's argv is read" still holds. Main's boundary check then runs on the
  argv as handed over. Main's DSH tests carry no `native_controls` key, so the
  slice guard admits them unchanged. No other production file conflicted.
- **Inventoried test/pin conflicts:** `tests/witness_digests.rs` and
  `bundle/compose_tests.rs` at commit 8. Both sides' prose was kept and the
  slice's values were taken as placeholders. They were then re-measured at the
  tip (below). `bundle/model_policy_tests.rs` had no textual conflict. It
  broke semantically (below).
- **Test and documentation conflicts not in the inventory:**
  `crates/brokkr-protocol/src/adapters/tests.rs` at commit 5, where both sides
  appended at end of file (#326's Pass D drift cases and the slice's
  native-control cases), and `docs/guides/provider-adapters.md` at commit 9
  (#315's "A work seat on a provider with no per-tool flags" subsection and the
  slice's "Native capabilities" section). Both sides were kept verbatim in each
  file.

### Semantic replay breaks: baseline red, then fixed in the unit commit

- `crates/brokkr-runtime/src/agents/tests.rs`: #315's test
  `harness_work_support_cannot_rescue_a_boxed_seat_without_a_workspace_fragment`
  called `compose(…, &mut Vec::new(), …)`. The slice had removed the `notices`
  parameter by then. Baseline red: `error[E0061]: this function takes 5
  arguments but 6 arguments were supplied` at two call sites. Fix: drop that
  argument. The assertions are unchanged.
- `crates/brokkr-runtime/src/bundle/model_policy_tests.rs`: #315's synthetic
  `smith_codex` adapter declared no `native_capabilities`. Four tests failed
  with the slice's 0066 ruling 1 refusal:
  `the_same_seat_with_hands_compiles_under_namespace_through_the_workspace_fragment`,
  `the_same_seat_under_harness_compiles_on_the_providers_work_fragment`,
  `the_same_seat_is_refused_under_harness_naming_the_measured_work_gap` and
  `the_same_seat_is_refused_under_harness_when_the_provider_declares_no_work_fragment`.
  The refusal was "seat 'work' (office 'smith') in realm '<unmapped>':
  provider 'codex' is known to carry native capability 'web-search', which
  this seat does not hold, and no valid control denies it: its adapter
  declares its native capabilities unmeasured …". Fix: the fixture declares
  the shipped adapter's measured OFF (`-c web_search="disabled"`). After the
  fix all four pass with their assertions unchanged. The baseline red is the
  removal record: without the declaration all four fail on that exact
  refusal.

### Measured pins

Every value is the left side the test reported, copied one failing pair at a
time. None was recomputed. They moved because engine 0.10.0 became 0.11.0
(#321), which enters every manifest's identity, and because the claude and
codex model maps changed (#320).

| Pin | replayed | measured |
|---|---|---|
| witness `recipes/fast` = compose `recipes/fast` | dcc9f956… | 72b516cf… |
| witness `recipes/node` | b20e648b… | b53b1105… |
| witness `recipes/preflight` | 451b90c7… | dfcf0823… |
| witness `recipes/night-shift` | 11bd9bec… | b1eab215… |
| witness `recipes/wager-harness` | 0f13ccdd… | 96033813… |
| witness `recipes/triage` = compose triage manifest | e4f24ee6… | d888665e… |
| witness `recipes/research` | cd9b978b… | ce0fd9f4… |
| witness `recipes/research-dsh` | 22d9f849… | a58359d5… |
| witness `recipes/gpt-flash` | d434415b… | 2533f3b9… |
| witness `bundles/verify` = compose `bundles/verify` | 3248ac90… | fbceddeb… |
| compose `recipes/panel-review` | 11cb41f0… | 9725d931… |
| compose `bundles/self` | 596541a8… | c4e36f6f… |

### Gates on the unit candidate (784a1759 plus the unit's working tree)

- `cargo fmt --all -- --check`: passed, exit 0, after `cargo fmt --all`
  reflowed the two edited `compose` calls.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`:
  passed, exit 0, rerun after the last edit.
- `cargo test --workspace --all-features --locked --no-fail-fast`: exit 0 over
  77 test binaries: 2412 passed, 0 failed, 5 ignored. After rustfmt's
  whitespace-only reflow, `cargo test -p brokkr-runtime --all-features`
  passed again (25 binaries ok, 0 failed).
- `cargo run -p brokkr-cli -- compile --bundle bundles/self` and
  `--bundle bundles/verify`: both exit 0.
- `git diff --check`: passed, exit 0.
- Frozen bytes: `policy/phase-machine.json`, `policy/schemas/`, `fixtures/`,
  `reference/` and `extensions/` are equal to origin/main. `contracts/`
  differs only by the adopted slice's additive new-version files, and this unit
  touched none of them.
- `openspec validate --all --strict`: **not run.** The seat's command
  permissions refused it. No pass is claimed.
- External exact coverage (`scripts/coverage-exact.sh`): **pending.** The
  seat's permissions refused it, so it needs a host or CI run.
- macOS host and remote CI on the pushed head: **pending.** This Linux seat
  never pushes.

Pending and not-run gates are not green. The replay, the pins and the local
gates are done. The unit is not complete (see below).

### Review return, 2026-09-23: manual repairs inventory and split

Review of 6cac60bb..07d65e71 (run `build-decision-0065-slice-one-re-08fc67a8`,
gpt-6-astra, residual/medium) returned the unit with two findings.

- **C1:** task 1.1 was ticked even though external exact coverage and macOS
  were still pending. It is reopened. It stays open until those results exist.
  Remote CI remains a pending handoff.
- **C2:** the record above treated an unlisted production conflict as within
  scope. That was wrong. This section inventories every manual repair the
  replay made outside the unit's named files. Nothing was changed or
  re-resolved in this visit.

**Named by the unit** (within scope): `adapters/claude.json`,
`adapters/codex.json` and `adapters/lanetally.json` (auto-merged, no manual
edit); `tests/witness_digests.rs` and `bundle/compose_tests.rs` (resolved at
dfadcc9c, commit 8, and re-pinned at 07d65e71); `bundle/model_policy_tests.rs`
(semantic fix at 07d65e71).

**Not named by the unit**, one row per manual repair:

| # | File | Kind | Where | Manual content |
|---|---|---|---|---|
| X1 | `crates/brokkr-protocol/src/adapters.rs` | **production** | 4f24af60 (commit 14), 23f57ea2 (commit 27) | In `dsh_launch_with` both guards were kept, and the order was chosen by hand: the slice's guard (`native_controls::managed`, later `composed_launch`) runs first, then #313's `dsh_input_boundaries`. #313's comment "Original adjacency first" was reworded to "next". |
| X2 | `crates/brokkr-runtime/src/agents/tests.rs` | test | 07d65e71 | #315's `harness_work_support_cannot_rescue_a_boxed_seat_without_a_workspace_fragment` drops the `&mut Vec::new()` notices argument at two `compose` call sites (baseline `E0061`). The assertions are unchanged. |
| X3 | `crates/brokkr-protocol/src/adapters/tests.rs` | test | 53cfd0a0 (commit 5) | End-of-file append conflict: #326's Pass D cases and the slice's native-control cases, both kept verbatim. |
| X4 | `docs/guides/provider-adapters.md` | docs | 74460ce1 (commit 9) | #315's work-seat subsection and the slice's "Native capabilities" section, both kept verbatim. |

**X1 is unproven.** The resolution claims the slice guard refuses "before the
seat's argv is read". No test binds that order. Mutation in this visit: in
`dsh_launch_with`, `dsh_input_boundaries(extra)?` was moved above
`composed_launch("dsh", extra, input)?`, so it ran on the raw argv. The code
compiled. `cargo test -p brokkr-protocol --all-features --locked` passed in
full: lib 466 passed, 0 failed; `seatbelt_lifetime_probe` 99 passed, 2
ignored; the remaining binary 1 passed. No test failed, so no failing
assertion can be recorded. The order swap was then reverted, and
`git status` was clean afterwards. The two guards' relative order is
therefore arbitrary as far as the suite can tell. A plan that carries a native
control *and* a boundary-faulted argv (for example `--model --effort`) would
show the difference, but no test builds one.

**Requested split** (a plan amendment for the council or operator. This seat
does not edit design.md's accepted Rebuild units):

- **Unit 1 (as ruled):** the replay, the three adapter JSON files, the three
  named test/pin files and this record. Amend its inventory to name X2–X4.
  They are test and docs conflicts or semantic breaks that the replay needs
  just to compile or keep both sides. They need no production judgement.
- **New unit 1b, DSH guard order after #313:** one production file
  (`crates/brokkr-protocol/src/adapters.rs`) plus its owning suite
  (`adapters/tests.rs`). Rule which refusal wins when a launch carries both a
  native-control plan fault and a DSH boundary fault. Add a test that asserts
  the exact winning reason. Record this visit's order swap as its binding
  mutation (fails, then restored). It must land before any unit that reworks
  DSH composition under operator ruling 2 (final-command parse-back). X1's
  resolution stays in history as replayed. Unit 1b proves it or reverses the
  order; it is not re-resolved in unit 1.

Gates in this visit: the `brokkr-protocol` suite under the mutation, as
above. This visit changed only `tasks.md` and `evidence.md`, and
`git diff --check` passed. `openspec validate --all --strict` was refused by
this seat's command permissions and is **not run** here. The review
independently recorded a strict 18/18 pass on 07d65e71, before this
evidence-only edit. External exact coverage, macOS and remote CI are still
**pending**.

## Unit 1b — the DSH guard order, X1 bound, 2026-09-23

Run `build-decision-0065-slice-one-re-bc7556ea`, based on c5aacc75. This is
task 1.2: the split of X1 that the review return requested.

**(i) The order, from the specs.** Triage found that no clause decided the
order between the two guards. The operator ruled it in the addendum to
`operator-ruling-2026-09-23.md` (c5aacc75). The authority refusal wins:
composition runs first, and it refuses a missing authority, a native control
the launch does not consume, or an unparseable authored argv before the
boundary check reads any argument. The boundary check then inspects the
composed argv (ruling 2). This visit records that addendum as the
native-control delta requirement "A DSH launch refuses authority before it
reads its boundary", with one scenario. The replayed code already follows the
ruled order, so **the order did not change**. The only production edit is to
the `dsh_launch_with` comment, which now cites the addendum.

**(ii) The case that tells the orders apart.** The new test is
`adapters::tests::a_dsh_native_control_is_refused_before_the_boundary_check_reads_the_argv`
in `crates/brokkr-protocol/src/adapters/tests.rs`. Its fixture uses a
canonicalised tempdir as both the workdir and `DSH_HOME`. It installs no
provider (`/nonexistent/dsh`), and its composite closure panics if it is
called. Every assertion compares the exact reason:

| Input | Argv | Refusal |
|---|---|---|
| none (by hand) | `--model -`, `--model --effort` | `dsh driver: --model needs a model id after it` |
| unmeasured plan (the plan the engine writes) | `--model -` | the same boundary reason |
| known plan, managed argv `--effort low` | `--model -` + fragment | `…carries managed arguments for provider 'dsh', which its launch does not consume…` |
| known plan, a tool selection | `--model -` | `…carries a tool selection for provider 'dsh'…` |
| known plan, managed argv | `--model --effort` + fragment | the seat's arguments do not parse: the 'dsh' grammar cannot place argument 2 ('--effort')… |
| `native_controls: null` | both | the missing-authority reason (`NO_AUTHORITY`) |
| unmeasured plan (positive) | `--model deepseek-v4-flash --effort high` | launches as `dsh --profile headless --patch <overlay>`, with no refusal |

Two things were observed while building the case.

- **`--model --effort` alone cannot bind the order under a plan.**
  Composition parses the authored argv against the DSH grammar, and that parse
  already refuses an option in a value slot. So under any engine-written plan
  this argv is refused at composition whatever the plan carries, and it never
  reaches the boundary. The shape that separates the two guards is
  `--model -`: the grammar reads the lone `-` as a positional and accepts it
  as the model value, while the boundary check refuses any value that starts
  with `-`. `--model --effort` is kept as a composition-reason row.
- **An `unmeasured` plan's decoder ignores `argv` and `selection`**
  (`native_controls.rs` `decode`). A native control therefore rides only a
  `known` plan, which the fixture uses. A plan that carries those keys under
  an unmeasured inventory is not refused, and its launch-arguments fragment
  still reaches the composed argv. That code is outside this unit's files and
  has not changed. It is recorded here for the unit that owns the plan decode
  under operator ruling 2.

**(iii) Binding mutation.** In `dsh_launch_with`, `dsh_input_boundaries(extra)?`
was moved above `composed_launch("dsh", extra, input)?`, so it ran on the raw
argv. This is the swap recorded under "Review return", and the code compiled.
`cargo test -p brokkr-protocol --all-features --locked --lib dsh` then showed
**87 passed, 31 failed**. The one real failure was the new test's
managed-arguments assertion (`adapters/tests.rs:14823`,
`assert_eq!(launch(&with_managed, &input).err(), Some(unconsumed("managed arguments")))`),
with left `Some("dsh driver: --model needs a model id after it")` and right
the managed-arguments refusal. The other 30 were `ADAPTER_ENV` `PoisonError`
failures in sibling tests, caused by that panic. The swap was then reverted,
and `git diff` showed only the comment edit in `adapters.rs`.

**Gates** (c5aacc75 plus this unit's working tree): `cargo fmt --all -- --check`
passed. `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
passed. `cargo test -p brokkr-protocol --all-features --locked` passed: lib 467
passed and 0 failed (466 before, plus this test), `seatbelt_lifetime_probe`
99 passed with 2 ignored, and the remaining binary 1 passed. `git diff --check`
passed. `openspec validate --all --strict` was refused by this seat's command
permissions and was **not run**. `cargo run --locked -p brokkr-cli -- compile
--bundle bundles/self` passed (digest `185ef2ca…d894`). The
`brokkr-runtime` `witness_digests` pins passed 4/4, so no pin moved. The
full workspace suite and the verify bundle were not rerun in this visit.
Task 1.1 keeps unit 1's external gates, which the
host owns: exact coverage on a4b08863 is running, and draft PR #319 carries
macOS and remote CI. None of them is claimed here.

## Unit 2 — specification adoption, 2026-09-23

Run `build-decision-0065-slice-one-re-9f0b932b`, phase `specify`, based on
`5a47b090aae6d8e67487ff75b7e295240d372646` on `slice-0065-capabilities`.
Adopted `decision-0065-capabilities-slice-one` and all preceding history;
no replay, new change, implementation or task closure occurred. Task 2.1 is
still open. The run context supplies no `returned_from`.

Read the operator ruling and its addendum first, then the Rebuild units
preamble/unit 2, D5, task 2.1 and its SC7/SCM/RGR deltas. Also read README,
decisions 0004/0005/0009/0065, the relevant 0046 boundary ruling and proposed
0066's status, the named loaders/suites and their history. Read
`dialects/openspec.json`, its specify/return files and OpenSpec's rendered
`instructions proposal` then `instructions specs` for this adopted change.
No workflow runner was invoked and no new council was convened.

Amended artifacts in dependency order: proposal, the owning
seat-capability-resolution delta, tasks and this evidence. The delta's
`## Decisions` records why D5 rejects the historical ambiguous-empty reading,
why omission inherits per field, and why sandbox narrowing cannot override
realm/boundary authority. Its scenarios give concrete typed values, malformed
inputs, subset/widening cases and executable/container placement. The existing
realm refusal delta and design's Rebuild units remain coherent without edits;
raw-option refusal and final composition retain their assigned later units.
Decision 0066 remains proposed.

### Source observations and intended baseline reds

These are source observations at the adopted head, **not executed tests**.
The implementation visit must capture the real failing tests before a fix.

| Owning file/suite | Observed baseline | Intended unit 2 assertion |
| --- | --- | --- |
| `agents/load.rs`; `agents/tests.rs` | `parse_tools` accepts only allow/mcp and rejects an empty allow array as ambiguous; the existing loader table expects that rejection. | Exact decoding keeps explicit `[]` distinct from omission and retains each valid sandbox class; invalid fields retain their complete causes. |
| `agents.rs`; `agents/tests.rs` | `Agent` has optional ordered allow but no typed sandbox field. Local composition and hands already have separate authority rules. | Exact local values survive validation, including inherited and explicit empty; no capability or hands authority is inferred. |
| `bundle.rs`; `bundle/agent_tests.rs` | Seat/body/member/step closed key lists lack tools, so inline and agent-backed local declarations cannot yet reach the D5 rules. | Each executable form accepts exact local declarations or refuses exact malformed/widening/boundary causes; a container does not absorb a declaration. |
| `bundle/agent_tests.rs` | `AgentFixture` currently derives paths from its lexical TempDir, while `agents/tests.rs` already retains a canonical root. | Unit 2 fixtures derive paths and full expected causes from one canonical temporary root. No provider installation or `.forge/` read is permitted. |

No Rust test was added or mutated by this specification visit. Baseline reds,
independent compiling removal failures (test and assertion), restored passes
and runtime validation remain owed by task 2.1 in its two named suites. This
visit does not present decoder acceptance as proof of lowering or final launch.

### Observed validation and limits

- `openspec validate --all --strict`: **passed, 18/18**. Informational notices
  include long existing requirements and unrelated archive-target notices for
  `adapter-resume-safety` / `sdd-progress-markers`; no validation failed.
- `git diff --check`: **passed**.
- `cargo fmt --all -- --check`: **unavailable**, exit 127, `/bin/bash: cargo:
  command not found`. Neither cargo/rustc/rustup is on this seat's PATH; the
  conventional cargo binary locations checked also yielded no executable.
- Workspace clippy, the runtime suite, `cargo test --workspace` and the cargo
  compile of `bundles/self`: **not run**, because this boxed specify seat has
  no Cargo toolchain. No existing target binary substitutes for those gates.
- External exact coverage, macOS and remote CI: **pending**, with no new
  external result observed. This does not close unit 1's outstanding gates or
  establish a fully green unit 2.

Only proposal.md, specs/seat-capability-resolution/spec.md, tasks.md and
this evidence file change. Production, tests, pins, grants, frozen contracts,
policy/schemas, policy/phase-machine.json, fixtures, reference and extensions
retain their adopted bytes. The engine result is a run-local file, not a fifth
committed artifact. No push is authorized or performed.


## Unit 2 — chief design synthesis, 2026-09-23

Run `build-decision-0065-slice-one-re-9f0b932b`, phase `design`, adopted
`d98163740e24839deb5ddccd1a9e5d563f804b18` on `slice-0065-capabilities`.
The working tree began clean. Every prior commit is retained; no replay,
implementation, task closure, new council or push occurs in this phase.
The run context has no `returned_from`.

Read the operator ruling/addendum, Rebuild units preamble/unit 2, task 2.1,
framing and its SC7/SCM/SC8/RGR requirements, README and decisions
0004/0005/0009/0065, the relevant 0046 ruling, 0063 and proposed 0066.
Read `dialects/openspec.json`, its design/return instructions, the current
proposal and rendered `openspec instructions design --change
 decision-0065-capabilities-slice-one`. No workflow runner was invoked.
Read both current positions in full, robustness.md and simplicity.md under
`.forge/design/positions/`, and checked their claims against the three named
production files, both owning suites, the existing Codex/Claude data and
read-only dispatch/grammar seams. Those run-local reads are design evidence;
new tests may not read .forge/.

### Decisions and amended artifacts

Design D5.1 explicitly adopts, combines or rejects the current positions'
claims. It preserves the earlier whole-rebuild council table as historical,
rather than misattributing it to the replacement unit 2 position files.
D5.2 chooses one strict decoder, independent field inheritance, a retained
Agent.allow plus typed sandbox, and a checked local value in SiteFacts.
Source search found Agent's only construction in agents/load.rs; existing
out-of-scope SiteFacts literals use Default, avoiding a Candidate or test-file
expansion merely to add these facts.

D5.3 answers both robustness questions: sandbox admission is bounded to
matching existing Codex engine fragments under the existing hands/boundary
law; any new inline/direct-empty/sandbox path without representation refuses
runnable compilation until lowering/transport exists. No unsupported class is
silently dropped or clamped. Pure decoding/narrowing still proves all three
classes. The existing hands replacement rule wins over a blanket demand for
direct tool_permissions support: syntax/narrowing and mapped native-alias
checks remain, but a dormant list is not broker policy. Evidence is the two
named hands regressions and the current composition branch; no new live
provider semantics are claimed.

In dependency order, amended design.md, the owning seat-capability-resolution
scenarios/Decisions, task 2.1's notes, and this evidence. The added scenarios
bind the intermediate refusal, matching sandbox fragments, unchanged hands,
shared-office isolation and whole-chain compatibility. The proposal, other
five deltas, accepted Rebuild units order and proposed decision 0066 remain
coherent without edits. This visit does not change grants, production, tests,
pins, contracts, policy, fixtures, reference or extensions.

### Observed checks and remaining proof

- `openspec validate --all --strict --no-interactive`: **passed, 18/18**.
  Existing long-requirement and unrelated archive-target informational notices
  remain; there were no failed items.
- `git diff --check`: **passed** after the design/scenario/task changes.
- Each of `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`,
  `cargo test -p brokkr-runtime --all-features --locked`,
  `cargo test --workspace`, and
  `cargo run --locked -p brokkr-cli -- compile --bundle bundles/self` was
  attempted and **unavailable**, shell command-not-found (127). Cargo does not
  resolve in this chief seat; the positions' availability observations belong
  to those seats and do not establish availability here.
- No Rust test was added, no baseline red or mutation was executed, and no
  implementation/removal/restoration proof is claimed. D5.4 and task 2.1 retain
  every such obligation in the two authorized suites.
- External exact coverage, macOS and remote CI remain **pending**. No new host
  result was observed, and the unit 1 obligations/security hold remain open.

Only design.md, specs/seat-capability-resolution/spec.md, tasks.md and
evidence.md belong in this design commit. The mandatory chief result is a
separate gitignored run-local record with inputs.change. Final committed-head
OpenSpec/diff checks and scope verification are recorded there after commit;
these document checks cannot establish a fully green implementation.


## Unit 2 — executable task breakdown, 2026-09-23

Run `build-decision-0065-slice-one-re-9f0b932b`, phase `tasks`, based on
`4a6a2288e2825c29e23ec9515e66613fbf53462c` on `slice-0065-capabilities`.
The worktree began clean. All preceding commits, including replay, unit 1b,
unit 2 specification and chief design, remain adopted. The supplied run
context has no `returned_from`. This visit changes only tasks.md and this
record; it adds no Rust tests, implementation, task completion or gate claim.

Read the operator ruling/addendum before Rebuild units and unit 2, then the
current task, proposal, D5.1–D5.4 and Open Questions, owning SCM/SC7/SC8/RGR
requirements and related hands/native-denial requirements. Checked the named
production files and owning fixture/suite paths against that design, and read
README and decisions 0004/0005/0009. Read dialects/openspec.json and its tasks
and return instructions, and rendered `openspec instructions tasks --change
 decision-0065-capabilities-slice-one`. No workflow runner, provider, council,
replay or delegation was invoked.

### Breakdown and requirement coverage

Task 2.1 remains the accepted closure anchor (previous 3.14). Its seven new
numbered checkbox substeps execute in dependency order inside the same unit;
they do not commission seven additional visits. Every checkbox names and
links its requirements. All eight entries stay unchecked. Every task and
status outside group 2 is byte-identical to the adopted head.

| Ordered work | Requirement/scenario coverage |
| --- | --- |
| 2.1.1, baseline and canonical fixtures | SC8 full refusal/mutation proof and canonical-root scenario; SCM's distinct empty, sandbox and inline baseline behaviors. |
| 2.1.2, shared decoding and pure narrowing | SCM strict decoding, malformed tools, field omission/empty and local widening scenarios; SC7 empty/nonempty MCP migration; SC8 exact values/causes and original-source duplicates. |
| 2.1.3, effective clone and candidate compatibility | SCM unchanged hands, unavailable later candidate and ordered restrictions; SC7 native alias refusal; TD6 unchanged workspace authority; SC8 independent mapping/fallback proofs. |
| 2.1.4, executable traversal and checked facts | SCM each executable/container, inherited/selected/default body and shared-office isolation scenarios; strict original source keys, fallible adapter context and wrapper relocation; SC8 full site/source causes. |
| 2.1.5, existing representation or refusal | SCM all D5.3 rows, unsupported intermediate lowering, exact Codex fragments and independent boundary authority; RGR typed authority only; TD6 unchanged hands; NCR unchanged native OFF; SC8 independent admission/refusal assertions. |
| 2.1.6, per-test/per-row proof audit | SC8 baseline/fix/compiling mutation/restoration record, including tests already passing baseline; SCM/SC7 owning scenarios. A first table failure proves no subsequent row. |
| 2.1.7, gates and committed evidence | SC8 actual results and evidence limits; MP5 measured pins/history and split before out-of-scope pin work; SCM bounded unit completion. |

The source check agrees with the chief's design: `parse_tools` currently
returns only optional allow and rejects empty; `report_under` clones the
agent before composition; `resolve_report` checks every entry's gap;
`SiteFacts` has no local restriction field; `needs_adapters` does not yet
account for typed tools. `AgentFixture` currently derives paths from a
lexical TempDir while agents' `Tree` already retains a canonical root.
These are observations of source, not executed behavioral reds.

No earlier artifact needs changing to write this breakdown. D5.3 and its
owning scenarios already settle the intermediate representation question:
keep runnable restrictions refused until they can be delivered, with bounded
matching existing Codex/hands fragments as specified. Unit 9's nonempty
capability-restriction transport is a later dependency, not a new assumption
for local tools decoding. Unit 1's external gates stay pending. No design
choice, scenario, proposed decision or accepted unit order is amended here.
The three production/two suite ceiling and stop-before-split rule are explicit;
lowering/origin transport, migration, final launch and archive remain deferred
to their existing owners.

### Observed validation and limits

- `openspec validate --all --strict --no-interactive`: **passed, 18/18**.
  Existing long-requirement notices and unrelated archive-target notices for
  adapter-resume-safety / sdd-progress-markers remain informational.
- Read-only `openspec instructions apply --change
  decision-0065-capabilities-slice-one --json`: its task parser recognizes
  2.1 and 2.1.1–2.1.7 as eight separate unchecked entries. No apply workflow
  or implementation was run.
- A read-only requirement/link audit verified every unit 2 checkbox cites
  existing requirement titles and matching anchors, and verified task bytes
  outside group 2 are unchanged. `git diff --check`: **passed**.
- Each of the seven Cargo commands listed in task 2.1.7 was attempted here:
  format, workspace clippy, runtime suite, both workspace suites and self/verify
  bundle compilation. Every command exited **127**, `cargo: command not found`.
  These checks are **unavailable**, not passed. The task draft remains usable;
  an implementation seat with Cargo must execute and record all of them.
- No new Rust test, baseline failure, compiling mutation or restored pass was
  produced by this document-only phase. Those remain explicit implementation
  obligations, not task-draft completion evidence.
- External exact coverage, macOS and remote CI remain **pending**. No new host
  or remote result was observed and no fully green claim is made.

The document commit contains only tasks.md and evidence.md. Production, tests,
pins, grants, frozen contracts, policy/schemas, policy/phase-machine.json,
fixtures, reference and extensions retain the adopted bytes. The engine result
is a separate gitignored run-local JSON record with result `drafted` and
inputs.change; its notes record the committed SHA and final document checks.
No push or archive occurs.


## Unit 2 — implementation, 2026-09-23

Run `build-decision-0065-slice-one-re-9f0b932b`, phase `implement`, based on
`308a8a28e5bf7f753c44ab389e249b4e917d0b61` on `slice-0065-capabilities`. The
worktree began clean; every preceding commit is adopted and nothing was
replayed. The run context has no `returned_from`. This visit executes unit 2
alone: D5 typed `tools.allow` / `tools.sandbox` decoding and strict local
admission in the three named production files, proved in the two owning
suites. Nothing later in the Rebuild units order is touched.

### Production, inside the allowlist

- `crates/brokkr-runtime/src/agents.rs`: the three-case `Sandbox` enum with
  an explicit `reach` (read-only < workspace-write < danger-full-access,
  never lexical); the two-field `LocalTools` value with `narrow`, which
  inherits per field, keeps written order, refuses an addition by name and a
  widening by class, and never clamps; `Agent.sandbox` beside the retained
  `Agent.allow`, assembled by `Agent::local`; `decode_local_tools`, the narrow
  exposure of the loader's decoder for bundle use; `ResolveError::LocalTools`;
  `report_narrowed`, which applies a site's declaration to a PRIVATE clone
  before composition (`report_under` keeps its five-argument signature, so
  `engine/boundary_tests.rs`, outside the allowlist, did not move); in
  `compose`, the explicit-empty allow set is refused with the D5.3
  unsupported-representation cause, and the native-alias refusal is one
  closure applied both on the direct path and beside hands.
- `crates/brokkr-runtime/src/agents/load.rs`: `parse_tools` returns
  `LocalTools` and is `pub(crate)`; vocabulary `allow`, `sandbox`, `mcp`;
  presence before type; explicit `[]` is a value; duplicates refuse; the
  sandbox word decodes exactly or refuses with the vocabulary; the strict
  reader `read_request_source` is unchanged.
- `crates/brokkr-runtime/src/bundle.rs`: `SiteFacts.local`; `needs_adapters`
  opens the adapters for a `tools` key (message extended); `tools` admitted in
  SEAT/BODY/MEMBER/STEP key lists; `decode_site_tools`,
  `refuse_tools_on_container`, `record_inline_tools`, `expressed_sandbox` (the
  public codex grammar, exactly one `--sandbox`, the `sandbox_mode`
  configuration door refused) and `admit_local_sandbox` (D5.3 rows);
  `resolve_reference` narrows through `report_narrowed` and stores the
  effective value; every inline executable branch (seat loop, selected body,
  panel member, sequence step, dialect validator) records a checked value or
  refuses a nonempty field; containers refuse the key; a dialect step with no
  supplied check refuses the key; `enforce_model_policy` is now hands law →
  `enforce_route_policy` (the unchanged remainder, extracted) → admission
  last.

No new module, dependency, `Candidate` field, public contract, shipped JSON,
grant, pin or frozen byte. `git status` lists exactly the five allowed Rust
files plus tasks.md and this file.

### Baseline observed before repair

- At `308a8a28` the loader table test
  `the_library_loader_names_the_file_and_the_key_it_refuses` passed (1
  passed, 506 filtered). After the production change and before any test
  edit, the runtime lib suite ran 506 passed, 1 failed: that test panicked at
  `agents/tests.rs:65:45` (`library_error` `unwrap_err` on `Ok`) on its
  `{"allow": []}` row — explicit empty now decodes instead of refusing. The
  row was replaced by allow-null, duplicate and unknown-sandbox rows.
- Source observations at the adopted head, as the specification visit
  recorded: no sandbox field existed and no key list admitted `tools`, so the
  positive declarations had no baseline to run against. Each new test is
  bound by an independent compiling mutation instead (ledger below); no
  compiler error is counted as proof anywhere.

### New and changed tests

`agents/tests.rs` (7 new; one table amended):
`typed_tools_decode_exactly_and_keep_empty_distinct_from_omission`,
`typed_tools_decoding_refuses_each_malformed_field_with_its_full_cause`,
`repeated_tools_keys_refuse_from_the_original_source_even_when_equal`,
`narrowing_inherits_per_field_and_refuses_each_widening_exactly`,
`report_narrowed_composes_from_a_private_clone_and_leaves_the_office_untouched`,
`an_explicit_empty_allow_set_is_kept_and_refused_until_lowering_delivers_it`,
`hands_keep_their_replacement_and_still_refuse_a_mapped_native_alias`.

`bundle/agent_tests.rs` (9 new): `AgentFixture` now canonicalises its
temporary root once and derives every path from it, and gains
`compile_under` / `compile_with_policy`;
`an_agent_backed_seat_narrows_its_office_per_field_and_records_the_effective_value`,
`an_inline_site_records_a_checked_empty_declaration_and_refuses_each_nonempty_field`,
`a_typed_declaration_needs_the_adapter_context_and_a_missing_one_is_named`,
`repeated_tools_keys_in_a_bundle_refuse_from_the_original_source`,
`tools_beside_a_container_refuse_at_every_container_form`,
`every_executable_form_owns_its_local_declaration` (ordinary seat, panel
member, sequence step, selected case, selected default, inherited base
layer),
`two_sites_sharing_one_office_keep_their_own_effective_fields_in_either_order`,
`a_typed_sandbox_admits_only_where_an_existing_codex_fragment_expresses_it_exactly`
(boxed read-only; harness gate read-only; harness work workspace-write; each
other class per row; open work; open gate precedence; missing gate fragment
precedence; classless fragment; unreadable fragment; configuration door;
competing authored control; provider labelled codex dispatching another
harness; claude with hands; later candidate; unchanged holdings, native OFF,
hands and boundary),
`a_seat_narrows_a_boxed_office_and_admission_judges_the_effective_class`,
`a_dialect_step_owns_only_a_checked_empty_declaration`.

Every assertion is a complete `assert_eq!` on the effective value, the
composed argv, the selected fragment or the full diagnostic; no `is_err()`
and no substring stands as a commissioned proof. Fixtures live under
`tempfile` roots; the dialect test reads the repository's shipped
`dialects/openspec.json`, `agents/` and `adapters/` exactly as the existing
generated-validator test does. No test reads `.forge/`, discovers a provider
or starts a model.

### Mutation ledger

Each mutation is a compiling edit inside the three production files, applied
alone against its target tests (batches group mutations whose targets are
disjoint), observed, then reverted. After every batch the marker grep over
the three files was empty and the suites were green again.

| Batch | Mutation (file, change) | Test | Observed failure |
| --- | --- | --- | --- |
| A | load.rs: duplicate-name find guarded by `name.is_empty()` | `typed_tools_decoding_refuses_each_malformed_field_with_its_full_cause`; loader table | panicked `agents/tests.rs:65:45` (`unwrap_err` on `Ok`) at the `["cargo","git","cargo"]` row; earlier rows are untouched by the edit |
| A | agents.rs `narrow`: subset find guarded by `name.is_empty()` | `narrowing_inherits_per_field_and_refuses_each_widening_exactly` | `tests.rs:3095` left `Ok(LocalTools { allow: Some(["cargo", "make"]), sandbox: Some(WorkspaceWrite) })`, right `Err(("allow", "names 'make', which the office's 'tools.allow' [\"cargo\", \"git\"] does not; …"))` |
| A | agents.rs `compose`: explicit-empty guard conjoined with a false term | `an_explicit_empty_allow_set_is_kept_and_refused_until_lowering_delivers_it` | panicked `tests.rs:123:6` (`refusal` `unwrap_err` on `Ok`): composition produced a command |
| A | bundle.rs `needs_adapters`: key `tools-never` | `a_typed_declaration_needs_the_adapter_context_and_a_missing_one_is_named` | panicked `agent_tests.rs:9:18` "expected compilation to fail" |
| A | bundle.rs `refuse_tools_on_container`: reads `tools-never` | `tools_beside_a_container_refuse_at_every_container_form` | panicked `agent_tests.rs:9:18` |
| A | bundle.rs dialect no-check refusal: reads `tools-never` | `a_dialect_step_owns_only_a_checked_empty_declaration` | panicked `agent_tests.rs:9:18` at the `clarify:check` assertion |
| B | agents.rs `Sandbox::parse`: `"loose"` maps to `ReadOnly` | decoding table (loose rows); loader table | panicked `tests.rs:65:45` at the first `loose` row |
| B | agents.rs `reach`: `WorkspaceWrite => 3` | `narrowing_inherits_per_field_and_refuses_each_widening_exactly` | `tests.rs:3104` left `Ok(… sandbox: Some(DangerFullAccess))`, right `Err(("sandbox", "requests 'danger-full-access', which reaches wider than the office's 'workspace-write'; …"))` |
| B | agents.rs hands branch: alias lookup filtered to empty names | `hands_keep_their_replacement_and_still_refuse_a_mapped_native_alias` | panicked `tests.rs:123:6`: the fallback link composed instead of refusing |
| B | bundle.rs `record_inline_tools`: presence conditions falsified | `an_inline_site_records_a_checked_empty_declaration_and_refuses_each_nonempty_field` | panicked `agent_tests.rs:9:18` at the `allow: []` row |
| B | bundle.rs `resolve_reference`: local stored only when unspecified | `an_agent_backed_seat_narrows_its_office_per_field_and_records_the_effective_value`; `two_sites_sharing_one_office_keep_their_own_effective_fields_in_either_order` | `agent_tests.rs:1065` left `None`, right `Some(LocalTools { allow: Some(["cargo", "git"]), sandbox: None })`; `agent_tests.rs:1483` left `None`, right `Some(LocalTools { allow: Some(["git"]), sandbox: None })` |
| C | load.rs: explicit `[]` returned as unspecified | `typed_tools_decode_exactly_and_keep_empty_distinct_from_omission` | `tests.rs:2886` left `None`, right `Some([])` (this edit also failed `every_executable_form_owns_its_local_declaration` at its ordinary-seat empty row — a cross-effect, so that test is bound separately in batch D) |
| C | agents.rs `narrow`: subset arm replaced by an office-ordered intersection | `narrowing_inherits_per_field_and_refuses_each_widening_exactly` | `tests.rs:3093` left `Ok(… allow: Some(["cargo", "git"]) …)`, right `Ok(… allow: Some(["git", "cargo"]) …)` |
| C | bundle.rs `admit_local_sandbox`: authored competing check filtered away | `a_typed_sandbox_admits_only_where_an_existing_codex_fragment_expresses_it_exactly` | panicked `agent_tests.rs:9:18` at the competing-control row; every earlier row is untouched by the edit |
| C | load.rs `parse_agent`: source read through the non-strict `read_json` | `repeated_tools_keys_refuse_from_the_original_source_even_when_equal` | panicked `tests.rs:65:45`: the repeated key loaded |
| D | bundle.rs `MEMBER_KEYS` without `tools` | `every_executable_form_owns_its_local_declaration` | panicked `agent_tests.rs:1396:58` (`unwrap` on the panel-member compile, unknown key `tools`) after the ordinary-seat form passed |
| D | bundle.rs no-hands guard conjoined with `class.is_empty()` | `an_agent_backed_seat_narrows_its_office_per_field_and_records_the_effective_value` | `agent_tests.rs:1116` left `"bundle: seat 'work' link 1 requests 'tools.sandbox' 'read-only' but dispatches the 'claude' harness …"`, right `"bundle: seat 'work' requests 'tools.sandbox' 'read-only' without hands; …"` |
| D | bundle.rs fragment match accepts any nonempty class | `a_typed_sandbox_admits_only_where_an_existing_codex_fragment_expresses_it_exactly`; `a_seat_narrows_a_boxed_office_and_admission_judges_the_effective_class` | both panicked `agent_tests.rs:9:18`: the boxed workspace-write row, and the read-only-at-harness-work row, compiled |
| E | agents.rs `report_narrowed`: effective allow not applied to the clone | `report_narrowed_composes_from_a_private_clone_and_leaves_the_office_untouched` | `tests.rs:3190` left `LocalTools { allow: Some(["cargo", "git"]), sandbox: None }`, right `LocalTools { allow: Some(["git"]), sandbox: None }` |
| E | bundle.rs admission judges only the first candidate | `a_typed_sandbox_admits_only_where_an_existing_codex_fragment_expresses_it_exactly` | panicked `agent_tests.rs:9:18` at the later-candidate row |
| F | bundle.rs `open` routed through the harness fragments | same | `agent_tests.rs:1603` left `"… the \`hands.harness.work\` fragment … under the \`open\` boundary expresses 'workspace-write' …"`, right `"… under the \`open\` boundary, where a work seat runs at the harness's own default …"` |
| G | bundle.rs configuration door keyed `sandbox_mode_never` | same | panicked `agent_tests.rs:9:18` at the configuration-door row |
| H | bundle.rs admission inserted BEFORE the hands law | same | **passed** (1 passed). Finding: an agent-backed site with hands is judged by the resolver's D33 pre-check before its local fact is stored, so the standing refusals precede admission whatever its position inside `enforce_model_policy`. |
| H | the same plus the local fact stored before the D33 pre-check | same | `agent_tests.rs:1610` left the open-work admission refusal, right `"bundle: seat 'work' is a gate with hands under the \`open\` boundary, … \`open\` never holds a model gate (decision 0046 ruling 4)"` |

The precedence proof therefore rests on two retained facts: the D33
pre-check runs before the local fact exists, and admission is the last
statement of `enforce_model_policy`. Both stand in the delivered code.

Honest limits of the ledger:

- `repeated_tools_keys_in_a_bundle_refuse_from_the_original_source` passes
  at baseline: its reader is `bundle/compose.rs::read_layers`, outside the
  allowlist, and no in-scope caller can weaken it, so it carries no mutation.
  The agent-side reader was mutated (batch C) and is bound.
- The canonical-root habit in `AgentFixture` is by construction on this
  Linux host, whose temporary root is not an alias; macOS is not observed.
- Where a mutation silences a refusal, the failure is the test's refusal
  helper panicking on `Ok` (`agents/tests.rs:65:45`, `:123:6`,
  `bundle/agent_tests.rs:9:18`); the row reached is the first row the edit
  can affect, named above. Where the mutation changes a value, the exact
  left/right is recorded.

### Gates on the restored tree

All on `308a8a28` plus this working tree, after every mutation was reverted
(marker grep over the three files: empty).

- `cargo fmt --all -- --check`: **passed**, after `cargo fmt --all`
  reformatted only the five touched files.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D
  warnings`: **passed**, after two test-only findings were repaired (a type
  alias for the forms table; `contains_key`).
- `cargo test -p brokkr-runtime --all-features --locked`: **passed** — 524
  lib tests and every integration binary.
- `cargo test --workspace --all-features --locked`: **passed** — 77 green
  result lines, no failure (`.forge/ws-test-all-features.log`).
- `cargo test --workspace`: **passed** — 77 green result lines, no failure.
- `cargo run --locked -p brokkr-cli -- compile --bundle bundles/self` and
  `… bundles/verify`: **passed**, each printing its manifest.
- `openspec validate --all --strict --no-interactive`: **passed, 18/18**
  before the document edits; rerun after them and recorded in the run-local
  result.
- `git diff --check`: **passed**.
- External exact coverage (`scripts/coverage-exact.sh`), macOS and remote
  CI: **pending**, not observed here; nothing is called fully green on their
  account, and unit 1's pending results stay pending.

What remains open, by owner: lowering and origin transport for the typed
values (units 3–4), the shipped migrations (units 6–8), authored-flag refusal
(unit 12), and the final-command proofs (units 13–21). Unit 2 accepts no
typed restriction it cannot yet deliver: inline direct lists, direct explicit
empty and every non-matching sandbox class refuse compilation with the
owning site, field and cause.


## Unit 2 — specification re-adoption and proof correction, 2026-09-23

Run `build-decision-0065-slice-one-re-e332dc8d`, phase `specify`, adopted head
`f9a691cf7c10aaaeccc1f8c5f9a3e6f9af2566c8` on `slice-0065-capabilities`.
The worktree began clean. `git merge-base --is-ancestor <commit> HEAD`
returned 0 for each of `d9816374`, `4a6a2288`, `308a8a28` and `f9a691cf`.
Every commit remains adopted; none was replayed. The supplied run has no
`returned_from`. The operator's explanation of the previous review's host
configuration failure is retained; it is neither a review finding nor a
review pass.

Read the operator ruling/addendum first, then the accepted Rebuild units
preamble and unit 2, task 2.1 and its owning deltas. Read README, decisions
0004/0005/0009/0065, proposed 0066, `dialects/openspec.json` and its own
`openspec/specify.md` / `openspec/return.md` instructions through the workspace
hands. Adopted the proposal before the deltas; their requirements and
scenarios already express D5's decoding, independent inheritance, exact
narrowing, existing authority and refusal until representation exists. No
specification ambiguity or new design decision was found, so their Decisions
and the accepted design remain unchanged. No council was reconvened and no
workflow runner was invoked.

The source diff from `308a8a28` to `f9a691cf` stays within the three named
production files and two owning suites, plus tasks/evidence. Static inspection
confirmed the shared decoder, private office clone, site facts and bounded
D5.3 admission paths. This is not a new runtime or final-launch proof.

### Corrections to the completion claim

The implementation and existing positive evidence remain adopted. The blanket
claim that task 2.1's proofs are complete is not supported by the recorded
artifacts:

- **Baseline history (2.1.1):** the implementation section says the new tests
  lacked runnable baselines and substitutes source observations. Its only
  recorded red is an old empty-list-rejection assertion failing *after* the
  decoder changed. That is not the commissioned positive behavior failing
  before repair. Preserve that history; do not relabel it. Any later test of
  retained revision `308a8a28` must be explicitly recorded as retrospective,
  not as an experiment that preceded implementation.
- **Mutation completeness (2.1.2–2.1.6):** the ledger explicitly records no
  mutation for
  `repeated_tools_keys_in_a_bundle_refuse_from_the_original_source`.
  Its explanation that the strict reader lives outside the allowlist does
  not satisfy the required compiling proof or authorize another file.
  Investigate the permitted caller/test seam; if no valid in-scope proof
  exists, inventory a split before proceeding. Table-wide claims also need
  independent row evidence: batch B stops the invalid decoder table at its
  first `loose` row; batch B's inline-field mutation stops at `allow: []`;
  batch D's executable-form mutation stops at the panel member. These do not
  establish the later sibling, sandbox or executable-form assertions. Keep
  the existing failures, then supply only the missing independent proofs and
  their restored passes.
- **Full causes (2.1.2/2.1.6):**
  `narrowing_inherits_per_field_and_refuses_each_widening_exactly` compares
  only `.unwrap_err().0` for the valid-sibling/invalid-field cases and the
  refusing sandbox-pair rows (`agents/tests.rs`, lines 3120–3170 at the
  adopted head). Those assertions omit the cause. The evidence statement
  that every assertion checks a complete value or diagnostic is too broad;
  these rows need exact complete errors and independent mutation evidence.
- **Test count:** comparing `#[test]` functions at `308a8a28` and `f9a691cf`
  gives 7 new agent tests and **10** new bundle tests, **17 total**. The
  historical “9 new” bundle count and commit message's “Sixteen new tests”
  are superseded by this measured count; history is not rewritten.

Task 2.1 and the proof portions of 2.1.1–2.1.6 reopen. This does not order a
new implementation, specification or design, nor defer the missing proof to
unit 25. The existing owning tasks already require it. Task 2.1.7 retains the
prior visit's recorded local gate results with their original scope. No
missing proof is waived, no new test is claimed, and no failing or restored
Rust execution is invented in this specify visit.

### Validation and scope of this visit

- `openspec validate --all --strict --no-interactive`: **passed, 18/18**.
  Existing informational archive/length advisories do not fail validation;
  no archive was attempted.
- `git diff --check`: **passed**.
- Fresh `cargo fmt --all -- --check`, locked all-target/all-feature clippy,
  runtime all-feature tests, both workspace test commands, and self/verify
  bundle compiles: **unavailable**. Initial shell attempts at format, clippy
  and runtime tests returned `cargo: command not found` (127 for the latter
  two standalone calls). Explicit subprocess attempts for all seven commands
  then confirmed no cargo executable; none ran. The PATH cargo directory is
  absent in this box. Prior passes remain historical, not fresh results.
- External exact coverage, macOS and remote CI: **pending**, unchanged. No
  coverage counts or fully green claim is made. No host-shell fallback,
  provider call or frozen-file edit was used.
- Only tasks.md and evidence.md change in this visit, to correct observed
  completion claims as the commission requires. Proposal, all six deltas,
  design, Rust, shipped data, measured pins and frozen bytes are unchanged.
  Decision 0066 remains proposed. No push or archive.

The final commit and post-commit OpenSpec/diff results are recorded in this
run's result file so recording them does not change the validated head.

## Unit 2 — rerun chief council disposition, 2026-09-23

Run `build-decision-0065-slice-one-re-e332dc8d`, phase `design`, adopted head
`368bc34e1370f35407e4e8b0ef35c7b374b7acdb`, `slice-0065-capabilities`.
The worktree began clean. The four commissioned commits and the proof
correction remain adopted; no replay or replacement specification/design.
The supplied context has no `returned_from`. The failed review startup is
neither a behavioral finding nor a completed review.

Read the operator ruling/addendum before the Rebuild units preamble/unit 2,
then D5, tasks 2.1.1–2.1.7, the owning SC7/SCM/SC8 scenarios and both current
positions in full. Also read the proposal, relevant implementation and owning
tests, historical proof correction, README and decisions 0004/0005/0009/0063/
0065/proposed 0066. Read `dialects/openspec.json` and its `openspec/design.md`
and `openspec/return.md` through the workspace tool, plus the read-only
`openspec instructions design --change decision-0065-capabilities-slice-one
--json` output. No dialect workflow runner, provider or archive was invoked.

### Evidence and disposition

D5.5 explicitly adopts or combines every current position's claim, and rejects
the unsupported alternatives with reasons. D5.1 is labelled historical because
its run-local position paths now hold the rerun positions. The design's context
now describes the adopted implementation, rather than its pre-implementation
state. D5.3 makes its existing competing-control check explicit in both the
selected fragment and authored contribution. Tasks 2.1.5/2.1.7 reflect the
repair and fresh-gate obligations; all unit 2 boxes remain open. Rebuild units,
proposal and deltas remain unchanged: SCM's existing exact-fragment scenario
already forbids competing controls, so no ambiguity or upstream defect was
found. Decision 0066 stays proposed.

The robustness seat's `.forge/design/unit2-robustness-probe.json` records the
following at `368bc34e`; these are attributed compile observations, not a new
chief execution, live-provider experiment or regression/removal proof:

| Probe row | Recorded outcome |
| --- | --- |
| Matching boxed Codex hands fragment | Admitted with exact ReadOnly local fact. |
| Authored sandbox bypass | Admitted with ReadOnly and bypass in candidate argv. |
| Selected hands.workspace sandbox bypass | Admitted with matching sandbox and bypass in the engine fragment. |
| Authored full-auto | Admitted with ReadOnly. |
| Authored sandbox_workspace_write.network_access=true | Admitted with ReadOnly. |
| Authored --sandbox danger-full-access | Refused with the full existing competing-control cause. |

Chief source inspection corroborates the mechanism: expressed_sandbox only
extracts `--sandbox` and rejects config under `sandbox_mode`; the grammar
recognizes the two switches, and both contributions call this same incomplete
check. The existing sandbox regression covers `sandbox_mode` and authored
`--sandbox`, not these admitted neighbors. Reject deferral to unit 12 or a
claim that the independent namespace makes typed admission correct. Accept
simplicity's conditional zero-production-diff advice only where no defect is
demonstrated; this defect requires the narrow bundle.rs repair it allows.
No provider precedence or runtime escape is inferred.

The full-cause narrowing gap is also confirmed in the source: valid-sibling
cases and refusing sandbox pairs use `.unwrap_err().0`. The historical ledger
and 368bc34e establish the missing original reds, incomplete independent row
mutations and unbound strict-bundle-source test. Keep those limitations and
the corrected 17-test count. No new test or mutation is authored in this
design phase; implementation must produce the required baseline/red/removal/
restoration evidence in the two owning suites, or inventory an in-scope-seam
failure before requesting a split. A diagnostic is not a replacement suite.

### Current-visit validation

On `368bc34e` plus this documentation-only working tree:

- `openspec validate --all --strict --no-interactive`: **passed, 18/18**.
  Informational length/archive advisories do not fail validation; no archive
  was attempted.
- `git diff --check`: **passed**.
- `git merge-base --is-ancestor <commit> HEAD`: **passed** independently for
  `d9816374`, `4a6a2288`, `308a8a28`, `f9a691cf` and `368bc34e`. The Rebuild
  units suffix is byte-for-byte equal to the adopted version.
- All seven Cargo commands in task 2.1.7 were attempted explicitly:
  formatting, locked all-target/all-feature clippy, runtime all-feature tests,
  both workspace test commands and self/verify bundle compiles. **Unavailable**:
  subprocess could not find `cargo` in this seat's workspace box. None ran;
  no test/compiler version is claimed. The robustness seat's Cargo 1.98.0
  observation remains attributed to that seat, not relabelled as a fresh
  result. Logs and command/status records are under
  `.forge/design/chief-unit2-e332dc8d-gates/`.
- External exact coverage, macOS and remote CI remain **pending**. No fully
  green or task-closure claim, host-shell workaround or gate reduction.

Only design.md, tasks.md and evidence.md are authored for this phase, in that
order. Production, tests, measured pins, shipped data and frozen bytes do not
change. The final commit and post-commit OpenSpec/diff/scope results are
recorded in this run's result file so that recording them does not move the
validated documentation head. No push.
