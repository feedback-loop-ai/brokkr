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
