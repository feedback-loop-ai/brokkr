# Operator ruling, 2026-09-23: refuse, never reconcile

Three councils have held slice one. Each repair closed the holes its council
named, and the next council found their neighbours: a crafted parenthesis on
which Brokkr's list parser and Claude's own list splitter disagree; an authored
`--tools` list that widens the restriction the engine composed; a separator the
serializer trusts; Codex's managed OFF argv passed through unparsed; a charter
or policy outside the recipe pinned instead of refused. The third council's
positions are recorded verbatim in `.forge/tasks/council-positions-80bfd784.md`
(run `build-decision-0065-slice-one-th-80bfd784`, reviewed
`3b31c5de..b6dbb057`); its adversarial position was blocked by the provider's
classifier and no chief ruled.

The cause is the approach, not the individual repairs. Brokkr tried to
understand and merge what a recipe wrote into a harness's own flags, and every
merge rule is one more place where two parsers can disagree. The operator
ruled on 2026-09-23 to stop reconciling and to refuse instead. All four parts
were accepted as proposed.

## Ruling 1: a recipe cannot author a capability-bearing flag for a known harness

For the harnesses Brokkr drives (claude, codex and dsh, and LaneTally, which
shares the Claude composition path), an authored driver command or seat
setting that carries a capability-bearing option is refused at compilation.
Nothing is merged. The capability-bearing options are:

- tool lists, allow or deny (`--tools`, `--allowedTools`, `--disallowedTools`
  and their aliases and `=` spellings);
- MCP configuration (`--mcp-config`, `-c mcp_servers.*` in every spelling,
  attached forms such as `-cVALUE` included);
- plugin directories;
- permission modes;
- web and search options.

Tools come from one place: typed agent and seat data, plus the realm's grant,
composed by the engine alone. This follows from the realm-only ruling of
decision 0065. Shipped recipes that write these options inline, the preflight
reviewer among them, move to typed tool declarations in the same slice. The
refusal carries a bounded reason that names the option and never echoes its
value.

## Ruling 2: the launch proves itself

The adapter's declared ON and OFF argv for every capability is parsed when the
adapter data is loaded; a declaration that does not parse refuses the load.
The engine's final composed command is parsed back before launch, and the
capability state it actually expresses is compared with the state the plan
recorded. A mismatch refuses the launch. This is decision 0066's principle
("a denial is something the launch proves") made total: it covers the final
command, not an intermediate composer.

## Ruling 3: canonical containment, as originally ruled

A charter, policy or other consumed input whose filesystem-resolved path
leaves the recipe's tree is refused. It is not pinned and admitted. The
repaired artifacts that permitted outward links when pinned are a
specification defect against the operator's rule (third council, R3 and C7),
and they revert to refusal.

## Ruling 4: the doctor asks the composer about the whole plan

The doctor stops assessing each capability alone. It submits the complete
plan to the same composer the launch uses, and it reports what that composer
admits or refuses. It never promises a combination the composer would refuse.

## What this supersedes

- Every repair that merged, folded or reconciled authored capability options
  with engine-composed ones: the list folding in `native_controls.rs`, and
  the admission scanning of authored lists, which ruling 1 replaces with
  refusal.
- The "pinned outward link" permission in the specification, design and
  evidence (ruling 3).
- Any evidence or task that claims completion on the reconciling approach.
  Those tasks reopen and are reconciled against these rulings.

Decision 0065's standing rulings are unchanged: off by default with no
grandfathering, realm-only grants, and tools as an abstraction with dialects
as the concrete.

## Addendum, 2026-09-23: precedence in `dsh_launch_with` (rebuild unit 1b)

Rebuild unit 1b's triage (run `build-decision-0065-slice-one-re-bc7556ea`)
found that no clause settles the order of the two guards the replay kept in
`dsh_launch_with`. The operator ruled the current order:

1. **The authority refusal wins.** Composition runs first. A plan carrying a
   native control, or a site with no engine-computed authority, is refused
   there, before the boundary check reads any argv. Under ruling 1, a
   capability-bearing flag the recipe wrote is refused as authored input,
   whatever else is wrong with the argv.
2. **The boundary check inspects the composed argv,** the command that will
   actually launch (ruling 2). For DSH, which folds nothing in, this is
   byte-for-byte the argv as handed over today. If the path ever composes
   controls, the check covers them too.

Unit 1b records this as one requirement with a scenario in the native-control
delta, and binds it with a test that tells the two orders apart: a native
control plus a boundary-faulted argv such as `--model --effort`, asserting the
authority refusal's exact reason. It changes no production code unless the
test shows the code differs from this ruling.

## Addendum, 2026-09-24: rebuild unit 4's file inventory

Rebuild unit 4's implement seat (run `triage-directive-operator-ruling-dbc7463e`)
found two council-accepted texts naming different production files for the
unit. The unit's own text under "Rebuild units" names `engine.rs` and
`bundle.rs`, under a stop-and-split rule. Design D5.7, added by unit 3 and
accepted by its clean council, names `agents.rs` as the third file, for
Candidate storage and the `resolve_report` projection. D5.7 also forbids
recovering the lowering from `Candidate::parts` or bytes, so the unit cannot
be built without it.

The operator ruled **(a): D5.7's inventory governs unit 4.** Its production
files are `engine.rs`, `bundle.rs` and `agents.rs`, which is within the
preamble's three-file limit, and its test files include `engine/tests.rs`,
`engine/agent_tests.rs` and `engine/resume_tests.rs` for the Candidate
constructor migration. The edits already on the branch (07d88b44 and the
review return e361a36e) are admitted under this ruling. No separate unit is
split out.

## Addendum, 2026-09-24: the permission template at inline sites

Rebuild unit 5b lowered typed tools at inline Claude and LaneTally sites and
emitted no permission mode, because the design did not settle how the
adapter's `acceptEdits` permission template reaches an inline command once
ruling 1 removes the authored `--permission-mode`. Unit 6 cannot migrate
`fast`, `node` or `preflight` without it.

The operator ruled **(a): the engine emits the adapter's declared permission
template at inline sites** as its own engine-owned segment, the same way it
already emits it when composing an agent-backed seat:

1. The template comes from the adapter's declaration (for Claude,
   `--permission-mode acceptEdits`), never from the recipe.
2. It is appended in its own engine-owned origin, not the authored one, and
   the expected state records it.
3. The launch parses it back with the rest of the final command (ruling 2).
   An authored permission mode remains refused (ruling 1; unit 5b-fix).
4. It applies only where the seat's typed declaration lowers at an inline
   site. Every other inline shape keeps its existing refusal.

## Addendum, 2026-09-25: a standing admission for forced test lines

Rebuild units 5c-fix-b, 5c-fix2 and 6 each stopped, correctly under the
scope rule of the implementer charter, to ask for one to four lines in test
files outside their named inventory. The operator ruled a **standing
admission** for units 7 through 27: a few test-file lines outside a unit's
named files are admitted without stopping when the compiler forces them (for
example, a new struct field every literal must name) or when a test fixture
needs them because it swaps the shipped driver for a fixture driver and the
unit's change reaches it. Each such line adds no assertion, removes no
assertion and changes no tested behaviour, and each is recorded in
`evidence.md` and in the unit's `tasks.md` note. Anything else outside a
unit's files, including any production file, any new assertion or any changed
behaviour, still stops as `oversized`.

## Addendum, 2026-09-25: inline Codex sandbox classes are narrowed

