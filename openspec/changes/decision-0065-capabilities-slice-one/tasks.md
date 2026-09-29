# Decision 0065, slice one — ordered rebuild tasks

Adopt every commit through `44430402`, specification draft `a84197cd` and design
revision `3c5402be` on `slice-0065-capabilities`. The [operator ruling](operator-ruling-2026-09-23.md)
is authoritative: “Nothing is merged”; outward input “is not pinned and admitted.”
This tasks visit changes documentation only and leaves decision 0066 proposed.

Execute groups **1–27 from top to bottom**, one visit per group, matching the
single [design Rebuild units](design.md#rebuild-units) order. Each group inherits
the preceding committed result; its named files, production-file ceiling and
proof scope are in that design unit. Group 0 retains adopted work, not work to
redo. Group 28 is the later council/archive dependency, outside this commission.
Task numbers now follow execution order; `previous` records the ID in 3c5402be
so earlier councils and evidence remain traceable. Cross-unit checks close only
at their last dependency. Every task cites its requirement by name.

Reopened tasks cite the operator ruling; R10/R12 and V1/R13 are third-council
proof gaps. Checked foundations retain only the historical scope indexed in
[evidence.md](evidence.md#adopted-foundations), not a blanket launch-safety claim.
Each implementation unit records baseline red before fixes and its independent
removal/restored results in the owning suite and evidence. Use production-compiled
fixtures, independent full literals and one retained canonical Linux/macOS root.
Keep grants empty, frozen bytes unchanged and every mutation restored; never push.
Run the applicable house gates per unit, commit its work and tick only finished
tasks. External exact coverage and CI must name the final source head; pending
is not green. No archive occurs until the final conditional task is authorized.

## 0. Adopted artifacts and retained foundations

- [x] 0.1 Adopt the specification/design amendments through 3c5402be, retain proposed 0066 and all four rulings, and order this breakdown by the design units. Verify every requirement has scenarios and a named task, with preserved historical evidence and reopened superseded claims. Documentation only; record actual documentation gate outcomes separately, including unavailable commands. Requirements: [Authored provider configuration cannot supply capability authority][RGR], [Every accepted native control reaches the final command][NCC], [Active instructions and policy cannot escape bundle identity][MPI], [Installed native capabilities absent from grants are explicit][CD2], [Digest pins are measured and their history remains truthful][MP5]. (previous 0.1)

- [x] 0.2 Add `contracts/tool-dialect.v1.schema.json` with D3's closed discriminated shape, identity/serves/tools, provider-native binding, reserved workspace hands, abstract-class equality annotation and separate disclosure metadata. Verify positive native/hands examples and schema rejection of unknown/mixed kinds, missing binding fields, invalid classes and egress vocabulary. Requirements: [A tool dialect binds one capability to exactly one implementation kind][TD1]; [Dialects describe their disclosure using the existing egress vocabulary][TD3]; [Reserved hands preserves the existing workspace authority][TD6]. Retained evidence: evidence.md, Adopted foundations (historical scope only). (previous 1.1)

- [x] 0.3 Complete that new tool-dialect contract's MCP branch with exactly one stdio argv or URL connection, named tools, pinned version, 0012 binding names and optional retention declaration; define the embedded Draft 7 restriction schema field. Verify both connection forms as data and schema rejection of mixed/incomplete forms and literal credential fields, without server/network access. Task 0.9 completes semantic binding/reference checks. Requirements: [The MCP contract is whole before its runtime exists][TD4]; [Restriction keys are defined by the selected dialect][TD5]. Retained evidence: evidence.md, Adopted foundations (historical scope only). (previous 1.2)

- [x] 0.4 Add `contracts/realms.v6.schema.json`, preserving v5 fields and admitting the per-realm grants map with optional tool/office lists and dialect-owned restriction values. Verify valid omission/empty maps and lists, invalid nulls/types/duplicates, and schema refusal of `capabilities` under every legacy version. Requirement: [Realms v6 adds grants without changing frozen versions][RG1]. Retained evidence: evidence.md, Adopted foundations (historical scope only). (previous 1.3)

- [x] 0.5 Add `contracts/run-manifest.v11.schema.json` with the adopted structured capability section, explicit empty outcomes, per-candidate records, relative sources/digests and inactive grant context. Register all three new contracts additively in `crates/brokkr-runtime/tests/frozen_contracts.rs`; verify hand-built positive/negative v11 examples and unchanged hashes for every existing version. Requirement: [A new manifest version records capabilities per executable seat][MP1]. Retained evidence: evidence.md, Adopted foundations (historical scope only). (previous 1.4)

- [x] 0.6 Extend `brokkr-core/src/realms.rs` and runtime realm representation for v6 grants, preserving omitted versus explicit tool/scope lists. Check field presence before defaults so v1–v5 reject even `capabilities: {}`. Verify core realm tests for all six versions, absent map/field, empty maps, malformed data and unchanged hands/boundary fields. Requirements: [Realms v6 adds grants without changing frozen versions][RG1]; [Office scopes and tool subsets only narrow a grant][RG3]. Retained evidence: evidence.md, Adopted foundations (historical scope only). (previous 2.1)

- [x] 0.7 Introduce D2's explicit `CapabilityContext` and thread it through CLI compile/run/rerun, runtime compile entry points and semantic library lint. Resolve definition/dialect roots beside the active realm map, otherwise under the operated repository; convenience APIs must construct an explicit no-grant context. Verify mapped/unmapped and neighboring-realm context tests, and that recipe roots and agent-library overrides cannot replace operator authority. The retained grant/intersection/floor work connects authorization to every entry point. Requirements: [Realm context is resolved before capability authorization][RG5]; [Only an operator's realm selects concrete implementations][RG2]. Retained evidence: evidence.md, Adopted foundations (historical scope only). (previous 2.2)

- [x] 0.8 Implement the strict abstract-definition loader in a focused runtime `capabilities` module: safe names, contained paths including symlinks, filename/name equality, exactly name/classes, nonempty unique classes and deterministic duplicate/conflict rejection. Read/hash the same bytes and validate every definition in the root. Verify CQ2's valid operator-defined name, missing roots, absent/conflicting metadata, recipe-local impersonation and class-order equality with exact owning diagnostics. Requirement: [Capability classes belong to the abstraction][TD2]. Retained evidence: evidence.md, Adopted foundations (historical scope only). (previous 2.3)

- [x] 0.9 Implement strict selected-dialect loading and restriction validation, including containment, duplicate-key rejection in new authority data, serves/class agreement and same-buffer SHA-256. Promote the already locked `jsonschema` workspace dependency from runtime dev-only to production with default features disabled, for complete operator-schema validation; permit only local embedded Draft 7 references, reject external retrieval and reserved-key redefinition through references/composition. Verify valid/invalid schemas, unchanged values/array order, missing/escaping references and schema/loader agreement. Check MCP undeclared secret references and URL userinfo without echoing credentials or opening a secret store. Confirm no package/version upgrade and record the dependency-edge justification. Requirements: [A tool dialect binds one capability to exactly one implementation kind][TD1]; [Restriction keys are defined by the selected dialect][TD5]; [Only an operator's realm selects concrete implementations][RG2]; [The MCP contract is whole before its runtime exists][TD4]. Retained evidence: evidence.md, Adopted foundations (historical scope only). (previous 2.4)

- [x] 0.10 **M2:** Strictly parse each original recipe layer in `bundle/compose.rs::read_layers` and agent source in `agents/load.rs::read_json` with `parse_strict` before map conversion. Refuse duplicate capability names and outer `capabilities` fields, both strength orders, equal repetitions and a later `{}`. Cover direct, nested panel, sequence, selected-case and inherited/overridden layers with raw JSON; assert full source/key/location diagnostics and valid inheritance/subtraction controls. Keep errors at the original source. Independently remove each reader's strict parsing, observe its intended equality fail, restore and pass. Requirements: [Requests use abstract names and a closed strength vocabulary][SC1]; [Refusal proofs assert the full reason][SC8]. Retained evidence: evidence.md, Adopted foundations (historical scope only). (previous 3.1)

- [x] 0.11 Preserve omission versus explicit request maps: agent-backed omission inherits, an explicit unchanged-strength subset subtracts, `{}` removes all, and inline maps supply office asks. Verify addition/strength-change refusals and inherited required subtraction across ordinary, panel, sequence, selected and composed bodies; retain both stable office and execution-site identities. Requirement: [A seat can subtract but cannot widen its office][SC2]. Retained evidence: evidence.md, Adopted foundations (historical scope only). (previous 3.2)

- [x] 0.12 Ship operator `capabilities/web-search.json` and `web-fetch.json` as reads/egress definitions and only the three adopted provider-native tool dialects: Codex search, Claude search and Claude fetch. Add Codex's measured cold-default ON and exact OFF pair, Claude's declared search/fetch selection controls, and DSH/LaneTally's explicit commissioned unmeasured reasons; describe generic exec's unknown child inventory honestly. Shipped dialect restriction schemas admit only empty objects until a control exists. Verify library-data tests load each binding, tools/keys agree, evidence is scoped, and repository realms still grant nothing. Requirements: [Capability classes belong to the abstraction][TD2]; [Codex web-search OFF uses the controller's measured fragment][NC2]; [Other provider declarations preserve the commissioned uncertainty][NC5]. Retained evidence: evidence.md, Adopted foundations (historical scope only). (previous 3.4)

- [x] 0.13 Migrate `agents/researcher.json` to abstract web-search/web-fetch wants, retain local command restrictions and remove concrete web aliases; add the DATA-only sentence beside capability use in its charter and any other affected capability-using charter. Refuse nonempty legacy `tools.mcp`, even optional entries, and native-owned legacy allow aliases with full migration reasons; keep empty MCP lists valid. Verify strict library and charter tests, local restrictions unchanged and declarations contain no implementation choices. Requirements: [Legacy concrete permissions cannot grandfather a capability][SC7]; [Capability responses are data and confer no authority][MP4]. Retained evidence: evidence.md, Adopted foundations (historical scope only). (previous 3.5)

- [x] 0.14 **M3:** Immediately after `Bundle::assemble` actually loads a library, call existing `authority.definitions.lint(&library)` before seat resolution/subtraction; return deterministic `CompileError::Capability` agent diagnostics. Resolve every loaded agent's requires/wants, including unseated and later-subtracted asks, without loading otherwise unused libraries. Add those definition names to the manifest's consulted set. Prove the exact CQ2 agent-lint error with a valid seated worker plus invalid unseated researcher, direct/composed cases, valid-library control and retained CLI lint. Remove only the compile invocation, observe its named regression fail, restore and pass. The consulted-definition proof survives in evidence; owner-bound charter work remains in unit 17. Requirements: [Capability classes belong to the abstraction][TD2]; [Requests use abstract names and a closed strength vocabulary][SC1]; [Capability authorization participates in bundle identity][MP2]. Retained evidence: evidence.md, Adopted foundations (historical scope only). (previous 3.7)

- [x] 0.15 **First H2 prerequisite (retained):** At `agents::compose`, record the appended hands-fragment length; `Candidate::parts()` uses that captured boundary to recover the base and managed vectors. This observed positional provenance is the adopted implementation of the original separate-vector design. Inline vectors remain wholly authored. Preserve origin through candidate construction; do not reconstruct it by matching/deleting strings or recognizing a server name. Test identical-looking authored and engine fragments retain distinct origin while existing hands/model commands remain unchanged. This proves the historical hands boundary only; expanded template/local origins remain open in 4.2. Requirements: [Authored provider configuration cannot supply capability authority][RGR]; [Reserved hands preserves the existing workspace authority][TD6]. Retained evidence: evidence.md, Adopted foundations (historical scope only). (previous 3.8)

- [x] 0.16 Validate every selected-realm grant before seat compatibility: definition/dialect existence, serves/classes, tool subset, offices and restriction schema. Then refuse MCP grants with the complete slice-two diagnostic even for wants, no requests or `offices: []`, and refuse reserved hands with the 0043/0046 reason. Verify exact diagnostics and positive native controls without server launch; invalid data cannot become an optional drop. Requirements: [Any MCP realm grant refuses until slice two][SC6]; [Reserved hands preserves the existing workspace authority][TD6]; [Restrictions are validated, carried and pinned without engine interpretation][RG4]. Retained evidence: evidence.md, Adopted foundations (historical scope only). (previous 4.1)

- [x] 0.17 Implement the pure office-asks-minus-subtractions intersection with scoped realm grants. Resolve omitted versus empty scopes/tools exactly; produce named required refusals and deterministic complete wanted-drop notices, never enable an unused grant. Verify missing grant, wrong office, no tools, subtracted and no-ask cases from positive examples, including the exact D4 missing-grant reason. Requirements: [Compilation holds only the request and applicable grant intersection][SC3]; [Office scopes and tool subsets only narrow a grant][RG3]. Retained evidence: evidence.md, Adopted foundations (historical scope only). (previous 4.2)

- [x] 0.18 **H1:** Replace security-relevant best-effort adapter loading with a preserved `Result` shared by pin and capability resolution. Separate optional effort exemptions from mandatory denial; audit suppressed errors affecting authorization, including `load_pin_adapters`, `site_capabilities`, `native_plan` and retry consumers. Apply D4's known-power floor (Codex search; Claude search/fetch), plus declared powers, without embedded switches or borrowed DSH/LaneTally inventories. Refuse absent roots/providers, legacy/omitted/empty metadata, omitted known powers, malformed/unreadable or unrelated broken adapter files and unmeasured known OFF when no valid denial exists. Assert whole source/load causes with site/office/realm/provider/capability for no-ask/no-grant inline work, no map, v1–v5 and v6 empty/omitted grants, absent definitions, scope, subtraction and optional drops; positive valid metadata delivers OFF. The new full composition check is separately open in 12.2; this retained row proves the load-error/floor protection only. Requirements: [Known native powers require a valid delivered denial or refusal][NCR]; [An impossible native denial is a compile refusal][SC5]; [Realms v6 adds grants without changing frozen versions][RG1]; [Other provider declarations preserve the commissioned uncertainty][NC5]. Retained evidence: evidence.md, Adopted foundations (historical scope only). (previous 4.5)

- [x] 0.19 Retain the full MCP-unbuilt doctor report with its capability/dialect and independent realm/inventory lines after the shared-assessment refactor. Deterministic installed/absent fixtures must prove no capability server or model request is invoked; invalid authority stays distinct from empty grants. Requirements: [Doctor explains unbuilt bindings without running capabilities][CD4]; [Doctor reports grants for every realm][CD1]; [Any MCP realm grant refuses until slice two][SC6]. Retained evidence: evidence.md, Adopted foundations (historical scope only). (previous 8.5)

- [x] 0.20 **M4:** Run the two adopted separately runnable wants-only tests. Remove provider compatibility itself, then restriction compatibility itself in a separate experiment; leave notice recording/OFF composition intact and use otherwise-valid fixtures. Each optional test must reach and fail its whole expected notice-vector equality before any required-refusal or other substantive assertion. Restore each check and record the pass. Required and unused tests stay independent. Also rerun existing grant/scope/impossible-OFF/notice/MCP removals and grant/scope-deleted positive controls, with exact reasons/notices including optional/unused MCP. Record each mutation, test, intended assertion, actual failure and restored pass; historical M3/M4 required failures, fallback holdings and M6/M8 notice/OFF removals are not these optional proofs. Requirements: [Provider compatibility cannot expand a holding][SC4]; [Restrictions are validated, carried and pinned without engine interpretation][RG4]; [Refusal proofs assert the full reason][SC8]; [Compilation holds only the request and applicable grant intersection][SC3]; [An impossible native denial is a compile refusal][SC5]; [Any MCP realm grant refuses until slice two][SC6]. Retained evidence: evidence.md, Adopted foundations (historical scope only). (previous 9.1)

## 1. Unit 1 — Rebase onto current origin/main

- [ ] 1.1 Unit 1 rebases onto current origin/main retaining every adopted commit and seven main commits. Resolve inventoried conflicts, measure witness/compose pins and keep all gates green. Verify SHAs/replay mapping/results; pending is not green. Requirements: [Digest pins are measured and their history remains truthful][MP5], [Refusal proofs assert the full reason][SC8]. Reopened/remaining: operator ruling 1–4 / R10. (previous 0.4) Observed 2026-09-23 (evidence.md, Unit 1): all 45 commits replayed onto 072cdd9b; the three adapter JSON files auto-merged with main's roster kept; one production conflict outside the inventory (`adapters.rs` `dsh_launch_with`, both guards kept); twelve pins measured; fmt, clippy, the workspace tests (2412 passed) and both bundle compiles passed. `openspec validate` was not run (seat permission). External exact coverage, macOS and remote CI are pending and not green. Reopened 2026-09-23 after review (evidence.md, "Review return"): C1, completion waits for the pending external results; C2, the `adapters.rs` resolution is a fourth production conflict, resolved without the split the unit requires. No test binds its guard order (an order-swap mutation left the full `brokkr-protocol` suite green). The inventory (X1–X4) and the split request are recorded. Open until the split is ruled and those results exist.

- [x] 1.2 Unit 1b binds the guard order in `dsh_launch_with` (X1, the inventoried split of unit 1). Record the operator's addendum as one native-control requirement with a scenario; build the case that tells the orders apart (a native control beside a boundary-faulted argv, exact reasons) and a positive both guards admit; prove it with an order-swap mutation. Change production code only if the test shows it differs from the ruling. Requirements: [A DSH launch refuses authority before it reads its boundary][NCD], [Refusal proofs assert the full reason][SC8]. New in unit 1b: operator ruling 2026-09-23, addendum. Observed 2026-09-23 (evidence.md, "Unit 1b"): the current order is the ruled one, so no code moved (the `dsh_launch_with` comment now cites the addendum). `a_dsh_native_control_is_refused_before_the_boundary_check_reads_the_argv` passes; under the order swap its managed-arguments assertion fails with the boundary's reason; restored and green. fmt, clippy, the `brokkr-protocol` suite, the self bundle compile, the witness pins (unchanged) and `git diff --check` passed. `openspec validate` was not run (seat permission). Unit 1's external gates stay with 1.1.

## 2. Unit 2 — Decode typed inline declarations

Original tasks-phase baseline: `4a6a2288e2825c29e23ec9515e66613fbf53462c`, on
`slice-0065-capabilities`, 2026-09-23. Adopt every preceding commit, including
unit 1b, specification `d9816374` and design `4a6a2288`; replay nothing.
There is no `returned_from`. D5.1–D5.4 answer this unit's design questions;
Open Questions for unit 9 and external gates retain their separate ownership.

Specification re-adoption, run `build-decision-0065-slice-one-re-e332dc8d`
(2026-09-23), adopts `d9816374`, `4a6a2288`, `308a8a28` and `f9a691cf`
without replay or replacement. That visit retained the proposal, deltas and D5 design. The implementation
remains adopted, but the evidence audit below
reopens **only the incomplete proof portions** of 2.1.1–2.1.6 and therefore
2.1: the original baseline reds were not recorded, the mutation ledger does
not bind every independently claimed row, one new bundle test explicitly has
no mutation, and some narrowing rows assert only a field instead of the full
cause. Preserve existing implementation and proofs; repair those gaps in the
two owning suites and ledger without reimplementing the unit. Evidence.md's
“Unit 2 — specification re-adoption and proof correction” names the findings.
Do not invent pre-repair history or substitute a first-row failure for later
rows. If a required proof cannot fit the existing file allowlist, stop and
inventory the needed split before touching another file.

The specification visit retained checked 2.1.7 for the prior observed local
gates and could not run fresh Rust checks because cargo was absent in that
seat. Those are historical observations, not this seat's environment status.

Council design follow-up, the same run at `368bc34e`: D5.5 reconciles both
positions and adopts robustness's demonstrated D5.3 admission gap. Alongside
the proof corrections above, **2.1.5 now requires a production repair**:
matching typed sandbox admission currently overlooks full-auto, sandbox bypass
and sandbox_workspace_write configuration. Judge both selected engine and
authored contributions through the existing grammar in bundle.rs, retaining
supported hands/effort configuration and refusal precedence. Record baseline
reds, independent removals/restored passes in bundle/agent_tests.rs and the
ledger. The specification's no-competing-control scenario already owns this;
no new specification or broader flag campaign is needed. Reopen 2.1.7 for
fresh final-restored checks after the repair; preserve every historical pass.
External exact coverage, macOS and remote CI remain pending.

Tasks re-adoption, the same run, phase `tasks`, starts at
`107c4d179fc0a5807ade3d4f3628d8c8ec77a7c5` (2026-09-23). Retain the four
commissioned commits plus proof correction `368bc34e` and council amendment
`107c4d17`. The remaining work below follows D5.5; it does not restart the
original implementation. There is no `returned_from`. Every unit 2 checkbox
remains open for the recorded incomplete portion. Sufficient existing proof
is retained by test/row and revision, not rerun merely to rewrite the ledger.
Missing original pre-implementation reds remain missing historical evidence;
a later reconstruction is retrospective, never an original observation.

Production paths below are under `crates/brokkr-runtime/src/`: only
`agents.rs`, `agents/load.rs` and `bundle.rs`. Tests stay in `agents/tests.rs`
and `bundle/agent_tests.rs`. Implementation observations belong in this file
and evidence.md. This tasks visit edits only those two artifacts; the adopted
D5.5, proposal and deltas already settle the work. A fourth production file,
another suite or a measured pin edit requires an inventoried split **before**
editing. No new module,
Candidate field, dependency, public contract, shipped JSON, grant or frozen
byte belongs to unit 2. Decision 0066 stays proposed.

- [ ] 2.1 Complete D5 typed declaration decoding and strict local admission in the three named production files, with all ordered substeps 2.1.1–2.1.7 below finished. Tick this aggregate only after its implementation, exact proofs, applicable observed gates and commit are recorded; pending external results stay pending and do not establish fully green completion. This remains the accepted Rebuild units closure anchor, not a new unit. Requirements: [Legacy concrete permissions cannot grandfather a capability][SC7], [Shipped inline permissions migrate before refusal lands][SCM], [Authored provider configuration cannot supply capability authority][RGR], [Refusal proofs assert the full reason][SC8]. Reopened/remaining: operator rulings 1–2. (previous 3.14)

Execute the following substeps in order within the next implementation visit.
For each remaining behavioral repair, establish the baseline assertion on the
adopted implementation before changing it, then independently mutate, restore
and rerun its proof before advancing. Already-correct cases start with an
observed pass; never manufacture a baseline red. Substep 2.1.6 audits those
records; it does not defer all mutation work to the end.

- [x] 2.1.1 Inventory the remaining proof before any production repair. Compare the seven new agent tests and ten new bundle tests from f9a691cf (plus the amended loader table) with the existing ledger, naming each independent row, requirement/scenario, exact expected value or complete cause, sufficient retained experiment and missing proof. Preserve both canonical retained fixture roots and contained inline role bytes; do not redo that implementation. Run the relevant current baselines in the two owning suites. Record existing decoder/empty/omission/placement/hands successes as passes; empty-agent-allow rejection, absent sandbox decoding and inline-tools key rejection describe the pre-f9a691cf implementation, not today's baseline. Keep the missing original reds explicitly missing; label any 308a8a28 reconstruction retrospective. Identify the D5.5 competing-control regressions to add in 2.1.5 and run each against the adopted guard before its repair; a compiler error, unrelated fixture failure or provider/.forge dependency cannot count. Requirements: [Refusal proofs assert the full reason][SC8], [Shipped inline permissions migrate before refusal lands][SCM]. (substep of previous 3.14)

- [x] 2.1.2 Retain the adopted shared strict decoder and pure narrowing in `agents.rs` / `agents/load.rs`; complete only missing exact assertions and independent proofs in `agents/tests.rs`. Keep the sole `Agent.allow`, existing three-case sandbox enum, two-field local value and narrow `parse_tools` exposure. Accept only allow/sandbox/absent-or-exactly-empty MCP; preserve ordered `Some([])`, omitted fields and all three exact sandbox values. Narrow each field independently: omitted tools, `{}` or a missing sibling inherits; explicit allow retains written order and must be a subset of any office list; unspecified office fields may be restricted; an empty office list cannot widen; sandbox reach is read-only <= workspace-write <= danger-full-access, with no clamping. Verify in `agents/tests.rs` complete values and causes for null/non-object tools, unknown keys, null/non-array allow, non-string/duplicate/malformed names (including raw harness patterns), null/non-string/unknown sandbox, malformed/nonempty MCP, every sandbox comparison, independent field widenings and partial inheritance. From original raw agent JSON, independently verify repeated tools/allow/sandbox keys, including equal repeats; retain `read_request_source` strictness. Replace the field-only `.unwrap_err().0` assertions in `narrowing_inherits_per_field_and_refuses_each_widening_exactly` with complete independent `(field, cause)` expectations for both valid-sibling/invalid-field cases and every refusing sandbox pair. Preserve sufficient existing mutations and add a compiling mutation/restored pass for every still-unbound new test or independently claimed row. Change production only if an exact assertion reveals a defect. Requirements: [Shipped inline permissions migrate before refusal lands][SCM], [Legacy concrete permissions cannot grandfather a capability][SC7], [Refusal proofs assert the full reason][SC8]. (substep of previous 3.14)

- [x] 2.1.3 Retain the adopted effective private-office clone before composition in `agents.rs`, keeping source/digest and Candidate unchanged; fill only missing assertions or independent proofs in `agents/tests.rs`, repairing production only for a demonstrated defect. Validate the whole candidate chain, including unavailable later candidates, while retaining existing hands/model-policy refusal precedence. Where direct tools apply, use each candidate's exact `ToolPermissions.names` values and `native.capability_of`; unknown mappings and arbitrary mapped native aliases refuse with full agent/provider/model/field causes. Preserve hands replacement: syntax/subset checks still apply, known mapped native aliases still refuse, but dormant direct lists need no otherwise-unused direct mapping and empty does not disable hands. Direct explicit empty stays unrepresentable until lowering exists. Verify in `agents/tests.rs` exact ordered mapped argv, unchanged hands fragments and declaration values, independent unknown/native/fallback-only refusals, and that optional wants do not forgive local errors; mutate each enforcement independently and restore. Requirements: [Shipped inline permissions migrate before refusal lands][SCM], [Legacy concrete permissions cannot grandfather a capability][SC7], [Reserved hands preserves the existing workspace authority][TD6], [Refusal proofs assert the full reason][SC8]. (substep of previous 3.14)

- [x] 2.1.4 Retain the adopted shared decoding, effective office narrowing and checked local facts through existing executable paths in `bundle.rs`; fill only missing exact assertions and independent proofs in `bundle/agent_tests.rs`. Preserve tools in SEAT_KEYS/BODY_KEYS/MEMBER_KEYS/STEP_KEYS and its explicit refusal, even `{}`, on panel/sequence/selection containers, panel-valued steps and dialect steps without executable checks. An exec-generated check must refuse nonempty local fields it cannot represent. Validate shape/placement before representation; obtain required adapter data through the existing fallible context and extend `needs_adapters` without opening an unused agent library or swallowing load failures. Store `Some(local value)` for every visited executable, including two unspecified fields; leave unvisited facts distinguishable and relocate the entire fact with wrappers. Verify `bundle/agent_tests.rs` ordinary seats, panel members, sequence steps, every selected case/default and inherited bodies, each with omission/subset/empty/malformed/widening cases and complete owning-site causes. Bind every raw duplicate tools/allow/sandbox row in `repeated_tools_keys_in_a_bundle_refuse_from_the_original_source` through the existing `compose::resolve(&dir)?` caller in `bundle.rs` or a valid existing test seam; do not edit `bundle/compose.rs`, add a parser, or count an unrelated failure as removal of strict-source enforcement. If no compiling in-scope enforcement mutation works, inventory the exact split and stop before widening. Verify missing/malformed adapter-context refusal and two sites sharing one office in both traversal orders with exact isolated fields and unchanged office source/digest. Independently mutate each path/check in the named files and restore. Requirements: [Shipped inline permissions migrate before refusal lands][SCM], [Legacy concrete permissions cannot grandfather a capability][SC7], [Refusal proofs assert the full reason][SC8]. (substep of previous 3.14)

- [x] 2.1.5 Repair D5.5's demonstrated competing-control gap in `bundle.rs::expressed_sandbox` and bind the remaining D5.3 admission rows in the owning suites; retain the rest of the adopted intermediate admission. Keep unspecified defaults and existing mapped nonempty agent-direct/hands paths; refuse inline direct allow, direct empty and unsupported sandbox lowering with the exact owning-field/representation cause. After independent constitutional checks, admit explicit sandbox only for actual Codex dispatch with hands and exactly matching existing engine fragments: harness gate read-only, harness work workspace-write, boxed read-only. Use the existing public protocol grammar to require exactly one sandbox control of the requested class and refuse competing controls or opaque configuration in both the selected fragment and other contributions; authored bytes cannot supply representation. Before repairing the guard, add independent cases pairing each of `--full-auto`, `--dangerously-bypass-approvals-and-sandbox`, configuration under `sandbox_mode`, and configuration under `sandbox_workspace_write` (including the demonstrated `sandbox_workspace_write.network_access=true`) with both authored argv and the selected engine fragment. Use a matching requested class so a mismatch cannot hide the competing control. The sandbox_mode neighbor already refuses: record its actual baseline, while each newly exposed admission must fail the exact full-refusal assertion. Reuse parsed canonical option identities and config paths; no new grammar or blanket `-c` refusal. Retain exact positive hands/effort configurations and boxed/gate/work selections. For each missing proof, independently remove the corresponding guard or contribution check, observe that row's intended full assertion failure, restore and pass; an already-passing neighbor or first failing loop row proves no later case. Refuse each nonmatching class, missing fragment, open work, no-hands/inline sandbox, opaque dispatch labelled codex, and Claude/LaneTally/DSH/exec sandbox. Verify every D5.3 row independently in the owning suites, with exact effective fields, selected fragments, boundary and complete refusals; prove existing open-gate/missing-hands/model-policy errors keep precedence and later candidates cannot hide a mismatch. Under empty realm grants, compare full holdings/native OFF facts for admitted fixtures and confirm hands policy/boundary stay unchanged. Mutate each guard or selected-fragment check independently, observe the intended exact assertion fail, restore and pass. These are compile-admission proofs; no final-launch or live-provider claim follows. Requirements: [Shipped inline permissions migrate before refusal lands][SCM], [Authored provider configuration cannot supply capability authority][RGR], [Reserved hands preserves the existing workspace authority][TD6], [Known native powers require a valid delivered denial or refusal][NCR], [Refusal proofs assert the full reason][SC8]. (substep of previous 3.14)

- [x] 2.1.6 Audit the accumulated baseline/fix/removal/restoration ledger in evidence.md against every new test and each independently claimed row in the two owning suites. Record the revision, exact test and row, command, compiling in-scope mutation, actual left/right or full failing assertion, restoration and passing rerun. Separate retained historical proof, newly observed baseline passes/reds, and any retrospective reconstruction; do not relabel missing original baseline history as completed. A table aborting at its first row proves no later row; isolate mutations/filters so each claimed row reaches its assertion. Reject `is_err()`, substring-only reasons, production-derived expected output, compilation errors or unrelated fixture failures as proof. Confirm every mutation is restored and all new fixtures retain one canonical temporary root, with no `.forge/` reads or installed provider. Verify coverage against SCM's unit 2 scenarios and SC7/SC8, distinguishing later migration/final-command scenarios that remain open. Requirements: [Refusal proofs assert the full reason][SC8], [Shipped inline permissions migrate before refusal lands][SCM], [Legacy concrete permissions cannot grandfather a capability][SC7]. (substep of previous 3.14)

- [ ] 2.1.7 Run the local gates below on restored work, record their actual results and exact external-evidence status in evidence.md, and inspect the diff against this unit's file allowlist. Stop for an inventoried split if gates expose a necessary out-of-scope edit; do not re-pin another suite or repair a later unit here. Only tick completed work; commit tasks/evidence with the implementation in repository style, never push. Verify the committed SHA, clean worktree and retained ancestry; record the committed-head result in the run-local result so recording it does not move the validated source head. External exact coverage and relevant Linux/macOS/CI evidence must identify their tested head; unavailable/pending gates cannot be called green or close unit 1's pending results. Requirements: [Refusal proofs assert the full reason][SC8], [Digest pins are measured and their history remains truthful][MP5], [Shipped inline permissions migrate before refusal lands][SCM]. (substep of previous 3.14)

Implementation observations, the same run, phase `implement`, based on
`0444421d` (2026-09-23), recorded in full under evidence.md "Unit 2 — proof
repair and the D5.5 guard". The adopted implementation was checked against
the unit and kept; the one production edit is `bundle.rs::expressed_sandbox`,
which now refuses the two codex sandbox switches and configuration under
`sandbox_workspace_write` beside `sandbox_mode`, in the selected fragment and
in the authored command alike. The sixteen-row regression was observed red
on the adopted guard first (12 of 16 rows compiled; the four `sandbox_mode`
rows were baseline passes), then green. Every table in the two owning suites
now reaches every row, the field-only narrowing assertions are complete
`(field, cause)` expectations, the strict raw-bundle test is bound through
the in-scope `compose::resolve(&dir)?` caller, and every claimed row has a
named compiling mutation with its observed failure and restored pass (ledger
rows A1–L2). Fresh local gates passed on the restored tree as listed in
evidence.md; external exact coverage, macOS and remote CI remain pending and
the aggregate is ticked on that stated basis, not as fully green. The
committed SHA is recorded in the run-local result.

Review return, the same run, phase `implement` based on `ca2c9156`
(2026-09-23), recorded in full under evidence.md "Unit 2 — review return,
seven chief findings answered". The chief (gpt-6-astra, `residual`, medium
security) found the admission guard trusting adapter data for the class
(F1), ignoring grammar-typed loads and unestablished configuration (F2), the
generated validator holding no local fact (F3), ledger claims without a
binding mutation (F4), matrices short of the commissioned rows (F5) and an
outward fixture link (F6). All are answered in `bundle.rs` and the two
owning suites: `admitted_sandbox` is D5.3's table keyed on the path alone
and checked independently of the matching fragment; `expressed_sandbox`
refuses `--profile` and any `-c` assignment outside `mcp_servers.brokkr.*`
and `model_reasoning_effort`, naming only the argument position; the
dialect validator records the checked-unspecified value; the executable-forms
table carries omission and inherited-body rows (27), a nested inline-forms
table is new (28), the admission table carries the six table rows, four
opaque rows, one established positive and the LaneTally/DSH/exec/bare rows
(34), and a dialect-wrapped relocation test is new. Baseline reds for the
ten F1/F2 rows and the F3 row were observed on the adopted bytes; ledger
rows R1–R10 bind every row the chief named, including tools omission,
`tools: []`, the unrestricted-office row per field, read-only under the two
wider classes and the dialect sandbox row. `agents.rs` and `agents/load.rs`
carry no net change. 2.1 and its substeps stay ticked on the same stated
basis: fresh local gates on the restored tree, external exact coverage,
macOS and remote CI pending.

Second review return, the same run, phase `implement` based on `4a441bf7`
(2026-09-23), recorded in full under evidence.md "Unit 2 — second review
return, S1 and three proof gaps answered". The chief (gpt-6-astra,
`residual`, medium security) found `expressed_sandbox` ignoring `--add-dir`
(S1), substring-only assertions for the unknown direct mapping and the
later-candidate gap with no unavailable-fallback case (P1), inherited-body
rows short of explicit empty, malformed and widening (P2), and two ledger
rows (B6, R2b) claiming assertions their experiments never reached (P3).
All four are answered: `bundle.rs::expressed_sandbox` refuses `--add-dir`
in both spellings wherever it stands, with eight rows observed red on the
adopted guard and then green (2.1.5); both agent tests assert the complete
refusal and a new unavailable-fallback test binds the whole-chain check
(2.1.3); the executable-forms table carries the three inherited rows with
their complete composed causes (2.1.4); B6 and R2b are corrected in place
and reruns S-M8/S-M9 reach the rows they claimed (2.1.6). Ledger rows
S-M1–S-M9 bind every new row. Fresh local gates passed on the restored tree
(2.1.7). 2.1 and its substeps stay ticked on the same stated basis; external
exact coverage, macOS and remote CI remain pending.

### 2-fix — third-review sandbox admission omissions

Specify run `build-decision-0065-slice-one-re-11592690` adopts every commit
through `6a7044e5`. The commission carries S1 (security) and A1 (adversarial)
from the third review in `build-decision-0065-slice-one-re-e332dc8d`; the chief
independently reproduced both and the residual was **not accepted**. There is
no `returned_from` in this run. These are implementation omissions against
D5.3/D5.5 and SCM, not an upstream specification defect.

Reopen **2.1, 2.1.5, 2.1.6 and 2.1.7 only for 2-fix**. The checked decoder,
narrowing and site-fact work and all prior observations remain adopted. The
historical completion paragraphs above retain their original tested scope;
they do not close these newly demonstrated cases. This subsection is the
entire remaining unit 2 commission, not a replay of earlier substeps.

Production scope for this repair is `crates/brokkr-runtime/src/bundle.rs` and,
only if the admission check needs resolved contributions exposed,
`crates/brokkr-runtime/src/capabilities.rs`. Tests stay solely in
`crates/brokkr-runtime/src/bundle/agent_tests.rs`. Tasks/evidence accompany the
repair. No new module, suite, grammar change, origin redesign, shipped data,
pin, grant or frozen byte enters this visit. Stop and inventory a split before
exceeding this actual scope; later final-launch units 13–15 cannot own this
compile-admission repair.

Design adoption at `e132f678`, the same run: [D5.6](design.md#d56-unit-2-fix-council-disposition-2026-09-23)
reconciles both current council positions. Keep earlier admission in place;
check every resolved outcome before publishing capability notices/facts, using
the effective inherited sandbox and existing controls projection/typed decoder.
Inspect the complete selected/substituted argv, with a private native context
that permits the established canonical `web_search` config key without
exempting any sandbox or root control. The inspected interface supports a
bundle.rs-only repair; capabilities.rs remains conditional. No repair checkbox
closes from this design, and the accepted numbered unit order remains intact.

Tasks adoption, run `build-decision-0065-slice-one-re-11592690`, phase
`tasks`, starts at `a4787b2e2ad30bf4a054f2372f3e4ad312a9f70c`
(2026-09-23). Retain every commit, including unit 2 through `6a7044e5`,
specification `e132f678` and design `a4787b2e`. Open questions concern unit 9
and external evidence, not this repair; D5.6 explicitly has no unresolved
unit 2-fix question. No upstream amendment is needed. The following numbered
checkboxes replace the provisional `2-fix.1`–`2-fix.4` breakdown; their
`previous` references preserve traceability. They remain substeps of unit 2,
closing only the reopened 2-fix portions of 2.1.5–2.1.7 and aggregate 2.1.
Do not redo the checked 2.1.1–2.1.4 or any adopted proof.

Execute 2.2–2.8 in order, within this one repair visit. Capture the full planned
baseline before production edits. Each behavioral task includes its own
independent compiling mutations and restored passes before advancing; 2.6
checks their record rather than deferring proof to an audit. All new tests and
separately claimed rows belong in `bundle/agent_tests.rs`, use the canonical
`AgentFixture` root and production compilation, and compare independently
written complete expected values/reasons. Never read `.forge/` from a test,
require an installed provider, manufacture an Outcome as compilation evidence,
or count an unrelated refusal or compiler error as a binding mutation.

- [x] 2.2 **2-fix baseline and exact assertions.** Before production edits, stage every planned row for 2.3–2.5 in `bundle/agent_tests.rs` and record its test/row, requirement/scenario, full expected cause or value, command, source revision and observed result in evidence.md. In particular, run S1.1–S1.3 (native OFF with `--add-dir=/srv/shared` at harness work, sandbox bypass at a harness gate, and `sandbox_workspace_write.network_access=true` at harness work) and A1.1–A1.8 (each of `--cd /`, `--cd=/`, `-C /`, `-C/` independently in authored argv and `hands.harness.work`). Use the matching typed sandbox so another mismatch cannot mask admission; each row must reach its own complete refusal assertion and report actual left/right values. Record unexpected compilation as that row's baseline red, and already-correct neighbors as passes. Retain the two empty-grant gate/work positives plus existing boxed/hands/effort controls with exact local fields, selected fragments, boundary, holdings, native OFF facts and full resolved denial argv. Verify all planned rows execute even if earlier rows fail, using the existing `each_row` seam or individual test filters. Supplied chief observations are not this visit's executions; never fabricate missing historical reds. Requirements: [Shipped inline permissions migrate before refusal lands][SCM], [Refusal proofs assert the full reason][SC8], [Known native powers require a valid delivered denial or refusal][NCR]. (previous 2-fix.1 and baseline portion of 2-fix.3)

- [x] 2.3 **2-fix canonical root refusal and contribution context.** In `bundle.rs::expressed_sandbox`, use the existing grammar's canonical identity to refuse `--cd` for every value, including the current workspace; do not change grammar.rs or infer last-option priority. Keep every parsed occurrence subject to the existing sandbox/root/load/config checks. Add only the small private context needed to distinguish resolved native configuration from authored/selected-hands input. New causes identify site, candidate link, contribution and canonical option; root cases name `--cd`, config cases name `--config` and only a fixed known table, never an authored assignment. Keep the option/cause portion within 512 Unicode scalars, without echoing a path, attached token or payload through any wrapper. Verify all eight A1 rows with independent complete literal reasons, retain selected-hands/authored positives and earlier refusal precedence, then independently remove the relevant check for each row, observe that row wrongly compile at its intended assertion, restore and pass. S1 remains open until 2.4. Requirements: [Shipped inline permissions migrate before refusal lands][SCM], [Authored provider configuration cannot supply capability authority][RGR], [Refusal proofs assert the full reason][SC8]. (previous root/context portion of 2-fix.2 and its proofs in 2-fix.3)

- [x] 2.4 **2-fix resolved native admission and valid denial.** In `bundle.rs::record_capabilities`, after successful `site_capabilities` resolution and before publishing notices/facts, judge every outcome of a site with an effective typed sandbox from `SiteFacts.local`, including an inherited office class. Keep `enforce_model_policy` and earlier local admission in their existing order. Use `Outcome::controls()` and the existing typed managed decoder to inspect the complete selected ON/OFF plus instantiated restriction argv once; do not re-resolve, reconstruct fragments from labels, scan only the primary or truncate a new zip. Missing/malformed controls refuse with bounded context, never default to empty. Apply the shared guard to all native argv; any native `--sandbox`, even a matching one, competes with the sole selected-hands representation. Permit only the additional exact canonical `web_search` configuration key in resolved-native context, preserving the exact `-c`, `web_search="disabled"` denial pair and existing established compatible controls. Do not permit `web_search.*`, arbitrary tables or metadata-defined exceptions, extend this allowance to authored/hands input, or stop scanning after a valid denial. Verify S1.1–S1.3 independently reach their complete option-naming refusal; each compiling enforcement removal must wrongly admit its own case, then restore and pass. Independently remove the native-denial allowance to fail each matching gate/work positive, restore and pass; assert full resolved argv and facts, not only OFF labels. Change capabilities.rs only after recording a concrete missing exposure that defeats the inspected existing projection. Requirements: [Shipped inline permissions migrate before refusal lands][SCM], [Known native powers require a valid delivered denial or refusal][NCR], [Authored provider configuration cannot supply capability authority][RGR], [Refusal proofs assert the full reason][SC8]. (previous resolved-native portion of 2-fix.2 and its proofs in 2-fix.3)

- [x] 2.5 **2-fix complete contribution and preservation proofs.** Complete the baselined matrix in `bundle/agent_tests.rs`: all four canonical root spellings independently in each of resolved ON, OFF and nonempty substituted restriction argv (twelve rows in addition to A1); a real temporary realm/dialect grant and holding must select ON, and validated nonempty restrictions must actually reach their transport. Bind all-occurrence scanning beside valid controls and retain native competing `--sandbox`, added-root, bypass/full-auto, sandbox-table, opaque-load, duplicate/malformed-command and unqualified-config refusals. Verify the exact-key/native-only config allowance with its descendant and authored/hands neighbors; preserve established hands/effort support. Include an otherwise valid primary with a conflicting later candidate, inherited effective sandbox, untyped admission preservation and long Unicode/newline payloads within grammar limits with the identical value-free bounded cause. Assert exact site/link/contribution reasons and full admitted facts/argv. For every new test and independently claimed row, run an isolated compiling in-scope mutation that fails that row's intended assertion (each defect must wrongly compile), record actual left/right, then restore and pass. A broad mutation's first failure proves no unreported row. Synthetic restriction refusal transport does not qualify provider support or close unit 9; cold comparisons do not prove a launch or close units 13–15. Requirements: [Shipped inline permissions migrate before refusal lands][SCM], [Refusal proofs assert the full reason][SC8], [Known native powers require a valid delivered denial or refusal][NCR], [Reserved hands preserves the existing workspace authority][TD6]. (previous remaining matrix/proofs of 2-fix.3)

- [x] 2.6 **2-fix evidence and requirement audit.** Check the recorded baseline/fix/mutation/restoration entry for all eleven chief cases, all twelve native root rows and every additional new test or independently claimed row from 2.2–2.5 against the owning suite. Record exact test/row, command, revision, compiling in-scope mutation, observed intended assertion with actual left/right and restored pass; retain sufficient adopted evidence by revision without relabeling it. Check the coverage map below and reject `is_err()`, substring-only causes, production-derived expectations, unrelated failures, unexplained rows or unexecuted mutation claims. Confirm every mutation is restored, every fixture keeps its canonical root, and every test runs without `.forge/` or a provider. Tick only the repaired proof portion of 2.1.6 when this audit passes; admission 2.1.5 additionally requires 2.3–2.5. Requirements: [Refusal proofs assert the full reason][SC8], [Shipped inline permissions migrate before refusal lands][SCM]. (previous audit portion of 2-fix.3)

- [x] 2.7 **2-fix restored local gates and commit.** Run every local command below on the restored tree and record exit/result and tested revision in evidence.md. Inspect the entire diff against the repair allowlist and unchanged frozen/shipped/pin paths; if a required repair needs another file or suite, stop with the exact split before editing it. Commit only the in-scope repair, suite and tasks/evidence in repository style, retain all adopted ancestry and never push. Verify the committed head and clean worktree, recording the SHA in run-local evidence so it does not move the tested source. Tick this task only for observed local passes and the commit; unavailable checks remain unpassed. This closes only the local portion of 2.1.7, not its external evidence or aggregate 2.1. Requirements: [Refusal proofs assert the full reason][SC8], [Digest pins are measured and their history remains truthful][MP5], [Shipped inline permissions migrate before refusal lands][SCM]. (previous local-gate/commit portion of 2-fix.4)

- [ ] 2.8 **2-fix external evidence and completion boundary.** Obtain external exact coverage on the final repaired source head using `bash scripts/coverage-exact.sh`, with nonzero covered/total equality for lines, branches and logical functions; verify both workflows and the script consume `rust-nightly-version.txt`. Record actual tested-head evidence and relevant Linux/macOS/remote results; unavailable results remain pending, with no nested-box substitution or lowered gate. Unit 1's outstanding evidence and later candidate-wide gates retain their owners. Close aggregate 2.1 and the remaining gate portion of 2.1.7 only when their required evidence exists, all finished repair tasks are ticked and the work is committed. A committed local repair with pending external checks is not fully green or complete. Requirements: [Refusal proofs assert the full reason][SC8], [Digest pins are measured and their history remains truthful][MP5]. (previous external-evidence/completion portion of 2-fix.4)

Requirement/scenario coverage for the remaining 2-fix work:

| Owning requirement / scenario | Execution and verification tasks |
| --- | --- |
| SCM: Resolved native contributions cannot compete with a typed sandbox (S1.1–S1.3; ON/OFF/restriction paths) | 2.2 baseline; 2.4 repair and independent proofs; 2.5 complete contributions; 2.6 audit |
| SCM: Canonical root-changing options cannot contest a typed sandbox (A1.1–A1.8; twelve native spelling rows) | 2.2 baseline; 2.3 shared refusal and proofs; 2.4 native placement; 2.5 matrix; 2.6 audit |
| SCM: Valid native denial preserves matching typed sandboxes; NCR: Known native powers require a valid delivered denial or refusal | 2.2 exact positives; 2.4 narrow allowance and removal proofs; 2.5 native-only/neighbor checks |
| SCM: An existing sandbox fragment must match the typed request; Field omission inherits while an explicit empty list subtracts; An incompatible later candidate cannot hide behind selection | 2.2 retained baselines; 2.4 effective local fact/every outcome; 2.5 inherited/later-candidate/preservation rows |
| RGR: Authored provider configuration cannot supply capability authority; TD6: Reserved hands preserves the existing workspace authority | 2.3 earlier admission and canonical refusal; 2.4 native-only allowance; 2.5 hands/effort and authored-neighbor preservation |
| SC8: Refusal proofs assert the full reason, including canonical-root and exact removal/restoration obligations | 2.2–2.5 row-level experiments; 2.6 complete ledger audit; 2.7 restored gates; 2.8 external evidence |
| MP5: Digest pins are measured and their history remains truthful | 2.7 unchanged pin/frozen scope audit and truthful tested-head record; 2.8 pending versus observed external evidence; no pin edits commissioned |

This tasks visit changes only tasks.md and evidence.md. It performs no Rust
repair or mutation and ticks no implementation checkbox. Observed task-draft
checks and tool availability are recorded under evidence.md's “Unit 2-fix —
ordered tasks adoption”; prior specify/design observations remain historical.
Archive remains conditional task 28.1 after the later units, outside 2-fix.

Implementation observations, the same run, phase `implement` based on
`07228d50` (2026-09-23), recorded in full under evidence.md "Unit 2-fix —
repair: canonical root refusal and resolved native admission". The one
production file is `bundle.rs`; `capabilities.rs` did not change, because the
existing `Outcome::controls()` projection and the typed `managed` decoder
already expose the complete resolved argv. `expressed_sandbox` refuses
canonical `--cd` in every spelling and value, and takes a private
`Contribution` context whose only effect is to establish the exact
`web_search` key in resolved native plans. The new `admit_native_sandbox`, called
from `record_capabilities` before notices/facts are published, judges every
outcome's resolved ON/OFF/restriction argv under the effective (inherited)
class and refuses any native `--sandbox`. The five new tests in
`bundle/agent_tests.rs` were observed red on the adopted guard first (all
eleven chief cases and the twelve native root rows compiled), then green.
Ledger rows A-M1–A-M8, N-M1–N-M3, S-M1–S-M12 and P-M1 each bind their rows with a
compiling mutation, and every mutation was restored. Tasks 2.1.5, 2.1.6 and 2.2–2.7 are ticked on
observed local passes and the commit. 2.8, the gate portion of 2.1.7 and
aggregate 2.1 stay open because external exact coverage, macOS and remote CI
are pending. The committed SHA is in the run-local result.

Review return, the same run, phase `implement` based on `b4fd8d36`
(2026-09-23), recorded in full under evidence.md "Unit 2-fix — review
return: S2, SC1, C1/SC2 and SC3 answered". The chief (gpt-6-astra,
`residual`, medium security) found a grammar problem rendered with its
token, so a duplicate or malformed root selector echoed its whole value
past the 512-scalar bound (S2); no resolved-native opaque-load, duplicate or
malformed row despite 2.5/2.6 (SC1); native sandbox-table refusals omitting
`--config` (C1/SC2); and the audit claiming a baseline for rows added after
it (SC3). All four are answered in the same files: `expressed_sandbox` names
an unplaceable argument by position and the grammar's fixed cause, never its
token, and a native table refusal names `--config`; the new test
`an_unreadable_contribution_refuses_by_position_without_echoing_its_token`
binds duplicate/malformed rows in all three contributions plus a native bare
word and trailing option, and the native OFF table gains its profile-load
row. Every new or changed row was observed on `b4fd8d36` first (11 red, the
profile-load row already correct), and ledger rows R-M1–R-M9, U-M1–U-M4,
C-M1–C-M2 and L-M1 each bind exactly one row. The audit claim is corrected in
place, with an explicitly retrospective adopted-guard run for the rows that
had none. 2.2–2.7 stay ticked on the same basis, now including this return;
2.8, the gate portion of 2.1.7 and aggregate 2.1 stay open.

Local gates for 2.1.7, from the house and D5.4/D10:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p brokkr-runtime --all-features --locked
cargo test --workspace
cargo test --workspace --all-features --locked
cargo run --locked -p brokkr-cli -- compile --bundle bundles/self
cargo run --locked -p brokkr-cli -- compile --bundle bundles/verify
openspec validate --all --strict --no-interactive
git diff --check
```

External exact coverage is `bash scripts/coverage-exact.sh` on a capable host
or CI, with nonzero covered/total equality for lines, branches and logical
functions and the pinned compiler agreement. Keep it pending until observed;
never lower the gate or substitute a nested-box failure. Supported-host
claims require actual Linux/macOS evidence.

Units 3–4 own new lowering/origin transport. Their later implementation may
remove a temporary refusal only once that path delivers its restriction;
unit 2 never accepts then discards it. Shipped migrations, general authored-flag
refusal, final command/resume proofs and archive remain in their assigned
later units. This tasks re-adoption ticks none of the above; its completion is a committed task draft, not implementation or fully green completion.

## 3. Unit 3 — Lower typed tools and define private origins

- [ ] 3.1 Unit 3 defines typed lowering and private segment decoding/reassembly, with expected state independent of generated argv. Verify exact mappings and identical-byte origin distinction in agents/native-controls suites; no public schema. Requirements: [Shipped inline permissions migrate before refusal lands][SCM], [Authored provider configuration cannot supply capability authority][RGR], [Every accepted native control reaches the final command][NCC]. New explicit substep of 4.2: operator rulings 1–2; both design positions. (previous 3.20)

Specification adoption, 2026-09-23, baseline `b4839426`: this visit amends
only unit 3's acceptance detail in SCM/NCC and records no implementation
completion. Preserve the three production files and two owning suites named
in Rebuild units. Task 3.1 remains open; 4.2 advances only when the primitives
are implemented and remains open until unit 4 proves transport. Do not remove
unit 2 guards or activate authored refusal to make a primitive test pass.

For 3.1, bind SCM's three Unit 3 scenarios and NCC's four Unit 3 scenarios:
exact mapped prefixes/order and separate template/local origins; omitted,
empty and sandbox expected values with hands semantics retained; all five
origin kinds including equal-byte copies; fallible mandatory-state decoding;
exact ordered reassembly; independent expected native/local/hands state.
Use the existing runtime `agents/tests.rs` and protocol
`native_controls/tests.rs`, with canonical temporary roots and no installed
provider or `.forge/` reads. Every added test/row needs its observed baseline,
independent compiling mutation, exact failing assertion and restored pass in
evidence. Existing passing neighbours are labelled as such, never invented
reds. An unavailable primitive is not a compiling baseline failure.

Run fmt, workspace clippy, each touched crate suite, workspace tests, self
bundle compilation, strict OpenSpec validation and diff checks on restored
work. Keep external exact coverage and unobserved host/remote results pending.
If the primitive requires changes beyond the named production/test scope,
stop and inventory a split before implementation, preserving prior commits.

Observed specify checks: strict OpenSpec all-items validation passed 18/18,
and change validation and `git diff --check` passed. Cargo could not be
executed in this seat (`No such file or directory: 'cargo'`), so fmt, clippy,
both owning crate suites, workspace tests and self compilation are unavailable,
not green. Full details are in evidence's unit 3 specification entry.

Council design adoption, 2026-09-23, run
`build-decision-0065-slice-one-re-29dd19f2`, baseline `12110687`: D5.7 reads
and explicitly reconciles both current positions. Keep task 3.1 and aggregate
4.2 open. Unit 3 retains structured composition on ChainEntry and native typed
contribution/expectation in its producer before legacy projections. Candidate
storage and all dispatch changes are unit 4 work; a helper pass is no proof
of their transport. This placement follows SCM/NCC's existing unit boundary,
without changing a requirement or activating refusal.

Implement within the same three production files and two suites. Preserve
current admission, direct-empty refusal, dormant hands semantics and exact
mapped values. The seven scenario proof rows and their independent mutations
are specified in D5.7. Exercise native expectations through the real producer
from agents/tests.rs; hand-built decoder records cannot substitute for it.
Record actual baseline, compiling mutation assertion and restored pass per
new test/claimed row, never invented reds or a single first-row failure for
an entire table. No tests, mutations or behavioral closure occurred in this
design visit. See the unit 3 council entry in evidence.md for observed checks.

### Unit 3 executable order — tasks adoption, 2026-09-23

Baseline `ae1850c2`, run `build-decision-0065-slice-one-re-29dd19f2`, phase
`tasks`. Adopt specification `12110687`, design `ae1850c2` and every preceding
commit. There is no `returned_from`. The tasks below refine 3.1 (previous
3.20) and only the unit 3 prerequisite portion of 4.2 (previous 3.15); they
do not replace or renumber either aggregate. Execute **3.2–3.10 in order**.
All are implementation obligations, unchecked by this tasks seat.

The production allowlist is exactly runtime `agents.rs`, runtime
`capabilities.rs` and protocol `native_controls.rs` under their existing
crate src roots. The only suites to extend are runtime `agents/tests.rs` and
protocol `native_controls/tests.rs`. Every step updates this change's
`tasks.md`/`evidence.md`. No new module, public schema/manifest field, dependency,
shipped data or measured pin is planned. Required edits beyond these files
must stop for an inventoried split before implementation. Preserve unit 2's
admission fences and today's serving consumers. Unit 4 alone takes Candidate
storage/constructors, selected-candidate binding, driver-extras projection and
protected input transport; units 12–15 retain activation/final-launch proof.

Each behavioral task includes its own tests and proof before advancing. For
every new test and independently claimed row, record the tested revision,
command, baseline outcome, isolated compiling mutation, failing test and exact
assertion/actual values, restoration and passing rerun. An already-correct
neighbor begins with an observed pass; a missing API/build error is unavailable,
not a behavioral red. When an API must first be introduced, record that limit
and run the new assertion as soon as it compiles, before its behavioral fix.
No `is_err()`, substring refusal, production-derived oracle or first-row table
failure substitutes for an independent assertion. Use the existing canonical
Tree/retained TempDir for file fixtures, pure in-memory decoder records where
appropriate, no `.forge/` fixture reads and no installed provider.

- [x] 3.2 **Capture the adopted baseline and row ledger before behavior changes.** Verify clean branch/history against `ae1850c2`, read D5.7 and the seven scenarios in the coverage table below, and inventory the three production files/two suites. In the owning suites stage exact literals for currently expressible mapped limits, direct-empty/native-alias refusals, dormant hands, legacy controls and known/unmeasured outcomes; run each row before editing its behavior. Record richer-type/API gaps as unavailable and schedule their first compiling baseline in 3.3–3.7. Verify the ledger distinguishes intended reds from actual passes/unavailable results and identifies an independent mutation per claimed row. Requirements: [Refusal proofs assert the full reason][SC8], [Denial and admission have removal proofs and bounded live claims][NC6], [Shipped inline permissions migrate before refusal lands][SCM]. (refines 3.1, previous 3.20: baseline/proof portion)

- [x] 3.3 **Define the shared private composition types in protocol native_controls.rs.** Introduce exactly authored/template/local/hands/native origins, ordered segments and D5.7's mandatory typed expectation, with runtime-visible Rust types but no public wire contract. Preserve explicit unspecified/empty/nonempty local intent, direct versus hands application, exact sandbox classes, candidate identity, native inventory/holdings/tools/restrictions and hands intent. Keep raw native argv/restriction transport plus typed Selection and its mappings distinguishable until materialization. In protocol native_controls/tests.rs assert literal five-kind sequences, equal-byte copies, repeated origins/occurrences, empty segments and empty-string arguments; separately relabel and coalesce contributions in compiling mutations, fail the exact origin/occurrence assertions and restore/pass each. Verify legacy consumers remain unchanged. Requirements: [Every accepted native control reaches the final command][NCC], [Authored provider configuration cannot supply capability authority][RGR]. (refines 3.1, previous 3.20: private types/origins)

- [x] 3.4 **Retain local composition before flattening in runtime agents.rs.** Replace the Composed tuple with D5.7's structured value on ChainEntry; derive compatible argv/effort/hands_fragment projections and explicitly represent unavailable/refused composition. Capture effective LocalTools/Sandbox and required HandsSpec before fallible emission; retain adapter driver/model/effort as template, exact ordered mappings as local and selected workspace tokens as hands. Preserve declared harness fragments for unit 4's boundary selection. Keep Candidate layout unchanged and label resolve_report's lossy projection. In agents/tests.rs assert the complete pytest/cargo contribution `["--allowedTools", "Bash(.venv/bin/pytest:*),Bash(cargo:*)"]`, literal narrow gh-pr-view/gh-run-view fixture mappings, separate acceptEdits template, omission/empty/nonempty and all three sandbox classes plus unspecified. Verify direct-empty and mapped-native-alias full refusals, dormant empty/unmapped lists without unused mapping demands or local flags, required hands with empty native grants, and matching Codex hands retaining one exact sandbox control. Do not remove bundle admission fences. Independently mutate emitted prefix/order/separator/tag, expected allow/sandbox state, dormant-flag emission and hands retention; record each exact failure and restored pass. Requirements: [Shipped inline permissions migrate before refusal lands][SCM], [Reserved hands preserves the existing workspace authority][TD6], [Authored provider configuration cannot supply capability authority][RGR], [Every accepted native control reaches the final command][NCC], [Refusal proofs assert the full reason][SC8]. (refines 3.1, previous 3.20: local producer; advances only unit 3's portion of 4.2, previous 3.15)

- [x] 3.5 **Retain native contribution and independent expectation in runtime capabilities.rs.** Build them with NativePlan from the selected Serving inventory, native-key/holding relation and typed Holding before rendering controls; keep Outcome identity and existing controls/manifest/prompt projections compatible. Retain exact provider/harness/model, known or unmeasured inventory/reason, abstract held/denied powers, admitted tools and original structured restriction objects. Preserve measured default ON with no argv. Retain raw argv and native Selection provenance without dropping or double-emitting either; reuse the existing selection-lowering path, with any necessary helper confined to protocol native_controls.rs, without authored-list reconciliation as an origin oracle. In agents/tests.rs use the real Authority resolver/producer and canonical fixture data, asserting complete literal expectation, raw controls, selection/mappings and restriction substitution separately. Independently corrupt argv, selection and restriction serialization and assert unchanged expected state beside the exact emission failure; separately mutate each claimed held/denied/subset/restriction/default-ON/unmeasured expectation and restore/pass. Verify distinct candidates never borrow holdings. Synthetic restriction transport proves byte retention only and does not close unit 9. Requirements: [Every accepted native control reaches the final command][NCC], [Restrictions are validated, carried and pinned without engine interpretation][RG4], [Provider compatibility cannot expand a holding][SC4], [Native capability controls are adapter-owned evidence-bearing data][NC1]. (refines 3.1, previous 3.20: native producer)

- [x] 3.6 **Implement the fallible mandatory private reader in protocol native_controls.rs.** Decode ordered segments and the complete expected envelope without legacy-pair inference, `.ok()`, silent defaults or optional fallback. Enforce the closed engine-owned shapes and all required variant members: absent/null record or state, wrong containers, non-string argv/tool members, unknown members and origin/inventory/application/sandbox variants refuse. Preserve valid explicit empties/unspecified variants and arbitrary already-validated restriction-object contents without a second dialect validator; claim no raw duplicate-key detection from Value. In protocol native_controls/tests.rs independently assert every claimed malformed row's full bounded cause, fixed field path and numeric position, including long/newline/Unicode payload sentinels with no value/tag/key echo and D6's 512-scalar bound; assert successful complete typed values too. Bypass each independently claimed check with a compiling mutation, reach its intended assertion, then restore/pass it. Verify the old authored/managed pair alone cannot decode as the richer record and today's serving reader is not rewired. Requirements: [Every accepted native control reaches the final command][NCC], [Refusal proofs assert the full reason][SC8]. (refines 3.1, previous 3.20: strict private decoding)

- [x] 3.7 **Bind exact ordered reassembly in protocol native_controls.rs.** Compare the full segment concatenation with the explicitly supplied argv slice, including length, empty strings and all occurrences; no wrapper trimming, byte-search ownership, prefix/count-only checks or truncation. In protocol native_controls/tests.rs assert a complete valid round trip and separate full mismatch causes for added, dropped, replaced and distinctly reordered tokens, including independent trailing-length and empty-string-loss cases. Verify equal-byte origin swaps can still reassemble while their literal origin sequences remain different. In agents/tests.rs check local composition against its full candidate argv; leave driver-extras projection to unit 4. Independently bypass correspondence and mutate empty/length/order handling for every claimed row, record the intended exact assertion and restore/pass. State explicitly that correspondence supplies no authentic engine authorship or final semantic check. Requirements: [Every accepted native control reaches the final command][NCC], [Authored provider configuration cannot supply capability authority][RGR], [Refusal proofs assert the full reason][SC8]. (refines 3.1, previous 3.20: exact correspondence)

- [x] 3.8 **Audit unit 3's primitive proof and downstream boundary.** Check all seven scenario rows against 3.2–3.7's observed per-test/per-row baseline, mutation and restored-pass ledger; run any missing in-scope proof in its owning suite, not a broad historical campaign. Verify literal expected state is independent of emitted output, unmapped/failed composition is not successful empty state, no pending selection is claimed as complete argv, no new admission/refusal activation occurred, Candidate remains unchanged and no out-of-scope file/pin/frozen bytes moved. Verify all adopted commits remain ancestors and no mutation remains. Keep 4.1/4.2, unit 9 and units 12–15 open; record any newly required scope as a split before editing. Requirements: [Every accepted native control reaches the final command][NCC], [Shipped inline permissions migrate before refusal lands][SCM], [Denial and admission have removal proofs and bounded live claims][NC6], [Refusal proofs assert the full reason][SC8], [Slice-one records do not claim later-slice behavior][MP6]. (refines 3.1, previous 3.20: proof/scope audit)

- [x] 3.9 **Validate and commit restored unit 3 work.** Run `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`; `cargo test -p brokkr-runtime --all-features --locked`; `cargo test -p brokkr-protocol --all-features --locked`; `cargo test --workspace`; `cargo run --locked -p brokkr-cli -- compile --bundle bundles/self`; `openspec validate --all --strict --no-interactive`; and `git diff --check`. Record actual exits and unavailable checks separately, fixing only within this unit or stopping for a split. Verify every completed step is ticked and evidence matches the tested tree. Commit restored changes in repository message style, never push, then verify clean scoped status and record the committed SHA/final documentation checks in run-local evidence without editing the validated source head. This box requires successful listed local gates and a commit; a committed draft or unavailable tool is not an implementation pass. Keep 3.1 open while proof/gates remain outstanding. Requirement: [Digest pins are measured and their history remains truthful][MP5] (scenario: Validation remains a proof obligation); house commit/no-push rule. (refines 3.1, previous 3.20: local validation/commit)

- [ ] 3.10 **Record external evidence before a fully green unit claim.** Obtain host/CI `bash scripts/coverage-exact.sh` outside the nested workspace box on the committed unit candidate with the unchanged pinned compiler. Record revision, exit and nonzero covered/total counts for source lines, branches and logical functions, each exactly equal; never lower the gate or invent missing counts. Record actual supported Linux/macOS and applicable remote results with their tested heads, retaining unavailable results as pending and final-candidate obligations with unit 27. Verify 3.2–3.9, all primitive row proofs and the applicable gates before ticking 3.1; leave aggregate 4.2 and conditional archive 28.1 open. Requirements: [Digest pins are measured and their history remains truthful][MP5] (scenario: Validation remains a proof obligation), [Refusal proofs assert the full reason][SC8] (scenario: macOS canonical fixture roots preserve exact diagnostics), [Slice-one records do not claim later-slice behavior][MP6]. (refines 3.1, previous 3.20: external proof/completion boundary)

| Owning requirement / exact unit 3 scenario | Execution and proof | Existing owning suite |
| --- | --- | --- |
| [Shipped inline permissions migrate before refusal lands][SCM] — Unit 3 lowering preserves exact mapped limits | 3.4; emission and expectation mutations independently | runtime agents/tests.rs |
| [Shipped inline permissions migrate before refusal lands][SCM] — Unit 3 lowering retains absence empty and sandbox intent | 3.4; each allow/sandbox row, complete existing refusals | runtime agents/tests.rs |
| [Shipped inline permissions migrate before refusal lands][SCM] — Unit 3 primitives cannot bypass the delivery handoff | 3.4 and 3.8; dormant local/hands mutations, admission retained | runtime agents/tests.rs; existing admission gates remain untouched |
| [Every accepted native control reaches the final command][NCC] — Unit 3 equal bytes retain different supplying origins | 3.3 and 3.7; independent relabel/coalesce proofs | protocol native_controls/tests.rs; runtime agents/tests.rs for producers |
| [Every accepted native control reaches the final command][NCC] — Unit 3 private decoding never defaults missing authority | 3.6; each field/type/tag/member cause independently | protocol native_controls/tests.rs |
| [Every accepted native control reaches the final command][NCC] — Unit 3 reassembly checks every argument in order | 3.7; independent addition/deletion/replacement/order/empty cases | protocol native_controls/tests.rs; runtime agents/tests.rs for full argv |
| [Every accepted native control reaches the final command][NCC] — Unit 3 expected state is independent of emission | 3.4–3.5; real local/native producers, separate state and emission mutations | runtime agents/tests.rs; protocol native_controls/tests.rs for typed record retention |

No upstream amendment is required by this breakdown: D5.7 settles storage and
SCM/NCC settle the primitive boundary. Unit 9's open provider qualification is
not a prerequisite for synthetic byte-retention tests here. The tasks visit
adds no implementation, tests, mutation outcomes or completion ticks. Observed
documentation validation and unavailable Cargo gates are recorded in evidence.

Implementation visit, 2026-09-23, run `build-decision-0065-slice-one-re-29dd19f2`,
baseline `4d8b7668`: 3.2–3.9 were ticked. The three production files
and two owning suites are the only source files changed. The seven scenario rows
are bound by ten new tests (five per suite) and 34 compiling mutations. Every
mutation failed its intended assertion and was restored; the restored diff was
compared byte for byte with the pre-mutation one. All listed local gates passed
on the restored tree. **3.10 stays open**: external exact coverage, macOS and
remote CI are pending. So 3.1 stays open, and 4.2 stays open for unit 4's
transport. Candidate, bundle admission, the serving readers and authored
refusal are unchanged. The ledger is evidence's "Unit 3 — implementation"
entry.

**Correction (review return, same run).** That visit overclaimed 3.2, 3.5 and
3.8. Its baseline measured suite totals only. Its typed contribution dropped
the selection's list flags and separators (R1), so 3.5 was not done. Its
ledger exempted most round-trip and reassembly rows. M1–M3 stopped at the
first mapped row. It had no selection-emission or denied/restriction
expectation mutation. Several of its decoder mutations reworded a cause
without removing the check. The return visit repaired these. It measured six
baseline probes, retained the typed selection mappings and materialized them
from the real producer. It added 45 isolated compiling mutations, each with
its failing rows and restored pass. It reran every listed gate. 3.2–3.9 are
ticked on that record; see evidence's "Unit 3 — review return". **3.10, 3.1
and 4.2 stay open.**

**Second correction (SC1, same run).** The return ticked 3.8 while three
per-row proofs were still missing:

- It exempted the "codex with a selection" row.
- The exchanged equal-byte record had no mutation that reached it.
- It had no runtime mutation for unspecified or nonempty allow intent, or for
  read-only or workspace-write sandbox intent.

This visit supplies them. It changed no production file. The equal-bytes test
now judges its written and exchanged records side by side, so origin
mutations reach both. It added 11 isolated compiling mutations and reran two:

- C1–C6 cover every `Intent::of`/`Sandbox::intent` arm.
- X1 disables both of the codex row's guards. X1a and X1b show that either
  guard alone masks the other.
- M13r and M14r rerun M13 and M14 against the side-by-side test.
- X2 and X3 each fail only the exchanged record.

The ledger is evidence's "Unit 3 — second review return: SC1". 3.4, 3.7 and
3.8 stay ticked on that record. **3.10, 3.1 and 4.2 stay open.**

## 4. Unit 4 — Wire origins through runtime dispatch

- [x] 4.1 Unit 4 wires the selected candidate's private segments through engine.rs SiteSpawn, bundle projection, boundary composition, placeholder expansion and final input merging. Verify absent/reordered/overridden records refuse and legitimate typed controls survive. Requirements: [Shipped inline permissions migrate before refusal lands][SCM], [Authored provider configuration cannot supply capability authority][RGR], [Every accepted native control reaches the final command][NCC]. New explicit substep of 4.2: operator rulings 1–2; robustness runtime evidence. (previous 3.21)

- [x] 4.2 Units 3–4 lower typed restrictions and carry distinct authored/template/local/hands/native origins end to end through runtime SiteSpawn, boundary/expansion and input assembly. Close only after unit 4 verifies exact limits, selected candidate, reassembly and override refusal; 15.2 owns final authored-counterfeit refusal. Requirements: [Shipped inline permissions migrate before refusal lands][SCM], [Authored provider configuration cannot supply capability authority][RGR], [Reserved hands preserves the existing workspace authority][TD6]. Reopened/remaining: operator ruling 1–2. (previous 3.15)

D5.7's inventoried storage split amends this future unit before implementation:
production files are runtime agents.rs, bundle.rs and engine.rs (three total).
Migrate Candidate storage, agents.rs::resolve_report and bundle.rs's model-policy
and expanded-candidate constructors together. Existing engine/tests.rs,
engine/agent_tests.rs and engine/resume_tests.rs join the already named
engine/capability_tests.rs and engine/boundary_tests.rs for constructor updates;
runtime tests/capability_launch.rs retains its integration obligation.
These future edits do not widen unit 3. Carry the selected native and local
expectations together with origins, project driver extras structurally, and
write the private record after all untrusted input merges. Do not recover the
richer record from Candidate::parts, raw bytes or the old authored/managed pair.
Keep 4.1/4.2 open until their full protected-transport proofs pass; final
serving-command and authored-counterfeit closure stays with 15.2. Any additional
file or pin dependency requires an inventoried split before editing.

Implementation visit, 2026-09-23, run `triage-directive-operator-ruling-dbc7463e`
(triage `chore`), baseline `48779b32`. The production files changed are
exactly runtime `agents.rs`, `bundle.rs` and `engine.rs`. The tests are in the
three named suites, plus the constructor migrations in `engine/tests.rs`,
`engine/agent_tests.rs` and `engine/resume_tests.rs`. No new module, pin,
shipped data or frozen byte moved.

- `Candidate` now carries its entry's `Lowering`. Both bundle constructors
  carry it: the model-policy projection verbatim, and the expanded candidate
  through `{brokkr}`/`./` expansion applied segment by segment.
- `SiteSpawn`'s trailing `managed` count is replaced by ordered `segments`
  and a sealed `LaunchRecord`. Every boundary arm carries the selected link's
  segments and labels only what it composed itself as `hands`: the box's
  placeholder expansion and exec prefix, the harness fragment and the network
  prefix. A command that is not the link's own composition refuses, and so
  does a link that never composed. An inline command stays authored, even
  when its bytes equal an engine composition.
- The legacy `launch_arguments` pair is now a projection of the extras
  segments.
- `mark_capabilities` seals the record last, from the serving outcome and the
  link's (or inline site's) typed local and hands state. It refuses a record
  planted before sealing.
- The dispatch door (`spawn_site` → `verify_record`) admits exactly the
  sealed record. A missing, malformed, reordered or relabelled record refuses,
  as does one planted where none was sealed and an argv that no longer
  reassembles. Each refuses before any driver starts.

Seven new tests and four amended ones are bound by 28 compiling mutations, two
of them rerun. Each was observed failing its intended row and then restored. All local gates passed.
4.1 and 4.2 are ticked on that local record. See evidence's "Unit 4 —
implementation". **Not fully green:** external exact coverage, macOS and
remote CI are pending. 15.2 still owns final authored-counterfeit refusal, and
units 12–15 own the serving command's final check. The driver still reads the
legacy pair; it does not yet read the record.

Review return, 2026-09-23, same run, reviewed head `07d88b44` (residual,
medium). 4.1 and 4.2 are unticked again: the review found them unsupported
while C1 stands.

- **C1, scope — blocked, awaiting the operator (resolved 2026-09-24 by ruling
  (a), below).** The commission quoted unit 4
  with an allowlist of `engine.rs` and `bundle.rs` and the three test suites.
  The settled design (Rebuild unit 4 as amended by D5.7, and this group's note
  above) inventories `agents.rs` as the third production file, and
  `engine/tests.rs`, `engine/agent_tests.rs` and `engine/resume_tests.rs` for
  the `Candidate` constructor migration. D5.7 forbids recovering the lowering
  from `Candidate::parts` or bytes, so the unit cannot be built inside the
  narrower allowlist. The first visit followed the design and edited those
  files at `07d88b44`. This return neither reverts nor widens them. It asks
  the operator to rule which scope governs: admit D5.7's inventory for unit 4,
  or commission the `Candidate` storage and constructor migration as its own
  split before unit 4.
- **SC1, fixed in `engine.rs`.** The driver extras are now cut structurally.
  The launch starts behind the leading `hands` segments that carry the
  engine's box or network prefix. The extras follow the three-token driver
  verb, less only an escape `--` directly behind it, as the driver's
  trailing-argument parser reads them. The first `--` is no longer searched
  for in the flattened argv. `seal()`, `launch_arguments()` and
  `verify_record()` share the cut.
- **SC2, fixed in `engine/boundary_tests.rs`.** The namespace arms now assert
  every segment literally: the MCP argument in full, the box prefix and the
  mapped script.
- **SC3, fixed in the three suites' unit-4 tests.** `capability_launch`'s
  `Operator` and a new `capability_tests::canonical_engine` derive every
  fixture path from a canonicalised root. Removing the canonicalisation is not
  observable on this Linux host, even with a `..`-spelled `TMPDIR`. macOS stays
  pending.

The mutation ledger and gates are in evidence's "Unit 4 — review return".

Re-run, 2026-09-24, run `triage-directive-operator-ruling-0842e965` (triage
`chore`), head `2835618a`. **C1 is resolved by the operator's ruling (a)**, the
addendum to `operator-ruling-2026-09-23.md`: D5.7's inventory governs unit 4,
so `engine.rs`, `bundle.rs` and `agents.rs` are its production files and
`engine/tests.rs`, `engine/agent_tests.rs` and `engine/resume_tests.rs` carry
the `Candidate` constructor migration. `07d88b44` and `e361a36e` are adopted
unchanged. Their diff from `48779b32` touches exactly those files, the three
named suites and these records. No code changed on this visit. Every local gate
was rerun on `2835618a` and matched the recorded results, so 4.1 and 4.2 are
ticked again on that record. See evidence's "Unit 4 — re-run under the scope
ruling". **Not fully green:** external exact coverage, macOS and remote CI are
pending. 15.2 still owns final authored-counterfeit refusal, and units 12–15
own the serving command's final check.

## 5. Unit 5 — Supply local mappings and scaffold support

- [x] 5.1 Unit 5 supplies local mappings/scaffold support. Verify npm/npx/node, narrow gh-pr-view/gh-run-view and existing names with library/init suites, no native grants. Requirements: [Shipped inline permissions migrate before refusal lands][SCM], [Legacy concrete permissions cannot grandfather a capability][SC7]. Reopened/remaining: operator ruling 1–2. (previous 3.16)

- [x] 5.2 Unit 5 aligns generated adapters/tools with migration/native assessment. Verify init_stacks/init_doctor/library-data scaffolds and native OFF. Requirements: [Native capability controls are adapter-owned evidence-bearing data][NC1], [Realms v6 adds grants without changing frozen versions][RG1], [Shipped inline permissions migrate before refusal lands][SCM]. Reopened/remaining: operator ruling 2–4. (previous 8.2)

Observed 2026-09-24 (evidence.md, "Unit 5 — local mappings and scaffold
support"), run `triage-directive-operator-ruling-83bcb215`. The claude and
LaneTally adapters map `npm`, `npx`, `node`, `gh-pr-view` (`Bash(gh pr
view:*)`) and `gh-run-view` (`Bash(gh run view:*)`), with no native value and
both acceptEdits drivers unchanged. `init.rs` changes only a comment, and no
generated byte moved. Four new tests (U5-A–D) cover this. U5-A and U5-B are in
`library_data.rs`: the exact name maps, and exact whole-argv lowering to the
node recipe's and verify reviewer's inline values. U5-C is in `init_stacks.rs`:
14 stack rows with vocabulary parity, no grant, and native OFF. U5-D is in
`init_doctor.rs`: stack scaffolds read through doctor exactly like the
unrestricted control. U5-A–C were red at baseline. M1–M5b each bound, and each
was restored. Eight witness pins, four uncomposed pins and one composed pin
were measured and moved with the adapter bytes. fmt, clippy, the workspace
tests (2465 passed), both bundle compiles, `openspec validate` and `git diff
--check` all passed. A finding for units 11 and 22 is recorded: an OFF that
denies nothing still compiles, and doctor does not see it. **Not fully
green:** external exact coverage, macOS and remote CI are pending.

## 5b. Unit 5b — Lower typed local tools at inline Claude and LaneTally sites

- [x] 5b.1 Unit 5b lowers a typed `tools.allow` at an inline Claude or LaneTally site onto its adapter's tool permissions, through unit 3's lowering and unit 4's origin transport, as the engine's own `local` segment behind the authored command. The D5.3 guard is lifted only for that shape, and every other inline shape keeps an exact refusal. Verify exact compiled facts, sealed records and the final Claude command, with an independent compiling mutation for each new test and row. Requirements: [Shipped inline permissions migrate before refusal lands][SCM], [Refusal proofs assert the full reason][SC8]. (inserted before 6.1 by the unit 6 split)

Observed 2026-09-24 (evidence.md, "Unit 5b — typed local tools at inline
Claude and LaneTally sites"), run `triage-directive-operator-ruling-b31f0b89`.
The design settles the allow's flag, origin and native composition (D5, D5.7,
D6, D5.3). The lowering is `agents::lower_allow`, which the agent path now
calls too. The compiler records it as `SiteFacts.inline_local`, and
`engine::compose_site_at` appends it behind the authored command. The
expected state seals it as listed/direct. One new agent_tests test (18 rows)
and four claude rows in the forms test were added. Obsolete D5.3 rows now carry
exact driver and sandbox causes. Two new capability_launch tests cover the
final Claude command and the LaneTally spawn and record. M1–M18 each bound and
were restored. fmt, clippy, the brokkr-runtime suite, both bundle compiles,
`openspec validate` and `git diff --check` passed. **Owed:** a dispatch-level
test in `engine/capability_tests.rs` that binds the label `Engine::compose_at`
hands over (unit 20's suite; outside this unit's inventory). **Open, operator's
to rule before unit 6:** how the adapter's acceptEdits template reaches an
inline command once ruling 1 removes the authored `--permission-mode`. **Not
fully green:** external exact coverage, macOS and remote CI are pending.

**Correction (5b-fix):** the tick at `efb61050` overstated completion, and
5b.1 completes only with 5b-fix below. Its council held the unit
(SECURITY-HOLD) on three counts. First, the inline path refused authored
tool lists only, so `--permission-mode bypassPermissions` beside a typed
allow compiled (S1). Second, "the final Claude command" was proved for
Claude and not for LaneTally, whose serving branch was never observed (C1).
Third, the dispatch handoff was unbound (C2).

- [x] 5b-fix Unit 5b-fix repairs unit 5b under its council's SECURITY-HOLD (chief S1, C1, C2, E1). On the inline typed path, every authored capability-bearing option refuses at compile with a bounded reason. That covers tool lists, MCP/plugin/settings/agents loads, `--strict-mcp-config` and `--permission-mode` in split and `=` forms, in the claude and lanetally grammars. The reason names the canonical option and its position and never its value. A provider-free LaneTally serving test proves the wrapper's whole ordered final argv with native OFF. An engine-dispatch test proves that each site composes its own lowered allow. An honest retrospective baseline red is recorded for 5b's positive behaviour. Requirements: operator ruling 1, [Refusal proofs assert the full reason][SC8]. (run `triage-directive-operator-ruling-5b7b5137`)

Observed 2026-09-24 (evidence.md, "Unit 5b-fix"). Production: `bundle.rs`
(`authored_capability_control` replaces the list-only check). The inline
lowering test grows from 18 to 28 exact rows, and capability_launch gains the
permission-mode compile regression on the shipped adapters, the LaneTally
serving test and its re-entered driver. `engine/capability_tests.rs` gains the
dispatch test (single, sequence step and two panel members, each with its own
list), as the commission permitted. The E1 retrospective restored `c10fc837`'s
runtime sources: the success assertion was red with the D5.3 refusal and
passed at `efb61050`. M1–M11 each bound and were restored. M2's rows still
refuse downstream, by the capability-server cause. M8, the survivor unit 5b
recorded, now fails C2. fmt, clippy, the brokkr-runtime suite, both bundle
compiles, `openspec validate` and `git diff --check` passed. **Open,
operator's:** the acceptEdits template for inline commands (unchanged). **Not
fully green:** external exact coverage, macOS and remote CI are pending.

**Correction (5b-fix2):** the tick above says every authored
capability-bearing option refuses. That claim was false, and 5b-fix completes
only with 5b-fix2 below. Its council held it (SECURITY-HOLD) for three
reasons: `--add-dir`, which the grammar types `Inert`, still compiled (S1);
grammar failures on the path echoed their raw token (S2); and the
`--dangerously-skip-permissions` row had no mutation of its own (E1).

- [x] 5b-fix2 Unit 5b-fix2 repairs unit 5b a second time, under the 5b-fix council's SECURITY-HOLD (chief S1, S2, E1), and sweeps the whole Claude/LaneTally grammar rather than one flag. On the inline typed path, `--add-dir` refuses in its split, `=`, variadic and repeated forms. So does each session selector, `--bg` and `--input-format`. `--effort` stands only with a value the adapter declares. Every grammar failure refuses by position, a bounded label and the grammar's cause, and never echoes its token. The sweep table, covering all 27 modelled options and the unmodelled catalogue names, is recorded with a citation for each. Requirements: operator ruling 1; realm-capability-grants "Authored provider configuration cannot supply capability authority"; [Refusal proofs assert the full reason][SC8]. (run `triage-directive-operator-ruling-ffd42ea5`)

Observed 2026-09-24 (evidence.md, "Unit 5b-fix2"). The only production file
changed is `bundle.rs`: `authored_capability_control` gains four arms, and
`unplaced_label` and the declared-effort check are new. `grammar.rs` is
unchanged; its misclassifications are noted for unit 10. The inline-lowers
test grows from 28 to 91 rows. capability_launch gains the shipped-adapter
`--add-dir` and permission-mode table for both drivers, and the
2048-character value test. At `fb201c93`, 64 of 91 rows and both launch tests
were red. M1–M8 (with M5a–d) each bound and were restored. M8 is the E1 row's
own mutation. fmt, clippy, the brokkr-runtime suite, the brokkr-cli init
suites, both bundle compiles, `openspec validate` and `git diff --check`
passed. **Open, operator's:** `--bg` and `--input-format` are refused fail
closed, because neither the delta nor the reference classifies them. The
acceptEdits template is unchanged. **Not fully green:** external exact
coverage, macOS and remote CI are pending.

**Correction (5b-fix3):** the tick above says `--effort` stands only with a
value the adapter declares, and sweep row 16 records that. Its council
returned a residual (maximum medium, security). Adapter membership is
declaration, not classification: an adapter that added `ultracode` admitted it
(R2). The refusal also printed the adapter's unbounded effort vocabulary (R1),
and an alias was labelled by the spelling written rather than its canonical
option (R3). 5b-fix2 completes only with 5b-fix3 below.

- [x] 5b-fix3 Unit 5b-fix3 repairs unit 5b a third time, under the 5b-fix2 council's residual (chief R1, R2, R3). On the inline typed path, an authored `--effort` stands only with one of the CLI reference's plain levels (`low`, `medium`, `high`, `xhigh`, `max`), whatever the adapter declares. Every other value refuses, `ultracode` included, under a fixed cause that names neither the value nor the adapter's vocabulary. An unplaceable alias is labelled by its canonical option. Requirements: operator ruling 1; realm-capability-grants "Authored provider configuration cannot supply capability authority" (bounded cause, canonical alias names); [Refusal proofs assert the full reason][SC8]. (run `triage-directive-operator-ruling-ffd42ea5`)

Observed 2026-09-24 (evidence.md, "Unit 5b-fix3"). The only production file
changed is `bundle.rs`. The adapter-membership effort check is removed.
`authored_capability_control` gains an `--effort` arm judged against the fixed
plain levels, and `unplaced_label` returns `spec.canonical`. `grammar.rs` is
unchanged. The inline-lowers test grows from 91 to 109 rows: 6 alias rows, 10
unplain-effort refusals and 4 plain admissions, over fixture adapters that
declare `ultracode` and a 2055-character name. capability_launch gains the
shipped-adapter effort test. At `8c2924f1`, 20 of 109 rows and the new launch
test were red. M1–M3 each bound and were restored. fmt, clippy, the
brokkr-runtime suite (550 lib tests and every integration suite), both bundle
compiles, `openspec validate --all --strict` (18 passed) and `git diff
--check` passed. **Open, operator's:** unchanged (`--bg`, `--input-format`,
the acceptEdits template). **Not fully green:** external exact coverage, macOS
and remote CI are pending.

## 5c. Unit 5c — Emit the adapter's permission template at inline typed sites

- [x] 5c.1 Unit 5c emits the adapter's declared permission template (for claude, `--permission-mode acceptEdits`) at an inline Claude or LaneTally site whose typed declaration lowers, as the engine's own `template` segment read from the adapter data, the way agent composition emits it. The sealed record carries it, and the launch parses it back with the rest of the final command. An authored permission mode stays refused, every inline shape that does not lower keeps its refusal, and a seat whose adapter declares no template gets none. Verify the whole ordered final command for an inline claude site and a LaneTally site, the no-template seat and the authored-mode refusal, each bound by a compiling mutation. Requirements: operator ruling of 2026-09-24 ("the permission template at inline sites"), operator rulings 1 and 2, [Refusal proofs assert the full reason][SC8]. (inserted before 6.1 by the operator's ruling (a); run `triage-directive-operator-ruling-9d597b08`)

Observed 2026-09-24 (evidence.md, "Unit 5c"). Production: `agents.rs`
(`driver_template`, which `compose` now calls, and `inline_template`, the
part of it behind the driver verb), `bundle.rs` (`SiteFacts.inline_template`,
recorded only beside `inline_local`) and `engine.rs` (`compose_site_at`
places it between the authored command and the `local` segment, the order an
agent's composition gives them). An adapter whose own driver does not
dispatch the site's driver is refused with an exact cause. The three 5b
launch tests now assert the template in the spawn, the sealed segments and
the whole ordered final Claude and LaneTally wrapper commands. These three
were red at `83a30446`. One new launch test covers a no-template adapter, and
the refusal test gained authored `acceptEdits` rows. One new agent_tests
table has 11 rows and did not compile at baseline, because the field was
absent. M1–M8 each bound and were restored. The expected state
(`native_controls::Expected`) has no template member. The template is sealed
as a `template`-origin segment of the launch record, which is how an
agent-backed seat records its own; a dedicated member would be a fourth
production file. fmt, clippy, the brokkr-runtime suite, both bundle
compiles, `openspec validate --all --strict` and `git diff --check` passed.
**Not fully green:** external exact coverage, macOS and remote CI are
pending.

Returned 2026-09-24 by review (evidence.md, "Unit 5c — review return").
**Reopened, blocked on a split.** R1 (high): the addendum's item 2 and this
unit say that the expected state records the template. The sealed record
above does not meet that. `Expected` is in
`crates/brokkr-protocol/src/native_controls.rs`, which is outside this unit's
three files, so the box above is unticked until a split unit lands. R2 (low)
is fixed: the no-template launch test builds on a canonicalised adapter root.
Emission, order, the no-template omission and the authored-mode refusal stay
as observed above.

The operator split 5c in two (run `triage-directive-operator-ruling-0bdb3908`;
design.md "Rebuild units", 5c-fix and 5c-fix2). 5c.1 closes when both have
landed. 5c-fix landed on 5c-fix-b's second return. 5c-fix2's code landed on
2026-09-25, but its review returned R1: four compile-required `Composition`
field lines sit outside its named test files and have no admission. So the
box above stays unticked until the operator admits them or rules a split
(5c-fix2.1 below). External exact coverage, macOS and remote CI stay
pending.

Closed 2026-09-25 with 5c-fix2 (5c-fix2.1 below), after the commissioner
admitted its four compile-required field lines (re-run note of 2026-09-25,
run `0065-rebuild-unit-5c-fix2-re-run-5112c091`). Emission and order stay as
observed at 5c, and the expected state now records the template on both
arms. External exact coverage, macOS and remote CI stay pending.

- [x] 5c-fix.1 Unit 5c-fix records the template in the expected state, inline arm. `Expected` gains a mandatory, kind-tagged `template` (`none` | `declared {argv}`) in its closed JSON value and `decode_record`, refusing missing, null, unknown-kind, malformed and extra members with fixed, value-free paths. The compiler records the adapter's declared template as a typed `SiteFacts` fact beside, and separate from, the emitted segment. `expected_state` fills the inline arm from that fact, and a seal whose template-origin segments contradict it is refused. Until 5c-fix2, an agent-backed seat records `none` only if its composition emitted none, and is otherwise refused. The omission, alteration and contradiction cases are each bound by a compiling mutation. Requirements: operator ruling of 2026-09-24 (item 2), operator ruling 2, [Refusal proofs assert the full reason][SC8]. (run `triage-directive-operator-ruling-0bdb3908`)

Observed 2026-09-24 (evidence.md, "Unit 5c-fix"). Production:
`native_controls.rs` (`TemplateExpectation`, `Expected.template`, its
encoding and strict decoding, and `declared []` refused as `none` spelled
twice), `bundle.rs` (`SiteFacts.declared_template`, recorded exactly where
`inline_local` is and expanded as the segment is) and `engine.rs`
(`inline_template`, `agent_template` and `permission_template`, and `seal`
now returns `Result` and refuses a contradicting emission with nothing
sealed). The agent-backed arm records `none` for a composition whose driver
template emits nothing behind its `<engine> driver <kind>` verb (codex, dsh,
the fixtures' opaque drivers) and refuses one that emits a template (every
shipped Claude and LaneTally agent seat). The new tests did not compile at
`33f95a61`. N1–N4, B1–B3 and E1–E7 each bound and were restored. fmt, clippy,
the brokkr-protocol and brokkr-runtime suites, both bundle compiles,
`openspec validate --all --strict` (18 passed) and `git diff --check`
passed. **Red, the ruled interim's consequence:** in `cargo test
--workspace`, brokkr-cli's `bootstrap_bench` (a pristine scaffold's
agent-backed Claude intake seat) parks on the agent-backed refusal. Every
other target passed. It stays red until 5c-fix2 or an operator ruling.
**Not fully green:** external exact coverage, macOS and remote CI are
pending.

Held 2026-09-24 by its council (SECURITY-HOLD, chief R1 high, R2 medium). The
seal and the agent arm judged only the first driver-template segment, so a
permission control carried by a later template contribution escaped both. The
operator commissioned the repair as 5c-fix-b. 5c-fix.1 closes on 5c-fix-b's
evidence, not on the evidence above alone.

- [x] 5c-fix-b.1 Unit 5c-fix-b judges every template contribution before expectation and sealing. Every `template`-origin segment behind an agent's driver template must be a model or effort pin free of any permission control (`native_controls::pin_fault`). The seal counts every other one as emitted, and the interim agent arm refuses it. An adapter whose `model_flag` or `effort_flag` spells a permission control is refused at compile with a bounded, value-free reason, and legitimate `--model`/`--effort` pins are preserved. The agent-backed omission, addition, alteration and contradiction proofs are each bound by a compiling mutation. The authorized test inventory is native_controls/tests.rs, bundle/agent_tests.rs, capability_launch.rs, the `Expected` constructors in agents/tests.rs and engine/capability_tests.rs, and 5c-fix's migrations there are admitted (R2). Requirements: operator ruling of 2026-09-24 (item 2), operator rulings 1 and 2, [Refusal proofs assert the full reason][SC8]. (run `triage-directive-operator-ruling-ce9cceae`)

Observed 2026-09-24 (evidence.md, "Unit 5c-fix-b"). Production:
- `native_controls.rs`: `permission_control` and `pin_fault`.
- `engine.rs`: `emitted_template` counts every later non-pin `template`
  segment, and `agent_template` requires every later one to be a pin.
- `bundle.rs`: `refuse_permission_pins`, covering the adapter declaration and
  the composition's pins.

The baseline reproduced all three of the chief's scenarios. The protocol
tables did not compile at `cc8cbd52`. P1–P6, E1–E3 and B1–B3 each bound and
were restored. fmt, clippy, the brokkr-protocol and brokkr-runtime suites,
both bundle compiles, `openspec validate --all --strict` (18 passed) and `git
diff --check` passed. **Red, unchanged:** brokkr-cli's `bootstrap_bench`,
from the interim agent-arm refusal, until 5c-fix2. **Not fully green:**
external exact coverage, macOS and remote CI are pending.

Returned 2026-09-24 by its council (residual, medium; no security residual,
no specification defect). **Correction:** that visit's evidence did not
support the tick above or 5c-fix.1's closure. Its permission-control list
omitted five specified controls: `--permission-prompt-tool`, `--add-dir`,
`--approve-for-me`, `--ignore-rules` and the `--yolo` alias. With one of them
declared as a dormant `effort_flag` on an effortless route, the seat compiled
and launched unrefused (R1). No independent mutation bound the two
driver-contradiction rows or the omission assertion (R2). The return visit
(evidence.md, "Unit 5c-fix-b — review return") completed the inventory in
`native_controls.rs`, so it now has eleven controls with long aliases. It
added dormant and emitted declaration rows with the exact refusal, and M1 to
M6 each bound and were restored. M4, M5 and M6 each fail exactly the omission
assertion or one contradiction row. The gates are recorded there, and the
pending items above still hold.

Returned again 2026-09-25 by its council (residual, medium; no security
residual, no specification defect). **Correction:** the first return's
evidence did not support the tick above or 5c-fix.1's closure either.
`permission_control` read only option names, so the specified Codex
permission/sandbox assignments (`-capproval_policy=never`,
`-c=sandbox_mode="danger-full-access"`,
`--config=sandbox_workspace_write.network_access=true`) were no control;
declared as a dormant `effort_flag` on an effortless Codex route, the seat
compiled, sealed `none` and launched (R1). The second return (evidence.md,
"Unit 5c-fix-b — second review return") classifies a whole assignment into or
under `approval_policy`, `sandbox_mode` or `sandbox_workspace_write` in every
config spelling, joins a split pin before judging it, and adds exact dormant
and emitted Codex declaration rows beside the legitimate model and effort
pins; N1 to N4 each bound and were restored. 5c-fix-b.1 and 5c-fix.1 close
on that return's evidence.

- [x] 5c-fix2.1 Unit 5c-fix2 records the template in the expected state, agent-backed arm: the composition carries the adapter's declared template as a typed fact, and `expected_state` fills the agent arm from it instead of refusing, so agent-backed Claude and LaneTally seats seal again with the template recorded and the contradiction check in force. Requirements: operator ruling of 2026-09-24 (item 2), operator ruling 2. (split from 5c by the operator; run `triage-directive-operator-ruling-0bdb3908`; commissioned in run `0065-rebuild-unit-5c-fix2-see-th-626be6dc`)

Observed 2026-09-25 (evidence.md, "Unit 5c-fix2"). Production:
- `agents.rs`: `Composition.template`, filled by `compose` from the
  adapter's `driver` declaration (`declared_template`) and never from the
  segments. `permission_template` moved here from `engine.rs`, unchanged.
- `bundle.rs`: `expand_lowering` expands the declared template as it
  expands the segment.
- `engine.rs`: the agent arm of `expected_state` records
  `composition.template`, and 5c-fix's interim agent-backed refusal
  (`agent_template`) is removed. The seal's contradiction check is
  unchanged and now judges agent-backed Claude and LaneTally seats against
  the recorded declaration.

The new tests did not compile at `b91ec0f7`, because the field was absent.
Four launch tests that pinned the interim refusal were red once it was
removed, and were rewritten to the recorded template. M1–M6 each bound and
were restored. fmt, clippy, `cargo test --workspace --all-features
--locked` (every target, brokkr-cli's `bootstrap_bench` green again), both
bundle compiles, `openspec validate --all --strict` (18 passed) and `git
diff --check` passed. **Exact coverage ran locally and is not green:** 5
lines and 3 branches in `bundle.rs` are uncovered, none of them in this
unit's diff (evidence.md records the baseline comparison). **Not fully
green:** exact coverage, macOS and remote CI are pending. 5c.1 closes with
this unit, and its emission and order stay as observed at 5c.

Review return, 2026-09-25 (evidence.md, "Review return, 2026-09-25: the
constructor migrations have no admission"). R1 (medium, scope): the three
engine test fixtures outside this unit's named test files were edited, and
the earlier inventories cited as cover do not apply to this unit. The field
lines are required to compile. The footprint is now as small as it can be:
`agents.rs` exposes `declared_template(driver)`, the fixture helper is
deleted, and the fixtures differ from `b91ec0f7` by exactly four `template:`
lines. M2′ was re-run on the reshaped function; it bound and was restored.
All gates passed again. **This box and 5c.1 stay unticked until the operator
admits those four lines into this unit's inventory, or rules a split.**

Admitted 2026-09-25 by the commissioner (the unit's re-run note of
2026-09-25, run `0065-rebuild-unit-5c-fix2-re-run-5112c091`): those four
`template:` lines in `engine/tests.rs`, `engine/capability_tests.rs` and
`engine/boundary_tests.rs`, and only those, join 5c-fix2's test inventory.
They are compiler-forced, add no assertion and change no behaviour. This is
the same kind of admission as the one made for 5c-fix-b's
`engine/capability_tests.rs`. The re-run adopted `207cab5e` and `5fe9d77b`
unchanged and checked their evidence on `5fe9d77b` (evidence.md, "Re-run,
2026-09-25: the admission, and the evidence on this head"). The three
fixtures still differ from `b91ec0f7` by exactly those four lines, M4 still
binds the same five launch tests, and every gate passed. This box and 5c.1
close. **Not fully green:** exact coverage on the final head, macOS and
remote CI stay pending.

## 5e. Unit 5e — Refuse a capability declaration in the wrong place

- [x] 5e.1 Unit 5e closes the site's `driver` object. An unknown key there (`tools`, `hands`, `capabilities`, `sandbox` or any other) refuses compilation with a bounded, value-free reason that names the site, the `driver` object and the key, and says where a capability key belongs. It is never ignored. First establish, citing code, whether this is a bug or an omission, and fix it at the narrowest point. The self and verify bundles and every recipe under `recipes/` still compile. Verify the exact refusal for `tools` under `driver` and for a second misplaced capability key, and the positive for a seat-level `tools`, each bound by a compiling mutation. Requirements: decision 0004 (closed input semantics), [Refusal proofs assert the full reason][SC8]. (inserted by operator commission after unit 7's probe; run `0065-rebuild-unit-5e-see-the-uni-90209c7a`)

Observed 2026-09-25 (evidence.md, "Unit 5e"). It was an omission. The site
vocabularies (`SEAT_KEYS`, `BODY_KEYS`, `MEMBER_KEYS`, `STEP_KEYS`) were
closed, but nothing checked the inside of an inline `driver`: only
`command` was read, and `confine` was refused by name. Production:
`bundle.rs` only. `DRIVER_KEYS` and `refuse_driver_keys` are called after
`refuse_confine` at the seat, selected-body, member and step sites. Tests:
three in `bundle/agent_tests.rs` (unit 7's `tools` shape, six other keys
including a 65-byte one, and the seat-level positive) and one in
`bundle/tests.rs` (step, member and select case). M1–M7 each bound and
were restored. M1 reproduces unit 7's fail-open as `compiled … sandbox:
None`. All 18 shipped bundles compile. fmt, clippy, the brokkr-runtime
suite (557 lib tests, 25 binaries), `openspec validate --all --strict` and
`git diff --check` passed. A local llvm-cov diagnostic covers every line
and branch of `refuse_driver_keys`. **Not fully green:** exact coverage,
macOS and remote CI are pending. A follow-up is named and not fixed here:
the bundle root has no unknown-key check.

Corrected 2026-09-25 (run `0065-rebuild-unit-5e-fix-see-the-87ebafa9`,
after the council on `dcd900d1` ruled SECURITY-HOLD). Unit 5e alone did not
close 5e.1, and its tick above overstated what it proved. What 5e proved: the
`driver` object is closed at all four sites, with the exact reasons and M1–M7
recorded. What it did not prove: (1) the unit named "any other object a recipe
author could plausibly put `tools`, `hands` or `sandbox` into", and the bundle
root was one. 5e deferred it as a follow-up instead, so a root `tools`, `hands`
or `sandbox` still compiled and confined nothing (chief R1). (2) The reason was
not bounded. It interpolated the author-written site label whole, so a
100,000-character member name produced a 100,427-byte refusal (chief R2).
(3) `bundle/tests.rs`'s `Fixture` compiled from a temporary root that was not
canonicalised (chief R3). 5e.1 closes only together with 5e-fix below.

- [x] 5e-fix.1 Unit 5e-fix repairs 5e. A capability-bearing key (`tools`, `sandbox`, `hands`, `capabilities`, `driver`, `boundary`) written at the root of any composition layer, whether leaf or base, refuses compilation. The bounded, value-free reason names the layer and the key. Every refusal this unit and 5e add renders the site identity bounded and safe. The every-site test's fixture compiles from one canonical temporary root. Verify the exact refusals for root `tools`, `hands` and `sandbox`, and a long-name regression for both refusals, each bound by a compiling mutation. The self and verify bundles and every recipe under `recipes/` still compile. Requirements: decision 0004 (closed input semantics), [Refusal proofs assert the full reason][SC8]. (inserted by operator commission after 5e's council; run `0065-rebuild-unit-5e-fix-see-the-87ebafa9`)

Observed 2026-09-25 (evidence.md, "Unit 5e-fix"). Production: `bundle/compose.rs`
(`ROOT_CAPABILITY_KEYS` and `refuse_root_capabilities`, called first in
`merge_layer`, so every layer is checked) and `bundle.rs` (`bounded_site`,
used by `refuse_driver_keys` and the root refusal). Tests:
`compose_tests.rs::a_capability_declared_at_a_bundle_root_is_refused_at_every_layer`
and `tests.rs::a_driver_refusal_names_a_long_or_unsafe_site_boundedly`.
`Fixture` now carries a canonical `root`. MA–MI each bound and were
restored. All 18 shipped bundles compile with the same top-level digests
5e recorded. fmt, clippy, the brokkr-runtime suite (25 binaries; the lib
reports 559 passed), `openspec validate --all --strict` (18 passed) and
`git diff --check` passed. **Not fully green:** exact coverage, macOS and
remote CI are pending. The root is still open to keys other than these six.
Closing it entirely is a follow-up.

Corrected 2026-09-25 (run `0065-rebuild-unit-5e-fix-b-see-t-74cfe22d`,
after the council on `17810f7c` ruled SECURITY-HOLD). 5e-fix did not close
5e-fix.1, and its tick above overstated what it proved. (1) Six named keys
were not "a capability-bearing key": `confine`, `allow`, `mcp`, `network` and
any other key still compiled at a standalone, inherited or derived root and
confined nothing (chief R1; reproduced on `17810f7c`, evidence.md "Unit
5e-fix-b"). (2) The chain note on a composed bundle's refusal still echoed
the leaf's and every ancestor's declared name whole, newlines included
(chief R2). (3) The `hands` row's value `{workspace: rw, …}` is not valid
hands, and its `tools` control ran `resolve` only, so neither was a
production-compiling control; `tests.rs`'s long-name test had no valid
control (chief R3). (4) No baseline red of the new regressions on `dcd900d1`
was run before the repair; the one now recorded is retrospective (chief R4).
5e-fix.1 closes only together with 5e-fix-b below.

- [x] 5e-fix-b.1 Unit 5e-fix-b repairs 5e-fix. Every composition layer's root is a closed vocabulary: `name`, `description`, `cost`, `policy`, `protected_phase`, `egress_minimum`, `seats`, `extends`, `override` and `remove`, each cited to the code that reads it, and any other key refuses compilation with a bounded, value-free reason naming the layer and the key. The chain note renders leaf and ancestor names bounded and safe. Verify exact refusals for `confine`, `allow`, `mcp`, `network` and an arbitrary key at standalone and inherited roots, each bound by a compiling mutation with a baseline red on `17810f7c`; the long/unsafe leaf and ancestor names through `Bundle::compile`; production-valid controls paired with each regression. The self and verify bundles and every recipe still compile. Requirements: decision 0004 (closed input semantics), [Refusal proofs assert the full reason][SC8]. (inserted by operator commission after 5e-fix's council; run `0065-rebuild-unit-5e-fix-b-see-t-74cfe22d`)

Observed 2026-09-25 (evidence.md, "Unit 5e-fix-b"). Production:
`bundle/compose.rs` (`ROOT_KEYS` and `refuse_unknown_root_keys` replace the
six-key list; `chain_note` bounds each name) and `bundle.rs` (`plain_label`
and `safe_label_byte` factored out of `bounded_site`). No third file. Tests:
`compose_tests.rs::a_bundle_root_is_a_closed_vocabulary_at_every_layer`
(replaces 5e-fix's root test), `compose_tests.rs::a_composed_refusal_names_long_or_unsafe_layers_boundedly`,
and a paired control in `tests.rs::a_driver_refusal_names_a_long_or_unsafe_site_boundedly`;
one stale-marker row moved from the arbitrary root key `absent` to
`egress_minimum`. Baseline: all three red on `17810f7c`, and `confine`,
`allow`, `mcp`, `network`, `frobnicate`, a 100,000-byte key and `evil\nkey`
compiled at every root. MJ–MU and per-key MK/ML each bound and were
restored. All 18 shipped bundles compile with the digests 5e-fix recorded.
fmt, clippy, brokkr-runtime (25 of 25 binaries, lib 560 passed),
brokkr-cli (lib 481 passed, 30 of 30 integration targets),
`openspec validate --all --strict` (18 passed) and `git diff --check`
passed. **Not fully green:** exact coverage, macOS and remote CI are pending.

Corrected 2026-09-25, in the same run's return visit after review of
`0a98fa55` (evidence.md, "Unit 5e-fix-b", "Return visit"). (1) "All three
red on `17810f7c`" overstated what was observed. The root test failed only
on its first row, `tools`, and never reached the commissioned keys. The
driver test failed on a wrong expectation in its own control, which is not
enforcement proof. That test is a control and passes on `17810f7c`. The
return visit then ran the committed assertions against `17810f7c`
production, filtered to one row at a time. It observed every per-key red:
`confine`, `allow`, `mcp`, `network` and `frobnicate`, each at the
standalone (`:1912`), inherited (`:1913`) and derived (`:1916`) roots,
each compiling to the recorded digest. It also observed the chain-note
test's remaining rows red at `:1996`. These are real observations of the
old code, made after the repair rather than before it. (2) The
100,000-byte-named standalone refusal gained its control, the same bytes
without `confine` compiling under that name. Mutation MV bound it at
`compose_tests.rs:1938`, and it was restored. fmt, clippy and
brokkr-runtime (25 of 25, lib 560), `openspec validate --all --strict` (18
passed) and `git diff --check` passed. The self and verify digests are
unchanged. **Not fully green:** exact coverage, macOS and remote CI
are pending.

## 5d. Unit 5d — Lower a typed sandbox at inline Codex sites

- [x] 5d.1 Unit 5d lowers a typed `tools.sandbox` at an inline Codex site through unit 5b's inline lowering and unit 5c's template recording: the engine appends the adapter's sandbox control as its own engine-owned segment, the expected state records the class as `Expected.local.sandbox`, and the seal parses it back. Exactly `workspace-write` is admitted at a work site and `read-only` at a gate; every other class, and any sandbox at a gate that is not read-only, refuses with a bounded, value-free reason. An inline Codex gate at read-only delivers through the last-message door, and the door is shown to be selected. Verify the whole final command for an inline Codex work site and gate, each refused class and a contradiction refused at the seal, each bound by a compiling mutation. Migrate no recipe. Requirements: operator ruling of 2026-09-25 ("narrow"), operator rulings 1 and 2, decision 0046 ruling 4, [Refusal proofs assert the full reason][SC8]. (inserted before 7.1 by the operator's ruling of 2026-09-25; run `0065-rebuild-unit-5d-see-the-uni-5b7d59c1`)

Observed 2026-09-25 (evidence.md, "Unit 5d"). Step 0 is `c7c9ba09`.
Production is `bundle.rs` and `engine.rs`; `native_controls.rs` did not
move, because `Expected.local.sandbox` already carries the class.

- `bundle.rs`: `lower_inline_sandbox` and `authored_sandbox_control`;
  `SiteFacts.inline_sandbox` (`InlineSandbox`); `record_inline_tools`
  lowers only at a seat, so nested bodies keep the refusal; the inline
  native plan is judged by `admit_native_sandbox`.
- `engine.rs`: `compose_site_at` appends the class as the engine's `local`
  segment, with the result path filled in. The inline arm of
  `expected_state` records the class. `seal` parses the `local` segments
  back (`local_sandbox_agrees`). `result_door`, which `mark_delivery` now
  calls, selects `last-message` at an inline Codex gate.

Tests: one new 19-row test in `bundle/agent_tests.rs`, four codex rows in
the nested-forms test, and two in `tests/capability_launch.rs`. Those two
cover the whole final command and door for the work seat and the gate, and
a 9-row seal-contradiction table. The new tests do not compile at the
baseline. M1–M16 each bound and were restored. A standing-admission
fixture line in `engine/capability_tests.rs` is recorded in the evidence.

fmt, clippy, brokkr-runtime (25 of 25 binaries, lib 561), both bundle
compiles (digests unchanged), `openspec validate --all --strict` and
`git diff --check` passed. No recipe was migrated. **Not fully green:**
exact coverage, macOS and remote CI are pending. The local llvm-cov
diagnostic could not run from this session.

**Correction (5d-fix):** the tick above overstated completion. 5d's council
held the unit (SECURITY-HOLD, spec_defect=true). The problems it found:

- the template and every non-`local` contribution went unjudged for
  competing sandbox effects (F1);
- the gate's capture was never required or bound to the engine's result
  path (F2);
- the migration scenario still named danger-full-access (F3);
- the baseline was a build error (F5);
- the work/read-only row had no independent mutation (F6).

5d.1 completes only with 5d-fix below. Its four `engine/capability_tests.rs`
fixture lines are **admitted under the 2026-09-25 extension of the standing
admission** (operator-ruling-2026-09-23.md, addendum "2026-09-25: the
standing admission extends to unit 5d"): the standing
admission names units 7–27, not 5d. Nothing is ticked on the strength of
that admission.

- [x] 5d-fix Unit 5d-fix repairs unit 5d under its council's SECURITY-HOLD (chief F1–F6). At an inline Codex site whose typed class the engine lowers, every contribution (authored, template, local and native) is judged at admission, and every spawn segment again at the seal, by the codex grammar's effect classification. Only the engine fragment's one `--sandbox` bears the sandbox. Any other permission control, writable root, load, root selector, sandbox or approval configuration, or unbounded assignment refuses with a bounded, value-free reason. A gate delivers only through the last-message door: file delivery, a missing capture and a capture in another contribution or to another target refuse at admission and again at the dispatch door against the engine-owned result path. A work seat carries no capture. The migration scenario follows the narrow ruling. Requirements: operator ruling of 2026-09-25 ("narrow"), operator rulings 1 and 2, decision 0046 ruling 4, design D5.3, [Refusal proofs assert the full reason][SC8]. (run `0065-rebuild-unit-5d-fix-see-the-569be761`)

Observed 2026-09-25 (evidence.md, "Unit 5d-fix"). Production is `bundle.rs`
(`codex_contributions`, `inline_codex_competitor`, `inline_codex_capture`,
wired into `lower_inline_sandbox` and `admit_native_sandbox`) and
`engine.rs` (`local_sandbox_agrees` and `verify_record`, through
`local_class`). `native_controls/grammar.rs` did not move. There is a
19-row agent test and a 25-row launch test.

On `e3d08c9f` they failed 18 of 19 and 23 of 25 rows at their assertions;
only the positives passed. F5 has behavioral probes at `1c7c9884`, red at
their `assert_eq!` and passing on the fix head. F6 has a compiling mutation
that fails `work, read-only` alone. MA–MT each bound and were restored.

fmt, clippy, brokkr-runtime (25 of 25 results, lib 562), both bundle
compiles (digests unchanged), `openspec validate --all --strict` and `git
diff --check` passed. **Not fully green:** workspace exact coverage, macOS
and remote CI are pending, and so is the F4 ruling.

**Correction (5d-fix-b):** the 5d-fix tick above overstated completion. Its
council held it again (SECURITY-HOLD, spec_defect=true):

- the launch never judged the native plan the driver appends (F1);
- a native `-o` escaped as inert, so a work capture or a second gate capture
  reached the final command (F2);
- `profile`/`profiles` configuration passed the three-table denylist (F3);
- the capture followed the recorded door alone, so a gate changed to `file`
  with its capture removed launched (F4);
- the launch refusal named no seat (F5);
- the 5d-fix evidence excused all of this (F6).

5d-fix completes only with 5d-fix-b below. The four
`engine/capability_tests.rs` fixture lines stay **admitted pending operator
ruling (F4)**; 5d-fix-b adds no out-of-inventory line.

- [x] 5d-fix-b Unit 5d-fix-b replaces 5d-fix's per-contribution denylist with one judgment of the whole inline Codex launch, `bundle.rs::inline_codex_launch`, run at admission over the compiled plan (`admit_inline_launch`: authored, template, `local` fragment and each resolved native plan) and again at the dispatch door over the argv the driver is handed (`engine.rs::inline_codex_door`: the sealed extras and the input's native plan). It admits a closed set: exactly one `--sandbox` of the site's class in the engine's `local` fragment; at a gate exactly one capture into the engine-owned result path in that fragment, at a work seat none; configuration only on the grammar's cited allowlist (`grammar.rs::LAUNCH_SETTINGS`: `model_reasoning_effort` at the adapter's levels, `web_search="disabled"`); and data or switch options. Every other effect, key or unplaceable option refuses with a bounded, value-free reason naming the seat, the contribution, the position and the option. The door and the capture follow the admitted class, never the recorded door alone. Requirements: operator ruling of 2026-09-25 ("narrow"), operator rulings 1 and 2, decision 0046 ruling 4, design D5.3, [Refusal proofs assert the full reason][SC8]. (run `0065-rebuild-unit-5d-fix-b-see-t-8067eebc`)

Observed 2026-09-25 (evidence.md, "Unit 5d-fix-b"). Production is `bundle.rs`,
`engine.rs` and `native_controls/grammar.rs`; tests are
`bundle/agent_tests.rs` (a 38-row admission table and a 4-row direct
judgment table) and `tests/capability_launch.rs` (a 26-row launch table and a
17-row door table). On `6407fb3e` the native launch rows all launched, and so
did the gate changed to `file` with no capture. Seven admission rows compiled
there. M1–M15 each compiled, bound and were restored. See evidence.md for the
gates. **Not fully green:** workspace exact coverage, macOS and remote CI are
pending, and so is the F4 ruling.

**Correction (5d-fix-c1, chief F5 of run
`0065-rebuild-unit-5d-fix-b-see-t-8067eebc`):** the 5d-fix-b tick above
overstated completion. Its council held it (SECURITY-HOLD, spec_defect=true):

- the dispatch door accepted a missing `native_controls` key and a plan with
  an empty argv, so the sealed web-search denial could be dropped (F1);
- the judgment ran before the Codex adapter composes the actual command
  (`--effort` translation, generated `--json` and `-C`, resume), so it did
  not judge the whole launch (F2);
- the plan refusal forwarded the decoder's cause, which can carry the
  plan's own text (F3);
- the site was spelled raw at admission and as `(unnamed)` for a dotted
  label at the door (F4);
- the tick and the evidence treated a pre-driver check as the whole launch
  and exempted `--effort` as inert despite its translation (F5). The counts
  above were "3-row" and "M1–M14"; they are corrected to the suite's four
  rows and M15.

5d-fix-b completes only with 5d-fix-c1 and 5d-fix-c2 below, split under the
preamble's three-file ceiling (design.md, Rebuild units, 5d).

- [x] 5d-fix-c1 Unit 5d-fix-c1 moves the whole-launch judgment into `native_controls/grammar.rs` as one pure public function, `judge_inline_codex_launch(class, argv, owned_capture) -> Result<(), BoundedCause>`, which `bundle.rs` admission and `engine.rs`'s dispatch door both call, with unchanged behaviour. At the door, wherever the sealed expectation carries a native denial, a missing `native_controls` key and a plan with no argv refuse, and each sealed denial must be expressed by the delivered argv as the grammar reads it (`inline_codex_denials`), never taken from the plan's claim (F1). A null or unreadable plan refuses with one fixed cause, never the decoder's (F3). Admission and the door name the seat in one bounded representation (`bundle.rs::bounded_site`, through `inline_codex_refusal`), so a dotted label keeps its identity and a long or control-character label is named by its lead and length (F4). The 5d-fix-b artifacts are corrected (F5). Requirements: operator ruling of 2026-09-25 ("narrow"), operator rulings 1 and 2, decision 0066, design D5.3, [Refusal proofs assert the full reason][SC8]. (run `0065-rebuild-unit-5d-fix-c1-see--b800f52a`)

Observed 2026-09-25 (evidence.md, "Unit 5d-fix-c1"). Production is
`grammar.rs`, `bundle.rs` and `engine.rs`. `adapters.rs` did not move. Tests:
- `bundle/agent_tests.rs`: the direct judgment table now has 5 rows, adding
  "no class". A new 3-row admission test covers plain, dotted and long
  labels.
- `tests/capability_launch.rs`: a new 11-row door test, and 2 corrected rows
  in the 17-row door table.
- `native_controls/tests.rs`: a denials test.

On `79a0ba86`:
- every F1 row launched: no plan at the work seat and at the gate, an
  empty argv, and the OFF replaced by an admitted effort at both;
- the unreadable-plan row carried the newline and the sentinel into the
  reason;
- the dotted seat was `(unnamed)` at the door;
- the long label was spelled raw at admission.

N1–N10 and a re-run of 5d-fix-b's M3, M10, M13 and M15 each compiled, bound
and were restored. fmt, clippy, both crate suites, both bundle compiles
(digests unchanged), every recipe, `openspec validate --all --strict` and
`git diff --check` passed. **Not fully green:** workspace exact coverage,
macOS and remote CI are pending, and so is the F4 fixture ruling of unit
5d-fix.

- [x] 5d-fix-c2 Unit 5d-fix-c2 runs `judge_inline_codex_launch` at the actual composition boundary, `adapters.rs::codex_command`, over the command the Codex adapter composes, including the `--effort` translation into `model_reasoning_effort` (an out-of-level effort such as `ultra` must refuse), generated `--json` and `-C` (an authored `--json` duplicating the driver's must refuse), and the `exec resume` transformation. It proves each transformed or generated path with exact regressions bound by compiling mutations. Production: `crates/brokkr-protocol/src/adapters.rs`. Tests: adapters/tests.rs and crates/brokkr-runtime/tests/capability_launch.rs. Requirements: chief F2 of run `0065-rebuild-unit-5d-fix-b-see-t-8067eebc`, operator ruling 2 of 2026-09-23 ("the launch proves itself"). The commission added chief F1 of run `0065-rebuild-unit-5d-fix-c1-see--b800f52a` (a quoted OFF key passes the denial proof) and `native_controls/grammar.rs` as the second production file. (run `0065-rebuild-unit-5d-fix-c2-see--ba7515f2`)

Observed 2026-09-25 (evidence.md, "Unit 5d-fix-c2"). Production is
`adapters.rs` and `grammar.rs`:
- **C1-F1:** `launch_setting` reads a `-c` assignment the way the harness
  does, or refuses it. A key counts only when spelled canonically, and
  nothing may stand around the `=`. So a double-, single- or partly quoted
  key and a spaced key are refused with one fixed cause, and none of them
  proves a denial.
- **F2:** `codex_launch_and_cold` judges both commands it can spawn, the
  plan's (cold or rejoin) and the cold replacement. It does so wherever the
  sealed record names the engine's `local` class, through the new
  `grammar::judge_inline_codex_command`, which calls
  `judge_inline_codex_launch`. That covers the translated effort, the
  generated `--json` and `-C`, and the rejoin's class assignment.

Tests:
- `adapters/tests.rs`: a 14-row cold table, a 4-row rejoin table and an
  attribution test.
- `tests/capability_launch.rs`: a new 10-row test, and the launch helper
  now hands the driver the record.

On `06a03f99` every targeted row launched. The partly quoted dotted key,
which the door already refused, did so under another cause. M1–M12 and M1'
each compiled, bound and were restored. fmt, clippy, the protocol, runtime
and cli suites, both bundle compiles (digests unchanged),
`openspec validate --all --strict` and `git diff --check` passed. **Not
fully green:** workspace exact coverage, macOS and remote CI are pending,
and so is the F4 fixture ruling of unit 5d-fix. 5d-fix-b's F2 is closed by
this unit.

## 6. Unit 6 — Migrate Claude recipes

- [x] 6.1 Unit 6 migrates fast/node/preflight to typed tools. Verify exact compiled local limits/native OFF and measure moved pins. Requirement: [Shipped inline permissions migrate before refusal lands][SCM]. Reopened/remaining: operator ruling 1–2. (previous 3.17)

Observed 2026-09-24 (evidence.md, "Unit 6 — blocked: inline sites have no
typed lowering"), run `triage-directive-operator-ruling-b5e42a33`. **Blocked,
not started.** Every seat to migrate is an inline Claude driver site. At
`c10fc837`, `bundle.rs::record_inline_tools` still refuses any typed
`tools.allow` there under D5.3, and no earlier unit lifts that guard or lowers
a typed list into an inline command. A probe of preflight's reviewer, migrated
as the Migration Plan says, was refused at compile. The only composing path is
agent-backed, and it needs new or widened agent files, which the design
rejects. The unit's three data files cannot close 6.1. It needs a prior
inventoried unit that lowers typed local tools at inline Claude/LaneTally
sites. No production, test or pin byte moved.

Observed 2026-09-25 (evidence.md, "Unit 6 — oversized: two fixtures outside
the inventory"), run `0065-rebuild-unit-6-see-the-unit-d4429c6f`, on
`c1db06b6`. **Oversized, not closed.** With 5b and 5c landed, the five seats
migrate within the three recipes. A new `capability_launch.rs` test was red at
the baseline (at its assertion) and green on the migrated tree. The final
command bytes are unchanged, and the ownership moves to the `template` and
`local` origins. The migration also breaks two fixtures outside the
inventory: `crates/brokkr-runtime/tests/node_recipe_gates.rs` (3 tests) and
`crates/brokkr-cli/src/tests.rs` (2 tests). Each swaps the shipped Claude
driver for a fixture driver, which then meets the retained D5.3 inline
refusal. A one-line `remove("tools")` in each fixture was the whole extra
footprint (scratch check). The operator's admission of those two lines is
needed. The patch is saved uncommitted at
`.forge/unit-6-d4429c6f-migration.patch`, and no production, test or pin byte
is committed. Re-checked on `e0b26369` after triage's re-framing: the same
3 + 2 fixture failures reproduce, and no admission exists yet. Still oversized.

Admitted 2026-09-25 by the commissioner (the unit's second re-run note of
2026-09-25, run `0065-rebuild-unit-6-second-run-s-d42be1db`): exactly one
`remove("tools")` line in `crates/brokkr-runtime/tests/node_recipe_gates.rs`
(`Fixture::new`, beside the provider re-point) and exactly one in
`crates/brokkr-cli/src/tests.rs` (`stage_hands_free_fast`, beside the
fake-driver swap) join unit 6's test inventory, and nothing else does
(evidence.md, "Unit 6 — second re-run: admitted, migrated and closed"). The
run adopted `e0b26369` and `6a6e18ea`, applied the saved patch on `6a6e18ea`
(the patch already carries both admitted lines), and measured seven moved
witness pins and two moved compose pins from the tests' own values. The new
launch test was red on the unmigrated recipes and was bound by three more
compiling mutations, each restored. fmt, clippy, `cargo test --workspace
--all-features --locked` (every target green), both bundle compiles,
`openspec validate --all --strict` (18 passed) and `git diff --check`
passed. This box and 6.1 close. **Not fully green:** exact coverage, macOS
and remote CI are pending.

## 7. Unit 7 — Migrate verify and Codex restrictions

- [x] 7.1 Unit 7 migrates verify/standby/review-first to typed permissions/sandbox. Verify exact commands, unchanged boundary authority and measured pins. Requirement: [Shipped inline permissions migrate before refusal lands][SCM]. Reopened/remaining: operator ruling 1–2. (previous 3.18)

Observed 2026-09-25 (evidence.md, "Unit 7 — oversized: inline Codex sites
have no typed sandbox lowering"), run `0065-rebuild-unit-7-see-the-unit-d8cb2dcd`,
on `d9a20de4`. **Oversized, not started.** Standby's two Codex seats and
review-first's Codex reviewer are inline sites. `bundle.rs::record_inline_tools`
still refuses any inline `tools.sandbox`, as D5.3 and unit 5b's text require.
Both recipes, migrated as the Migration Plan says, were refused at compile with
that exact reason. The D5.3 table also admits no `danger-full-access` and no
gate `workspace-write`, which are the classes these seats carry. The verify
reviewer's typed allow compiled (patch saved at
`.forge/unit-7-d8cb2dcd-verify-probe.patch`, untested). The unit needs a prior
inline-Codex-sandbox lowering unit (`bundle.rs`, `engine.rs`, possibly
`native_controls.rs`) and an operator ruling on the inline class authority.
No production, test or pin byte moved.

Observed 2026-09-25, second visit of the same run after triage re-ruled
`chore` (evidence.md, "Unit 7 — blocked on the second visit"), on `de16b442`.
**Blocked, not started.** No code or design byte moved since the first
visit. Review-first migrated as the Migration Plan says was refused again with
the same exact reason. D5.3 and unit 7 contradict each other, and only the
operator can resolve that. No production, test or pin byte moved.

Observed 2026-09-25 (evidence.md, "Unit 7 — verify and the inline Codex
seats migrated, narrowed"), run `0065-rebuild-unit-7-see-the-unit-c9f23334`
on `82e2dc61`. Unit 5d and the "narrow" ruling answered both blocks. Step 0
(the 5d extension addendum) was committed alone as `5b9ed910`. The verify
reviewer declares a typed allow that keeps both narrow gh prefixes.
Standby's implementer moves to typed `workspace-write` (it was
danger-full-access). Standby's reviewer and review-first's reviewer move to
typed `read-only` (they were workspace-write) and deliver through the
last-message door. No authored capability flag is left in the three files.
The new `capability_launch` test pins the four final commands, origins,
expected states and doors exactly. It was red on the unmigrated recipes
and was bound by three compiling mutations, each restored: a dropped gh
name, an unrestricted `Bash(gh:*)` mapping, and a gate put back at
workspace-write. The manifest's boundary, hands, capabilities and realms
are identical before and after. The `bundles/verify` witness pin and
compose pin were re-measured (`7263ad36…`). No line was needed under the
standing admission. fmt, clippy, `cargo test --workspace` (77 result lines,
all ok), both bundle compiles, `openspec validate --all --strict` (18
passed) and `git diff --check` passed. **Not fully green:** exact coverage,
macOS and remote CI are pending.

## 8. Unit 8 — Migrate wager and finish the inventory

- [x] 8.1 Unit 8 migrates wager-harness/advice and reruns all-directory inventory. Verify no shipped authored catalogue flag remains before refusal and no local limit broadens. Requirement: [Shipped inline permissions migrate before refusal lands][SCM]. Reopened/remaining: operator ruling 1–2. (previous 3.19)

Observed 2026-09-25 (evidence.md, "Unit 8 — wager-harness narrowed, and
the inventory re-run"), run `0065-rebuild-unit-8-see-the-unit-6b5868ee`
on `8adebd38`. Wager-harness's Codex implementer moves its authored
`--sandbox danger-full-access` to a typed `tools.sandbox` of
`workspace-write`. It is **narrowed** under the 2026-09-25 ruling, which
admits danger-full-access nowhere. The manifest, less `files`, is
byte-identical before and after. Both READMEs now give typed examples.
The node fork table's quoted refusal for an unmapped `pnpm` was observed.
The re-run inventory records a disposition for adapters, recipes, agents,
extensions, bundles and scaffolds. No newly discovered shipped migration
remains. Two new `capability_launch` tests bind the result. One pins the
seat's whole final command, origins, expected state and door. The other
sweeps every authored command under `recipes/`, `bundles/` and `agents/`
and finds no capability-bearing option. Both were red on the unmigrated
bundle. Three compiling mutations, each restored, made them fail: typed
`danger-full-access`, which is refused at compile with the narrowing
reason; the typed block removed; and an `--mcp-config` put in another
recipe. The `recipes/node` and `recipes/wager-harness` witness pins were
re-measured. No line was needed under the standing admission. fmt,
clippy, `cargo test --workspace` (77 result lines, all ok), both bundle
compiles, `openspec validate --all --strict` (18 passed) and `git diff
--check` passed. **Not fully green:** exact coverage, macOS and remote CI
are pending. Follow-up: `docs/guides/adopting-a-node-repo.md:226` still
gives the `--allowedTools` advice and is outside every unit's inventory.

## 9. Unit 9 — Qualify a supported nonempty restriction

9.1 is **deferred** to the restriction-transport slice by the operator's addendum
of 2026-09-25 (operator-ruling-2026-09-23.md, "the nonempty restriction
positive is deferred"; design.md D11). Unit 9 is closed by its return
(8338881a). The task, unticked, is under "Deferred to the restriction-transport
slice" below.

## 10. Unit 10 — Bound the grammar and redact diagnostics

- [x] 10.1 Unit 10 inventories every option/form/effect/consumer, including engine/wrapper/session positions and origins. Verify complete all-harness catalogue tests. Requirements: [Known provider commands have a closed argument grammar][RGP], [Every accepted native control reaches the final command][NCC]. Reopened/remaining: operator ruling 1–2. (previous 3.9)

- [x] 10.2 Unit 10 models five Codex forms, quoted/table/descendant/repeated keys and bounded inert configs. Verify malformed/unbounded refusal and value-redacted complete diagnostics. Requirements: [Known provider commands have a closed argument grammar][RGP], [Authored provider configuration cannot supply capability authority][RGR]. Reopened/remaining: operator ruling 1–2. (previous 3.10) Return visit 2026-09-25 (review F1; evidence.md, "Unit 10", "Return visit"): a refusal at a reserved Codex position (`exec`, session, stdin) now carries the fixed positional label, whatever the token spells. Baseline red and three per-slot mutations are recorded. Exact coverage, macOS and remote CI are pending.

- [x] 10.3 Unit 10 models Claude/LaneTally grammar without authored-list contributions. Verify managed patterns/separators, aliases, empties, inert prompts and wrapper boundaries. Requirements: [Known provider commands have a closed argument grammar][RGP], [Prompt values cannot absorb a composed control][NCP], [Explicit restrictive tool lists retain their meaning][NCT]. Reopened/remaining: operator ruling 1–2. (previous 3.11) Return visit 2026-09-25 (review F2; evidence.md, "Unit 10", "Return visit"): M25 and M26 bind `a_prompt_value_is_data_and_never_absorbs_a_control`. Their failing assertions and the restored pass are recorded.

- [x] 10.4 Unit 10 retains DSH closed grammar and bound route patch. Verify profile/web/plugin/capability/unknown/changed-patch refusals and route positive after #313/#326. Requirements: [Known provider commands have a closed argument grammar][RGP], [Authored provider configuration cannot supply capability authority][RGR], [Other provider declarations preserve the commissioned uncertainty][NC5]. Reopened/remaining: operator ruling 1–2. (previous 3.12)

Observed 2026-09-25 (evidence.md, "Unit 10"), run
`0065-rebuild-unit-10-see-the-uni-7614ce83`, on `8338881a`. Production:
`native_controls/grammar.rs` only. Tests: `native_controls/tests.rs`. No
standing-admission line was used.

- **Diagnostics.** A grammar refusal names a bounded label, never the
  token. The label is a canonical name, a plain long name, the
  terminator, or a fixed positional or unmodelled label. As D6 requires,
  the option/cause portion (label plus cause) stays within 512 scalar
  values. The fixed prose around it is outside that bound: two
  `--dangerously-bypass-approvals-and-sandbox` render 530 scalars with a
  234-scalar portion (second return visit, review R1; M27 and M28 bind).
- **Catalogue effects.** `Effect::Control(Power)` classifies the catalogue
  options the tables had called inert, and DSH's `--patch` is
  `Effect::Route`. The inventory test fixes every table literally.
  Unmodelled catalogue names still refuse, named, with no alias invented.
- **Codex configuration.** `setting` gives each assignment, in all five
  forms, a capability table or the one bounded inert effort, or refuses
  it with a fixed cause.
- **Managed lists and final positions.** `managed_separator` and
  `managed_patterns` bound managed lists. `parse_final` places codex
  `exec`/`exec resume … SESSION -` and admits no positional for the
  others.

The baseline red was observed on the redaction test. M1–M21 each bound
and were restored. fmt, clippy and the protocol, runtime and CLI suites
passed, as did both bundle compiles (digests unchanged), strict openspec
and `git diff --check`. Existing consumers are not rewired: units 12 and
13 move them onto these primitives. Workspace exact coverage, macOS and
remote CI are pending.

## 11. Unit 11 — Validate both declared halves at load

- [x] 11.1 Unit 11 parses both declared ON/OFF argv at load, even unused. Verify full Codex/mapping/separator/missing-authority refusals and valid positives. Requirements: [Native capability controls are adapter-owned evidence-bearing data][NC1], [Known native powers require a valid delivered denial or refusal][NCR], [Every accepted native control reaches the final command][NCC]. Reopened/remaining: operator ruling 1–2. (previous 3.3)

- [x] 11.2 Unit 11 parses a declared restriction transport with the empty restriction in its slot and carries only the empty restriction through real resolution; a nonempty restriction reaches only CQ1's outcomes with an exact reason (narrowed by the addendum of 2026-09-25; the qualified-restriction half is deferred below). Verify transport parsing, identity and required/wants/unused outcomes. Requirements: [Restrictions are validated, carried and pinned without engine interpretation][RG4], [Every accepted native control reaches the final command][NCC], [Capability authorization participates in bundle identity][MP2]. Reopened/remaining: operator ruling 1–2. (previous 4.4)

Observed 2026-09-25 (evidence.md, "Unit 11 — oversized"), run
`0065-rebuild-unit-11-see-the-uni-d48cbfd2`. Both tasks stay open. The unit
was built within its three production files and its three baseline reds
were observed. It was then reverted, because it changes the outcomes that
seven tests outside its named test files assert:

- four tests in `bundle/agent_tests.rs`;
- one in `tests/capability_launch.rs`;
- two in `brokkr-cli/src/doctor/capability_tests.rs`.

One of these, the "clean native restriction" row, is a nonempty-restriction
positive that ruling 4 forbids. No standing-admission line applies, since
every one would change an assertion. The split asks for those three test
files to be added to unit 11's inventory. The formatted patch is at
`.forge/unit-11-oversized-2026-09-25.patch`, uncommitted.

A second implement visit in the same run re-confirmed this on `789b812f`,
after triage re-ruled `chore` with an unchanged inventory. With the patch
applied, all seven tests failed; on the unpatched head, all seven passed.
The result is oversized again (evidence.md, "Second implement visit").

Built 2026-09-25 (evidence.md, "Unit 11 — built, under the inventory
ruling of 2026-09-25"), run `0065-rebuild-unit-11-see-the-uni-1d1020cd`,
on `e9d3ce8d`. The operator's option (a) widened this unit's test
inventory to `bundle/agent_tests.rs`, `tests/capability_launch.rs` and
`doctor/capability_tests.rs`, for assertion updates only
(`operator-ruling-2026-09-23.md`, addendum "rebuild unit 11's test
inventory"). The saved patch (`54de029c…05ac`) was applied unchanged.

- **Production:** the unit's three files, as the first visit built them.
  `check_declared` runs at load and parses both halves, the selection
  maps and separators, and the transport with `{}` in its slot. The
  deferral is refused with its exact reason. Codex's managed argv is
  parsed, not appended unread.
- **Baseline reds:** the three new tests were re-observed red.
- **Changed assertions** in the widened files:
  - five re-planted as well-formed, so their later refusal is still
    reached: the claude `elsewhere` row, the dsh row, the capability
    launch's codex selection, and both doctor tests;
  - two moved: the four native-OFF rows to the load refusal, and the
    restriction rows to the deferral refusal.

  In the unit's own `agents/tests.rs`, one fixture was re-planted and one
  positive became the deferral refusal.
- **Proof:** each change was bound by its red, M1–M14 each failed and
  was restored, and the restored suites passed.
- **Standing-admission lines:** none were used.
- **Gates:** fmt, clippy, the protocol, runtime and CLI suites
  (`witness_digests` 4/4), both bundle compiles, strict openspec and
  `git diff --check` all passed.
- **Pending:** exact coverage, macOS and remote CI.
- **Follow-up:** `doctor.rs:960-966` belongs to unit 22.

Returned by review 2026-09-26 (evidence.md, "Unit 11 — the review's
return"), same run, on `a03bdd1e`. F1 was medium and security: a declared
argv was checked for placement and a classified effect, never for its
value. A comma-only `--disallowedTools` loaded, and `denial_on` read
`Delivered` while nothing was denied.

- **Production:** a new `declared_values` in protocol `native_controls.rs`,
  called from `declared_argv` in `capabilities.rs`, the one path both
  halves and the substituted transport share. Every managed list value
  must be managed patterns. A permission control's value must be in its
  recorded bounded set; the engine records one only for Codex's
  `--sandbox`, so any other valued permission control refuses.
- **Tests:** eight rows were added to `every_declared_half…`: six exact
  refusals and two positives. Two `bundle/agent_tests.rs` fixtures that
  planted `-a never` were re-planted with value-free permission switches:
  `--dangerously-bypass-approvals-and-sandbox` in the competing-contribution
  row, and `--full-auto` in the admission test. Both still reach their
  D5.3 refusal. Reds, M15–M21 and restored passes are recorded.
- **F2:** the three touched fixture helpers now create their root under
  the canonicalised temporary base. Reverting them under a non-canonical
  `TMPDIR` failed nothing on Linux, so this has no Linux removal control;
  macOS is pending.
- **Standing-admission lines:** none were used.
- **Gates:** fmt, clippy, the protocol, runtime and CLI suites
  (`witness_digests` 4/4), both bundle compiles, strict openspec and
  `git diff --check` all passed. Exact coverage, macOS and remote CI are
  pending.

Returned by the second review 2026-09-26 (evidence.md, "Unit 11 — the
second review's return: oversized"), same run, on `0e7e777e`. F1 was
medium and security: `declared_values` let every `--config` assignment
through unread.

- **The fix is built and saved, not committed:**
  `.forge/unit-11-f1-oversized-2026-09-26.patch` (`12692853…46ed`).
  - **Production:** a new `Effect::Config` arm reads each declared value
    with `grammar::launch_setting`.
  - **Tests:** `every_declared_half…` gains five config refusals, two
    positives and a selection-entry coverage row. Its `codex sound`
    becomes a refusal.
  - **Widened inventory:** in `bundle/agent_tests.rs`, nine native-config
    rows moved to the load refusal, and the ON/transport fixtures were
    re-planted.
  - **Proof:** reds, and M22–M25 each failed and was restored. The gates
    passed.
  - **Standing-admission lines:** none.
- **Oversized:** `bundle.rs` DA:3376 and 3403–3408, the `Contribution::Native`
  config arms, are measured unhit. The exact gate needs them deleted, and
  `bundle.rs` is outside this unit. The split needed is to admit
  `bundle.rs` into unit 11 for that deletion, or to assign the deletion
  to a later unit.
- **11.1 and 11.2 are unticked.** They stay open until the F1 fix lands.
  A declared configuration value and a substituted configuration
  transport are not yet read on the branch.

Re-commissioned 2026-09-26, same run, without a ruling on `bundle.rs`
(evidence.md, "Unit 11 — re-commissioned without a ruling"):

- The saved patch still hashes `12692853…46ed` and still applies cleanly.
- Re-measured with the patch applied: all runtime tests pass, and
  `bundle.rs` DA:3376 and 3403–3408 are still 0.
- The tree was restored, and the unit is **oversized** again with the
  same split.
- **Standing-admission lines:** none.

Closed 2026-09-26 on the third visit, run
`0065-rebuild-unit-11-see-the-uni-a2e08218` (evidence.md, "Unit 11 — the
F1 fix lands; the two dead arms go to unit 12"):

- **The F1 fix is committed.** The saved patch still hashed
  `12692853…46ed`, applied cleanly on `a9b46001`, and was committed
  unchanged.
- **Re-verified:** the baseline reds, M22–M25 (each failed, then the tree
  diffed identical to the patch), the runtime, protocol and CLI suites,
  clippy, fmt, both bundle compiles, strict openspec and `git diff
  --check`.
- **Handed to unit 12:** deleting the two `Contribution::Native` arms of
  `expressed_sandbox` in `crates/brokkr-runtime/src/bundle.rs`, DA:3376
  and DA:3403–3408. They are dead because every declared `--config`
  assignment now passes the bounded reader at load. Such an assignment is
  either off the sandbox tables or an established key, so neither arm can
  run. `bundle.rs` is not edited here.
- **Exact coverage pending: deletion owned by unit 12.** Remote CI and
  macOS are also pending.
- **Standing-admission lines:** none.
- **11.1 and 11.2 are ticked.**

## 12. Unit 12 — Enable authored refusal and engine-only composition

- [x] 12.1 Unit 12 refuses every authored catalogue option regardless of grant/value/polarity/form. Verify complete all-harness matrix and bounded reasons without payloads; typed hands/route positives. Requirements: [Authored provider configuration cannot supply capability authority][RGR], [Known provider commands have a closed argument grammar][RGP], [Subtractive tool lists never grant a capability][RGS], [Reserved hands preserves the existing workspace authority][TD6], [Neither inline arguments nor fallback can override native denial][NC4]. Reopened/remaining: operator ruling 1–2. (previous 4.6)

- [x] 12.2 Unit 12 deletes authored folding and composes only engine controls. Verify exact empty/nonempty restrictions, OFF/hands or full managed conflict refusal. Requirements: [Every accepted native control reaches the final command][NCC], [Explicit restrictive tool lists retain their meaning][NCT], [Authored provider configuration cannot supply capability authority][RGR]. Reopened/remaining: operator ruling 1–2. (previous 4.3)

Handed from unit 11 (2026-09-26): unit 12 also deletes exactly the two
`Contribution::Native` arms of `expressed_sandbox` in `bundle.rs`, the
`--config` door text at DA:3376 and the door/writer/keys arm at
DA:3403–3408, and keeps the `Written` text. Unit 11's load reader made
both arms unreachable, and exact coverage stays pending until they are
deleted (evidence.md, "Unit 11 — the F1 fix lands; the two dead arms go to
unit 12").

Unit 12, first visit (2026-09-26, run
`0065-rebuild-unit-12-see-the-uni-245a74ff`): **oversized, nothing
landed.** The built core, dead-arm deletion included, is saved as
`.forge/unit-12-oversized-2026-09-26.patch`. With it applied, two test
files outside the inventory fail because their fixtures author a refused
option:

- `bundle/agent_tests.rs`, `a_resolved_seat_equals_the_equivalent_inline_seat`
  (`--allowedTools`);
- `brokkr-cli/tests/driver_conformance.rs`, `proof_codex_argv` and
  `proof_codex_driver` (`--sandbox danger-full-access`).

The split needed is a test-inventory widening, or a prior migration unit.
The patch's three standing-admission lines (`written: &[]` in
`capabilities/tests.rs`, `engine/capability_tests.rs` and
`agents/tests.rs`) are recorded in evidence.md, "Unit 12 — oversized".
12.1 and 12.2 stay open.

Re-commissioned 2026-09-26, same run, without a ruling on the two test
files (evidence.md, "Unit 12 — re-commissioned without a ruling"):

- The saved patch still hashes `f12b3e52…09e6` and still applies cleanly.
- With it applied, both out-of-inventory tests still fail on the ruling 1
  refusal (`agent_tests.rs:308:42`, `--allowedTools`;
  `driver_conformance.rs:3063:6`, `--sandbox`).
- The tree was restored, and the unit is **oversized** again with the
  same split.
- **Standing-admission lines:** none.

Built 2026-09-26 on the second visit, run
`0065-rebuild-unit-12-see-the-uni-8bdee503`, under the operator's
fixture-migration ruling of 2026-09-26 (evidence.md, "Unit 12 — built,
under the fixture-migration ruling of 2026-09-26"):

- **Step 0:** the ruling is appended verbatim to
  `operator-ruling-2026-09-23.md`, committed alone (`912a04db`).
- **Production:** the saved patch (`f12b3e52…09e6`) applied clean; an
  agent candidate's `written` is `&[]` (an agent reference authors no
  argv), replacing the patch's unreachable per-segment filter. The two
  dead `Contribution::Native` arms of `expressed_sandbox` are deleted and
  the `Written` text kept; llvm-cov shows DA:3376 and 3403–3408 gone and
  no `bundle.rs` line newly unhit.
- **Fixture migrations** (reason for each: the recipe may no longer author
  the option): `capability_launch.rs` — the shared inline seat to
  `tools.sandbox: workspace-write`, the boxed seat, `CODEX_SEAT` and the
  panel site drop `--sandbox`, the inline Claude seat drops
  `--permission-mode`; `bundle/agent_tests.rs` — the equivalent inline
  seat to `tools.allow: ["cargo"]`; `driver_conformance.rs` — the proof
  argv drops `--sandbox danger-full-access` and the single seat declares
  `workspace-write`. The live proof's NoHandsMember rows are re-planted to
  the adapter's fail-closed `sandbox-unavailable` cold retry, because a
  panel member can express no class (the one migration whose test no
  longer proves a rejoin at that shape).
- **New tests:** the all-harness authored-refusal matrix
  (`native_controls/tests.rs`) and the grant-state compile test
  (`capability_launch.rs`). Baseline reds at `912a04db`'s production;
  M1–M12b each failed as recorded and restored to the saved diff.
- **Standing-admission lines:** `capabilities/tests.rs:121`,
  `engine/capability_tests.rs:34`, `agents/tests.rs:4753` (`written:
  &[]`, forced by the new `Serving` field).
- **Gates:** runtime, protocol and CLI suites, clippy, fmt, both bundle
  compiles, strict openspec and `git diff --check` pass. **Pending:**
  exact coverage, macOS and remote CI.
- **12.1 and 12.2 are ticked.** 15.2 is advanced: compilation refuses by
  origin and composes only engine lists; the launch guard's value-reading
  over the legacy authored part stays for units 13–15.

Returned by review on 2026-09-26 (same run, second implement visit;
evidence.md, "Unit 12 — the review's return"):

- **12.2's tick at `b33de51f` was premature (F1).** The composer still
  unioned the selection into an explicit managed include list:
  `--tools Read` plus a granted fetch launched `--tools WebFetch,Read`.
  The fix makes an explicit include list, empty or not, a hard limit: an
  admission it does not name refuses the whole conflict, and so does a
  limit that would widen the boxed hands' own list. A compatible limit
  reaches its literal command. This is bound by protocol and compiled
  tests, baseline reds and M1–M4. 12.2 is ticked again on that evidence.
- **F3:** the authored refusal's prose is shortened. The bound is tested
  at argument 101 with room for a twenty-digit position; baseline red and
  M5.
- **F2 is blocked.** The re-planted NoHandsMember rows in
  `driver_conformance.rs` no longer prove a member rejoin, and D5.3
  refuses every typed class at a no-hands member. No admissible fixture
  restores the proof. The operator must rule: accept the fail-closed
  re-plant and move the member-rejoin positive later, or commission
  member lowering.
- **F4** stays a residual under 15.2.
- **Fixture migrations:** none new. **Standing-admission lines:** none.
- **Pending:** exact coverage, macOS and remote CI. **The unit is not
  complete until F2 is ruled.**

Closed 2026-09-26 on the third visit, run
`0065-rebuild-unit-12-see-the-uni-3a53001e` (evidence.md, "Unit 12 — F2
closed by deferral; e416d64b re-verified"):

- **F2 is closed by deferral.** The operator ruled option (a) on
  2026-09-26. The re-planted `NoHandsMember` rows are slice one's
  panel-member proof. The rejoin positive is under "Deferred to typed
  sandbox lowering at members" below, unticked, and in the owning delta.
  The ruling addendum is committed alone (`b663e689`).
- **Re-verified:** e416d64b's test locations, M1–M5 (M1 also against the
  compiled test), the workspace suite (protocol lib 490, runtime lib 565,
  `capability_launch` 45, CLI lib 481, `driver_conformance` 24), clippy,
  fmt, both bundle compiles, strict openspec and `git diff --check`.
- **Unit 11's pending coverage item is closed.** In `llvm-cov` with
  branches, DA:3376 and DA:3403–3408 are gone and no `bundle.rs` record is
  newly unhit. `native_controls.rs` has no zero line or branch.
- **No production or test byte moved.** Standing-admission lines: none.
  Fixture migrations: none new.
- **Pending:** exact coverage outside the box, macOS and remote CI.

Returned by the third review on 2026-09-26 (same run, fourth implement
visit; evidence.md, "Unit 12 — the third review's return"):

- **F1: the hard limit read only the selection.** A held fetch whose ON is
  an argv switch or a measured default launched past `--tools Read`. Now
  every tool of every held capability counts as an admission, unless a
  managed denial removes it, and a limit that does not name it refuses the
  whole conflict. Compatible limits reach their literal commands.
- This is bound by protocol and compiled rows, the review's two
  reproductions as baseline reds at `5fbd87d0`, and M6–M8. 12.2 stays
  ticked on that evidence.
- Standing-admission lines: none. Fixture migrations: none new.
- **Pending:** exact coverage outside the box, macOS and remote CI.
- **Amended in the same visit:** `a672f16a` let a managed denial excuse a
  held tool from the limit, which composed a granted fetch as denied. The
  exemption is deleted, and both denial rows now refuse. This is bound by
  M6, M9 and M10, with the compiled baseline red re-observed. See
  evidence.md, "Amended in the same visit: a denial does not excuse a
  holding".

Repaired 2026-09-26 as unit 12-fix, run
`0065-rebuild-unit-12-see-the-uni-d00aa84c` (evidence.md, "Unit 12-fix —
authority follows the selected holding; every limit holds"). The council
held SECURITY-HOLD on R1, R2 and R3.

- **One function:** `final_tools` in `native_controls.rs`, reached by
  compile admission and by launch through `compose_for_provider`.
  - Authority is `Controls::admits`, which `capabilities.rs` seals from the
    bound adapter entry narrowed by the grant. It is never read from the
    entries that share a capability's name.
  - Every include list of the template and the plan argv is a hard limit,
    and the lists hold as their intersection.
  - The hands' own list is the base an admission fills.
  - A WANTED holding a limit excludes drops with OFF (CQ1); a REQUIRED one
    refuses the whole conflict.
- **Proof:** R1, R2, R3 and the hands case are bound in both owning
  suites. Each has baseline reds observed on `bd6d7a31` and mutations
  M1–M8 with restored passes.
- **Recipes and bundles:** all 16 recipes and both bundles compile, with
  output byte-identical to `bd6d7a31`.
- **Standing-admission lines:**
  - `agents/tests.rs` 4824, 4998, 5095 and 5226 (the `segment` argument);
  - `adapters/tests.rs:14931` (the `admits` key in `claude_plan`).
- **Fixture migrations:** two `adapters/tests.rs` rows that authored
  `--tools` (the joined-spelling row, and the unboxed seat's last
  assertion, which now expects the R3 refusal at launch).
- 12.2 stays ticked, and 15.2 is advanced.
- **Pending:** exact coverage outside the box, macOS and remote CI.

Unit 12-fix-b (2026-09-26, run `0065-rebuild-unit-12-see-the-uni-ab0a1e57`),
the second repair after the council's SECURITY-HOLD on R1–R6: **oversized,
nothing landed** (evidence.md, "Unit 12-fix-b — oversized"). The one
function was completed against I1–I3, and the in-scope patch passes its
own suites and gates. It changes two assertions in
`crates/brokkr-protocol/src/adapters/tests.rs`, outside the unit's files
and outside both standing admissions:

- `an_explicitly_restrictive_managed_tool_list_reaches_the_final_command`,
  rows "split" and "equals": the plan's `--tools Read` limit with nothing
  held must now launch `--tools ""` (I1).

The compile-side half of R2 (`bundle.rs` composing the managed
`hands.harness.*` fragment at compile) also breaks two tests in
`crates/brokkr-runtime/src/bundle/model_policy_tests.rs`. Their planted
Claude gate fragment carries `--door`, which the Claude grammar cannot
place. The split needed is one of:

- (A) admit both files: the two `adapters/tests.rs` rows, and a re-plant of
  the fictional `--door` token at `model_policy_tests.rs:3934` and `:4046`;
- (B) admit `adapters/tests.rs` only, and drop the `bundle.rs` threading.
  R2 then refuses at launch, and a wanted holding excluded by a managed
  fragment is refused rather than dropped, deferred as a follow-up.

The patches are saved under `.forge/unit-12-fix-b/`. Standing-admission
lines: none landed. 12.2 stays ticked on the prior evidence; R1–R5 remain
open.

Second return in the same run: triage re-framed it as chore, but no
operator ruling on (A) or (B) exists. The same two `adapters/tests.rs`
failures were re-observed with the saved in-scope patch (evidence.md,
"Second return, same run"). The unit is still oversized, and it waits on
that ruling.

Second visit (2026-09-26, run `0065-rebuild-unit-12-see-the-uni-578edee1`),
under the operator's option-A ruling: **oversized again, nothing landed**
(evidence.md, "Unit 12-fix-b, second visit — oversized on one line"). The
ruling admits the gate fragment at `model_policy_tests.rs:3934` and `:4046`.
Once that is re-planted, the WORK fragment on the next line, `:3935`
(`["--permission-mode", "acceptEdits"]`), is refused at compile. It repeats
the Claude driver's own `--permission-mode acceptEdits`. The same fragment
would be refused at launch. With `:3935` also re-planted, assertions
unchanged, the whole workspace suite passes. The split needed:

- admit `model_policy_tests.rs:3935` under the same re-plant terms.

The patches are saved under `.forge/unit-12-fix-b/` (`visit2-*.patch`).
Standing-admission lines: none landed. R1–R5 remain open.

Third visit (2026-09-26, same run): **landed** (evidence.md, "Unit 12-fix-b,
third visit — landed").

- `final_tools` returns both final lists from every allowance source
  (plan, template, local and hands). Nothing is merged afterwards.
  - The include list is filled from the holdings alone.
  - Every restrictive list is a limit, the managed `hands.harness.*`
    fragment included.
  - `bundle.rs` composes that fragment at compile.
- **Proof:**
  - The 480-combination property test asserts I1–I3. Mutations M-I1, M-I2
    and M-I3 are each caught.
  - The chief reproductions are red on `1ecd3304`.
  - One new row, `a_held_carried_allowance_…`, covers two branches that
    llvm-cov found unhit. Mutations M-C1 and M-C2 are caught.
- **Admitted test changes:**
  - Option A: the `adapters/tests.rs` rows "every list joined, in aliases,
    nothing held" and "split"/"equals" (R5), each red on `1ecd3304`;
    `model_policy_tests.rs:3934` and `:4046`, the gate `--door` re-planted
    as `--max-turns 40`.
  - Standing fixture migration of 2026-09-26: `model_policy_tests.rs:3935`,
    the work fragment `--permission-mode acceptEdits` (refused as a
    repeat of the driver's permission mode) re-planted as `--max-turns 80`.
    This reading is an assumption, and it is flagged for review.
  - Every assertion is unchanged, and every baseline red is recorded.
- Unit 11's coverage handoff is closed: no unhit `bundle.rs` record lies in
  `expressed_sandbox`.
- 12.1 and 12.2 stay ticked. 15.2 is advanced, and it stays open.
- Standing-admission lines: none.
- **Pending:** exact coverage outside the box, macOS, remote CI and the
  council.

Unit 12-fix-c (2026-09-26, run `0065-rebuild-unit-12-see-the-uni-c0c83ac6`):
**oversized on one test row; nothing production landed** (evidence.md,
"Unit 12-fix-c — oversized on one test row").

- Built and saved under `.forge/unit-12-fix-c/` (`full.patch`
  `57582b50…`). Provenance is a typed record carried in the plan: the
  typed hands' argument count and the typed local permissions. It is never
  read from argv text. A managed fragment is always a limit. Every emitted
  allowance is in H ∪ W ∪ T and inside every limit.
- **Proof on the full patch:**
  - A 2880-combination provenance property test; the positions'
    reproductions compiled, sealed and verified.
  - Baseline reds on `add73ee2`; mutations M1–M7 caught.
  - Gates clean; the 18 compiles are identical.
  - No new unhit coverage record. Unit 11's handoff stays closed.
- **Stops on** `adapters/tests.rs:15525`. That row asserts a local
  permission emitted outside `--tools=Read`, which is S2's shape. The
  proposed re-plant is `--tools=Read,Bash`, with the output unchanged. Two
  other fixture lines there change no assertion.
- Forced literal lines (standing admission), one `provenance` field each:
  - `agents/tests.rs` after `:4752`;
  - `capabilities/tests.rs` after `:120`;
  - `engine/capability_tests.rs` after `:33`.
- Assumption for review: NCT's "empty built-in list" is read as the hands'
  own base, so W is bounded by every plan list.
- 12.1 and 12.2 stay ticked on the prior evidence. 15.2 stays open.
- **Pending:** the ruling, exact coverage outside the box, macOS, remote
  CI and the council.

Second visit (2026-09-27, run `0065-rebuild-unit-12-see-the-uni-167a4539`),
under the operator's admission riding the standing 2026-09-26 option-A
ruling: **landed** (evidence.md, "Unit 12-fix-c, second visit — landed").

- `full.patch` applied as saved, except the `adapters/tests.rs` row
  "every list joined, in aliases, nothing held": its input keeps
  `--tools=Read` and its expectation is now the exact refusal (the typed
  local `Bash(git:*)` outside the template's limit, S2's second shape).
  Red on `add73ee2`, which launched `--tools=` beside it.
- Fixture lines under the standing admission, no assertion changed:
  `adapters/tests.rs:14933` (`claude_plan`'s `"local"`) and `:15282`,
  `:15287` (`claude_admits_…` sets `plan["hands"]`); one `provenance` line
  each in `agents/tests.rs:4753`, `capabilities/tests.rs:121` and
  `engine/capability_tests.rs:34`.
- Mutations re-taken this visit, each caught and restored: hands inferred
  from the fragment's text, an untyped carried allowance admitted as
  local, and a typed local permission exempt from limits.
- 12.1 and 12.2 stay ticked; 15.2 is advanced and stays open.
- Assumption carried from the first visit: W is bounded by every plan
  list (NCT's "empty built-in list" read as the hands' own base).
- **Pending:** exact coverage outside the box, macOS, remote CI and the
  council.

The review's return (2026-09-27, same run, reviewed head `aee4faab`), one
MEDIUM: an untyped carried allowance's refusal spelled its whole
permission pattern, past D6's redaction and 512-scalar bound. **Repaired**
(evidence.md, "Unit 12-fix-c, the review's return").

- `native_controls.rs` only: the refusal names the tool alone when it is a
  plain name, and a fixed label otherwise. It never names the specifier.
- New protocol test `a_carried_refusal_names_its_tool_and_never_its_permission_payload`
  (compose and driver). `capability_launch`'s template row now names
  `Bash`, and the longest grammar-valid pattern is refused at compile.
- Red on `aee4faab`. Two mutations caught and restored.
- Standing-admission lines: none. 12.1 and 12.2 stay ticked. 15.2 stays
  open.
- **Pending:** exact coverage outside the box, macOS, remote CI and the
  council.

The second return (2026-09-27, same run, reviewed head `4183eb17`), one
MEDIUM, R1: a limit's refusal joined its raw patterns, with no redaction
and no bound. **Repaired** (evidence.md, "Unit 12-fix-c, the second
return").

- `native_controls.rs` only: a limit is named by bounded identities (a
  specified pattern as `Name(…)`, an unplain one by a fixed label), listed
  within 48 scalar values and the rest counted. `Outside` names its tool
  as a carried allowance is named.
- New protocol test `a_limit_refusal_names_bounded_identities_and_never_a_payload`
  (compose and driver), and two compiled rows in `capability_launch`.
- Red on `4183eb17`. Five mutations caught and restored.
- Standing-admission lines: none. 12.1 and 12.2 stay ticked. 15.2 stays
  open.
- **Pending:** exact coverage outside the box, macOS, remote CI and the
  council.

Unit 12-fix-d (2026-09-27, run `0065-rebuild-unit-12-see-the-uni-50c8438c`,
based on `fd1dd905`) answers the chief's SC-1 (`Unheld` spelled the plan's
pattern) and SC-2 (`Excluded` had no total bound). Both are **repaired**
(evidence.md, "Unit 12-fix-d").

- `native_controls.rs` only. Every `Conflict` variant is rendered by one
  function, `refused`:
  - a tool by its tool name alone, never its specifier;
  - a capability by its name;
  - an unplain one by a fixed label.

  Each name is cut to 128 scalars. Then the names, last first, are cut to
  keep the cause within 478 scalars, so the driver's refusal is at most
  512.
- Two new tests:
  - `every_composition_conflict_is_refused_in_bounded_identities`, in
    protocol, with compose and driver;
  - `a_compiled_conflict_is_refused_in_bounded_identities`, in
    `capability_launch`, compiled.

  Their rows are SC-1's sentinel and longest pattern, SC-2's 200/128
  pair, and the carried sibling.
- Red on `fd1dd905`. Six mutations were caught and restored.
- Second visit, based on `2632093f`: `Exclusion.clause` (the
  dropped-holding note) now renders its tool through `refused` too. It was
  red on `2632093f` with the sentinel path spelled, and mutation M7 was
  caught and restored.
- Standing-admission lines: none. 12.1 and 12.2 stay ticked. 15.2 is
  advanced and stays open.
- Assumption as first written, **withdrawn** by the third visit: the 512
  bound covers the complete driver refusal and the compile refusal's cause
  portion, not the compiler's site label. The closure claim above ("at most
  512" for the driver only) was incomplete in the same way.
- Third visit (the second review's return, based on `eba3f1b0`): the
  compiler's whole line (`bundle: `, the site and the cause) is at most 512.
  `Refusal::at_compile` cuts the site to what the cause leaves it, never
  below 64 scalars, and the composition cause bound is 438 (512 − 8 − 64 −
  2). A tool both admitted and denied is named through `refused` by its tool
  name alone. There are two new tests, one in protocol and one compiled, on
  Claude and LaneTally. They were red on `eba3f1b0`, and mutations M1–M4
  were caught and restored. Three existing expectations moved with the new
  bound; each move is recorded in evidence.md. Standing-admission lines:
  none. 12.1 and 12.2 stay ticked; 15.2 stays open.
- Fourth visit (audit, based on `237a6e29`): no production or test byte
  moved. Every identity-bearing `Conflict` arm renders through `refused`.
  The one variant not yet mutated, `Outside`, was mutated to spell its tool
  (M8). It was caught by 12-fix-c's limit test at `tests.rs:3106`, then
  restored. The locked all-features workspace run exited 0 with 77 `ok`
  summaries. fmt, clippy, `bundles/self`, strict OpenSpec validation and
  the diff check are clean. Standing-admission lines: none.
- **Pending:** exact coverage outside the box, macOS, remote CI and the
  council.

Unit 12-fix-e (2026-09-27, run `0065-rebuild-unit-12-see-the-uni-72e4162a`,
based on `1b2df47b`), the diagnostic sink split from 12-fix-d, answering the
chief's C1, S1 and SC-D2. Result: **oversized**. The fix is built and proved,
and saved as a patch rather than committed (evidence.md, "Unit 12-fix-e").

- **Why it stops.** The one sink bounds the whole driver line to 512. One
  existing assertion outside this unit's files asserts a longer line:
  `crates/brokkr-protocol/src/adapters/tests.rs:15027`, an authored grammar
  refusal of 520 scalar values. That is a D6 violation the sink now cuts, so
  its expectation must move. A changed expectation is not a forced line, so
  the standing admission does not cover it.
- **The ruling asked for.** Admit that one expectation for this unit, as an
  assertion update only. It becomes its first 511 scalar values and `…`,
  ending `(decision 0066 …`. The proposed change is saved beside the patch.
- **Saved:** `.forge/unit-12-fix-e/full.patch` (sha256 `ee3e77e5…`) and
  `adapters-tests.proposed.patch` (sha256 `0f18e8bd…`). Both apply clean on
  this unit's docs commit.
- Production files: `native_controls.rs` and `bundle.rs`. `engine.rs` is not
  needed: the launch refusal renders in `native_controls.rs`.
- Proved on the patched tree: baseline reds on `1b2df47b` for C1, S1 and
  SC-D2; mutations M1–M8 each compiled (M7 fails to compile, as intended)
  and was caught, then restored.
- Standing-admission lines: none. 12.1 and 12.2 stay ticked. 15.2 stays
  open.
- **Pending:** the operator's ruling on the one expectation; then the
  workspace run, exact coverage outside the box, macOS, remote CI and the
  council.
- **Second return (same run, 2026-09-27).** Triage re-ruled chore, and no
  operator addendum admits the expectation. Both saved patches still hash
  and apply clean on `3ab20db9`. With `full.patch` applied,
  `adapters/tests.rs:15027` fails again, then the patch was reverted.
  Result: **oversized** again, waiting on the same ruling.
- **Third visit (run `0065-rebuild-unit-12-see-the-uni-2986103d`,
  2026-09-27).** The operator admitted the one expectation at
  `adapters/tests.rs:15027` as an assertion update only (ruling addendum
  "2026-09-27"). Both saved patches were applied and committed. The admitted
  line's baseline red was observed on pre-fix production, and M5 (sink
  bypassed) fails it again. The baseline reds for C1, S1 and SC-D2 and
  mutations M1–M8 were re-taken and each reproduced. fmt, clippy, the
  protocol and runtime suites, the workspace run (77 ok, 0 failed),
  `bundles/self` and strict OpenSpec (18/18) passed (evidence.md, "Third
  visit"). Standing-admission lines: none. Result: **complete**. 12.1 and
  12.2 stay ticked; 15.2 stays open.
- **Pending:** exact coverage outside the box, macOS, remote CI and the
  council.
- **Fourth visit (same run, returned from review at `0ff4caa7`).** Result:
  **oversized**, and nothing but this record moved. C-E1 (MEDIUM): the
  compiler site is formatted raw in `capabilities.rs:1601-1606`, and
  `at_compile` only cuts it. C-E1 needs `capabilities.rs`. A-E1 (LOW):
  resume re-wraps the raw capability reason, and fixing that needs
  `brokkr-cli/src/lib.rs`. The ruling asked for is to admit both files, or
  to split A-E1 into 12-fix-f (evidence.md, "Fourth visit").
- **Fifth visit (same run, re-fired by triage without a ruling).** Result:
  **oversized**, and nothing but this record moved. Both patches are already
  applied (their hashes match, and the reverse check passes). C-E1 and A-E1
  still stand at the same sites. No addendum admits `capabilities.rs` or
  `lib.rs` (evidence.md, "Fifth visit").

Unit 12-fix-f (2026-09-27, run `0065-rebuild-unit-12-see-the-uni-d31110e4`,
based on `2088a97a`) is split from 12-fix-e under the preamble and recorded
under unit 12 in design.md's Rebuild units. It answers C-E1 and A-E1.
Result: **complete** (evidence.md, "Unit 12-fix-f").

- **C-E1.** The compiler's site is now a typed `Site` (seat, office,
  realm) in `native_controls.rs`. Each part is quoted whole where it is a
  plain label of at most 64 bytes, and is otherwise rendered by its plain
  lead and its length, never echoed. `at_compile` takes the `Site`.
  `capabilities.rs` builds it in `who()`, and it opens all ten capability
  refusals and notices.
- **A-E1.** `unreproducible` in `brokkr-cli/src/lib.rs` makes the whole
  resume line through `bounded_line`: at most 512 scalar values and
  control-free.
- **Tests.** New: an exact `Site` test, the invariant driven with
  adversarial seats, offices and realms, a compiled regression for the
  reviewer's seat, and a CLI resume regression. One expectation moved in
  `capability_launch.rs`: the 300-scalar realm is now named by its bounded
  identity.
- **Proof.** Baseline reds on `2088a97a` for the compiled and resume
  regressions. M1 (site echoed raw) and M2 (sink skipped) each compiled and
  were caught, then restored. fmt, clippy, the protocol, runtime and CLI
  suites, the workspace run (77 ok, 0 failed), `bundles/self`,
  `bundles/verify`, strict OpenSpec (18/18) and the diff check all passed.
- Standing-admission lines: none. 12.1 and 12.2 stay ticked; 15.2 stays
  open.
- **Pending:** exact coverage outside the box, macOS, remote CI and the
  council.

## 13. Unit 13 — Build final assessment and share structural consumers

- [x] 13.1 Unit 13 supplies the pure complete builder/checker and private checked command, with independent expected state and shared structural consumers. Verify managed contradictions, empty/absent distinction, prefix/selector grammar and no unchecked post-validation mutation. Cold/resume integration remains 14.1/15.1. Requirements: [Every accepted native control reaches the final command][NCC], [Known provider commands have a closed argument grammar][RGP], [Explicit restrictive tool lists retain their meaning][NCT]. New explicit prerequisite: operator ruling 2; both design positions. (previous 7.7)

- [x] 13.2 Unit 13 replaces managed raw consumers with shared parsing. Verify inert --image resume, prompt/duplicate meaning and unchanged resume eligibility. Requirements: [Known provider commands have a closed argument grammar][RGP], [Prompt values cannot absorb a composed control][NCP], [Eligible Codex resumes reimpose the capability control][NC3]. Reopened/remaining: operator ruling 1–2. (previous 3.13)

Unit 13 (2026-09-27, run `0065-rebuild-unit-13-see-the-uni-9db14032`,
based on `67571c3b`; evidence.md, "Unit 13"):

- **13.1.** `native_controls.rs` gains the pure `check_final`. It parses
  the whole final command at its fixed positions and compares the
  command's `State` with the composed plan's. The fields compared are the
  include list (absent and empty kept apart), the allow and deny lists,
  the Codex class (`--sandbox` and `sandbox_mode` are one effect), the
  other capability-bearing options and the rejoin. It then checks the
  sealed `Expected` from typed inputs. Success is a private `Checked`,
  which can only be read or taken whole.
- **13.2.** `adapters.rs`'s Codex and Claude selector, effort, sandbox,
  resume-blocker, restriction and persistence readers read the grammar's
  one parse where the argv parses. `--image resume` is a value through
  selection, eligibility and the final parse. Eligibility and every
  cause are otherwise unchanged, and unparsable by-hand argv keeps its
  conservative reading.
- **Tests.** There are two new adapters tests and five new native_controls
  tests. One adapters assertion moved with the intended change: `-m
  resume` is now admitted. The constructor invariant now drives
  `Why::Final`.
- **Proof.** Baseline red at `adapters/tests.rs:1955` on `67571c3b`. M1–M7
  and M9–M11 each compiled, were caught and were restored.
- **Gates.** fmt and `git diff --check` are clean. clippy with `-D
  warnings` is clean. `brokkr-protocol` lib: 509 passed. Strict OpenSpec:
  18 of 18. `cargo test --workspace --all-features --locked
  --no-fail-fast`: exit 0, 77 `ok` summaries. `bundles/self` and
  `bundles/verify` both compile.
- Fixture migrations and standing-admission lines: none.
- **Follow-ups.** The composer's `node_patterns` readers and `bundle.rs`'s
  `config_key` are unchanged (evidence.md).
- **Pending.** Integration (14.1, 15.1, 15.2), exact coverage outside the
  box, macOS, remote CI and the council.

Unit 13 review return (2026-09-27, same run, reviewed `67571c3b..202adc7f`;
evidence.md, "Review return"):

- **F1–F3.** `check_final` now takes the sealed `LaunchRecord` and derives
  the whole authority from it (`authority`): every capability-bearing
  option is accounted for once by the template, the typed hands, the
  boundary or a measured Codex OFF for a power the plan answers for;
  Claude's include, allow and deny lists are exact against the holdings,
  the hands tool and the lowered limits; `AllowIntent` and `Application`
  must agree; required hands must be the engine's workspace hands, carried
  whole, allowed and undenied; Codex runs the hands', site's or boundary's
  class. Baseline reds B1–B8 on `202adc7f`.
- **F4.** `parse_final("dsh")` reads the real serving command at fixed
  positions, and `check_final` binds its one staged overlay (B9).
- **F5.** `split_dsh_model` and `split_dsh_patch` read the grammar's nodes
  where the argv parses; the DSH grammar admits only their separate
  spellings (B10).
- **F6.** M17 binds the DSH inventory test at `:8103`. **F7.** The shim
  fixture uses one canonicalised root; its removal control does not
  discriminate on this host (evidence.md). **F8** asks nothing of the
  code.
- **Proof.** M1–M23 each compiled, were caught and restored. Four existing
  grammar assertions moved with F5's intended change; the unit's own
  record-based fixtures replaced the `Expected`-only ones.
- **Gates.** fmt, `git diff --check`, clippy `-D warnings`: clean.
  Protocol: 513 + 99 + 1 passed. Workspace `--all-features --locked`: 77
  `ok` summaries. Both bundles compile. Strict OpenSpec 18/18. The
  protocol coverage diagnostic leaves no unhit record in the changed code.
- Fixture migrations and standing-admission lines: none.
- **Pending.** Integration (14.1, 15.1, 15.2), exact coverage outside the
  box, macOS, remote CI and the council.

Unit 13 second review return (2026-09-27, same run, reviewed
`67571c3b..86374055`; evidence.md, "Second review return"):

- **F1.** Every Codex effect must be owed by a sealed source; the old
  measured-OFF free pass is gone. Each effect is then judged by its
  meaning (`codex_meaning`), whoever supplied it. Refused: an OFF for a
  held or unanswered power, a web switch or assignment for an unheld
  power, and the bypass, `--full-auto` or a sandbox table beside the
  class.
- **F2.** The template's, boundary's and native include limits, empty or
  not, must each name every held, hands and lowered local tool. A lowered
  local tool must be available, and the include list may name it
  (`final_tools` I1).
- **F3.** A holding's excluded guard tools stay unavailable. The plan's
  selection and native allow/deny names must reach the command.
- **F4.** `--fork-session` and `--bg` refuse as selectors, read from the
  adapter's `CLAUDE_SELECTORS_BARE`.
- **F5.** The native argv is parsed and read on its own, and its effects
  are owed.
- **F6.** The DSH joined-spelling rows assert the whole grammar problem.
  **F7** asks nothing of the code.
- **Proof.** Baseline reds on `86374055`: 7 tests (five `Ok`, two intended
  wording moves). M1–M28 each compiled, were caught and were restored.
  Four assertions moved with the intended change, and F2's named positive
  became its full refusal.
- **Gates.** fmt, `git diff --check` and clippy `-D warnings` are clean.
  Protocol: 517 + 99 + 1 passed. Workspace: 77 `ok` summaries. Both
  bundles compile. Strict OpenSpec: 18/18. The coverage diagnostic leaves
  no unhit record in the changed functions.
- Fixture migrations and standing-admission lines: none.
- **Follow-up.** Under a non-hands limit, `final_tools` leaves a lowered
  local tool out of the include list it writes, and the check refuses
  that command. Shipped adapters do not reach this shape.
- **Pending.** Integration (14.1, 15.1, 15.2), exact coverage outside the
  box, macOS, remote CI and the council.

Unit 13-fix (2026-09-27, run `0065-rebuild-unit-13-see-the-uni-7fa17ffb`,
based on `640b5b2e`; answers the chief's security hold F1–F6; evidence.md,
"Unit 13-fix"). **Result: oversized on two out-of-file expectations.**
The fix is built and proved but not committed; it is saved as
`.forge/unit-13-fix/full.patch`. F6's builder names the typed local
permission's tool in the include list it writes, so two literals in
`crates/brokkr-runtime/tests/capability_launch.rs` (`:6576` and `:6988`,
`--tools ""` behind the template's `Read,Bash` limit beside a lowered
`Bash(ls:*)`) must become `--tools Bash`. A changed expectation is outside
both standing admissions. The proposed change is
`.forge/unit-13-fix/capability-launch.proposed.patch`. 13.1 and 13.2 are
unticked here, because the council held unit 13's closure; they stay open
until the patch lands and is reviewed.

- **One state, derived and read back.** `State` gains `off` (measured OFF
  switches) and `prompt`. `check_final(harness, command, composed,
  controls, record, Serving { session, overlay, prompt })` compares the
  parsed command with the composition in order, then by meaning with the
  state `derive` builds from the sealed record and plan alone. The
  origin-consuming `authority` and `codex_meaning` are gone.
- **F1.** Every contribution's lists are owed: the boundary's denials
  (and allows and limits) join the derived state.
- **F2.** Every sandbox contribution (typed local class, local fragment,
  template, hands, boundary, native) names one class;
  `danger-full-access` anywhere, two classes, or a class at a harness
  with no mapping refuse as sealed.
- **F3.** A capability-table assignment is read as exactly a rejoin's
  class, exactly the measured OFF, a server entry, or refused as read.
- **F4.** Only the template's `acceptEdits` mode, the typed workspace
  hands, tool lists, one class and the plan's own OFF are established;
  `--settings`, other loads, servers outside the hands and other
  controls refuse as sealed. Unit 13's `--settings` positive is replaced.
- **F5.** The DSH final grammar models the terminal prompt as data and
  refuses a missing prompt and extra positionals.
- **F6.** `final_tools` names the carried lowered local tools in the
  include list it writes; the composer reads a bare denial as the check
  does ("both admitted and denied").
- **Proof (on the patched tree).** Baseline reds for F1–F6 on
  `640b5b2e`. New finding tests and two metamorphic properties: 1,187
  generated sealed states (707 compose and check; 480 Claude/LaneTally
  states refuse only as a list written twice) and 22,671 single
  mutations, all refused. M1–M20 each compiled, were caught and restored.
- **Assertions moved with the intended change** in unit 13's own tests
  and two composer tests reached by F6 (evidence.md lists each).
- Fixture migrations and standing-admission lines: none.
  `adapters.rs` did not change.
- **Ruling asked for:** admit the two `capability_launch.rs` expectations
  for this unit, as assertion updates only (evidence.md, "Why it stops").
- **Pending.** The ruling, then the patch; integration (14.1, 15.1,
  15.2), exact coverage outside the box, macOS, remote CI and the
  council.
- **Re-fire, 2026-09-27 (same run, after triage re-ruled `chore`).** No
  addendum since `be1f094d`, so the stop stands (evidence.md, "Unit
  13-fix re-fire"). Both saved patches still apply to `be1f094d` and still
  hold; 13.1 and 13.2 stay unticked.

Unit 13-fix second visit (2026-09-27, run
`0065-rebuild-unit-13-see-the-uni-e6795fae`, based on `90d49864`;
evidence.md, "Unit 13-fix second visit"). **Result: complete.** The
operator's 2026-09-27 ruling admits the two `capability_launch.rs`
expectations as assertion updates only. It is appended verbatim to
`operator-ruling-2026-09-23.md`. Both saved patches were applied unchanged,
and their hashes match the record. 13.1 and 13.2 are ticked here; they are
still subject to the council.

- **Admitted changes** (`crates/brokkr-runtime/tests/capability_launch.rs`).
  At `:6572-6578`, the comment is corrected and `""` becomes `"Bash"`; at
  `:6989`, `""` becomes `"Bash"`. The reason is F6: the typed `Bash(ls:*)`
  under the template's `Read,Bash` limit stays usable. The baseline red is
  0 passed / 2 failed with only `full.patch`. The restored pass is 53
  passed. Mutation N7 fails both again.
- **Mutation proofs.** N1–N8 cover F1 (boundary deny), F2 (native class
  and `danger-full-access`), F3 (unestablished assignment), F4 (unjudged
  effects), F5 (missing DSH prompt), F6 (lowered tools) and the skipped
  derived-state equality. Each compiled, was caught and was restored, and
  `cmp` confirmed the restoration byte for byte.
- **Gates.** fmt, `git diff --check`, workspace clippy with `-D warnings`,
  the workspace tests (exit 0, 77 ok summaries), both bundles and strict
  OpenSpec (18/18) are clean.
- Fixture migrations and standing-admission lines: none.
- **Pending.** Integration (14.1, 15.1, 15.2), exact coverage outside the
  box, macOS, remote CI and the council.

Unit 13-fix-b (2026-09-27, run `0065-rebuild-unit-13-see-the-uni-eb99d7ad`,
based on `e61185f9`; answers the chief's security hold R1–R5; evidence.md,
"Unit 13-fix-b"). **Result: complete.** 13.1 and 13.2 stay ticked. The
exact proofs R5 asks for now back them, and they are still subject to the
council.

- **Recompose, don't re-derive.** `derive` is gone. `check_final(harness,
  command, controls, expected, dialect, serving)` proves the sealed inputs
  are the plan's. It rebuilds the template, the lowered local permissions
  and class, the hands expanded from the typed `Transport`, and the
  dialect's boundary. It composes them through `compose_for_provider` and
  compares the final command's effects with the recomposition's in order.
  The record's argv is never read.
- **R1.** The hands' values come from `Transport::expand`. Their options
  must be exactly the transport's own.
- **R2 and R3.** The composer's refusals, including the known-power floor
  and both admitted and denied, are the check's, unchanged.
- **R4.** `State` keeps every effect in command order. A Codex rejoin's
  class leads, as `codex_plan` places it.
- **R5.** Property 1 covers 1,179 launches: 699 check, and 480 refuse with
  the composer's hand-written refusal. Property 2 covers 14,269 mutations,
  swaps of any two consecutive effects among them. Every one is asserted
  against its complete written `Refusal`.
- **Proof.** Baseline reds for R1–R4 on `e61185f9`. For R5, a wrong-cause
  mutation survived the old property. Mutations M1–M13 each compiled, were
  caught and were restored (`cmp`). M8 survived once, until a class-last
  rejoin case was added.
- **Gates.** fmt, `git diff --check` and workspace clippy `-D warnings`
  are clean. Protocol: 526 + 99 + 1 passed. Workspace: 77 `ok` summaries.
  Both bundles compile. Strict OpenSpec: 18/18.
- **Tests removed or replaced.** Four tests fed `check_final` a
  composition wrong in the same way as the command, which it no longer
  takes (listed in evidence.md).
- Fixture migrations and standing-admission lines: none. Only
  `native_controls.rs` and `native_controls/tests.rs` moved.
- **Follow-ups.** `engine::hands_command` should call `Transport::expand`
  (units 14–15). A list written twice across the template and the
  fragment is refused as `authored` by the composer; that behaviour
  predates this change.
- **Pending.** Integration (14.1, 15.1, 15.2), exact coverage outside the
  box, macOS, remote CI and the council.

Unit 13-fix-c (2026-09-27, run `0065-rebuild-unit-13-see-the-uni-8ad71c84`,
based on `bcd7272c`; answers the chief's security hold R1–R6; evidence.md,
"Unit 13-fix-c"). **Result: oversized on one out-of-inventory test file.**
The repair is built and proved in `native_controls.rs`, `adapters.rs` and
`native_controls/tests.rs`, and saved uncommitted as
`.forge/unit-13-fix-c/full.patch`. R1 refuses typed hands without their
whole transport, so three `brokkr-runtime` tests that plant an incomplete
Codex hands fragment (`bundle/agent_tests.rs:967`, a Claude one at
`:4063`) fail. Moving them changes one assertion (`:4497`, 12 → 16, and
its argv literal), which neither standing admission covers. The proposed
change is `.forge/unit-13-fix-c/agent-tests.proposed.patch`. 13.1 and 13.2
are unticked here, as at 13-fix, until the patches land and are reviewed.

- **R1.** The composer takes hands from the plan's typed count, never from
  an include node. It writes the include restriction and the hands
  allowance itself, and refuses typed hands without strict MCP or the MCP
  document, or without the Codex class or any of the server's three
  bindings. A denial of the hands tool or its server refuses.
- **R3.** Every held tool is in the composer's admitted set, so any denial
  that removes one refuses, even with empty selection lists.
- **R2.** `toml_basic` is the one encoder for the executable and every
  server argument, expected and emitted alike. A non-UTF-8 path refuses.
- **R4.** `check_final` rebuilds the complete serving argv with the
  drivers' own builders (`serving_command`, which calls `claude_serving`,
  `codex_cold`, `codex_rejoin` and `dsh_command`; the launch calls them
  too). It compares the argv token for token from the executable.
  `Serving` carries the engine's choices; `Dialect::sandbox` is the typed
  class's local fragment, and `{result_path}` is filled from the chosen
  path. The rebuild found that a boxed Codex rejoin is one the driver
  declines, and it now refuses it.
- **R5.** Property 1 covers 1,600 launches, each against its own model's
  complete command or `Refusal`, floors and exact permission conflicts
  included. Property 2 covers 30,167 mutations, none surviving. M1–M15 each
  fail a generated property and were restored (`cmp`). M9 survived once,
  until an agent site was served from another workdir.
- **R6.** Baseline reds on `bcd7272c` assert complete values and refusals.
  The note on 13-fix-b's `is_err()` baselines is in evidence.md, and the
  historical rows are unchanged.
- **Assertions moved with the intended change** in the unit's own tests,
  and `native_controls/tests.rs` fixture repairs (`claude_controls`, two
  authority rows and the folding test's hands), are listed in evidence.md.
  Standing-admission lines and fixture migrations on `full.patch`: none.
- **Gates (both patches applied).** fmt, `git diff --check` and clippy
  `-D warnings` are clean. Protocol: 531 + 99 + 1 passed. Workspace: exit
  0, 77 `ok` summaries. Both bundles compile. Strict OpenSpec: 18/18.
- **Ruling asked for:** admit `agent-tests.proposed.patch` for this unit,
  as fixture re-plants and one assertion update only.
- **Pending.** The ruling, then both patches; the `tomllib` cross-check
  (needs approval in the seat); integration (14.1, 15.1, 15.2), exact
  coverage outside the box, macOS, remote CI and the council.
- **Re-fire, 2026-09-27 (same run, after triage re-ruled `chore`).** There
  is no addendum after "2026-09-27: rebuild unit 13-fix's two expectations",
  so the stop stands (evidence.md, "Unit 13-fix-c re-fire"). Both saved
  patches still apply to `6d9544e2` and still hold. 13.1 and 13.2 stay
  unticked.
- **Second visit, 2026-09-27** (run `0065-rebuild-unit-13-see-the-uni-f0384cf8`,
  based on `144a2051`; evidence.md, "Unit 13-fix-c second visit").
  **Result: complete.** The operator admitted the runtime fixture changes,
  and the ruling is appended as the addendum "2026-09-27: rebuild unit
  13-fix-c's runtime fixtures". Both saved patches landed exactly as
  recorded (digests match, and `git apply -R --check` is clean). 13.1 and
  13.2 are ticked.
  - **Admitted changes** in `crates/brokkr-runtime/src/bundle/agent_tests.rs`:
    `CODEX_WORKSPACE` (`:967`) is now the shipped eight-token fragment;
    the Claude hands fixture (`:4067-4073`) gains `--strict-mcp-config`;
    and the assertion at `:4507-4527` is now 16 tokens, with the two `-c`
    pairs added to its literal. The reason: under R1, the fixtures must
    carry the shipped adapters' whole hands transport.
  - **Baseline reds.** With `full.patch` alone, the three tests fail on
    R1's complete "no server command binding" refusal. Without the
    `--strict-mcp-config` token, the class-free positive fails on the
    "no strict MCP configuration" refusal. The old length fails `16 ≠ 12`.
    Each edit was restored, and the file then passed 50/50.
  - **Mutations for the moved assertion** (temporary edits to `bundle.rs`,
    restored). Reordering the bindings fails the literal. Inserting an
    inert option fails the length. Dropping or repeating a pair is refused
    at compile before the assertion is reached.
  - **Re-taken mutations.** M1–M15 and M5b were re-taken on the landed
    code, and every one fails a generated property, restored by `cmp`.
  - **Gates.** fmt, `git diff --check` and clippy `-D warnings` are clean.
    Workspace: exit 0, with 77 `ok` summaries (protocol 531 + 1 doctest,
    runtime 565). Both bundles compile. Strict OpenSpec: 18/18.
  - **Pending.** The `tomllib` cross-check (needs seat approval);
    integration (14.1, 15.1, 15.2); exact coverage outside the box; macOS;
    remote CI; and the council.

Unit 13-fix-d (2026-09-27, run `0065-rebuild-unit-13-see-the-uni-a1326516`,
based on `2ab38565`). It answers the chief's security hold R1–R5; see
evidence.md, "Unit 13-fix-d". **Result: complete.** 13.1 and 13.2 stay
ticked. The only production change is in `native_controls.rs`;
`grammar.rs` and `adapters.rs` did not move.

- **R1.** `captured` makes the result capture a typed sink. It is read by
  its parsed canonical option in every contribution. It is admitted only in
  the local sandbox fragment or the boundary, as exactly `{result_path}`,
  and emitted from `Serving::output` at that one value position. A work
  seat has none, and a gate has exactly one. Any other capture or
  placeholder refuses.
- **R2.** `admit` is the one admission. The hands tool the composer
  synthesizes now passes every limit, as a carried one does.
- **R3.** The no-mapping path no longer returns early. Availability is
  judged before either return. **Assumption:** a selected deny list with no
  mapping now refuses as "a final tool list with no selection mapping". It
  was dropped before, and `decode` cannot produce it.
- **Generated cases.** Incompatible limits (72 per harness), absent
  mappings (128 per harness) and misbound Codex captures (36). Each has a
  complete refusal or command from its own model, and the tallies were
  counted by hand. P2 has no survivor.
- **Proofs.** Baseline reds on `2ab38565` for the three new tests and P1.
  MD1–MD5 are bound by their own test and P1. MD6 supplies R4's missing
  mutation for `the_recipes_words_…` at its exact assertion.
- **R5.** MA1–MA4 are recorded as out-of-scope and not counted. MD7, made
  in this unit's own file, fails the three admitted runtime fixture tests at
  their compile positives. No in-scope mutation reaches the resolver-made
  literal at `agent_tests.rs:4507-4527`, and that gap is recorded.
- **Moved assertion** in the unit's own tests:
  `every_sealed_sandbox_contribution_…`'s gate fixture now captures through
  `{result_path}` with a chosen path, with its red recorded. Standing-admission
  lines and fixture migrations: none.
- **Gates.** fmt, `git diff --check` and clippy `-D warnings` are clean.
  Workspace: 77 `ok` summaries. Both bundles compile. Strict OpenSpec:
  18/18.
- **Pending.** Integration (14.1, 15.1, 15.2), exact coverage outside the
  box, macOS, remote CI and the council.

## 14. Unit 14 — Integrate checked cold commands

- [x] 14.1 Unit 14 parses full serialized cold commands and compares exact plan state. Verify dropped OFF/terminator/separator/prefix-duplicate refusal and typed positives. Requirements: [Codex web-search OFF uses the controller's measured fragment][NC2], [Known native powers require a valid delivered denial or refusal][NCR], [Every accepted native control reaches the final command][NCC]. Reopened/remaining: operator ruling 1–2 / R10. (previous 7.1)

Unit 14 (2026-09-27, run `0065-rebuild-unit-14-see-the-uni-e639d515`,
based on `22358e51`; evidence.md, "Unit 14 — stopped before
implementation"). **Result: oversized.** Nothing was built, and 14.1 stays
open.

- `check_final` needs the adapter's declared dialect (the permission flag
  and the unexpanded local sandbox, hands and boundary fragments), the
  adapter's pins and the hands `Transport`. The driver input carries none
  of them (`engine.rs:1307-1394`). The record's segments carry them only
  expanded and merged, and D6 and D5.7 forbid recovering them by text.
- **Split asked for.** 14a: the engine seals those typed serving inputs
  and expands hands through `Transport::expand` (`engine.rs`,
  `agents.rs`/`bundle.rs`, and `native_controls.rs` if the record carries
  them; the operator names the three). 14b: this unit as written, in
  `adapters.rs`.
- Fixture migrations and standing-admission lines: none.
- **Pending.** The operator's split, then 14a and 14b, exact coverage
  outside the box, macOS, remote CI and the council.
- **Re-fire (same run, after triage ruled `chore` again): blocked.** No
  operator ruling answered the stop. Re-checked on `3fe9c512`: the driver
  input still carries none of `check_final`'s typed inputs (evidence.md,
  "Re-fire after triage"). Nothing moved; 14.1 stays open.

**Split (the operator's session, 2026-09-27).** Unit 14 runs as three
units, in order: 14a1, 14a2, then 14b (design.md, "Rebuild units", unit
14). 14.1 closes only at 14b.

Unit 14a1 (2026-09-27, run `0065-rebuild-unit-14-see-the-uni-5642ffd7`,
based on `5aac22f4`; evidence.md, "Unit 14a1"). **Result: complete.**
14.1 stays open. Production: `agents.rs` and `bundle.rs` only.

- **Agent path.** `Composition` carries `serving: Box<ServingInputs>`:
  the declared permission flag and separator, the `hands.workspace`
  fragment where boxed hands compose, the `hands.harness` gate and work
  fragments under `harness` beside hands, the model and effort pins, and
  the agent's `HandsSpec`. `compose` now takes the realm's `Boundary`.
- **Inline path.** `SiteFacts::inline_dialect` records the permission
  flag an allow lowered onto and the class fragment a Codex sandbox
  lowered onto, as declared. `SiteFacts::inline_serving()` adds no pins
  and the site's resolved `HandsSpec`.
- **Proofs.** Two new tests. MA1–MA6, MA4b and MA5b (agent path) and
  MI1–MI6 (inline path) each fail at a named row, restored by `cmp`.
- **Owning-suite updates** in `agents/tests.rs`: two whole-`Composition`
  expectations gain the carried value, and two `compose` calls name a
  `Boundary`. **Standing-admission lines:** four `serving:
  Default::default(),` lines in the engine test fixtures. Fixture
  migrations: none.
- **Gates.** fmt, `git diff --check`, clippy `-D warnings`, the runtime
  suite (25 `ok` summaries), both bundles and strict OpenSpec are clean.
- **Pending.** 14a2 and 14b, exact coverage outside the box, macOS,
  remote CI and the council.
- **Return (same run, from review at `f320bf39`): complete.** Chief F1:
  the inline class fragment was read from its emitted segment. Now
  `lower_inline_sandbox` returns the declared fragment beside the segment,
  and `record_inline_tools` records it (`bundle.rs` only). Chief F2: two
  compiled-bundle tests use adapters whose fragments and models carry
  `./` tokens that the compile expands. They assert that each carried
  value stays as declared while its emitted segment is expanded, on the
  agent path and on the inline path. The baseline red is MF1 on the old
  code, which fails the carried rows. MF1–MF8 and MF5b each fail at a
  named row, and the emission-only MF1 and MF8 leave the carried rows
  holding (evidence.md, "Unit 14a1 return"). Standing-admission lines and
  fixture migrations: none. Gates: fmt, clippy, the runtime suite (25
  `ok`), both bundles, strict OpenSpec and `git diff --check` are clean.

Unit 14a2 (2026-09-27, run `0065-rebuild-unit-14-see-the-uni-c96bee8d`,
based on `858e1077`; evidence.md, "Unit 14a2"). **Result: complete.**
14.1 stays open until 14b. Production: `engine.rs` and
`native_controls.rs` only.

- **Sealed beside the record.** `native_controls::SealedServing` (the
  dialect's permission flag and the sandbox, hands and boundary fragments,
  unexpanded; the pins; the typed `HandsSpec`) rides the driver input under
  `serving_inputs`, a closed, kind-tagged JSON sibling of `launch_record`.
  `mark_capabilities` seals it from the selected candidate's composition
  or the inline site's facts, and selects the boundary fragment by the
  class `SiteSpawn` now records from composition. A planted copy, an
  unrecorded dialect and an unclassed spawn carrying fragments each
  refuse. The dispatch door admits exactly the sealed inputs, on the
  no-record branch too.
- **Decode.** Anything missing, null, mistyped, unknown-tagged, carrying
  an unknown member or a non-canonical hands declaration refuses with an
  exact, bounded reason.
- **`hands_command`** expands through `Transport::expand` (closes the
  13-fix-b follow-up). A non-UTF-8 executable or workdir refuses the
  spawn.
- **Proofs.** Four new tests. MH1 (the old encoder) is the baseline red.
  MH1–MH3, ME1–ME10 (with ME2g and ME3p/h/s) and MS1–MS5 each fail at a
  named row, and each was restored and checked with `cmp`.
- **Owning-suite updates.** Ten `hands_command` call sites state its
  `Result`, and three whole-`SiteSpawn` expectations state its class.
  **Standing-admission lines:** two `site.inline_dialect =
  Some(Default::default());` in hand-built inline fixtures
  (`capability_tests.rs:1046`, `:1141`). Fixture migrations: none.
- **Gates.** fmt, clippy, the protocol and runtime suites (28 `ok`), the
  CLI suite (33 `ok`), both bundles, strict OpenSpec and `git diff
  --check` are clean.
- **Pending.** 14b, exact coverage outside the box, macOS, remote CI and
  the council.

Unit 14b (2026-09-27, run `0065-rebuild-unit-14-see-the-uni-f3db2206`,
based on `e7267446`; evidence.md, "Unit 14b — stopped"). **Result:
oversized.** Nothing was committed but these records, and 14.1 stays open.

- The seams were built in `adapters.rs` and set aside as
  `.forge/unit-14b/unit-14b-seams.patch`, which applies cleanly. On that
  patch the workspace suite had 4 failures; the protocol suite passed.
- **A.** `check_final` refuses `unspecified` + `dormant` (`native_controls.rs:2164-2176`),
  which `agents::compose` seals for every agent with hands and no allow
  (`agents.rs:1198-1200`). Two compiled Claude seats in
  `capability_launch.rs` fail at `:6457` and `:6751`.
- **B.** `check_final` requires the hands to carry the transport's server
  (`native_controls.rs:2332-2347`), but an inline hands site is sealed
  `required` (`engine.rs:4940-4943`) with no workspace fragment
  (`bundle.rs:505-511`). The real engine's inline Codex panel member is
  refused in `driver_conformance.rs:2781`.
- **Ruling needed.** Which side changes for A and for B, and the file each
  ruling names; neither is `adapters.rs`. Then 14b re-runs from the patch.
- Fixture migrations and standing-admission lines: none committed.
- **Pending.** That ruling, then 14b, exact coverage outside the box,
  macOS, remote CI and the council.

Unit 14b, second visit (same run, after triage re-ruled it a chore;
evidence.md, "Unit 14b — second visit"). **Result: blocked.** No ruling
on A or B exists; the production lines both cite are unchanged, and the
patch still applies. Nothing but these records moved; 14.1 stays open.

Unit 14a3 (2026-09-27, run `0065-rebuild-unit-14-see-the-uni-07ead96a`,
based on `522dde93`; evidence.md, "Unit 14a3"). **Result: complete.** 14.1
stays open until 14b. The operator's rulings are recorded alone as the
addendum "2026-09-27: hands make the allow list dormant, and inline hands
are served" (`420a0304`). Production: `native_controls.rs` and `bundle.rs`
only.

- **A.** `check_final`'s table admits `unspecified` with `dormant` exactly
  where hands are required, and the refusal names that. The rows in
  `the_sealed_inputs_are_checked_independently_of_the_command` assert two
  things: beside hands the shape rebuilds the exact cold command, and
  without them it refuses with the exact cause. The baseline red on the old
  code panics at the admission. Mutations `=> true` and `=> false` each
  fail at a named line.
- **B.** An inline site with hands carries its dispatch driver's declared
  `hands.workspace` fragment in `inline_dialect.hands`, unexpanded, beside
  its `HandsSpec` (`record_capabilities`). The compile refuses inline
  model-harness hands unboxed (decision 0046 ruling 4), so no boundary
  filter is needed. Two rows were added to
  `an_inline_sites_composition_carries_each_serving_input_as_its_adapter_declares_it`:
  "codex with boxed hands" and "codex without hands". The baseline red
  fails the first. Mutations B1 and B2 fail named rows.
- **Next for 14b** (its patch was applied as scratch and reverted):
  - A's two seats pass the table. They now meet a plan-denial refusal
    (`:6457`) and R1 on a fixture without the MCP server (`:6751`).
  - B's inline member now meets the count check: its plan types 0 hands
    arguments. That plan type (`bundle.rs`) and the missing inline hands
    emission (`engine.rs`) are outside this ruling.
- Fixture migrations and standing-admission lines: none.
- **Gates.** fmt, clippy, the protocol and runtime suites (28 `ok`), the
  CLI suite (33 `ok`), both bundles, strict OpenSpec (18 passed) and `git
  diff --check` are clean.
- **Pending.** 14b, exact coverage outside the box, macOS, remote CI and
  the council.

Unit 14b, third visit (2026-09-27, run `0065-rebuild-unit-14-see-the-uni-d32a2322`,
based on `5e483c52`; evidence.md, "Unit 14b — third visit"). **Result:
oversized.** Nothing but these records moved, and 14.1 stays open. The patch
was applied, run and reverted, and it still applies.

- The protocol suite passes. Of the three failures A and B named, none
  passes yet, and none can be fixed in `adapters.rs`:
  - `capability_launch.rs:6457` passes the table and refuses in
    `check_final`'s denial pass (`native_controls.rs:2538-2554`). The plan
    holds `web-fetch` and also denies it, through an unselected second
    entry for the same capability.
  - `:6751` passes the table and refuses at R1. These are hands under the
    `harness` boundary: a managed limit and no MCP server, beside a sealed
    workspace fragment.
  - `driver_conformance.rs:2781` meets the count check. The inline plan
    types 0 hands (`bundle.rs`), and the engine emits no inline hands
    (`engine.rs`).
- The in-scope row `:3752` was not moved, because no production change
  landed.
- **Split needed.** Three rulings and units: inline hands provenance and
  emission (`bundle.rs`, `engine.rs`); hands under `harness`
  (`native_controls.rs` or `agents.rs`); an unselected entry for a held
  capability (`native_controls.rs`, or the plan). Then 14b re-runs.
- Fixture migrations and standing-admission lines: none.
- **Pending.** Those rulings and units, 14b, exact coverage outside the
  box, macOS, remote CI and the council.

Unit 14a4a (2026-09-27, run `0065-rebuild-unit-14-see-the-uni-fee33bd3`,
based on `24c51d8f`; evidence.md, "Unit 14a4a"). **Result: complete.** This
follows through on ruling (B). 14.1 stays open until 14b. Production:
`bundle.rs` and `engine.rs` only.

- An inline site with hands records its `hands.workspace` fragment,
  expanded as an agent's `hands` segment is, as `SiteFacts::inline_hands`.
  It is recorded before its plan resolves, so the plan types its length
  (it was `0`) and composes with it. `compose_site_at` appends it last as
  the engine's `hands` segment, and `hands_command` expands it through
  `Transport::expand`.
- Tests:
  - A new compiled regression in `bundle/agent_tests.rs`: inline and agent
    both type `10`, and the segment is exact and expanded.
  - A new conformance path in `capability_launch.rs`: the boxed inline
    seat's sealed hands segment, typed count and launched command equal its
    agent-backed Codex fallback's.
  - Two assertions in `capability_launch.rs` that encoded the old
    emission: the boxed inline seat's segments (`:734`), and
    `assert_intact`'s one `--sandbox` (`:483`).
  - All four were red on `24c51d8f`. Mutations M1 (`hands: 0`), M2 (no
    emission) and M3 (unexpanded) each fail named tests.
- Standing-admission line: `driver_conformance.rs:3341-3350`. It hands the
  recorder-replayed driver the extras the engine composed
  (`launch_arguments`) in place of a hard-coded no-hands argv, which the
  driver refused as not reassembling the record. No assertion or tested
  behaviour changed. Fixture migrations: none.
- **Scratch with 14b's patch** (applied, run and reverted, and it still
  applies): the `:2781` count refusal is gone, and `capability_launch`
  shows only the three recorded refusals. `:2781` now departs at argument
  12, because the in-process engine binds the test executable into the
  box's transport while the subprocess driver rebuilds it with
  `target/debug/brokkr`. This is left for 14b's re-run.
- **Gates.** fmt, clippy, the runtime suite and the CLI suite (every
  summary `ok`), both bundles, strict OpenSpec (18 passed) and `git diff
  --check` are clean.
- **Pending.** 14a4b (`:6457`, `:6751`), then 14b, exact coverage outside
  the box, macOS, remote CI and the council.

Unit 14a4a, review return (same run, from `73ff236e`; evidence.md, "Review
return, 2026-09-27"). **Result: complete.** 14.1 stays open until 14b.

- F1: `bundle.rs:1768` expands each seat's inline hands against the layer
  that wrote the seat, as an agent's are. It was the leaf. The new
  inherited regression in `bundle/agent_tests.rs` was red under the
  restored leaf directory (`bundle/schema.json` against
  `base/schema.json`), and it passes with the fix.
- F2: the claim that the inline seat's command passes the final check
  had not been observed, and it is corrected in evidence.md.
  - `capability_launch.rs` now has `checked_launch`, which runs the
    existing `check_final` on the sealed cold command with the sealed
    inputs and a transport consistent with the engine's.
  - A compiled inline Codex panel member with hands and an agent member
    both pass it. Each gives the exact written-out command.
  - The 14a4a parity test now requires that `Ok`.
  - Mutations: no emission makes the driver refuse on the count, and an
    untyped plan makes `check_final` refuse on the count, the refusal
    `:2781` met. Each was restored to a pass.
- Fixture migrations and standing-admission lines: none.
- **Gates.** fmt, clippy, the runtime suite, `driver_conformance` and both
  bundles are clean. The workspace suite, strict OpenSpec and `git diff
  --check` are recorded in evidence.md.
- **Pending.** 14a4b, 14b, exact coverage outside the box, macOS, remote
  CI and the council.

Unit 14a4a, second review return (same run, from `61b9554a`; evidence.md,
"Second review return, 2026-09-27"). **Result: complete.** 14.1 stays open
until 14b.

- C1: `record_capabilities` in `bundle.rs` now walks each select case in
  the layer that wrote it (`roots[case_origin]`), as `parse_select`
  parses it. The default stays with the seat's owner. Before, a
  leaf-written inline case expanded its hands against the base.
- The new mixed-origin regression in `bundle/agent_tests.rs` was red on
  `61b9554a`: the inline case got `base/schema.json` and the agent case
  got `bundle/schema.json`. Two compiling mutations each fail it (cases
  in the seat's directory; the default in the leaf), and it passes
  restored.
- Fixture migrations and standing-admission lines: none.
- **Gates.** fmt, clippy, the runtime suite, `driver_conformance` and both
  bundles are clean. The workspace suite, strict OpenSpec and `git diff
  --check` are recorded in evidence.md.
- **Pending.** 14a4b, 14b, exact coverage outside the box, macOS, remote
  CI and the council.

Unit 14a4b (2026-09-27, run `0065-rebuild-unit-14-see-the-uni-8b6d5d16`,
based on `48ad7d82`; evidence.md, "Unit 14a4b"). **Result: complete.** This
builds operator rulings (1) and (2) of 2026-09-27, which step 0 recorded
(`a05475f0`). 14.1 stays open until 14b. Production: `native_controls.rs`
only.

- Ruling (1): `check_final`'s denial pass (`delivered`) skips a
  capability the plan also holds, so an unselected entry's OFF for a held
  capability is no denial. The held pass still keeps that entry's tools
  unavailable.
- Ruling (2): R1 takes the sealed harness fragment for the seat's hands
  where no workspace fragment is sealed. Everywhere else it still binds
  the box transport.
- `agents.rs` did not move. Under `harness` it already seals the harness
  fragment and no `hands.workspace`. Clearing its `HandsSpec` there would
  have changed two `bundle/agent_tests.rs` rows outside this unit, so
  that attempt was reverted.
- Tests:
  - Two new `native_controls` tests with exact positive and negative
    rows. The negatives cover a selected entry's OFF still denying, and a
    `hands.workspace` or unbound workspace fragment under `harness`
    refusing.
  - Two checked compiled rows in `capability_launch.rs` (`:6634`,
    `:6946`), with `checked_launch` now per provider.
  - All four positives were red on `48ad7d82`. Five mutations each fail
    a named row.
- With 14b's patch (applied, run, reverted; it still applies):
  `:6457` and `:6751` pass. Only `:3752` (now `:3960`) and
  `driver_conformance.rs:2781` remain, and both are 14b's.
- Fixture migrations and standing-admission lines: none.
- **Gates.** fmt, clippy, the protocol and runtime suites, the workspace
  suite (77 summaries `ok`), both bundles, strict OpenSpec (18 passed) and
  `git diff --check` are clean.
- **Pending.** 14b, exact coverage outside the box, macOS, remote CI and
  the council.

Unit 14a4b, the review's return (2026-09-28, same run, on `d460e7cf`;
evidence.md, "Unit 14a4b — the review's return"). **Result: oversized.**
Nothing but these records moved. The "complete" above is withdrawn.

- **F1.** R1 admits the harness fragment only when it is nonempty. A
  declared-empty fragment under `harness` and a seat with hands under
  `open` are indistinguishable: a scratch probe (reverted) showed they seal
  the same serving inputs and the same command. Ruling (2) needs a typed
  boundary fact, and only `engine.rs` (`serving_inputs`, `:4739`) can
  seal one. That file is outside the unit.
- **F2.** The two `native_controls/tests.rs` tests move with F1's fix,
  because their harness rows change shape.
- **Split needed.** One unit, 14a4c: `native_controls.rs`, `agents.rs` and
  `engine.rs`. Its tests are the empty-fragment positive, the `open` and
  boxed negatives, and F2's relocated rows, all in the named suites. The
  ruling should also say whether older sealed serving inputs must still
  decode.
- Fixture migrations and standing-admission lines: none.
- **Pending.** The ruling, 14a4c, 14b, exact coverage outside the box,
  macOS, remote CI and the council.

Unit 14a4c (2026-09-28, run `0065-rebuild-unit-14-see-the-uni-653d5daf`,
based on `a4a184c7`; evidence.md, "Unit 14a4c — built"). **Result:
complete — withdrawn by the review's return below.** It completes ruling (2) of 2026-09-27 and closes 14a4b's F1 and
F2. 14.1 stays open until 14b.

- **Production.**
  - `engine.rs`: `serving_inputs` (now `pub`) seals the run's boundary as
    `SealedDialect::stands` for a site with hands, and `None` otherwise.
    So a declared-empty fragment pair under `harness` is never read as no
    boundary.
  - `native_controls.rs`: the closed `SealedBoundary` word, its member in
    the serving JSON and its reader, and `Dialect::stands`. R1 admits the
    harness fragment, of any length, exactly when the sealed boundary is
    `harness` and no workspace fragment is sealed. Under any other
    boundary the transport binds the hands.
- **Tests.**
  - `capability_launch.rs`: 14a4b's probe pair, compiled under `harness`
    (admitted, exact argv) and under `open` (the whole R1 refusal).
    `checked_launch` now takes the engine's own `serving_inputs`.
  - `native_controls/tests.rs`: 14a4b's two tests are adopted in place
    (F2). They gain the empty-fragment positive and five negatives
    (`open`, the box, no boundary, and a bound workspace fragment). Each
    of the five words round-trips, and four `stands` tamper rows were
    added.
  - Baseline reds for both positives were observed on `a4a184c7`.
    Compiling mutations M1–M5 each fail a named row and were restored.
- **Standing-admission lines.** `engine/capability_tests.rs:758`, `:802`
  and `:958` each gain `"stands": {"kind": "none"}`. That JSON is the
  serialised struct, so each exact literal must name the new member. No
  assertion was added or removed. The council should rule on these,
  because the JSON forces them, not the compiler.
- Fixture migrations: none.
- **Assumption.** Older serving inputs without `stands` refuse to decode
  (closed JSON, no grandfathering). Resume is unit 15's work.
- **Follow-up.** The `self.boundary` argument at `mark_capabilities` is not
  bound by any test (M6 survived).
- **Gates.** fmt and clippy are clean. The protocol and runtime suites
  pass. The workspace suite has 77 summaries `ok`. Both bundles compile.
  Strict OpenSpec: 18 passed. `git diff --check` is clean.
- **Pending.** 14b (its saved patch must name `Dialect::stands`), exact
  coverage outside the box, macOS, remote CI and the council.

Unit 14a4c, the review's return (2026-09-28, same run, on `6a8dfde7`;
evidence.md, "Unit 14a4c — the review's return"). **Result: oversized.**
The "complete" above is withdrawn: F2 needs a ruling.

- **F1, closed.** The probe pair in `capability_launch.rs` is now sealed
  by the engine itself. A run is started in a world whose realm declares
  the boundary, and `Engine::mark_capabilities` (now `pub`) seals the
  composed spawn. The test asserts the sealed `stands` exactly
  (`harness`, `open`) and checks the command against what the engine
  wrote. Mutation M7 (`Boundary::Open` at the call site) fails `:7141`.
  M7b (the same, with that line disabled) fails the `harness` `Ok`. M8
  (`Boundary::Harness`) fails `:7163`. All three were restored.
- **F2, oversized.** The three `"stands": {"kind": "none"}` lines in
  `engine/capability_tests.rs` (`:758`, `:802`, `:958`) change exact
  assertions outside the named suites. The settled design forces them:
  the closed serving JSON names every member. Removing them fails that
  test at `:750`. They stay in the tree. The unit needs an admission for
  these three assertion lines, or a split that names that file.
- Fixture migrations and standing-admission lines: none this visit.
- **Gates.** fmt and clippy clean; `brokkr-runtime` 25 summaries `ok`;
  strict OpenSpec and `git diff --check` clean.
- **Pending.** The ruling on F2, 14b, exact coverage outside the box,
  macOS, remote CI and the council.

Unit 14a4c re-fire (2026-09-28, same run, on `aa850fba`; evidence.md,
"Unit 14a4c re-fire — the stop stands"). **Result: oversized.** Triage
re-ruled `chore` with no ruling on F2: `operator-ruling-2026-09-23.md` has
no addendum after 2026-09-27, and the new framing says the 2026-09-25
admission "does not generally admit edits to expected JSON assertions".
Re-verified, not rebuilt: without the three lines the serving-inputs test
still fails at `:750`; restored, it and the probe pair pass. Nothing moved
but these records. **Pending.** The same as above.

Unit 14a4c second visit (2026-09-28, run
`0065-rebuild-unit-14-see-the-uni-411e4cb1`, on `9f5a00f9`; evidence.md,
"Unit 14a4c second visit — F2 admitted"). **Result: complete.**

- **The ruling.** The operator's ruling of 2026-09-28 admits F2's three
  lines as assertion updates only. They are
  `engine/capability_tests.rs:758`, `:802` and `:958`, each
  `"stands": {"kind": "none"}`. The optional-member encoding is not taken.
- **Nothing new was built.** The lines have been in the tree since
  `6a8dfde7`. Their baseline red on the `a4a184c7` bytes is
  `.forge/unit-14a4c-f2-forced.txt` (fails at `:750`). The test passes
  again in this visit's runtime run.
- F1 was closed on the first visit. F2 is now closed. 14.1 stays open
  until 14b.
- Fixture migrations: none.
- Standing-admission lines: only the three the ruling admits.
- **Gates.**
  - fmt and clippy are clean.
  - `brokkr-protocol` passed.
  - `brokkr-runtime` returned 25 summaries `ok`.
  - Strict OpenSpec: 18 passed.
  - `git diff --check`: clean.
- **Pending.** 14b (its saved patch must name `Dialect::stands`), exact
  coverage outside the box, macOS, remote CI and the council.

Unit 14b, fourth visit (2026-09-28, run `0065-rebuild-unit-14-see-the-uni-cd674eef`,
based on `eb36be47`; evidence.md, "Unit 14b — fourth visit"). **Result:
complete.** 14.1 closes here. Production: `adapters.rs` only.

- The saved patch landed with two edits. `served_cold` passes the sealed
  `stands`. The test helper `seal_serving` seals through the engine's own
  `serving_inputs`. The Codex, Claude/LaneTally and DSH cold seams now spawn
  only `Checked::into_argv`.
- `:6457` and `:6751` (now `:6838`, `:7146`) pass. `driver_conformance.rs:2781`
  (now `:3334`) passes with one standing-admission line (below). The
  in-scope row "unmeasured, no plan" (now `:4141`) expects the unpaired
  refusal.
- The integration moved two more expectations in the named suites. The probe
  pair `:7370` now expects the driver's own refusal prefix under `open`.
  An agent record's authored `--json` (`adapters/tests.rs:16410`) is now
  refused.
- New tests:
  - A ten-row compiled table (`capability_launch.rs:1059`): exact positives;
    dropped OFF for Claude and Codex; the `:` separator; cross-origin
    `--verbose` and `--json`. It also pins a boxed Claude seat's dropped
    deny list as still denied behind `--tools ""`.
  - A DSH seam test (`adapters/tests.rs:16461`).
  - A LaneTally refusal inside the serving-child test (`:2942`).
  - An authored terminator is refused at composition, before any seam, by
    `native_controls.rs`, so no row was committed for it.
- Baseline reds on HEAD's `adapters.rs`: seven assertions across three
  tests. Mutations M1–M5b each failed named rows and were restored.
- Standing-admission line: `driver_conformance.rs:3186-3190`. It rewrites
  the test binary's path to `brokkr_bin()` in the captured Start, because
  the in-process engine binds the test binary. No assertion was added or
  removed, and no tested behaviour changed. The council should rule on it.
  Fixture migrations: none.
- **Gates.** fmt and clippy are clean. The protocol suite passed (539, 99
  and 1). `brokkr-runtime` returned 25 summaries `ok`, and `brokkr-cli` 33
  `ok`. Both bundles compile. Strict OpenSpec: 18 passed. `git diff
  --check`: clean.
- **Pending.** Exact coverage outside the box, macOS, remote CI and the
  council. Unit 15 checks rejoins and the cold replacement.

## 15. Unit 15 — Integrate eligible resume and replacement

- [x] 15.1 Unit 15 checks actual resume and cold replacement independently. Verify session/stdin/eligibility/controls and selected fallback OFF; cold never counts as resume evidence. Requirements: [Eligible Codex resumes reimpose the capability control][NC3], [Every accepted native control reaches the final command][NCC], [Denial and admission have removal proofs and bounded live claims][NC6], [Neither inline arguments nor fallback can override native denial][NC4]. Reopened/remaining: operator ruling 1–2 / R10. (previous 7.2)

- [x] 15.2 Units 12–15 integrate refusal/provenance across compile and private launch input; close only after unit 15 covers cold, actual resume and replacement. Verify counterfeit/missing origins, structural substitutions and candidate replacement before work. Requirements: [Authored provider configuration cannot supply capability authority][RGR], [Known native powers require a valid delivered denial or refusal][NCR], [Every accepted native control reaches the final command][NCC]. Reopened/remaining: operator ruling 1–2. (previous 4.7)

Unit 15 (2026-09-28, run `0065-rebuild-unit-15-see-the-uni-4a097606`, based
on `51dcf6b5`; evidence.md, "Unit 15"). **Result: complete.** 15.1 and 15.2
close here. Production: `adapters.rs` only.

- `served` (formerly `served_cold`) checks with the engine's chosen session.
  - The Codex rejoin is checked with its thread.
  - Its cold replacement is checked on its own, with none. That checked
    command is the one pre-work replacement's, published `cold` and never
    as a resume.
  - The Claude/LaneTally `--resume` rejoin and the DSH `--session` rejoin
    are checked like their cold launches.
  - Eligibility (gate, session, version, persistence, class and effort,
    accounting, owned DSH root) is unchanged and runs first.
- Tests:
  - Two in `adapters/tests.rs`:
    - Codex: rejoin and replacement exact; each departs as the other; OFF
      dropped, unreadable and unpaired inputs refuse; version drift
      declines; the end-to-end replacement.
    - DSH: the exact rejoin, and two structural substitutions.
  - A fourteen-row compiled table in `capability_launch.rs`:
    - Codex inline and agent rejoins, ON and OFF.
    - The selected fallback, declined and served cold with its OFF.
    - Claude rejoins, unboxed and boxed.
    - Refusals: dropped OFF, an unreadable member, a missing record, and
      counterfeit origins both ways.
- Baseline reds on HEAD's `adapters.rs`: both unit tests, and six compiled
  rows.
- Mutations: M1, M2, M4 and M5 fail named rows. M3 and M6 (either Codex
  check removed alone) survive as equivalent mutants: both commands come
  from one validated composition, so the other check refuses every tampered
  input first. They are recorded as such.
- Fixture migrations and standing-admission lines: none.
- **Gates.**
  - fmt and clippy: clean.
  - Protocol suite: 541, 99 and 1 passed.
  - `brokkr-runtime`: 25 `ok`.
  - `brokkr-cli`: 33 `ok`.
  - Both bundles compile.
  - Strict OpenSpec: 18 passed.
  - `git diff --check`: clean.
- **Pending.** Exact coverage outside the box, macOS, remote CI and the
  council. Whether a resumed provider honours the controls is still owed
  to the controller.

Unit 15, review return (2026-09-28, same run, based on `8809662a`;
evidence.md, "Unit 15, review return"). The chief's SC15-1 (medium) stands:
the record above closed 15.2 too early. `served` read only the authored
segments of the sealed record and never checked the whole ordered record
against the driver's arguments. An agent-backed Codex rejoin whose record
was emptied, reversed or grown by a hands token was served as if intact.
**Result: complete.** 15.1 and 15.2 close on this return.

- `served` (`adapters.rs:3021`) takes `handed`, the arguments the driver
  was handed. It refuses with `reassemble`'s whole reason
  (`native_controls::reassemble`) before it reads any origin (`:3046`).
  Every caller passes its own: Codex `extra` for the rejoin and the cold
  replacement, Claude/LaneTally's pre-composition `extra` (`:3451`), and
  DSH's `extra` through `dsh_served` (`:4526`). Cold launches go through
  the same function, so unit 14b's cold seams gain the same check.
- Tests:
  - `adapters/tests.rs`: the Codex rejoin test runs three tampered records
    (emptied, reversed, grown) through the rejoin and the replacement
    separately. Each gets the exact reassembly refusal.
  - `capability_launch.rs`: the compiled table grows from 14 rows to 19. The
    new rows are the Codex agent rejoin with its record emptied (0 of 6),
    reversed (at 0) and grown (at 6, 7 of 6); the selected fallback served
    cold with its record grown (at 12, 13 of 12); and a boxed Claude rejoin
    with its record reversed (at 0, 13 of 13).
  - The DSH `dsh_served` calls and the two direct `served` calls in
    `adapters/tests.rs` pass the new argument. That file is named by the
    unit, so this is not an admission.
- Mutations: M1 drops the check, which is HEAD's behaviour and the baseline
  red. All five new rows and the Codex unit test fail. M2 checks only an
  empty record: four rows and the unit test fail on reversal. M3 checks
  only rejoins: the fallback row and the unit test's replacement assertion
  fail. All restored; the tests pass.
- Fixture migrations and standing-admission lines: none.
- Gates: fmt and clippy are clean. `cargo test --workspace --all-features
  --locked` has no binary with a failure. `bundles/self` compiles. Strict
  OpenSpec passes 18. `git diff --check` is clean.
- **Pending.** Exact coverage outside the box, macOS, remote CI and the
  council. Live resumed-provider enforcement is still owed.

Unit 15, second review return (2026-09-28, same run, based on `4a2b3a6a`;
evidence.md, "Unit 15, second review return"). **Result: oversized.** 15.1
and 15.2 are reopened: the record above closed them too early.

- SC15-R2-1 (medium) stands and is not built here. `served` still serves a
  launch whose input carries the engine's plan (`native_controls`) but
  neither sealed input as composed. The guard it asks for was built and
  measured, then reverted. It breaks 93 of 541 protocol lib tests and 19 of
  58 `capability_launch` tests. Their fixtures hand the driver the plan
  without the sealed pair: `engine_input` in `adapters/tests.rs`, and
  fixtures such as `try_launch` in `capability_launch.rs`. Sealing those fixtures moves more
  than a hundred tests owned by earlier units onto the final check. That is
  a fixture migration no standing admission covers and not a narrow visit.
  The split it needs is in evidence.md.
- SC15-R2-2 (low) is answered. In
  `a_compiled_rejoin_is_served_only_as_its_final_check_returns_it`, the
  selected fallback's cold command and the boxed Claude hands are now
  independent ordered literals. Only the test executable, which `{brokkr}`
  binds, is substituted. They no longer use `Transport::expand`, the adapter
  fragments or `boxed_hands`/`checked_codex`. Two mutations in
  `native_controls.rs` fail the new literals and pass HEAD's test. Each is
  one production change that moves the actual and the old expected command
  together: the TOML argument join `","` to `", "`, and a trailing space on
  the MCP document. Both were restored.
- CH15-RUN-1 (low) is a panel-prose defect and needs no change here.
- Fixture migrations and standing-admission lines: none.
- **Pending.** SC15-R2-1, then exact coverage outside the box, macOS,
  remote CI and the council.

Unit 15, fourth visit (2026-09-28, same run, re-fired by triage, based on
`e6f45e97`; evidence.md, "Unit 15, fourth visit"). **Result: oversized
again.** No addendum rules the 15-fix-a/15-fix-b split. The guard was
re-measured and then reverted: 92 protocol lib and 19 `capability_launch`
failures. 15.1 and 15.2 stay open. Fixture migrations and standing-admission
lines: none.

Unit 15-fix-a (2026-09-28, run `0065-rebuild-unit-15-see-the-uni-a2f84ac0`,
based on `0558fa5d`; evidence.md, "Unit 15-fix-a"). **Result: blocked.**
Tests only; `adapters.rs` is unchanged.

- Unit 15's `UNSEALED` arm was re-applied only for measurement, then
  reverted. Before migration it broke 92 protocol lib tests (79 by
  `ADAPTER_ENV` poisoning) and 19 in `capability_launch`.
- Helpers:
  - Runtime: `seal_as_dispatch` seals the way `mark_capabilities` does.
    `try_launch` and `rejoin` use it.
  - Protocol: `sealed_pair` and `Seal` sit beside `engine_input`.
- Under the guard, all 19 runtime tests pass, and 82 of 92 protocol tests.
  Seven of those 82 had their fixtures migrated. Each test is named in
  evidence.md.
- Four fixture inputs moved beyond adding the pair, and need a ruling:
  - the class order;
  - counterfeit hands replaced by the shipped fragment;
  - `model_verbosity` replaced by `model_reasoning_effort`;
  - a read-only class without a capture, sealed as a template.
- Ten tests cannot pass with their assertions unchanged:
  - Two assert the unsealed-with-plan exemption. 15-fix-b must change them.
  - Five assert that authored content launches, content the final check
    refuses by construction: `-o`, authored lists, `--settings`, and
    `/run/hands.json`.
  - In three, `claude_plan`'s `local` contradicts the argv.
- 15.1 and 15.2 stay open. Standing-admission lines: none.
- Gates: fmt, clippy, the protocol and runtime suites, `bundles/self`,
  strict OpenSpec (18 passed) and `git diff --check` are clean.

Unit 15-fix-a, re-fired (2026-09-28, same run, based on `92f037c3`;
evidence.md, "Unit 15-fix-a, re-fired"). **Result: blocked.** No ruling
has landed since the blocked record, and the framing has not changed. The
guard was re-applied and re-measured, and it fails the same ten tests. It
was then reverted. No test file moved. 15.1 and 15.2 stay open.

Unit 15-fix-b (2026-09-28, run `0065-rebuild-unit-15-see-the-uni-d4133989`,
based on `a7518495`; evidence.md, "Unit 15-fix-b"). **Result: complete.**
15.1 and 15.2 close on this evidence. This answers SC15-R2-1, under the
operator's rulings of 2026-09-28 on the split and on the ten.

- Production, `adapters.rs` only: `served` refuses with the whole reason
  `UNSEALED` when the input carries `native_controls` with neither
  `launch_record` nor `serving_inputs`. Only a launch with no plan and no
  sealed input keeps the by-hand exemption.
- New test: `the_engines_plan_with_neither_sealed_input_is_refused_at_every_seam`.
  It covers the eligible Codex rejoin, driven whole with nothing spawned,
  then that rejoin and its cold replacement each through `served` on its
  own, then the Claude cold seam. The DSH and inline Codex cold seams are
  the two exemption tests' changed rows.
- The ten tests are changed as ruled; evidence.md lists each change:
  - The two exemption rows now assert the exact `UNSEALED` refusal.
  - The five authored-content tests are sealed. They assert the final
    check's exact refusal, or, where the row's purpose is a launch, the
    typed allow or typed hands with the transport's own document.
  - The three contradictory-plan tests re-plant `local` to `[]`.
- Fixture moves beyond the ruled ones, all in `adapters/tests.rs`:
  - `Seal` gains `pins` and `Seal::claude`, and `claude_sealed` is added.
  - Where a sealed row's argv carries no lowered list, its plan's `local`
    is `[]` (evidence.md names each test).
  - The LaneTally row's plan names LaneTally.
- Baseline red: under the guard with the old fixtures, the ten failed. The
  migrated tests passed after the change. Against `a7518495`'s
  `adapters.rs`, three failed, each on its unsealed row: the DSH row, the
  inline Codex row, and the new test.
- Mutations M1 to M5, each compiling and restored: exempt cold servings,
  exempt rejoins, exempt Claude, exempt DSH, and drop the by-hand exemption.
  Each fails exactly the rows it should.
- Standing-admission lines: none.
- Gates: fmt, clippy, the protocol, runtime and CLI suites, `bundles/self`,
  strict OpenSpec (18 passed) and `git diff --check` are clean.
- Pending: exact coverage outside the box, macOS, remote CI and the council.

Unit 15-fix-b, review return (2026-09-28, same run, based on `69e0dd75`;
evidence.md, "Unit 15-fix-b, review return"). **Result: complete.** 15.1
and 15.2 stay closed, now also on the proofs the council's residual asked
for. No production file moved.

- CH15B-2 (low): in `claude_admits_only_held_native_tools_beside_its_hands`,
  the expected MCP document and the neither-held row are now independent
  ordered literals. Only the test executable is substituted. A trailing
  space on `Transport::document` fails the new test at `:15996`, and HEAD's
  test passes under it. The mutation was restored.
- CH15B-1 (medium): each changed sealed assertion now has a baseline pass,
  a compiling production mutation that removes its protection, the failure
  it caused, and a restored pass. That is 16 mutations across
  `native_controls.rs` and `grammar.rs`: the result capture, the
  sandbox-class read, the repeat rule, the inert recipe's-words check
  (all harnesses, and LaneTally alone), the plain-name read (all, and
  LaneTally alone), the carried-allow origin, the D11 restriction (cold,
  and resume alone), and five list folds (joins, created lists, two-name
  lists, a first-name-only join, lists beside no `--tools`), plus the
  Codex OFF argv. Rows that share a loop's assertion with a row reached on
  their own are listed in evidence.md.
- Standing-admission lines and fixture migrations: none beyond the ruled
  CH15B-2 literal.
- Gates: fmt, clippy, the protocol suite (542 lib), strict OpenSpec (18
  passed) and `git diff --check` are clean.
- Pending: exact coverage outside the box, macOS, remote CI and the council.

## 16. Unit 16 — Bind canonical inputs and policy bytes

- [ ] 16.1 Unit 16 resolves actual files/owners and refuses outward/excluded/nonregular/unpinned inputs. Bind the verified read to the contained target by handle or refuse. Verify controlled replacements, equal-byte outward links/FIFOs and standalone/inherited full causes; path-string checks alone prove no race guarantee. Requirements: [Active instructions and policy cannot escape bundle identity][MPI], [Library charter pins are enforced at consumption][MPL]. Reopened/remaining: operator ruling 3. (previous 5.1) Reopened by unit 16-fix-c (2026-09-28): the alias acceptance's proof (baseline red, caught mutation, restored pass on a filesystem that accepts a case alias) is pending, so this task is not claimed complete. Unit 16-fix-d (2026-09-28): the alias surface was removed under the refusal ruling, so the pending alias-positive proof is retired, not claimed; a spelling its directory does not list is refused on every filesystem. Kept open by the 16-fix-d return (2026-09-28, council F5): the case-insensitive row of that refusal is unobserved (it needs macOS or a casefolded directory, and this seat refuses `unshare`), so this task is not claimed complete until that refusal is observed. Still open after the second 16-fix-d return (2026-09-28, council F4 on `39f6fddd`): this seat was refused `chattr +F` as well, so that row stays unobserved.

- [x] 16.2 Unit 16 binds regular policy read/hash/parse to owner pin, comparing later walk before sealing. Verify FIFO/changed-buffer refusal and allowed identity movement. Requirements: [Active instructions and policy cannot escape bundle identity][MPI], [Capability authorization participates in bundle identity][MP2]. Reopened/remaining: operator ruling 3. (previous 5.2)

- [x] 16.3 Unit 16 replaces outward-link positives with containment refusal. Verify four role/policy standalone/inherited escapes and contained-link/byte-change controls. Requirement: [Active instructions and policy cannot escape bundle identity][MPI]. Reopened/remaining: operator ruling 3. (previous 6.1)

Unit 16 (2026-09-28, run `0065-rebuild-unit-16-see-the-uni-2d820f61`, based
on `ca51d8d0`; evidence.md, "Unit 16"). **Result: complete.** 16.1, 16.2
and 16.3 close here. Production: `bundle.rs` and `bundle/compose.rs` only.

- `bound_input` (in `bundle.rs`) replaces `active_input`. It serves both
  the inline role (`parse_role`) and every layer's table (`own_table`).
  - The written reference keeps its lexical checks: out of the layer,
    under a skipped tree, or through `..`.
  - The canonical target is then resolved once. It must stand inside the
    declaring layer (the new outward refusal, even with equal bytes) and
    outside every skipped tree.
  - The target must be a regular file before any open. It is opened
    `O_NONBLOCK`, and the handle's own kind is checked. The path is resolved
    again and must still be the same target, whose `(dev, ino)` must be the
    handle's. The bytes come from that handle only.
  - A host other than Linux or macOS refuses (the host cannot bind the read).
- The table is parsed from the bound buffer. Its digest is compared with
  the declaring layer's walk before that layer's identity is sealed. That
  covers every ancestor in `resolve`, overridden ones included, and the leaf
  in `assemble`.
- Tests, all in `bundle/compose_tests.rs`:
  - The revoked outward-link positive in
    `a_symlink_and_a_parent_step_cannot_carry_an_active_input_out_of_the_pin`
    is now the exact refusal.
  - Five new tests: the four escapes with equal and changed bytes, plus a
    directory link; contained links followed, with each byte change moving
    identity; FIFO, directory and missing inputs; controlled replacements
    through a test-only stage hook; and a table changed between its read and
    its walk.
- Baseline red on `ca51d8d0`'s production: three tests fail. Mutations
  M1–M13, each compiling and restored, fail the rows they should. M6 (no
  `O_NONBLOCK`) hangs and is killed at 60 s.
- Standing-admission lines and fixture migrations: none.
- Gates: fmt, clippy, the runtime (579 lib) and CLI (482 lib) suites,
  `bundles/self` and `bundles/verify`, strict OpenSpec (18 passed) and
  `git diff --check` are clean. A crate-scoped `llvm-cov` diagnostic found
  one new unhit line (a missing table), and an assertion now covers it.
- Pending: exact coverage outside the box, macOS, remote CI and the council.

Unit 16, second visit (2026-09-28, run `0065-rebuild-unit-16-see-the-uni-dcd20d0c`,
based on `d9751252`; evidence.md, "Unit 16, second visit"). **Result:
complete.** It answers the review return: S16-1/C1/SC2, S16-2/C2/SC1, SC3
and S16-3/SC4. 16.1, 16.2 and 16.3 stay closed. Production: `bundle.rs` and
`bundle/compose.rs` only.

- **The declared reference is pinned (S16-1, C1, SC2).** `bound_input` also
  returns the walk key of the reference as written. `Consumed::check` needs
  the parsed digest under that key and under the target's key. A link
  retargeted after the read, with the old target left in place, is refused
  at the leaf and at an overridden ancestor.
- **The handle holds the checked file (S16-2, C2, SC1).** The kind check's
  `(dev, ino)` is kept. The handle must hold that same file, so a file or a
  parent renamed over the target before the open is refused. This holds for
  equal bytes and changed bytes. The post-open checks run while the handle
  holds the file, so its number cannot be reused.
  **Corrected by unit 16-fix (R2):** this overstated the binding. The kind
  check was a path `metadata` taken before any handle existed, so nothing
  held the checked file between that check and the open. A file unlinked
  and recreated there could take its freed number, and `bb90cebb` then
  accepted and compiled the new file (observed on ext4, below).
- **Bounded, sourced refusals (SC3).** A missing or unresolvable input now
  names the declaring layer's file, the seat (for a role), the kind and the
  reference. The clause is "which does not exist" or "which cannot be
  resolved (<io kind>)". A table no longer surfaces a bare io error. An
  authored reference goes through the new `bounded_reference`, and the site
  through `bounded_site`. Over 128 bytes, a reference is named by its lead
  and its length.
- **Independent containment proofs (S16-3, SC4).** The target's key is taken
  by `let … else`, so the outward refusal no longer depends on the
  `expect("contained target")` panic. The outward test is split into role
  and table tests. Their mutations reach the exact assertions.
- Tests in `bundle/compose_tests.rs`:
  - Three new: `a_table_link_retargeted_after_its_read_is_refused`,
    `a_replacement_before_the_open_is_refused_with_equal_or_changed_bytes`
    and `a_missing_or_unresolvable_input_names_its_source_kind_and_reference`.
  - The FIFO test's bare-io absent row moves to the last of these.
- Three baseline reds were observed on `d9751252`. Mutations N1–N12b each
  compile, fail their intended assertion, and are restored. The first
  visit's M3 (handle kind) is now subsumed by the identity check and no
  longer binds on its own. It stays as a guard against inode reuse.
- Standing-admission lines and fixture migrations: none.
- Gates: fmt, clippy, the runtime (25 binaries, 583 lib) and CLI (33
  binaries, 482 lib) suites, and the other workspace crates (20 results,
  0 failed) all pass. `bundles/self` (`45dc1c7e…`) and `bundles/verify`
  (`f7cbd4bb…`) compile. Strict OpenSpec (18 passed) and
  `git diff --check` are clean.
- Pending: exact coverage outside the box, macOS, remote CI and the council.

Unit 16, third visit (2026-09-28, run `0065-rebuild-unit-16-see-the-uni-f7be5d6c`,
at `9075fb60`; evidence.md, "Unit 16, third visit"). **Result: complete.**
It re-verifies the review return against `9075fb60`, whose review never
completed because of the classifier. No production or test file moved.
16.1, 16.2 and 16.3 stay closed.

- The focused suite passed (34). With `d9751252`'s production under the
  current tests, the same three baseline reds were observed.
- Nine mutations were re-run: R1, R2, R5, R5b, R5d, R7, R8, R9 and R12a,
  matching N1, N2, N5, N5b, N5d, N7, N8, N9 and N12a. Each failed its
  recorded assertion with the recorded value and was restored.
- Standing-admission lines and fixture migrations: none.
- Gates: fmt and clippy are clean. The runtime suite (25 results, 583 lib)
  and the workspace suite, with and without all features (77 results each,
  0 failed), pass. `bundles/self` (`45dc1c7e…`) and `bundles/verify`
  (`f7cbd4bb…`) compile. Strict OpenSpec (18) and `git diff --check` are
  clean.
- Pending: exact coverage outside the box, macOS, remote CI and the council.

Unit 16-fix (2026-09-28, run `0065-rebuild-unit-16-see-the-uni-ae47dd7f`,
based on `bb90cebb`; evidence.md, "Unit 16-fix"). **Result: complete, with
the pending items below.** It repairs the council's R1–R3 under "one read,
one set of bytes". 16.1, 16.2 and 16.3 stay closed on this visit's
evidence. The SECURITY-HOLD is the council's to lift, not this record's.
Production: `bundle.rs` and `bundle/compose.rs` only.

- **R1, declaring documents.** `read_layers` reads each layer's
  `bundle.json` through `bound_input`, once. It parses that buffer and keeps
  its digest (`Pinned`). `Consumed` now carries the layer's document and its
  optional table. `check` refuses a walk that does not pin the document's
  bytes, then checks the table as before. Every ancestor is checked in
  `resolve` before its digest is sealed, overridden ones included; the leaf
  is checked in `assemble` (`Resolved::leaf_read`). A `bundle.json` that
  links out of its layer, or is not a regular file, is refused like any
  consumed input.
- **R2, no metadata-then-path gap.** The pre-open path `metadata` is gone.
  `read_bound` opens the target once, non-blocking, and checks the handle's
  own kind. It reads from that handle. Then, while the handle still holds
  the file, it requires the written path to resolve to the same contained
  target, and that target to be the held file. Nothing reopens the path.
  `ReadStage` now runs `Opened`, `Checked` (the handle's kind), `Read`
  (before re-verification).
  - Correction of 16.1's earlier claim: a file put at the target before the
    open is the file opened and checked. It is bound, not refused. Any
    replacement after the open is refused.
  - The resolution is not descended by directory handle (no `openat`
    without a new dependency or `unsafe`). `read_bound`'s documentation
    now says so, instead of the overstrong claim.
- **R3, aliases.** Where no step of the written path is a link, the
  reference is keyed by its target's name, the name the directory lists. So
  a case or normalization alias binds to the listed entry. Through a link it
  keeps the written spelling. A key the walk does not list gets a new
  refusal ("lists under no entry of the name it was read by: … an alias … or
  that entry was removed after the read"), never "bytes changed".
- **Tests** (`bundle/compose_tests.rs`). Five new:
  `a_declaring_document_replaced_after_its_read_is_refused`,
  `a_declaring_document_is_bound_like_any_consumed_input`,
  `an_input_unlinked_and_recreated_after_its_check_is_never_read`,
  `a_bound_read_supplies_its_handles_bytes_and_never_a_second_reads` and
  `a_reference_is_keyed_by_the_entry_its_directory_lists`.
- **Unit 16's own tests, changed:**
  - `a_replacement_between_check_and_read_…`: the removal row now expects
    *replaced* (it said "cannot be read (entity not found)"), and a new
    socket row covers a failed open.
  - `a_table_link_retargeted_…`: it retargets at the next bound read (the
    charter, or the overriding leaf's table), because a retarget before
    verification is now refused as replaced. Its expected refusal is
    unchanged.
  - Each change is bound (evidence.md).
- **Proof.**
  - Baseline on `bb90cebb`: 5 reds on tmpfs. On the ext4 root there are 6,
    including the inode reuse (`number reused: true`, compiled).
  - Mutations MA–MN each compile, fail their intended assertion, and are
    restored. MI (key always the written spelling) survives on Linux: no
    Linux filesystem here accepts an alias, so it is pending on macOS.
- Standing-admission lines and fixture migrations: none.
- **Gates.** fmt and clippy are clean. The runtime suite passes (25
  results, lib 588), as does the workspace suite with and without all
  features (77 results each, 0 failed). `bundles/self` (`45dc1c7e…`) and
  `bundles/verify` (`f7cbd4bb…`) compile unchanged. Strict OpenSpec (18) and
  `git diff --check` are clean.
- **Pending.** Exact coverage outside the box. macOS: the R3 alias branch,
  MI, and the socket row's "uncategorized error" wording. Remote CI and the
  council.

Unit 16-fix-b (2026-09-28, run `0065-rebuild-unit-16-see-the-uni-f91ce8a1`,
based on `e04637a6`; evidence.md, "Unit 16-fix-b"). **Result: complete, with
the pending items below.** It repairs the chief's F1–F5 on unit 16-fix's
SECURITY-HOLD. F6 is not acted on: this record argues for no gate outcome.
16.1, 16.2 and 16.3 stay closed on this visit's evidence. Production:
`bundle.rs` and `bundle/compose.rs` only.

*Corrected by unit 16-fix-c (chief F2):* those claims were not all
observed.

- F1 was not fully repaired. A hard link could stand in for a hidden
  reference; on `c908db9d` that reproduced as a sealed identity equal to a
  stable compile of other bytes.
- F4's alias proof stayed pending (M14 survived).

So "repairs F1–F5" and "16.1 … stay closed" did not hold. 16.1 is reopened
above.

- **Correction of 16.1's claims (F5).** Two earlier notes overstated the
  binding:
  - Unit 16-fix said "any replacement after the open is refused".
  - Unit 16-fix said "nothing reopens the path".

  The guarantee is narrower. The buffer a caller parses, and whose digest
  its layer's walk takes, is the one read from the file the resolution
  held; if the reference no longer resolves to that file in the same steps,
  there is a refusal. A swap made after the open and undone before the
  check is not seen, and the buffer is still the held file's
  (`a_bound_read_supplies_…`). Until this visit the walk did reopen consumed
  files by path (F3).
- **F2, owner-rooted.** `observe` resolves a reference from its layer's
  directory handle one name at a time. Each name is opened inside the handle
  before it, with `openat` and `O_NOFOLLOW | O_NONBLOCK`. A link's text
  comes from `readlinkat` and is followed by this code: `..` above the
  layer, an absolute text outside it, or more than 40 links is refused.
  Nothing is canonicalized and then opened.
  - Both calls are declared through FFI in `bundle.rs`, so there is no new
    dependency. This is the crate's first `unsafe`: two calls and one
    `from_raw_fd`.
  - A host other than Linux or macOS refuses.
- **F1, one observation.** Both keys, and whether a written step was a link,
  come from that observation. After the read, the reference is observed
  again and must take the same steps. At the walk the input is verified
  once more (`Held::intact`): observed again, and its bytes read back
  through the held handle. A reference entry replaced after the read is
  refused, for tables, declaring documents, charters and ancestors.
  Observation errors fail closed.
- **F3, one set of bytes.**
  - `walk_files` takes each consumed key's digest from its bound buffer and
    never reads that path.
  - `parse_role` keeps each charter's read (`CharterRead`).
  - Ancestors are sealed in `Resolved::seal`, once the compile has bound
    every charter, so no walk reads a charter that was consumed. Ancestor
    refusals now carry the chain note.
  - `resolve` keeps its old contract by sealing with no charters.
- **F4.** The alias rows run on `cfg!(target_os = "macos")`, not on a probe.
  Linux accepts no alias here, so their proof, and M14's (MI's analogue),
  is macOS's. That is stated in the test.
- **Tests** (`bundle/compose_tests.rs`). Five new:
  `a_reference_replaced_after_its_bound_read_is_refused`,
  `a_charter_is_pinned_from_the_buffer_it_was_read_into`,
  `the_walk_never_reads_what_a_bound_read_supplied`,
  `a_resolution_descends_the_directory_handles_it_holds` and
  `a_link_loop_names_no_file_and_an_absolute_contained_link_is_followed`.
  Changed assertions:
  - Four ancestor rows now expect the chain note:
    `a_declaring_document_replaced_…` (ancestor and overridden ancestor),
    `a_table_changed_…` and `a_table_link_retargeted_…` (overridden
    ancestor). They are bound by M11.
  - One exhaustive `match` in `a_bound_read_supplies_…` gains `_ => {}`.
    No assertion moves.
- **Proof.**
  - Baseline on `e04637a6`: 8 reds. Every F1 and F3 row (19) compiled. F1's
    and the walk's rows needed a scratch stage hook at F1's window and
    before the walk's read.
  - Mutations M1–M16 each compile and are restored. All are caught except
    M14 (the alias key), which survives on Linux as expected. M9 hangs and
    is killed at 60 s.
- **Standing admission (2026-09-25):** nine compiler-forced arguments in
  `bundle/tests.rs` (evidence.md lists each). None adds or removes an
  assertion. Fixture migrations: none.
- **Gates:** see evidence.md, "Unit 16-fix-b", "Gates".
- **Pending.** Exact coverage outside the box. macOS: the alias rows and M14,
  the socket row's wording, the `openat`/`readlinkat` FFI and `O_NOFOLLOW`
  (0x0100) on a macOS host. Remote CI and the council.

Unit 16-fix-c (2026-09-28, run `0065-rebuild-unit-16-see-the-uni-0a952e68`,
based on `c908db9d`; evidence.md, "Unit 16-fix-c"). It answers the chief's
F1–F5 on unit 16-fix-b's SECURITY-HOLD; F6 is not acted on. Production:
`bundle.rs` and `bundle/compose.rs` only. **16.2 and 16.3 stay closed on
this visit's evidence. 16.1 is reopened: F2's alias proof is pending.** So
this record does not claim F1–F5 repaired.

*Corrected by unit 16-fix-d (chief F1–F3 on 16-fix-c):* F1 was not
repaired. Its committed test put the name back before the second
observation, so it proved only that interleaving. With the name hidden at
each observation and put back between them, both observations bound the
other entry `h.json`, and the compile sealed ruling B's bytes under
`policy.json` while it parsed A: observed on `95d4ff19` as `compiled to
32393b07…`. Two consumed hard-link names were admitted (F2), and a failed
candidate lookup was discarded (F3). 16.2's policy binding was therefore
also affected. Unit 16-fix-d removes the alias search and repairs these.

- **F1, the key belongs to the handle.**
  - `observe` builds one `Binding`: both keys and the `(dev, ino)` of the
    handle read.
  - Each name is found by listing the held directory handle
    (`fdopendir`/`readdir`), never a path. It is the name read by where
    that is listed; otherwise exactly one other entry holding the file (an
    alias). None is *replaced*; two or more are refused naming two. A
    listing error refuses.
  - The post-read check and `Held::intact` compare the whole binding.
  - The walk refuses a file that is a consumed file under another name,
    naming both.
  - On `c908db9d` the chief's interleaving reproduced: the raced compile
    sealed `32393b07…`, which is a stable compile of the other ruling. It
    is refused now.
  - K1, which drops the key comparison, compiles to `32393b07…` again.
- **F2, F3.** The alias rows are gated by a probe of the fixture's own
  canonical root (`accepts_case_alias`), not by `target_os` or TMPDIR.
  Where the root accepts no alias, the exact missing error is asserted.
  - T1 (probe forced to "accepts") fails on this host.
  - The positive rows never ran: this seat refused `unshare`, `chattr` and
    a moved TMPDIR. Their proof is **PENDING**.
- **F4.** A table's *changed* refusal now says "entry, target or bytes".
  Three assertions take it (K6).
- **F5.** `Held` documents only the handles it retains.
- **Tests.** Two new: `a_hard_link_cannot_stand_in_for_a_hidden_reference`
  and `a_consumed_file_has_one_name_in_its_layer`. Changed: the three F4
  assertions and the alias test's gate.
- **Proof.**
  - Baseline on `c908db9d`: 5 reds.
  - Mutations K1–K7 and T1 each compile, fail their assertion, and are
    restored (231 passed unmutated).
- **Behaviour note.** A hard link inside a layer to a consumed file is now
  refused (design D7). `bundles/self` (`45dc1c7e…`) and `bundles/verify`
  (`f7cbd4bb…`) are unchanged.
- **Standing-admission lines and fixture migrations:** none.
- **Gates.**
  - fmt and clippy are clean.
  - Runtime: 25 results, lib 595.
  - Workspace, with and without all features: 77 results each, 0 failed.
  - Strict OpenSpec (18) and `git diff --check` are clean.
- **Pending.**
  - The F2 alias proof, and with it 16.1.
  - macOS: the FFI link names, the `d_name` offset, `__error`, and the
    socket wording.
  - 32-bit Linux.
  - Exact coverage outside the box.
  - Remote CI and the council.

Unit 16-fix-d (2026-09-28, run `0065-rebuild-unit-16-see-the-uni-1d9ed775`,
based on `95d4ff19`; evidence.md, "Unit 16-fix-d"). It answers the chief's
F1–F4 on unit 16-fix-c's SECURITY-HOLD by removing the alias surface, not
patching it; F5 is not acted on. Production: `bundle.rs` and
`bundle/compose.rs` only. **16.1 closes, and 16.2 and 16.3 stay closed, on
this visit's evidence; the pending items below stay pending.**

*Corrected by the 16-fix-d return (council on `e5c58a8c`, F1–F5):* 16.1 did
not close: its case-insensitive refusal row is unobserved, so it is open
again. "Contained link bindings still compile" held for a link as the last
step only. A path through a contained linked directory (`also -> roles`)
was refused as a second name for its own entry. The F3 regression below
bound an absent walked key, not a failing lookup: every lookup in it
succeeded, and M3 removed the walk's guard, not error propagation. The F2
inherited row, the two-other-names row and the moved-away-and-back row
had no independently observed red or caught mutation. The return repairs
each of these; see its note below.

- **F1, F3: the alias search is deleted.** `listed`, `listing`,
  `many_names` and the `fdopendir`/`readdir`/`closedir`/`errno` FFI are
  gone. Each name is bound exactly as it was looked up in the held
  directory; no listing, search or inode match substitutes another entry.
  A name that does not resolve, or resolves to another file when observed
  again, is refused. There are no candidate lookups left to swallow.
- **F1, F2: one name per consumed file, checked at the walk.**
  `walk_files` refuses a consumed key it lists no entry for; a name under
  no consumed key that is a consumed file ("another name"); and a consumed
  file met under a second name, consumed or not ("two names"). A link is
  its own entry, so contained link bindings still compile.
  `Standing::Unlisted` could no longer be reached and is removed.
- **F4.** The alias-positive test is now a refusal test,
  `a_spelling_its_directory_does_not_list_is_refused`. The pending
  alias-positive proof is **retired, not claimed**, because the surface
  was removed under the refusal ruling.
- **Regressions.** Three new tests, each red on `95d4ff19`, each caught by
  a compiling mutation, each passing again once restored:
  - `a_name_hidden_at_every_observation_binds_no_other_entry` (F1). On
    `95d4ff19` it compiled to `32393b07…`. Mutation M1 drops the
    another-name refusal.
  - `two_consumed_names_for_one_file_are_refused` (F2), standalone and
    inherited. On `95d4ff19` it compiled. Mutation M2 drops the two-names
    refusal.
  - `a_name_that_no_longer_resolves_is_refused_not_found_elsewhere` (F3).
    On `95d4ff19` it gave the *changed* refusal. Mutation M3 drops the
    unlisted-key refusal.
- **Changed assertions, all in `bundle/compose_tests.rs`.**
  - The alias test's rows.
  - `a_consumed_file_has_one_name_in_its_layer`. Two other names are now
    *replaced*. A name moved away and back between the two observations
    now compiles to the undisturbed identity.
  - 16-fix-c's `a_hard_link_cannot_stand_in_for_a_hidden_reference` is
    replaced by the F1 regression.
- **Standing-admission lines and fixture migrations:** none.
- **Gates:** fmt and clippy are clean. The runtime suite gave 25 results
  (lib 597); the workspace suite gave 77 results, 0 failed. `bundles/self`
  (`45dc1c7e…`) and `bundles/verify` (`f7cbd4bb…`) are unchanged. Strict
  OpenSpec (18) and `git diff --check` are clean.
- **Pending.**
  - The case-insensitive rows of the spelling refusal (macOS, or Linux on
    a casefolded directory). This host's fixture root accepts no alias.
  - macOS `openat`/`readlinkat`, the flag values and the socket wording.
  - Exact coverage outside the box.
  - Remote CI and the council.

Unit 16-fix-d, return (2026-09-28, run
`0065-rebuild-unit-16-see-the-uni-1d9ed775`, based on `e5c58a8c`;
evidence.md, "Unit 16-fix-d, return"). It answers the council's F1–F5 on
`e5c58a8c`; F6 is not acted on. Production: `bundle.rs` and
`bundle/compose.rs` only. **16.2 and 16.3 stay closed on this visit's
evidence. 16.1 stays open: its case-insensitive refusal row is pending.**

- **F1, a linked directory is not a second name.** The walk now holds each
  consumed file to one *entry*: the `(dev, ino)` of the directory holding
  it, and its name there. A path through a contained linked directory
  lists the target's own entry again, so it is accepted, consumed or not.
  Two hard links are two entries and are still refused.
  - New test `a_path_through_a_contained_linked_directory_is_the_entry_it_lists`,
    standalone and inherited. It checks the pinned digest under both
    paths, and that the charter's bytes alone move the leaf's and the
    ancestor's identity. A hard-link control through the same link is
    still refused.
  - Red on `e5c58a8c`, both rows. Caught by M1 and M2.
  - `Binding.key` and the `walk_files` documentation now say this.
- **F2, a failing lookup.** New test
  `a_lookup_that_fails_is_refused_where_it_fails`. The exact lookup fails
  while `tables/h.json` holds the file, standalone and inherited, at three
  points:
  - at the read's second observation, with `NotFound`: refused as
    *replaced*;
  - at the same point, with `NotADirectory`: refused as *replaced*;
  - at the walk's verification: refused as *changed*.
  M4 swallows the error at the second observation, and M5 at the walk's
  verification. Each is caught on exactly its rows. **No baseline red
  exists for this test.** It passes on `e5c58a8c` and on `95d4ff19`: an
  exact lookup's error already refused on both. The swallowed errors were
  candidate lookups inside the deleted search, and no lookup reaches them
  now. The old F3 test keeps its assertion and is documented as the
  absent-key case it is.
- **F3, proof records.** Each row below was observed on `95d4ff19` and
  under a mutation:
  - `two_consumed_names_for_one_file_are_refused` now compares standalone
    and inherited together. Its inherited half is red on `95d4ff19`
    (`compiled to 5cdd9efb…`).
  - The two-other-names row is its own test,
    `a_name_gone_at_its_second_observation_is_not_searched_for`. It is
    caught by M4.
  - The moved-away-and-back row is its own test,
    `an_entry_moved_away_and_back_is_the_name_it_was_read_by`. It is red on
    `95d4ff19` (*replaced*) and caught by M6, a path look after the lookup.
- **F4, context.** Each consumed key now carries who read it: the file and
  the document, the table, or the seat and its role. The walk's
  unlisted-key refusal opens with that, as the input's own refusal does.
  - Changed assertions: the spelling rows, the absent-key row, and a
    removed table link and a removed charter link, standalone and
    inherited.
  - Red on `e5c58a8c`. Caught by M3.
- **F5.** 16.1 is open again, as above.
- **Standing-admission lines and fixture migrations:** none.
- **Gates.**
  - fmt and clippy are clean.
  - Runtime: 25 results, lib 601.
  - Workspace, all features: 77 results, 0 failed.
  - `bundles/self` (`45dc1c7e…`) and `bundles/verify` (`f7cbd4bb…`) are
    unchanged.
  - Strict OpenSpec (18) and `git diff --check` are clean.
- **Pending.**
  - The case-insensitive rows of the spelling refusal, and with them 16.1.
  - macOS `openat`/`readlinkat` and the flag values.
  - Exact coverage outside the box.
  - Remote CI and the council.

*Corrected by the second 16-fix-d return (council on `39f6fddd`, F1–F4):*

- "Each row below was observed on `95d4ff19` and under a mutation" did not
  hold for the inherited half of `two_consumed_names_…`. It was red on
  `95d4ff19`, but no recorded mutation failed that test: M1–M6 all show it
  passing.
- F2 (the chief's F3 on 16-fix-c) was not closed. `a_lookup_that_fails_…`
  has no baseline red. The old case, one candidate lookup succeeding and
  another failing, had neither a reproduction nor a red.
- The walk skipped `realms.json`, `dialects/` and `capabilities/` before its
  one-entry check, so a hard link to a consumed file there was never
  refused.

Unit 16-fix-d, second return (2026-09-28, run
`0065-rebuild-unit-16-see-the-uni-1d9ed775`, based on `39f6fddd`;
evidence.md, "Unit 16-fix-d, second return"). It answers the council's F1–F4
on `39f6fddd`; F5 is not acted on. Production: `bundle.rs` only. Tests:
`bundle/compose_tests.rs`. **16.2 and 16.3 stay closed on this visit's
evidence. 16.1 stays open: its case-insensitive refusal row is still
pending.**

- **F1, skipped trees.** Whenever a layer consumed anything, the walk now
  also searches the trees under the names it skips, but it pins nothing
  there. Each file there is held to the same one-entry rule
  (`one_entry`, shared with the pinned walk). A layer that consumed nothing
  never reads them.
  - New test `a_skipped_tree_holds_no_second_name_for_a_consumed_file`.
    Hard links at `capabilities/second.json`, `dialects/second.json` and
    `realms.json` are each refused as another name for `policy.json`,
    standalone and inherited, in one comparison. As a control, a copy at
    the same name leaves both identities unchanged.
  - Red on `39f6fddd` and `95d4ff19`: all six cells compiled to the
    unlinked identities. Caught by M1 (no skipped tree searched: all rows)
    and M2 (skipped directories not descended: the two directory rows,
    while `realms.json` still refused).
- **F2, the old failing candidate.** New test
  `a_failing_lookup_beside_another_name_binds_neither`. `policy.json` is
  hidden behind `h.json` and `h2.json` at each observation and put back
  between them. `h2.json` is removed when anything looks it up.
  - On `95d4ff19` plus a test-only hook in `listed`, called before each
    candidate's lookup, the lookup of `h2.json` failed and was discarded.
    `h.json` was then bound, and the compile sealed `compiled to 3f843a52…`.
    Without the hook, `95d4ff19` refused two other names, because the lookup
    never failed.
  - On this head no entry is looked up in the name's place. The walk
    refuses the unlisted key. Caught by M4 (that refusal off: `h.json` is
    then refused as another name).
- **F3, the inherited half.** M3 disables the two-names refusal.
  `two_consumed_names_…` then fails with both halves in one assertion:
  `compiled to 2662257a…` standalone and `compiled to 5cdd9efb…` inherited.
- **F4.** Still pending. This seat was refused `chattr +F` as well as
  `unshare`, so no casefolded directory was available.
- **Helper change:** `hidden_at_every_observation` takes `failing`. Both
  existing callers pass `false`, and their assertions are unchanged.
- **Standing-admission lines and fixture migrations:** none.
- **Gates.**
  - fmt and clippy are clean.
  - Workspace, all features: 77 results, all ok. The runtime lib has 603.
  - `bundles/self` (`45dc1c7e…`) and `bundles/verify` (`f7cbd4bb…`) are
    unchanged.
  - Strict OpenSpec (18) and `git diff --check` are clean.
- **Pending.**
  - The case-insensitive rows of the spelling refusal, and with them 16.1.
  - macOS `openat`/`readlinkat` and the flag values.
  - Exact coverage outside the box.
  - Remote CI and the council.

Unit 16-fix-e (2026-09-28, run `0065-rebuild-unit-16-see-the-uni-fd9035f4`,
based on `af773294`; evidence.md, "Unit 16-fix-e"). It finishes unit 16 on
the chief's F1–F5 for `af773294` under the operator's ruling of 2026-09-28.
Production: `bundle.rs` only. Tests: `bundle/compose_tests.rs`. **16.2 and
16.3 stay closed. 16.1 stays open: its case-insensitive refusal row (F4) is
still unobserved and is not claimed.**

- **F1, the refusal names who read it.** A consumed entry the walk has
  listed but can no longer observe is refused naming its consumer (the
  declaring document, the input kind and site, the reference), its entry and
  the io kind, never a bare `bundle io:`. This covers the entry under its
  own key and, for a second name, the target entry it is compared with.
  - New test `a_consumed_entry_the_walk_cannot_observe_names_who_read_it`,
    standalone and inherited. The table is removed at the `Walked` seam for
    `aa.bin`, listed before it; `roles/` is moved aside at the `Walked` seam
    for `zz.md`, a hard link to the charter listed after it.
  - Red on `af773294`: every cell said "bundle io: No such file or directory
    (os error 2)". Caught by M1 (the table rows) and M2 (the charter rows).
- **F2, every branch exercised.** New test
  `a_root_rewalk_passes_over_what_the_walk_skips`: `layer_drift` over a
  leaf's own directory, which consumed nothing, with a realm map, a dialect,
  a capability definition and an unlistable dialect directory written after
  the compile, names no drift; a new pinned file is `added: notes.md`.
  Green on `af773294` by design (the branch was already there, unexercised).
  Caught by M5 (the branch off: the unlistable directory is then refused).
  The crate-scoped branch coverage is recorded in evidence.md.
- **F3, skipped trees are not followed out.** The search of a skipped tree
  now asks each entry's own type: a linked directory there is one entry,
  never descended. A directory the walk cannot list, skipped or not, is
  refused naming it (`'./dialects/private'`) and the io kind.
  - New test `a_skipped_tree_is_searched_without_following_a_link`. Links
    out of the layer (to a tree with an unlistable directory) and back to
    its root leave both identities unchanged, and an unlistable
    `dialects/private` is refused naming it, standalone and inherited.
  - Red on `af773294`: every cell said "bundle io: Permission denied (os
    error 13)". Caught by M3 (links followed) and M4 (the listing refusal
    off).
- **F5.** The 16-fix-d follow-up claimed a link cycle under a skipped tree
  refuses. It did not; the correction is in evidence.md.
- **F4.** Still pending, and 16.1 with it.
- **Standing-admission lines and fixture migrations:** none.
- **Gates.** fmt and clippy are clean. Workspace, all features: 77 results,
  all ok; the runtime lib has 606. `bundles/self` (`45dc1c7e…`) and
  `bundles/verify` (`f7cbd4bb…`) are unchanged. Strict OpenSpec (18) and
  `git diff --check` are clean.
- **Pending.** F4 and with it 16.1; macOS; exact coverage outside the box;
  remote CI and the council.

Unit 16-fix-e, return (2026-09-28, run `0065-rebuild-unit-16-see-the-uni-fd9035f4`,
based on `e132c3a5`; evidence.md, "Unit 16-fix-e, return"). It answers the
chief's F1 and F3 on `e132c3a5`. Production: `bundle.rs` only. Tests:
`bundle/compose_tests.rs`. **16.2 and 16.3 stay closed. 16.1 stays open: F4,
its case-insensitive refusal row, is still unobserved and is not claimed.**

- **F1, collection names who read it.** A consumed entry is walked as the one
  entry it was read by and never descended, whatever stands there now, so its
  own check names its consumer. A directory the walk cannot list that holds a
  consumed entry now opens its refusal with who read that entry.
  - New test `a_consumed_entry_replaced_before_the_walk_lists_it_names_who_read_it`.
    At the new `Listing` seam for the base layer, `policy.json`, or in a
    second compile `roles/`, is replaced by a mode-000 directory. Standalone
    and inherited.
  - Red on `e132c3a5` plus the seams: every cell said `bundle directory
    './policy.json'` or `'./roles' cannot be listed (permission denied)`, with
    no consumer. Caught by M1 (a consumed entry descended again: the table
    rows) and M2 (the consumer dropped from `unlisted`: the `roles` rows).
- **F3, descent is bound to the no-follow observation.** A directory in a
  skipped tree is listed only while its entry is still the `(dev, ino)` its
  parent's listing found, asked without following a link. This is checked
  before the listing and again after it. A replacement is refused naming the
  directory before any name listed through it is walked.
  - New test `a_skipped_directory_replaced_by_a_link_is_not_followed`.
    `dialects` is replaced by a link out of the layer (to a tree holding a
    mode-000 directory) at the `Listing` seam, and at the new
    `DirectoryChecked` seam. Each is refused naming `'./dialects'`, standalone
    and inherited. The link left standing compiles to the ordinary
    directory's identity.
  - Red on `e132c3a5` plus the seams: every swapped cell refused the outside
    tree's `'./dialects/locked'`. Caught by M3 (no pre-check: the `before`
    rows say `while`), M4 (no post-check: the `while` rows reach
    `'./dialects/locked'`) and M5 (skipped entries classified through links:
    this test and `a_skipped_tree_is_searched_without_following_a_link`).
- **F2.** Still exercised. In the clean crate-scoped diagnostic,
  `DA:6705,6` and `BRDA:6704,1,2,6`, and there is no zero `DA`/`BRDA` in
  `bundle.rs` 6600–6999.
- **F4.** Still pending, and 16.1 with it.
- **Standing-admission lines and fixture migrations:** none.
- **Gates.** fmt and clippy are clean. Workspace, all features: 77 results,
  all ok; the runtime lib has 608. `bundles/self` (`45dc1c7e…`) and
  `bundles/verify` (`f7cbd4bb…`) are unchanged. Strict OpenSpec (18) and
  `git diff --check` are clean.
- **Pending.** F4 and with it 16.1; macOS; exact coverage outside the box;
  remote CI and the council.

Unit 16-fix-e, second return (2026-09-28, run
`0065-rebuild-unit-16-see-the-uni-fd9035f4`, based on `0fcec52f`; evidence.md,
"Unit 16-fix-e, second return"). It answers the chief's F1–F3 on `0fcec52f`.
Production: `bundle.rs` only. Tests: `bundle/compose_tests.rs`. **16.2 and
16.3 stay closed. 16.1 stays open: the earlier F4, its case-insensitive
refusal row, is still unobserved and is not claimed.**

- **F2, the listing is bound to a handle.** Every directory is now listed
  through a handle (`fdopendir`/`readdir`), never through `read_dir(path)`.
  A directory in a skipped tree is opened through its parent's handle with
  `O_DIRECTORY | O_NOFOLLOW`. It is listed only if that handle holds the
  `(dev, ino)` its parent's no-follow lookup found, and only while its path
  still stands as that directory. A link put in its place is never opened,
  so the outside directory is never listed.
  - `a_skipped_directory_replaced_by_a_link_is_not_followed` now asserts
    that inotify sees nothing open or read the outside directory. It gains
    a row with a new directory put in place.
  - Red on `0fcec52f`: the `DirectoryChecked` rows saw `IN_OPEN` and
    `IN_ACCESS` on it. Caught by M2 and M3 (a path open or listing
    restored), M4 (no after-check) and M5 (no identity check).
- **F1 and F3, an entry that cannot be observed is named as itself.** A
  skipped-tree entry whose no-follow lookup fails is refused as `bundle
  entry './<key>', in a tree the walk skips, cannot be observed (<kind>)`.
  It is never named as its directory's listing.
  - New test `a_skipped_entry_the_walk_cannot_observe_is_refused_naming_it`,
    at the new `Skipped` seam, for `realms.json` and `dialects/gone.json`,
    standalone and inherited.
  - Red on `0fcec52f` plus the seam: every cell said "bundle directory ...
    cannot be listed (entity not found)". Caught by M1.
  - M6 and M7 bind the unlistable/replaced split and the no-follow lookup
    through the existing `a_skipped_tree_is_searched_without_following_a_link`.
- **Coverage diagnostic.** The walk's `FNDA:0` closure is gone. No zero
  `DA`/`BRDA`/`FNDA` falls in the changed ranges. The other zero records are
  the same as before this visit.
- **Evidence.** The return's F3 paragraph, which claimed the replacement was
  never listed, now carries a correction with the observed listing.
- **Standing-admission lines and fixture migrations:** none.
- **Gates.** fmt and clippy are clean. Workspace, all features: 77 results,
  all ok; the runtime lib has 609. `bundles/self` (`45dc1c7e…`) and
  `bundles/verify` (`f7cbd4bb…`) are unchanged. Strict OpenSpec (18) and
  `git diff --check` are clean.
- **Pending.** The earlier F4 and with it 16.1; macOS (`fdopendir`/`readdir`
  names and offsets, `O_DIRECTORY`); exact coverage outside the box; remote
  CI and the council.

## 17. Unit 17 — Select charter owner and source at compile

- [x] 17.1 Unit 17 binds selected charter owner/reference/target/digest, no longest-prefix guess. Verify external/nested/overlapping owners and all site/candidate paths. Requirements: [Library charter pins are enforced at consumption][MPL], [A new manifest version records capabilities per executable seat][MP1]. Reopened/remaining: operator ruling 3. (previous 6.2)

Unit 17 (2026-09-29, run `0065-rebuild-unit-17-see-the-uni-d263ffa3`,
based on `7f1ab492`; evidence.md, "Unit 17"). Production: `bundle.rs`,
`agents.rs`, `agents/load.rs`. Tests: `bundle/agent_tests.rs`,
`agents/tests.rs`. **17.1 closes.**

- Every site with a charter carries `CharterPin {owner, reference, path,
  target, digest}`. The owner is `Layer {dir, key}` for an inline role,
  bound from unit 16's handle read, or `Library {agent, root}` for an
  agent's charter, from the library's new `CharterSource`.
  `Bundle::charters` holds every binding of a path.
- `charter_text` looks up the exact told path and checks every binding. It
  no longer takes the longest layer root, folds `..` onto a pin, or falls
  back to a neighbouring pin. Its refusal wording is unchanged.
- Proved at every site form (seat, member, step, case, default; leaf and
  inherited), including the fallback candidate, external, nested and
  overlapping owners. A `..` spelling is refused, and a recipe reference out
  to an external library refuses at compile.
- Baseline on `7f1ab492`, by scratch probe: `..` admitted, the nested
  library's charter claimed by `layer 'fixture'`, no inline binding.
  Mutations M1–M7 each fail named rows.
- Standing-admission line: `bundle/tests.rs:853–854`, the forced `sites`
  argument. Fixture migrations: none.
- Gates: fmt and clippy clean; runtime lib 612; workspace 77 results, all
  ok; `bundles/self`/`verify` digests unchanged; strict OpenSpec (18) and
  `git diff --check` clean.
- **Pending.** Per-site consumption with the target recheck (unit 18);
  start and resume (unit 19); macOS; exact coverage outside the box; remote
  CI and the council.

## 18. Unit 18 — Consume the bound charter and render its buffer

- [x] 18.1 Unit 18 checks owner/target/bytes at dispatch and sends verified buffer. Verify changed/equal-byte retarget refusal without recompilation and no provider work. Requirements: [Library charter pins are enforced at consumption][MPL], [Active instructions and policy cannot escape bundle identity][MPI], [Capability responses are data and confer no authority][MP4]. Reopened/remaining: operator ruling 3. (previous 6.3)

- [x] 18.2 Unit 18 verifies selected binding/holding/prompt across every serving shape; authored merges cannot replace text and rendering cannot reread. Requirements: [Library charter pins are enforced at consumption][MPL], [Prompt capability statements reflect the serving seat's pinned holdings][MP3], [Capability responses are data and confer no authority][MP4]. Reopened/remaining: operator ruling 3. (previous 6.5)

- [x] 18.3 Unit 18 verifies full rendered holdings/drops/DATA statements match selected verified charter across paths, no authored replacement/reread. Requirements: [Prompt capability statements reflect the serving seat's pinned holdings][MP3], [Capability responses are data and confer no authority][MP4], [Library charter pins are enforced at consumption][MPL]. Reopened/remaining: operator ruling 2–4. (previous 8.1)

Unit 18 (2026-09-29, run `0065-rebuild-unit-18-see-the-uni-b50b3d04`,
based on `4cf26494`; evidence.md, "Unit 18"). Production: `engine.rs`,
`bundle.rs`, `adapters.rs`. Tests: `engine/boundary_tests.rs`,
`engine/capability_tests.rs`, `adapters/tests.rs`. **18.1, 18.2 and 18.3
close.**

- `SiteSpawn::charter` carries the binding unit 17 selected for the site
  that `compose_at` composes, whatever its shape. The dispatch door checks
  that binding alone. The input's `role_path` must be its path
  (`replaced`). Its reference is re-read through unit 16's `bound_input`
  from the owner's own root (layer or library). The buffer must match the
  digest (`changed`) and the compiled target (`retargeted`), and a failed
  read is named `missing`, `nonregular`, `outward`, `replaced`,
  `unreadable` or `unbound`. The text of that read is written after every
  merge, and an input arriving with `role_text` is refused.
- An engine launch (one carrying `native_controls`) never rereads
  `role_path` when rendering. The driver refuses one without `role_text`
  before rendering, with no invocation.
- Proved without recompiling, over external libraries, standalone and
  inherited layers, equal-byte in-tree and outward retargets, FIFO,
  missing, restored, and controlled replacements at `Opened`, `Read` and
  `Verified`. Also proved across an ordinary seat, a selected default,
  a sequence with a fallback step, a nested panel (a primary member and an
  unresolved one) and a top-level panel. Each site gets its own charter
  text and full holdings, drops, native and DATA statements, and every
  prompt re-renders byte-identical after the files change. (Corrected by
  unit 18-fix, council F2: every served fixture here held nothing and
  asked for nothing, so no granted holding, unmet want, subtraction or
  provider drop was observed at unit 18; they are observed below.)
- Baseline probes B1 and B2 and mutations M1–M9 each fail named rows.
- Standing-admission lines: `engine/tests.rs:4914–4920`
  (`compiled_triage_engine` keeps each site's `charter` when it clears the
  facts for the fake driver). No assertion changed. Fixture migrations:
  none.
- Gates: fmt and clippy clean; runtime lib 614 and protocol lib 543;
  workspace 77 results, all ok; `bundles/self`/`verify` digests unchanged;
  strict OpenSpec (18) and `git diff --check` clean.
- **Follow-up.** The relative-`role_path` fallback for unbound fixture
  sites remains.
- **Pending.** Start and resume (unit 19); macOS; exact coverage outside
  the box; remote CI and the council.

Unit 18-fix (2026-09-29, run `0065-rebuild-unit-18-see-the-uni-b39bb56a`,
based on `56873c18`; evidence.md, "Unit 18-fix"). Answers the unit 18
council's SECURITY-HOLD (F1–F4). Production: `bundle.rs` only. Tests:
`engine/boundary_tests.rs`, `engine/capability_tests.rs`. **18.1, 18.2 and
18.3 stay closed, now on what is observed below.** (Corrected by unit
18-fix-b: F1 was NOT closed here. The pin kept a path, not the compile's
binding, so an equal-byte file at the same path, or an equal-byte
`roles`/`charters` directory, was accepted; the owner identity came from a
separate walk (F2); and an unobservable ancestor compiled an ownerless
bundle (F3). They are closed at unit 18-fix-b below.)

- **F1.** The dispatch door no longer opens a charter's owner by its stored
  path. Both of unit 16's resolutions in the door's read (`owner_read`, one
  resolver: `bound_input`'s body, given its directory) reach the owner from
  `/` a name at a time without following a link. The owner and every
  directory above it must be the ones the compile bound (`Bundle::
  charter_owners`, recorded before the seal). A link where a directory
  stood, or a replacement directory, refuses as `replaced` before any
  driver starts. A gone owner is `missing`. The compiled target and digest
  checks are unchanged.
- **F2.** Every served fixture now resolves its own asks against a realm
  that grants `web-search` (Codex) and `web-fetch` (Claude). Observed, each
  with the selected charter: a held grant (single seat, selected default,
  panel member `a`); an ungranted want (default, `final`, `b`); a
  subtraction (fallback `draft`, `a`); a provider-incompatible drop
  (`draft` on DSH, `final` on Codex); Codex's native OFF (`final`, `b`);
  DSH's unmeasured statement (`draft`). Nonempty-restriction transport
  stays deferred (D11).
- **F3/F4.** Refusal kinds are a closed `FaultKind`, set where the resolver
  refuses and never read back from its prose. Each arm is reached by an
  exact refusal with a caught mutation: `unreadable` (a failed read),
  `outward` (a `..` link), `unbound` (a library `..` charter; an owner not
  recorded), and a foreign owner (`bundle '…'` / `unpinned`). The two
  missing-owner fallbacks are now one (`owned`).
- Baseline reds B1–B2 on `56873c18`; mutations M1–M11, MS1–MS2, MD1–MD3
  each fail named rows.
- Standing-admission lines: `engine/resume_tests.rs:291`,
  `engine/tests.rs:102` and `brokkr-cli/src/recipes/tests.rs:70`, one
  `charter_owners: Default::default(),` each (a new `Bundle` field every
  literal must name). No assertion changed. Fixture migrations: none.
- Gates: fmt and clippy clean; workspace 77 results, all ok (runtime lib
  616, protocol lib 543, cli lib 482); `bundles/self`/`verify` digests
  unchanged; strict OpenSpec (18) and `git diff --check` clean.
- **Follow-up.** The compile still opens a layer by its canonical path and
  the library still loads its charter by path (units 16–17). A library
  `..` charter compiles and then refuses at dispatch.
- **Pending.** Start and resume (unit 19); macOS; exact coverage outside
  the box; remote CI and the council.

Unit 18-fix-b (2026-09-29, run `0065-rebuild-unit-18-see-the-uni-e580ec40`,
based on `5ae91c6f`; evidence.md, "Unit 18-fix-b"). Answers the unit
18-fix council's SECURITY-HOLD (F1–F3). Production: `bundle.rs` only.
Tests: `engine/boundary_tests.rs`, `engine/capability_tests.rs`. **18.1,
18.2 and 18.3 stay closed, on what is observed below.**

- **F1.** `CharterPin` carries the compile read's whole `Binding` (both
  keys and the file read) and the owner identity, and has no target path.
  It is built only by `CharterPin::of` from a bound read. Both the inline
  role and, new, the agent's library charter are read by `owned_input` at
  compile. The dispatch door re-reads through the same resolver and
  compares the whole binding. An equal-byte file at the same path, or an
  equal-byte `roles`/`charters` directory, refuses as `replaced` for both
  owner kinds before any driver starts. (Corrected by the 18-fix-b return:
  the directory rows copied the charter too, so they proved only the
  file's identity. A replaced directory holding the ORIGINAL charter was
  accepted; the binding held no directory below the owner. Closed there.)
- **F2.** The owner identity is taken by the read's own opener, compared
  by its second resolution, by the seal (`Held::intact`) and by the door.
  The separate recording walk and `Bundle::charter_owners` are gone. A
  deterministic swap at the owner's observation (the `Owning`/`Owned`
  seam), during the read or at the first observation after it, refuses the
  compile. (Corrected by the 18-fix-b return: the seal checked only the
  layers' charter reads. The library read was dropped after its pin was
  made, so its owner was never checked at the seal. Closed there.)
- **F3.** An owner directory the compile cannot open refuses AT COMPILE,
  naming the directory and its cause (`… '{dir}' cannot be opened
  (permission denied) …`). The 18-fix evidence's `unreadable` prediction
  was wrong (the compile succeeded, ownerless) and is corrected there.
- A library charter named through `..` now refuses at compile. It used to
  compile and then refuse at dispatch.
- Baseline reds on `5ae91c6f`: B1 (F1, all four rows accepted), B3 (F3,
  the compile succeeded). B2 (F2) ran on `5ae91c6f` plus only the inert
  seam: all four rows compiled, and a probe dispatched the twin owner.
  Mutations M1–M10 each fail the named rows; all restored.
- Changed own-file tests: `every_way…` (an unconstructible
  unrecorded-owner row is replaced by a skipped-tree `unbound` row, bound
  by M9; the library `..` row expects the compile refusal, bound by M10).
  Two tests' restores now put back the compiled file rather than rewriting
  equal bytes; no assertion changed.
- Standing-admission lines: `charter_owners: Default::default(),` removed
  from `engine/resume_tests.rs:291`, `engine/tests.rs:102` and
  `brokkr-cli/src/recipes/tests.rs:70` (the field is gone);
  `bundle/agent_tests.rs:6317–6318` and `:6333–6334` name `binding` and
  `directory` for `target` in `layer_pin`/`library_pin`. No assertion was
  added or removed. Fixture migrations: none.
- Gates: fmt and clippy clean; workspace 77 results, all ok (runtime lib
  619, protocol lib 543, cli lib 482); `bundles/self`/`verify` digests
  unchanged; strict OpenSpec (18) and `git diff --check` clean. A lib-only
  branch-coverage diagnostic found no unhit line or branch in the changed
  ranges (not the gate).
- **Follow-up.** `CharterSource::target` (`agents.rs`) is no longer read by
  the pin. The library still loads its charter by canonical path before
  the pin's bound read, which now re-reads it and requires the same bytes.
- **Pending.** Start and resume (unit 19); macOS; exact coverage outside
  the box; remote CI and the council.

Unit 18-fix-b return (2026-09-29, run `0065-rebuild-unit-18-see-the-uni-e580ec40`,
second implement visit, based on `b4cb335b`; evidence.md, "Unit 18-fix-b
return"). Answers the 18-fix-b council's residual (F1–F3; F4 was a run
defect). Production: `bundle.rs` only. Tests: `engine/boundary_tests.rs`.
**18.1, 18.2 and 18.3 stay closed, on what is observed below.**

- **F1.** `Binding` now carries every step its resolution took (each
  directory's and the file's `(dev, ino)`, each link's text). `Held`'s
  separate `steps` are gone, so the read's second resolution, the seal and
  the door all compare one value. A replaced `roles`/`charters` directory
  holding the ORIGINAL charter (same inode) refuses as `replaced` for both
  owner kinds before any driver starts.
- **F2.** The library charter's bound read is held (`LibraryRead`) until
  the seal. Before any identity is sealed, `Held::intact` checks it: its
  owner, every entry on its way and its bytes. A library directory
  replaced right after the read was verified (every entry moved into the
  new one) refuses the compile.
- **F3.** An owner the compile cannot reach carries its own remedy: make
  the directory readable, or compile elsewhere. It no longer says to move
  the charter to `roles/`.
- Baseline reds on `b4cb335b` (only the test file changed): F1 both rows
  `Ok`; F2 `Ok("recipe")`; F3 the old remedy. Mutations R1–R3 each fail
  their test; all restored.
- Changed own-file assertion: the F3 test's expected text (new remedy).
  Standing-admission lines: none (`Binding::expected`, in `bundle.rs`,
  now derives the steps; `agent_tests.rs` is unchanged). Fixture
  migrations: none.
- Gates: fmt and clippy clean; workspace 77 results, all ok (runtime lib
  621, protocol lib 543, cli lib 482); `bundles/self`/`verify` digests
  unchanged; strict OpenSpec (18) and `git diff --check` clean.
- **Follow-up.** The library refusal for an unreachable owner keeps its
  generic sentence (it names no wrong remedy).
- **Pending.** Start and resume (unit 19); macOS; exact coverage outside
  the box; remote CI and the council.

## 19. Unit 19 — Enforce charter integrity at start and pinned resume

- [x] 19.1 Unit 19 proves owner-bound start/resume checks for changed/excluded/retargeted/missing/restored charters. Verify retained map-edit/identity/unmapped-root behavior. Requirements: [Library charter pins are enforced at consumption][MPL], [Active instructions and policy cannot escape bundle identity][MPI], [Realm context is resolved before capability authorization][RG5]. Reopened/remaining: operator ruling 3. (previous 6.4)

Unit 19 (2026-09-29, run `0065-rebuild-unit-19-see-the-uni-6f959130`, based
on `58a268e6`; evidence.md, "Unit 19"). Production: `engine.rs`,
`bundle.rs`. Tests: `engine/boundary_tests.rs`, `engine/capability_tests.rs`,
`brokkr-cli/tests/capability_verbs.rs`. **19.1 closes on what is observed
below.**

- `bundle::charters_intact` checks every compiled binding (layer and
  library) through `pinned_charter`, the dispatch door's own owner-bound
  read: owner, every step of the read's binding, and bytes.
  `Engine::start_in_world`, `start_with_dispatch` and `resume` call it
  before `create_run` or the resumed engine, and refuse with
  `EngineError::CharterMoved`, naming the owner and bounded cause.
- Over an already compiled bundle (no recompile, no dispatch), both
  owners refuse a new start, a dispatch-bound start and a pinned resume
  for changed, unbound (excluded `capabilities/`), retargeted (equal-byte
  twin in the owner), outward (equal-byte copy outside) and missing
  charters. Nothing is written, and the restored original file resumes the
  same run. In a mapped and an unmapped context, a changed charter refuses
  start and resume. Restored, each run resumes into its own pinned
  context: realm `private`, or no world.
- Baseline reds with production reverted: both runtime tests fail at their
  first start assertion. Mutations M1–M5 (each door removed; bytes-only
  check; library pin skipped) and M6 (resume's manifest refusal, for the
  CLI test) each fail the named assertion. All are restored.
- Retained and passing: the map-edit, identity and unmapped-root tests
  (named in evidence.md).
- Standing-admission lines: none. Fixture migrations: none.
- Gates: fmt and clippy clean. Workspace: 77 results, all ok (runtime lib
  623, protocol lib 543, cli lib 482). `bundles/self` and `verify` digests
  are unchanged. Strict OpenSpec (18) and `git diff --check` are clean.
- **Pending.** macOS; exact coverage outside the box; remote CI and the
  council.

**Reopened 2026-09-29 by the council (run `…-6f959130`, review at
`c66be187`, F1 medium and F2 low). The return visit is OVERSIZED; nothing
in production moved.** F1: the `resume` verb recompiles before
`Engine::resume`. A charter relinked to an equal-byte twin inside its owner,
or replaced by a new file of equal bytes, compiles to the pinned manifest,
and the recompile binds it again. The run's own binding survives nowhere, so
the resume passes. F2: `capability_verbs.rs` built its fixture under the raw
`TempDir` path. The return visit built and proved a fix, saved as
`.forge/unit-19-fix/unit-19-f1-oversized.patch` (sha256 `f9cbac78…`; it
applies to `c66be187`). Evidence.md, "Unit 19 — review return", has the
details. It does these things:

- Each start door records the bindings it checked in `run/started` as a
  `charters` list: owner, reference, key, target and a binding digest.
- `resume` holds the recompiled bundle's bindings to that record.
- A run that recorded no bindings refuses (`unrecorded`).
- It fixes F2.

Two test files outside the unit need non-admissible changes:

- `brokkr-cli/tests/witness_journal.rs`: the golden `run/started` key list
  gains `charters`.
- `brokkr-cli/src/tests.rs`: `resume_concludes_an_accepted_but_unconcluded_operator_stop_and_exits_three`
  resumes the frozen old-engine journal
  `fixtures/journals/tui-graph-the-selection-box-gets-80f98deb.ndjson`, which
  records no bindings, and is refused `unrecorded`.

The second needs an operator ruling first: do runs with no recorded bindings
refuse (no grandfathering), or keep the intact-only check?

**Re-fired 2026-09-29 after triage re-ruled `chore`; OVERSIZED again.** No
addendum on unit 19 exists in `operator-ruling-2026-09-23.md`. The saved
patch still applies to `9bed6803`, its hash is unchanged, and both blocking
tests are unchanged. Evidence.md, "Unit 19 — re-fire", has the checks.
Nothing was rebuilt.

**Landed 2026-09-29 as unit 19-fix (run `…-2183fb26`, based on `184ed2f6`;
evidence.md, "Unit 19-fix"). 19.1 closes on what is observed there.** The
operator's ruling of 2026-09-29 is appended verbatim to
`operator-ruling-2026-09-23.md` ("runs without recorded charter bindings;
unit 19 admission"). It rules no grandfathering: an unrecorded run is refused
`unrecorded`. It also admits the two test files.

- The saved patch's sha256 is unchanged (`f9cbac78…`), and
  `git apply --check` passed on `184ed2f6`. It was applied verbatim, with no
  re-derivation.
- Admitted lines:
  - `witness_journal.rs:38-40`: `charters` added to the golden `run/started`
    key list.
  - `src/tests.rs`: the frozen-fixture resume now asserts the exact
    `unrecorded: roles/implementer.md` refusal (`:2714-2720`). The operator
    stop's conclusion is re-proved on a copy that records its bindings
    (`:2722-2729`), through a private helper `stopped_mid_flight_copy`
    (`:157-187`). The fixture file is only read.
- No standing-admission lines. No fixture migrations.
- Baseline reds:
  - With production at `184ed2f6`: witness `:245`, verbs `:485` and
    boundary `:2826` fail.
  - With the patch in and the old test text: witness `:243` and tests.rs
    `:2695` fail.
- Mutations, each caught and then restored:
  - M1 and M1k (start door records nothing, or drops the key).
  - M1d (dispatch start).
  - M2 (resume door: intact-only).
  - M3 (`unrecorded` grandfathered).
  - M4 (no `retargeted` arm).
  - M5 (digest ignores identity, so an equal-byte replacement passes).
  - M6 (a recorded run finds no binding).
- Gates: fmt and clippy clean. The runtime suite passed (25 results, all ok,
  lib 624) and so did the CLI suite (33 results, all ok, lib 482).
  `bundles/self` (`45dc1c7e…`) and `verify` (`f7cbd4bb…`) are unchanged.
  Strict OpenSpec and `git diff --check` are clean.
- Follow-up for the operator: a run with no record, over a bundle that binds
  no charter, still resumes. The check is per binding, as the saved patch
  wrote it. Refusing that case too fails two tests in `engine/tests.rs`,
  which is outside this unit.
- **Pending.** macOS; exact coverage outside the box; the workspace-wide
  `cargo test` (only the two touched crates were run); remote CI and the
  council.

**Review return 2026-09-29 (run `…-2183fb26`, F1, based on `edf364b9`;
evidence.md, "Unit 19-fix — review return (F1)").** The council found the
follow-up above to be a grandfathered run, which ruling point 1 forbids. That
reopened 19.1. It closes again on what is observed there.

- `charters_as_started` now refuses a run whose `run/started` has no
  `charters` record at all, whatever the bundle binds. With bindings, it
  names the first binding, as before. With none, it names the bundle:
  `bundle '<name>'`, `unrecorded: the run started with no charter record`.
- `Engine::resume` now reads the pinned world before the charter record, so a
  tampered pin still refuses as tampering. This kept
  `resume_carries_no_world_where_the_run_had_none_and_refuses_a_broken_pin`
  unedited.
- New test: `boundary_tests.rs`
  `a_run_that_recorded_no_bindings_is_refused_even_by_a_bundle_that_binds_none`.
  - Baseline red at `edf364b9` production: `Ok("legacy")`.
  - M7 (absent record read as `[]`) is caught there.
  - M8 (absent record always names the bundle) is caught at
    `boundary_tests.rs:2956`, which expects `agent 'worker'`.
  - M9 (charter check before the pin check) is caught at
    `engine/tests.rs:4670`.
  - Each was restored and passes.
- Standing-admission line (ruling of 2026-09-25): `engine/tests.rs:2959`
  adds `"charters":[]` to the `run/started` that
  `an_accepted_operator_stop_is_carried_to_a_conclusion_that_cites_it`
  plants. That test swaps in the fixture driver `driver`, and this change
  reaches its resume: without the line, its `Engine::resume(...).unwrap()` at
  `:2973` fails `unrecorded`. The line adds and removes no assertion and
  changes no tested behaviour. It is the record every start now writes.
- No fixture migrations.
- Gates: fmt and clippy clean. Workspace `cargo test --all-features
  --locked` gave 77 results, all ok (runtime lib 625, CLI lib 482).
  `bundles/self` (`45dc1c7e…`) and `verify` (`f7cbd4bb…`) are unchanged.
  Strict OpenSpec (18) and `git diff --check` are clean.
- **Pending.** macOS; exact coverage outside the box; remote CI and the
  council.

**Second review return 2026-09-29 (run `…-2183fb26`, F1, based on
`8b1af7e7`; evidence.md, "Unit 19-fix — second review return (F1)").** The
council found no test for a present record that lacks a binding the bundle
selects (`bundle.rs:5697`, `None => "unrecorded"`). 19.1 stays ticked on what
is observed there; no production file moved.

- `boundary_tests.rs`
  `a_resume_over_a_recompile_is_held_to_the_bindings_the_run_started_over`
  gains the row `f-partial`: the run's own record with agent 'worker'
  dropped, refused exactly `agent 'worker'`, `unrecorded: worker.md`.
  - Mutation M10 (`None => continue`) survived the 16 `boundary_tests::a_`
    tests without the row (`16 passed`); with it, `:2968` (now `:2972`
    after `cargo fmt`) failed `f-partial` with `bundle 'recipe'`,
    `unselected: …`. Restored; `16 passed`.
- No admitted lines and no fixture migrations.
- F3 and F4 [info] are follow-ups, not changed here.
- Gates: fmt, clippy clean; `cargo test -p brokkr-runtime --all-features
  --locked` all ok (lib 625). Strict OpenSpec (18) and `git diff --check`
  clean.
- **Pending.** macOS; exact coverage outside the box; the CLI crate suite
  (untouched, not rerun); remote CI and the council.

## 20. Unit 20 — Audit compiled refusal and serving shapes

- [ ] 20.1 Unit 20 proves all-harness/form/site authored refusals via real compilation. Verify complete causes/no provider work and typed/hands/route positives. Requirements: [Authored provider configuration cannot supply capability authority][RGR], [Known provider commands have a closed argument grammar][RGP], [Reserved hands preserves the existing workspace authority][TD6]. Reopened/remaining: operator ruling 1–2 / R10. (previous 7.3)

**Unit 20, 2026-09-29 (run `0065-rebuild-unit-20-see-the-uni-23e98946`,
based on `4d11cc4b`; evidence.md, "Unit 20").** An audit: no production file
moved, and only `crates/brokkr-runtime/tests/capability_launch.rs` changed.
The protocol suite (`adapters/tests.rs`) and the engine suite
(`engine/capability_tests.rs`) were audited and are unchanged, because
neither compiles a bundle. Before this visit, none of the three suites
asserted a compiled site's selected charter, and DSH had no compiled row.

- **Authored refusals.** New
  `an_authored_capability_option_refuses_every_site_shape_of_every_harness`
  covers Claude, LaneTally, Codex and DSH. Each harness is planted at a work
  seat, a panel member, a sequence step, a select case and default, and an
  inherited seat; Claude and Codex also at a gate. That is 26 exact
  compile refusals naming site, office, canonical option and harness, and
  4 clean compiles. No driver runs.
  - LaneTally and DSH cannot hold a gate (decision 0021 ruling 2).
  - DSH's `--profile` is refused by its closed grammar (decision 0066
    ruling 6).
- **Serving positives.** New
  `every_compiled_site_shape_is_served_its_whole_command_beside_its_selected_charter`
  has 18 Claude/Codex rows over every inline and agent-backed site shape,
  primary and fallback. Each row asserts the site's charter pin, the whole
  cold command and the whole command served when a work site is offered
  its session: an actual `exec resume` for the inline Codex work seat, and
  declined-and-cold elsewhere. Gates are never offered one.
- **LaneTally and DSH positives.** New
  `every_compiled_wrapper_and_dsh_site_is_spawned_by_its_own_driver_beside_its_charter`
  serves 6 LaneTally/DSH rows through the real driver: typed, template,
  DSH, and the DSH route-overlay positive.
- **Hands positives.** The two existing whole-command tests
  (`a_boxed_inline_seats_hands_…` and `an_eligible_rejoin_of_a_compiled_codex_seat_…`)
  now also assert their sites' charter pins.
- **Mutations.** MC1, MC2, MS2–MS5, MR1 and MR2 each fail their rows and
  are restored. MS3, MS5 and MR2 are seen by no other compiled test.
- **Follow-up F-LT.** An inline LaneTally typed allow on the shipped
  adapter compiles, and its launch is then refused by the final check.
  This is fail-closed, and it was not changed here.
- No standing-admission lines and no fixture migrations.
- **Gates.** fmt and clippy are clean. `cargo test -p brokkr-runtime
  --all-features --locked` is all ok (lib 625, `capability_launch` 62).
  Strict OpenSpec (18) and `git diff --check` are clean.
- **Pending.** macOS; the protocol and CLI suites and `bundles/self`/`verify`
  (not rerun); exact coverage outside the box; remote CI and the council.

**Unit 20 review return, 2026-09-29 (same run, based on `ee25bcda`;
evidence.md, "Unit 20, review return").** 20.1 is **reopened**. The first
visit's completion claim was wrong in two ways. Its rows omitted
agent-backed gates, Codex select and inherited sites, the inline Claude
sequence step, and every nested, inherited and fallback LaneTally/DSH site.
And the typed LaneTally row is a production defect, not a follow-up. Again
only `capability_launch.rs` changed; no production file moved.

- **R1.** The two serving tests are now one,
  `every_compiled_site_shape_of_every_harness_is_served_its_whole_command_beside_its_charter`.
  It has 114 rows, and it asserts that they are every compiled site and
  candidate of its bundle, no more and no fewer. Nine carriers (four
  inline harnesses; the offices `pair`, `pair-claude`, `pair-tally`,
  `pair-flash`; and `boxed`, Codex with hands under `harness`) are seated
  at a work seat, a gate where they may hold one, two panel members, two
  sequence steps, a select case, the default and an inherited seat. There
  are also three typed/routed seats. Each row asserts the site's charter
  pin, its whole cold command, and at a work site the whole command served
  when a session is offered. The `boxed-gate` row is the agent-backed
  Codex gate with hands, served through its last-message door.
- **R2.** `a_compiled_inline_codex_panel_member_with_hands_…` now asserts
  both boxed members' own charter pins.
- **R3.** A DSH prompt is compared with the literal `dsh_prompt`, not with
  `render_prompt`.
- **R4.** `recorded_argv` drops only the record's last NUL terminator, so
  an empty argument stays an argument.
- **R5 (blocker, not fixed).** An inline LaneTally seat with a typed
  `tools.allow` compiles on the shipped adapters, and its driver's final
  check then refuses the launch. The row keeps the exact refusal as a
  reproduction. Root cause: an unmeasured plan carries no provenance.
  `NativePlan::Unmeasured` (`capabilities.rs:1904-1908`, emitted at
  `:1257`) writes no `local`, and the protocol's `decode`
  (`native_controls.rs:211-218`) returns default provenance for
  `unmeasured`. So `final_tools` reads the lowered list as the template's
  (`Conflict::Carried`). Closing 20.1 needs a production unit over those
  two files. Either the unmeasured plan carries and decodes the lowered
  provenance, or the compile refuses the shape (D5.3). The operator
  chooses which.
- **R6.** `inline_command` now delegates to `body_command`. `sealing_moved`
  and `dispatched` share `sealing_at`, under `compose_at`'s boundary rule.
  `lanetally_serving_child` is removed. The LaneTally serving test now
  uses the shared `serve_driver`, `recording_harness`, `recorded_argv` and
  `result_error`.
- **Mutations.** MR4, MR3, MD, MC1, MC2, MLT and MRJ each fail their rows
  and are restored. Under MR4 and MR3 the pre-return test passed.
- No standing-admission lines and no fixture migrations.
- **Gates** and **pending**: see evidence.md.

**Unit 20, third visit (2026-09-29, same run, at `945bdec9`; evidence.md,
"Unit 20, third visit"): blocked.** No ruling on R5 has landed, and the
`lanetally-typed` row still asserts the refusal. 20.1 stays open until the
operator chooses delivery or a compile refusal, and a production unit over
`capabilities.rs` and `native_controls.rs` lands. Nothing else changed.

**Unit 20-fix, 2026-09-29 (run `0065-rebuild-unit-20-see-the-uni-7c8035c7`,
based on `27d4b5de`; evidence.md, "Unit 20-fix"): 20.1 closes.** The
operator ruled R5 on 2026-09-29: the compile refuses the shape. The ruling
is landed verbatim as an addendum to `operator-ruling-2026-09-23.md`.

- **Production.** `capabilities.rs` refuses a lowered typed `tools.allow`
  on an unmeasured plan at compile. The refusal names the site, harness,
  provider and the plan's own unmeasured cause. Otherwise the unmeasured
  plan carries and writes the provenance it was served. In
  `native_controls.rs`, `decode` reads an unmeasured plan's provenance
  through the helper the known branch uses, never a default.
- **Tests.** In the matrix, the `lanetally-typed` row is now an exact
  compile refusal in both its inline and agent-backed (`tally-typed`)
  forms. A pinned resume recompiles through the same compiler (corrected
  by the review return below: its line is the CLI's, and is unproved). The rest of
  the matrix is 113 rows. There are two new provenance tests, one in
  `capability_launch.rs` and one in `adapters/tests.rs`.
- **Proof.** Baseline reds were observed on `27d4b5de`. Mutations M1–M4
  each fail and are restored.
- **Standing-admission line.** `bundle/agent_tests.rs:551`: the `invented`
  fixture adapter declares `"native_capabilities": {"known": {}}`. This
  change reaches that fixture driver's typed allow. The line adds or
  removes no assertion.
- **Compiler-forced line.** `capability_launch.rs:4101` names the new
  field.
- No fixture migrations.
- **Gates.** fmt, clippy, the protocol, runtime and CLI suites,
  `bundles/self` and `verify`, strict OpenSpec (18) and `git diff --check`
  are clean.
- **Pending.** macOS, exact coverage outside the box, remote CI and the
  council. 21.3 stays open for unit 21's restriction rows.

**Unit 20-fix, review return (2026-09-29, same run, at `39b9b1a8`;
evidence.md, "Unit 20-fix, review return"): oversized. 20.1 is reopened.**

- **R1 (medium), not closed here.** The inline and agent-backed pinned
  `brokkr resume` refusal/no-launch assertions are absent. The claim above
  that a resume is refused "in the same words" was wrong:
  `brokkr-cli/src/lib.rs:2312` renders the compiler's refusal through
  `unreproducible` (`:2062`), as `run '<run>' pins a different bundle:
  capabilities differ: …`. That line is reachable only from a brokkr-cli
  suite (`crates/brokkr-cli/tests/capability_verbs.rs`, beside its existing
  `pins a different bundle` rows), which is outside this unit's test files.
  The matrix doc comment now says so. The split needed: a unit 20-fix-b
  whose one test file is `capability_verbs.rs`, with no production file.
  20.1 stays open until it lands.
- **R2 (low), closed.** In `capabilities.rs`, the compile refusal now
  states its complete cause straight after the site, in fixed words. The
  adapter's own reason follows as the line's tail, and the 512-scalar
  bound cuts only that. The matrix literal binds both forms, and each
  keeps the shipped reason's first sentence whole.
- **R3 (low), closed.** The new `capability_launch.rs` test
  `a_compiled_unmeasured_site_carries_its_typed_hands_into_its_compose`
  compiles a boxed LaneTally office with hands on a copied adapter and
  asserts two exact compile refusals. The first names the carried count
  ("types 2 arguments … as the box's hands"). Mutation M5 removes the
  compiler's transfer, and the site then COMPILES (`Ok(2)`) with its hands
  untyped.
- No standing-admission lines and no fixture migrations.
- **Gates** (re-observed in the committing session): fmt, clippy,
  brokkr-runtime and brokkr-cli suites, `bundles/self`, strict OpenSpec
  (18) and diff check are clean. **Pending**: unit 20-fix-b for R1, then
  20.1, unit 21 for 21.3, macOS, external exact coverage, remote CI and
  the council. See evidence.md.

**Unit 20-fix, second triage return (2026-09-29, same run, at `8d79faad`;
evidence.md, "Unit 20-fix, second triage return"): blocked.** Triage
re-ruled chore, but no addendum grants `capability_verbs.rs`. R1's pinned
`brokkr resume` rows can be proved only there. The matrix and protocol
suites were re-run and pass. Nothing else changed. 20.1 stays open until
the operator admits that file or commissions unit 20-fix-b.

## 21. Unit 21 — Prove compiled cold and actual-resume restrictions

- [ ] 21.1 Unit 21 proves managed Read/empty restrictions in compiled cold/eligible-resume Claude/LaneTally shapes. Verify authored lists refuse, prompt integrity and independent inventory. Requirements: [Every accepted native control reaches the final command][NCC], [Explicit restrictive tool lists retain their meaning][NCT], [Prompt values cannot absorb a composed control][NCP], [Subtractive tool lists never grant a capability][RGS]. Reopened/remaining: operator ruling 1–2 / R10. (previous 7.4)

21.2 is **deferred** to the restriction-transport slice by the operator's
addendum of 2026-09-25 (design.md D11). The task, unticked, is under "Deferred
to the restriction-transport slice" below.

- [ ] 21.3 Units 20–21 map every supported launch shape to real compiled full literal/refusal assertions and selected charter facts. Verify holdings-only/is_ok do not close rows; close only after unit 21 proves the managed Read/empty and CQ1 restriction rows too (narrowed by the addendum of 2026-09-25; the held nonempty rows are deferred below). Requirements: [Denial and admission have removal proofs and bounded live claims][NC6], [Every accepted native control reaches the final command][NCC], [Library charter pins are enforced at consumption][MPL]. Reopened/remaining: operator ruling 1–2 / R10. (previous 7.5)

  **Advanced by unit 20 (2026-09-29).** evidence.md, "Unit 20 — the
  compiled refusal and serving-shape matrix", maps every supported
  non-restriction row to a compiled whole-command or full-refusal assertion
  and the site's charter pin. The rows cover:

  - Claude, Codex, LaneTally and DSH;
  - inline and agent-backed forms;
  - work seat, gate, panel member, sequence step, select case and default,
    and inherited sites;
  - primary and fallback candidates;
  - cold launches, and the offered session: either an eligible rejoin, or
    declined and served cold.

  One row is a known gap. The boxed inline Codex panel member's charter pin
  is covered only by the same-shape unboxed member row. Still open for
  unit 21: the managed Read/empty and CQ1 restriction rows.

  **Corrected by unit 20's review return (2026-09-29).** The claim above
  was wrong: the first visit's rows were not every supported
  non-restriction row. The enumeration is now the 114-row matrix, which
  asserts that it covers every compiled site and candidate. The boxed
  members' own pins are asserted (R2), so that gap is closed. Beside unit
  21's restriction rows, 21.3 also waits on the R5 production unit. The
  typed LaneTally row is a full refusal where the settled design expects
  either a delivered launch or a compile refusal.

  **Advanced by unit 20-fix (2026-09-29).** The R5 production unit has
  landed. The typed LaneTally row is now the compile refusal the operator
  ruled, in both its inline and agent-backed forms. 21.3 now waits only on
  unit 21's restriction rows.

## 22. Unit 22 — Submit whole plans to doctor

- [ ] 22.1 Unit 22 submits whole plans in both doctor paths. Verify interacting OFF/final-state conflicts and explicit adapter-only scope. Requirements: [Doctor reports grants for every realm][CD1], [Installed native capabilities absent from grants are explicit][CD2], [Restrictions are validated, carried and pinned without engine interpretation][RG4]. Reopened/remaining: operator ruling 2–4. (previous 8.3)

- [ ] 22.2 Unit 22 independently asserts full doctor/compile outcomes for all grant shapes. Remove whole-plan assessment, observe intended failure, restore/pass; no model invocation. Requirements: [Installed native capabilities absent from grants are explicit][CD2], [Unknown inventories and live-control gaps remain unmeasured][CD3], [Known native powers require a valid delivered denial or refusal][NCR]. Reopened/remaining: operator ruling 2–4. (previous 8.4)

## 23. Unit 23 — Audit launch enforcement removals

- [ ] 23.1 Unit 23 independently removes authored refusal/load parsing/final parse-state/cold-resume empty-restriction and CQ1/ON-OFF enforcement (narrowed by the addendum of 2026-09-25; the held nonempty restriction removal is deferred below). Verify intended compiled final assertions fail, restore/pass. Requirements: [Denial and admission have removal proofs and bounded live claims][NC6], [Every accepted native control reaches the final command][NCC], [Authored provider configuration cannot supply capability authority][RGR], [Refusal proofs assert the full reason][SC8]. Reopened/remaining: operator ruling 1–4 / R10. (previous 9.2)

## 24. Unit 24 — Audit identity enforcement removals

- [ ] 24.1 Unit 24 independently removes containment/regular-policy pin/owner-target/read-binding/verified-buffer enforcement. Verify exact failures including equal-byte retargets, restore/pass. Requirements: [Active instructions and policy cannot escape bundle identity][MPI], [Library charter pins are enforced at consumption][MPL], [Capability authorization participates in bundle identity][MP2]. Reopened/remaining: operator ruling 1–4 / R10. (previous 9.3)

## 25. Unit 25 — Audit proof history and portability

- [ ] 25.1 Unit 25 audits baseline red/fix/removal/restored revisions from each owning unit. Record reds before fixes, never invent history. Verify intended exact assertions. Requirements: [Refusal proofs assert the full reason][SC8], [Denial and admission have removal proofs and bounded live claims][NC6]. Reopened/remaining: operator ruling 1–4 / R10. (previous 0.2)

- [ ] 25.2 Unit 25 audits canonical roots including R12 restriction-resume. Retain TempDir, canonicalize once for writes/expectations. Verify alias-root regression and Linux/macOS results. Requirement: [Refusal proofs assert the full reason][SC8]. Reopened/remaining: operator ruling 1–2 / R12. (previous 3.6)

- [ ] 25.3 Unit 25 audits finding/baseline/fix/removal/restored revisions and compiled matrix. Verify stale evidence closes nothing; retain L1 and historical security/spec-defect facts. Requirements: [Refusal proofs assert the full reason][SC8], [Denial and admission have removal proofs and bounded live claims][NC6], [Digest pins are measured and their history remains truthful][MP5]. Reopened/remaining: operator ruling 1–4 / R10. (previous 9.4)

## 26. Unit 26 — Update guides, measured pins and scope audit

- [ ] 26.1 Unit 26 corrects guides to refusal/typed migration/containment/whole-plan doctor. Verify no outward-link or merging advice remains; retain live/later-slice limits. Requirements: [Active instructions and policy cannot escape bundle identity][MPI], [Authored provider configuration cannot supply capability authority][RGR], [Installed native capabilities absent from grants are explicit][CD2], [Slice-one records do not claim later-slice behavior][MP6]. Reopened/remaining: operator ruling 1–4. (previous 10.1)

- [ ] 26.2 Unit 26 compiles self/verify/all witnesses and measures bytes/pins with reasons. Verify witness/compose/library tests; retain historical measurements. Requirement: [Digest pins are measured and their history remains truthful][MP5]. Reopened/remaining: operator ruling 1–4. (previous 10.2)

- [ ] 26.3 Unit 26 audits frozen bytes/empty grants/hands/MCP/later-slice scope. Verify inventory, compiler-pin agreement and additive contracts only. Requirements: [Realms v6 adds grants without changing frozen versions][RG1], [Legacy concrete permissions cannot grandfather a capability][SC7], [Reserved hands preserves the existing workspace authority][TD6], [Slice-one records do not claim later-slice behavior][MP6]. Reopened/remaining: operator ruling 1–4. (previous 10.3)

## 27. Unit 27 — Validate and commit the rebuilt candidate

- [ ] 27.1 **Unit 27, rebuilt final candidate:** Run `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`; resolve failures and record actual exits. Keep dependency versions and required compiler/license constraints unchanged by the repair. Requirement: [Digest pins are measured and their history remains truthful][MP5] (scenario: Validation remains a proof obligation). Reopened/remaining: operator ruling 1–4 / V1/R13. (previous 11.1)

- [ ] 27.2 **Unit 27, rebuilt final candidate:** Run each crate suite with `cargo test -p <crate> --all-features --locked` for `brokkr-core`, `brokkr-store`, `brokkr-protocol`, `brokkr-runtime`, `brokkr-view`, `brokkr-bridge` and `brokkr-cli`, then `cargo test --workspace` and `cargo test --workspace --all-features --locked`. Record each actual result and obtain relevant Linux and macOS fixture/suite results on the repair candidate; Linux is not macOS evidence. Missing commands/skipped boundary execution remain pending. Attribute issue-255's known flake if observed without repairing it here or counting a failed suite green. Requirements: [Digest pins are measured and their history remains truthful][MP5] (scenario: Validation remains a proof obligation); [Refusal proofs assert the full reason][SC8] (scenario: macOS canonical fixture roots preserve exact diagnostics). Reopened/remaining: operator ruling 1–4 / V1/R13. (previous 11.2)

- [ ] 27.3 **Unit 27, rebuilt final candidate:** Run `cargo run --locked -p brokkr-cli -- compile --bundle bundles/self` and the equivalent `bundles/verify` compile; confirm final measured pins. Run `openspec validate --all --strict --no-interactive` and `git diff --check`, recording results for the final candidate. Keep repaired artifacts coherent and council/live measurement status explicit. Requirement: [Digest pins are measured and their history remains truthful][MP5] (scenario: Validation remains a proof obligation). Reopened/remaining: operator ruling 1–4 / V1/R13. (previous 11.3)

- [ ] 27.4 **Unit 27, rebuilt final candidate:** Run the authorized `bash scripts/coverage-exact.sh` on the final head with the unchanged pinned compiler and exact gate. Preserve the chief's failed baseline separately (34897/35073 lines, 5730/5744 branches, 3443/3453 logical functions; exit 1). Inspect uncovered regions and fix/prove in-scope omissions without assuming a regression cause; do not widen this repair into unrelated work. Preserve the fresh report/revision and report **source lines, branches and logical functions** separately as covered/total counts, each nonzero and exactly equal (literal 100%), including every added production line. If workspace restrictions prevent execution, obtain host/CI evidence outside the nested box; leave this task pending until that final-head result exists. Missing counts are unavailable, not zero/zero or rounded 100%. Do not exclude lines or lower thresholds. Requirement: [Digest pins are measured and their history remains truthful][MP5] (scenario: Validation remains a proof obligation). Reopened/remaining: operator ruling 1–4 / V1/R13. (previous 11.4)

- [ ] 27.5 **Unit 27, rebuilt final candidate:** Audit the complete finding/test/removal matrix against 25.3 and the actual gate results, leaving unavailable obligations open. Commit restored work, task ticks and observed evidence in repository message style; never push. Require clean scoped status, adopted history, no leftover mutation and all required repair proofs/gates green before a repair-completion claim. Run strict all-item OpenSpec validation, diff cleanliness and the unchanged exact-coverage gate again on the resulting final committed HEAD; record that SHA, command exits and all three fresh counts in run-local evidence/result outside tracked inputs so recording the final check does not silently create a different source head. A failure or further tracked edit reopens the affected task and requires a new committed candidate and final-head checks. Report the actual commit and coverage counts with 28.1 open and the security hold awaiting council. Requirements: [Digest pins are measured and their history remains truthful][MP5]; [Refusal proofs assert the full reason][SC8]; house commit/no-push rule. Reopened/remaining: operator ruling 1–4 / V1/R13. (previous 11.5)

## 28. Later council judgment and final archive

- [ ] 28.1 **Do not archive or fold in this commission.** The operator holds this final task open for council re-judgment; completed local repairs/gates do not clear the security hold. If subsequently authorized after that judgment, the dialect's final operation is `openspec archive decision-0065-capabilities-slice-one --yes`, folding the six deltas into living truth with append-only provenance, followed by archived strict validation and a commit. That later work is outside this repair visit, so leave this box unchecked. Requirements: [Slice-one records do not claim later-slice behavior][MP6]; [Digest pins are measured and their history remains truthful][MP5]; operator no-archive ruling and dialect archive operation. (previous 12.1)

## Deferred to the restriction-transport slice

The operator's addendum of 2026-09-25 ("the nonempty restriction positive is
deferred", operator-ruling-2026-09-23.md; design.md D11) moves these out of
slice one. They stay unticked and are not slice-one work; their text is kept
as it stood.

- [ ] 9.1 **DEFERRED (addendum 2026-09-25).** Unit 9 qualifies nonempty restriction semantics and production fixture; --settings syntax alone is insufficient. Verify provider/version evidence; report upstream before dependent work if no supported transport meets the positive. Requirements: [Restrictions are validated, carried and pinned without engine interpretation][RG4], [Every accepted native control reaches the final command][NCC]. Reopened/remaining: operator ruling 1–4 / R10. (previous 0.3) Returned upstream 2026-09-25 (run `0065-rebuild-unit-9-see-the-unit-f241ed65`; evidence.md, "Unit 9"; design.md, Open questions): no shipped provider has a bounded supported transport. All three shipped dialects admit only `{}`. Claude and Codex declare restrictions unsupported, and DSH and LaneTally are unmeasured. The single `{restrictions_json}` slot reaches Claude only through `Load` options. Claude WebSearch has no specifier, and WebFetch `domain:` rules are pre-grants that preapproved domains and other settings scopes widen (Claude Code docs, fetched 2026-09-25). Codex host filtering and the installed Claude version were not observable from the seat and are pending. No production, test or pin byte moved, and no admitted test line was used. The operator ruled option (a), defer, on 2026-09-25.

- [ ] 21.2 **DEFERRED (addendum 2026-09-25).** Unit 21 separately compiles held nonempty restriction cold/resume fixtures via real realm/dialect/candidate resolution. Verify independent final literals, manifest and session, no fabricated Controls. Requirements: [Restrictions are validated, carried and pinned without engine interpretation][RG4], [Every accepted native control reaches the final command][NCC], [Denial and admission have removal proofs and bounded live claims][NC6]. Reopened/remaining: operator ruling 1–2 / R10. (previous 7.6)

- **11.2's restriction half, as it stood:** "Unit 11 carries unit 9's qualified restriction through real resolution. Verify JSON/encoding/identity and required/wants/unused outcomes; 21.2 owns final proof."
- **Unit 11's retired nonempty-restriction positives** (rebuild unit 11, 2026-09-25; each is now the exact deferral refusal, evidence.md "Unit 11 — built"):
  - `capabilities/tests.rs`, `an_expressible_restriction_rides_one_typed_argument_unchanged` (replaced by `a_declared_transport_carries_only_the_empty_restriction`). A `requires` over transport `["--search-restrict", "{restrictions_json}"]` held `{"allow":{"hosts":["yaml.org","sourceware.org"]}}` and composed `--search-on --search-restrict {"allow":{"hosts":["yaml.org","sourceware.org"]}}`, array order intact.
  - `agents/tests.rs`, `unit3_native_expectation_is_sealed_from_typed_inputs_not_from_emission`, row "a held subset under a restriction". The expectation held `web-search` on `lookup` with that restriction, and the contribution argv was `--fetch-off --search-restrict {"allow":{"hosts":["yaml.org","sourceware.org"]}}`.
  - `bundle/agent_tests.rs`, `resolved_native_on_and_restriction_contributions_obey_the_same_refusals`, has two retired pieces:
    - the "clean native restriction" row compiled with the typed `workspace-write` class, and its resolved argv was `-c web_search="live" -c web_search={"allow":{"hosts":["example.org"]}}`;
    - the four "native restriction <spelling>" rows refused a root selector carried in the substituted restriction argv (D5.3).
- **21.3's restriction portion:** the held nonempty restriction rows of the launch matrix ("close only after unit 21 proves the restriction rows too").
- **23.1's restriction portion:** the independent cold/resume removal of the held nonempty restriction's delivery.

## Deferred to typed sandbox lowering at members

The operator's addendum of 2026-09-26 ("the panel-member rejoin positive is
deferred", operator-ruling-2026-09-23.md) accepts the re-planted panel-member
rows as slice one's panel-member proof and moves this positive, unticked, to
the later slice that lowers a typed sandbox at panel members (decision 0072's
follow-up). The owning delta carries the same pointer: specs/native-capability-controls/spec.md,
"Deferred to typed sandbox lowering at members".

- [ ] **DEFERRED (addendum 2026-09-26).** The no-hands panel member rejoin positive, as it stood before `b33de51f`: in `driver_conformance.rs::the_compiled_live_inline_codex_shapes_rejoin_their_provider_confirmed_root`, the `NoHandsMember` rows (wrapped and unwrapped) rejoined the provider-confirmed root as `resumed` with no refusal, and the exec resume argv re-expressed the sandbox class and effort. Since `b33de51f` (fixture migration of 2026-09-26) those rows bind the adapter's fail-closed `sandbox-unavailable` cold retry, because design.md D5.3 refuses a typed class at a panel member and ruling 1 refuses an authored `--sandbox`. Unit 12's review F2 (evidence.md, "Unit 12 — the review's return", F2) is closed by this deferral, not by a positive.

[TD1]: specs/tool-dialect-contract/spec.md#requirement-a-tool-dialect-binds-one-capability-to-exactly-one-implementation-kind
[TD2]: specs/tool-dialect-contract/spec.md#requirement-capability-classes-belong-to-the-abstraction
[TD3]: specs/tool-dialect-contract/spec.md#requirement-dialects-describe-their-disclosure-using-the-existing-egress-vocabulary
[TD4]: specs/tool-dialect-contract/spec.md#requirement-the-mcp-contract-is-whole-before-its-runtime-exists
[TD5]: specs/tool-dialect-contract/spec.md#requirement-restriction-keys-are-defined-by-the-selected-dialect
[TD6]: specs/tool-dialect-contract/spec.md#requirement-reserved-hands-preserves-the-existing-workspace-authority
[RG1]: specs/realm-capability-grants/spec.md#requirement-realms-v6-adds-grants-without-changing-frozen-versions
[RG2]: specs/realm-capability-grants/spec.md#requirement-only-an-operators-realm-selects-concrete-implementations
[RG3]: specs/realm-capability-grants/spec.md#requirement-office-scopes-and-tool-subsets-only-narrow-a-grant
[RG4]: specs/realm-capability-grants/spec.md#requirement-restrictions-are-validated-carried-and-pinned-without-engine-interpretation
[RG5]: specs/realm-capability-grants/spec.md#requirement-realm-context-is-resolved-before-capability-authorization
[SC1]: specs/seat-capability-resolution/spec.md#requirement-requests-use-abstract-names-and-a-closed-strength-vocabulary
[SC2]: specs/seat-capability-resolution/spec.md#requirement-a-seat-can-subtract-but-cannot-widen-its-office
[SC3]: specs/seat-capability-resolution/spec.md#requirement-compilation-holds-only-the-request-and-applicable-grant-intersection
[SC4]: specs/seat-capability-resolution/spec.md#requirement-provider-compatibility-cannot-expand-a-holding
[SC5]: specs/seat-capability-resolution/spec.md#requirement-an-impossible-native-denial-is-a-compile-refusal
[SC6]: specs/seat-capability-resolution/spec.md#requirement-any-mcp-realm-grant-refuses-until-slice-two
[SC7]: specs/seat-capability-resolution/spec.md#requirement-legacy-concrete-permissions-cannot-grandfather-a-capability
[SC8]: specs/seat-capability-resolution/spec.md#requirement-refusal-proofs-assert-the-full-reason
[NC1]: specs/native-capability-controls/spec.md#requirement-native-capability-controls-are-adapter-owned-evidence-bearing-data
[NC2]: specs/native-capability-controls/spec.md#requirement-codex-web-search-off-uses-the-controllers-measured-fragment
[NC3]: specs/native-capability-controls/spec.md#requirement-eligible-codex-resumes-reimpose-the-capability-control
[NC4]: specs/native-capability-controls/spec.md#requirement-neither-inline-arguments-nor-fallback-can-override-native-denial
[NC5]: specs/native-capability-controls/spec.md#requirement-other-provider-declarations-preserve-the-commissioned-uncertainty
[NC6]: specs/native-capability-controls/spec.md#requirement-denial-and-admission-have-removal-proofs-and-bounded-live-claims
[CD1]: specs/capability-doctor/spec.md#requirement-doctor-reports-grants-for-every-realm
[CD2]: specs/capability-doctor/spec.md#requirement-installed-native-capabilities-absent-from-grants-are-explicit
[CD3]: specs/capability-doctor/spec.md#requirement-unknown-inventories-and-live-control-gaps-remain-unmeasured
[CD4]: specs/capability-doctor/spec.md#requirement-doctor-explains-unbuilt-bindings-without-running-capabilities
[MP1]: specs/capability-manifest-and-prompts/spec.md#requirement-a-new-manifest-version-records-capabilities-per-executable-seat
[MP2]: specs/capability-manifest-and-prompts/spec.md#requirement-capability-authorization-participates-in-bundle-identity
[MP3]: specs/capability-manifest-and-prompts/spec.md#requirement-prompt-capability-statements-reflect-the-serving-seats-pinned-holdings
[MP4]: specs/capability-manifest-and-prompts/spec.md#requirement-capability-responses-are-data-and-confer-no-authority
[MP5]: specs/capability-manifest-and-prompts/spec.md#requirement-digest-pins-are-measured-and-their-history-remains-truthful
[MP6]: specs/capability-manifest-and-prompts/spec.md#requirement-slice-one-records-do-not-claim-later-slice-behavior
[NCR]: specs/native-capability-controls/spec.md#requirement-known-native-powers-require-a-valid-delivered-denial-or-refusal
[NCC]: specs/native-capability-controls/spec.md#requirement-every-accepted-native-control-reaches-the-final-command
[NCD]: specs/native-capability-controls/spec.md#requirement-a-dsh-launch-refuses-authority-before-it-reads-its-boundary
[RGR]: specs/realm-capability-grants/spec.md#requirement-authored-provider-configuration-cannot-supply-capability-authority
[MPI]: specs/capability-manifest-and-prompts/spec.md#requirement-active-instructions-and-policy-cannot-escape-bundle-identity
[RGP]: specs/realm-capability-grants/spec.md#requirement-known-provider-commands-have-a-closed-argument-grammar
[RGS]: specs/realm-capability-grants/spec.md#requirement-subtractive-tool-lists-never-grant-a-capability
[NCP]: specs/native-capability-controls/spec.md#requirement-prompt-values-cannot-absorb-a-composed-control
[NCT]: specs/native-capability-controls/spec.md#requirement-explicit-restrictive-tool-lists-retain-their-meaning
[MPL]: specs/capability-manifest-and-prompts/spec.md#requirement-library-charter-pins-are-enforced-at-consumption
[SCM]: specs/seat-capability-resolution/spec.md#requirement-shipped-inline-permissions-migrate-before-refusal-lands
