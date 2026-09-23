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
