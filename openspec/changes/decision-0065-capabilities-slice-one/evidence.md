# Decision 0065, slice one — evidence after the operator ruling

## Current status — unit 2 second review return, 2026-09-23

The second review return on `4a441bf7` (S1 `--add-dir`, P1–P3 proof gaps)
is answered; see “Unit 2 — second review return” at the end of this file.
Task 2.1 and its substeps stay ticked on the stated basis: fresh local
gates on the restored tree, with external exact coverage, macOS and remote
CI pending. Earlier sections retain their dates and scope.

## Historical status — unit 2 council design, 2026-09-23

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


## Unit 2 — tasks re-adoption and remaining repair order, 2026-09-23

Run `build-decision-0065-slice-one-re-e332dc8d`, phase `tasks`, adopted head
`107c4d179fc0a5807ade3d4f3628d8c8ec77a7c5`, branch
`slice-0065-capabilities`. The worktree began clean. All four commissioned
commits (`d9816374`, `4a6a2288`, `308a8a28`, `f9a691cf`), proof correction
`368bc34e` and council amendment `107c4d17` remain ancestors. No replay,
replacement specification/design or implementation occurred. The supplied
context has no `returned_from`; the failed review startup is not proof of
correctness or a new behavioral finding.

Read the operator ruling/addendum first, then the Rebuild units preamble and
unit 2, current tasks, D5.2–D5.5, SCM/SC8 scenarios, historical unit 2 evidence,
and the relevant production/test seams. Read README and decisions 0004/0005/
0009, decision 0065's standing authority context, and the installed dialect's
`openspec/tasks.md` and `openspec/return.md`. No provider, workflow runner,
delegation, archive or host-shell fallback was used.

### Observed gaps and executable order

The adopted implementation is present. `expressed_sandbox` uses the existing
public grammar but checks only `--sandbox` and configuration under
`sandbox_mode`; its loop does not judge the two recognized switches or
`sandbox_workspace_write`. This source observation corroborates D5.5 and the
previous council's attributed probes; it is not a fresh compile or live
provider experiment. The narrowing test still has field-only
`.unwrap_err().0` assertions. The strict raw-bundle test reaches
`compose::resolve(&dir)?` in the allowed bundle.rs caller; no compiling
mutation at that seam was executed or claimed in this tasks phase.

The previous wording still commissioned initial decoding/wiring and called
pre-f9a691cf behavior today's baseline. Group 2 now retains the implementation
and sufficient existing experiments, names the actual remaining work, and
keeps missing original baseline reds historical. A retrospective run cannot
be relabelled as the original observation. All eight unit 2 boxes remain open;
all task IDs, previous-ID traceability and tasks outside group 2 are retained.

| Ordered task | Remaining work and requirement coverage |
| --- | --- |
| 2.1.1 | Inventory the 17 new tests (7 agent, 10 bundle) plus amended loader rows against retained/missing proof; record current baselines and canonical fixture facts. SCM strict local values and SC8 truthful baseline/removal/canonical-root evidence. |
| 2.1.2 | Retain decoding/narrowing, replace partial errors with complete independent causes, and bind remaining malformed/duplicate/inheritance/order/class rows. SCM strict values, malformed tools and field-wise narrowing; SC7 no grandfathering; SC8 full reasons. |
| 2.1.3 | Retain private-clone composition, whole-chain validation, exact mappings and hands replacement; supply only missing independent proofs. SCM hands/later-candidate scenarios, SC7 native aliases, TD6 unchanged hands authority and SC8 exact values/causes. |
| 2.1.4 | Bind missing executable/container/context/shared-office/wrapper rows and each strict raw-bundle duplicate case through an allowed caller or existing test seam. SCM site ownership and original-source strictness, SC7 no silent authority and SC8 independent rows. An unworkable seam requires an inventoried split before widening. |
| 2.1.5 | Add exact regressions before the bundle.rs guard repair. Pair full-auto, sandbox bypass, sandbox_mode config and sandbox_workspace_write config independently with authored and selected engine contributions. Keep already-refusing rows labelled baseline passes; retain supported hands/effort and boxed/gate/work positives, precedence, holdings and OFF facts. SCM exact-fragment/unrepresentable scenarios, RGR realm-only authority, TD6 hands, NCR native denial and SC8 baseline/removal/restoration. |
| 2.1.6 | Audit every independently claimed row against retained or new compiling mutation, exact failure and restored pass; preserve missing historical reds as missing. SCM/SC7 coverage and SC8 evidence integrity. |
| 2.1.7 | Run restored local gates, record external status and commit only in-scope work. SC8 observed checks, MP5 truthful pins/history and SCM bounded completion. |

The paired sandbox cases use a matching requested class so a mismatch cannot
mask the competing control. The tasks require existing parsed option/config
identities, not another parser or blanket configuration refusal. Each missing
row must reach its own intended assertion; the first failure in a loop is not
proof for later rows. The raw-source mutation must target the enforcement,
not substitute an unrelated fixture or diagnostic failure. If that proof
cannot fit, the existing stop-and-split rule applies before another file moves.

No earlier artifact needs changing for an honest breakdown: D5.5 and SCM's
existing no-competing-control scenario already own the demonstrated gap.
Proposal, deltas, design/Rebuild units and proposed decision 0066 are unchanged.
The three production/two suite ceiling, later lowering/migration/final-launch
ownership and conditional final archive remain in force. This visit does not
claim unit 2 completion, final launch proof or fully green status.

### Observed validation and limits

On the adopted source plus these documentation edits:

- `openspec validate --all --strict --no-interactive`: **passed, 18/18**.
  Existing requirement-length and unrelated archive-target notices are
  informational; no archive was attempted.
- Requirement/link audit: **passed**. Every unit 2 checkbox cites existing
  requirement titles and anchors; all eight remain unchecked and the task
  bytes outside group 2 are unchanged.
- Read-only `openspec instructions apply --change
  decision-0065-capabilities-slice-one --json`: all eight unit 2 entries are
  recognized as unchecked tasks. No apply workflow was executed.
- `git merge-base --is-ancestor`: **passed** for each of the six adopted
  commits named above. `git diff --check`: **passed**.
- All seven Cargo commands in 2.1.7 were explicitly attempted: format,
  locked all-target/all-feature clippy, runtime all-feature suite, both
  workspace suites and self/verify compiles. Each subprocess failed to start
  because `cargo` is absent in this workspace box. **Unavailable**, not
  passed; no test execution, compiler version, new baseline red, mutation or
  restoration is claimed. Historical passes remain historical.
- External exact coverage, macOS and remote CI remain **pending**. Their
  absence does not require an earlier spec/design repair or prevent a task
  draft; it does prevent a fully green implementation claim.

Only tasks.md and evidence.md are changed and committed for this phase.
Production, tests, pins, shipped data, grants and frozen surfaces are unchanged.
The final SHA and post-commit checks go in the required run-local result file
so that recording them does not move the committed documentation head. No push.

## Unit 2 — proof repair and the D5.5 guard, 2026-09-23

Run `build-decision-0065-slice-one-re-e332dc8d`, phase `implement`, based on
`0444421d` on `slice-0065-capabilities`. The worktree began clean; the four
commissioned commits (`d9816374`, `4a6a2288`, `308a8a28`, `f9a691cf`), the
proof correction `368bc34e`, the council amendment `107c4d17` and the tasks
re-adoption `0444421d` are all ancestors and none was replayed. There is no
`returned_from`. This seat HAD cargo (1.98.0) and openspec (1.12.0); every
Rust result below is a fresh observation on this revision, not a historical
pass carried forward.

### Adoption check (2.1.1)

The adopted implementation was read against unit 2 and D5.2–D5.5 and kept
whole: the shared strict decoder, the sole `Agent.allow` beside the
three-case enum, the two-field local value, per-field narrowing on a private
clone, the checked site fact for every executable form, container refusal,
the fallible adapter context and the D5.3 admission rows. The one production
defect D5.5 named was confirmed at the source before any change:
`expressed_sandbox` extracted `--sandbox` and refused configuration under
`sandbox_mode` only; the grammar's two switches and `sandbox_workspace_write`
were parsed and ignored. Nothing else in the three files was found wrong.

Current baseline on `0444421d`, before any edit (`cargo test -p
brokkr-runtime --all-features --locked --lib -- agents::tests
bundle::agent_tests`): **96 passed, 0 failed** (65 agent, 31 bundle). The
missing original pre-`f9a691cf` reds stay missing; nothing below is a
reconstruction of them, and no `308a8a28` retrospective was run.

### Production, inside the allowlist

`crates/brokkr-runtime/src/bundle.rs`, `expressed_sandbox` only (task
2.1.5): two private constants name the codex switches
(`--full-auto`, `--dangerously-bypass-approvals-and-sandbox`) and the two
configuration tables (`sandbox_mode`, `sandbox_workspace_write`). A parsed
node whose canonical name is one of the switches refuses with a cause naming
the switch; a `Config` node whose key stands in or under either table refuses
with the existing door cause, now naming the table. The check is the same
loop the fragment and the authored contribution already went through, so
both are judged and neither provenance nor argument order is trusted. No
value is echoed. `agents.rs` and `agents/load.rs` are unchanged; no new
module, dependency, `Candidate` field, public contract, shipped JSON, grant,
pin or frozen byte.

### Baseline red before the repair (2.1.5)

The regression
`a_competing_control_beside_a_matching_sandbox_refuses_in_either_contribution`
(bundle/agent_tests.rs) pairs each of the four controls with the authored
command and with each selected fragment (`hands.workspace` under a box,
`hands.harness.gate` for a gate, `hands.harness.work` for work), sixteen
rows, each under the MATCHING requested class. Run on the adopted guard
before the repair (`.forge/unit2-pre-repair.log`): **12 of 16 rows failed**,
every one of them `compiled: … ("work", Some(LocalTools { allow:
Some(["cargo"]), sandbox: Some(ReadOnly) }))` where the full refusal was
expected — the three full-auto fragments and the authored full-auto, the
four bypass rows, the four `sandbox_workspace_write` rows (the demonstrated
`network_access=true`). The four `sandbox_mode` rows passed at baseline, as
D5.5 predicted for the existing door check. After the repair the same
command shows **16 of 16 rows** refusing with the exact cause, and the three
plain fixtures admit with the exact effective value and the exact argv
(dispatch token onward; `{brokkr}` is expanded to the engine path).

