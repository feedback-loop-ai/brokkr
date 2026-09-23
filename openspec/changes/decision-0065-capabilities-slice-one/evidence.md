# Decision 0065, slice one — evidence after the operator ruling

## Current status — documentation revision, 2026-09-23

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
