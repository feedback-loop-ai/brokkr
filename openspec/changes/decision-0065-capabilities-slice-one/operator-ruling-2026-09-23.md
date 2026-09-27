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