### Every row reaches its assertion

The specification correction found that a table's first failing row proved
no later row. Both suites now compute every row of a table first and report
every mismatch together (`each_row`, with `library_outcome` /
`outcome` rendering an unexpected load or compile beside the expected
refusal instead of panicking at it). A mutation that touches several rows
therefore names each of them. Converted: the decoder table (15 rows), the
decode-positive test (12 rows, including the `parse` inverse per class), the
repeated-key tests in both suites (3 rows each), the narrowing test (22
rows, the former field-only `.unwrap_err().0` rows replaced by complete
`(field, cause)` expectations for both valid-sibling cases and every
refusing class pair), the narrows test (3 + 7 rows), the inline test (12
rows: workspace-write and both-fields rows added), the adapter-context test
(2 rows: a malformed adapter file added), containers (5 rows), executable
forms (20 rows: a malformed row per form added), two sites (4 rows), the
boxed-office test (2 rows), the dialect test (4 rows) and the admission
table (19 rows: gate and work admitted fixtures now assert holdings, native
OFF and hands unchanged; rows added for a missing `hands.harness.work`
fragment, a DECLARED empty gate fragment, and the judges list's precedence
with a class that would otherwise mismatch). New tests: the regression above
and `an_optional_want_does_not_forgive_a_local_error` (agents/tests.rs).
Counts: 66 agent tests, 32 bundle tests, **98** in the two suites.

### Mutation ledger

Every mutation is a compiling edit inside the three production files,
applied on the repaired tree, run against the tests named, then restored
with `git checkout -- <the three files>` and a clean `git status` (batches
A–L, logs `.forge/unit2-mut-*.log`). Mutations were batched only where their
target rows are disjoint; each row below names the one mutation that
explains it. "compiled" means the row's `outcome` was a successful compile
where a refusal was expected; the full left/right is in the log.

| # | File, mutation | Rows that failed (test: row) |
| --- | --- | --- |
| A1 | load.rs `only_keys` admits `invented` | decoder: `{"invented":1}` (loaded) |
| A2 | agents.rs `narrow` subset find guarded by `name.is_empty()` | narrowing: adds a name, valid class beside an added name, empty office a name (all `Ok(...)`, right `Err(("allow", …))`); narrows: `["git","make"]`; forms: widening ×5 (left the compose "maps no tool permission named 'make'" gap); report_narrowed and optional-want widening asserts (`unwrap_err` on Ok) |
| A3 | load.rs `parse_agent` reads via `read_json` | repeated (agents): allow, tools, sandbox (all loaded) |
| A4 | bundle.rs `refuse_tools_on_container` reads `tools-never` | containers: all 5 rows compiled |
| A5 | bundle.rs admission `.take(1)` on the chain | admission: later candidate (compiled) |
| A6 | bundle.rs `compile_with_capabilities` falls back to a lenient serde read of bundle.json/policy.json into `compose::Resolved` on a resolve error (in-scope caller of `compose::resolve(&dir)?`; compose.rs untouched) | repeated (bundle): allow and tools compiled, sandbox reached the no-hands admission refusal — none the strict-source refusal |
| B1 | load.rs allow `Some(Null) => None` | decoder: `{"allow":null}`, `{"allow":null,"sandbox":"read-only"}`; (its narrows row was reached in batch C) |
| B2 | agents.rs widening check `false &&` | narrowing: widens the class, valid subset beside a widened class, WW under RO, DFA under RO, DFA under WW; boxed-office: widened to DFA |
| B3 | agents.rs `report_narrowed` does not apply `effective.allow` | report_narrowed `tests.rs:3282` left `allow: Some(["cargo","git"])` right `Some(["git"])`; optional-want `:3453` left `Some(["cargo","git"])` right `Some([])`; narrows `:1126` subset local; forms: subset and explicit empty for every non-member form |
| B4 | bundle.rs `record_inline_tools` allow presence `false` | inline: `allow: []`, `allow: [cargo]`, both fields; dialect: validate allow [] and [cargo] |
| B5 | bundle.rs `open` routed through the harness fragments | admission: open work (left the work-fragment mismatch) |
| B6 | bundle.rs `needs_adapters` key `tools-never` | adapter-context: compiled (`:9:18`). CORRECTED (second review return, P3): this run happened before the test was rows, and the old `error` helper aborted at the missing-directory case, so the malformed-adapter assertion was never reached; "both rows share the enforcement" was an inference, not an observation. The rerun that reaches both rows is S-M8 below. |
| B7 | bundle.rs `MEMBER_KEYS` without `tools` | forms: work:a ×4 (unknown key); two-sites: unwrap on unknown key 'tools' at work:a |
| C1 | load.rs allow `Some(String(one)) => Some(vec![one])` | decoder: `{"allow":"cargo"}` |
| C2 | agents.rs `reach` WorkspaceWrite => 3 | narrowing: widens the class, valid subset beside a widened class, WW under DFA, DFA under WW |
| C3 | bundle.rs `record_inline_tools` sandbox presence `false` | inline: sandbox read-only, workspace-write, danger-full-access |
| C4 | bundle.rs `SANDBOX_TABLES` keyed `sandbox_mode_never` | competing: all four `sandbox_mode` rows; admission: configuration door |
| C5 | agents.rs explicit-empty compose guard `&& false` | explicit-empty test `:133:6` (composed); optional-want `:3456` (composed); narrows: `{"allow":[]}`; forms: explicit empty ×5 |
| C6 | bundle.rs seat-level container call `&&`-chained | containers: panel, sequence, select at a seat |
| D1 | load.rs `string_array` skips non-strings | decoder: `{"allow":[1]}`; forms: malformed ×5 |
| D2 | agents.rs `widens` uses `>=` | narrowing: inherits same class, RO under RO, WW under WW, DFA under DFA |
| D3 | bundle.rs `record_inline_tools` stores nothing | inline: None, `{}`, `{"mcp":[]}` (left `local: None`); dialect first assert; narrows review-site assert `:1136` |
| D4 | bundle.rs harness check `&& false` | admission: codex label other harness, claude with hands, later candidate (all compiled) |
| D5 | bundle.rs body-level container call `&&`-chained | containers: panel as a selected body |
| D6 | bundle.rs `SANDBOX_SWITCHES` without `--full-auto` | competing: all four `--full-auto` rows |
| E1 | load.rs duplicate find guarded by `name.is_empty()` | decoder: `["cargo","git","cargo"]`; optional-want and hands syntax asserts (`:65:45`, loaded); narrows `["git","git"]` (batch G) |
| E2 | agents.rs subset arm replaced by office-ordered intersection | narrowing: inherits `["git","cargo"]` (left `["cargo","git"]`) |
| E3 | bundle.rs `SEAT_KEYS` without `tools` | forms: work ×4; containers: panel/sequence/select at a seat (unknown key instead of the container cause); narrows `:1107` unwrap |
| E4 | bundle.rs classless arm `None => {}` | admission: classless fragment (compiled) |
| E5 | bundle.rs step-level container call `&& false` | containers: panel as a sequence step |
| E6 | bundle.rs `SANDBOX_SWITCHES` without the bypass switch | competing: all four bypass rows |
| F1 | load.rs `named` check removed | decoder: `Bash(cargo:*)`; inline: `Bash(cargo:*)` |
| F2 | agents.rs allow inheritance `(_, None) => None` | narrowing: inherits `(None,None)`, inherits `(None, RO)`; narrows omission ×3; admission `:1715` boxed local (left `allow: None`) |
| F3 | bundle.rs `STEP_KEYS` without `tools` | forms: work:first ×4; dialect `:2287` unwrap (unknown key at design:validate) |
| F4 | bundle.rs `SANDBOX_TABLES` without `sandbox_workspace_write` | competing: all four `sandbox_workspace_write` rows |
| G1 | load.rs sandbox `Some(Null) => None` | decoder: `{"sandbox":null}` |
| G2 | bundle.rs `BODY_KEYS` without `tools` | forms: work:engine ×4, work:default ×4 |
| G3 | bundle.rs unreadable fragment `=> Ok(None)` | admission: unreadable fragment (left "names no `--sandbox` class") |
| G4 | bundle.rs `fragment.is_empty()` guard `&& false` | admission: declared empty gate fragment (left "names no `--sandbox` class") |
| G5 | bundle.rs no-hands guard `&& class.is_empty()` | narrows: `{"sandbox":"read-only"}` (left "dispatches the 'claude' harness") |
| H1 | load.rs non-string sandbox `=> None` (null kept) | decoder: `{"sandbox":1}`, `{"sandbox":{…}}` |
| H2 | agents.rs sandbox inheritance `(_, None) => None` | narrowing: inherits ×4 and empty office empty request; boxed-office admitted assert `:2171` |
| H3 | bundle.rs `resolve_reference` stores the fact only when unspecified | narrows omission ×3 (`None`); two-sites ×4; forms subset ×5 |
| I1 | agents.rs `Sandbox::parse` maps an unknown word to ReadOnly | decoder: both `loose` rows; narrows `loose`; inline `loose` |
| I2 | bundle.rs authored contribution check removed | competing: all four authored rows; admission: authored competing class |
| I3 | bundle.rs any nonempty class matches | admission: boxed WW/DFA, gate WW/DFA, work RO/DFA; boxed-office: narrowed to read-only at harness work |
| J1 | load.rs nonempty `mcp` check disabled | decoder: the `mcp: [{"server":"github"}]` row |
| J2 | agents.rs `parse` rotates the three words | decode-positive: declared fields, declared local(), class ×3, parse ×3, legacy mcp [] |
| J3 | load.rs explicit `[]` returned as unspecified | decode-positive: explicit empty allow; explicit-empty `:3362` (left `None` right `Some([])`); forms explicit empty ×5; inline `allow: []` |
| J4 | load.rs `{}` decodes allow as `Some([])` | decode-positive: tools {}; narrows `:1107` unwrap (empty refusal at omission); inline `{}` and `{"mcp":[]}` (with J3: the sandbox-only rows also compiled — an interplay of the two, each row bound alone in B4/C3) |
| K1 | load.rs `mcp: []` refused too | inline `{"mcp":[]}`; decode-positive and narrows aborted at their fixture `unwrap` (`:57:45`, `:1107:46`) because the fixtures write `mcp: []` |
| K2 | bundle.rs `SANDBOX_TABLES` without `sandbox_mode` | competing: all four `sandbox_mode` rows; admission: configuration door |
| K3 | bundle.rs admission moved BEFORE the hands law and route policy AND the local fact stored before the D33 pre-check | admission: open gate, missing gate fragment, missing work fragment (each left the admission refusal), and — with the sharpened row — judges list (left the gate mismatch, right the judges refusal) |
| L1 | bundle.rs competing checks skipped unless `part == "authored command"` | competing: all twelve fragment rows; admission: configuration door |
| L2 | agents.rs `report_narrowed` does not apply `effective.sandbox` | report_narrowed `:3347` (left `sandbox: None` right `Some(ReadOnly)`); boxed-office: narrowed to read-only at harness work (compiled) |

