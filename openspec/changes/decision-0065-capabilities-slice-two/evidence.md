# Slice-two specification evidence

## Scope and source

This is the single durable record of specification visits, council dispositions
and document validation for `decision-0065-capabilities-slice-two`. Historical
commit labels identify observations; no reader needs those commits to recover
the reasoning. Refer to this file and its sections after squash. It contains
no executed U0 or runtime proof.

Branch `slice-0065-two-spec` starts at main `2a23488b`; slice one was on main
at `7c92e45d`. Original run: `build-decision-0065-slice-two-sp-cd8dd603`, recipe
`claude-flash-dsh-adv`. Repair run: `build-decision-0065-slice-two-re-7ac921ae`.
The repair's specify seat adopted HEAD `34b34836`. Its supplied journal named
specify after triage's design result, with no `returned_from` field. The
operator commission supplies the held findings below. Only the engine selects
routing; dependent repairs do not claim that later seats executed or assented.

R1–R5 stay verbatim in [the operator record](operator-ruling-2026-10-03.md).
0077 remains proposed. Owning review corrected the initial realms v7/v2 dialect
choices to the next realms version after v7 and frozen tool-dialect v1. The
first draft's unavailable local signing/format/typos obligations gave way to
the authorized unsigned commit and external operator handoff.

## Visit chronology

All visits below were recorded on 2026-10-03. Counts describe their snapshots,
not the repaired plan. The records from the first council onward reported
clean diff checks and strict OpenSpec 20 passed, zero failed, with informational
notices. None claimed runtime proof.

| Historical label | Recorded result and change |
| --- | --- |
| `f1e9443b` | Initial bounded design, adopted by the first council below. |
| `7aa9e9ff` council | Upstream: secret reads/startup, all-writer custody, confirmed settlement, correlation/budgets and excess DATA/public-lifecycle requirements. 26 requirements, 97 scenarios, 48 PRs, 100 open tasks; eleven files repaired. |
| `7f36e922` specify | Drafted: adopted owning repairs, clarified durable acceptance and bounded identity. 26 requirements, 100 scenarios, 48 PRs, 100 tasks, 56 production paths. Clarify subsequently reported clear for this scope. |
| `b310e094` council | Upstream: unsafe numeric masking conflicted with the broker promise; failed-session judgment was missing. 26 requirements, 106 scenarios, 44 PRs, 100 tasks, 55 paths, 521 links. |
| `1c96ca2c` specify | Drafted: adopted masking/session repairs after checking secret.rs, secrets.md, process.rs and checkpoints.rs. 26 requirements, 107 scenarios, 44 PRs, 100 tasks, 55 paths, 521 links. Clarify subsequently reported clear for this scope. |
| `f80e14f2` council | Drafted: startup binding collisions explicit; discovery combined with selection/rendering. 26 requirements, 108 scenarios, 43 PRs, 100 tasks, 55 paths, 530 links. |
| `34b34836` tasks | Added fold task 49.3 and explicit launch-environment/integrated gate proofs. 26 requirements, 108 scenarios, 43 PRs, 101 tasks, 55 paths, 540 links. |
| `b226f8ca` repair specify | Adopted the held change; added U1f2 scaffold migration, delayed emission in U4f2, native boundary proofs and this durable record. 26 requirements, 113 scenarios, 45 PRs, 105 open tasks. |
| Repair clarify and design | Supplied journal records clarify clear and robustness/simplicity positions produced. Design adopted the repaired draft and reconciled both full positions below, retaining the plan and making the independent reviewer proof explicit. |

## Reservation evidence

Earlier offices recorded checking titles, bodies, full file lists and patches
for open PRs #404, #452, #460, #486, #494 and #500 on 2026-10-03; none claimed
0077. The local gap table reserved 0072/0074/0075, and 0076 existed. A later
refresh failed: gh lacked authentication and the public API hostname did not
resolve. This repair makes no new remote-check claim; recheck concurrent
claims before implementation. The operator assigns realms v7 to #487; PR
#494's earlier v6 wording has no reservation authority. Open #467 is 0077's
strictness prerequisite; the tasks name all work. Closed slice-one PR #319
is historical delivery, not slice-two work.

## First council reconciliation

This preserves the complete substantive first reconciliation, subsequently
adopted at the owning specification review.

| Claim / source | Disposition and evidence |
| --- | --- |
| R1 and map 1: harness owns the child, final proof is singleton | Adopt. Proposed 0077 amends the words "launched by the engine"; use one engine-configured broker child per holding, never an engine sibling outside #403. |
| R2 and map 1: existing box channel only | Adopt namespace/workspace-hands/network-false checks. A wanted request cannot excuse a hard site refusal; provider carriage still follows 0065 ruling 5. |
| R3 and first draft: Codex isolation unproven | Adopt measurement-first candidates. Reject choosing a private Codex home or table replacement from source inspection alone. |
| R4 and map 2: no artifact store and only engine writes journal | Combine protected broker ledgers with engine publication/fold. Reject a writable worktree ledger or broker SQLite writer. |
| R5: one PR per narrow unit, inert until enabling | Adopt. Interpret inert as no compiled MCP execution. U1–U4 can independently correct strictness, panics, native gates and native recording. |
| Draft: realms v7 veto | Amend proposal/SC2/CR1 first: use the next realms version after v7, preserving #487's fields. |
| Draft: tool-dialect v2 mandatory | Reject under 0071 ruling 6. Frozen v1 already has every needed field. Preserve its syntax and distinguish valid-but-unexecutable URL/reference connections through SC1 scenarios. |
| Draft: local signing/format/typos must succeed here | Amend SD4 under the second commission: unsigned local commit, operator format/typos and squash signing outside the box. First-visit failures remain historical. |
| Draft: retained data is always actually delivered | Refine CR2 before design: durable child outcome is not proof of harness receipt across a crash. Normal delivery uses exactly the persisted masked data; the crash scenario names the limit. |
| Draft: U6 proxy precedes U8 ledger | Split: U6 includes minimum durable writer, otherwise forwarding violates MB3/CR3. U8 adds staging, publication, engine fold/recovery and inspect. |
| Slice-one D4 / D11 | Adopt validation before compatibility and independent native OFF; reject a broker-specific nonempty restriction bypass. |
| Both positions: preserve R1–R5, D4/D11, typed contracts, exact independent final intent and no new platform | Adopt. The checked launch tree and singleton check support this common foundation; no position permits weak native OFF, copied authority or broker journal writes (0071 rulings 1–3, 5, 8, 10). |
| Robustness A: secret-read isolation | Adopt at MB2/MB4. hands.rs:1–13 and 0043 leave host reads outside the tool box; native read-only access and mode 0600 prove no confidentiality against the same user. U0 needs positive canary controls for store and child process state; unsupported secret-bearing holdings hard-refuse (rulings 3, 9; security). |
| Robustness B: protected startup inputs | Adopt at MB3. process.rs:173–178 inherits workdir; a pinned argv/version cannot protect a replaceable script or config. Use private cwd and protected operator installation, refusing an unprovable startup arrangement. Reject heuristic interpreter-flag allowlists and a new installer/attestation service (rulings 2, 3, 9; security). |
| Robustness C: all managed writers | Adopt at CR2. hands.rs:512–522 exposes the worktree writable and :574–604 layers binds afterward. A zero-grant sibling can attack another member's evidence. U8a2 separately owns dispatch/coordination; U8a protects mounts. Refuse unsafe writers before launch and protect historical artifacts (rulings 3, 9; security). |
| Robustness D plus simplicity A: ledger/fold semantics | Combine at CR3/CR4/CC2/SC4. Keep strict private Opened/Started/Terminal/Closed and one lifetime. Adopt one public settled row and settlement-only folding. checkpoints.rs offer has no append receipt and its held queue can lose rows; use confirmed fenced append with disk-backed retry, not queued-as-committed evidence. A live pending/committed state machine is unnecessary once live folding is removed (rulings 3, 5, 7–9). |
| Robustness E: response correlation and progress limits | Adopt at MB3. Match typed ID, method and phase, never a raw ID in journal identity; terminate uncertain sessions without replay. Absolute deadlines and a bounded notification budget prevent keepalive traffic defeating limits (rulings 3, 8, 9). |
| Robustness F: aggregate retention | Adopt at CR3. The former 4,096 × 8 MiB per broker allowed 32 GiB before multiplying brokers. Fixed 256 MiB attempt shares and bounded publication copies reserve before forwarding; no new quota service or silent downgrade (rulings 3, 5, 9). |
| Simplicity B: one DATA declaration | Adopt at GP2. 0065 says “in the same paragraph”, not in every later reference. One qualifying declaration per capability satisfies the rule and the existing researcher charter; later references pass. Lexical lint cannot judge prose obedience (rulings 5, 9). |
| Simplicity C: reuse canonicalization and ownership | Adopt with robustness's numeric safeguard. core/canonical.rs already supplies ordered bytes/hash; its numeric parsing can round precise values, so the broker edge rejects a changed numeric value. One buffer, one sealed inventory and existing engine launch facts replace extra serializers/cursors/launch state. hands Session is unsuitable because it deletes scratch (rulings 1, 3, 5, 6). |
| Simplicity D: consolidate two preparation PR pairs | Adopt U5a+U5b as U5a and U5c+U5d as U5c, preserving all tasks within three production files. Reject combining U4a/U4b, which requires four. A cohesive consumed extraction is still required for oversized parents; no speculative public scaffolding (rulings 4, 6). |
| Both positions: binding proofs and bounded costs | Adopt exact variants, real compiling removal failures, fake-server compile-to-inspect proof and pending external gates. Accept serial calls/fsync, delayed MCP display, refusal of unmeasured shapes and known #403/0012 residuals. The compile fence is not protection for a directly invoked U6 command: its handler stays fail-closed until its own protections are complete (rulings 8–10). |

## Masking and session reconciliation

Both current positions were checked against the source, not treated as votes.
No additional position is present in the journal-derived roster. Their pass
results mean the advice was produced; they prove no broker or harness behavior.

| Current claim | Disposition, evidence and owner |
| --- | --- |
| Both: preserve R1–R5 and the returned safeguards | Adopt unchanged: independent complete server intent; D4 validation and native OFF; gate class/stable office and one DATA declaration; measured strictness/read isolation; protected startup/all managed writers; durable settled evidence; versioned historical meaning. The checked singleton final proof, writable hands mounts and unconfirmed checkpoint queue support D3–D8. No new resolver, broker journal writer or public lifecycle (0071 rulings 1–3, 5, 7–10). |
| Robustness R-G: Closed must record session failure | Adopt typed Clean/Failed closure with first fatal safe cause, synced from reserved capacity and inspected by the engine. process.rs:499–520 takes driver status; child exit cannot override harness success by itself. Reject an end marker alone or deliberately missing closure as the ordinary fault channel. MB3/MB5, CR3/CR4, D7, U6d/U6e/U6f and U8d/U8e/U9b own the proof; no extra tool row for zero calls (rulings 2, 3, 8, 9). |
| Robustness R-H: scalar secrets conflict with the broker promise | Adopt conservative output-edge refusal with exact cause "MCP response cannot be safely masked". secret.rs:637–668 and secrets.md:50–52 explicitly preserve numeric scalars; 876543210 also passes exact-number checks. Reject carrying that residual into MB4/CR2, coercing numbers, or a second encoding catalogue. Reuse masking/canonicalization and check before stage/delivery. Owning specifications and 0077 change first, so return upstream (rulings 3, 5, 8, 9; high security if left fail-open). |
| Simplicity cut 1: use secret.rs for the injector | Adopt: 677 lines and an existing secret-value responsibility versus a small injector at adapters.rs:695. U6a touches only adapters.rs and secret.rs, with typed errors and current harness consumers. machine_proof.rs:3141 currently excludes secret.rs; explicitly scan actual calls there too, exclude the accessor definition, assert one call in the shared injector and catch an added second invocation. Reject a move that merely hides the call from the proof (rulings 4–6, 8, 9). |
| Simplicity cut 2: keep inspect dispatch in readouts.rs | Adopt: 346 lines, inspect already at :107. U8g touches cli_args.rs/readouts.rs; runtime still verifies files and view still derives provenance. Reject a new thin forwarding module without a measured size need (rulings 4–7, 10). |
| Simplicity cut 3a: U5a absorbs U5e | Adopt their exact three-file union. Put independent U5c first, then U5a closes tasks 20/21/24; U5f depends on U5a. Frozen v1 and native consumers remain, all MCP compiles still refuse (rulings 4–6). |
| Simplicity cut 3b: U6f absorbs U6g | Adopt: the cleanup row adds tests, no production path. Complete callable serving and its process proofs land together in the same three-file union; U7a depends on U6f. Keep every cleanup and output proof (rulings 4, 9). |
| Simplicity cut 3c: U9b absorbs U9c | Adopt: enablement and its eight guide/contract documentation paths land together, three production files. Quiesce old same-worktree writers, explain the serialization cost, and enable only after all earlier proof (rulings 4–6, 9). |
| Simplicity cut 3d: U10a absorbs U10b | Adopt: audit/removal restoration precedes final gates on that candidate within one test/evidence PR. No production files, no remote result presumed (rulings 4, 9, 11). |
| Both: retain justified splits and bounded costs | Adopt separate U4a/U4b (four-path union) and U8a2 (different cross-writer proof). Accept static quota waste, serial/fsynced calls, delayed display and same-worktree run serialization. Reject dynamic quotas, live scanners, retries of uncertain effects, garbage collection, another box/installer/transport or a registry (rulings 1–6, 10). |
| Robustness proof inventory; simplicity evidence owed | Combine: real compile through launch/broker/settlement/inspect, independent engine session-failure removal, scalar leak refusal removal and all prior negative/positive controls. Preserve native lost/stranded checkpoint failure handling; broker ledgers reconstruct no native observations. U0, external gates and operator acceptance remain pending; gate-owned findings stay informational (rulings 8, 9, 11). |

## Startup and discovery reconciliation

Both positions in the supplied roster were read in full. Their pass results
mean advice was produced, not that the design or runtime is accepted. Adopt,
combine or reject the claims below on their checked evidence, not by vote.

| Claim / source | Disposition, evidence and owner |
| --- | --- |
| Both: preserve R1–R5, D4/D11 and the adopted repairs | Adopt. engine.rs:4434 and hands.rs:1119 establish harness launch; the singleton final check needs an independently sealed complete set. Proposed 0077 changes only the launch letter of 0065 ruling 6. Namespace-only admission, realm authority, engine-only journal and U9b enablement stand (0071 rulings 1–3, 5, 8, 10). |
| Robustness R-I: injection can replace protected HOME/TMPDIR | Adopt under existing MB3. adapters.rs:687–704 overwrites names; secret.rs:20–30,105–137 permits both. One consumed base-environment builder owns fixed values and their reserved set, rejects collisions before store lookup/spawn, and forbids later replacement. Reject trusting current values, version pins, global name/schema edits or a second injector. D5/D6 and U6c/U6f/U9b bind each collision separately; no extra production path (rulings 2, 3, 5, 8–10; high security if bypassed). |
| Robustness 1 and simplicity's bounded session: isolation facts differ | Combine unchanged at SI1/SI2/MB2 and D2/D3. Codex's own adapter says ambient exclusion is unmeasured. Carriage, ambient strictness, native write containment and store/process read isolation stay separate; retain all positive sentinels and invocation shapes. Reject guessed Codex precedence, borrowed wrapper evidence or a roster exemption when self cannot compile (rulings 2, 3, 8, 9). |
| Robustness 2 and simplicity's one authority derivation | Adopt selected holding plus independent final equality, canonical gate class/stable office and independent native OFF. marks.rs selects one provider/model; realm reaches() cannot prove explicit gate egress scope. Preserve GP1/GP2's writes-first rule and one same-paragraph DATA declaration. No fallback union, argv-derived authority or equal-byte authored provenance (rulings 2, 3, 5, 8, 10). |
| Robustness 3 and simplicity's private durability | Combine and preserve R-G at MB3/MB5/CR3/CR4, D7. process.rs:499–520 trusts driver status; checkpoints.rs:169–248 offers no append receipt. Keep latched Clean/Failed closure, independent engine judgment, durable acceptance, reserved closure space, confirmed fenced append and full-payload deduplication. Retain 0/1/4,096-call controls and native loss/stranding refusals; no live scanner, public lifecycle, cursor or uncertain replay (rulings 3, 5, 7–9). |
| Robustness 4 and simplicity's one prepared buffer | Combine and preserve R-H at MB4/CR2, D5/D7. The shared masker leaves numeric scalars unchanged, including exact 876543210. Retain the exact unsafe-masking refusal independently of precision validation and retention policy, one masked canonical buffer, shared encoding definitions and fixed attempt shares. Persistence proves prepared output, not receipt; no coercion, truncation, silent downgrade or quota reset (rulings 3, 5, 8, 9). |
| Robustness 5 and simplicity's accepted custody cost | Adopt CR2 and D6 unchanged. Writable hands mounts expose sibling/historical evidence; a digest does not establish custody. Keep private bound handles, all-writer U8a2, read-only publication, alias checks, writer lease through settlement, older-engine quiescing and verify-before-reuse/inspect. Accept same-root serialization; distinct roots still need alias-safe write reach (rulings 1, 3, 7–10). |
| Simplicity: only required contracts and one read model | Adopt frozen tool-dialect v1, seat-record v6, manifest v12 and the next realms version after v7, with historical meaning intact. Keep attribution in view, inspect in readouts.rs and the shared injector in secret.rs; its cardinality proof must now inspect that module. D4/D8 and U4/U5/U6a/U8g own these choices (rulings 1, 3–8, 10). |
| Simplicity: absorb standalone discovery U7e | Adopt. marks.rs already selects holdings/clears notice; adapters.rs already renders the fixed notice/capability contract. U7c carries and clears only selected server/tool/discovery identifiers as a projection of configuration intent. U7d renders them and closes tasks 37.1–37.2 with 36.1–36.2; U8a depends on U7d. Retain both runtime and protocol suites, separate selection/rendering removals, unchanged requested digest and native/no-MCP prompts. Reject another registry, full plan in the prompt or string-key state beyond the edge (rulings 3–6, 10). |
| Both: bounded units and real consumers | Adopt 43 PRs and all 100 existing tasks; final archive task 49.3 brings the checklist to 101 without another PR. Keep U4a/U4b and U8a/U8a2 separate: their unions exceed three production files and the latter proves all writers. Keep U6's minimal ledger before forwarding and the public command's own incomplete-serving/retention refusals; the compile fence cannot protect manual CLI calls. U1–U4 remain independent as D9 states (rulings 4, 6, 9). |
| Simplicity's exclusions; robustness's bounded extension seam | Reject URL execution, reconnect/retry/pooling, extra MCP methods, nonempty restriction transport, installers/attestation/interpreter analyzers, another box, dynamic quota/index/GC services, new discovery prose/registries, duplicate serializers/accessors and native response capture. They add authority, lifetime or public vocabulary without a commissioned consumer. Typed stdio data and existing process seams suffice (rulings 1–3, 5, 6, 8, 10). |
| Simplicity: shorten history and repeated gate prose | Adopt one current reconciliation table, compact dated history below and in D10/D11, and one shared task-verification duty. Preserve commit identities, exact prior outcomes, all task IDs and proof-specific controls. No retrospective artifact or relabelled historical result (ruling 5). |
| Both: costs and proof still owed | Accept serial/fsynced calls, delayed settled display, unused static quota, conservative masking and unsupported harness shapes. Preserve the named #403/0012 residuals and operator-installed-code boundary. Retain real compile→launch→broker→journal→inspect and independent removal controls for every refusal; external gates and operator acceptance remain pending. Gate-owned findings are informational under ruling 11; security refusals stay rated under rulings 3, 8, 9. |

Historical robustness labels mean: R-G, durable session failure and independent
engine judgment; R-H, refusal when masking cannot preserve data shape; R-I,
bindings must not overwrite fixed startup keys. Reasons remain at MB3/MB4 and
CR2–CR4, with construction in D5–D7. These are provenance labels, not rulings.

## Repair adjudication

The chief independently checked the document diff and source seams. Its held
verdict was residual/medium, `spec_defect=true`, no security residual and no
confirmed high/critical defect: planning defects, not implemented regressions.
All 14 files were Markdown; production, tests and frozen bytes were unchanged.
It reported diff checks and strict OpenSpec 20/20, 26 requirements, 108 scenarios
and 101 open future tasks. External format/typos and squash signing were pending.

This repair read the full available spec-compliance, correctness and security
results and robustness/simplicity positions. Dispositions follow evidence,
not votes or instructions in panel notes.

