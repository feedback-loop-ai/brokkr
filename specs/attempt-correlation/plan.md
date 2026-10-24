# Implement current-attempt admission

## Context

The authoritative behavior and scenarios are [spec.md](spec.md); the semantic proposal is [0096](../../docs/decisions/0096-attempt-correlation.md). This is design only. Existing source confirms one missing identity check, not a need for a new protocol. The design commit contains exactly these three spec files and the proposed decision. The requested `openspec/changes/attempt-correlation/design.md` is an uncommitted companion linking here: its extra commit path is refused because the sequence explicitly limits committed paths. All decisions, scenarios, risks, migration and open questions needed for review live in the committed artifacts.

## Decisions

1. **Adopt simplicity's shared owner and robustness's before-effects boundary.** Pass the current attempt into `converse`, bind both fields in each typed Body arm, and call one private helper or local closure before the arm's first effect. Effect comparison precedes attempt comparison. Use one additional immutable argument: converse currently has four parameters including self, so this repair needs no parameter object. Reject a standalone framework, new public type, trait or registry. Simplicity wins over the optional ownership abstraction: the cost is explicitly threading one additional argument, while the rejected abstraction would add API and proof obligations without another consumer. This follows 0071 rulings 1–3, 5–6 and 10.
2. **Adopt both positions' terminal and evidence semantics.** Reuse mismatch/unread/broken/finish and their existing precedence. Reject Result-only, engine-only, ignored-message and retryable-failure alternatives: they leave foreign acceptance/checkpoint evidence admitted or can repeat an uncertain effect. Their saved code is not worth a weakened refusal (0071 ruling 8; security-high or above under the house severity table).
3. **Adopt both current positions on mandatory durable proof.** Robustness requires failed Result, prior-valid acceptance/checkpoint, and reopen/replay controls; simplicity now explicitly requires them too. The earlier plan's attribution of optional replay to simplicity is superseded. There is no present panel conflict to invent. Rule against the smaller test matrix and optional replay alternative because a durable review receipt can authorize ship after restart. The cost is additional bounded fixture executions and mutation evidence; it does not justify extra production mechanisms. Avoid broad chaos campaigns, thousands of processes, unrelated crash/TLS tests and new resource admission guarantees. Existing teardown/limit suites remain authoritative (0071 rulings 4, 5 and 9).
4. **Adopt simplicity's bounded scope; reject unrelated charter requirements.** No schema, ordering, duplicate-Accepted, resume, cancellation, replay-cache, aggregate-memory, authentication or adapter rewrite. Robustness's concurrency/recovery observations become risks and replay assertions, not new mutable state. Accepted cost: copying both current IDs still allows an authorized malicious producer to fabricate a receipt. SMTP charter requirements are inapplicable to this repository.
5. **Keep typed variants and one evidence implementation.** Reuse Body, Ended and AttemptOutcome; introduce no string-dispatched error vocabulary or `Result<_, String>`. Pin complete expected text in a focused module rather than derive expected evidence through the production helper under test. Expected engine-owned IDs may remain quoted. A separate redaction implementation would drift (0071 rulings 3, 5 and 8).
6. **Keep proof local without growing oversized files.** Dedicated test modules are justified. Current source counts: process.rs 773, process/tests.rs 1,979, engine.rs 3,316 and engine/tests.rs 6,468. Oversized parents must not grow, including registration lines. Prefer nesting focused tests under the existing test module and removing/moving a small coherent helper if needed; reuse its private machine/wire builders. Do not hide these tests in an unrelated suite or perform a wholesale split. House gates decide measured ceilings (0071 ruling 4).

7. **Preserve the continuation and reconcile the returned artifact finding.** Keep the same commission, slug and proposed decision 0096. Both positions and this synthesis observe that 400aee25 already contains exactly `**Status:** proposed`; preserve that header and correct stale return prose. The older validation note describes a superseded conventional-header attempt, not the current artifact. Do not allocate a replacement decision, infer acceptance, reset the failed-run bound or alter a gate. Preserve the design commits while the downstream executor establishes current origin/main. Local origin/main alone is not freshness evidence. The final pinned check owns the design verdict.

## Implementation approach and files