Retained from the first implementation's ledger without rerun: the
D33-precedence observation (admission placed before the hands law alone
passes; K3 is the two-part experiment that makes it fail). Every mutation
above was restored; after each batch the three files matched the committed
tree byte for byte.

Honest limits:

- The strict raw-bundle source test is now bound by A6, a compiling
  mutation of the in-scope caller in `bundle.rs`; `bundle/compose.rs` was
  not edited. No split is needed.
- LaneTally, DSH and a raw exec dispatch have no dedicated adapter fixture
  here. The harness check is one branch (`harness != "codex"`), bound by
  the shim, claude and later-candidate rows (D4); an inline site refuses a
  class before any driver is read (the inline rows, C3), whatever its
  command dispatches. A dedicated boxed DSH/LaneTally fixture would meet
  the tier refusal first and prove nothing more about this unit.
- The `outcome` string of the inline test renders every site's fact, so a
  mutation of the agent-backed `work` seat (F2, H3) shows up in the inline
  test's rows too; those rows are read as that mutation's, not the inline
  path's.
- Fixtures: `AgentFixture` and `Tree` canonicalise once and derive every
  path; the alias-root habit is exercised on this Linux host only. No test
  reads `.forge/`, discovers a provider or starts a model.

### Gates on the restored tree

All on `0444421d` plus this working tree, after every mutation was
restored.

- `cargo fmt --all -- --check`: **passed** (after `cargo fmt --all`
  touched only the three edited files).
- `cargo clippy --workspace --all-targets --all-features --locked -- -D
  warnings`: **passed**, no warning.
- `cargo test -p brokkr-runtime --all-features --locked`: **passed** — the
  lib suite and every integration binary, 25 green result lines, no
  failure (`.forge/unit2-gate-runtime.log`).
- `openspec validate --all --strict --no-interactive`: **passed, 18/18**
  (informational length notices only).
- `git diff --check`: **passed**.
- `cargo test --workspace --all-features --locked`: **passed** — 77 green
  result lines, no failure, no panic, every binary ran to its result
  (`.forge/unit2-gate-ws-all-features-2.log`).
- `cargo test --workspace`: **passed** — 77 green result lines, no failure;
  the run completed under a timeout (`.forge/unit2-gate-ws-2.log`). The
  seat's first attempt at this command (`.forge/unit2-gate-ws.log`) stopped
  inside the brokkr-cli integration binaries with no result line and no
  failure; that log is a stopped run, not a red, and the rerun is the
  observation.
- `cargo run --locked -p brokkr-cli -- compile --bundle bundles/self` and
  `... --bundle bundles/verify`: both **compiled** and printed their plan.
- The seat resumed once after the WIP commit `38660cc8`; on the resumed
  tree (that commit plus the final test-row conversions) fmt, clippy, the
  two owning suites (98 passed), both workspace suites, both bundle
  compiles, openspec validate (18/18) and `git diff --check` were each
  observed again before the unit's commit.
- External exact coverage (`scripts/coverage-exact.sh`), macOS and remote
  CI: **pending**, not observed here; nothing is called fully green on
  their account, and unit 1's pending results stay pending.

## Unit 2 — review return, seven chief findings answered, 2026-09-23

Run `build-decision-0065-slice-one-re-e332dc8d`, phase `implement`, returned
from `review` (chief gpt-6-astra, result `residual`, medium security
residual) on head `ca2c9156`, `slice-0065-capabilities`. The worktree began
clean; every earlier commit of this unit remains an ancestor and none was
replayed. This seat had cargo 1.98.0 and openspec 1.12.0; every Rust result
below is a fresh observation on this revision. The chief's seven findings
are the work this visit owns; each is answered below by name. Production
edits stay inside `bundle.rs`; `agents.rs` and `agents/load.rs` carry no net
change (they were mutated and restored byte for byte). No new module,
dependency, `Candidate` field, public contract, shipped JSON, grant, pin or
frozen byte.

### Findings and answers

- **F1 (medium, security) — admission accepted any class the fragment
  happened to express.** Confirmed at the source: `admit_local_sandbox`
  compared the request with the selected fragment and nothing else, so a
  gate fragment written `--sandbox workspace-write` admitted a typed
  `workspace-write` gate. Repaired: `bundle.rs::admitted_sandbox` is D5.3's
  table keyed on the path alone (boxed → read-only, harness gate →
  read-only, harness work → workspace-write). It is checked as its own
  condition after the fragment match, so adapter data is a representation
  and never an authority; the refusal names the site kind, boundary, the
  admitted class, the provider and the fragment that matched. Six new rows
  pair each matching-but-prohibited request/fragment (boxed and gate under
  workspace-write and danger-full-access, work under read-only and
  danger-full-access). The seven pre-existing mismatch rows keep their
  wording, because the fragment check still runs first and independently.
- **F2 (medium, security) — `Effect::Load` ignored and config outside the
  two doors permitted.** Confirmed: `expressed_sandbox` walked every node
  but acted only on `--sandbox`, the two switches and the two tables, so
  `--profile ci` (a grammar-typed load) and `-c approval_policy="never"`
  passed beside a matching class. Repaired in the same loop: a `Load` node
  refuses naming its canonical option; a `Config` node whose key is neither
  under `mcp_servers.brokkr` (the hands transport) nor exactly
  `model_reasoning_effort` (the effort) refuses as unestablished, naming
  only the argument position, never the key. Four new rows: a profile load
  in the fragment (`--profile=ci`) and in the authored command (`-p ci`), an
  unestablished assignment in each. One positive row: a fragment carrying
  the shipped transport's three assignments plus the effort assignment
  admits with the exact effective value and the exact fragment.
- **F3 (low) — `verify:dialect-verify` recorded no local fact.** Confirmed:
  the generated validator went through `record_capabilities` only.
  Repaired: `record_inline_tools(dialect_site, &synthetic, …)` runs after
  it, so the generated site records the checked-unspecified value through
  the same path every visited executable takes. New test
  `a_dialect_wrapped_verify_relocates_its_declaration_and_the_validator_records_a_checked_value`
  compiles an agent-backed `verify` declaring `allow: ["git"]` under the
  shipped openspec dialect, on the fixture's own library and adapters plus
  the shipped exec adapter's bytes, and asserts as rows: `verify:checks`
  carries `Some(allow ["git"])` with one chain link, `verify` holds nothing,
  `verify:dialect-verify` and `design:validate` each hold
  `Some(LocalTools::unspecified())`; then the composed argv after the
  dispatch tokens is exactly `--allowedTools Bash(git:*)`.
- **F4 (medium) — the ledger overstated row-complete proof.** Every named
  gap now has its own compiling mutation, observed failure and restored
  pass (rows R3–R10 below): `tools: null` and `tools: []` (a new bundle
  row for `[]` beside the existing `null`), tools omission at agent-backed
  and inline sites, the unrestricted-office/both-fields narrowing row (one
  mutation per field), read-only under workspace-write and
  danger-full-access, and the dialect `validate sandbox` row, which C3 does
  bind (rerun as R8 and recorded). The inherited-body assertions are now
  rows of the executable-forms table, not assertions after it.
- **F5 (medium) — commissioned matrices incomplete.** The executable-forms
  table gains an omission row per form (five), with the inherited body
  as two rows: 27 rows. A new table
  `every_inline_executable_form_records_or_refuses_its_own_declaration`
  covers the nested inline forms — panel member, sequence step, selected
  case, selected default — with omission, `{}`, `allow: ["cargo"]`,
  `allow: []`, `sandbox`, malformed and unknown-key rows: 28 rows. The
  admission table gains the four commissioned other-harness rows —
  LaneTally, DSH, exec and a bare program (`<custom>`) — each a boxed agent
  with hands on a fixture adapter of that provider, each refused with the
  full harness cause: 34 rows.
- **F6 (low) — the fixture role was an outward symlink.** `AgentFixture`
  now writes `roles/work.md` as real bytes inside the bundle, the same
  `# work\n` the charter holds. The AC-5 equality test compared the two
  role paths canonicalised, which only the link made equal; it now compares
  the bytes each path holds and asserts each path is the one its seat
  named (the charter in the library, the role in the bundle).
- **F7 (run integrity) — panel prose directing the gate.** No action in
  this seat; recorded here as the chief recorded it.

### Baseline observed before the repair

