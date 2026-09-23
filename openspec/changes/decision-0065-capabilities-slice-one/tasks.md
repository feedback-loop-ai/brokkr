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

- [ ] 2.1.5 Repair D5.5's demonstrated competing-control gap in `bundle.rs::expressed_sandbox` and bind the remaining D5.3 admission rows in the owning suites; retain the rest of the adopted intermediate admission. Keep unspecified defaults and existing mapped nonempty agent-direct/hands paths; refuse inline direct allow, direct empty and unsupported sandbox lowering with the exact owning-field/representation cause. After independent constitutional checks, admit explicit sandbox only for actual Codex dispatch with hands and exactly matching existing engine fragments: harness gate read-only, harness work workspace-write, boxed read-only. Use the existing public protocol grammar to require exactly one sandbox control of the requested class and refuse competing controls or opaque configuration in both the selected fragment and other contributions; authored bytes cannot supply representation. Before repairing the guard, add independent cases pairing each of `--full-auto`, `--dangerously-bypass-approvals-and-sandbox`, configuration under `sandbox_mode`, and configuration under `sandbox_workspace_write` (including the demonstrated `sandbox_workspace_write.network_access=true`) with both authored argv and the selected engine fragment. Use a matching requested class so a mismatch cannot hide the competing control. The sandbox_mode neighbor already refuses: record its actual baseline, while each newly exposed admission must fail the exact full-refusal assertion. Reuse parsed canonical option identities and config paths; no new grammar or blanket `-c` refusal. Retain exact positive hands/effort configurations and boxed/gate/work selections. For each missing proof, independently remove the corresponding guard or contribution check, observe that row's intended full assertion failure, restore and pass; an already-passing neighbor or first failing loop row proves no later case. Refuse each nonmatching class, missing fragment, open work, no-hands/inline sandbox, opaque dispatch labelled codex, and Claude/LaneTally/DSH/exec sandbox. Verify every D5.3 row independently in the owning suites, with exact effective fields, selected fragments, boundary and complete refusals; prove existing open-gate/missing-hands/model-policy errors keep precedence and later candidates cannot hide a mismatch. Under empty realm grants, compare full holdings/native OFF facts for admitted fixtures and confirm hands policy/boundary stay unchanged. Mutate each guard or selected-fragment check independently, observe the intended exact assertion fail, restore and pass. These are compile-admission proofs; no final-launch or live-provider claim follows. Requirements: [Shipped inline permissions migrate before refusal lands][SCM], [Authored provider configuration cannot supply capability authority][RGR], [Reserved hands preserves the existing workspace authority][TD6], [Known native powers require a valid delivered denial or refusal][NCR], [Refusal proofs assert the full reason][SC8]. (substep of previous 3.14)

- [ ] 2.1.6 Audit the accumulated baseline/fix/removal/restoration ledger in evidence.md against every new test and each independently claimed row in the two owning suites. Record the revision, exact test and row, command, compiling in-scope mutation, actual left/right or full failing assertion, restoration and passing rerun. Separate retained historical proof, newly observed baseline passes/reds, and any retrospective reconstruction; do not relabel missing original baseline history as completed. A table aborting at its first row proves no later row; isolate mutations/filters so each claimed row reaches its assertion. Reject `is_err()`, substring-only reasons, production-derived expected output, compilation errors or unrelated fixture failures as proof. Confirm every mutation is restored and all new fixtures retain one canonical temporary root, with no `.forge/` reads or installed provider. Verify coverage against SCM's unit 2 scenarios and SC7/SC8, distinguishing later migration/final-command scenarios that remain open. Requirements: [Refusal proofs assert the full reason][SC8], [Shipped inline permissions migrate before refusal lands][SCM], [Legacy concrete permissions cannot grandfather a capability][SC7]. (substep of previous 3.14)

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

