# Codex 0.156 boxed-seat hands discovery — implementation tasks

This is the tasks-phase draft for `2026-09-24-codex-0156-boxed-seat`, based on
[the capability delta](specs/boxed-hands-discovery/spec.md) and
[design D1–D11](design.md#decisions). No implementation task is complete yet.
The design has no blocking open question; no upstream artifact needs changing.
There is no `returned_from` finding on this visit.

Execute numbered tasks in order and tick each only after its stated check
passes. Tests accompany the code they prove; group 7 checks the combined
production path. Production changes stay in Rust under `crates/`; temporary
fixtures stay outside frozen `fixtures/`. Do not add a provider call, workflow
runner, dependency, recipe control, launch change, resume qualification,
release/version bump or Windows obligation. Supported hosts are Linux and
macOS (0063). Decisions remain `proposed` until the operator accepts them.

Use `.forge/tasks/codex-0156-boxed-seat-proof.md` for measurements and mutation
evidence. Every mutation task below requires: unchanged test expectations;
a recorded production diff and exact command; successful compilation; the
named test's relevant failing assertion; restoration of that diff; and a
passing rerun of the same test. A build error, unrelated failure or predicted
failure is not a kill. If a mutation serves multiple claims, observe and name
each failing case separately rather than crediting assertions after the first
panic. Restore each mutant before proceeding to the next.

Cargo and rustup were absent from the tasks seat's boxed PATH, as in the prior
visits. The implementation must obtain actual toolchain results; these tasks
do not grant an exception. Exact coverage additionally needs host/CI namespace
execution. An unavailable check remains pending and its checkbox remains open;
implementation completion and archive wait for the required green evidence.

Tasks-visit validation (2026-09-24): strict OpenSpec validation passes; its
status reports the planning artifacts done, not implementation completion.
The checklist audit confirms 39 unchecked tasks in 11 sequential groups,
each naming a requirement and its verification, covering all six requirements.
Workspace tests, formatting, clippy and self-bundle compilation were attempted
through workspace hands and each exited 127 (`cargo: command not found`).
The unchanged exact-coverage script also exited 127 at its Cargo invocation
(line 33), producing no coverage result. Those checks remain pending; this
visit changes only the task breakdown and claims no runtime or mutation proof.

Implementation visit (2026-09-24): Cargo 1.98.0 was on this seat's PATH,
and every check below was actually run. The semantic decision is
[0069](../../../docs/decisions/0069-a-boxed-seat-is-told-how-to-find-its-hands.md)
(`Status: proposed`; 0066 is unclaimed on main but skipped, since numbers
collide when claimed out of order). The measured identities, the scenario
map and all 40 restored compiling mutations are recorded in the run-local
ledger `.forge/tasks/codex-0156-boxed-seat-proof.md`. Its essentials are
also in the decision's enforcement bindings and in the pin comments of
`tests/witness_digests.rs` and `bundle/compose_tests.rs`.

Exact coverage (task 10.3) ran on the host with the pinned nightly, over the
final implementation source before the archive: the unchanged
`bash scripts/coverage-exact.sh` exited 0 with lines 32580/32580, branches
5540/5540 and functions 3181/3181. A first run had refused two new-code gaps.
Those were closed by removing an unreachable guard and an unreachable
branch, and mutation E4 was re-bound against the new form.

## 1. Establish the before-state and proof ledger

- [x] 1.1 Create the task-local proof record with a mapping from every delta scenario and design D10 protection to its task, eventual test and mutation. Mark unperformed checks pending. Verify the delivered ledger accounts for all six named requirements and the inline, relocation and custom-name scenarios in D3–D6. **Requirement: Every discovery guarantee has deterministic proof and restored mutation evidence.**
- [x] 1.2 Before production or adapter edits, compile every existing witness and compose case using the Rust suites (`cargo test --locked -p brokkr-runtime --test witness_digests -- --nocapture` and `cargo test --locked -p brokkr-runtime bundle::compose_tests -- --nocapture`). Record actual per-bundle digests and commands, including Codex fallback consumers and unaffected cases; use a temporary measurement helper if needed rather than copying expected hashes as observations. Capture base `aa58d07a` frozen-surface and Codex adapter bytes for later comparisons. Verify the recorded baseline is reproducible before continuing. **Requirement: Adapter byte changes receive measured identity updates.**

## 2. Add the narrow declaration and loader proof

- [x] 2.1 Add the dedicated shared `HandsNotice` type and byte-based validation to `crates/brokkr-protocol/src/adapters.rs` (D2), distinct from capability `Notice`, with only `workspace_tool` and `discovery_tool` in its private serialization. Add passing exact-value/diagnostic tests for object shape, both required string fields, unknown members, valid ASCII identifiers at 1/128 bytes, and invalid empty/129-byte, leading-digit, whitespace, newline, punctuation and non-ASCII values in either field. Verify no regex dependency or public schema is added. **Requirement: A provider declares only the tool identifiers needed for discovery.**
- [x] 2.2 Extend `agents/load.rs::parse_adapter` and `agents.rs::Adapter` with optional validated `hands.notice`. Check presence before decoding: omitted is `None`; explicit null, false, string, array, missing/wrong-type/extra members refuse. Require supported, nonempty `hands.workspace`; retain legacy empty fragments when notice is absent. Extend `agents/tests.rs` with exact provider/field/reason diagnostics and retained values for every case, including unsupported hands and missing/empty workspace. Verify the targeted loader suite passes without changing existing launch/harness behavior. **Requirement: A provider declares only the tool identifiers needed for discovery.**

## 3. Carry provider facts through compilation

- [x] 3.1 Carry `Option<HandsNotice>` from each adapter through `agents.rs::resolve_report` into each `Candidate` and through `bundle.rs` candidate reconstruction after `expand_command` (D3). Set the discarded policy-only projection to `None`; keep `ChainEntry`, resolution reports and serialized contracts unchanged. Extend resolver/bundle tests to assert distinct candidate declarations survive reconstruction exactly and absent metadata stays absent. Verify the targeted suites pass. **Requirement: Only an actually boxed seat receives its selected provider's notice.**
- [x] 3.2 Make optional inline adapter loading return an error-preserving result (D3), without `.ok()` error erasure or diagnostic-string classification. Add passing compile tests distinguishing no inline consumer, absent optional adapter directory, valid library missing the provider, and valid adapter missing the notice from a present invalid library/notice, whose exact normal loader diagnostic must propagate. Verify agents/gates/secrets/dialects retain their mandatory loading refusals and custom drivers gain no inferred association. **Requirement: A provider declares only the tool identifiers needed for discovery.**
- [x] 3.3 Collect inline built-in discovery metadata into canonical `SiteFacts::inline_hands_notice`, separately from `inline_resume`, and preserve it when site labels relocate under the dialect verify wrapper. Bind consumed adapters through the existing `pin_drivers`/manifest `drivers` witness even with no resume qualification. Add passing compiler tests for exact metadata, relocated labels, witnesses and unchanged unqualified resume facts; ensure no runtime adapter reopen or new manifest member is introduced. **Requirements: Only an actually boxed seat receives its selected provider's notice; Adapter byte changes receive measured identity updates.**

## 4. Assemble guidance for the actual execution attempt

- [x] 4.1 Add the engine helper beside `mark_hands`/`mark_delivery` (D4): clear the private `hands_notice` first, then use canonical resolved workspace hands, `Boundary::is_boxed()` and the selected candidate's declaration. Consult inline site metadata only when no candidate exists; an absent candidate notice cannot fall through to inline data. Add passing exact carrier tests for namespace/seatbelt/container data, harness/open, no-hands, unknown/unregistered facts, missing declarations and seeded stale values. Verify those tests pass and retain exact runtime refusals for unbuilt boundaries. **Requirement: Only an actually boxed seat receives its selected provider's notice.**
- [x] 4.2 Invoke that helper after ordinary context assembly and requested-input digest verification at the actual single/selected-body label, each `member_runs` member and each single sequence step; nested panels use their member labels (D4). Keep it out of durable `seat_input`; non-executing containers and dialect exec steps supply no model guidance. Extend engine tests to inspect independently assembled inputs at each path and verify requested input remains byte-identical across notice changes. **Requirement: Only an actually boxed seat receives its selected provider's notice.**

## 5. Render the fixed engine contract

- [x] 5.1 Render D5's exact discovery paragraph inside the mandatory result contract after the generic hands paragraph, substituting only validated identifiers when the model invocation has the applicable boxed-hands facts. Extend protocol tests with an independent literal expectation containing `mcp__brokkr__workspace`, conditional `tool_search` loading, native shell/apply_patch refusal by design, “not a blocker”, and all workspace writes including the result file. Verify exactly one engine paragraph, the exact result path/object contract, and zero discovery prose for absent/invalid carriers and exec, even with a synthetic carrier. **Requirements: Only an actually boxed seat receives its selected provider's notice; Discovery guidance is engine-owned and exposes no other configuration.**
- [x] 5.2 Make the generic boxed paragraph use the same validated applicable workspace identifier, defaulting to its existing literal when no notice applies (D5). Add passing exact contract tests for `fixture_workspace`/`fixture_search`, no contradictory legacy workspace name, and removal of the declaration restoring the baseline. Verify those tests pass and assert shipped Codex generic prose and boxed Claude guidance remain byte-identical, with existing harness/open, no-hands and harness-gate last-message instructions intact. **Requirement: A provider declares only the tool identifiers needed for discovery.**

## 6. Ship the declaration and dated observation

- [x] 6.1 Add only the specified `hands.notice` identifiers to `adapters/codex.json` now that the loader can read them; leave Claude without a notice. Extend shipped-adapter tests to assert the two exact strings and absence on Claude, and compare Codex's workspace server attachment, read-only sandbox, MCP approval and harness fragments with the before-state. Verify these adapter/resolution tests pass with unchanged composed launch arguments. **Requirement: A provider declares only the tool identifiers needed for discovery.**
- [x] 6.2 Append operator-supplied 2026-09-24 hands-discovery measurements to Codex's existing limitations, each within 400 characters, preserving historical entries verbatim (D7). Cover installed 0.156.0 versus qualified 0.154.0; adapter-equivalent attachment; gpt-5.6-sol/gpt-6-sol no-tool probes; reported `removed, true`; both ineffective false toggles; and no measured per-server switch. Record only supplied six-arm wager outcomes: four searches/225 workspace calls/delivery for gpt-5.6-sol, no searches or result files after native-write failures for gpt-6-sol/gpt-6-luna, and gpt-5.6-luna's brief discovery then apply_patch fallback. Verify exact dated/provenance content with tests and review against the delta, without claiming unreported arms or new probes. **Requirement: Dated hands evidence does not requalify Codex resume.**
- [x] 6.3 Explicitly state in the added limitations that the observation does not qualify 0.156.0 resume and task 11.1 still awaits its operator ruling. Add passing exact preservation assertions against the before-state for identity `{"version":"0.154.0","applies_to":"0.154.0"}`, status, classes, boundaries, hands coordinate, historical evidence, qualified argv and eligibility behavior. Verify the new preservation tests and relevant existing resume-refusal tests pass, including boxed hands remaining unqualified; do not add a resume evidence axis or relax its validator. **Requirement: Dated hands evidence does not requalify Codex resume.**

## 7. Prove the combined production path without a provider

Use existing controlled driver seams to capture production dispatch input and
invoke the real renderer. Tests below load adapters, ingest actual temporary
recipes, compile and dispatch; direct construction of trusted renderer input
alone cannot discharge these tasks. Compare a complete expected prompt or an
independent literal contract suffix and the assembled carrier. Authored prose
may reproduce headings: never split on its first occurrence to infer ownership.

- [x] 7.1 Add an integrated shipped-adapter proof of exactly one complete notice for boxed Codex and zero engine notices for boxed Claude, harness Codex, open Codex, Codex without hands and exec. Verify generic boxed guidance, exact result-file contract and the separately admitted harness gate's last-message door remain correct. Use canonical boundary data tests for unbuilt boundaries; verify the focused tests pass without spawning a provider or claiming live boundary execution. **Requirement: Only an actually boxed seat receives its selected provider's notice.**
- [x] 7.2 Exercise admissible failure-to-start fallback in both directions, Codex→Claude and Claude→Codex. Assert the actual selected provider's exact paragraph/absence per attempt, clearing of prior metadata, unchanged requested digest and absence of a different-effect refusal. Verify an already-started model's discovery failure does not acquire a new fallback category and the targeted engine tests pass. **Requirement: Only an actually boxed seat receives its selected provider's notice.**
- [x] 7.3 Exercise single, selected strategy body, mixed panel, single sequence step and nested panel dispatch with notice-bearing and notice-free providers. Inspect each site's exact carrier and contract, including seeded stale data, parent/container facts, unselected bodies and sibling providers. Verify all paths pass with no notice inheritance or cross-site leakage. **Requirement: Only an actually boxed seat receives its selected provider's notice.**
- [x] 7.4 Exercise admitted inline built-in boxed Codex through compilation and dispatch, including dialect verify relocation; assert its exact notice and consumed adapter witness at the relocated label with no new resume qualification. Verify absent optional libraries, missing provider/notice and unassociated custom drivers preserve their allowed behavior, while a present malformed notice yields the exact compile refusal. **Requirements: Only an actually boxed seat receives its selected provider's notice; Adapter byte changes receive measured identity updates.**
- [x] 7.5 At actual recipe/site/agent-override/input ingestion boundaries, attempt `hands.notice` and the private `hands_notice` with false, null and forged identifiers. Add exact structural/closed-input refusal assertions; exercise otherwise legal declared inputs and prior-result/context propagation to prove they cannot replace or suppress the carrier or forge provider/hands/boundary facts. Verify applicable Codex remains exact and those inputs cannot create guidance for boxed Claude, unboxed Codex or no-hands sites. **Requirement: Discovery guidance is engine-owned and exposes no other configuration.**
- [x] 7.6 Compile and dispatch valid recipes with a silent charter, instructions to omit guidance, and notice-like quotes in charter, house, feature and context, including a duplicate result-contract heading and false provider/hands/boundary claims. Assert the independent engine contract and canonical carrier remain unchanged for applicable Codex and remain notice-free for the negative sites. Verify every case passes without censoring ordinary authored text or claiming model obedience. **Requirement: Discovery guidance is engine-owned and exposes no other configuration.**
- [x] 7.7 Through real loading/resolution/dispatch, test a separately named temporary provider declaring `fixture_workspace`/`fixture_search`, plus a Codex-shaped adapter with no notice. Assert exact custom/absent guidance and consistent generic naming. Put distinct sentinels in unrelated valid adapter evidence and launch/MCP configuration; verify every sentinel is absent from the whole prompt while baseline workdir, result path, house and task retain their roles. **Requirements: A provider declares only the tool identifiers needed for discovery; Discovery guidance is engine-owned and exposes no other configuration.**

## 8. Reconcile identities and document the semantic choice

- [x] 8.1 Recompile the full witness/compose set and record every before/after manifest digest beside task 1.2, including a Claude-served chain consulting Codex only as fallback and an inline Codex consumer. Update only measured changed pins in `crates/brokkr-runtime/tests/witness_digests.rs`, `crates/brokkr-runtime/src/bundle/compose_tests.rs` and any demonstrated additional consumers, with the adapter-byte cause. Verify both suites pass and unaffected pins remain identical; no guessed hashes, global replacements, engine bump or recipe edits. **Requirement: Adapter byte changes receive measured identity updates.**
- [x] 8.2 Add temporary-fixture identity tests that separately vary notice bytes and dated evidence bytes, asserting consumer digest changes (including fallback-only and inline consumers) while unrelated identities stay fixed. Preserve the exact old-run manifest mismatch refusal. Verify these tests and the resume preservation tests pass together, distinguishing byte identity from the still-0.154.0 qualification. **Requirement: Adapter byte changes receive measured identity updates.**
- [x] 8.3 Write the next available numbered semantic decision and its index entry with `Status: proposed`, D1–D11's ownership/applicability choices, alternatives and named tests as enforcement bindings. Explain adapter facts versus engine selection/renderer wording, rejection of charter/recipe control, bounded custom-name substitution, inline invalid-data refusal, measured identities and the separate resume ruling. Verify the decision/index links resolve and no acceptance is claimed. **Requirement: Every discovery guarantee has deterministic proof and restored mutation evidence.**
- [x] 8.4 Update `docs/guides/provider-adapters.md` narrowly with the optional closed declaration, identifier grammar/limits, nonempty workspace prerequisite, boxed selected-provider applicability, engine-owned prose and absence compatibility. Describe co-distribution with the loader (older loaders reject the new key), measured digest movement and rollback of code/adapter/pins together. Verify its example loads in the adapter suite and its claims match the spec and proposed decision, including no resume requalification. **Requirement: A provider declares only the tool identifiers needed for discovery.**

## 9. Bind each protection to restored compiling mutations

Every checkbox in this group uses the record/restore/rerun protocol above and
serves **Every discovery guarantee has deterministic proof and restored mutation
evidence** in addition to the behavior named in the checkbox. Add any missing
targeted assertion before claiming its mutation; do not alter expectations to
make a mutant fail or survive.

- [x] 9.1 Mutate declaration validation to admit malformed shape/type/identifier data or a notice without compatible workspace hands; also erase the present-invalid inline load error. Verify each claimed validation protection is killed by its exact loader/compile diagnostic assertion from groups 2–3, and record restored passing reruns. **Requirement: A provider declares only the tool identifiers needed for discovery.**
- [x] 9.2 Mutate candidate reconstruction to drop a loaded notice; separately drop inline collection, relocation or its adapter witness. Verify the corresponding production-path notice, label or identity assertion from groups 3/7 fails after successful compilation, then passes on restoration for every claimed propagation path. **Requirements: Only an actually boxed seat receives its selected provider's notice; Adapter byte changes receive measured identity updates.**
- [x] 9.3 Mutate the paragraph away and mutate each asserted element: workspace identifier, discovery identifier/conditional loading, native-refusal-by-design explanation, “not a blocker”, and workspace/result-file write direction. Verify the relevant independent literal assertion fails for each recorded wording claim and passes after restoration. **Requirement: Only an actually boxed seat receives its selected provider's notice.**
- [x] 9.4 Mutate applicability to produce guidance for boxed Claude, harness Codex, open Codex, no-hands, unresolved/unknown/unregistered sites and exec, covering every relevant guard separately. Verify each named negative case reaches and fails its own exact absence/contract assertion; restore and rerun each case green. **Requirement: Only an actually boxed seat receives its selected provider's notice.**
- [x] 9.5 Mutate selection to use an unselected/first candidate, and separately place notice stamping into the requested input/digest path. Verify both fallback directions expose the wrong notice or different-effect/digest error through the relevant assertions; restore and rerun the affected tests green without changing fallback policy. **Requirement: Only an actually boxed seat receives its selected provider's notice.**
- [x] 9.6 Mutate executing labels/selections to a parent or sibling and retain a stale carrier where it should clear, including a selected candidate with no notice improperly falling through to inline metadata. Verify selected-body, panel, sequence and nested-panel isolation protections each have an observed relevant assertion failure, then restored passing reruns. **Requirement: Only an actually boxed seat receives its selected provider's notice.**
- [x] 9.7 Mutate data-driven discovery into a Codex/provider-name rule and separately restore the old literal only in the custom provider's generic paragraph. Verify custom-provider, Codex-without-declaration and consistent-workspace-name assertions fail as applicable and pass after restoration. **Requirement: A provider declares only the tool identifiers needed for discovery.**
- [x] 9.8 Mutate the production authority boundary to accept injected recipe/input metadata, honor a suppression value, or require charter guidance. Verify replacement, suppression and silent-charter protections through the actual ingestion/propagation tests, including false/null/forged and quoted-text cases, observing each relevant exact refusal/carrier/contract failure before restoration and a green rerun. **Requirement: Discovery guidance is engine-owned and exposes no other configuration.**
- [x] 9.9 Mutate propagation/rendering to echo unrelated configuration or evidence into the prompt. Verify each claimed sentinel-exclusion assertion fails in the whole-prompt proof and passes after restoration; a renderer-only synthetic input does not replace the integrated evidence. **Requirement: Discovery guidance is engine-owned and exposes no other configuration.**
- [x] 9.10 Mutate consumed-adapter identity retention and the production qualification facts/eligibility guarded by the new preservation assertions. Verify fallback-only/inline byte-witness, unaffected-identity and historical qualification claims have targeted compiling kills and restored green tests; never change the historical expected 0.154.0 values or count a compile refusal as a kill. **Requirements: Adapter byte changes receive measured identity updates; Dated hands evidence does not requalify Codex resume.**

## 10. Close proof coverage and run the house gates

- [x] 10.1 Reconcile every delta/design scenario against the final test/mutation ledger, with no unobserved kills and every mutant restored. Review exact paragraph/absence/fact/diagnostic assertions (no `is_err()`), proposed decision bindings and scope. Verify `git diff --exit-code aa58d07a -- contracts/ policy/phase-machine.json policy/schemas/ fixtures/ reference/ extensions/` is clean, unaffected pins/qualification remain fixed, and the feature diff contains only authorized changes. **Requirements: Every discovery guarantee has deterministic proof and restored mutation evidence; Adapter byte changes receive measured identity updates.**
- [x] 10.2 Run and record `cargo test --workspace --all-features --locked`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, `cargo run --locked -p brokkr-cli -- compile --bundle bundles/self`, and `openspec validate 2026-09-24-codex-0156-boxed-seat --strict --no-interactive`. Verify all exit successfully on the restored implementation; fix failures and rerun affected checks before ticking. Record any tooling failure honestly as pending, with no success claim. **Requirement: Every discovery guarantee has deterministic proof and restored mutation evidence.**
- [x] 10.3 Run the unchanged `bash scripts/coverage-exact.sh` using `rust-nightly-version.txt` and record an actual clean exact-coverage result for the final implementation source. If the workspace box cannot create namespaces, obtain host/CI validation and keep this task pending until that result exists; no ignored boundary tests, exclusions or lower threshold. Verify the evidence identifies the checked source and command without adding a live provider call, workflow-runner invocation or Windows obligation. **Requirement: Every discovery guarantee has deterministic proof and restored mutation evidence.**

## 11. Fold the living truth and commit — final task

- [x] 11.1 After all preceding tasks pass and are ticked, run the declared dialect operation `openspec archive 2026-09-24-codex-0156-boxed-seat --yes`. Verify it moves the change under `openspec/changes/archive/` and folds its delta into `openspec/specs/boxed-hands-discovery/spec.md`. Append exactly one ``- `<actual-archived-directory-name>` — folded <YYYY-MM-DD>`` line under the capability's final `## Provenance` heading using the real fold date; preserve prior provenance verbatim. Tick this task in the moved tasks file, run `openspec validate --archived --strict --no-interactive` and `git diff --check`, and verify every checkbox is finished with green test/gate evidence. Commit all completed implementation, evidence, pin, decision/documentation and archive changes using repository message style, verify the commit and clean worktree, and report its hash. Never push. **Requirement: Every discovery guarantee has deterministic proof and restored mutation evidence; dialect obligation: OpenSpec archive.**