On the adopted `bundle.rs` bytes (`git show HEAD:…` swapped in with the new
tests present, then the repaired file restored and `cmp`-checked;
`.forge/unit2r-baseline-red.log`): **98 passed, 2 failed**. The ten F1/F2
admission rows each `compiled: … sandbox: Some(WorkspaceWrite | ReadOnly …)`
where the full refusal was expected, and the F3 row read `Some((None, 0))`
against `Some((Some(LocalTools { allow: None, sandbox: None }), 0))`. Every
other new row passed at baseline (the other-harness, established-positive,
omission, inline-form, relocation and `tools: []` rows) and is bound by
mutation below rather than by a manufactured red. The pre-edit suites on
`ca2c9156` were **98 passed** (`.forge/unit2r-baseline-suites.log`). After
the repair and the new tests: **100 passed** (66 agent, 34 bundle;
`.forge/unit2r-after-tests.log`).

### Mutation ledger, review return

Each batch is a compiling edit inside the three production files, run
against both owning suites, then restored from a byte copy taken before the
batch and checked with `cmp` (logs `.forge/unit2r-mut-R*.log`). Batches
combine only disjoint rows; each row below names the one mutation that
explains it. "compiled" means `outcome` printed a successful compile where
a refusal was expected.

| # | File, mutation | Rows that failed (test: row) |
| --- | --- | --- |
| R1a | bundle.rs table check `requested != admitted && false` | admission: the six matching-but-prohibited rows (compiled) |
| R1b | bundle.rs `Effect::Load` check `&& false` | admission: workspace fragment loads a profile, authored command loads a profile (compiled) |
| R1c | bundle.rs `!established && false` | admission: workspace fragment / authored command assigns unestablished configuration (compiled) |
| R2a | bundle.rs `ESTABLISHED_KEYS` = `model_reasoning_effort_never` | admission: established transport and effort admit (left: the unestablished refusal at argument 8) |
| R2b | bundle.rs harness check exempts `lanetally`, `dsh`, `exec` | admission: lanetally and dsh with hands (left: the codex grammar cannot read `--tools`), exec with hands (left: no `hands.workspace` fragment). CORRECTED (second review return, P3): the `<custom>` row (`bundle/agent_tests.rs`, the bare-program adapter) was excluded from this mutation by design and so was NOT bound by it; the ledger's "as intended" claimed a row this experiment never reached. The mutation that binds it is S-M9 below. |
| R2c | bundle.rs dialect `record_inline_tools` call replaced by `let _` | dialect-wrapped: verify:dialect-verify (left `Some((None, 0))`) |
| R2d | bundle.rs `relocate_verify_facts` inserts `local: None` | dialect-wrapped: verify:checks (left `Some((None, 1))`) |
| R3 | load.rs `parse_tools` omission → `allow: Some([])` | decode-positive: tools omitted (left `allow: Some([])`); every bundle test aborted at its fixture's inline review seat — counted for the decoder row only, superseded by R3b for bundle rows |
| R3b-a | bundle.rs `resolve_reference` omission → `allow: Some([])` before narrowing | forms: omission ×5 (left the explicit-empty refusal); narrows: `None` (unwrap on that refusal); inline test: None/`{}`/`{"mcp":[]}` (the agent-backed `work` site in the rendered outcome) |
| R3b-b | bundle.rs `record_inline_tools` returns early when `tools` is absent | inline forms: omission ×4 (left `None`); forms: inherited body, review (left `None`); dialect-wrapped: verify:dialect-verify and design:validate (left `Some((None, 0))`) |
| R4 | load.rs non-object `tools` decoded as unspecified | decoder: `null`, `[]` (loaded); narrows: `null`, `[]` (compiled); loader-names test (`:65`) |
| R5 | agents.rs allow arm `(None, Some(_)) => None` | narrowing: unrestricted office, both fields (left `allow: None`) |
| R6 | agents.rs sandbox arm `(None, Some(_)) => None` | narrowing: unrestricted office, both fields (left `sandbox: None`); report_narrowed `:3347`; narrows: `{"sandbox":"read-only"}` (compiled) |
| R7 | agents.rs `reach` ReadOnly => 3 | narrowing: read-only under workspace-write, read-only under danger-full-access (left the widening refusal), inherits `(None, RO)`, WW under RO, DFA under RO; boxed-office: narrowed to read-only at harness work |
| R8a | bundle.rs `record_inline_tools` sandbox presence `false` (C3 rerun) | dialect: validate sandbox (left the no-hands admission refusal); inline: sandbox ×3; inline forms: sandbox ×4 |
| R8b | agents.rs `report_narrowed` does not apply `effective.allow` (B3 rerun) | forms: subset ×5, explicit empty ×5, inherited body, work (left `["cargo","git"]`); two-sites ×4; dialect-wrapped: verify:checks; report_narrowed, explicit-empty, optional-want asserts |
| R9a | bundle.rs `record_inline_tools` allow presence `false` (B4 rerun) | inline: `allow: []`, `allow: ["cargo"]`, both fields; inline forms: allow [cargo] ×4, allow [] ×4; dialect: validate allow [] and [cargo] |
| R9b | agents.rs allow inheritance `(_, None) => None` (F2 rerun) | narrowing: inherits `(None,None)`, inherits `(None, RO)`; forms: omission ×5; narrows: omission ×3; plus the compose-path asserts that read the office list |
| R10a | bundle.rs `record_inline_tools` stores nothing | inline: None/`{}`/`{"mcp":[]}` (left `local: None`); inline forms: omission ×4 and `{}` ×4; forms: inherited body, review; dialect-wrapped: verify:dialect-verify, design:validate |
| R10b | load.rs `string_array` skips non-strings | decoder: `{"allow":[1]}`; forms: malformed ×5; inline forms: malformed ×4 |
| R10c | load.rs `only_keys` admits `invented` | decoder: `{"invented":1}`; inline: `{"invented":1}`; inline forms: unknown key ×4 |

Retained from the first ledger without rerun: A1–L2 as recorded above,
with the corrections the chief named now covered by the rows here. Every
mutation was restored; after the last batch `git status` showed only
`bundle.rs` and `bundle/agent_tests.rs` modified, and the three production
files matched their saved copies byte for byte.

Honest limits:

- R3 is recorded as it happened: too broad for the bundle rows, kept for
  the decoder row it did reach, and replaced by the two narrower R3b
  mutations. R3b-a's inline-test rows fail through the rendered `work`
  site, as the first ledger already noted for F2/H3.
- The other-harness rows are refused at the D5.3 harness check, before any
  fragment is read; R2b shows that with the check exempted, LaneTally and
  DSH fall to the codex grammar's refusal of their Claude-shaped fragment
  and exec to the missing-fragment refusal — so no harness other than
  codex reaches the fragment match under any of the three refusals.
- The fixture adapters for LaneTally, DSH, exec and the bare program are
  test data probing the dispatch name; they claim nothing about those
  harnesses' real sandbox support, which D5.3 already refuses.
- No test reads `.forge/`, discovers a provider or starts a model. The
  dialect-wrapped test reads the shipped `dialects/openspec.json` and
  `adapters/exec.json` bytes from the workspace root, as the existing
  dialect tests do.

### Gates on the restored tree

All on `ca2c9156` plus this working tree, after every mutation was
restored.

- `cargo fmt --all -- --check`: **passed** (after `cargo fmt --all`
  touched only `bundle/agent_tests.rs`).
- `cargo clippy --workspace --all-targets --all-features --locked -- -D
  warnings`: **passed**, no warning (`.forge/unit2r-gate-clippy.log`).
- `openspec validate --all --strict --no-interactive`: **passed, 18/18**
  (informational length notices only; `.forge/unit2r-openspec.log`).
- `git diff --check`: **passed**.
- `cargo test -p brokkr-runtime --all-features --locked`: **passed**, 528
  library tests (the two owning suites contribute 34 bundle agent tests and
  66 agent tests, 100 in all) plus every integration binary of the crate
  (`.forge/unit2r-gate-runtime-final.log`).
- `cargo test --workspace --all-features --locked`: **passed**, every one
  of the 77 test binaries green, no failure, no hang under a 580 s timeout
  (`.forge/unit2r-gate-ws-final.log`).
- `cargo run --locked -p brokkr-cli -- compile --bundle bundles/self`:
  **passed**, the compiled bundle printed
  (`.forge/unit2r-gate-self-final.log`).
- External exact coverage (`scripts/coverage-exact.sh`), macOS and remote
  CI: **pending**, not observed here; nothing is called fully green on
  their account, and unit 1's pending results stay pending.

## Unit 2 — second review return, S1 and three proof gaps answered, 2026-09-23

Run `build-decision-0065-slice-one-re-e332dc8d`, phase `implement`, returned
from `review` (chief gpt-6-astra, result `residual`, medium security
residual) on head `4a441bf7`, `slice-0065-capabilities`. The worktree began
clean; every earlier commit of this unit remains an ancestor and none was
replayed. This seat had cargo 1.98.0 and openspec 1.12.0; every Rust result
below is a fresh observation on this revision. The chief's four findings
(S1, P1, P2, P3) are the work this visit owns; R1 is the chief's own
disposition of gate-directed prose and needs no action here. Production
edits stay inside `bundle.rs`; `agents.rs` and `agents/load.rs` carry no net
change (they were mutated and restored byte for byte). No new module,
dependency, `Candidate` field, public contract, shipped JSON, grant, pin or
frozen byte.

### Findings and answers

- **S1 (medium, security) — `expressed_sandbox` ignored `--add-dir`.**
  Confirmed at the source: the codex grammar types `--add-dir` as
  `Effect::Inert` with `equals: true` and `repeat: true`, and the guard
  acted on `--sandbox`, the two switches, the two tables, `Load` and
  `Config` only, so a harness work seat requesting `workspace-write` with
  hands admitted `--add-dir /srv/shared` and `--add-dir=/srv/shared` in the
  authored command and in every selected fragment. Repaired in the same
  loop: a node whose canonical name is `--add-dir` refuses, naming the
  option and never its value, wherever it stands (selected fragment or
  authored command). Eight rows join the competing-control regression
  (`a_competing_control_beside_a_matching_sandbox_refuses_in_either_contribution`,
  now 24 rows): the split and the `=` spelling, each paired with the
  authored command, the `hands.workspace` fragment under a box (read-only),
  the `hands.harness.gate` fragment (read-only) and the `hands.harness.work`
  fragment (workspace-write). The three plain positives at the end of that
  test are the supported positive: the same fragments without the control
  admit with the exact effective value and, for the box, the exact argv.
  The codex grammar declares `--add-dir` `attached: false`, so there is no
  third spelling to cover.
