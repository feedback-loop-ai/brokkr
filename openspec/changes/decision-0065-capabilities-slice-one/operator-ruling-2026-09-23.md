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