OPERATOR RULING, 2026-09-25 ("narrow"), on the inline Codex sandbox classes: rebuild unit 7 found that standby's implement seat (danger-full-access), standby's review seat (workspace-write) and review-first's review seat (workspace-write) cannot move to typed tools.sandbox, because design D5.3 refuses inline sandboxes and admits danger-full-access nowhere and workspace-write at no gate. The operator ruled to NARROW the seats, not widen D5.3: (1) review seats are gates and change no files, so they run `read-only` and deliver their result through decision 0046's last-message door (the harness writes the seat's final message to the result path); (2) implementer seats run `workspace-write`; (3) `danger-full-access` is admitted nowhere, including wager-harness; (4) new unit 5d lowers a typed sandbox at inline Codex sites as an engine-owned segment, admitting only `workspace-write` at work sites and `read-only` at gates, recorded in the expected state and parsed back at launch; every other inline sandbox shape keeps its refusal.

## Addendum, 2026-09-25: the standing admission extends to unit 5d

OPERATOR RULING, 2026-09-25 ("extend the standing admission to 5d"): the standing admission for forced test lines now also covers the inserted units 5d, 5d-fix and 5d-fix-b, on the same terms: no assertion added or removed, no tested behaviour changed, and each line recorded.

## Addendum, 2026-09-25: the nonempty restriction positive is deferred

OPERATOR RULING, 2026-09-25 ("defer"), on unit 9's return: rebuild unit 9 (commit 8338881a, evidence.md "Unit 9") found no supported, bounded provider transport for a nonempty restriction. Every shipped dialect admits only the empty restriction, every shipped adapter declares restrictions unsupported or unmeasured, and Claude WebFetch `domain:` rules are approval pre-grants that preapproved domains and other settings scopes widen. The operator chose option (a), DEFER: (1) the nonempty-restriction positive leaves slice one and moves to the slice that measures a provider restriction transport; (2) the deferred obligations are NCC "H3 a held restriction survives final composition", the second M3 ("restriction removal fails at final launch"), the H1-H3 matrix's restriction rows, 9.1, 11.2's restriction half, 21.2 and the restriction portions of 21.3 and 23.1; (3) slice one's restriction behaviour is exactly CQ1's reachable outcomes on shipped data: refuse a requires, drop a wants with OFF, and leave an unused grant pinned but inactive, and every dialect keeps its empty-only restriction schema; (4) nothing in slice one may record, compile or claim an enforced nonempty restriction.

## Addendum, 2026-09-25: rebuild unit 11's test inventory

OPERATOR RULING, 2026-09-25 (option a), on unit 11's oversized return: unit 11's test inventory is widened, for this unit only, to add `crates/brokkr-runtime/src/bundle/agent_tests.rs`, `crates/brokkr-runtime/tests/capability_launch.rs` and `crates/brokkr-cli/src/doctor/capability_tests.rs`, for ASSERTION UPDATES ONLY; no production file is added. In each file, a planted malformed declaration either moves to the exact load refusal it now meets, or is re-planted as well-formed so that the compile-time refusal it exists to bind is still reached, preferring the re-plant wherever the test's purpose is the later refusal. The nonempty-restriction positive becomes the exact deferral refusal, and its original positive is recorded under "Deferred to the restriction-transport slice". Every changed assertion is recorded in `evidence.md` with its reason and bound by a baseline red and a mutation plus restored pass. The `doctor.rs:960-966` follow-up belongs to unit 22.

## Addendum, 2026-09-26: a standing admission for fixture migration

OPERATOR RULING, 2026-09-26 (standing fixture migration, units 12-27): a test fixture that authors, inline, an option the operator's refusal ruling now refuses (for example --allowedTools, --sandbox, --permission-mode, -c capability settings) MAY be migrated to its typed declaration (tools.allow, tools.deny, tools.sandbox under the narrow ruling), or re-planted so the test still proves what it exists to prove, in any test file, without stopping. Record each migration in evidence.md and in the unit's tasks.md note, with its reason. Bind every changed assertion with a baseline red and a mutation plus restored pass. This admits no production file and no new behaviour; anything else outside the unit's files still stops as oversized.

## Addendum, 2026-09-26: the panel-member rejoin positive is deferred

OPERATOR RULING, 2026-09-26 (F2 option a): the re-planted panel-member rows, which bind the fail-closed sandbox-unavailable retry, are accepted as slice one's panel-member proof. The no-hands panel member rejoin POSITIVE is deferred to a later slice, alongside typed sandbox lowering at members (decision 0072's follow-up). STEP 0 (docs, commit alone): append this ruling verbatim as an addendum "2026-09-26: the panel-member rejoin positive is deferred" to operator-ruling-2026-09-23.md, and move that obligation in tasks.md and the owning delta under "Deferred" with a pointer, never ticked. Then verify e416d64b's claims and record F2's closure by deferral in evidence.md. No production change is expected; if review finds anything else, fix it within this unit's files.