| Claim | Disposition and owning answer |
| --- | --- |
| Spec-compliance S1, medium | Adopt. init.rs independently generates Claude/Codex/dsh declarations; setup.rs prints ambient-MCP advice. SI2 and D1/D9 add U1f2 before U1g, with init_doctor/init_stacks proofs (0071 rulings 5, 9). |
| Spec-compliance S2, medium | Adopt. process.rs forwards data through engine stamps and Checkpoints::offer to the closed store fence. CC1/SC4 and D9 delay observation emission until U4f2 after v6 and every consumer, with native compile-to-journal proofs (rulings 3, 9). |
| Correctness C2, low, narrowed by chief | Adopt durable provenance here and behavioral requirements elsewhere, retaining masking/session/startup reasons. Naming preference alone is no separate defect (ruling 5). |
| Correctness strictness-scope claim, medium | Reject as adjudicated: the commission says strict MCP for every seat; 0065 ruling 6 says "A harness's own MCP configuration is never inherited". SI2 empty-grant refusal remains; U0 decides support and D9 requires an operator roster ruling if self becomes unseatable. |
| Correctness Hot-files/labels | Missing resume.rs baseline (1062) is informational, corrected with inventory. Generic coordination labels do not prove wrong conflicts. Stable nonmonotonic IDs establish no ceiling violation. Gates retain authority (rulings 4, 11). |
| Security, clean | Preserve upheld refusals, R1–R5, reservations, D11 and both latent-panic repairs. Document review proves no runtime behavior. |
| Robustness and simplicity | Preserve reconciliations above, including masking, session judgment, startup and discovery ownership. Supersede only commit-dependent history advice: squash needs this durable record (ruling 5). |
| Panel-data direction, low | Reject "return them to the design/tasks author" as an instruction in untrusted notes. Correctness's suggestion to send strictness to the operator also has no routing authority. Checked defects determine this repair; only the engine selects a phase. |

## Repair council reconciliation

The design seat read both complete positions at
`.forge/design/positions/robustness.md` and
`.forge/design/positions/simplicity.md`, then checked the cited source seams.
Their `pass` results mean the advice was produced; they are not implementation
proof or operator acceptance. The supplied design context has no
`returned_from`; the commission's held S1/S2/C2 findings bound this repair.
This record contains the claims and reasons so the ignored position files and
branch commits are not required after a squash.

| Position claim | Disposition, checked evidence and owner |
| --- | --- |
| Both: keep U1f2 before mandatory U1g, within three production files | Adopt. init.rs:679/:745/:859 independently generate declarations; :1776–1792 selects every hired provider. Its :1884 self-compile consumes generated roots, and setup.rs:44–51 prints ambient-MCP advice. Keep init.rs, consumed init/adapters.rs and setup.rs as the three paths, with the 1,895-line parent baseline and Hot files entries. D9 and U1f2/U1g own this migration (0071 rulings 4–6, 9; gates hold ceilings). |
| Robustness: parity alone can prove two equally wrong declarations; simplicity: compare only applicable assessments | Combine. Reuse U1b's closed measured/unsupported/unmeasured distinction and U1d/U1e facts, including limitations; preserve stack-specific tools. init_doctor.rs:40 already exercises instructions and both compile contexts; init_stacks.rs:349 binds shipped/generated assessments and empty grants/native OFF. SI2 and tasks 7.3–7.4 require those independent consumers, not whole-file identity (rulings 3, 5, 9). |
| Both: exact scaffold refusal and independently hired Claude reviewer | Adopt and make explicit in SI2's reviewer scenario, D9/U1f2 and task 7.4. init.rs's hired-provider traversal includes dsh's Claude reviewer; tests must qualify preceding work candidates before each reviewer negative. Pin both SI2 causes with full site/office/realm context, the internal unmapped initialization wrapper and mapped starter compile. Synthetic support tests plumbing, not U0 qualification. No roster substitution, grant, resume qualification or initialization exemption (rulings 8, 9; 0065 ruling 6's "never inherited"). |
| Both: retain delayed native emission without a runtime switch | Adopt. process.rs:488–497 forwards data; engine.rs:2091–2107 stamps then offers it; engine/checkpoints.rs:221–235 embeds it; store/lib.rs:316–329 validates against immutable engine-line dispatch. D9 keeps U4a/U4b additive, U4c/U4d legacy-producing, U4e/U4f consumer-only, and U4f2's two-file serializer activation last. Keep legacy records valid at every merge and forbid private/partial public groups (rulings 3, 6, 9). |
| Robustness: one typed observation and selected authority; simplicity: no unused staging framework | Combine. U4c consumes its shared normalization in legacy lowering; the engine decodes once and derives attribution from the selected holding before Checkpoints::offer. Keep single/inline/fallback/panel plus sequence/resume/replacement consumers inventoried. Reject rollout flags, dual writes, schema exceptions, public observation versions, cursors and broker dependencies. Any needed carrier change requires a newly budgeted split before emission; process.rs/store/lib.rs remain review seams (rulings 2, 3, 5, 6, 10). |
| Both: retain native boundary proofs and independent removal controls | Adopt D9 and U4a–U4f2 proofs. Each preparation merge compiles and journals legacy native/local records through process, engine, fenced append, export and verification. U4e/U4f inject typed observations through that path; U4f2 uses actual adapter lowering of deterministic U0-grounded events. Exact groups, selected ownership, counts, private/spoof removal, distinct repeated calls, start/completion deduplication, history filtering and unheld/missing-identity refusals each bind. Neither serializer-only proof nor the MCP fence protects this native path (rulings 3, 8, 9). |
| Both: durable evidence, behavioral specifications and archive-safe links; simplicity: trim D1 routing narration | Adopt. Keep chronology and reconciliation here, leave R1–R5 verbatim in their authoritative record, and use file/section links. Remove D1's duplicate panel-routing narration. Task 49.3 retains evidence in the archive and repairs links after moving; no history scenario enters living specs. Keep scalar-secret leakage, harness-success/session-failure and protected-startup collision reasons at their behavior owners (ruling 5). |
| Both: preserve upheld scope and honest costs | Adopt. Universal strict MCP, reservations, proposed 0077, D11 and the two panic repairs remain unchanged. Unsupported scaffolds refuse; self becoming unseatable still requires D9's operator roster decision. Legacy native records before U4f2 are an explicit incomplete implementation state, never proof of ruling 8. Linux/macOS only; U0, mutations, runtime and external gates remain pending. Reject generic scaffold frameworks, source-checkout dependencies and broader broker redesign within this repair (rulings 4–6, 9, 11). |

Panel direction remains rejected for the reasons in
[Repair adjudication](#repair-adjudication); this council supplies no routing
authority. No unresolved upstream requirement fault was found in S1/S2/C2.

## Repair validation

The specify repair ran `git diff --check` and `openspec validate --all --strict`:
both exited 0; OpenSpec reported 20 passed, zero failed, with existing
informational long-requirement and unrelated archive notices. A foreground
Python static audit counted 26 requirements, 113 scenarios, 105 open tasks,
all 101 prior task IDs retained, 45 dependency-ordered PRs with at most three
production files each, 58 paths matching Hot files, and 578 valid local
Markdown links/anchors. It checked task closures and requirement ownership,
proposed 0077, unchanged R1–R5 bytes, and absence of branch-commit references
outside this record. The audited patch has eleven modified Markdown files
and this new evidence record; no production/test/frozen bytes change.

Final staged and unstaged whitespace checks are repeated before the plain
unsigned document commit. No Rust tests, U0 measurements, mutations, cargo
formatting, typos or external gates were run. Operator `cargo fmt`,
`typos --hidden` and signing of the squash remain pending. No push, archive,
remote reservation or operator acceptance is claimed.

The design reconciliation independently reran staged/unstaged
`git diff --check` and `openspec validate --all --strict`: all exited 0,
with OpenSpec 20 passed, zero failed and the same informational notices.
Its foreground static audit counted 26 requirements, 114 scenarios, 105 open
tasks with all prior IDs retained, 45 dependency-ordered PRs within three
production files each, 58 paths matching Hot files, and 579 resolving local
links/anchors. Requirement ownership and exact task closures agree. It also
checked unchanged R1–R5 bytes, proposed 0077, absence of branch-commit
references outside this record, and the four-file design repair scope:
design, this evidence record, SI2 and dependent tasks. All 15 paths changed
by the commission remain Markdown; production, tests and frozen artifacts
are untouched. The external and runtime obligations above remain pending.


### Tasks repair validation

The tasks seat adopted the repaired design and deltas, then made S1's proof
ownership follow design.md section D9: U1f2 proves generated/shipped metadata
parity, instructions and legacy compilation; U1g owns the active-admission
success and exact unsupported/missing-evidence matrix at both real compile
consumers, including the independently failing dsh roster's Claude reviewer.
S2's CC1/CC3/SC4 boundary obligations are now cited on every U4 preparation
task. Their coverage rows follow execution order, and repaired groups link
directly to their bounded design inventories. No requirement, unit dependency,
production inventory or task ID changed (0071 rulings 3, 5 and 9).

Source checks used foreground `sed -n` reads of init.rs at 665–690,
736–753, 850–870, 1755–1810 and 1810–1895, setup.rs at 28–64, init_doctor.rs at
1–115 and init_stacks.rs at 1–170. They confirmed independent generated
Claude/Codex/dsh declarations, the inherited-MCP instruction and both scaffold
compile consumers. Reads of process.rs at 480–502, engine.rs at 2082–2115,
engine/checkpoints.rs at 205–242 and store/lib.rs at 307–335 confirmed the
forwarding/append-fence seams behind delayed emission. These were source
observations, not executed Rust proofs.

The foreground `python3 - <<'PY'` task/coverage/dependency/link audit reported:

```text
PASS: 26 requirements, 114 scenarios, 105 unchecked tasks; all requirement citations and coverage owners resolve.
PASS: 45 dependency-ordered PRs; maximum three production files; 58 production paths exactly match Hot files.
PASS: All design task closures, task dependencies and stable task IDs agree; the 101 original task IDs remain present.
PASS: 596 local Markdown links/anchors resolve; no branch-commit references outside evidence.md.
PASS: R1–R5 bytes preserved, 0077 proposed, archive is the pending final task.
```

The audit also checked the commission diff: all 15 changed paths are Markdown,
with no production, test or frozen artifact edit. The task repair itself
changes tasks.md and this evidence record only. Its first audit drafts needed
parser corrections for the earlier handoff table and the numeric-secret
example; the corrected audit exited 0 without changing those artifacts.

`openspec validate --all --strict` exited 0 with `20 passed, 0 failed
(20 items)`. Its notices remain informational: long requirements and the two
unrelated archive-target notices already recorded above. `git diff --check`
exited 0 without diagnostics. `git diff --cached --check` and
`git diff --check` also exited 0 without diagnostics after staging both
documents. `git diff --check 2a23488b..HEAD` exited 0. The plain unsigned
commit is authorized; `git config --get commit.gpgsign` returned `false`.
`command -v cargo` and `command -v typos` found neither tool.

All 105 future implementation tasks, including the final fold, remain open.
No runtime, harness measurement or mutation result is claimed; operator
formatting, typos and squash signing remain pending. This record retains
history; task 49.3 preserves it in the archive and folds only behavior into
living specifications. No upstream artifact change was needed to write the
repaired breakdown.

### Implement repair validation

The implement seat confirmed the scaffold migration (SI2, D9, U1f2, tasks
7.3–7.4 and 8.1–8.2) and consumer-first emission (CC1, SC4, D9, U4a–U4f2,
tasks 13.1–18.4) already present, then finished the provenance repair. It
removed the remaining review-finding labels and visit wording from the
Decisions prose of four deltas (strict-mcp-isolation,
capability-call-checkpoints, slice-two-contracts, slice-two-delivery), from
design D1/D9/D11 and the unit preamble, from proposal's header, Impact and
Decisions, and from the tasks preamble. Each reason is kept in behavioural
terms; no requirement heading, SHALL statement, scenario or task changed.

A grep of proposal, design, tasks, the deltas and 0077 for eight-hex commit
identifiers outside the two base commits returned no match.
`git diff --check` exited 0 without diagnostics. This seat's permission
allowlist refused every `openspec validate` form tried (`--all --strict`,
with `--no-interactive`, by absolute path and by change name), so strict
OpenSpec was not re-run here. The last observed result is the tasks seat's
`20 passed, 0 failed` above; it predates these prose-only edits. Re-running
`openspec validate --all --strict` stays pending for the operator, alongside
cargo formatting, typos and squash signing.

## U6a implementation evidence

Branch `s2/U6a` from main `7f0aa4ad`, run
`0065-slice-two-unit-u6a-see-the--cd11b416`, recorded 2026-10-03. Tasks 26.1
and 26.2; MB4 (single plaintext injector), SD2, SD4.

**Change.** `bind_environment` moved from protocol `adapters.rs` into the
existing `secret.rs` as `pub(crate)`, returning the narrow
`secret::BindError::NotUtf8(name)`, whose `Display` keeps the operator text
`secret '<name>' is not valid UTF-8` (name only, never the value). The four
existing harness spawns (`run_cli` for exec, `invoke_stream_json` for
claude/lanetally, `invoke_codex`, `spawn_dsh`) call
`secret::bind_environment(..).map_err(|error| error.to_string())?`. The
injection-discipline comment moved with the function. The module doc and
the `expose_for_spawn` doc now name the injector and count secret.rs. There
is no new module, `lib.rs` registration, dependency or public item.
`resolve_bindings` is unchanged, so a missing binding is still refused
before spawn, and the injector reads only resolved bindings: no ambient
environment fallback exists.

**SD2 audit.** Production files touched: exactly the row's two. Lines:
`adapters.rs` 7268 → 7249, `secret.rs` 677 → 706 (under the 800 ceiling);
`adapters/tests.rs` 19096 → 19093 and `machine_proof.rs` 3483 → 3479, so no
file over its ceiling grew. `cli_and_stderr_helpers_cover_empty_stdin_and_unicode_boundaries`
fell to 100 lines, so its `#[expect(clippy::too_many_lines)]` became
unfulfilled under `-D warnings` and was removed (ruling 4). Removing the
test's `#[cfg(unix)]` block (Linux and macOS are both unix; decision 0063)
saved those lines. `quality/file-lines.txt`, `quality/too-many-lines.txt`
(entry removed; later adapters.rs rows −19 and adapters/tests.rs rows −3, as
the forced-lint clippy JSON pass of measure.sh step 5 printed them) and
`quality/suppressions.txt` (test `too_many_lines` 258 → 257) were updated by
hand. The seat could not run measure.sh or ratchet.sh; `cargo test -p
brokkr-cli --test suppressions` (6 passed) holds the tree to the edited
suppressions file. Witness pins: `cargo test -p brokkr-runtime --test
witness_digests` passed 6 with no bless, so no pin moved. Budgets: `--test
budgets` passed 4 and the heap_claude/codex/dsh tests passed, with no prompt,
lockfile or transcript input changed.

**Exact assertions.** `machine_proof.rs`
`expose_for_spawn_has_exactly_one_production_call_site` walks every crate's
`src/` tree minus test files, secret.rs included. It records each
`expose_for_spawn(` occurrence that is not preceded by `fn ` (the definition)
with the name of the nearest enclosing `fn`. It asserts the list equals
exactly `[(crates/brokkr-protocol/src/secret.rs, "bind_environment")]`.
`adapters/tests.rs` `cli_and_stderr_helpers_cover_empty_stdin_and_unicode_boundaries`
asserts `secret::bind_environment` on an invalid-UTF-8 binding returns
`Err(BindError::NotUtf8("TOKEN"))`, and that the exec spawn path (`run_cli`)
returns the exact text `secret 'TOKEN' is not valid UTF-8`. The existing
harness tests `every_model_harness_receives_its_bindings_and_masks_its_stderr`,
`a_dsh_seat_receives_its_bindings_and_masks_its_stderr` and
`a_bound_claude_seat_runs_and_journals_only_the_secret_name` keep the
injection and the leak scans (stderr masked to `[secret:API_TOKEN]`, value
absent) bound.

**Compiling mutations**, each run and then restored. M1–M4 were re-run
against the test's final form.

| Mutation | Command | Failing test and assertion |
| --- | --- | --- |
| M1: second call inside secret.rs (`let _ = self.secret.expose_for_spawn();` in `BoundSecret::name`) | `cargo test -p brokkr-cli --test machine_proof -- expose_for_spawn` | machine proof `assert_eq!` at :3179, left `[(secret.rs, "name"), (secret.rs, "bind_environment")]` |
| M2: second call outside secret.rs (in adapters.rs `masked_text`) | same | left `[(secret.rs, "bind_environment"), (adapters.rs, "masked_text")]` |
| M3: definition filter removed from the test (`|| true`) | same | left `[(secret.rs, ""), (secret.rs, "bind_environment")]` (the definition counted) |
| M4: the one call moved into a helper `plaintext` in secret.rs | same | left `[(secret.rs, "plaintext")]`; the location is bound, not only the count |
| M5: invalid UTF-8 silently skipped (`let Ok(..) else { continue }`), an environment fallback | `cargo test -p brokkr-protocol --lib -- cli_and_stderr_helpers` | `assert_eq!` at tests.rs:382, left `Ok(())`, right `Err(NotUtf8("TOKEN"))` |
| M6: `Display` changed to `secret {0:?} …` | same | `assert_eq!` at tests.rs:384, left `"secret \"TOKEN\" is not valid UTF-8"` |
| M7: injector call removed from `run_cli`, `invoke_stream_json` and `spawn_dsh` | `cargo test -p brokkr-protocol --lib -- cli_and_stderr_helpers every_model_harness a_dsh_seat_receives a_bound_claude_seat` | all four failed: Claude `exit_code` 7 ≠ 0, dsh 7 ≠ 0, the bound claude seat, and `run_cli` `unwrap_err` on `Ok` |
| M8: injector call removed from `invoke_codex` only | same | `every_model_harness…` failed with "Codex: the binding never reached the child", left 7 |

After every mutation was restored, each command passed: the machine proof
passed 1, and the four protocol tests passed.

**Gates on the candidate tree.** `cargo fmt --all -- --check` was clean, and
`cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
was clean. `cargo test -p brokkr-protocol --all-features --locked` passed:
lib 628 with 1 ignored, `hands_exits` 6, `secret_drop` 1 and doctests 1.
`cargo test -p brokkr-cli --all-features --locked` passed every target: lib
619 with 1 ignored, `machine_proof` 61, `suppressions` 6, `ratchets` 13,
`layering` 15, and the rest green. `cargo run --locked -p brokkr-cli --
compile --bundle bundles/self` compiled. `openspec validate --all --strict`
gave 20 passed, 0 failed. `git diff --check` was clean.

**Pending.** `typos --hidden`, `quality/ratchet.sh files` and `clones`
(jscpd) were refused by this seat's permission allowlist and were not
observed. The files rule was checked by hand above; clones are unmeasured.
Also pending: the workspace-wide `cargo test --workspace` beyond the touched
crates plus brokkr-runtime's `witness_digests` and `budgets`, exact coverage
on a capable host, remote CI, and signing of the squash if the seat signature
is not the steward's.

### U6a repair visit (CI on PR #525, head 83633e24)

Run `0065-slice-two-unit-u6a-see-the--767bd1f6`, 2026-10-03. CI failed two
required checks: `quality/ratchet.sh clones` (three new production clones in
`adapters.rs`: the spawn boilerplate at `run_cli`, `invoke_stream_json`,
`invoke_codex` and `spawn_dsh`) and `scripts/coverage-exact.sh` (functions
4833/4836: the `.map_err(|error| error.to_string())` closures at
`adapters.rs:2438`, `:3786` and `:5004` were never called).

**Change.** One spawn helper in `adapters.rs`, `spawn_harness(command,
workdir, stdin, stdout, env, bindings)`: it splits the argv (refusing an
empty one as `empty command`), sets the args, the workdir, the extra
environment, the stdio dispositions and a piped stderr, calls
`secret::bind_environment` once, maps its typed refusal once, and spawns
under the one `could not invoke the agent CLI` context. `spawn_piped` is its
piped-stdin/stdout, no-extra-environment form. `run_cli`,
`invoke_stream_json` and `invoke_codex` call `spawn_piped`; `spawn_dsh`
passes its unsigned-commit `GIT_CONFIG_*` triple and the host identity as
`env` and `Stdio::null()` stdin. The order is unchanged: the extra
environment is set before the injector, so a declared binding still
overrides it. `adapters.rs` stays at 7249 lines (`wc -l`), so no file over
its ceiling grew. `quality/too-many-lines.txt` follows two start lines that
moved (`fold_stream_event` 1488 → 1512, `dsh_launch_with` 4509 → 4515), as
the forced `clippy::too_many_lines` JSON pass of measure.sh step 5 printed
them for the protocol lib. No length moved. One behaviour edge: the claude,
codex and dsh spawns indexed `command[0]` and would have panicked on an
empty argv. They now return `empty command`, as exec always did.

**Coverage diagnostic.** I ran `cargo +nightly-2026-09-05 llvm-cov -p
brokkr-protocol --all-features --locked --branch --lcov` on this tree and on
HEAD's `adapters.rs`, then reduced both with the gate's function rule (file
plus start line, any positive instance covers) in `jq -R`. At HEAD,
`adapters.rs` showed 376/385, with the uncovered starts at 2398, 2401, 2407,
3903, 5774 and 7241 plus the three closures CI named (2438, 3786, 5004).
This tree shows 378/384. The six that remain are at the same untouched
functions (`stage_prompt` and its two closures, `invoke_dsh_launch` closure,
a `.map` closure in the dsh tree, `serve`). The workspace run covers them,
since CI's 4833/4836 named only the three closures. `spawn_harness` (:707),
its closures (:717, :726) and `spawn_piped` (:731) are all hit, and
`secret.rs` is 57/57 in both runs. The workspace-wide exact gate itself is
pending on a capable host.

**Exact refusal through a real call site.**
`cli_and_stderr_helpers_cover_empty_stdin_and_unicode_boundaries` drives a
non-UTF-8 binding through `run_cli` → `spawn_harness` and asserts
`secret 'TOKEN' is not valid UTF-8` (tests.rs:383). The test needed no edit.

| Mutation | Command | Failing test and assertion |
| --- | --- | --- |
| R1: the helper's refusal mapping becomes `.map_err(\|_\| "refused".to_string())` | `cargo test -p brokkr-protocol --lib --locked -- cli_and_stderr_helpers` | `assert_eq!` at tests.rs:383, left `"refused"`, right `"secret 'TOKEN' is not valid UTF-8"` |
| R2: the helper drops the extra environment (`.envs(env.iter().copied().take(0))`) | `cargo test -p brokkr-protocol --lib --locked -- dsh` | `the_dsh_seat_commits_unsigned_under_the_host_identity` (tests.rs:8724) and `the_dsh_driver_promotes_the_seats_branch_out_of_the_private_store` (tests.rs:8757) failed; 127 passed, 2 failed |

Both were restored. Afterwards the four binding tests
(`cli_and_stderr_helpers…`, `every_model_harness…`, `a_dsh_seat_receives…`,
`a_bound_claude_seat…`) passed.

**Gates on the repaired tree.** `cargo fmt --all -- --check` and `git diff
--check` were clean. `cargo clippy --workspace --all-targets --all-features
--locked -- -D warnings` was clean. `cargo test -p brokkr-protocol
--all-features --locked` passed lib 628 (1 ignored), `hands_exits` 6,
`secret_drop` 1 and doctests 1. `cargo test -p brokkr-cli --all-features
--locked` passed every target, including lib 619, `machine_proof` 61,
`ratchets` 13, `suppressions` 6 and `layering` 15. `cargo test -p
brokkr-runtime --test witness_digests --test budgets` passed 4 and 6 with no
bless. `compile --bundle bundles/self` compiled, and `openspec validate --all
--strict` reported 20 passed.

**Pending.** `quality/ratchet.sh clones` and `files`, `jscpd` and `typos
--hidden` were refused by this seat's permission allowlist and were not
observed. "Duplication holds" is therefore unconfirmed here. The files rule
was checked by hand (7249 = 7249). The workspace exact-coverage gate and
remote CI are also pending.

## U0 measurement

Unit U0 ran D2's matrix on one Linux host from 2026-10-03 11:50Z to
2026-10-04 00:20Z against claude 2.1.287, the LaneTally wrapper, codex-cli
0.160.0 and dsh 0.1.5-rc.1. The durable record is
[slice-two-mcp-isolation.md](../../../docs/evidence/adapters/slice-two-mcp-isolation.md)
with per-cell data in
[slice-two-mcp-observations.json](../../../docs/evidence/adapters/slice-two-mcp-observations.json).
Verdicts were read from sentinel lifecycle logs and harness-reported tool
listings, never from model text.

- **Claude:** `--strict-mcp-config` with an explicit engine configuration
  passed cold, with and without hands. A managed MCP file makes the
  harness refuse the flag before any model call.
- **dsh:** an engine-only `DSH_HOME` with the engine row in the overlay
  passed cold. The existing profile with the overlay did not.
- **Codex:** neither candidate passed. The private home is defeated by the
  workdir trust codex writes into it on the work shape and by the
  `/etc/codex` layers. The `-c mcp_servers` table override merges with
  every source.
- **LaneTally:** excluded every ambient source at startup through the
  actual wrapper. Its engine answer and canaries remain pending.
- **exec:** inapplicable.

Read-isolation canaries were measured separately. Claude's cold hands
shape excluded both the store and the process channel. Codex's and dsh's
native shells read the store and excluded the process channel.

Process note. Claude cells C01–C10 used a copy of the operator's OAuth
access token in a disposable HOME, with the refresh token withheld. The
controller then ruled that no credential may be copied, the copy was
shredded, and those ten results were kept. Every later cell used plainly
fake keys: LaneTally against a dead loopback endpoint, Codex and dsh
against the shared local Spark model. Two questions are pending operator
approval because each would need a credential copy: LaneTally's
authenticated rows, and Codex's ChatGPT-login auth and OpenAI-model
discovery. macOS remains pending (no host). No adapter, production file
or test changed. Tasks 1.1 and 1.2 carry this evidence.

2026-10-04, approved legs. The operator approved both pending items and
they ran that day under the controller's handling rules: minimal copies,
mode 0600, shredded at the end of each leg, and re-scanned with 0
exact-value matches. Through the actual wrapper, LaneTally passed D2 on
the cold shape with and without hands (LT08 to LT12). Its canaries match
Claude's. Under a private home, the operator's ChatGPT login authenticates
Codex. For `gpt-6-luna`, MCP tools are discovered through the code-mode
`exec` catalogue, not `tool_search`, and the account's `codex_apps`
connectors also load, so candidate (a) still fails. No item remains
pending except macOS. The cells are in the record above.

## U3a implementation evidence (tasks 10.1–10.2)

Branch `s2/U3a` from main `7f0aa4ad`, run
`0065-slice-two-unit-u3a-see-the--d7f83769`, recorded 2026-10-03. The tested
tree is the commit that carries this section, the second implement visit's,
on top of `8f0cdc56`. That visit answers the review return of `8f0cdc56`
(F1 clones and completion evidence, F2 class construction, F3 fixture office
vocabulary). Every result below was observed in the second visit. None is
carried from `8f0cdc56` or an earlier visit.

### What changed

- Production stays in the row's three files. `capabilities/gates.rs` (new,
  136 lines) holds the one pure GP1 check, `check(site, definition, grant)`.
  It returns `GateRefusal::{Writes, Egress { office }}` (`thiserror`), and
  writes is matched before egress. It also holds the typed `Cause`
  (`Ungranted`, `Incompatible`, `Gate`), which renders GP1's required form
  without the incompatible-grant suffix.
- `capabilities.rs`: `SiteAsks.class: SeatClass` (`pub(crate)`) is the
  executable site's own class. No construction can leave it out: `SeatClass`
  and `SiteAsks` derive no `Default`, and the one constructor,
  `SiteAsks::at(class, label, agent, site)`, replaces `SiteAsks::of` and takes
  the class. `of`, now a work-site shorthand for `at`, lives in the
  `#[cfg(test)]` module `gate_class.rs`, so the suites that call it
  (`engine/capability_tests.rs`, `engine/resume_tests.rs`, `agents/tests.rs`)
  are not edited and production cannot reach it. The doctor's seatless
  `Authority::assess` builds its site with an explicit `class: Work`, and its
  comment says that answers no gate's holding and authorizes no launch. The
  check runs in `holding` after grant presence, office reach and
  the nonempty tool set, and before provider carriage. D4's grant-presence
  and office-reach checks moved, with their exact text, into `reaching`.
  `holding` is now under 100 lines (absent from the step-5 clippy run), so
  its `too_many_lines` expectation is removed. Its cyclomatic complexity
  against its baseline is the CRAP ratchet's, which needs the exact-coverage
  LCOV and is pending below. This visit observed no complexity figure.
- `bundle.rs`: `site_asks` reads `parse_class` once per agent-backed or
  inline executable site and constructs through `SiteAsks::at`. The agent's
  harness fragment reuses `asks.class`. The office is still the agent's name
  or the inline site's label. A dialect step is a gate though it writes no
  class. `step_class` is now that fact's one home, read by `parse_sequence`
  for the compiled step and by the capability record of the dialect step, so
  the record carries `Gate`, not a default. Its asks are still empty.
  `parse_sequence` shrinks from 247 to 243 lines.
- Public API: one line of `quality/public-api/brokkr-runtime.txt` moves,
  `SiteAsks::of` to `SiteAsks::at(SeatClass, …)`, with the same count. The
  snapshot was regenerated with
  `cargo +nightly-2026-09-05 public-api -p brokkr-runtime -sss --color never`.
  `diff` against it differs only in the provenance header. The
  `pub(crate)` field adds no public item.
- Tests: `capabilities/tests/gate_class.rs` (5 tests, 295 lines) and
  `bundle/agent_tests/gate_tests.rs` (5 tests, 310 lines) are new child
  modules. Each parent's `mod` line now sits at its end, and its header is
  main's bytes again. `capabilities/tests.rs` offsets the line by rewrapping
  the last test's doc comment. `agent_tests.rs` offsets it with
  `openspec_with_exec`, the shared builder that replaces the copied
  exec-adapter/openspec block. Parents stay at 2,160 and 6,788 lines.
- `tests/capability_launch.rs` shrinks from 11,883 to 11,878 lines. The
  dialect writer is now one helper, `hosts_dialect`, which
  `a_restriction_value_moves_the_manifest_digest_even_where_it_is_inactive`
  calls instead of carrying its own copy (same bytes, same file).
  `hosts_grant(operator, offices)` names each office once, and an empty list
  names none. Each caller now derives its list from its own fixture facts
  (F3), not from a hand-written list:
  - the cold/resume test seats no gate and passes `&[]`, so its grant is
    main's;
  - the compiled matrix passes `matrix_offices()`, the `carrier_sites`
    labels of `CARRIERS`, which begin with each carrier, the office of its
    agent-backed sites;
  - the boxed matrix builds one grant per carrier from the `office_of`
    mapping its own assertions use (hoisted from the inner loop), over its
    `shapes` labels.

  Every gate row still reaches CQ1's "provider 'codex' cannot express
  restriction 'allow.hosts'", and the boxed fallback still reaches "provider
  'claude' cannot carry a binding to provider 'codex'". No expected text
  changed. Neither CQ1 matrix test grew: 159 lines, and 225 down to 223.

### Mutations

Each mutation was compiled and run with
`cargo test -p brokkr-runtime --all-features --locked --lib gate_`, then
restored. All eleven were observed in the second visit, on its final
production code. Lines are each test's panicking assertion. In gate_class.rs,
:84 is the shared `gate_loses` helper's requires assertion (`unwrap_err` on a
holding, or `required(cause)`), and :145 the check-table row assertion.