- **P1 (medium, spec-compliance) — substring assertions at
  `agents/tests.rs` for the unknown direct mapping and the later-candidate
  gap.** Both now assert the complete refusal through a new
  `resolution_outcome` helper that renders an unexpected resolution as its
  candidates' provider/model pairs beside the expected refusal, so a
  mutation reaches the exact assertion instead of an `unwrap_err` panic:
  `a_tool_the_provider_does_not_name_is_a_hard_failure` expects the whole
  claude/opus refusal naming `'git'`;
  `a_capability_gap_on_a_later_chain_entry_fails_just_as_loudly` expects the
  whole codex/sonnet `tool_permissions unsupported` refusal with the
  restriction `["cargo", "git"]`. New test
  `an_unavailable_fallback_with_a_local_gap_still_refuses_the_whole_chain`:
  an otherwise-valid claude primary (recorded `Available`) with a `second`
  fallback recorded `Unavailable` that maps no permission for `git`
  refuses with the whole second/sonnet cause; the same chain with the
  fallback's mapping completed resolves to `["claude/opus"]` alone,
  `chosen_index` 0, `skipped` empty.
- **P2 (medium, spec-compliance) — the inherited body covered a subset and
  the unspecified inline site only.** The executable-forms table
  (`every_executable_form_owns_its_local_declaration`) now writes the base
  layer per row and carries inherited explicit empty, widening and
  malformed declarations (30 rows). Each refuses with the leaf form's
  complete owning-site cause (`seat 'work'`), wrapped once by the existing
  composition note `(composed: fixture -> base)` from
  `compose.rs::Resolved::chain_note`; the assertion is the exact printed
  string including that note. The three rows were observed passing on the
  adopted bytes (they are not a new refusal, they are a composed input the
  table had not exercised) and are bound by S-M5, S-M6 and S-M7 below.
- **P3 (medium, spec-compliance) — two ledger rows overstated.** Both rows
  are corrected in place above (B6, R2b) rather than rewritten: B6's run
  predates the row conversion and its log shows the old `error` helper
  aborting at the missing-directory case, so the malformed-adapter
  assertion was never reached; R2b excluded `<custom>` by construction, so
  "unaffected, as intended" claimed a row the experiment never reached.
  S-M8 reruns B6 against the row-based test and reaches both rows; S-M9 is
  a new mutation that reaches the `<custom>` row.

### Baseline observed before the repair

On the adopted `bundle.rs` bytes with the eight new rows present
(`.forge/unit2s-baseline-red-S1.log`): **8 of 24 rows failed**, every one
`compiled: … ("work", Some(LocalTools { allow: Some(["cargo"]), sandbox:
Some(ReadOnly | WorkspaceWrite) }))` where the full `--add-dir` refusal was
expected — the authored split and equals rows and the three fragments for
each. The sixteen earlier rows passed. After the repair the same test
shows 24 of 24 rows refusing (`.forge/unit2s-post-repair-S1.log`). The
pre-edit suites on `4a441bf7` were **100 passed**; after the repair and the
new tests: **101 passed** (67 agent, 34 bundle;
`.forge/unit2s-after-tests.log`). The P1 exact assertions and the P2
inherited rows passed at baseline and are bound by mutation below.

### Mutation ledger, second review return

Each mutation is a compiling edit inside the three production files, run
against both owning suites (`cargo test -p brokkr-runtime --all-features
--locked --lib -- agents::tests bundle::agent_tests`), then restored from a
byte copy taken before the first mutation and checked with `cmp` (logs
`.forge/unit2s-mut-M*.log`). Each mutation was applied alone.

| # | File, mutation | Rows that failed (test: row, left) |
| --- | --- | --- |
| S-M1 | bundle.rs `node.name() == ADDED_ROOT && false` | competing: all eight `--add-dir` rows (split and equals × authored, workspace, gate, work), each `compiled: …`; nothing else |
| S-M2 | agents.rs compose `names.get(tool).or_else(\|\| names.values().next())` | unknown mapping `tests.rs:533` (left `resolved: ["claude/opus", "claude/sonnet"]`); unavailable-fallback `:743` (left `resolved: ["claude/opus"]`); sibling `report_walks_the_whole_chain_without_refusing` (`gap.is_some()`) |
| S-M3 | agents.rs `resolve_report` gap check `.filter(\|_\| entry.presence != Presence::Unavailable)` | unavailable-fallback `:743` only (left `resolved: ["claude/opus"]`) |
| S-M4 | agents.rs `resolve_report` first loop `.iter().take(1)` | later-link gap `tests.rs:710` (left `resolved: ["claude/opus", "codex/sonnet"]`); unavailable-fallback `:743`; three siblings that rely on the whole-chain walk (`mapped above` expects, `unwrap_err` on Ok) |
| S-M5 | agents.rs `narrow` allow widening `.filter(\|_\| false)` (B2 rerun) | forms: inherited body, widening (left the compose gap "maps no tool permission named 'make'", composed) plus the five leaf widening rows; narrowing ×3; report_narrowed, optional-want, per-field narrows |
| S-M6 | agents.rs explicit-empty compose guard `&& false` (C5 rerun) | forms: inherited body, explicit empty (left `compiled: … allow: Some([])`) plus the five leaf explicit-empty rows; explicit-empty test; optional-want; narrows |
| S-M7 | load.rs `string_array` non-string `=> continue` (D1 rerun) | forms: inherited body, malformed (left the explicit-empty refusal, composed — `[1]` decoded as `[]`) plus the five leaf malformed rows; inline forms ×4; decoder `{"allow":[1]}`; two loader tests |
| S-M8 | bundle.rs `needs_adapters` key `tools-never` (B6 rerun) | adapter-context: **both** rows — missing adapters directory (left `compiled: …`) and malformed adapter file (left `compiled: …`); nothing else |
| S-M9 | bundle.rs `dispatch_driver(..).unwrap_or_else(\|\| "codex".to_string())` | admission: `custom with hands` only (left the missing-`hands.workspace`-fragment refusal for provider 'custom', right the `<custom>` harness refusal) |

Every mutation was restored; after each the mutated file matched its saved
copy byte for byte, and after the last `git status` showed only
`bundle.rs`, `agents/tests.rs` and `bundle/agent_tests.rs` modified.

Honest limits:

- S-M2 and S-M4 fail sibling tests beside the rows they bind; each row
  above names the one mutation that explains it, and the siblings are
  recorded as siblings, not as proof of anything further.
- The inherited rows (P2) are composed inputs reaching the same
  enforcement the leaf rows reach; S-M5–S-M7 are reruns of B2, C5 and D1
  observed on this revision with the inherited rows named, not new
  enforcement.
- The `--add-dir` refusal is a compile-admission proof under the typed
  sandbox guard only. It is not unit 12's general authored-flag refusal,
  and it claims nothing about a live provider or a host escape.
- No test reads `.forge/`, discovers a provider or starts a model.

### Gates on the restored tree

All on `4a441bf7` plus this working tree, after every mutation was
restored.

- `cargo fmt --all -- --check`: **passed** (after `cargo fmt --all`
  reflowed the edited files).
- `cargo clippy --workspace --all-targets --all-features --locked -- -D
  warnings`: **passed**, no warning (`.forge/unit2s-gate-clippy.log`).
- `openspec validate --all --strict --no-interactive`: **passed, 18/18**
  (informational length notices only; `.forge/unit2s-openspec.log`).
- `git diff --check`: **passed**.
- `cargo test -p brokkr-runtime --all-features --locked`: **passed**, 529
  library tests plus every integration binary of the crate, 25 green
  result lines (`.forge/unit2s-gate-runtime.log`).
- `cargo test --workspace --all-features --locked`: **passed**, 77 green
  result lines, no failure, no panic, under a 580 s timeout
  (`.forge/unit2s-gate-ws-all-features.log`).
- `cargo test --workspace`: **passed**, 77 green result lines under the
  same timeout (`.forge/unit2s-gate-ws.log`).
- `cargo run --locked -p brokkr-cli -- compile --bundle bundles/self` and
  `... --bundle bundles/verify`: both **compiled**
  (`.forge/unit2s-gate-self.log`, `.forge/unit2s-gate-verify.log`).
- Frozen paths (`policy/`, `contracts/`, `fixtures/`, `reference/`,
  `extensions/`): `git diff --stat` empty.
- External exact coverage (`scripts/coverage-exact.sh`), macOS and remote
  CI: **pending**, not observed here; nothing is called fully green on
  their account, and unit 1's pending results stay pending.


## Unit 2-fix — specification adoption, 2026-09-23

Run `build-decision-0065-slice-one-re-11592690`, phase `specify`, adopts
`decision-0065-capabilities-slice-one` at
`6a7044e5292a86b877a0e11a1c66ab8a255fcabf` on `slice-0065-capabilities`.
The worktree began clean. Ancestry checks returned 0 for `5a47b090`,
`d9816374`, `4a6a2288`, `308a8a28`, `f9a691cf`, `368bc34e`, `107c4d17`,
`0444421d`, `ca2c9156`, `4a441bf7` and `6a7044e5`. Every adopted commit is
retained; no replay or branch change occurred. The supplied context has no
`returned_from`. The commission carries the chief's third-review findings
from `build-decision-0065-slice-one-re-e332dc8d`; that residual was not accepted.