## Addendum, 2026-09-27: rebuild unit 12-fix-e's one expectation

OPERATOR RULING, 2026-09-27: admitted for this unit, as an assertion update only, the one expectation at crates/brokkr-protocol/src/adapters/tests.rs:15027 (an_authored_plugin_or_later_list_value_is_refused_at_the_final_command). It asserted a 520-scalar refusal, which breaches D6's 512-scalar bound. It becomes the line's first 511 scalars and "…", exactly as the proposed patch writes it. Apply full.patch and the proposed patch, re-verify the record in evidence.md, take the mutation proofs, and record the admitted change with its baseline red. Anything else outside the unit's files still stops.

## Addendum, 2026-09-27: rebuild unit 13-fix's two expectations

OPERATOR RULING, 2026-09-27: admitted for this unit, as assertion updates only, the two expectations in crates/brokkr-runtime/tests/capability_launch.rs at :6576 (a_narrowed_grant_admits_its_subset_and_a_template_limit_is_never_widened) and :6988 (every_position_reproduction_refuses_by_provenance_and_typed_origins_launch). Each `""` becomes `"Bash"`, because the typed local permission Bash(ls:*) under the template's --tools Read,Bash limit is usable (F6), and the first comment is corrected, exactly as the proposed patch writes it. Apply both patches, re-verify the record, take the mutation proofs, and record the admitted changes with their baseline reds. Anything else outside the unit's files still stops.

## Addendum, 2026-09-27: rebuild unit 13-fix-c's runtime fixtures

OPERATOR RULING, 2026-09-27: admitted for this unit, in crates/brokkr-runtime/src/bundle/agent_tests.rs: (1) re-plant the fixture constant CODEX_WORKSPACE (:967) as the shipped eight-token adapters/codex.json fragment; (2) add --strict-mcp-config to the Claude hands fixture at :4063; (3) the assertion that moves with that re-plant, at :4497-4512: argv.len() == 16, and the literal gains the two -c pairs. These make the fixtures match the shipped adapters under R1's complete required-hands semantics. Apply full.patch and the proposed patch, re-verify the record, take the mutation proofs, and record the admitted changes with their baseline reds. Anything else outside the unit's files still stops.

## Addendum, 2026-09-27: hands make the allow list dormant, and inline hands are served

OPERATOR RULINGS, 2026-09-27, on unit 14b's two mismatches (evidence.md, "Unit 14b — stopped"):
(A) An agent with hands and no allow list is served. Under decision 0043 ruling 2, a site with hands does not consult its tool allow list, so `local: {allow: unspecified, application: dormant}` beside `hands: required` is CONSISTENT. check_final's consistency table admits unspecified with dormant exactly when hands are required, and nowhere else.
(B) An inline site with hands is served like an agent. The inline composition carries the adapter's DECLARED hands.workspace fragment (and its typed HandsSpec), as 14a1 made the agent composition do, so the inline dialect's hands is not empty and R1's transport check passes on the engine's own fragment.
Rebuild unit 14a3 builds both: (A) in `crates/brokkr-protocol/src/native_controls.rs`, (B) in `crates/brokkr-runtime/src/bundle.rs` (`SiteFacts::inline_serving`). No `adapters.rs` change. Unit 14b then re-runs from its saved patch.