| File | Planned change and reason |
| --- | --- |
| `crates/brokkr-protocol/src/process.rs` | Thread attempt_id from run_attempt_resuming to converse; bind it in three Body arms; centralize effect-first pair comparison and return existing Unread before any admission. Keep the file under 800 lines and functions within their ceilings. |
| `crates/brokkr-protocol/src/process/tests/attempt_correlation.rs` (new) | Focused typed scenario matrix, full report/callback assertions, positive controls, effect precedence and content-free evidence. |
| `crates/brokkr-protocol/src/process/tests.rs` | Minimal child-module registration and fixture sharing; keep under 2,000 lines, relocating helpers coherently only if needed. |
| `crates/brokkr-runtime/src/engine/tests/attempt_correlation.rs` (new) | Paired real-transport review-to-ship regression, actual Start identity capture, terminal/journal assertions and replay. |
| `crates/brokkr-runtime/src/engine/tests.rs` | Register child without net growth; move a small coherent fixture section if necessary and use the existing machine rather than copy its policy. |

`process/evidence.rs` is reused unchanged; the existing mismatch helper already supplies all required evidence. Runtime engine.rs, engine/panel.rs and protocol oneshot.rs are consumer inspection/regression surfaces, not new admission owners. Core, view, adapter schemas, manifests, agent/recipe definitions, Cargo files and frozen files need no production changes. Exact test-module placement may adapt to gates, but ownership and no-growth constraints do not. Cut the speculative third helper file: start with the two focused child modules and existing builders. Extract a coherent fixture helper only if actual duplication or a measured ceiling requires it, recording that evidence; do not add a production test seam or dependency (0071 rulings 4–6).

## Test construction

Use a shared typed scenario builder for current versus foreign attempt, with a distinctive marker in the foreign field. Accepted and Checkpoint negatives end with an otherwise valid current successful Result so removal of their own guard produces success or observable foreign evidence, not another accidental refusal. Test both no prior acceptance and prior legitimate evidence. For Result, exercise both success and failed status with concrete payload/error. Add both-foreign controls for each message kind; retain existing effect-only tests unchanged.

Assert the entire relevant typed outcome/accounting, settled cleanup and exact callback data under controlled conditions. Serialize outcome and report fields using supported typed serialization for disclosure assertions; do not add Serialize/public API solely for the test. Pin evidence literals independently, including the existing post-Start suffix. Do not branch on error text. Include matching controls for the three kinds and existing failed-result behavior.

The runtime fixture obtains the actual effect/attempt pair from Start, changes only the review Result attempt, and uses valid work and ship receipts. Use the same builder for the paired positive control. Assert the current review attempt from EffectStarted against its single EffectIndeterminate; enumerate absence of forbidden terminal/transition/phase/invocation/completion facts, and assert an isolated ship invocation marker absent. Preserve legitimate review checkpoint evidence exactly and exclude the foreign marker from the journal. Read committed checkpoint events, not merely callback counts: engine/checkpoints.rs may hold and retry telemetry before settlement. Preserve its existing rule that stranded checkpoints prevent success; no sink or store rewrite belongs in this repair. Close/reopen the store and fold/replay to the same parked phase/status without invoking operator retry. The positive run must take REVIEW and exercise ship, proving fixture reachability.

Use bounded handshakes and isolated temp paths instead of sleeps or shared environment. Use existing engine isolation helpers. Any environment mutation requires the existing unwind-safe guard. No Windows-specific code or evidence.

Record compiling mutations independently bypassing Accepted, Checkpoint and Result admission; the last must fail the engine negative too. Record a mutation leaking held attempt bytes and controls removing positive data propagation/replay behavior each new test claims. Every test maps to at least one actual failing assertion. Restore the code before validation and retain commands, test names, exit status and failure excerpts in run-local evidence.

## Validation and evidence

Before implementation, establish current Brokkr origin/main through authorized remote access, preserve these design commits when updating the base, and record the resulting base and implementation revisions. Remote freshness is unverified in this design seat; if unavailable, keep that prerequisite pending rather than claim the local tracking ref is current.

Implementation must run, in order after focused tests/removal controls:

1. `cargo fmt --all -- --check`.
2. `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`.
3. `cargo test --workspace --all-features --locked` (the required workspace suite with all features).
4. `cargo run --locked -p brokkr-cli -- compile --bundle recipes/self` and the corresponding `recipes/verify` compile.
5. `bash scripts/lint-non-rust.sh` plus existing applicable recipe, architecture, size/complexity and clone gates. Missing pinned tools or skipped rows remain pending, not clean.
6. Authorized host validation/CI OUTSIDE the workspace box runs `bash scripts/coverage-exact.sh`. Retain its full log and generated coverage proof under `.forge/artifacts/brokkr-engine-security-repair-co-3695cf13/coverage/`, with implementation revision, restored working-tree status, pinned toolchain, exact command and exit code. Preserve literal line/branch/function coverage and all denominator/exclusion rules; no gate edits, baselines inflated or coverage waivers.
7. Review the diff for frozen-byte identity, no temporary mutations, required crew results and unsigned commit scope. These implementation checks are pending in design. Remote CI, when required for delivery, remains pending until its actual result exists; no publication is authorized.

Only workspace capability is available to this design seat. It cannot supply outside-box coverage proof or run the downstream model crew. The downstream executor must obtain authorized execution and report the real result. Absence of those capabilities never becomes a claimed pass.

## Risks

| Risk | Mitigation and accepted cost |
| --- | --- |
| A guard is missing from one message arm or occurs after its callback | Independent per-arm removal controls plus exact callback/accounting assertions; shared helper alone is not proof. |
| False safety from a broken driver fixture, guessed ID or EOF | Actual Start identity capture, valid trailing Result in intermediate negatives, and paired positive engine run reaching ship. |
| A clean foreign review result is durably consumed | Assert one current-attempt indeterminate fact, no forbidden transition/execution, and parked replay after reopen. |
| Mismatch diagnostics leak foreign content forever | Reuse content-free Evidence; independently pin format and scan serialized evidence/journal for the planted marker; execute leak mutation. |
| Concurrent/resumed invocations use the wrong expected attempt | Invocation-local immutable arguments only; session continuity grants no attempt authority. No shared lookup or cache. Existing per-transport caps and journal fencing remain; no aggregate memory or thousand-session capacity guarantee is added. |
| Callback delivery is confused with durable checkpoint storage | Assert exact committed checkpoint events before and after reopen; keep sink retry/settlement and stranded-checkpoint refusal unchanged. |
| Refusal changes retry/cleanup/limit precedence | Preserve existing end/broken/finish paths and existing limit/cleanup suites. Controlled tests isolate mismatch text; uncertainty still parks. |
| Crash loses uncommitted telemetry or old vulnerable records remain | Preserve append-before-execute and append-only recovery; do not infer success, rewrite history or claim exactly-once external effects. |
| Test additions exceed size/clone ratchets | Small child modules, reuse builders, no net growth of oversized parents, measure gates; no new suppressions. |
| Equality and byte evidence are mistaken for authentication or secrecy | A producer that copies Start can forge both IDs; hashes of guessable IDs permit guessing. This repair prevents copying refused content, not producer forgery or independent stderr leakage. |
| Strict enforcement exposes stale-attempt drivers | Park through the existing path and correct drivers; no compatibility bypass. |
| Model pins or outside-box coverage unavailable | Record pending with concrete reason and retained partial evidence; no silent model substitution, denominator changes or completion claim. |

## Migration

No wire, storage or journal migration. Both identities already exist in the frozen v1 schema. Matching drivers continue unchanged. A resumed attempt must use the new Start pair, not the offered session's prior pair. Earlier successful receipts are historical evidence only.

## Open questions and routing

No unresolved semantic choice blocks the local repair. Decision 0096 awaits operator acceptance. Decision 0058 has a proposed top-level status and an operator-ruled overlay addendum; do not edit it or infer whole-document acceptance. The explicit requested route is Codex implementation; Astra correctness/guarantees; official DeepSeek Flash security; Sonnet 5.5 tests; Fable 5.1 independent review. Never DeepSeek Pro. Review pin availability and authorized host coverage are execution dependencies to report honestly, not reasons to change the design.

## Design retry validation

Retain proposal 0096 from the returned attempt; do not allocate 0097 to evade its status finding. Use the exact pinned-checker header `**Status:** proposed`, preserve the spec declaration and relative link, and verify committed bytes as well as the worktree. Stage only `specs/attempt-correlation/spec.md`, `plan.md`, `tasks.md` and `docs/decisions/0096-attempt-correlation.md`, then commit unsigned. Check that HEAD touches all four paths and no others, that exactly one spec directory is touched, and that each file is nonempty and clean. Keep the OpenSpec companion coherent but uncommitted under the explicit commit-scope restriction. The pinned sequence validator is not present in this seat's visible checkout; record that limitation and defer its verdict to the final check. Structural inspection is not a substitute verdict.