Read the operator ruling/addendum, accepted Rebuild units preamble and unit 2,
then task 2.1 and its SCM/SC8/RGR/NCR requirements, D5.3–D5.5 and the earlier
unit 2 evidence. Also read README, decisions 0004/0005/0009/0065 and proposed
0066, `dialects/openspec.json`, its specify/return instructions, and the
rendered OpenSpec proposal then specs instructions. Inspected the named
production/suite sources and the existing grammar read-only. No workflow
runner, council or provider was invoked.

### Findings adopted and scope of the answer

**S1 (medium, security):** the chief reproduced native OFF contributions
carrying an added filesystem root, sandbox bypass and workspace-write network
access beside valid denial. Source inspection agrees: the typed admission
checks the selected hands fragment and candidate-authored argv in bundle.rs,
while `record_capabilities` resolves the native plan afterwards. The native
plan assembles the selected ON/OFF disposition and resolved restriction argv
in capabilities.rs. Checking only earlier contributions cannot answer this
finding. The repair must judge the actual resolved contributions, preserving
every valid denial control; blanket rejection of native configuration would
break the legitimate `-c`, `web_search="disabled"` denial positive.

**A1 (medium, security):** the chief reproduced all four root-selector
spellings in authored argv and hands.harness.work. The existing grammar
already canonicalizes them to `--cd` (alias `-C`, split/equals/attached forms)
and classifies that option Inert. `expressed_sandbox` explicitly refuses
`--add-dir` but does not inspect `--cd`. The required repair can therefore
use the existing canonical identity without widening to grammar.rs. The
panel's assertion that a later `-C` necessarily wins is unestablished and is
not adopted. The demonstrated omission is uncertain competing root authority
being admitted, not a proven live escape.

No specification defect or new semantic choice was found. Adopted the
proposal before amending the owning seat delta with explicit S1/A1 and valid
native-denial scenarios. Its `## Decisions` records the acceptance and the
reasoned refusal of origin/priority exemptions and blanket denial rejection.
These instantiate D5.3/D5.5 and SCM's existing no-competing-control rule;
the other deltas, design and accepted unit order remain coherent unchanged.
Decision 0066 remains proposed. There is no reason for an `upstream` result.

Tasks.md records **2-fix under unit 2** and reopens only aggregate 2.1 and its
2.1.5 admission, 2.1.6 proof and 2.1.7 fresh-gate portions for this repair.
Earlier decoder/narrowing/site facts and all recorded observations survive.
Production remains bounded to bundle.rs and, only if exposure of resolved
native contributions requires it, capabilities.rs. The sole owning suite is
bundle/agent_tests.rs. Any larger actual scope requires a split before work;
units 13–15 cannot absorb this compile-admission omission.

### Chief's reproductions and intended independent assertions

These are the supplied chief observations, **not new executions by this
specify seat**. The chief used a canonical temporary fixture through
`Bundle::compile_under` → `engine::compose_site` → `adapters::codex_command`
and compared complete expected cold argv. All eleven defect cases and the
two baseline controls matched; no provider was launched. Implementation must
first execute the exact refusal regressions on the adopted guard and retain
its own observed baseline, compiling mutation failure and restored pass per
row. No unexecuted result below is claimed as a baseline red.

| Case | Contribution / matching typed path | Independent refusal to bind |
| --- | --- | --- |
| S1.1 | Native web-search OFF; harness workspace-write work | Added `--add-dir=/srv/shared` beside legitimate denial refuses. |
| S1.2 | Native OFF; harness read-only gate | `--dangerously-bypass-approvals-and-sandbox` beside legitimate denial refuses. |
| S1.3 | Native OFF; harness workspace-write work | `sandbox_workspace_write.network_access=true` beside legitimate denial refuses. |
| A1.1 | Authored argv; harness workspace-write work | `--cd /` refuses as canonical `--cd`. |
| A1.2 | Authored argv; harness workspace-write work | `--cd=/` refuses as canonical `--cd`. |
| A1.3 | Authored argv; harness workspace-write work | `-C /` refuses as canonical `--cd`. |
| A1.4 | Authored argv; harness workspace-write work | `-C/` refuses as canonical `--cd`. |
| A1.5 | hands.harness.work; matching workspace-write | `--cd /` refuses as canonical `--cd`. |
| A1.6 | hands.harness.work; matching workspace-write | `--cd=/` refuses as canonical `--cd`. |
| A1.7 | hands.harness.work; matching workspace-write | `-C /` refuses as canonical `--cd`. |
| A1.8 | hands.harness.work; matching workspace-write | `-C/` refuses as canonical `--cd`. |

Each refusal needs its full bounded cause without the option value. Extend
these proofs to the actual resolved native ON/OFF/restriction contributions
and root-selector spellings, retaining matching gate/work positives with
exact typed class, hands fragment, empty holdings and native OFF facts.
Every newly claimed row must independently reach its own intended failing
assertion under a compiling in-scope mutation, then pass after restoration.
The existing canonical AgentFixture and each_row helper are available;
no new suite, .forge fixture reads or installed provider is commissioned.
No Rust test or production edit, mutation, restored pass or security closure
is claimed in this specification visit.

### Observed validation and limits

- `openspec validate decision-0065-capabilities-slice-one --strict --no-interactive`:
  **passed**.
- `openspec validate --all --strict --no-interactive`: **passed, 18/18**.
  Existing informational length and unrelated archive-target advisories remain;
  no archive was attempted.
- `git diff --check`: **passed**.
- Artifact/link audit: **passed**. Exactly the four intended artifact paths
  changed; the six proposal capabilities resolve to their deltas, all eight
  reopened/new task entries are unchecked, and each 2-fix requirement link
  resolves to its owning requirement.
- Fresh format, locked all-target/all-feature clippy, runtime all-feature tests,
  `cargo test --workspace`, locked all-feature workspace tests and self/verify
  bundle compiles: **unavailable**. Each exact command in tasks.md's local-gate
  list was attempted via subprocess and raised `FileNotFoundError` for `cargo`.
  No Rust command ran, and the reporting script's exit 0 is not a gate pass.
  No host-shell fallback or existing target binary was substituted.
- External exact coverage, macOS and remote CI: **pending**, unchanged; prior
  local results remain historical. No fully green or unit-complete claim is made.

Only proposal.md, specs/seat-capability-resolution/spec.md, tasks.md and this
evidence file change. No production, test, pin, grant, shipped data, frozen
contract, policy, fixture, reference or extension bytes change. The engine
result is written through workspace hands to its commissioned run-local file,
not committed as a fifth artifact. The final commit and post-commit checks
belong in that result so recording them does not move the validated head.
No push occurs.

## Unit 2-fix — council design adoption, 2026-09-23

Run `build-decision-0065-slice-one-re-11592690`, phase `design`, began clean at
`e132f6783bdd3f3c72e1ac96c91621fdf587eb53` on `slice-0065-capabilities`.
Adopted `decision-0065-capabilities-slice-one` and every preceding commit;
unit 2 through `6a7044e5` and specification `e132f678` remain intact. There is
no `returned_from`. S1/A1 are the supplied third-review findings, with their
residual unaccepted, not an upstream specification defect.

Read the operator ruling/addendum before the Rebuild units preamble/unit 2,
then the owning tasks/deltas, D5.3–D5.5 and both current council positions in
full: `.forge/design/positions/robustness.md` and `simplicity.md`. Read README,
decisions 0004/0005/0009 and the relevant 0063/0065 rules; inspected the named
Rust files, existing grammar/managed decoder and Codex adapter read-only.
Read `dialects/openspec.json`, its design/return instructions, and rendered
`openspec instructions design --change decision-0065-capabilities-slice-one
--json` through workspace hands. No workflow runner or provider was invoked.

### Council disposition and dependent artifacts

Authored design first, then updated dependent tasks and this evidence. D5.6's
claim-by-claim table adopts or combines both positions on the inspected seams:
`record_capabilities` has every resolved outcome before facts/notices are
published; `Outcome::controls()` and the managed decoder already expose its
selected/substituted argv; the grammar already canonicalizes all four root
spellings. Thus bundle.rs alone is the planned production repair, with
capabilities.rs conditional only on demonstrated missing exposure.

D5.6 preserves earlier refusal precedence, uses the effective inherited sandbox,
checks every outcome and every parsed contribution, and refuses canonical
`--cd` without a value. It combines robustness's explicit private context
with simplicity's small helper argument: a narrow native-only canonical
`web_search` key allowance preserves the measured OFF pair without exempting
competing controls. It rejects blanket native-config refusal or trust,
primary-only/OFF-only checks, priority assumptions, duplicate resolution,
new origin machinery, and deferral to final-launch units, with reasons.
No new spec ambiguity was found: the adopted SCM scenarios already encode
S1/A1 and valid-denial behavior, so the proposal and deltas remain unchanged.
Decision 0066 remains proposed.

Tasks 2-fix.1–2-fix.4 remain unchecked, as do the reopened portions of
2.1/2.1.5–2.1.7. They now reference D5.6's implementation seam and exact proof
requirements: every chief case, all four root spellings in resolved native
ON/OFF/nonempty substituted restriction paths, inherited sandbox, later
candidate, untyped preservation, bounded Unicode/newline diagnostics, and
full matching denial values. Every new independently claimed row needs a
compiling mutation and observed assertion failure followed by restoration
and pass. The chief's eleven cold-command cases/two controls remain supplied
evidence; this seat ran no Rust regression, mutation or reproduction. Synthetic
restriction refusal fixtures will not close unit 9's provider qualification.
The Rebuild units list retains its numbered order, with 2-fix nested under
unit 2; no later unit is pulled forward or closed.

### Observed checks and limits

- `openspec validate decision-0065-capabilities-slice-one --strict --no-interactive`:
  **passed**.
- `openspec validate --all --strict --no-interactive`: **passed, 18/18**.
  Informational long-requirement and unrelated archive-target advisories remain;
  no archive was attempted.
- `git diff --check`: **passed**.
- Each exact cargo command from tasks.md's local-gate list was attempted:
  format; locked all-target/all-feature clippy; runtime all-feature tests;
  `cargo test --workspace`; locked all-feature workspace tests; self and verify
  compiles. All seven were **unavailable** with `FileNotFoundError: cargo`.
  No cargo process ran, and the reporting script's exit 0 is not a Rust gate
  pass. No host-shell fallback or prebuilt binary was substituted.