| # | Compiling mutation | Failing tests (assertion line) |
| --- | --- | --- |
| M1 | writes arm disabled (`if false && …`) | `the_gate_check_reads…` (:145), `a_gate_never_holds_writes…` (:84, `unwrap_err` on a holding) |
| M2 | egress arm before writes | `the_gate_check_reads…` (:145), `a_gate_never_holds_writes…` (:84, `required(WRITES)`) |
| M3 | egress arm disabled | `the_gate_check_reads…` (:145), `a_gates_egress…` (:84), `a_dropped_gate_want…` (:273), fallback (gate_tests.rs:194), relocated verify (:283), inline (:228), every form (:179) |
| M4 | `match SeatClass::Gate` (class ignored) | `the_gate_check_reads…` (:145), `a_work_site…` (:240), inline (:228), every form (:179) |
| M5 | last gate arm refuses reads-only | `the_gate_check_reads…` (:145), `a_gates_egress…` (:201, named holding), inline (:228), every form (:179) |
| M6 | `let _ = gates::check(…)` in `holding` | `a_gate_never_holds_writes…` (:84), `a_gates_egress…` (:84), `a_dropped_gate_want…` (:273), all four GP1 tests of gate_tests.rs (:179, :194, :228, :283) |
| M7 | GP1 required form gains the incompatible suffix | `a_gate_never_holds_writes…` (:84), `a_gates_egress…` (:84), relocated verify (:289), inline (:228), every form (:179) |
| M8 | `site_asks` forces Work | fallback (:194), relocated verify (:283), inline (:228), every form (:179) |
| M9 | the inline branch alone forces Work | `an_inline_gate_is_judged_at_its_own_class_and_label` (:228) only |
| M10 | the dialect step's record built at `Work` | `a_dialect_step_is_recorded_at_the_class_its_step_compiles_to` (:309): left `("validate", Gate, Work)`, right `("validate", Gate, Gate)` |
| M11 | `step_class` forgets a dialect step is a gate | the same test (:309): left `("validate", Work, Work)` |

M9 is the reason for the inline test, and M10 for the dialect-step test: no
other suite saw either. M10 is F2's removal control. Before this visit, a
dialect step's record was built through `Default`, and no test read the
record's class.

Fixture control C1 (F3). `hosts_grant` was given no office at the compiled
matrix (`&matrix_offices()[..0]`) and at the boxed matrix (`&every[..0]`).
`cargo test -p brokkr-runtime --locked --test capability_launch --
a_restricted_grant` then failed both, and the cold/resume test stayed `ok`.
The matrix's gate rows reported "a gate's egress capability requires the
realm grant to name office 'codex-gate' explicitly" (likewise 'pair',
'pair-claude' and 'boxed'). The boxed requires rows reported GP1's form for
office 'x', 'boxed-codex' and 'boxed-fallback'. So the derived lists are what
keep those rows on CQ1 and on the Claude carriage refusal. After restoring,
the same run passed 4 of 4 (the filter also matches
`a_restriction_value_moves…`).

With all of M1–M11 restored, `cargo test -p brokkr-runtime --all-features
--locked --lib` passed 711 of 711.

### Scenarios to tests

- Reads permitted: `a_gates_egress…` (reads abstraction, offices absent, held
  `["reads"]`, `--search-on`) and the check's reads row.
- Writes never, writes before egress: `a_gate_never_holds_writes…`, for
  [reads, writes], [reads, writes, egress] and [writes], with offices absent
  and [reviewer]. Requires refuses, and wants drops with the optional form
  and native OFF.
- Default reach is not egress authority: the absent list gets GP1's cause.
  [reviewer] holds. [] keeps D4's "grants it to no office". [review] (the
  label) keeps D4's scope cause.
- Optional drops keep independent denial: `a_dropped_gate_want…`.
  Unsupported and unmeasured OFF still refuse, and the default OFF is
  recorded off.
- Nested sites: `every_executable_form…` covers the single seat, panel
  member, sequence step, select case and default (20 rows). The inline site
  is `an_inline_gate…`, every fallback candidate is
  `every_fallback_candidate…`, and the relocated verify is
  `a_relocated_verify…`. A dialect step that asks nothing is recorded at the
  gate class its compiled step has, which `a_dialect_step_is_recorded…`
  shows (F2). Both of these tests compile one shared fixture,
  `wrapped_verify`.
- Subtraction and unused grants: `a_work_site_a_subtraction…`. Nothing is
  held, there is no notice, OFF holds, and the grant stays pinned.
- MCP: classes are checked at the helper (`gates::check`, MCP dialect rows).
  `Authority::load` still refuses every MCP grant, absent, [] and named,
  with the exact unbuilt-kind cause. The fence is unchanged.

### Gates observed

All observed in the second visit, on the final tree.

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`:
  finished, no warning or error.
- `cargo test -p brokkr-runtime --all-features --locked --lib`: 711 passed.
- `--tests --no-fail-fast`: 26 of 26 binaries `ok`. After the last
  `capability_launch.rs` edit, `capability_launch` (68), `witness_digests` (6)
  and `budgets` (4) were run again and passed. `witness_digests` ran
  unblessed, so no witness or compose pin moved.
- `cargo test -p brokkr-cli --all-features --locked --no-fail-fast`: every
  binary `ok`, no FAILED. That includes the `suppressions` test and the heap
  budget tests, against the committed baselines.
- The remaining crates (`--workspace --exclude brokkr-cli --exclude
  brokkr-runtime`): every binary `ok`, no FAILED.
- `cargo run --locked -p brokkr-cli -- compile --bundle bundles/self`:
  compiles, digest `11c7d0e7…`.
- `openspec validate --all --strict`: 20 passed, 0 failed.
- `git diff --check` and `git diff --cached --check`: clean.
- Public API: the regenerated brokkr-runtime snapshot is committed, as above.
- File sizes (`wc -l`), against main's `quality/file-lines.txt` baseline where
  one exists, else the ceilings: bundle.rs 7,802 (main 7,815),
  capabilities.rs 2,363 (main 2,369), gates.rs 136 (ceiling 800),
  gate_class.rs 295 and gate_tests.rs 310 (ceiling 2,000), the parents
  unchanged at 2,160 and 6,788, capability_launch.rs 11,878 (main 11,883).
- Re-measured listings, for the touched files only, by measure.sh's own
  steps. `file-lines.txt`: the rows above. Main's listing already omits files
  added since its last measure, so it was not regenerated whole.
  `too-many-lines.txt`, from the step-5 clippy run (`--force-warn
  clippy::too_many_lines`, JSON): against main, `record_capabilities` went
  from 179 to 173 and `parse_sequence` from 247 to 243. `holding` is removed,
  and the boxed CQ1 test went from 225 to 223. Every other entry in the
  touched files kept its count, and only its line number moved.
  `suppressions.txt`: production `too_many_lines` went from 46 to 45. No
  entry grew.
- Budgets: `crates/brokkr-runtime/tests/budgets.rs` (prompt bytes and crate
  count) and the brokkr-cli heap tests pass against the committed budgets.

### Controller rebuild and gates (2026-10-04)

The seat's permission layer refused `typos --hidden`, `bash quality/ratchet.sh
files|clones` and `bash scripts/measure-budgets.sh` ("This command requires
approval") in both implement visits, and the run stopped blocked on them
alone. The review's four F1 clones had been folded at their source. The
controller (the session driving these units) then did the following:

- **Rebuilt the unit on main at `0e835584`,** after #527 and #520 had landed.
  The diff `7f0aa4ad..89ffb9c1` applied cleanly, and main had not moved any
  file the unit touches.
- **Folded one new test clone.** `bash quality/ratchet.sh clones` found a
  7-line clone: the two CQ1 matrices' identical opening, at
  `capability_launch.rs:11439` and `:11643`. It appeared when the review
  answer moved `hosts_grant` into the boxed matrix's per-carrier loop. It is
  now one fixture, `cq1_codex()`, with `SeatClass::{Gate, Work}` imported
  once at the top of the file.
  - `capability_launch.rs` is 11,883 lines, main's baseline.
  - The two matrices fell to 157 and 221 lines.
  - `file-lines.txt` and `too-many-lines.txt` changed only those entries.

On the final tree:
- `cargo fmt --all -- --check`: clean.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D
  warnings`: clean.
- `bash quality/ratchet.sh clones`: duplication holds. `files`: file size
  holds. `api`: public API holds. `baselines origin/main`: no baseline raised.
- `typos --hidden` and `git diff --check`: clean.
- `openspec validate --all --strict`: 20 passed, 0 failed.
- `cargo test -p brokkr-runtime --all-features --locked --lib`: 720 passed,
  including main's own tests since `7f0aa4ad`.
- `--tests`: every binary `ok`, `capability_launch` (68) and `budgets` (4)
  among them.
- The brokkr-cli `suppressions` test passes.
- `bundles/self` compiles, digest `11c7d0e7…`.
- `bash scripts/measure-budgets.sh`: no budget rises. It tightened 52
  prompt-byte budgets by 13 bytes each and re-dated the notes. That is main's
  drift since 2026-09-30, not this unit's, so those edits were not kept.

Task 10.2 is ticked on these observations.

### Implement visit on the rebuilt tree (run `0065-slice-two-unit-u3a-see-the--cf43401a`)

This visit reviewed the rebuilt unit against U3a's row, GP1 and the
standing scope ruling. It found nothing to fix and changed no code. Observed
on this tree:

- The format check and the workspace clippy run with `-D warnings`: both
  clean.
- `cargo test -p brokkr-runtime --all-features --locked --lib`: 720 passed.
  `--tests --no-fail-fast`: every one of the 26 binaries `ok`.
  `capability_launch` passed 68, `budgets` 4 and `witness_digests` 6 (unblessed).
- `capability_launch.rs` is 11,883 lines (`wc -l`), at its baseline.
- M6 was re-run here (`let _ = gates::check(…)` in `holding`), with
  `cargo test -p brokkr-runtime --all-features --locked --lib gate`. It
  failed 7 tests, exactly the table's row: gate_class.rs :84 (twice) and
  :273, and gate_tests.rs :179, :194, :228 and :283. After the restore,
  61 of 61 passed.
- `openspec validate --all --strict`: 20 passed, 0 failed.
- `git diff --cached --check`: clean.
- `bundles/self` compiles, digest `11c7d0e7…`.

This seat's permission layer refuses `typos --hidden`, `quality/ratchet.sh`
and `scripts/measure-budgets.sh`, so it did not run them. Their results above
are the controller's. Only this file changed after the controller's run, and
CI's required jobs re-check them on push.

### Pending

- `bash scripts/coverage-exact.sh` and the cyclomatic ratchet on its LCOV are
  pending for a capable host or CI. They are not passed.
- Remote Linux and macOS CI is pending.

## U2 implementation evidence (tasks 9.1–9.2)