- [ ] 2-fix.1 Capture the current baseline in the existing canonical temporary fixture before repair. Each of the chief's three native OFF cases (added root at workspace-write, bypass at a read-only gate, network-access configuration at workspace-write) and eight root-selector cases (four spellings independently in authored argv and hands.harness.work) must reach its exact full-refusal assertion. Record each observed red or already-passing row, with full test/row and assertion; preserve the two matching gate/work native-denial controls as exact positives. These are eleven supplied defect reproductions, not eleven executions already observed by this specify seat. Requirements: [Shipped inline permissions migrate before refusal lands][SCM], [Refusal proofs assert the full reason][SC8].

- [ ] 2-fix.2 Check the actual resolved native ON/OFF/restriction contributions against each candidate's typed sandbox at compile admission, preserving every valid native denial control, including the exact Codex web-search OFF pair. Refuse canonical `--cd` in every checked contribution (authored argv, selected hands and resolved native), covering `--cd PATH`, `--cd=PATH`, `-C PATH` and `-CPATH`; name the normalized option and never echo its value. Preserve bounded complete causes, existing authority/error precedence and matching sandbox positives. Neither an inert grammar label, engine origin nor unestablished last-option priority is admission evidence. Follow D5.6's post-resolution placement and narrow configuration allowance; any native `--sandbox` is a competing representation even when matching. Missing/unreadable plans refuse rather than becoming empty argv; no re-resolution or primary-only scan. Stay inside the production scope above. Requirements: [Shipped inline permissions migrate before refusal lands][SCM], [Known native powers require a valid delivered denial or refusal][NCR], [Authored provider configuration cannot supply capability authority][RGR].

- [ ] 2-fix.3 In bundle/agent_tests.rs bind each chief case and every added ON/OFF/restriction, spelling, redaction and positive row independently. Each new test and separately claimed row needs its own compiling in-scope mutation that makes the intended assertion fail, followed by restoration and a passing rerun. Record the mutation, exact failing test/row, actual assertion and restored result in evidence.md; a first-row failure, `is_err()`, substring-only reason, unrelated refusal or compile error closes nothing. Compare full values and full reasons, including full resolved denial argv rather than only OFF inventory labels. Exercise all four root spellings in selected native ON, OFF and nonempty substituted restriction paths; ON must hold through a real fixture realm/dialect, and restrictions must reach their transport. Bind inherited sandbox, a later-candidate conflict, untyped preservation and Unicode/newline redaction with the 512-scalar cause bound. Synthetic transport refusals do not qualify provider restriction support for unit 9. Retain matching sandbox/denial positives, independently remove the native denial allowance to bind them, and use one canonical temporary root with no .forge reads or installed provider. Do not claim a live escape or later final-launch proof. Requirements: [Refusal proofs assert the full reason][SC8], [Shipped inline permissions migrate before refusal lands][SCM], [Known native powers require a valid delivered denial or refusal][NCR].

- [ ] 2-fix.4 Restore every mutation, run the applicable local gates below on the repaired tree, record actual results and commit exactly the in-scope repair plus tasks/evidence in repository style. Preserve all adopted ancestry and never push. External exact coverage, macOS and remote CI stay pending until evidence for the tested head exists; no fully green claim follows from local checks. Only then close the repaired admission/proof/gate portions and aggregate, on that stated evidence basis. Requirements: [Refusal proofs assert the full reason][SC8], [Digest pins are measured and their history remains truthful][MP5].

The specification visit adds acceptance scenarios to the owning delta and
records the S1/A1 dispositions there under `## Decisions`; it implements no
Rust repair, runs no mutation and ticks no repair task. Its observed checks
and limitations are recorded under evidence.md's “Unit 2-fix — specification
adoption”.

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

## 4. Unit 4 — Wire origins through runtime dispatch

- [ ] 4.1 Unit 4 wires the selected candidate's private segments through engine.rs SiteSpawn, bundle projection, boundary composition, placeholder expansion and final input merging. Verify absent/reordered/overridden records refuse and legitimate typed controls survive. Requirements: [Shipped inline permissions migrate before refusal lands][SCM], [Authored provider configuration cannot supply capability authority][RGR], [Every accepted native control reaches the final command][NCC]. New explicit substep of 4.2: operator rulings 1–2; robustness runtime evidence. (previous 3.21)

