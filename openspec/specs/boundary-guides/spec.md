# boundary-guides Specification

## Purpose
The prose follows the code: the guides say where the boundary lives and
what each word means, the platform paragraph stops saying Linux only,
the readouts' unboxed rendering is documented, and the decision carries
its one-line erratum. Reference sections are updated, never removed
(decision 0046 rulings 1 to 5 and consequences; the commission's
erratum).

## Requirements

### Requirement: The guides document the boundary and never lose a section
The guides SHALL be updated as follows, each section kept and amended,
none removed: `docs/guides/provider-adapters.md`'s Hands section
documents `hands.harness` with its `gate`, `work` and `result` members,
the `{result_path}` token and the two workspace tokens refused there,
the three-shape convention, codex's fragments and its `last-message`
door — the fragment ruling 4's own word and the door the tool's
documented capture, with the measurement of the capture under the
read-only class recorded when made and named as the operator's and
pending until then — and, per claude member, either the measurement,
the claude version and what the mode denies and allows, or that the
member is undeclared pending the operator's measurement, with the
candidates and the recipe; and its doctor section names the
`boundaries` line and what *offered* and *ready* mean on it;
`docs/guides/recipe-authoring.md`'s `hands` row stops saying Linux only
and says the boundary lives in the realm and that under `harness` and
`open` a bind's `mask` is declared and not enforced and that clearing
the environment confines nothing on disk, an unboxed script reaching
any host path the operator's uid may read (design DD10), and its
`driver.confine` row
says the field is refused and points at decision 0046 ruling 5;
`docs/guides/quickstart.md`'s platform paragraph says that a realm on
macOS or Windows declares `boundary: harness` today, what that means —
judged under the harness's own sandbox, rendered *unboxed* — and that
`namespace` is the default and needs bubblewrap, and that `seatbelt`
and `container` are named by decision 0046 and refuse at start until
slices (ii) and (iii) build them, and names the split as it stands
rather than a promise: which shipped bundles run under `harness` today
— the nine whose hands sites are their own `./` exec gates — and which
refuse, by name and why — `bundles/self`, `recipes/panel-review`,
`recipes/triage` and `recipes/night-shift` until the operator's claude
measurement lands, and the last two until a decision admits the dialect
step, the ground the compiler reaches first for them — pointing at the
pin test that is the record (design DD8, DD20);
`docs/guides/journal-and-verification.md` gains the unboxed rendering
and what `boundary` on a record and on `effect/started` means;
`docs/guides/read-surfaces.md`'s seats-table example is refreshed from
the renderer's header line, which already carries `model`, and shows the
`boundary` column beside it, and its verb list gains `brokkr seats`
beside `inspect`, its `--json` named as `inspect`'s own view model;
`docs/guides/quickstart.md`'s `rerun` line says the
rerun compiles in the discovered realm as `run` does; no guide states
that the network was off under `harness` or `open` — the prefix is
described as a narrowing the engine attempts on Linux (design DD15);
`docs/guides/repository-layout.md` names `boundary` beside
`house` and `dialect` in the `realms.json` row and the new contract
files in the `contracts/` row; `docs/guides/driver-authoring.md` and
`ARCHITECTURE.md` stop describing the `docker run` wrapper as a trust
class and point at the boundary, and driver-authoring's opening
paragraph, which says the engine runs the shipped verifier and shipper
through `brokkr hands exec`, says so of `namespace` and adds that under
`harness` and `open` the same `exec` dispatch runs with no verb of
Brokkr's around it, in a fixed environment, the network narrowed on Linux
where `unshare` permits and never stated as off, that the declaring
layer is re-walked at every unboxed spawn and a pinned byte that moved
refuses the gate, the residual being the interval between the re-walk
and the `exec` (design DD9), and that the `hands` subcommand gains no
verb for it; the two blueprint pages that still present the container trust class follow the same
way, every section kept — `docs/extension-model.md`'s seat-field table,
whose `trust` row says the tier "decides what the engine mounts into
the sandbox", says the wall itself is the realm's `boundary` (decision
0046) and that the tier decides what is mounted inside it, and
`docs/target-architecture.md`'s runner table, whose `policy-confined`
row is an OCI container with a pinned digest, points at decision 0046's
`container` boundary — declared by the realm, refused at start until
slice (iii) measures it — and whose `public-evidence-only` row names
the same boundary for its container form, each page's status line
untouched; and `contracts/README.md` gains rows
and a paragraph for `realms.v4`, `run-manifest.v9`, `seat-record.v4`
and `effect-boundary.v1` in the style of the rows before them.