Run `0065-slice-two-unit-u2-see-the-u-5afffdf6` wrote this unit on 2026-10-04,
on branch `s2/U2`, cut from main at `e58dc071`, and stopped before verify
because its seat could not run `typos`, the ratchets or the budget measure.
Run `0065-slice-two-unit-u2-see-the-u-ca7233a6` reviewed the same tree,
uncommitted on `e58dc071`, changed no production or test line, and observed
every result below on it again. The mutations were applied to that tree and
restored before the commit.
After CI on PR #530 refused MSRV 1.88's E0716 at doctor's closure and
`Authority::holding`'s CC 18 over its baseline 17, the controller moved
`holding`'s inventory lookup into `native_serving` and had doctor bind its
own `(provider, key)` (dd61af5b), and run `0065-slice-two-unit-u2-see-the-u-c346e208`
re-ran the table below on that tree over main `4f56f251`, where `holding`
measures CC 16 and M2, M3 and M5 fail as recorded, at `capabilities.rs:1614`
and `doctor.rs:1115` for the two panics.

### What changed

- Production stays in the row's three files.
  - `capabilities/binding.rs` (new, 106 lines) is the one home of the binding.
    It projects a grant's binding from its dialect's kind, with one arm per
    kind and no wildcard: `bound(realm, capability, dialect)`.
    `Unbound::{Mcp, Hands, Missing}` (`thiserror`) is the typed reason a
    grant binds no native provider. Its `Mcp` and `Hands` texts are the
    compile fence's old words, byte for byte. `Authority::dialect` returns
    `Missing` where no dialect was loaded. `Authority::binding` now returns
    `Result<Native<'_>, Unbound>`, where `Native { dialect, provider,
    adapter_key }` replaces `Option<(&str, &str)>`.
  - `capabilities.rs` drops the private `bindings` map, which restated the
    dialect's kind (0071 ruling 5). `Authority::load`'s fence is now
    `bound(..)?` for every loaded dialect, used by a seat or not, so the old
    MCP and hands refusals are unchanged. `holding` reads its dialect through
    `Authority::dialect`. A missing one becomes `Cause::Incompatible` through
    the grant's dialect name, where it used to index `self.dialects[..]`. It
    reads the binding through `bound` after GP1, and a non-native kind
    becomes `Cause::Incompatible` with the `Unbound` text, where it used to
    index `self.bindings[..]`. The file shrinks from 2,363 to 2,337 lines.
  - `doctor.rs` moves the per-grant line out of `report_capabilities` into
    `report_grant(report, what, (capability, grant), alone, adapters,
    plan)`. That function is the direct seam. It reads `Authority::binding`
    and reports a failing `MISSING` line with the binding's own words, where
    the code used to `expect` "a grant that loaded is bound to a provider".
    The `wants` and `office` closures became the free functions `wants` and
    `grant_office`, which both loops use. The `unread` set folded into
    `read: BTreeMap<&str, Option<(provider, key)>>`, where `None` means the
    grant's line failed. Two function-local imports moved to the module. The
    file stays at its 1,572-line baseline. `report_capabilities` falls from
    254 to 184 lines and keeps its baseline `too_many_lines` and
    `excessive_nesting` expectations (clippy `-D warnings` passes, so both
    are still fulfilled).
- No compile bypass: production builds an `Authority` only through `load`
  and `nothing`. The seam fixtures build theirs in test code, from the
  existing public fields.
- Public API: `quality/public-api/brokkr-runtime.txt` gains `Unbound` (12
  lines) and `Native` (4 lines), and `Authority::binding` changes its return
  type. The first visit's note that its regenerated snapshot "differs only in
  the provenance header" was wrong: against main the snapshot adds `Unbound`
  and `Native` and changes `Authority::binding`'s return type, 1,398 to 1,415
  items (1,432 to 1,449 over main `4f56f251`, the same 17), a raise the
  operator ruled on 2026-10-04.
- Tests:
  - `capabilities/tests/binding.rs` (new child module, 115 lines, 2 tests).
    The parent's `mod binding;` line is offset by rewrapping
    `provider_compatibility_drops_a_want…`'s doc comment, so
    `capabilities/tests.rs` goes from 2,160 to 2,159 lines.
  - `doctor/capability_tests.rs` gains one test, going from 1,910 to 1,975
    lines (ceiling 2,000).
  - `agents/tests.rs` and `bundle/agent_tests.rs` are unchanged. The row
    allows them but needed no edit.

### Scenarios to tests

- Non-native grant at the resolver seam:
  `an_mcp_grant_at_the_resolver_is_a_typed_refusal_and_never_a_native_binding`.
  The fixture is a structurally valid `library-docs` grant through the `mcp`
  dialect `docs-mcp`, inserted past the fence beside the native
  `web-search`. Its checks:
  - `binding` is exactly `Unbound::Mcp { realm, capability, dialect }`;
  - the native control is exactly `Native { &dialects["web-search"],
    "test-native", "web-search" }`;
  - `requires` refuses with the whole incompatible form carrying the MCP
    cause;
  - `wants` drops with its whole notice ("no native denial is claimed"),
    while the native want is held with `--search-on`.
- Missing binding: `a_missing_binding_is_a_typed_refusal_and_never_a_panic`.
  `binding` is exactly `Unbound::Missing`. `requires` and `wants` give their
  whole forms, and the native power stays OFF (`--search-off`).
- Doctor seam:
  `a_grant_line_reads_its_binding_by_kind_and_assumes_no_native_provider`.
  `report_grant` is given an `mcp` authority built past the fence, a native
  authority with its dialect removed, and the loaded native authority. The
  first two return `None`, the third returns `Some(("codex", "web-search"))`.
  `healthy` is false, and the three lines are exact: the MCP `MISSING` line,
  the missing-binding `MISSING` line, and the native `ok` line.
- The old compile refusal is unchanged and still pinned by
  `an_mcp_grant_refuses_until_slice_two_even_unused_and_a_hands_grant_is_reserved`
  (absent and `[]` offices, the reserved hands grant) and, in doctor, by
  `an_unbuilt_or_invalid_grant_is_one_failing_line_and_the_rest_still_prints`
  and `a_failing_grant_takes_no_neighbour_with_it_and_fabricates_no_denial`.
  All three pass unedited.

### Mutations

Each mutation was compiled and run with
`cargo test -p brokkr-runtime --all-features --locked --lib capabilities::tests`
and, where marked, with
`cargo test -p brokkr-cli --all-features --locked --lib a_grant_line_reads`.
Each was then restored.

| # | Compiling mutation | Failing tests (assertion line) |
| --- | --- | --- |
| M1 | `bound`'s `Mcp` arm returns `Ok(("", ""))` (empty-provider fallback) | `an_mcp_grant_at_the_resolver…` (binding.rs:40); doctor seam (capability_tests.rs:943, left `Some(("", ""))`, right `None`); the fence's own `an_mcp_grant_refuses…` (tests.rs:73) and `the_gate_check_reads…` (gate_class.rs:164) |
| M2 | `holding` substitutes a native binding: `bound(..).unwrap_or((provider, capability))` | `an_mcp_grant_at_the_resolver…` (binding.rs:58): left "…but provider 'test-native' declares no native capability 'library-docs' serving it…", right the MCP cause; 41 others pass |
| M3 | `holding` indexes again: `let dialect = &self.dialects[capability]` | `a_missing_binding…` panics "no entry found for key" at capabilities.rs:1587, inside the `requires` call of binding.rs:95 (its binding assertion at :86 passed first) |
| M4 | `Authority::binding` indexes: `&self.dialects[capability]` | `a_missing_binding…` panics "no entry found for key" at binding.rs:98, from its first assertion (:86); doctor seam panics at the same site (marked run) |
| M5 | doctor restores the expect: `Ok(native.expect("a grant that loaded is bound to a provider"))` | doctor seam panics at doctor.rs:1110: "a grant that loaded is bound to a provider: Mcp { realm: \"private\", capability: \"library-docs\", dialect: \"docs-mcp\" }" (marked run) |
| M6 | doctor reports the failing line `ok` instead of `missing` | doctor seam fails `assert!(!report.healthy)` (capability_tests.rs:947) (marked run) |

M1 and M4 remove behaviour under both seams. M2 and M3 bind the resolver's
two former index sites, M5 the doctor's former `expect`, and M6 its failing
line. The second visit applied all six again with the Edit tool and got the
same failing tests at the same lines and with the same messages as the table.
After restoring them, `git diff -- crates/` was empty, the capabilities
filter passed 42 of 42 and `--lib doctor::` passed 77 of 77.

### Gates observed

The third visit ran every gate below on the tree this section's commit
carries, and each result is what that run printed.

| Gate | Observed on the U2 tree |
| --- | --- |
| Format and clippy (`-D warnings`, all targets and features) | No diff and no warning. `report_capabilities` still fulfils its baseline `too_many_lines` and `excessive_nesting` expectations. |
| `brokkr-runtime`, all test binaries | 27 binaries `ok`: lib 740, `capability_launch` 68, `budgets` 4, `witness_digests` 6 unblessed, so no witness or compose pin moved and none needed to. |
| `brokkr-cli`, every binary | All `ok`: lib 627 (1 ignored), with `suppressions` (no count moved), `ratchets`, `layering` and the three heap tests. |
| The other workspace crates | Every result `ok`, none FAILED. |
| `compile --bundle bundles/self` | Compiles. |
| `openspec validate --all --strict` | 20 passed, 0 failed. |
| `typos --hidden` | No finding. |
| `quality/ratchet.sh files`, `clones`, `api` | "file size holds", "duplication holds", "public API holds". |
| `quality/ratchet.sh baselines origin/main` | Refuses one raise, `public-api/brokkr-runtime.txt` 1,432 to 1,449 items. The operator ruled it on 2026-10-04, and the pull request carries the Ruling line. |
| `scripts/measure-budgets.sh` | 101 prompt sites, 327 packages, heap peaks unchanged. Nothing U2 moved needs a new budget (see below). |
| `too_many_lines`, re-measured as measure.sh's step 5 does | Touched entries equal the committed listing: `report_capabilities` 184 at `doctor.rs:875` (was 254 at `:868`), `native_plan` 256 at `capabilities.rs:1869` (was `:1895`), and test entries 115 at `capability_tests.rs:1328` and 102 at `capabilities/tests.rs:627` and `:1732`. `report_grant`, `bound` and the new tests are under 100 lines. |
| File lines (`wc -l`) | `capabilities.rs` 2,337 (was 2,363), `binding.rs` 106 (new), `doctor.rs` 1,572 (its baseline), `capability_tests.rs` 1,975 (ceiling 2,000), `capabilities/tests.rs` 2,159 (was 2,160), `tests/binding.rs` 115 (new). |

The budget measure rewrote three files that U2 does not touch, so the second
visit restored them. Its dates moved, and 52 review, judge and author prompt
sites measured 13 bytes under their budgets. U2 changes no prompt input,
which makes that 13-byte gap an earlier change on main, left for a later
re-measure. Step 5 also places untouched entries (`init.rs`, `cli/src/tests.rs`,
`bundle/tests.rs`) a few lines from their committed lines. That drift is also
main's, and U2 leaves it.

### Pending

`bash scripts/coverage-exact.sh`, the cyclomatic ratchet on its LCOV, and
remote Linux and macOS CI are pending for a capable host or CI. None of them
has passed.

## U5c implementation evidence (tasks 22.1–23.2)

Run `0065-slice-two-unit-u5c-see-the--d6380594` built this unit on
2026-10-04 on branch `s2/U5c`, cut from main at `4c65f6a1`. The next realms
version after v7 is allocated here as **v8**: `forge.realms/v8`,
`contracts/realms.v8.schema.json`.

### The change, file by file

Production stays inside the row's three files.

`crates/brokkr-core/src/realms/grants.rs` is new (244 lines) and is now the
one home of the grant. `CapabilityGrant`, its `value()` serialization and
`reaches`, `GRANT_KEYS` and the parser moved here from `realms.rs`, which
re-exports the public names, so `brokkr_core::realms::CapabilityGrant` and
`GRANT_KEYS` keep their paths for runtime and doctor. The parser is now
version-aware. A grant carries `retention: GrantRetention`, a closed enum
with three variants. `Unreserved` is any v6 or v7 grant, where `retain` is
not the engine's key. `Inherit` is a v8 grant with `retain` left out, and
`Veto` is a v8 grant writing `retain: false`. `GrantRetention::reserved_keys`,
private to `grants.rs` since the second visit, is the reserved-key selection: v6 and v7 give `GRANT_KEYS`, the three keys
as before, and v8 gives those three plus `retain`. The parser filters
restrictions through that selection, so a v6 or v7 `retain` stays a
restriction and a v8 veto never becomes one. `value()` writes
`retain: false` back for a veto, so the manifest's pin of the grant keeps it.
Under v8 any other `retain` value refuses with CR1's cause, naming the realm
and the capability. The private `GrantError` (`thiserror`) replaces the old
`Result<_, String>`. Its first five variants render the old refusal text
byte for byte, which the unedited
`a_malformed_grant_is_refused_naming_the_realm_the_capability_and_the_field`
proves. `parse_grants` maps the error to `RealmsError::Invalid` at that
edge, which kept the call in `RealmMap::of` to one line.

`crates/brokkr-core/src/realms.rs` adds `SCHEMA_V8` to `SCHEMAS`, to the
duplicate-key strict set and to the unknown-version refusal. It shrinks from
918 to 808 lines. `RealmMap::of` keeps its 218-line baseline, measured at
`realms.rs:514`.

`contracts/realms.v8.schema.json` is v7's bytes with four changes: the
version, `$id`, title and description; the grant description's reserved-key
list; and one new grant property, `retain: {"const": false}`. No frozen file
moved. v7 is now pinned in `FROZEN` at
`ed10a6ba4610668408cc326b593403f22d8abb5310304a5f1f6d74bd6b480d03`, and
`contracts/README.md` documents v8.

### Test edits outside the owning suites

These edits are mechanical, and the new field or the newly allocated label
forces each one:

- `crates/brokkr-core/src/realms/tests.rs` is `realms.rs`'s own unit
  module. Its `CapabilityGrant` literal gains `retention`. The
  unknown-label probe moves from `v8` to `v9`, and the eight-label refusal
  text is re-pinned.
- `crates/brokkr-runtime/src/agents/tests.rs` (`try_resolve_on`) gains the
  `retention` field. This file sits over the test ceiling at its 5,839-line
  baseline, so its `tools` read became a single `from_value` of the removed
  key, which reads an absent key as `None` exactly as before. The file is
  now 5,838 lines.
- `crates/brokkr-cli/src/doctor/capability_tests.rs` gains the `retention`
  field (1,975 to 1,976 lines, under the 2,000 ceiling).
- `crates/brokkr-cli/tests/realms.rs`: the "future version" probe in
  `a_missing_or_malformed_map_refuses_before_any_seat_spawns` used
  `forge.realms/v8`. It failed once v8 existed (exit `Some(0)`, expected
  `Some(1)`), and it now uses `v9`.

### Scenarios and the tests that own them

| Scenario | Test (`crates/brokkr-core/tests/realms.rs` unless named) |
| --- | --- |
| Legacy round trip; v6/v7 `retain` is a restriction; omitted and empty lists stay apart (22.1) | `an_older_grant_keeps_retain_as_a_restriction_and_round_trips_exactly` |
| v8 `retain: false` is `Veto`, never a restriction, kept in `value()`; omission is `Inherit` (23.1) | `a_v8_grant_reads_retain_false_as_the_veto_and_omission_as_inherit` |
| Bad veto values `true`, `null`, `{}`, `"false"`, `0` refuse with CR1's exact cause, rendered text pinned once (23.1) | `a_v8_retain_other_than_false_is_refused_by_realm_and_capability` |
| v8 keeps v7's provisional offices, reads as v7 apart from the version, and refuses a duplicated key (23.1) | `a_v8_map_keeps_the_v7_provisional_offices_and_its_refusals` |
| The v8 contract is v7 plus `retain` const false; v7 still admits any `retain` as a restriction; the v7 label is refused under v8 | `frozen_contracts.rs` `the_v8_realm_schema_reserves_only_the_retention_veto` |
| Frozen pins do not move; v7 is now pinned | `frozen_contracts.rs` `the_frozen_contracts_and_the_corpus_keep_their_exact_bytes` |

### Removal mutations

Each mutation was applied with the Edit tool, run with
`cargo test -p brokkr-core --test realms` (M6 and M7 with
`cargo test -p brokkr-runtime --test frozen_contracts`), and then restored.
M1–M6 were applied twice: once on the first tree, and again on the tree
committed here, after `cargo fmt` and the `grants_of` split. The second
pass failed the same tests with the same values. The lines below are from
that pass; core lines are in `crates/brokkr-core/tests/realms.rs`. M7 ran
once.

| # | Compiling mutation | What failed (file:line, observed) |
| --- | --- | --- |
| M1 | `reserving = !older_than(schema, SCHEMA_V6)`: v6/v7 reserve `retain` | `an_older_grant…` (:34, left `Veto`, right `Unreserved`); `a_v8_map_keeps…` (:142, left `Inherit`, right `Unreserved`) |
| M2 | `Unreserved.reserved_keys()` returns the four v8 keys | `an_older_grant…` (:35, left `["dialect", "tools", "offices", "retain"]`, right the three) |
| M3 | `value()` never writes the veto back (`&& false`) | `a_v8_grant_reads…` (:80; `value()` lacked `"retain": Bool(false)`, as in the first pass) |
| M4 | a non-false v8 `retain` is read as `Veto` | `a_v8_retain_other_than_false…` (:100, `unwrap_err` on `Ok`) |
| M5 | `SCHEMA_V8` left out of the duplicate-key strict set | `a_v8_map_keeps…` (:150, `unwrap_err` on `Ok`: last-wins) |
| M6 | v8 contract's `retain` widened to `{"type": "boolean"}` | `the_v8_realm_schema…` (frozen_contracts.rs:745, left `Some(Null)`, right `Some(Bool(false))`) |
| M7 | one byte appended to v7's title | `the_frozen_contracts…` (frozen_contracts.rs:145, "contracts/realms.v7.schema.json bytes moved") |

After all seven were restored, `sha256sum` of v7 printed the pinned digest
again, and both suites passed.

### Gates on the committed tree

| Gate | Result |
| --- | --- |
| `cargo fmt --all -- --check`; workspace clippy with `-D warnings` | Clean. |
| `brokkr-core` | lib 105, `realms` 4, the other three binaries ok. |
| `brokkr-runtime` | lib 740 and every integration binary ok, `frozen_contracts` 12. `witness_digests` passed 6 unblessed, so no witness or compose pin moved. |
| `brokkr-cli` | lib 627 (1 ignored); `--tests --no-fail-fast` gave 44 result lines, none FAILED. |
| `compile --bundle bundles/self` | Exit 0. |
| `quality/ratchet.sh files`, `clones`, `api` | "file size holds", "duplication holds", "public API holds". A first clones run found the two v7/v8 tests' shared contract read; it is now one helper, `published`. |
| `quality/ratchet.sh baselines 4c65f6a1` | Refuses one raise: `public-api/brokkr-core.txt` 581 to 589 items (`GrantRetention` and its method, the `retention` field, `SCHEMA_V8`). This needs the operator's `Ruling:` line. The runtime snapshot changes only the rendered path of `CapabilityContext::grants`' element type, with no count change. |
| `too_many_lines` (step 5, core and runtime) | `RealmMap::of` 218 at `realms.rs:514` (was `:613`). `the_new_contracts_exist…` 109 at `frozen_contracts.rs:169` (was listed at `:156`). The listing records both lines. |
| File lines | `realms.rs` 808 (was 918), `grants.rs` 244, `tests/realms.rs` 156, `frozen_contracts.rs` 962, `realms/tests.rs` 1,181. |

### Second visit: the review's return

Review returned three findings on `0665fe5f`; this visit answers each.