- [ ] 4.2 Units 3–4 lower typed restrictions and carry distinct authored/template/local/hands/native origins end to end through runtime SiteSpawn, boundary/expansion and input assembly. Close only after unit 4 verifies exact limits, selected candidate, reassembly and override refusal; 15.2 owns final authored-counterfeit refusal. Requirements: [Shipped inline permissions migrate before refusal lands][SCM], [Authored provider configuration cannot supply capability authority][RGR], [Reserved hands preserves the existing workspace authority][TD6]. Reopened/remaining: operator ruling 1–2. (previous 3.15)

## 5. Unit 5 — Supply local mappings and scaffold support

- [ ] 5.1 Unit 5 supplies local mappings/scaffold support. Verify npm/npx/node, narrow gh-pr-view/gh-run-view and existing names with library/init suites, no native grants. Requirements: [Shipped inline permissions migrate before refusal lands][SCM], [Legacy concrete permissions cannot grandfather a capability][SC7]. Reopened/remaining: operator ruling 1–2. (previous 3.16)

- [ ] 5.2 Unit 5 aligns generated adapters/tools with migration/native assessment. Verify init_stacks/init_doctor/library-data scaffolds and native OFF. Requirements: [Native capability controls are adapter-owned evidence-bearing data][NC1], [Realms v6 adds grants without changing frozen versions][RG1], [Shipped inline permissions migrate before refusal lands][SCM]. Reopened/remaining: operator ruling 2–4. (previous 8.2)

## 6. Unit 6 — Migrate Claude recipes

- [ ] 6.1 Unit 6 migrates fast/node/preflight to typed tools. Verify exact compiled local limits/native OFF and measure moved pins. Requirement: [Shipped inline permissions migrate before refusal lands][SCM]. Reopened/remaining: operator ruling 1–2. (previous 3.17)

## 7. Unit 7 — Migrate verify and Codex restrictions

- [ ] 7.1 Unit 7 migrates verify/standby/review-first to typed permissions/sandbox. Verify exact commands, unchanged boundary authority and measured pins. Requirement: [Shipped inline permissions migrate before refusal lands][SCM]. Reopened/remaining: operator ruling 1–2. (previous 3.18)

## 8. Unit 8 — Migrate wager and finish the inventory

- [ ] 8.1 Unit 8 migrates wager-harness/advice and reruns all-directory inventory. Verify no shipped authored catalogue flag remains before refusal and no local limit broadens. Requirement: [Shipped inline permissions migrate before refusal lands][SCM]. Reopened/remaining: operator ruling 1–2. (previous 3.19)

## 9. Unit 9 — Qualify a supported nonempty restriction

- [ ] 9.1 Unit 9 qualifies nonempty restriction semantics and production fixture; --settings syntax alone is insufficient. Verify provider/version evidence; report upstream before dependent work if no supported transport meets the positive. Requirements: [Restrictions are validated, carried and pinned without engine interpretation][RG4], [Every accepted native control reaches the final command][NCC]. Reopened/remaining: operator ruling 1–4 / R10. (previous 0.3)

## 10. Unit 10 — Bound the grammar and redact diagnostics

- [ ] 10.1 Unit 10 inventories every option/form/effect/consumer, including engine/wrapper/session positions and origins. Verify complete all-harness catalogue tests. Requirements: [Known provider commands have a closed argument grammar][RGP], [Every accepted native control reaches the final command][NCC]. Reopened/remaining: operator ruling 1–2. (previous 3.9)

- [ ] 10.2 Unit 10 models five Codex forms, quoted/table/descendant/repeated keys and bounded inert configs. Verify malformed/unbounded refusal and value-redacted complete diagnostics. Requirements: [Known provider commands have a closed argument grammar][RGP], [Authored provider configuration cannot supply capability authority][RGR]. Reopened/remaining: operator ruling 1–2. (previous 3.10)