## Addendum, 2026-09-27: an unselected entry neither grants nor denies, and harness hands are the harness fragment

OPERATOR RULINGS, 2026-09-27 (on 14b's third visit, run 0065-rebuild-unit-14-see-the-uni-d32a2322):
(1) An UNSELECTED inventory entry neither grants nor denies a capability the seat holds through its selected entry. This mirrors 12-fix's ruling that authority follows the selected holding only. check_final's denial pass counts denials only from the selected entries' semantics. An unselected entry's OFF for a held capability is not a denial (capability_launch.rs:6457).
(2) Under the `harness` boundary there is no Brokkr box. "The engine's workspace hands" are the adapter's own hands.harness.work or hands.harness.gate fragment (decision 0046 ruling 4). The composition seals THAT fragment, not hands.workspace, when the boundary is harness. check_final's R1 admits exactly that fragment under harness, and nowhere else (capability_launch.rs:6751).

## Addendum, 2026-09-29: runs without recorded charter bindings; unit 19 admission

OPERATOR RULING 2026-09-29 (unit 19).
1. NO GRANDFATHERING FOR UNRECORDED RUNS. A run whose run/started records no charter bindings (every run started before this change) is refused at pinned resume with the exact cause `unrecorded`. There is no intact-only or digest-only fallback. Such runs are concluded and re-fired.
2. ADMISSION FOR TWO TEST FILES, for unit 19 only: (a) crates/brokkr-cli/tests/witness_journal.rs may add `charters` to the pinned run/started key list; (b) crates/brokkr-cli/src/tests.rs may change the resume of the frozen fixture fixtures/journals/tui-graph-the-selection-box-gets-80f98deb.ndjson to assert the exact `unrecorded` refusal, and may re-prove the TUI graph resume on a fresh journal the test builds with recorded bindings. The frozen fixture file itself does not move. Each changed assertion is bound by a baseline red, a compiling mutation and a restored pass.

## Addendum, 2026-09-29: a typed restriction on an unmeasured plan refuses at compile (R5)

A typed tools restriction (tools.allow / tools.deny) on a harness whose native controls are unmeasured (LaneTally today) REFUSES AT COMPILE with its exact unmeasured cause, per D5.3. It is never lowered onto an unmeasured plan and never left for the launch check to refuse: compile and launch agree. When that harness's controls are measured, a later unit may admit it.

## Addendum, 2026-09-29: the R1 pinned-resume proof is unit 20-fix-b

Unit 20-fix-b is commissioned with ONE test file, `crates/brokkr-cli/tests/capability_verbs.rs`, and NO production file. It proves that a pinned `brokkr resume` refuses the typed LaneTally allow, inline and agent-backed, with the exact rendered cause (the line brokkr-cli's `unreproducible` renders). Cold compilation does not stand in for resume coverage.

## Addendum, 2026-09-29: unit 21's residual is split as 21-fix-a and 21-fix-b

OPERATOR RULING 2026-09-29 (unit 21).
1. 21-FIX-A (production) is commissioned with `crates/brokkr-protocol/src/native_controls.rs` (R1) and `crates/brokkr-runtime/src/capabilities.rs` (R3), plus their owning tests.
2. R1: the final launch check judges delivery on the state parsed from the FINAL command it was handed (read_state of the parsed final argv), never on a recomposition and never by comparing against the same builder that produced the command. A restriction removed inside the serving builder must be refused by the final check.
3. R3 WORDING: a capability the realm grants but no site of the seat requests is reported, in prompt and manifest, with the exact reason "granted, but this seat does not request it". A capability the realm does not grant keeps "the realm does not grant it to this seat". The two causes are distinct values, never one generic text.
4. 21-FIX-B (tests only) follows 21-fix-a: R2's restriction rows and M1/M2/M4 re-run as final-check refusals, in unit 21's suites.

## Addendum, 2026-09-29: unit 22 admits init_doctor.rs for its assertion updates

Unit 22 may change crates/brokkr-cli/tests/init_doctor.rs for ASSERTION UPDATES ONLY: the wording in scaffolded_claude_denials (about :444-446) that the whole-plan readout replaces, and the one expected plan line after about :482 that the readout adds. No other line of that file moves; no test is added or removed there.

## Addendum, 2026-09-29: main merged into the slice, and the rulings the merge needed

The operator ruled that main comes into `slice-0065-capabilities` after unit 22 and before the audit units 23–27, by a MERGE, not a rebase: one resolution of the conflicts (about 66 hunks in 21 files), and every unit commit that `tasks.md`, `evidence.md` and the journal cite stays reachable. Pull request #319 still squash-merges to one commit on main. Where main's code-health work and this slice met, the operator ruled:

1. **No `unsafe` in brokkr-runtime (decision 0071 ruling 1 holds).** The owner-rooted lookups of the bound reads (units 16–19: the directory listing, `openat` with no-follow, the link text) use rustix 1.1.4's safe calls, a direct edge to the version brokkr-protocol and brokkr-cli already take; `forbid(unsafe_code)` stays. The same removals and refusals must hold; the audit units prove them on the merged head.
2. **Main's ratchets: baseline now, split later.** Every 0065 item over main's ceilings (functions over 100 lines, nesting, struct bools, file lines, suppression counts) carries a scoped `#[expect(.., reason)]` naming this ruling, and the ledgers under `quality/` are re-measured. Splitting them is follow-up work after #319. The pull request carries the `Ruling:` line `quality/ratchet.sh baselines` reads.
3. **Missing or broken adapter data: 0065's refusal wins over decision 0069's optional read.** Adapter data that cannot be loaded, whether the adapters directory is missing or the provider's adapter file is absent from a present directory, cannot prove a known native power off, so the seat refuses with 0065's capability wording (off by default, no grandfathering). A present-but-broken adapter root that gets past the capability check still refuses with the loader's own words.
4. **The prompt-byte budget measures what a seat is told (decision 0071 ruling 5).** The capabilities statement is written by `SiteMarks::capabilities`, called from `SiteMarks::site`, so #342's budget renders it; the budget hands the adapter the verified charter text as dispatch does. Every prompt budget rises by the capabilities paragraph (median 367 bytes over 101 sites).
5. **Unit 22 admits `crates/brokkr-cli/tests/init_doctor.rs`** for its two assertion updates only (recorded in its own addendum of the same date).

Seams recorded, not reopened: #372's unreadable-charter start refusal and unit 18's dispatch-door charter refusal meet at the dispatch door, which refuses first with decision 0066 ruling 5's words (still a failure to start); `brokkr init`'s Codex and dsh scaffolds (#379) carry the shipped adapters' `native_capabilities` declarations, so a scaffold compiles under 0065; main's retirement of the `python3` and `pytest` grants (#355) removes them from every typed allow list the slice migrated.

## Addendum, 2026-09-30: unit 22's residual is finishing unit 22-fix-b

OPERATOR RULING 2026-09-30 (unit 22).
22-fix's commits are merged; its residual is not accepted. 22-fix-b fixes SC22-1, SC22-2 and A22-1. For SC22-2 it may export, from ONE brokkr-protocol file (adapters.rs or native_controls.rs), the existing final validation the launch already runs for a DSH or LaneTally command, so doctor calls the same judgment; the export changes no launch behaviour.

## Addendum, 2026-09-30: unit 26c removes the unreachable arms and refuses the validator's name

OPERATOR RULING 2026-09-30 (unit 26c).
1. The exact gate admits no exclusion, so each arm 26b proved unreachable is REMOVED by restructuring the code so the impossible state cannot be represented (a narrower type, a function that returns only what its callers can receive, a match that no longer has the arm) — never by `unreachable!()`, `expect`, `#[coverage(off)]` or any exclusion, which leave an uncovered region or evade the gate. Behaviour for every reachable input is unchanged.
2. The dialect-verify defect is fixed: a recipe site (a wrapped verify panel member or any other site) whose address equals the synthetic dialect validator's refuses at compile with an exact reason, and the validator's facts are never lost. Regression: baseline red (the scratch compile 26b recorded in .forge/u26b/alias-probe.log), compiling mutation caught, restored pass.
3. ADMISSION for unit 26c only: production crates/brokkr-protocol/src/adapters.rs, crates/brokkr-protocol/src/native_controls.rs, crates/brokkr-runtime/src/bundle.rs, crates/brokkr-runtime/src/capabilities.rs and crates/brokkr-runtime/src/engine.rs, for exactly (1) and (2); their owning test suites.

## Addendum, 2026-09-30: unit 26c's two narrowings are accepted as proven

Unit 26c (run 0065-rebuild-unit-26c-see-the-un-b18b678d; 08345c77, 62424f3d, 6287d527) removed every arm 26b proved unreachable and refused the injected validator's name at compile; its seat diagnostic measured lines, branches and functions equal. Its review asked that two invariants be held by type rather than by tested proof: `refuse_permission_pins` taking a resolved `Candidate` and the adapters (typing it needs `crates/brokkr-runtime/src/agents.rs`), and `ServingShape::of` repeating the harness vocabulary `native_controls/grammar.rs` holds (typing it needs that file). The operator accepts both as 26c left them, held by their tests and recorded mutations: task 26c.1 closes on that record. Typing them is follow-up work after #319, beside the splitting the merge's ratchet ruling deferred.

## Addendum, 2026-09-30: unit 27b resolves each compile root once, and the socket refusal is judged per host

OPERATOR RULING 2026-09-30 (unit 27b).
1. Every root a compile binds (the bundle directory, the agents/library roots, the operator and capability definition roots, each composed layer) is resolved ONCE at compile entry to its canonical path, and every handle, binding and owner identity is taken from that canonical path. A path through a symlinked ancestor compiles; a later replacement of the bound directory, or of any ancestor of the canonical path, still refuses exactly as units 16–19 prove. No second resolver, and no following of links after entry.
2. The socket refusal's expected text is each supported host's own error kind (Linux and macOS, decision 0063), derived from the platform rather than a hard-coded Linux word; the refusal itself does not change.
3. SQ3/SQ4 (unit 27, 7f4b9f54): the dialect step in crates/brokkr-runtime/src/engine/sequence.rs composes at its own label (:395) and runs mark_capabilities (:405); removing either survived every suite. Bind each with a test that fails when it is removed (the generated validator's driver input carries its own site's composition and explicit null capabilities/controls, "the generated validator holds nothing, and says so"), or, if a removal is provably equivalent, record the proof in evidence.md and report it for the operator's ruling rather than leaving a silent survivor. Tests only.
4. CI's spell checker (typos, non-Rust lints) flagged possessives written into test names and one fixture; rename them, updating every reference in tasks.md/evidence.md: doctor/capability_tests.rs `dsh_and_lanetally_plans_are_judged_by_their_launchs_own_final_validation` (-> `launches_own`), adapters/tests.rs `the_dsh_reading_doctor_calls_is_the_launchs_own_judgment` and `the_lanetally_reading_doctor_calls_is_the_launchs_own_judgment` (-> `launch_own`), native_controls/tests.rs `a_selection_mapping_is_read_against_the_harnesss_own_lists` (-> `harness_own`) and `an_unselected_entrys_off_for_a_held_capability_is_no_denial` (-> `entry_off`), and doctor/capability_tests.rs:1704's office string ending `-in-ful` (keep it exactly 64 bytes, e.g. `-in-all`). Never list a genuine typo in _typos.toml.
5. ADMISSION for unit 27b only: production crates/brokkr-runtime/src/bundle.rs and, only if the entry point lives there, crates/brokkr-runtime/src/launch.rs; tests in the owning suites.