#### Scenario: provider-adapters documents hands.harness
- **WHEN** `docs/guides/provider-adapters.md` is read
- **THEN** its Hands section names `hands.harness`, `gate`, `work`, `result`, `{result_path}`, the codex fragments and door with the door's measurement recorded or named as pending and the operator's, and, per claude member, the claude version it was measured against or that it is undeclared pending the operator's measurement; and its doctor section names the `boundaries` line

#### Scenario: recipe-authoring points the two rows at the realm and at 0046
- **WHEN** the site vocabulary table is read
- **THEN** the `hands` row says the boundary is the realm's, that a mask is not enforced under `harness` and `open`, that clearing the environment confines nothing on disk, and no longer says Linux only, and the `driver.confine` row says the field is refused under decision 0046 ruling 5

#### Scenario: quickstart's platform paragraph
- **WHEN** the quickstart's platform paragraph is read
- **THEN** it says a realm on macOS or Windows declares `boundary: harness` today, that such a run is judged under the harness's own sandbox and rendered *unboxed*, that `seatbelt` lands in slice (ii), which shipped bundles run under `harness` today and which four refuse by name and on which ground, and it no longer says macOS and Windows adopters need a Linux box

#### Scenario: journal-and-verification and read-surfaces show the rendering
- **WHEN** the two guides are read
- **THEN** one explains the unboxed rendering and the record's `boundary`, and the other's seats table carries a `boundary` column beside `model` and its verb list names `brokkr seats`

#### Scenario: The layout, driver and architecture pages follow
- **WHEN** `repository-layout.md`, `driver-authoring.md` and `ARCHITECTURE.md` are read
- **THEN** the realm row names `boundary`, the contracts row names the four new files, no page describes `driver.confine` as a working trust class, and driver-authoring's opening paragraph qualifies `brokkr hands exec` with `namespace`, describes the unboxed dispatch, states that the layer is re-walked at every unboxed spawn and a moved byte refuses the gate, and names no new verb

#### Scenario: The blueprint pages point at the boundary
- **WHEN** `docs/extension-model.md` and `docs/target-architecture.md` are read
- **THEN** the seat-field `trust` row names the realm's `boundary` as the wall and decision 0046, the runner table's `policy-confined` row names the `container` boundary and slice (iii) in place of a working OCI wrapper, no row is removed, and each page keeps every section and its status line

#### Scenario: The contracts README lists the four files
- **WHEN** `contracts/README.md` is read
- **THEN** it carries rows for `realms.v4`, `run-manifest.v9`, `seat-record.v4` and `effect-boundary.v1`, and its extension-schema paragraph names `effect/started.boundary` among the fields `fold` never reads

### Requirement: Decision 0046 carries the erratum
`docs/decisions/0046-the-boundary-is-named.md` SHALL gain a `## Erratum`
heading followed by one line saying that rulings 3 and 6 name
`seat-record.v3` for the boundary field, that v3 already exists (landed
by #202 under decision 0034 rulings 6 and 7, the dialect state), and
that the field therefore lands as `seat-record.v4`, additive on v3, with
nothing else renumbered. The `Status:` line and every other line of the
decision SHALL be untouched, so the decisions index test passes
unchanged.

#### Scenario: The erratum line
- **WHEN** the decision file is read
- **THEN** a `## Erratum` heading precedes exactly one line naming v4, v3, #202 and decision 0034 rulings 6 and 7, and the decisions index test passes without a change to `docs/decisions/README.md`