- [ ] 10.3 Unit 10 models Claude/LaneTally grammar without authored-list contributions. Verify managed patterns/separators, aliases, empties, inert prompts and wrapper boundaries. Requirements: [Known provider commands have a closed argument grammar][RGP], [Prompt values cannot absorb a composed control][NCP], [Explicit restrictive tool lists retain their meaning][NCT]. Reopened/remaining: operator ruling 1–2. (previous 3.11)

- [ ] 10.4 Unit 10 retains DSH closed grammar and bound route patch. Verify profile/web/plugin/capability/unknown/changed-patch refusals and route positive after #313/#326. Requirements: [Known provider commands have a closed argument grammar][RGP], [Authored provider configuration cannot supply capability authority][RGR], [Other provider declarations preserve the commissioned uncertainty][NC5]. Reopened/remaining: operator ruling 1–2. (previous 3.12)

## 11. Unit 11 — Validate both declared halves at load

- [ ] 11.1 Unit 11 parses both declared ON/OFF argv at load, even unused. Verify full Codex/mapping/separator/missing-authority refusals and valid positives. Requirements: [Native capability controls are adapter-owned evidence-bearing data][NC1], [Known native powers require a valid delivered denial or refusal][NCR], [Every accepted native control reaches the final command][NCC]. Reopened/remaining: operator ruling 1–2. (previous 3.3)

- [ ] 11.2 Unit 11 carries unit 9's qualified restriction through real resolution. Verify JSON/encoding/identity and required/wants/unused outcomes; 21.2 owns final proof. Requirements: [Restrictions are validated, carried and pinned without engine interpretation][RG4], [Every accepted native control reaches the final command][NCC], [Capability authorization participates in bundle identity][MP2]. Reopened/remaining: operator ruling 1–2. (previous 4.4)

## 12. Unit 12 — Enable authored refusal and engine-only composition

- [ ] 12.1 Unit 12 refuses every authored catalogue option regardless of grant/value/polarity/form. Verify complete all-harness matrix and bounded reasons without payloads; typed hands/route positives. Requirements: [Authored provider configuration cannot supply capability authority][RGR], [Known provider commands have a closed argument grammar][RGP], [Subtractive tool lists never grant a capability][RGS], [Reserved hands preserves the existing workspace authority][TD6], [Neither inline arguments nor fallback can override native denial][NC4]. Reopened/remaining: operator ruling 1–2. (previous 4.6)

- [ ] 12.2 Unit 12 deletes authored folding and composes only engine controls. Verify exact empty/nonempty restrictions, OFF/hands or full managed conflict refusal. Requirements: [Every accepted native control reaches the final command][NCC], [Explicit restrictive tool lists retain their meaning][NCT], [Authored provider configuration cannot supply capability authority][RGR]. Reopened/remaining: operator ruling 1–2. (previous 4.3)

## 13. Unit 13 — Build final assessment and share structural consumers

- [ ] 13.1 Unit 13 supplies the pure complete builder/checker and private checked command, with independent expected state and shared structural consumers. Verify managed contradictions, empty/absent distinction, prefix/selector grammar and no unchecked post-validation mutation. Cold/resume integration remains 14.1/15.1. Requirements: [Every accepted native control reaches the final command][NCC], [Known provider commands have a closed argument grammar][RGP], [Explicit restrictive tool lists retain their meaning][NCT]. New explicit prerequisite: operator ruling 2; both design positions. (previous 7.7)

- [ ] 13.2 Unit 13 replaces managed raw consumers with shared parsing. Verify inert --image resume, prompt/duplicate meaning and unchanged resume eligibility. Requirements: [Known provider commands have a closed argument grammar][RGP], [Prompt values cannot absorb a composed control][NCP], [Eligible Codex resumes reimpose the capability control][NC3]. Reopened/remaining: operator ruling 1–2. (previous 3.13)

## 14. Unit 14 — Integrate checked cold commands