- External exact coverage, macOS and remote CI remain **pending**. Historical
  results retain their original tested scope; no fully green claim is made.

Only design.md, tasks.md and evidence.md are committed for this design seat.
No Rust/test, pin, shipped data, grant, frozen contract, policy, fixture,
reference or extension edits are made. Final artifact/ancestry checks and the
committed SHA are recorded in the required run-local result through workspace
hands, so recording the result cannot move the validated source head. No
push, security closure or implementation completion follows from this draft.

## Unit 2-fix — ordered tasks adoption, 2026-09-23

Run `build-decision-0065-slice-one-re-11592690`, phase `tasks`, began clean
at `a4787b2e2ad30bf4a054f2372f3e4ad312a9f70c` on
`slice-0065-capabilities`. Adopt every commit, including unit 2 through
`6a7044e5`, specification `e132f678` and design `a4787b2e`. Ancestry checks
returned 0 for `5a47b090`, `d9816374`, `4a6a2288`, `308a8a28`, `f9a691cf`,
`368bc34e`, `107c4d17`, `0444421d`, `ca2c9156`, `4a441bf7`, `6a7044e5`,
`e132f678` and `a4787b2e`. No replay, branch change or push occurred.
There is no `returned_from`; this visit answers the commissioned S1/A1
breakdown after specification and council design. Their residual remains
unaccepted; no implementation or security closure is claimed.

Read the operator ruling/addendum first, then the accepted Rebuild units
preamble/unit 2, task 2.1, the owning SCM/SC8/RGR/NCR deltas, proposal,
D5.3–D5.6 and Open questions. Read `dialects/openspec.json`, its tasks/return
instructions, and the rendered `openspec instructions tasks --change
 decision-0065-capabilities-slice-one --json`. Inspected the existing
bundle admission and `record_capabilities` seams, `site_capabilities`,
`Outcome::controls()`, native selected/restriction argv assembly, the typed
managed decoder and canonical fixture/row helpers read-only. No provider,
workflow runner or new council was invoked.

### Executable breakdown and requirement coverage

Only tasks.md and this evidence file change. Numeric tasks **2.2–2.8** replace
the provisional `2-fix.1`–`2-fix.4` entries under the same unit 2-fix heading;
each carries its previous entry/portion and names its served requirements.
The accepted unit order, historical task text outside this subsection,
checked 2.1.1–2.1.4 and open aggregate 2.1/2.1.5–2.1.7 remain intact.
The coverage table maps the three adopted S1/A1/denial scenarios and the
inherited/fallback/preservation/proof obligations to concrete execution tasks.

The order is baseline capture (2.2), shared canonical root refusal (2.3),
resolved native admission and exact denial preservation (2.4), complete
contribution/preservation proofs (2.5), ledger audit (2.6), restored local
gates and commit (2.7), then actual external evidence (2.8). The behavioral
tasks each require their own independent compiling mutations and restored
passes before advancing; the ledger audit does not postpone proof. The
baseline task stages every planned row before production changes, including
the eleven chief cases and the twelve native ON/OFF/restriction root-spelling
rows. Already-correct neighbors start with observed passes. None of those
regressions or mutations was run by this tasks seat.

Source inspection supports D5.6 without another design choice:
`record_capabilities` receives resolved outcomes before publishing facts and
notices; `Outcome::controls()` and `native_controls::managed` expose the
complete selected/substituted argv. Earlier local admission stays ordered,
and the effective local fact provides inherited sandbox information. The
planned production file is bundle.rs; capabilities.rs remains conditional
on an evidenced missing exposure. The exact native-only `web_search` config
allowance preserves the measured denial pair while all sandbox/root checks
still apply. No new parser, module, origin machinery, public contract or
provider qualification is proposed. The sole suite stays bundle/agent_tests.rs.

Open questions belong to unit 9 and external proof; D5.6 explicitly settles
this repair. A synthetic nonempty restriction refusal does not qualify
provider support or close unit 9, and cold comparisons do not close units
13–15. Thus no earlier artifact needs amendment and no `upstream` result is
warranted. Proposed decision 0066, the specification/design and later units
remain unchanged. Every repair task is unchecked. The breakdown distinguishes
a committed local repair from complete, fully green work; pending external
evidence cannot close aggregate 2.1. Archive remains with conditional 28.1.

### Observed checks and limits

- `openspec validate decision-0065-capabilities-slice-one --strict --no-interactive`:
  **passed**.
- `openspec validate --all --strict --no-interactive`: **passed, 18/18**.
  Existing informational requirement-length and unrelated archive-target
  advisories remain; no archive operation was run.
- `git diff --check`: **passed**.
- Task artifact audit: **passed**. Tasks 2.2–2.8 are ordered, unchecked,
  requirement-named and traceable to their provisional predecessors. Every
  cited requirement name and anchor resolves to its delta. The rest of the
  numbered task groups and prior unit 2 substeps are byte-identical to the
  adopted head; the checked and reopened states above are preserved.
- Each of the seven exact cargo commands in tasks.md's local-gate list was
  attempted: format, locked all-target/all-feature clippy, runtime all-feature
  tests, `cargo test --workspace`, locked all-feature workspace tests, and
  self/verify compiles. All were **unavailable**: subprocess raised
  `FileNotFoundError: cargo` before any cargo process ran. The reporting
  script's exit 0 is not a Rust gate pass. No host-shell or prebuilt-binary
  fallback was used. This documentation draft asserts no fresh Rust pass.
- External exact coverage, macOS and remote CI remain **pending**. Previous
  gate observations retain their original tested revision and scope.

No production, suite, pin, shipped data, grant, frozen contract, policy,
fixture, reference or extension bytes change. Final scope/diff checks and
committed-head evidence go in the mandatory run-local result written through
workspace hands, so that recording the commit cannot move the validated head.
The tasks-phase result is a committed **drafted** breakdown, not a completed
repair or a claim that the security findings are fixed.


## Unit 2-fix — repair: canonical root refusal and resolved native admission, 2026-09-23

Run `build-decision-0065-slice-one-re-11592690`, phase `implement`, based on
`07228d50` (`slice-0065-capabilities`). The worktree began clean; every
adopted commit through `07228d50` remains an ancestor and none was replayed.
There is no `returned_from`; the commission is the chief's S1/A1 from the
third review. This seat had cargo 1.98.0, the pinned `nightly-2026-09-05`,
cargo-llvm-cov 0.9.0 and openspec; every Rust result below is a fresh
observation. Scratch logs are under `.forge/u2fix/` (run-local, not read by
any test).

### Production, inside the allowlist (2.3, 2.4)

Only `crates/brokkr-runtime/src/bundle.rs` changed. `capabilities.rs` did not:
the inspected projection sufficed — `SiteCapabilities.outcomes` in candidate
order, `Outcome::controls()` and the typed decoder
`brokkr_protocol::native_controls::managed` expose the complete resolved argv,
so no concrete missing exposure was found and none is recorded.

- **A1, root selector.** `expressed_sandbox` refuses a node whose canonical
  name is `--cd` — the grammar reads `--cd PATH`, `--cd=PATH`, `-C PATH` and
  `-CPATH` as that one node — for every value, the current workspace
  included, in every contribution it judges. The cause names `` `--cd` ``
  and never the path. `grammar.rs` is untouched; no last-option priority is
  inferred.
- **Contribution context.** A private `Contribution { Written, Native }`
  argument tells authored/selected-hands bytes from the resolved native plan.
  It selects one allowance and skips no check: in `Native` context only, the
  exact key `web_search` is established beside `mcp_servers.brokkr.*` and
  `model_reasoning_effort` (not `web_search.*`, not a table, not
  adapter-declared). A native unqualified assignment's cause names
  `` `--config` `` and the argument position, never the key or value.
- **S1, resolved native admission.** New `admit_native_sandbox`, called from
  `record_capabilities` right after `site_capabilities` succeeds and before
  notices/facts are published, for a site whose recorded effective local fact
  (`SiteFacts.local`, an inherited office class included) carries a sandbox.
  It iterates every outcome, decodes `outcome.controls()` once through
  `managed` (a decode error refuses with the decoder's bounded reason;
  nothing defaults to empty) and judges the whole resolved argv — selected ON
  or OFF plus substituted restriction transport — through the same guard.
  Any native `--sandbox`, matching or not, refuses: only the selected hands
  fragment represents the class. `enforce_model_policy` →
  `admit_local_sandbox` order is unchanged; sites without a typed class keep
  their existing admission.
- **Coverage fold.** The adopted `None => unreachable!(…)` arm of the grammar
  lookup in `expressed_sandbox` was the one zero-count line the coverage
  diagnostic found in the changed function (below); it became
  `.expect("the codex grammar is modelled")` with identical behaviour.

### Baseline observed before the repair (2.2)

The five new tests in `bundle/agent_tests.rs` were staged and run on the
adopted `bundle.rs` bytes (`07228d50`), `cargo test -p brokkr-runtime
--all-features --locked --lib -- bundle::agent_tests`
(`.forge/u2fix/baseline.log`, `baseline-links.log`). Every row reached its
own assertion through `each_row`.