**F1, the v7 label check bound nothing on its own.** The last assertion of
`the_v8_realm_schema_reserves_only_the_retention_veto` labelled the map v7
while `retain` still held `{}` from the loop, so the map was invalid for a
second reason. The test now restores `retain: false`, asserts the map is
valid under v8 ("the label's v8 control"), and only then relabels it v7. To
bind it, the validator was built for one run from a copy of the v8 schema
with `properties.schema` set to `{}`, which removes the label check and
nothing else. With the control in place, the test failed at
`frozen_contracts.rs:788`, "v7 label under the v8 schema". With the same
mutation and the two control lines removed (the first visit's shape), the
test passed: 1 passed. Both edits were restored and the test passed again.

**F2, `reserved_keys` had no consumer outside its module.** It is now a
private method; the public-API snapshot drops its `impl` and method lines,
and `quality/ratchet.sh api` printed "public API holds". The two integration
tests no longer call it; the version-aware selection is bound through
parsing. With `Unreserved` given the four v8 keys, `an_older_grant…` failed
at `tests/realms.rs:35` (left `Object {}`, right `{"retain": Bool(false)}`).
With `Inherit` and `Veto` given the three v7 keys,
`a_v8_grant_reads…` failed at `:68` (left with `"retain": Bool(false)`,
right `{"allow": …}` alone). Both were restored. This supersedes M2 above.
`ratchet.sh baselines 4c65f6a1` now reports `public-api/brokkr-core.txt`
587 items (was 581): `GrantRetention`, its three variants, the `retention`
field and `SCHEMA_V8`. It still needs the operator's `Ruling:` line.

**F3, the shared validation was incomplete.** On the second visit's tree,
every workspace crate's suite ran with `--all-features --locked`. The six
smaller crates (`core`, `store`, `protocol`, `view`, `bridge`,
`seatbelt-probe`) ran in one invocation, and every result line was ok.
`brokkr-runtime --no-fail-fast` printed 28 result lines, all ok, lib 740
and `frozen_contracts` 12. `brokkr-cli --lib` gave 627 passed and 1
ignored; `--tests --no-fail-fast` gave 45 ok result lines and none other.
Workspace clippy with `-D warnings`, fmt, `compile --bundle bundles/self`,
`openspec validate --all --strict` (20 passed), `typos --hidden`,
`git diff --check` and `ratchet.sh files`/`clones`/`api` held as well.
`scripts/measure-budgets.sh` ran ("measured 101 prompt sites, 327
packages", peaks claude 2011418, codex 11230208, dsh 4197432). Packages and
peaks equal the committed files. The prompt sites came out lower than
their budgets, mostly by 13 bytes. `git diff --stat 4c65f6a1` over `agents`,
`recipes`, `bundles`, `adapters`, `dialects`, `realms.json` and
`tests/budgets.rs` is empty, so this branch moves no prompt input, and the
three files were restored rather than re-dated here.

### Pending

Exact coverage (`scripts/coverage-exact.sh`), the cyclomatic ratchet on its
LCOV, and remote Linux and macOS CI have not run. Runtime still reserves the three v6 keys in
`reserved_fault` and does not read `retention`. Making that check
version-aware and carrying the veto into the holding is U5a's work (tasks
24.1–24.2).

## U1a implementation evidence (tasks 2.1–2.2)

Run `0065-slice-two-unit-u1a-see-the--4577bfbd` built this unit on
2026-10-04 on branch `s2/U1a`, cut from main at `3f2a84e6`. It is a pure
extraction: no behavior, refusal text, server acceptance or public path
changed, and the MCP compile fence is untouched.

### What moved

Production stays inside the row's two files. The new private module
`crates/brokkr-protocol/src/native_controls/mcp.rs` (237 lines) is now the
home of the single-server transport and its parser checks, moved byte for
byte with their doc comments:

| Item | Was in `native_controls.rs` | Visibility now |
| --- | --- | --- |
| `toml_basic`, the one TOML basic-string encoder | private fn | `pub(super)` |
| `Transport` and its `document`, `arguments`, `command`, `expand`, `effects` | pub struct | re-exported as `native_controls::Transport` |
| `untransported`, the composer's whole-transport check | private fn | `pub(super)`, imported by the parent |
| `authored_server_conflict`, the authored-server structural check | pub fn | re-exported as `native_controls::authored_server_conflict` |

One function is new: `Transport::carried_by`. It holds the R1 comparison
that `sealed_inputs` used to write inline (the hands carry exactly the
transport's owed effects, counted and matched). `sealed_inputs` now calls it
in one line, and that is the only change at a production caller. Every other
caller (`authored_conflict`, `compose_or_exclude`, `check_final`, the
adapters' serving builder and the runtime's `Transport` literals) is
unchanged, because the re-exports keep the old paths. `quality/ratchet.sh
api` printed "public API holds".

`native_controls.rs` shrinks from 4,539 to 4,320 lines. `sealed_inputs`
shrinks from 334 to 326 lines (now at `:2104`), and `decode_record` (`:1073`)
and `compose_or_exclude` (`:3735`) keep their sizes at new lines.
`quality/file-lines.txt` and `quality/too-many-lines.txt` record these
measurements. The second listing was re-read from a forced
`clippy::too_many_lines` run on `brokkr-protocol`, and no test entry moved.
No suppression was added or removed.

There is one test edit. `native_controls/tests.rs` sits over the test
ceiling at its 13,152-line baseline, so its first line became
`use super::{mcp::toml_basic, *};`. The file still has 13,152 lines. No
assertion, fixture or test name changed. No test was added: the row's proof
is that the existing hands-only exact-state and authored-option tests still
pass unchanged and fail when the moved code is removed.

### Removal mutations

Each mutation was applied to `mcp.rs` with the Edit tool. It was run with
`cargo test -p brokkr-protocol --all-features --locked --lib` (and with
M1, also with `cargo test -p brokkr-runtime --test capability_launch`), and
then restored. The lib suite has 631 tests.

| # | Compiling mutation in `mcp.rs` | What failed (observed) |
| --- | --- | --- |
| M1 | `untransported` never names a missing part (`!present && false`) | 2 failed: `typed_hands_carry_their_whole_transport_through_the_composer` (tests.rs:7454, left `Ok(Composed { … })`, right the "carry no strict MCP configuration ('--strict-mcp-config')" refusal); `every_generated_sealed_state_checks_or_refuses_exactly` (:12368). Runtime: `a_compiled_unmeasured_site_carries_its_typed_hands_into_its_compose` (capability_launch.rs:9759, left the "final tool list with no selection mapping" refusal, right the lanetally "carry no strict MCP config…" refusal) |
| M2 | `authored_server_conflict` ignores `mcp_servers` assignments | 3 failed: `an_authored_capability_server_is_refused_by_provenance_and_never_by_its_bytes` (:1048, left `Ok(None)`, right the "carry '-c mcp_servers'" refusal); `every_refusal_line_is_one_bounded_line_naming_no_payload` (:3693); adapters `an_authored_capability_server_is_refused_at_launch_in_the_drivers_own_voice` (adapters/tests.rs:15513) |
| M3 | `authored_server_conflict` ignores `mcp__` include/allow values | 4 failed: `a_tool_list_that_admits_a_native_tool_is_an_authored_control` (:483, left `Ok(None)`, right "carry '--allowedTools mcp__*'"); `an_authored_capability_server_is_refused_by_provenance…` (:1048); adapters `…refused_at_launch_in_the_drivers_own_voice` (:15543) and `an_authored_plugin_or_later_list_value_is_refused_at_the_final_command` (:16249) |
| M4 | `carried_by` admits extra controls (`>=` for `==` on the count) | 1 failed: `the_hands_are_bound_to_the_engines_transport_and_nothing_else` (:9269, left the "does not carry '--config'" refusal, right "is sealed with hands that are not the engine's workspace hands") |
| M5 | Codex's owed approval mode `"approve"` becomes `"prompt"` in `effects` | 8 failed, including `the_hands_are_bound_to_the_engines_transport_and_nothing_else` (:9186), `every_sealed_sandbox_contribution_names_one_admitted_class` (:10133, left the "not the engine's workspace hands" refusal, right `Ok(Checked { … })`), `every_single_mutation_of_a_checked_command_refuses` (:12831) and adapters `a_cold_codex_argv_carries_the_off_pair_exactly_when_search_is_not_held` (:14926) |
| M6 | `toml_basic` leaves `\` unescaped | 3 failed: `every_codex_transport_value_decodes_as_toml_to_exactly_itself` (:7253, left `"/opt/a\\u002fb/brokkr"`, right `"/opt/a\\\\u002fb/brokkr"`), `every_generated_sealed_state_checks_or_refuses_exactly` (:12358), `every_single_mutation_of_a_checked_command_refuses` (:12831) |
| M7 | `expand` drops its non-UTF-8 workdir refusal | 1 failed: `every_codex_transport_value_decodes_as_toml_to_exactly_itself` (:7335, left `Some([… lossy workdir …])`, right `None`) |

After all seven were restored, the protocol suite passed again: lib 631
passed and 1 ignored, and `hands_exits` 6, `secret_drop` 1 and the doc-test 1
passed. One earlier run of that suite, on the restored tree, failed
`oneshot::tests::a_produced_attempt_carries_its_result_and_checkpoints`.
That test does not touch this module. It passed on the immediate full re-run
and when the four `oneshot` tests ran alone, so it is recorded here as a
flake, not as a result.

### Gates on the tree

| Gate | Result |
| --- | --- |
| Formatting and workspace clippy (`-D warnings`, all targets and features, locked) | Clean. |
| `brokkr-protocol` | All four binaries ok (lib 631, 1 ignored). |
| `brokkr-runtime --tests --no-fail-fast` | 27 result lines, all ok: lib 740, `capability_launch` 68, `frozen_contracts` 12. `witness_digests` passed 6 without blessing, so no witness or compose pin moved. |
| `brokkr-cli --tests --no-fail-fast` | 45 result lines, all ok (lib 627, 1 ignored), and the suppressions test passed. |
| `brokkr-core`, `-store`, `-view`, `-bridge`, `-seatbelt-probe` | Every result line ok. |
| `compile --bundle bundles/self` | Exit 0. |
| `quality/ratchet.sh files`, `clones`, `api`, `listings` | "file size holds", "duplication holds", "public API holds"; listings read. |
| `quality/ratchet.sh baselines 3f2a84e6` | "no baseline raised since 3f2a84e6". |
| `openspec validate --all --strict`; `typos --hidden`; `git diff --check` | 20 passed, 0 failed; clean; clean. |

### Pending

Exact coverage (`scripts/coverage-exact.sh`) has not run, and neither has the
cyclomatic ratchet on its LCOV. The moved functions keep their measured
complexity (`toml_basic` 12, `authored_server_conflict` 10, `untransported`
4, `Transport::effects` 4), and each will appear under `mcp.rs` as a new
entry under the ceiling of 15. The CRAP baseline was not re-measured here.
Remote Linux and macOS CI have not run.

## U5a implementation evidence (tasks 20.1–21.2, 24.1–24.2)

Run `0065-slice-two-unit-u5a-see-the--2501fc4e` built this unit on
2026-10-04 on branch `s2/U5a`, cut from main at `fb61b9f8`. The code is
commit `280e2807`. The removal mutations below ran on `e853e0a1`, which
differs from `280e2807` only in `Authority::load`. That function's reservation
call was rewritten without a `match` so its complexity stays under the ceiling.
M3, which runs through that call, was repeated on `280e2807` and failed at
the same line. The MCP compile fence is unchanged: every structurally valid
`mcp` grant is still refused realm-wide with the U2 text.

### The change, file by file

The three production files are the row's own.

`capabilities/dialect.rs` is new (398 lines). It holds the tool-dialect loader
that `capabilities.rs` used to hold. The loader still judges the document
against the frozen v1 contract first, in the same two steps (shared fields,
then the kind's branch). It still hashes the bytes `read_document` bound
inside the operator root. After that it reads the same `Value` once into a
private `Document`, whose `kind` is the serde-tagged `DialectKind`. The
`mcp` variant now carries `McpServer` with `connection` (`Connection::Stdio`
argv or `Connection::Url`), `version`, `secrets` and `retained`, which reads
false when omitted. `ToolDialect` also gains `sends: Sends` (description and
`seat_composed`) and a typed `egress: EgressClass`, absent reading
`Uncontracted`. Both typed readings sit behind `expect`s, because the contract
has already admitted the document. The egress invariant is bound by a test,
M12. Checks now run in SC1's order: contract, name, the kind's secret
references, then the restriction schema. `reserved_fault` is now
`reserved_key`, which takes the reserved set as an argument and returns the
key it finds. The loader passes `GRANT_KEYS`. `ToolDialect::reserves(GrantRetention)`
passes `retain` for `Inherit` and `Veto`, and nothing for `Unreserved`, so a
v6 or v7 grant keeps `retain` as a dialect restriction.

`capabilities/binding.rs` grows from 106 to 262 lines.

- `bound` now returns a private three-way `Binding`: native, MCP, or
  `Unbound::Hands`. `native` keeps the old pair-or-`Unbound` reading for the
  compile fence and `Authority::binding` (doctor).
- `carried` is the resolver's reading. A native binding returns its pair. An
  `mcp` binding checks `representable` first (SC4: capability and dialect
  names of at most 128 bytes, and tools of at most 256 bytes in seat-record
  v5's tool vocabulary). It then returns the SC1 cause for a URL or an argv
  secret reference, or the fence's own `Unbound::Mcp` text for an executable
  stdio connection, because no broker exists until U9b.
- The new causes live in the typed `Unserved` enum.
- `Retention { declared, realm: GrantRetention }` is built by
  `Retention::of(dialect, grant)`. Its `effective()` is false under `Veto`,
  and `declared` otherwise.

`capabilities.rs` shrinks from 2,337 to 2,087 lines. It re-exports the new
types and adds `Holding::retention`, filled by `Retention::of` in
`holding()`. `holding()` reads `binding::carried`, and `Authority::load`
checks `dialect.reserves(grant.retention)` right after loading each grant's
dialect. That check gets the same `realm '…': capability '…':` prefix as the
other dialect faults. `Authority` gains a private field `loaded: Loaded`, a
private zero-sized marker type.

Reading SC4 against "unchanged native behavior": the identity bound applies
only to `mcp` holdings. A native dialect's tools are harness patterns such as
`Bash(/private/…:*)`. `capability_launch.rs` pins 200-byte capability names
and pattern tools on native seats, and a first draft that bounded native
holdings too failed three of those tests. M10 shows that putting the bound
back on native holdings fails this unit's native-unchanged assertion.

### The two carried residuals

U2 residual. `Authority` has a private field again. Its type, `Loaded`, is
private to `capabilities`, so outside that module (other crates included) a
struct literal no longer compiles. `#[non_exhaustive]` was considered and
rejected because it fences other crates only. Clippy's
`manual_non_exhaustive` refuses a private `()` field, which is why the field
has its own marker type. No duplicate binding map was restored. The test
`only_the_loaders_construct_an_authority` walks every production source under
`crates/*/src` and finds exactly two constructions: `load` and `nothing`. It
skips test modules and `tests/` directories, and it does not depend on a
compile failure. Doctor's tests still mutate a loaded `Authority`'s public
fields, which is unaffected. The public-API effect: cargo public-api lists
private fields in no form, so `Authority`'s lines in
`quality/public-api/brokkr-runtime.txt` are unchanged. Semantically, external
code can no longer build one with a literal.

U5c residual. These are the production readers of `GrantRetention` and
`CapabilityGrant::retention` in brokkr-runtime:

- `Authority::load` passes `grant.retention` to `ToolDialect::reserves`, which
  matches all three variants.
- `holding()` calls `Retention::of`, which stores `grant.retention`.
- `Retention::effective` matches the variants. The repair visit below made
  it test-only until manifest v12 (U5f) reads it, so it is no longer a
  production reader; the two above remain.

### Test edits outside the owning suites

`crates/brokkr-cli/tests/status_pages.rs:250` changes one pattern, from
`DialectKind::Mcp` to `DialectKind::Mcp(_)`, because the variant now carries
the server. The file's line count is unchanged. No assertion moved.

Inside the owning suite, `capabilities/tests.rs` drops from 2,159 to 2,121
lines. Its `every_kind_loads_as_data…` now asserts the whole typed
`McpServer` for both connection forms. Two tests moved to the new child
module `capabilities/tests/dialect_policy.rs` unchanged:
`an_mcp_launch_names_only_the_secrets…` and
`the_embedded_tool_dialect_contract_is_the_published_file`, which also gained
an egress-vocabulary assertion. `context()` keeps its exact bytes, so its
jscpd fingerprint does not move. U2's `past_the_fence` helper in
`tests/binding.rs` now takes the connection, and its `MCP`/`WHO` constants are
`pub(super)` for the sibling module. The other two owning suites,
`agents/tests.rs` and `bundle/agent_tests.rs`, did not need to change.

### Scenarios and the tests that own them

All tests below are in `capabilities/tests/dialect_policy.rs` unless named
otherwise.

| Scenario | Test |
| --- | --- |
| SC1: typed MCP fields kept, digest over the bound bytes, every declared fact moves identity, rotating a secret's value does not, no launch | `an_mcp_dialect_keeps_every_executable_fact_and_runs_nothing` |
| SC1: contract before references before restriction schema; nonboolean `retained` and native `retained` refused by the contract | `a_secret_reference_is_judged_before_the_restriction_schema_and_nothing_defaults` |
| Extraction parity: schema, duplicate, containment and reference refusals | the existing `tests.rs` suite, unchanged and green, plus the moved `an_mcp_launch_names_only_the_secrets…` |
| SC2: v8 inherit and veto refuse a schema claiming `retain` (properties, `allOf` required, `$ref` to `anyOf`, pattern, and since the repair visit a `dependencies` schema and a `$ref` from one); v6/v7 admit it; nested `allow.retain` is not reserved | `a_v8_grant_reserves_retain_through_references_composition_and_dependencies` (renamed by the repair visit) |
| SC2/D11: v6 and v7 `retain: false` stays a restriction, `Unreserved`, requires refuses and wants drops with the nonempty-restriction cause | `an_older_retain_stays_a_restriction_and_reaches_only_the_nonempty_restriction_outcomes` |
| CR1: the four outcomes (false, false, true, false), plus declared false and v6 `Unreserved`; a native holding carries its grant's veto; an idle vetoed grant stays pinned; veto and inherit differ in grant identity | `a_holding_carries_the_grants_veto_and_only_a_retaining_dialect_without_one_retains` |
| SC1 past the fence: URL and argv-reference causes in GP1's required and optional forms; an executable stdio meets the fence text; the real compile still refuses with the fence | `past_the_fence_an_unexecutable_connection_answers_with_its_own_cause` |
| SC4: 128/129-byte names, 256/257-byte tools and vocabulary; an `mcp` identity is checked before its connection; a native 129-byte dialect name is still held | `an_mcp_identity_is_carried_whole_or_refused_and_a_native_one_is_unchanged` |
| U2 residual | `only_the_loaders_construct_an_authority` |

### Removal mutations

Each mutation was a compiling edit to production code (M12 edits
`agents.rs`, which the test's invariant reads). Each was run with
`cargo test -p brokkr-runtime --lib capabilities::tests` and then restored
with `git checkout`. Line numbers are in `dialect_policy.rs` unless named.

| # | Mutation | Failed at (observed) |
| --- | --- | --- |
| M1 | `retained` gets `skip_deserializing` | 3 tests: `an_mcp_dialect_keeps…` :124 (`kind == Mcp(server)`); `tests.rs` :473 `every_kind_loads…`; `a_holding_carries…` :324 (the outcome table) |
| M2 | Restriction schema judged before the secret references | `a_secret_reference_is_judged_before…` :173 (left is the reserved-`tools` refusal, right is the `OTHER` reference cause) |
| M3 | `reserves` admits every version | `a_v8_grant_reserves…` :221 (`Ok("claims-retain")` against the reserved-key cause); repeated on `280e2807`, same line |
| M3b | `reserves` reserves `retain` under `Unreserved` too | `an_older_retain_stays…` :255 (`Err(…reserved grant key 'retain')` against `Ok((Unreserved, {"retain": false}))`); `a_v8_grant_reserves…` :224 |
| M4 | `effective()` returns `declared` under `Veto` | `a_holding_carries…` :324 |
| M5 | `holding()` builds the retention from an `Inherit` copy of the grant | `a_holding_carries…` :347 (`(Inherit, false)` against `(Veto, false)`) |
| M6 | `carried` answers a URL with the fence | `past_the_fence…` :391 |
| M7 | The argv-reference guard can never hold | `past_the_fence…` :391, on the reference row (left is the fence text) |
| M8 | `carried` skips `representable` | `an_mcp_identity…` :457 (the URL cause where the identity cause belongs) |
| M9 | The tool bound is loosened to 257 bytes | `an_mcp_identity…` :432 (the `fits` table) |
| M10 | `holding()` bounds native identities too | `an_mcp_identity…` :453 (`Err(…cannot be represented…)` against `Ok(long)`) |
| M11 | A third `Authority` literal (`copied`) in `binding.rs` | `only_the_loaders…` :493 (left lists `binding.rs copied`) |
| M12 | `EgressClass::parse` loses `contracted` | `the_embedded_tool_dialect_contract…` :96 |
| M13 | Undeclared secret names admitted unless `secrets` is empty | `a_secret_reference_is_judged_before…` :173; the moved `an_mcp_launch_names…` at its pinned `unwrap_err` (:50) |
| M14 | Loading runs the stdio argv | `an_mcp_dialect_keeps…` :158 (`(MCP, true)` against `(MCP, false)`: the marker file was written) |

After restoration the filtered suite passed 50 of 50 again.

### Measurements

`quality/file-lines.txt` records the five moved counts and the two new files.
`quality/too-many-lines.txt` moves three entries: `native_plan` (256 lines,
now `:1619`) and the two over-100 tests in `capabilities/tests.rs` (now
`:598` and `:1703`). The same forced clippy run shows many unrelated entries
already off main's listing, and those were left alone. No suppression was
added. `cargo crap` on an LCOV from the lib suite measures `holding` at 16
(baseline 17), `Authority::load` at 13 (baseline 15), `ToolDialect::load` at
10, `carried` at 8, and every other new function at 6 or less. The same
LCOV, from `cargo llvm-cov --branch`, shows `binding.rs` (113 lines, 12
branches) and `dialect.rs` (226 lines, 18 branches) fully covered by the lib
suite alone. No line `capabilities.rs` leaves uncovered lies in `load`,
`nothing` or `holding`. Witness and compose pins did not move:
`witness_digests` passed 6 without blessing. Prompt bytes, crate count and
heap are not inputs this unit touches. The `budgets` target passed inside
the runtime suite.

### Gates on `280e2807`

| Gate | Observed |
| --- | --- |
| rustfmt check, and clippy across the workspace with warnings denied | Both clean. |
| `cargo test -p brokkr-runtime --all-features --locked --no-fail-fast` | 28 result lines, every one ok (lib 748, `capability_launch` 68). |
| The same for `brokkr-cli` | 46 result lines, every one ok (lib 627, 1 ignored). |
| core, store, protocol, view, bridge, seatbelt-probe together | 24 result lines, none failed. |
| `compile --bundle bundles/self` (and `bundles/verify` before the amend) | Compiled and printed the manifest. |
| `quality/ratchet.sh` files, clones and api | Each reported that it holds, after `quality/public-api/brokkr-runtime.txt` took the reviewed diff. |
| `quality/ratchet.sh baselines fb61b9f8` | Refused: `public-api/brokkr-runtime.txt: 1466 public items (was 1449)`. The PR needs the operator's `Ruling:` line. |
| strict OpenSpec, `typos --hidden`, whitespace check against `fb61b9f8` | 20 of 20 valid; no typo; no whitespace finding. |

### Assumptions

The first visit read the task's "egress minimum" as preserving the dialect's
typed egress. Its council refused that reading as a substitute, and the
repair visit below withdrew it: MB4's comparison is not built, and tasks
24.1 and 24.2 are open again. The v8 collision text uses the dialect's name
for SC2's `<dialect>`, prefixed like every other dialect fault.

### Pending

`scripts/coverage-exact.sh` has not run, so the workspace-wide 100% result
and the full-LCOV CRAP ratchet are owed to CI or a capable host. The
operator's ruling on the public-API raise is owed. Remote Linux and macOS CI
have not run.

## U5a repair visit after the security hold (tasks 20.1–21.2; 24.1–24.2 reopened)

Run `0065-slice-two-unit-u5a-see-the--8009db7c` received the first visit's
whole implementation uncommitted on `fb61b9f8` and repaired its council's
findings on 2026-10-04. It stayed inside the row's three production files.
Its tests are in `capabilities/tests/dialect_policy.rs`.

### What changed

The reservation scan, `reserved_key` in `dialect.rs`, read the keys of a
`dependencies` object but never the schemas they map to. Draft-07 applies
such a schema to the same instance whenever its key is present, so
`{"dependencies": {"allow": {"properties": {"retain": {}}}}}`, or a local
`$ref` to that schema from a dependency, claimed `retain` at the grant's
own level and a v8 grant with `allow` and `retain: false` loaded as `Veto`.
The scan now follows each object-valued dependency exactly as it follows
`allOf` and the other same-instance keywords, `$ref`s and cycle guard
included. Both callers share the scan, so a key every version reserves
(`tools`) hidden the same way is now refused by the loader under every
realm version, with slice one's text. A list-valued dependency names keys
the grant must then carry. It is still not scanned: a reserved key there
leaves the grant unable to satisfy the schema, so it is refused as an
invalid restriction rather than admitted.

`McpServer::undeclared` now returns the typed `Undeclared` (`Malformed`,
`Secret`), and `conforms` returns `Outside`. Each `Display` is the old
text, and `ToolDialect::load` alone renders them to its `String`. Neither
variant holds the argument it judged. `Retention::effective` is
`#[cfg(test)] pub(super)`. Its production reader is manifest v12 (U5f),
which removes the `cfg`. `Authority`'s public fields were left alone: making
them read-only would also change `bundle.rs:1616`, which reads
`authority.definitions` directly, and that file is not in this row.

### The binding minimum is oversized

MB4 compares the dialect's egress with the operator's binding minimum, the
bundle's `egress_minimum`. `parse_egress_minimum` reads that minimum in
`Bundle::assemble` (`bundle.rs:1560`), after `Authority::load` (`:1477`).
It reaches neither `Serving` (built at `bundle.rs:5983–6018`) nor
`CapabilityContext` (built in `launch.rs:390`). `Authority::assess`, which
`brokkr doctor` calls (`crates/brokkr-cli/src/doctor.rs:970`), has no
bundle and so no minimum at all. Any comparison inside the three files
would therefore use a minimum nobody supplied. A default would make
`local` bundles fail open and `uncontracted` bundles fail closed, and the
commission forbids a substitute. Tasks 24.1 and 24.2 are unticked. The
split the controller needs is this:

| Part | Files | Tasks |
| --- | --- | --- |
| Carry the minimum into the authority and compare it in `carried`, with the exact cause and the requires/wants proofs | `capabilities.rs`, `capabilities/binding.rs`, `bundle.rs` | 24.1's egress-minimum clause, 24.2 |
| Choose the minimum doctor's hypothetical `assess` judges against | `crates/brokkr-cli/src/doctor.rs` | 24.1's egress-minimum clause, if the row must cover doctor |

Everything else in 24.1 is built and proved: version-aware reservation,
inherit/veto in the holding, D11, and the identity bound.

### Scenarios added or changed

| Requirement | Test | Assertion |
| --- | --- | --- |
| SC2, D3 step 1 | `a_v8_grant_reserves_retain_through_references_composition_and_dependencies` | 36 rows in one comparison: 8 restriction forms × (v8 inherit, v8 veto, v6, v7), plus hidden `tools` under the same four. The `moved` list must be empty, and any mutation names every row it moves. |
| SC1, ruling 8 | `the_dialect_edge_checks_refuse_with_typed_variants` | `undeclared` gives exact `Ok(())`, `Malformed { index: 1 }`, `Secret { index: 2, name: "OTHER_KEY" }`, and `Ok(())` for a URL. `conforms` gives `Ok(())`, and exact `Outside` for userinfo. The moved `an_mcp_launch_names…` still pins the rendered text. |
| SC4, ruling 9 | `an_mcp_identity_is_carried_whole_or_refused_and_a_native_one_is_unchanged` | The `fits` table compares exact `Ok(())` / `Err(Unserved::Identity)` per row, not booleans. |

### Removal mutations

Each was a compiling production edit, run with
`cargo test -p brokkr-runtime --lib <test>`, then restored by hand. Line
numbers were observed before the final `cargo fmt`.

| # | Mutation | Observed failure |
| --- | --- | --- |
| R1 | Dependency scan skips a schema that is a `$ref` | reservation `moved` (:321) lists exactly the two v8 `dependency reference` rows, `Ok` against the collision |
| R2 | Dependency scan follows only a `$ref` | `moved` lists the two v8 `dependency` rows and all four `hidden tools` rows |
| R3 | `properties` no longer names a key | `moved` lists the direct `properties` control's two v8 rows, plus `reference`, `dependency`, `dependency reference` and `hidden tools`, which are built on it |
| R4 | `reserves` scans under `Unreserved` too | `moved` lists all twelve v6/v7 rows of the six claiming forms, `Err` against `Ok` |
| R5 | The scan also descends into `properties` values | `moved` lists the four v8 `nested` and `nested in a dependency` rows |
| R6 | `representable` refuses with `Unserved::Url` | identity table (:522), `[Ok(()), Err(Url), …]` against `[Ok(()), Err(Identity), …]` |
| R7 | `Malformed` reports `index + 1` | typed variants (:92), `index: 2` against `index: 1` |
| R8 | `conforms` skips the kind's branch | typed variants (:117), `Ok(())` against the exact `Outside` |

After each restore, the filtered capabilities suite passed 51 of 51.

### Gates on the repaired tree

These ran on the working tree that this visit's commit records. rustfmt's
check was clean, and so was workspace clippy with warnings denied. The
runtime suite (`--all-features --locked --no-fail-fast`) printed 28 result
lines, all ok, with lib at 749 and `witness_digests` passing without a
bless. The CLI suite printed 46, all ok. The other workspace crates
together printed 24, none failing. `bundles/self` and `bundles/verify` both
compiled. OpenSpec strict validated 20 of 20. `typos --hidden` and the
whitespace check found nothing. `cargo +1.88 check` across the workspace
finished.

On the ratchets, `files`, `clones` and `api` each held. `baselines
fb61b9f8` refused with `public-api/brokkr-runtime.txt: 1464 public items
(was 1449)`, which needs the operator's ruling. The count is two below the
first visit's 1466, because `impl Retention` and `Retention::effective` left
the public API.

`quality/file-lines.txt` moved `binding.rs` to 265, `dialect.rs` to 439 and
`dialect_policy.rs` to 645. The forced `too_many_lines` clippy run lists no
new function in this unit's files. `scripts/measure-budgets.sh` rewrote the
budget files, lowering several prompt budgets by 13 bytes. None of those
inputs is this unit's, so its output was discarded; the `budgets` target
had already passed in the runtime suite. Exact coverage
(`scripts/coverage-exact.sh`), the operator's API ruling, and remote Linux
and macOS CI are pending.

## U5a judging visit after the split (tasks 20.1–21.2, 24.1–24.2)

Run `0065-slice-two-unit-u5a-see-the--b545fd43` received the first visit,
the repair and the controller's split documents uncommitted on `fb61b9f8`
and judged the whole unit on 2026-10-04 against U5a's amended row. The
binding minimum stays with U5a2 (tasks 24.3–24.4) and doctor's comparison
with U9a. Nothing here builds either. Read-only `Authority` accessors (low 5)
stay out of scope. The operator's ruling on the public API, 1449 to 1464, is
carried on the PR's `Ruling:` line, and this visit adds no public item.

### The HIGH repair holds, and three further gaps are closed

The hold's HIGH repair held when re-observed. Making the scan skip every
object-valued `dependencies` schema made the reservation test's `moved`
list (`dialect_policy.rs:356`) name exactly the v8 `dependency` and
`dependency reference` rows and all four `hidden tools` rows.

A temporary probe test, deleted afterwards, then loaded crafted restriction
schemas under a v8 grant and compiled each with `jsonschema::draft7`. It
found three ways a schema still escaped the walk in `reserved_key`, and one
way the walk crashed. All four are fixed in `dialect.rs`.

The first two are cases where the validator resolves a reference somewhere
other than where the walk looks. In the first, a subschema carries its own
`$id` (`http://brokkr.invalid/inner`) and `"$ref": "#/definitions/z"`
beneath it. The validator resolves that reference against the subschema's
own `definitions`, whose `z` requires `retain`. The walk resolves it from
the file's root, to a harmless `z`. In the second, the reference is
`#/definitions/a%25`. The validator decodes it to the definition `a%`,
which claims `retain`. A JSON pointer reads the literal key `a%25`, which
is harmless. In both cases the v8 grant passed the reservation check and
reached the later restriction validation. The validator's own error
(`'/definitions/z/required'`, `'/definitions/a%/required'`) showed it had
read the claiming definition. The loader now refuses either form with
`resolution_fault`, before the walk runs. That check refuses an `$id` below
the root and a percent-encoded `#` reference anywhere, data included,
because a reference can land in data. A root `$id` rebases nothing and
still loads.

The third gap is list-valued dependencies. The repair visit left
`{"dependencies": {"allow": ["retain"]}}` unscanned, on the reasoning that
the grant could not satisfy it. That holds only for a grant that writes
`allow`. The probe showed that a v8 grant without `allow` loaded the dialect
with no refusal, although the dialect requires `retain` whenever `allow`
is present, exactly as `required` would. The walk now reads a
dependency list's entries as names, the same way it reads `required`. A
reserved key written there is therefore refused under v8 with SC2's text,
and a key every version reserves is refused under every version.

The crash is a `$ref` that `reference_fault` never vetted, because it sits
under a data keyword (`{"$ref": "#/const", "const": {"$ref": "#/nope"}}`)
or under a `dependencies` entry named like one (`{"dependencies": {"enum":
{"$ref": "#/nope"}}}`). The walk reached such a reference and panicked in
its `expect("reference_fault resolved every reference first")`. The first
form was already possible on main. The repair's dependency walk added the
second. The walk now skips a target that names nothing, without panicking.
The compile that follows then refuses the dialect with `is not valid
draft-07: Pointer '/nope' does not exist`.

### Tests and removal mutations

The reservation test gained a `dependency list` row and now compares 40
outcomes. A new test,
`a_reference_the_validator_would_resolve_elsewhere_is_refused_and_none_panics`,
compares five loads in one assertion (`dialect_policy.rs:377`): the
nested-`$id` refusal, the percent-encoded refusal, the compile refusal for
both data-reached references, and the root-`$id` control, which is
`Ok("d")`. Each mutation in the table below compiled and was run with
`cargo test -p brokkr-runtime --lib dialect_policy`. The file was then
restored from a saved copy.

| Mutation | Observed failure |
| --- | --- |
| The nested-`$id` refusal never fires | the new test at :377, `Ok("d")` where the `$id` refusal belongs |
| A root `$id` is refused too | the new test at :377, the root row refused where `Ok("d")` belongs |
| The percent-encoding check never matches | the new test at :377, `Ok("d")` where the percent-encoded refusal belongs |
| The old `expect` is restored | the new test panicked at `dialect.rs:285` |
| Dependency lists are not read as names | `moved` at :356 lists the two v8 `dependency list` rows, refused as an invalid restriction (`'/dependencies'`) where SC2's collision belongs |
| `effective()` returns `declared` under `Veto` | the CR1 outcome table, `(true, Veto, true)` where `(true, Veto, false)` belongs |
| `Retention::of` reads a `Veto` grant as `Inherit` | the outcome table at :495, both veto rows reading `Inherit` |

After the restores, the capabilities suite passed 52 of 52.

### 24.1 and 24.2, observed

Task 24.1's amended scope is observed. The reservation check follows the
grant's version and reaches every same-instance schema: composition,
`$ref`, schema-valued and list-valued dependencies. A reference is
resolved as the validator would resolve it, or the dialect is refused.
Older maps keep `retain` as a restriction and reach only D11's
refuse/drop outcomes (`an_older_retain_stays…`). The holding carries the
typed `Retention`, and its four outcomes are pinned. A veto on a grant no
office reaches stays pinned in the manifest, and the grant stays idle. MCP
identities are bounded and refused, never truncated. No production line
in this unit opens a secret store or reads a clearance: a search of the
staged diff found `store_set` only in the test fixture that proves the
loader never reads the store. Task 24.2's assertions and mutations are the
ones recorded in this section and in the two above. Both tasks are ticked.

### Gates on this visit's tree

These gates ran on the tree that this visit's commit records. rustfmt's
check and workspace clippy with warnings denied were both clean. The
runtime suite printed 28 result lines, every one ok, with lib at 750 and
`witness_digests` passing without a bless. The witness and compose inputs
did not move. The CLI suite was all ok, with its lib at 627 and 1 ignored.
The other workspace crates together printed 24 result lines, all ok.
`bundles/self` compiled and printed its manifest. `cargo +1.88 check`
across the workspace finished. OpenSpec strict validated 20 of 20, and
neither `typos --hidden` nor the staged and unstaged whitespace checks
found anything. `quality/ratchet.sh files`, `clones` and `api` each held.
`quality/file-lines.txt` now records `dialect.rs` at 475 lines and
`dialect_policy.rs` at 696. No function grew past 100 lines, and no
suppression was added. Exact coverage (`scripts/coverage-exact.sh`) and
remote Linux and macOS CI are still pending.

## U5a second repair visit after the second security hold (tasks 20.1–21.2, 24.1–24.2)

Judging run `0065-slice-two-unit-u5a-see-the--b545fd43` stopped again with
REVIEW-SECURITY-HOLD. The controller traced both holds to one cause. The
reservation walk was a hand-written approximation of the draft-07
validator: it resolved `$ref` with `serde_json`'s pointer and walked
keywords the validator never applies, so every disagreement between them
was either a bypass or a false refusal. Run
`0065-slice-two-unit-u5a-see-the--49537c74` replaced the approximation
rather than patching it. The judging visit's `reference_fault`,
`resolution_fault` and `reserved_key` are gone, and so is its test
`a_reference_the_validator_would_resolve_elsewhere_is_refused_and_none_panics`,
whose `$id` and percent refusals no longer exist.

### One walk, resolved by the validator's own resolver

`dialect.rs` now indexes the restriction schema the way
`jsonschema::draft7::new` does. It builds a `jsonschema::Registry` (the
crate's re-export of its `referencing` registry) for draft 7, keyed by the
root `$id` or jsonschema's default base `json-schema:///`, with the default
retriever, which fetches nothing. The walk keeps a base URI per schema and
rebases it on every subschema through the registry resolver's
`in_subresource`, as the compiler does. Every `$ref` it follows goes through
that resolver's `lookup`. The walk and the validator therefore land a
reference on the same schema by construction. No pointer is decoded by hand.

The walk visits only draft-07's applicators reachable from the root:
composition, `not`, `if`/`then`/`else`, `items`, `additionalItems`,
`contains`, `additionalProperties`, `propertyNames`, `properties`,
`patternProperties` and the schema form of `dependencies`, plus `$ref`
through the resolver. A schema carrying `$ref` is read as draft 7 reads it:
only the reference applies, and its siblings are ignored. Data keywords and
definitions that no reachable reference names are never read. A schema
describes the grant's own top level when it is reached from the root through
same-instance applicators and references. Only there are reserved keys
looked for, by `properties`, `required`, `dependencies` keys and list
entries, and a matching `patternProperties` pattern, which is judged by the
validator as before. A followed target is remembered together with the level
it was met at. A definition first met below the top level is therefore
walked again when it is reached at the top. A temporary probe found that
dropping the level from that memory is a bypass, and the test below now pins
it.

A target the resolver finds outside the dialect's own bytes is refused. In
practice that is the built-in draft-07 meta-schema, which the validator
would have resolved without fetching. A schema whose references cannot be
indexed without fetching, or one with an `$id` that is not a URI, is refused
before the walk starts. One `expect` documents that invariant: once indexing
has succeeded, rebasing onto a reachable `$id` cannot fail. The comment on it
names the test that pins the four ways a bad `$id` could hide, behind a
reference into data and below an applicator there. A mutation that let
indexing failures pass broke that test.

### What the validator applies, and only that

Refusals that were false before are now positive parity controls. Each of
the following loads, and the same draft-07 validator compiles it: an `$id`
or a percent-bearing `$ref` inside `default` or `examples`, an `$id` or a
dangling `$ref` in a definition nothing references, an `$id` on an applied
subschema, a pointer the validator percent-decodes (`#/definitions/a%20b`),
the default base spelled out, and a plain-name anchor. The first four of
these loaded on main and were refused by the judging visit.

### Diagnostics name a keyword and a place

Refusals are a typed `SchemaFault` (ruling 8). The text names only the
keyword and the walk's own location, an RFC 6901 escaped pointer along the
path the walk took, such as `/properties/a~1b~0c/$ref`. Authored `$schema`,
`$ref` and `$id` values never appear. A `$schema` other than draft-07 is
refused with one fixed sentence, whatever was written. The compile's own
error text, which follows the walk, is unchanged.

### Seat-record's tool vocabulary, and SC4's refusal

`binding.rs` keeps its identifier check. A new test reads
`contracts/seat-record.v5.schema.json` and compares `representable` with the
published checkpoint `tool` pattern for every ASCII character, in both the
first position and a later one (258 probes). They agree on every probe. The
256-byte bound is SC4's, which seat-record v6 widens from v5's 80. v6 is not
yet published, so no second file holds that number.

In `capabilities.rs`, `holding` now returns a private `Unheld`: either a
`Cause`, which the ask's strength settles, or SC4's identity, which refuses
the compile as `<site>: capability call identity cannot be represented by
seat-record v6` whether the ask wants or requires. The remaining `Unserved`
variants are still matched by name, and the MCP compile fence still answers
first.

### The v8 `retain` reservation is still restated

Item 4 asked `dialect.rs` to read the v8 reservation from brokkr-core
instead of naming `retain` itself. Core's `GrantRetention::reserved_keys`
and its list are private to `brokkr-core/src/realms/grants.rs`, a file
outside this row. Reading them would mean making that function public and
re-measuring `quality/public-api/brokkr-core.txt`. This visit leaves the
`RETAIN` constant in place and reports the unit oversized by that one file.

### Tests and removal mutations

The new tests are in `dialect_policy.rs`.
`the_reservation_walk_lands_every_reference_where_the_validator_does` writes
each disagreeing shape (`~~0`, `~~1`, `~~~~0`, a percent-encoded pointer, an
inner `$id`) twice. In the first copy the `retain` claim sits where the
validator lands; in the second it sits where a plain pointer lands. Two more
rows cover a claim beside a `$ref` and a direct claim. For each of the 12
rows the test records whether the validator itself applies the claim, the
v8 outcome and the v6 outcome. The v8 grant is refused exactly where the
validator applies the claim, and v6 admits every row.
`a_key_every_version_reserves_is_found_where_the_validator_lands` does the
same for `dialect`, `tools` and `offices` at load (30 rows), and adds the
two-level definition. `what_the_validator_never_applies_is_never_walked`
holds the eight positive controls, and
`a_refused_restriction_schema_is_named_by_keyword_and_location_only` holds
the typed faults. Each mutation below compiled, was run with
`cargo test -p brokkr-runtime --lib capabilities::`, and was then reverted.
After every revert the filter passed 56 of 56.

| Mutation | Observed failure |
| --- | --- |
| Use `serde_json`'s pointer target instead of the resolver's | `dialect_policy.rs:444` and `:480` |
| Skip `in_subresource` rebasing | `:444` and `:480` (the inner `$id` rows) |
| Walk `examples` and `definitions` | `:518`, rows 2 and 5 refused as resolving to nothing |
| Walk the siblings of a `$ref` | `:444` only (the row with a claim beside a `$ref`) |
| Drop the containment check | `:532`, `None` where `Outside` belongs |
| Do not escape pointer segments | `:532`, `/properties/a/b~c/$ref` |
| Let an indexing failure pass | `:532` (both `Unindexed` rows `None`) and `tests.rs:602`, which received the compile's echoed URI |
| Ignore the level in the followed memory | `:486`, `Ok("d")` where `tools` is refused |
| Drop an unrepresentable identity as a want | `:820`, the wanted ask resolved `Ok` (dropped) where the refusal belongs |
| Add `+` to the tool vocabulary | `:863`, `(258, ["a+"])`; no other test noticed |

A variant that returned nothing when rebasing failed broke no test, which
is how this visit found the invariant. That path is now the `expect`
described above. A branch-coverage run of the capabilities tests on the
pinned nightly (`cargo llvm-cov --branch`) covered all of `dialect.rs`
(297 lines, 49 functions, 26 branches) and all of `binding.rs`. It also
covered every line this visit changed in `capabilities.rs`.

### Gates on this visit's tree

This visit's tree passed rustfmt's check and workspace clippy with
`-D warnings`. The runtime lib passed 754, and every runtime integration
binary passed, `witness_digests` included without a bless. The witness and
compose inputs therefore did not move. The CLI package ran 45 test binaries
and its doc-tests, with no failure. `bundles/self` compiled, and
`cargo +1.88 check` over the whole workspace finished. OpenSpec strict
validated 20 of 20, and `typos --hidden` and `git diff --check` reported
nothing. The `files`, `clones` and `api` ratchets held after
`quality/file-lines.txt`, `quality/too-many-lines.txt` and
`quality/suppressions.txt` were re-measured. The capabilities test that had
been 102 lines shrank under 100, so its `#[expect(clippy::too_many_lines)]`
was removed, and the test count of that lint fell from 257 to 256. Against
`fb61b9f8`, the `baselines` ratchet reports brokkr-runtime's public API at
1464 items, up from 1449. That is the growth the operator ruled; this visit
added no public item. Exact coverage on the full gate and remote CI on both
hosts are pending.

## U5a judging visit after the second repair (tasks 20.1–21.2, 24.1–24.2)

Run `0065-slice-two-unit-u5a-see-the--3425eacc` received the second repair
staged and uncommitted on `fb61b9f8` and judged all of U5a against its
amended row and both holds on 2026-10-04. It found nothing to fix, so no
production or test line differs from the repair that the previous section
describes. This section records what was observed, not what was changed.

### What was judged

The typed reading agrees with the frozen contract. `McpServer` requires
`connection`, `version` and `secrets` and defaults `retained` to false,
exactly as the contract's `mcp` branch does. `classes`, `egress` and
`restrictions` are optional at the top level there as here. So the loader's
`expect` after `conforms` cannot fire on a document the contract admits.
`reserves` refuses whenever the walk returns any fault, so a failure to
index or resolve at that stage would refuse, not admit.

A temporary probe test, deleted before the commit, gave eight further
shapes to `embedded_schema_fault` and to `jsonschema::draft7::new`. A
relative reference to a subschema named by its own `$id`, a root
self-reference under `allOf`, a schema-valued dependency, a two-step
reference chain ending in a `patternProperties` match, and an `if`/`then`
reference all reported the reserved key, while the validator compiled each
of them. A claim reached only through `items` was not reported, which is
right because it describes an array element rather than the grant. A root
`$id` beside a `$ref` naming an absolute URI, and an absolute reference to
an inner `$id`, were refused as `Unindexed`. The validator refused to
compile both of them as well. No shape escaped the walk or panicked.

### Mutations re-observed on this tree

Four mutations were applied one at a time to the staged files. Each run
used `cargo test -p brokkr-runtime --lib capabilities::`, and the file was
restored from a saved copy after each. With the tree back at the candidate,
the filter passed 56 of 56 and `git diff` was empty.

| Mutation | Test and assertion that failed |
| --- | --- |
| The followed-target memory ignores the level it was met at | `a_key_every_version_reserves_is_found_where_the_validator_lands`, `dialect_policy.rs:486`: `Ok("d")` where SC2's `tools` refusal belongs |
| `Walk::node` keeps the parent's base instead of rebasing through `in_subresource` | the same test at `:480`, and `the_reservation_walk_lands_every_reference_where_the_validator_does` at `:444` |
| `reserves` treats an `Inherit` grant as unreserved | `:444` and `a_v8_grant_reserves_retain_through_references_composition_and_dependencies` at `:356` |
| `Retention::of` reads a `Veto` grant as `Inherit` | `a_holding_carries_the_grants_veto_and_only_a_retaining_dialect_without_one_retains`, the four-outcome table at `:676` |

### 24.1 and 24.2

The amended scope of 24.1 was seen to hold on this tree. The reservation
check follows the grant's version: the third mutation shows that
`forge.realms/v8` without a veto still reserves `retain`. The check reaches
through references and composition, and the first two mutations bind that.
The holding carries the typed `Retention`, and its four outcomes, false,
false, true and false, are pinned by the fourth mutation's test. An older
map keeps `retain` as a nonempty restriction
(`an_older_retain_stays_a_restriction_and_reaches_only_the_nonempty_restriction_outcomes`).
A veto on a grant that no office reaches is pinned in the manifest while
the grant stays idle (`dialect_policy.rs:706`). The binding minimum is not
built here; that is U5a2's work. The ticks on 20.1–21.2 and 24.1–24.2
therefore stand.

### Findings carried, not fixed

Two low findings remain. Neither can be closed within the row's three
production files, and neither weakens a refusal. The first is second-hold
item 4: `dialect.rs` still writes the v8 `retain` reservation itself instead
of reading core's list (decision 0071 ruling 5). Fixing it needs
`brokkr-core/src/realms/grants.rs`, and the controller has carried it to
U10a's removal audits. The second is `Retention::effective`, which exists
only under `cfg(test)` until manifest v12 (U5f) reads it in production.
That is ruling 6 bent knowingly, and its doc comment names U5f as the
reader that lifts the `cfg`.

### Gates observed on this tree

Formatting and workspace clippy, with warnings as errors, both passed. In
brokkr-runtime the library ran 754 tests, its 26 integration binaries all
passed, and `witness_digests` passed 6 without a bless, so no pin moved.
In brokkr-cli the library ran 627 with one ignored, and its 43
integration binaries all passed. The other workspace crates printed 24
result lines, all passing. `bundles/self` compiled to digest `11c7d0e7…`.
The 1.88 toolchain checked the whole workspace with every target and
feature. OpenSpec strict validation passed 20 of 20. Neither `typos
--hidden` nor the staged and unstaged whitespace checks found anything.
The `files`, `clones` and `api` ratchets held. The `baselines` ratchet
against `fb61b9f8` raised only brokkr-runtime's public API, from 1449 to
1464, which is the operator's ruled growth; the PR names it on its
`Ruling:` line. The recorded line counts and `too-many-lines` anchors match
the files. Exact coverage and remote CI on Linux and macOS are still
pending.

`scripts/measure-budgets.sh` also ran on this tree. It measured 327
packages and the same three heap peaks the committed files hold, so only
the date in their notes would change. 52 prompt sites measured exactly 13
bytes under their budgets, and no site measured over. U5a's diff changes
no office text, house rule, dialect prose or notice that reaches a prompt.
The shrink therefore looks like drift that main already carries. That is
an inference: this visit did not measure `fb61b9f8` by itself. All three
budget files were restored, so the PR moves no budget.

## U5a third repair visit: reserved keys are stripped, not searched for (tasks 20.1–21.2, 24.1–24.2)

Run `0065-slice-two-unit-u5a-see-the--e3a5f3b6` implemented the operator's
ruling of 2026-10-04 (the second addendum in
[operator-ruling-2026-10-03.md](operator-ruling-2026-10-03.md), SC2's new
scenario and amended task 24.1) on the staged U5a tree, based on
`1b9e222d`. Three council rounds had each found a construct where the
reservation walk and the draft-07 validator disagreed. The ruling moves the
guarantee to the strip that brokkr-core already performs, so this visit
removed the walk instead of repairing it again.

### What changed

Only `capabilities/dialect.rs` changed in production. The walk is gone:
`Walk`, `APPLICATORS`, `Applies`, `applied`, `escaped`, `holds`, the
`patternProperties` probe, and the `Nowhere` and `Outside` faults, with the
reference resolution and rebasing they needed. In their place, `claimed`
reads four root forms only: the keys of root `properties` and root
`dependencies`, and the entries of root `required` and of any list in root
`dependencies`. A dependency schema, composition, a conditional and a
reference are not entered. `embedded_schema_fault` checks the draft, then
asks the validator's own resolver to index the schema without fetching
(`Unindexed`, unchanged), then applies `claimed` to the three keys every
grant-bearing version reserves. `reserves` applies `claimed` to `retain`
for an `Inherit` or `Veto` grant and to nothing for an `Unreserved` one.
The validator's compile of the whole schema is unchanged. Refusal texts
name the keyword or the reserved key and never the schema. Neither
`capabilities.rs`, `binding.rs` nor brokkr-core moved in this visit. Core's
`parse_grant` already validates only the grant's keys minus its version's
reserved keys, and `Authority::load` hands exactly those restrictions to the
validator.

Two loader outcomes change because the walk is gone, and the council
should weigh both. A local reference that names nothing is now refused by
the validator's compile, as "restriction schema is not valid draft-07:
Pointer '/definitions/nobody' does not exist". That text repeats the
authored pointer, as main's text did and as every compile error always
has. A reference to the validator's built-in draft-07 meta-schema now
loads, because the validator resolves it without a fetch; main and the walk
refused it as outside the file. A reference that would need a fetch is
still refused as `Unindexed`, and the crate is built without
`resolve-http` or `resolve-file`. The compile reported "`resolve-http`
feature or a custom resolver is required" for
`https://example.org/hosts.json` in a temporary probe deleted before the
commit. `tests.rs` pins both new outcomes.

### Tests

In `dialect_policy.rs`,
`a_direct_claim_of_a_reserved_key_is_refused_by_the_versions_that_reserve_it`
runs each of the four root forms for `dialect`, `tools`, `offices` and
`retain` under a v8 inheriting grant, a v8 veto and v6 and v7 vetoes, which
gives 64 rows. The three every-version keys refuse with their exact text
under every version. `retain` refuses under v8 with SC2's exact sentence.
Under v6 and v7 it loads, and the written `retain` stays a restriction.

`a_reserved_key_never_reaches_restriction_validation` is SC2's new
scenario. Four schemas forbid `retain` only through a reference, a
conditional, a dependency schema, or all three. For each one the test
records three things. First, the validator itself accepts the grant's
restrictions and refuses them with `retain: false` beside them, so the
claim is live. Second, the v8 veto loads as `(Veto, {"allow": …})`.
Third, the same grant under v6 is refused at `/retain` by the exact
clause. That v6 row is the in-test control showing that the indirect claim
decides a grant whenever `retain` reaches validation.

`an_indirect_claim_of_a_reserved_key_is_inert_however_it_is_reached` turns
the three holds' shapes into 88 rows: 22 shapes for each reserved key.
Each shape forbids the key with a `false` schema, and the grant writes
`dialect`, `tools`, `allow` and `retain: false`, so any claim that decided
a key would refuse the grant. The shapes are the `~~0`, `~~1` and
`~~~~0` pointer forms, the percent-encoded pointer and the `$id` rebasing
pair, each with the claim on the validator's target and on the decoy. They
also include an orphan `then`, an orphan `else`, an inactive `else`,
`additionalItems` with and without `items`, composition, a root `$ref`, a
pattern, a dependency schema, a dependency reference, a nested key, and a
claim beside a `$ref`. Every row loads as `(Veto, {"allow": …})`.
`what_the_validator_never_applies_is_never_refused` (renamed) keeps the
over-rejection shapes: `$id` in a default, unused definitions, and encoded
references in data. `a_refused_restriction_schema_names_no_authored_text`
replaces the location-naming test. It pins `Draft`, three `Unindexed` forms
and two `Reserved` forms, with `hunter2` written into each, and the four
hidden bad `$id`s. The walk-specific tests
`a_v8_grant_reserves_retain_through_references_composition_and_dependencies`,
`the_reservation_walk_lands_every_reference_where_the_validator_does` and
`a_key_every_version_reserves_is_found_where_the_validator_lands` are
gone. Their indirect refusals are now the inert rows above. In `tests.rs`,
`a_restriction_schema_stays_inside_its_file_and_off_the_engines_keys` now
lists the four direct forms and pins the two loader changes above.

### Removal mutations

Each mutation compiled and was run with
`cargo test -p brokkr-runtime --lib capabilities::`, then restored from a
saved copy. Line numbers are the ones observed before `cargo fmt` reflowed
the new tests. The tables now sit at `dialect_policy.rs:327`, `:391` and
`:492`, and the typed-fault array at `:547`. After every restore the
filter passed 56 of 56, and `git status` showed brokkr-core unchanged.

| Mutation | Test and assertion that failed |
| --- | --- |
| Removal control: core's `parse_grant` strips only `GRANT_KEYS`, so a v8 `retain` reaches validation | `a_reserved_key_never_reaches_restriction_validation` (the table at `:381`: every v8 row became the v6 refusal, e.g. "at '/retain': it does not satisfy '/definitions/forbids/properties/retain'"), `an_indirect_claim_…` (`:476`) and the four-outcome holding test (`:673`) |
| `claimed` ignores root `required` | `a_direct_claim_…` (`:324`) and `tests.rs:634` |
| `claimed` ignores lists in root `dependencies` | `a_direct_claim_…` (`:324`), `a_refused_restriction_schema_names_no_authored_text` (`:520`) and `tests.rs:634` |
| `claimed` ignores root `dependencies` keys | `a_direct_claim_…` (`:324`) and `tests.rs:634` |
| `reserves` treats an `Inherit` grant as unreserved | `a_direct_claim_…` (`:324`) only |
| `claimed` also searches `allOf` (an indirect claim decides again) | `an_indirect_claim_…` (`:476`) only |
| An indexing failure passes | `a_refused_restriction_schema_names_no_authored_text` (`:520`) and `tests.rs:602` |
| A reference to the draft-07 meta-schema is refused | `tests.rs:691`, the meta-schema pin |

### 24.1 and 24.2

The amended scope of 24.1 was observed on this tree. The four retention
outcomes are unchanged (`a_holding_carries_the_grants_veto_…`, which the
removal control also breaks). An older `retain` stays a restriction
(`an_older_retain_stays_a_restriction_…` and the v6/v7 rows above). Direct
claims are refused by the versions that reserve the key. A reserved key is
absent from restriction validation whatever the schema's composition, and
the removal control binds that. The inactive-grant row is unchanged. The
ticks on 20.1–21.2 and 24.1–24.2 stand on this evidence.

### Gates on this visit's tree

Formatting was applied, and workspace clippy with `-D warnings` finished
clean. The runtime library passed 754 tests. All 26 runtime integration
binaries passed, `witness_digests` among them without a bless, so no pin
moved. The CLI package printed 46 result lines and none failed. The other
workspace crates printed 24, all passing. `bundles/self` compiled to
digest `11c7d0e7…`, the same as before this visit. The 1.88 toolchain
checked the whole workspace with every target and feature. OpenSpec strict
validation passed 20 of 20. The `files`, `clones` and `api` ratchets held
after `quality/file-lines.txt` (`dialect.rs` 583 to 384, `tests.rs` 2113 to
2105, `dialect_policy.rs` 921 to 916) and the moved `too-many-lines.txt`
anchor (`tests.rs:1687`) were re-measured. The `baselines` ratchet against
`1b9e222d` reports brokkr-runtime's public API at 1464 items, up from 1449.
That is the operator's ruled figure, and the simpler check did not lower it
because every removed item was private to the module.
`scripts/measure-budgets.sh` measured 327 packages, the same three heap
peaks, and the same 13-byte shrink on the same prompt sites as the previous
visit. Its three files were restored, so the PR moves no budget. Exact
coverage and remote CI on Linux and macOS are pending.

### Carried

The `RETAIN` constant beside core's private reserved list goes to U10a,
and the binding minimum goes to U5a2, as the controller ruled. `claimed`
reads root `properties` even beside a root `$ref`, which draft-07 ignores.
That can over-refuse a claim the validator would never apply. It cannot
admit anything, because the strip already keeps the key out of validation,
and it follows the ruling's textual definition of a direct claim.

## U5a fourth repair visit: containment is restored beside the strip (tasks 20.1–21.2, 24.1–24.2)

Run `0065-slice-two-unit-u5a-see-the--400e7e2d` repaired the regression
that held the third visit's council (run `e3a5f3b6`). That council
confirmed the strip design, so this visit leaves the four direct root
forms, the version-aware strip, the typed retention and connection and
the MCP and hands fence as they were. When the third visit removed the
reservation walk, it also removed slice one's reference containment, and
the loader began to accept a `$ref` to the validator's built-in draft-07
meta-schema. The tree is still based on `1b9e222d`.

### What changed

Only `capabilities/dialect.rs` changed in production. `reference_fault`
is back with main's exact rule. It walks the whole restriction schema,
leaving out the data keywords `const`, `default`, `enum` and `examples`.
It refuses a `$ref` that does not begin with `#` as `SchemaFault::External`
and a fragment that names nothing under a plain JSON pointer as
`SchemaFault::Missing`. `embedded_schema_fault` runs it after the draft
check and before the resolver index and the direct-claim check, which is
main's order. The direct check stays independent of it. The two new
variants name the fault and never the reference: "has a '$ref' outside
the dialect file; a restriction schema is never fetched" and "has a '$ref'
that names nothing in the dialect file". The whole schema is still
compiled. A compile failure is now `SchemaFault::Uncompiled`, rendered as
"is not valid draft-07: the validator does not compile it", so the
compiler's own words never reach the operator. Before this visit a
missing pointer was echoed, for example "Pointer '/definitions/nobody'
does not exist". `Unindexed` is kept as a second layer behind containment.
`capabilities.rs`, `binding.rs` and brokkr-core did not move.

`tasks.md`'s preamble count is back to main's text, "The plan has 45 PRs
and 105 tasks." The U5a2 split is still recorded in design.md's
merge-order note, its row and section, tasks 24.3–24.4 and the ruling
addendum.

### Tests

`tests.rs`'s `a_restriction_schema_stays_inside_its_file_and_off_the_engines_keys`
now refuses both `https://example.org/hosts.json` and the draft-07
meta-schema with the exact External text. Before this visit it accepted
the meta-schema. It also pins the missing local `#/definitions/nobody` with
the exact Missing text and `{"type": "no-such-type"}` with the exact
Uncompiled text, in place of the old `starts_with` check.

In `dialect_policy.rs`, `every_reference_stays_inside_the_dialect_file_wherever_it_hides`
replaces `what_the_validator_never_applies_is_never_refused`, whose last
four rows were the contained-resolution positives this visit removes. The
new test has 11 rows. Each records whether the draft-07 validator compiles
the schema and what the loader answers. Four rows load: an `$id` in a
default, a `$ref` in `examples`, an unused definition's `$id` and an
applied `$id`. Three refuse as External: the meta-schema under `not`, an
external reference in an unused definition, and the validator's default
base `json-schema:///` spelled out. Four refuse as Missing: a pointer in an
unused definition, one inside an orphan `then`'s item list, a
percent-encoded pointer and a plain-name anchor. The validator compiles
six of the seven refused rows, which shows that containment does not
depend on resolution. No row's outcome contains its `hunter2` canary.

`a_refused_restriction_schema_names_no_authored_text` now expects
`External` and `Missing` for its external and missing-pointer rows. Its
loader row pins the direct `tools` claim beside a contained reference,
with the canary in a definition's data. `a_schema_the_compiler_refuses_is_named_by_its_fault_alone`
is new. It proves the compiler path on its own terms through two schemas
that pass every pre-compile check (`embedded_schema_fault` is `None`):
`{"type": "hunter2"}`, and a contained `$ref` into `const` whose data hides
a missing `#/definitions/hunter2`. The test also asserts that each
compiler error does contain `hunter2`, so the canary is live, and it pins
the loader's exact Uncompiled text. `an_indirect_requirement_of_retain_fails_a_v8_grant_closed`
is the optional pin. A dialect that requires `retain` only through
`allOf` refuses a v8 veto grant, "with an invalid restriction at '': it
does not satisfy '/allOf/0/required'", because the strip removed the key.

### Removal mutations

Each mutation compiled and ran under
`cargo test -p brokkr-runtime --lib capabilities::`, and each was undone
with the Edit tool. Afterwards `git diff` showed `dialect.rs` with only
this visit's change and brokkr-core unchanged, and the filter passed 58
of 58. Line numbers are the final tree's.

| Mutation | Test and assertion that failed |
| --- | --- |
| Containment removed (`reference_fault(..).filter(\|_\| false)`) | `every_reference_…` (`dialect_policy.rs:582`; all 7 refusal rows moved, 6 to a load and the unused external to `Unindexed`), `a_refused_restriction_schema_…` (`:595`; the External and Missing rows became `Unindexed` and `Reserved { key: "tools" }`) and `tests.rs:611` (the External text became `Unindexed`'s) |
| Data keywords walked as schemas | `every_reference_…` (`:582`; the `examples` row was refused as External), `a_schema_the_compiler_refuses_…` (the hidden `const` row became `Missing`) and `tests.rs` (the data-`$ref` row's `unwrap`) |
| The compile refusal rendered the compiler's error verbatim, as before this visit | `a_schema_the_compiler_refuses_…` (`:668`; the texts became "\"hunter2\" is not valid under any of the schemas listed in the 'anyOf' keyword" and "Pointer '/definitions/hunter2' does not exist") and `tests.rs:676` |
| Removal control in core: `parse_grant` strips only the first three reserved keys, so a v8 `retain` reaches validation | `an_indirect_requirement_of_retain_…` (it loaded as `(Veto, {"retain": false})`) and `an_indirect_claim_…` (`:492`) |

The data-keyword mutation was observed before `cargo fmt` and the
`tests.rs` compaction, so that row's line numbers are not the final
ones. The other three were run again, or first run, on the final layout.

### 24.1 and 24.2

The strip's proofs did not change, and this visit observed them again.
The 64-row direct-claim table, the 88-row inert table, the four retention
outcomes, the older-`retain` restriction and the inactive-grant row all
passed in the 58-test filter and in the full runtime library. The ticks
on 20.1–21.2 and 24.1–24.2 stand on this evidence and the containment
proofs above.

### Gates on this visit's tree

Formatting is clean, and workspace clippy with `-D warnings` finished
without a diagnostic. The runtime library passed 756 tests, and all 26
runtime integration binaries passed. `witness_digests` and `budgets` were
among them, without a bless, so no pin or budget moved. The CLI package
printed 46 result lines with no failure. The other workspace crates
printed 24, all passing. `bundles/self` compiled to digest `11c7d0e7…`,
unchanged. The 1.88 toolchain checked the whole workspace with every
target and feature. OpenSpec strict validation passed 20 of 20, `typos
--hidden` found nothing, and `git diff --check` was clean. The `files`,
`clones` and `api` ratchets held after `quality/file-lines.txt` was
re-measured: `dialect.rs` 384 to 427, `tests.rs` 2105 to 2103 (main 2159)
and `dialect_policy.rs` 916 to 1024. The `too-many-lines.txt` anchor moved
to `tests.rs:1685`. No new function crossed the ceiling, and the
suppression count is unchanged. A forced `too_many_lines` run on the local
clippy showed other differences only in files this unit does not touch
(`brokkr-core/src/policy.rs`, `brokkr-cli/src/tests.rs`,
`brokkr-protocol/src/hands.rs`, `agents/load.rs` and `bundle.rs`). Those
come from the measuring toolchain, so they are not re-baselined here. The
`baselines` ratchet against `1b9e222d` reports brokkr-runtime's public API
at 1464 items, up from 1449, the same figure as the last visit. The new
variants are `pub(super)`. The PR must name that ruling. Exact coverage
and remote CI on Linux and macOS are pending.

## U5a2: MCP egress against the bundle's binding minimum (tasks 24.3–24.4)

Run `0065-slice-two-unit-u5a2-see-the-e4801408` built U5a2 on branch
`s2/U5a2` from main at `819ee729`, with U5a landed. It touched the row's
three production files and no others.

### What changed

`Authority` carries a private `minimum`, the bundle's binding minimum
(decision 0036 ruling 4). `Authority::load` and `Authority::nothing` set
it to `ABSENT_EGRESS_MINIMUM`, which is `contracted`. That constant has one
home, in `capabilities/binding.rs`, and `bundle.rs`'s `parse_egress_minimum`
now returns it for an absent `egress_minimum` instead of restating the
class. `assemble` binds the parsed minimum with `with_minimum` straight
after parsing it. Every site resolution after that point gets the same
`&authority`: the agent context's candidates, the inline context's sites
and the unloaded-adapter path alike.

`carried` is now an `Authority` method in `binding.rs`. For an `mcp`
binding it answers in design D3 step 6's order: identity (SC4), then the
connection's SC1 causes (URL, argv reference), then the new
`Unserved::Below`. That cause applies when the dialect's typed egress is
below the minimum under `EgressClass`'s order, and reads "MCP dialect
'<dialect>' has egress '<class>' below binding minimum '<minimum>'". Only
then do the fence's own words come, until U9b. A native binding returns
before any of these checks, so its dialect's egress is never judged. The
mapping from `Unserved` to the resolver's `Unheld` moved beside the enum as
`Unserved::unheld`. It is an exhaustive match: SC4's identity refuses
whatever the strength, and every other cause, `Below` included, is the
dialect's incompatibility, which the ask's strength settles. A requires
refuses with the incompatible form, and a want drops with its notice and
"; no native denial is claimed". These are the forms U5a's SC1 causes
already take. The realm-wide fence in `Authority::load` did not move.

The shared `past_the_fence` fixture now declares `egress: contracted`,
which meets the absent minimum. Without that, its executable connection
met MB4 before the fence words that `an_mcp_grant_at_the_resolver_…` and
`past_the_fence_an_unexecutable_connection_…` pin. With it, both pin what
they pinned before. The inline codex seat that `an_inline_gate_…` wrote
inline became the `inline_codex` builder, which the new bundle tests share.

### Tests

Three tests were added to `capabilities/tests/binding.rs` and two to
`bundle/agent_tests/gate_tests.rs`. Those are submodules of the two owning
suites: `capabilities/tests.rs` and `bundle/agent_tests.rs` stand at or
over their baselines and did not grow.

| Test | What its one comparison holds |
| --- | --- |
| `an_mcp_dialect_below_the_binding_minimum_is_refused_or_dropped_and_at_or_above_it_passes` (`binding.rs:206`) | Nine rows, every minimum against every egress, with the relation written as data. Each row holds the exact required refusal, the want's empty holding and its exact notice. Below rows carry MB4's cause. At and above rows carry the fence's words. All rows are computed before one `assert_eq!`. |
| `an_absent_binding_minimum_is_contracted` (`binding.rs:216`) | With no minimum bound, `local` and `contracted` pass and `uncontracted` meets "below binding minimum 'contracted'". |
| `a_native_holding_is_unchanged_under_every_binding_minimum` (`binding.rs:248`) | `search-native`, whose egress is `uncontracted`, is held for requires and wants under all three minimums: `["web-search"]`, no notice, `--search-on`. |
| `every_mcp_grant_still_refuses_the_compile_under_every_binding_minimum` (`gate_tests.rs:406`) | 60 real compiles: absent, `local`, `contracted` and `uncontracted` minimums × three dialect egresses × five grants (unused, offices `[]`, office-excluded, asked at an agent, asked at an inline site). Each refuses with the fence's exact text. |
| `a_native_holding_is_unchanged_at_every_serving_path_under_every_binding_minimum` (`gate_tests.rs:443`) | Four minimums × the agent and inline paths. Each site holds `["web-search"]` with no notice. |

### Removal mutations

Each mutation compiled and ran under `cargo test -p brokkr-runtime --lib --
capabilities::tests::binding bundle::agent_tests::gate_tests`, and each was
undone with the Edit tool. The mutations ran on this visit's tree before
commit. Rows M1–M6 ran when the fence test had 48 rows, before the
inline-asked grant was added. That addition changed no test those rows
name.

| Mutation | Tests that failed |
| --- | --- |
| M1: the below-minimum arm deleted | the matrix (below rows) and the absent-minimum test |
| M2: `<` became `<=` | the matrix (at rows), the absent-minimum test and `an_mcp_grant_at_the_resolver_…`, whose fixture sits at the minimum |
| M3: `<` became `!=` | the matrix (above rows) and the absent-minimum test |
| M4: `with_minimum` ignored its argument | the matrix (the `local` and `uncontracted` minimum rows) |
| M5: `ABSENT_EGRESS_MINIMUM` set to `Local` | the absent-minimum test and `an_mcp_grant_at_the_resolver_…` |
| M6: natives judged too (the comparison moved ahead of the kind match) | both native tests, `an_mcp_grant_at_the_resolver_…`, `an_inline_gate_…` and `every_executable_form_…` |
| M7: the realm-wide fence loop in `Authority::load` removed | the fenced-compile test, all 48 rows and then all 60 |
| M8: `assemble`'s `with_minimum` line removed | on the first visit, none: the whole runtime library passed, 761 of 761 (bound on the return; see "The handoff, bound" below) |

On the first visit M8 was the residual this unit owed. Before U9b no compile can show which
minimum `assemble` bound, because the fence refuses every `mcp` grant
before a site is read. That is the inertness operator ruling 2026-10-03
accepts, and the commission forbids a test bypass. So the binding was
observed as a counterfactual instead. With M7 applied, the fenced-compile
test's asked rows ran the real compile through to resolution. At the
agent's candidate and at the inline site alike, a `local` minimum gave
"MCP dialect 'docs-mcp' has egress 'contracted' below binding minimum
'local'" for a `contracted` dialect. With M7 and M8 applied together, the
same rows lost the minimum: the `contracted` dialect met the fence's words,
and the `uncontracted` one was "below binding minimum 'contracted'". When
U9b lifts the fence, those asked rows become the committed binding of
this line.

### 24.3 and 24.4

The matrix, the absent minimum, the native controls at the resolver and
at both serving paths, and the 60-row fenced compile each passed on the
final tree. M1–M7 failed them and were restored. The first visit ticked
24.3 and 24.4 on that evidence with M8 carried to U9b. Review returned the
unit on exactly that gap; the return below binds M8, and the ticks now
stand on M1–M8.

### Gates on this visit's tree

Formatting is clean, and workspace clippy with `-D warnings` finished
without a diagnostic. The runtime library passed 761 tests, and all 26
runtime integration binaries passed. `witness_digests` and `budgets` were
among them, without a bless, so no pin or budget moved. The CLI package
printed 46 passing result lines and no failure. Core, protocol, view,
store and bridge passed too. `bundles/self` compiled to `11c7d0e7…`,
unchanged. The 1.88 toolchain checked the whole workspace. OpenSpec strict
validation passed 20 of 20, `typos --hidden` found nothing, and
`git diff --check` was clean. The `files`, `clones` and `api` ratchets held,
and `baselines` against `819ee729` reports nothing raised.
`quality/file-lines.txt` was re-measured: `capabilities.rs` 2109 to 2108,
`capabilities/binding.rs` 265 to 317, `capabilities/tests/binding.rs` 115
to 249 and `agent_tests/gate_tests.rs` 310 to 444. `bundle.rs` stayed at
7808. The `native_plan` anchor in `too-many-lines.txt` moved from 1641 to
1640. No new function crossed a ceiling, and no suppression was added.
Exact coverage and remote CI on Linux and macOS are pending.

### The handoff, bound (return from review)

Review (run `0065-slice-two-unit-u5a2-see-the-e4801408`, medium, ruling 9
of decision 0071) returned the unit because M8 was unbound: no committed
test failed when `assemble`'s `with_minimum` line was removed. The return
gives the minimum one home and makes the line observable through a real
compile, without lifting or bypassing the fence.

`assemble` now binds the parsed minimum to the authority first and reads
`egress_minimum` back from it through a new `Authority::minimum`
(`capabilities/binding.rs:258`, `bundle.rs:1560–1561`). The agent context
and the inline context take that value, so the route policy of decision
0036 ruling 4 and MB4's comparison in `carried` read the same field. The
bundle's two lines were replaced by two lines, so `bundle.rs` and
`assemble`'s length did not move. The route policy is not fenced, so a
compile shows which minimum the authority holds.

`the_authority_holds_the_binding_minimum_the_bundle_declares`
(`gate_tests.rs:451`) compiles the fixture's `worker` seat, on the
`contracted` claude route, with the secret `TOKEN` under an absent,
`local`, `contracted` and `uncontracted` minimum. Its one `each_row`
comparison holds "compiled" for three rows and, for `local`, the exact
refusal "bundle: seat 'work' declares secret bindings ["TOKEN"] but seats
driver 'claude' on its own declared destination, whose egress class is
contracted; this bundle binds no secret below local (decision 0021 ruling
4 as enacted by 0036 ruling 4 — an undeclared class is uncontracted, and
'egress_minimum' is where the operator rules the bar)".

| Mutation | Tests that failed |
| --- | --- |
| M8 on the return: `with_minimum(parse_egress_minimum(config)?)` became a block that still parses (so a bad vocabulary still refuses) but returns the unbound authority | under `cargo test -p brokkr-runtime --lib`, 759 passed and 3 failed: the new test (row `Some("local")`, left "compiled", right the refusal above), `model_policy_tests::the_operator_rules_the_minimum_into_the_bundle` and `model_policy_tests::an_agent_chains_route_resolves_through_the_adapter_that_maps_it` |

The mutation was restored from a copy of the file taken before it, and
the runtime library then passed 762 of 762. With the resolver's matrix
(M1–M5), which binds what `carried` does with the minimum it holds, the
chain from the bundle's `egress_minimum` to MB4's cause is now bound end to
end in committed tests. When U9b lifts the fence, the asked rows of the
fenced-compile test observe the same field directly.

Gates on the return's tree: formatting is clean and workspace clippy with
`-D warnings` finished without a diagnostic. The runtime library passed
762 tests and all 26 runtime integration binaries passed, `witness_digests`
and `budgets` among them without a bless. `bundles/self` compiled to
`11c7d0e7…`, unchanged. OpenSpec strict validation passed 20 of 20,
`typos --hidden` found nothing and `git diff --check` was clean.
`quality/file-lines.txt` was re-measured: `capabilities/binding.rs` 317 to
323 and `agent_tests/gate_tests.rs` 444 to 476; `bundle.rs` stayed at
7808. The `files` and `clones` ratchets held. Exact coverage and remote
CI on Linux and macOS are pending.

## U4a: seat-record validation in a consumed child (tasks 13.1–13.2)

Run `0065-slice-two-unit-u4a-see-the--9d9abe21` built U4a on branch
`s2/U4a` from main at `c118b1ce`. It touched the row's two production
files and no others.

### What changed

`seat_record.rs` keeps the version table and its dispatch: the embedded
schemas, the contract paths, the three engine lines, `SeatRecordVersion`
with `of_engine`, `of_manifest`, `contract` and `source`, `semver_triple`,
`SeatRecordError` with its unchanged text, and the five validator cells.
A new private `compiled` names each version's cell, so a version's bytes,
contract and cache are one row of that table and U4b adds v6 there alone.
The new private child `seat_record/validation.rs` holds what judges a
record: `compile`, `validator`, `SUBSETS`, `validate_seat_record`,
`record_of` and `validate_events`, moved without a changed line of logic.
The parent re-exports them at their old paths (`pub use` for
`validate_seat_record`, `pub(crate) use` for the other two), so `lib.rs`'s
append fence, its export sweep and `import.rs`'s offline verify call
exactly what they called before, and the `api` ratchet holds. The parent's
unit tests moved verbatim to `seat_record/tests.rs`, only dedented and
given the two imports the parent no longer carries. The parent shrank from
880 to 199 lines; the child is 89 and the moved tests 618.

### Tests

The thirteen moved tests pass unedited: embedded-byte parity for v1–v5,
every historical version's fields and refusals, the engine-line matrix
including malformed and missing engines, the token-subset rule and event
selection. The store suite's append, fence-reads-the-manifest and
export/offline-verify refusal tests pass unedited too.

D9's native legacy boundary matrix is new, in
`tests/capability_launch/legacy_journal.rs`, a child module of
capability_launch. Its two-line registration was paid for by removing two
`#[cfg(unix)]` attributes on helpers that already only run on unix hosts
(decision 0063), so capability_launch stays at its 11,883-line baseline.
`every_site_shape_journals_its_legacy_native_rows_through_export_and_verify`
(`legacy_journal.rs:460`) compiles one bundle on fixture adapters and
drives it through a real `Engine` and `Store`. Its driver is the test
binary re-entered as `legacy_driver_child`, the pattern
`two_engines_one_journal.rs` established, writing what the shipped Claude
lowering writes for a native `WebSearch` call and a local `Read`. The
sites are an agent-backed single seat, an inline seat, an agent whose
primary's driver is never installed (so decision 0016's fallback serves
the second model), a panel and a sequence each with one inline and one
agent site, two inline seats whose first attempt fails after it accepted,
and the protected review gate. One `assert_eq!` compares every journaled
checkpoint, keyed by seat and member, with the expected rows written out:
27 rows under 11 seat-and-member keys, with the engine's `boundary`, stamps and member
tags and its own `panel-member-finished` and `sequence-step-finished` rows.
The persistent root is rejoined with the engine's offer
(`launch: resumed`, root `resumed-root-0`). The other root is not offered,
so it is replaced cold (`replaced-root-1`). Both stamps are checked as 64
lowercase hex before they are named. No checkpoint or result in the run
carries `capability`, `dialect`, `call_id`, `call_state`,
`response_sha256` or a private `observation`. `export_ndjson` returns one
line per event, and `verify_export` folds it to `Completed` at the last
seq. Then a direct append of a partial group, a whole group and a private
observation, each beside a legacy `WebSearch` row, is refused with exactly
`SeatRecordError { seq: head + 1, path: "/", contract: v5 }`, and the head
does not move. The test passed three runs in a row.

### Removal mutations

Each mutation compiled, was run, and was undone with the Edit tool before
the next.

| Mutation | Tests that failed |
| --- | --- |
| M1: `validate_seat_record`'s schema check became `false && …` | the matrix (`legacy_journal.rs:440`: the partial-group append was accepted at seq 77) and 14 store tests, among them all three fence and export/verify tests and eleven moved version tests |
| M2: `record_of` returned `None` for a checkpoint | the matrix (`legacy_journal.rs:440`, the same acceptance) and 8 store tests, among them the three fence and export/verify tests |
| M3: `validate_events` swept `&events[..0]` | `export_and_offline_verify_refuse_a_nonconforming_seat_record` (export returned the planted prose row) and five moved event-sweep tests |
| M4: `of_engine`'s v5 arm removed | the matrix: the run parked at its first stamped row, "seat record at journal seq 5 violates contracts/seat-record.v4.schema.json at /" |
| M5: `compiled` handed v5 the v4 cell | `the_root_the_stamps_and_the_new_refusals_belong_to_v5_alone` and `the_zero_ten_line_reads_v5_and_the_nine_line_still_reads_v4` (`tests.rs:551`) |

After the restore, the store package passed (80 library tests and every
integration binary) and the matrix passed again.

### 13.1 and 13.2

The extraction is consumed by the existing fence, export and verify
callers, dispatch and refusal text did not move, and the parent shrank.
D9's legacy matrix passes at this merge, and the historical-version and
exact-refusal tests stay green. M1–M5 each failed an intended assertion
and were restored. That is the evidence both tasks are ticked on.

### Gates on this visit's tree

Formatting is clean and workspace clippy with `-D warnings` finished
without a diagnostic. The runtime library passed 763 tests and every
runtime integration binary passed: capability_launch 70 of 70 (the matrix
and its driver among them), `frozen_contracts`, and `witness_digests` and
`budgets` without a bless, so no witness pin moved. The CLI package
printed 46 passing result lines and no failure, and core, protocol, view,
store and bridge passed. `bundles/self` compiled to `11c7d0e7…`,
unchanged. OpenSpec strict validation passed 20 of 20, `typos --hidden`
found nothing and `git diff --check` was clean. The `files`, `clones` and
`api` ratchets held, and `baselines` against `c118b1ce` reports nothing
raised. `quality/file-lines.txt` was re-measured: `seat_record.rs` 880 to
199, plus `seat_record/validation.rs` (89), `seat_record/tests.rs` (618)
and `capability_launch/legacy_journal.rs` (496). No `too-many-lines.txt`
anchor moved, because every listed capability_launch function sits below
the two edits, which cancel. No suppression was added.
`scripts/measure-budgets.sh` rewrote only main's existing drift (the
measurement dates, and prompt budgets 13 bytes lower from earlier merges).
U4a moves no prompt, dependency or transcript, so those files were left
as they were. Exact coverage and remote CI on Linux and macOS are pending.