- [ ] 14.1 Unit 14 parses full serialized cold commands and compares exact plan state. Verify dropped OFF/terminator/separator/prefix-duplicate refusal and typed positives. Requirements: [Codex web-search OFF uses the controller's measured fragment][NC2], [Known native powers require a valid delivered denial or refusal][NCR], [Every accepted native control reaches the final command][NCC]. Reopened/remaining: operator ruling 1–2 / R10. (previous 7.1)

## 15. Unit 15 — Integrate eligible resume and replacement

- [ ] 15.1 Unit 15 checks actual resume and cold replacement independently. Verify session/stdin/eligibility/controls and selected fallback OFF; cold never counts as resume evidence. Requirements: [Eligible Codex resumes reimpose the capability control][NC3], [Every accepted native control reaches the final command][NCC], [Denial and admission have removal proofs and bounded live claims][NC6], [Neither inline arguments nor fallback can override native denial][NC4]. Reopened/remaining: operator ruling 1–2 / R10. (previous 7.2)

- [ ] 15.2 Units 12–15 integrate refusal/provenance across compile and private launch input; close only after unit 15 covers cold, actual resume and replacement. Verify counterfeit/missing origins, structural substitutions and candidate replacement before work. Requirements: [Authored provider configuration cannot supply capability authority][RGR], [Known native powers require a valid delivered denial or refusal][NCR], [Every accepted native control reaches the final command][NCC]. Reopened/remaining: operator ruling 1–2. (previous 4.7)

## 16. Unit 16 — Bind canonical inputs and policy bytes

- [ ] 16.1 Unit 16 resolves actual files/owners and refuses outward/excluded/nonregular/unpinned inputs. Bind the verified read to the contained target by handle or refuse. Verify controlled replacements, equal-byte outward links/FIFOs and standalone/inherited full causes; path-string checks alone prove no race guarantee. Requirements: [Active instructions and policy cannot escape bundle identity][MPI], [Library charter pins are enforced at consumption][MPL]. Reopened/remaining: operator ruling 3. (previous 5.1)

- [ ] 16.2 Unit 16 binds regular policy read/hash/parse to owner pin, comparing later walk before sealing. Verify FIFO/changed-buffer refusal and allowed identity movement. Requirements: [Active instructions and policy cannot escape bundle identity][MPI], [Capability authorization participates in bundle identity][MP2]. Reopened/remaining: operator ruling 3. (previous 5.2)

- [ ] 16.3 Unit 16 replaces outward-link positives with containment refusal. Verify four role/policy standalone/inherited escapes and contained-link/byte-change controls. Requirement: [Active instructions and policy cannot escape bundle identity][MPI]. Reopened/remaining: operator ruling 3. (previous 6.1)

## 17. Unit 17 — Select charter owner and source at compile

- [ ] 17.1 Unit 17 binds selected charter owner/reference/target/digest, no longest-prefix guess. Verify external/nested/overlapping owners and all site/candidate paths. Requirements: [Library charter pins are enforced at consumption][MPL], [A new manifest version records capabilities per executable seat][MP1]. Reopened/remaining: operator ruling 3. (previous 6.2)

## 18. Unit 18 — Consume the bound charter and render its buffer

- [ ] 18.1 Unit 18 checks owner/target/bytes at dispatch and sends verified buffer. Verify changed/equal-byte retarget refusal without recompilation and no provider work. Requirements: [Library charter pins are enforced at consumption][MPL], [Active instructions and policy cannot escape bundle identity][MPI], [Capability responses are data and confer no authority][MP4]. Reopened/remaining: operator ruling 3. (previous 6.3)

- [ ] 18.2 Unit 18 verifies selected binding/holding/prompt across every serving shape; authored merges cannot replace text and rendering cannot reread. Requirements: [Library charter pins are enforced at consumption][MPL], [Prompt capability statements reflect the serving seat's pinned holdings][MP3], [Capability responses are data and confer no authority][MP4]. Reopened/remaining: operator ruling 3. (previous 6.5)

- [ ] 18.3 Unit 18 verifies full rendered holdings/drops/DATA statements match selected verified charter across paths, no authored replacement/reread. Requirements: [Prompt capability statements reflect the serving seat's pinned holdings][MP3], [Capability responses are data and confer no authority][MP4], [Library charter pins are enforced at consumption][MPL]. Reopened/remaining: operator ruling 2–4. (previous 8.1)

## 19. Unit 19 — Enforce charter integrity at start and pinned resume

- [ ] 19.1 Unit 19 proves owner-bound start/resume checks for changed/excluded/retargeted/missing/restored charters. Verify retained map-edit/identity/unmapped-root behavior. Requirements: [Library charter pins are enforced at consumption][MPL], [Active instructions and policy cannot escape bundle identity][MPI], [Realm context is resolved before capability authorization][RG5]. Reopened/remaining: operator ruling 3. (previous 6.4)

## 20. Unit 20 — Audit compiled refusal and serving shapes

- [ ] 20.1 Unit 20 proves all-harness/form/site authored refusals via real compilation. Verify complete causes/no provider work and typed/hands/route positives. Requirements: [Authored provider configuration cannot supply capability authority][RGR], [Known provider commands have a closed argument grammar][RGP], [Reserved hands preserves the existing workspace authority][TD6]. Reopened/remaining: operator ruling 1–2 / R10. (previous 7.3)

## 21. Unit 21 — Prove compiled cold and actual-resume restrictions

- [ ] 21.1 Unit 21 proves managed Read/empty restrictions in compiled cold/eligible-resume Claude/LaneTally shapes. Verify authored lists refuse, prompt integrity and independent inventory. Requirements: [Every accepted native control reaches the final command][NCC], [Explicit restrictive tool lists retain their meaning][NCT], [Prompt values cannot absorb a composed control][NCP], [Subtractive tool lists never grant a capability][RGS]. Reopened/remaining: operator ruling 1–2 / R10. (previous 7.4)

- [ ] 21.2 Unit 21 separately compiles held nonempty restriction cold/resume fixtures via real realm/dialect/candidate resolution. Verify independent final literals, manifest and session, no fabricated Controls. Requirements: [Restrictions are validated, carried and pinned without engine interpretation][RG4], [Every accepted native control reaches the final command][NCC], [Denial and admission have removal proofs and bounded live claims][NC6]. Reopened/remaining: operator ruling 1–2 / R10. (previous 7.6)

- [ ] 21.3 Units 20–21 map every supported launch shape to real compiled full literal/refusal assertions and selected charter facts. Verify holdings-only/is_ok do not close rows; close only after unit 21 proves the restriction rows too. Requirements: [Denial and admission have removal proofs and bounded live claims][NC6], [Every accepted native control reaches the final command][NCC], [Library charter pins are enforced at consumption][MPL]. Reopened/remaining: operator ruling 1–2 / R10. (previous 7.5)

## 22. Unit 22 — Submit whole plans to doctor

- [ ] 22.1 Unit 22 submits whole plans in both doctor paths. Verify interacting OFF/final-state conflicts and explicit adapter-only scope. Requirements: [Doctor reports grants for every realm][CD1], [Installed native capabilities absent from grants are explicit][CD2], [Restrictions are validated, carried and pinned without engine interpretation][RG4]. Reopened/remaining: operator ruling 2–4. (previous 8.3)

- [ ] 22.2 Unit 22 independently asserts full doctor/compile outcomes for all grant shapes. Remove whole-plan assessment, observe intended failure, restore/pass; no model invocation. Requirements: [Installed native capabilities absent from grants are explicit][CD2], [Unknown inventories and live-control gaps remain unmeasured][CD3], [Known native powers require a valid delivered denial or refusal][NCR]. Reopened/remaining: operator ruling 2–4. (previous 8.4)

## 23. Unit 23 — Audit launch enforcement removals

- [ ] 23.1 Unit 23 independently removes authored refusal/load parsing/final parse-state/cold-resume restriction/ON-OFF enforcement. Verify intended compiled final assertions fail, restore/pass. Requirements: [Denial and admission have removal proofs and bounded live claims][NC6], [Every accepted native control reaches the final command][NCC], [Authored provider configuration cannot supply capability authority][RGR], [Refusal proofs assert the full reason][SC8]. Reopened/remaining: operator ruling 1–4 / R10. (previous 9.2)

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