| Test (line) | Rows | Baseline observed |
| --- | --- | --- |
| `a_root_selector_beside_a_matching_sandbox_refuses_in_the_authored_command_and_the_fragment` (2716) | A1.1–A1.8, `--cd .`, `--cd=<long>`, `-C<long>`, open-gate precedence | **11 of 12 red**: each of A1.1–A1.8 and the three extra root rows `compiled: … sandbox: Some(WorkspaceWrite)` where the full `--cd` refusal was expected; the open-gate row passed (standing refusal already first) |
| `a_resolved_native_off_contribution_cannot_compete_with_a_matching_sandbox` (2821) | S1.1–S1.3, full-auto, sandbox_mode, `--add-dir` split and `<long>`, unqualified, `web_search.` descendant, matching `--sandbox` ×2, native OFF ×4 root spellings, competing control before the denial | **16 of 16 red**, every row `compiled: …` (S1.2 and the gate rows `ReadOnly`, the rest `WorkspaceWrite`) |
| `resolved_native_on_and_restriction_contributions_obey_the_same_refusals` (3113) | native ON ×4 root spellings, restriction ×4, ON `--add-dir` | **9 of 9 red**, every row `compiled: …`; the real grant held web-search ON and the restriction reached its slot |
| `resolved_native_admission_judges_every_link_and_the_inherited_class_only_where_typed` (3240) | later candidate (link 2), inherited class | **2 of 2 red**, both `compiled: …` (first run failed on a fixture error — the second adapter's `judges` named an unmapped model; corrected before the recorded baseline, no production change) |
| `a_valid_native_denial_keeps_a_matching_sandbox_admitted_and_only_there` (2952) | gate/work positives beside the denial; `web_search` written in the authored command and in `hands.harness.work` | **passed** — the positives compiled (no native check existed) and the two written rows already refused as unqualified configuration |

The chief's observations were not re-used as this seat's executions; the
table is this seat's own run. After the repair the owning suite passed 39 of
39 (34 before plus the 5 new tests; `.forge/u2fix/post-repair.log`).
Two rows were added after the baseline, both positives within existing
tests: `clean native ON` and `clean native restriction` (moved into the row
table so a refusing mutation reaches them; see S-M9), and the untyped
control became an exact `outcome` comparison (see S-M12).

### Mutation ledger, unit 2-fix

Each mutation was a compiling edit to `bundle.rs` only, applied alone, run
with `cargo test -p brokkr-runtime --all-features --locked --lib --
bundle::agent_tests` (`.forge/u2fix/mut-*.log`), then restored from the byte
copy `.forge/u2fix/bundle.rs.repaired` and checked with `cmp`. These ran on
the repaired bytes before the coverage fold; the fold touches only the
grammar lookup, which no mutation edited, and the suite was rerun green on
the final bytes. "compiled" means the row's left was `compiled: [("review",
…), ("work", Some(LocalTools { … }))]` where its full refusal was expected.

| # | Mutation (bundle.rs) | Rows that failed (all others passed) |
| --- | --- | --- |
| A-M1 | root check `&& !(part == "authored command" && argv[node.at] == "--cd")` | root: A1 authored `--cd /`, authored `--cd .` — compiled |
| A-M2 | … authored `&& argv[node.at].starts_with("--cd=")` | root: A1 authored `--cd=/`, authored `--cd=<long>` — compiled |
| A-M3 | … authored `== "-C"` | root: A1 authored `-C /` — compiled |
| A-M4 | … authored `== "-C/"` | root: A1 authored `-C/` — compiled |
| A-M5 | … `part.starts_with("`hands.harness.work`")` and `== "--cd"` | root: A1 hands.harness.work `--cd /` — compiled |
| A-M6 | … work fragment `== "--cd=/"` | root: A1 hands.harness.work `--cd=/` — compiled |
| A-M7 | … work fragment `== "-C"` | root: A1 hands.harness.work `-C /` — compiled |
| A-M8 | … work fragment attached `-C` (`len() > 2`) | root: A1 hands.harness.work `-C/`, hands.harness.work `-C<long>` — compiled |
| N-M1 | root check skipped for `Native` argv holding `web_search="disabled"` | off: native OFF `--cd /`, `--cd=/`, `-C /`, `-C/`; links: inherited class — compiled |
| N-M2 | … `Native` argv holding `web_search="live"` without a restriction | on: native ON ×4 spellings — compiled |
| N-M3 | … `Native` argv holding the substituted restriction | on: native restriction ×4 spellings — compiled |
| S-M1 | `--add-dir` check `&& contribution == Written` | off: S1.1, `--add-dir` split at gate, `--add-dir=<long>`, `--add-dir` before the denial; on: native ON `--add-dir`; links: later candidate — compiled |
| S-M2 | switch check restricted to `Written` | off: S1.2 (`ReadOnly`), `--full-auto` — compiled |
| S-M3 | table check restricted to `Written` | off: S1.3 and sandbox_mode fail their exact assertion but do **not** compile — left is the native unqualified-`--config` refusal at argument 2 (defence in depth); recorded as not a wrong admission |
| S-M3b | S-M3 plus sandbox tables treated as established in `Native` | off: S1.3 (`WorkspaceWrite`), sandbox_mode (`ReadOnly`) — compiled |
| S-M4 | `break` out of the scan at the first native `web_search` assignment | 26 rows compiled: every native row whose competitor follows the denial (off 15, on 9, links 2); `--add-dir before the denial` still refused — binds all-occurrence scanning |
| S-M5 | native `--sandbox` refused only when its class differs | off: matching `--sandbox` at work and at gate — compiled |
| S-M6 | native allowance `config_under(&key, "web_search")` | off: `web_search.` descendant — compiled |
| S-M7 | every `Native` assignment established | off: unqualified config, descendant — compiled |
| S-M8 | allowance for every contribution (`\|\| key == "web_search"`) | denial: authored `web_search`, hands.harness.work `web_search` — compiled |
| S-M9 | native denial allowance removed (`&& key.is_empty()`) | denial: gate read-only and work workspace-write positives, left the native unqualified-`--config` refusal at argument 0; on: `clean native ON` and `clean native restriction` rows likewise (`mut-S-M9b.log`). Siblings: the refusal rows of the three native tests now fail on that earlier refusal, and the adopted positives in `a_typed_sandbox_admits_…`, `a_competing_control_…` and `a_seat_narrows_…` panic at their `unwrap` — recorded as siblings, not proof of more |
| S-M10 | `site.outcomes…take(1)` | links: later candidate — compiled |
| S-M11 | native check only when the seat itself writes `tools.sandbox` | 27 rows compiled (off 16, on 9, links 2) — every fixture's class is the office's, so reading only seat bytes admits them all, the inherited-class row included |
| S-M12 | effective class `.or(Some(WorkspaceWrite))` for untyped sites | links: untyped control — left the native `--cd` refusal, right `compiled: … allow: Some(["cargo"]), sandbox: None` |
| P-M1 | open-gate refusal arm `&& what.is_empty()` | root: open-gate precedence row — left the typed `open` boundary refusal; sibling: the adopted `open gate keeps its standing refusal` row of `a_typed_sandbox_admits_…` |

Row keys: *root* = `a_root_selector_…`, *off* =
`a_resolved_native_off_contribution_…`, *on* =
`resolved_native_on_and_restriction_…`, *links* =
`resolved_native_admission_judges_…`, *denial* =
`a_valid_native_denial_…`. After the last mutation `cmp` matched the saved
copy and `git status` showed only `bundle.rs` and `bundle/agent_tests.rs`
modified.

### Audit (2.6)

All eleven chief cases (S1.1–S1.3, A1.1–A1.8), the twelve native root rows
(ON, OFF, restriction × four spellings) and every other new row above have
a recorded baseline, an exact complete literal expectation written in the
test (helpers `root_refusal`, `competing`, `switch_cause`, `table_cause`,
`ADDED_ROOT_CAUSE`, `native_config_cause`, `native_sandbox_refusal`, none
derived from production), a compiling mutation reaching that row with its
actual left, and the restored pass. No `is_err()`, substring or
`unwrap_err` assertion was added. The long non-ASCII/newline payload rows
assert the identical value-free sentence. Every new fixture uses the
canonical `AgentFixture` root (the realm grant writes its definition and
dialect under it and names it as the operator root); no test reads
`.forge/` or needs an installed provider. The positive facts asserted are
the exact local value, the selected gate/work fragment, an empty
`hands_fragment`, the `harness` boundary, empty holdings, web-search OFF
and the full resolved denial argv `["-c", "web_search=\"disabled\""]`; for
the grant, the holding, ON and `["-c", "web_search=\"live\""]`; for the
restriction, the substituted `web_search={"allow":{"hosts":["example.org"]}}`.
The synthetic restriction transport qualifies no provider support (unit 9)
and these compile comparisons prove no launch (units 13–15).

### Gates on the restored tree (2.7)

All on `07228d50` plus this working tree (final bytes, after every mutation
was restored):

- `cargo fmt --all -- --check`: **passed**.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D
  warnings`: **passed** (after one `type_complexity` fix in the new test).
- `cargo test -p brokkr-runtime --all-features --locked`: **passed**, 534
  library tests, 25 green result lines (`.forge/u2fix/gate-runtime.log`).
- `cargo test --workspace`: **passed**, 77 green result lines, no failure.
- `cargo test --workspace --all-features --locked`: **passed**, 77 green
  result lines, no failure.
- `cargo run --locked -p brokkr-cli -- compile --bundle bundles/self` and
  `bundles/verify`: both **compiled**.
- `openspec validate --all --strict --no-interactive`: **passed, 18/18**
  (informational length notices only).
- `git diff --check`: **passed**. Frozen paths (`policy/`, `contracts/`,
  `fixtures/`, `reference/`, `extensions/`): no diff. No pin, shipped data,
  grant or other file moved.
- Coverage **diagnostic**, not the gate: `cargo +nightly-2026-09-05
  llvm-cov clean --workspace`, then `llvm-cov -p brokkr-runtime
  --all-features --locked --lib --branch --lcov`; in `bundle.rs` no `DA` or
  `BRDA` record in the changed ranges is zero, and `admit_native_sandbox`
  is hit (45). A first run without `clean` merged stale profiles whose line
  numbers matched no current source and was discarded.
- External exact coverage (`bash scripts/coverage-exact.sh`, workspace-wide,
  from a capable host or CI), macOS and remote CI: **pending**, not observed
  here. `ci.yml`, `release.yml` and `coverage-exact.sh` all consume
  `rust-nightly-version.txt`. Nothing is called fully green on their
  account; unit 1's pending results stay pending.
