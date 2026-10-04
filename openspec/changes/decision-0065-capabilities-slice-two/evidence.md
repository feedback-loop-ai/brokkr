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
