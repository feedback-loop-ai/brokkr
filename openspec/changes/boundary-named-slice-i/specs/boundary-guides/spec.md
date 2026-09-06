# boundary-guides

## Purpose

The prose follows the code: the guides say where the boundary lives and
what each word means, the platform paragraph stops saying Linux only,
the readouts' unboxed rendering is documented, and the decision carries
its one-line erratum. Reference sections are updated, never removed
(decision 0046 rulings 1 to 5 and consequences; the commission's
erratum).

## ADDED Requirements

### Requirement: The guides document the boundary and never lose a section
The guides SHALL be updated as follows, each section kept and amended,
none removed: `docs/guides/provider-adapters.md`'s Hands section
documents `hands.harness` with its `gate` and `work` members, the
three-shape convention, codex's fragments, and the measured claude
fragments with the claude version and what each denies and allows;
`docs/guides/recipe-authoring.md`'s `hands` row stops saying Linux only
and says the boundary lives in the realm, and its `driver.confine` row
says the field is refused and points at decision 0046 ruling 5;
`docs/guides/quickstart.md`'s platform paragraph says that a realm on
macOS or Windows declares `boundary: harness` today, what that means —
judged under the harness's own sandbox, rendered *unboxed* — and that
`namespace` is the default and needs bubblewrap;
`docs/guides/journal-and-verification.md` gains the unboxed rendering
and what `boundary` on a record and on `effect/started` means;
`docs/guides/read-surfaces.md`'s seats-table example is refreshed from
the renderer's header line, which already carries `model`, and shows the
`boundary` column beside it; `docs/guides/repository-layout.md` names `boundary` beside
`house` and `dialect` in the `realms.json` row and the new contract
files in the `contracts/` row; `docs/guides/driver-authoring.md` and
`ARCHITECTURE.md` stop describing the `docker run` wrapper as a trust
class and point at the boundary; and `contracts/README.md` gains rows
and a paragraph for `realms.v4`, `run-manifest.v9`, `seat-record.v4`
and `effect-boundary.v1` in the style of the rows before them.

#### Scenario: provider-adapters documents hands.harness
- **WHEN** `docs/guides/provider-adapters.md` is read
- **THEN** its Hands section names `hands.harness`, `gate`, `work`, the codex fragments, and the claude version the claude fragments were measured against

#### Scenario: recipe-authoring points the two rows at the realm and at 0046
- **WHEN** the site vocabulary table is read
- **THEN** the `hands` row says the boundary is the realm's and no longer says Linux only, and the `driver.confine` row says the field is refused under decision 0046 ruling 5

#### Scenario: quickstart's platform paragraph
- **WHEN** the quickstart's platform paragraph is read
- **THEN** it says a realm on macOS or Windows declares `boundary: harness` today, that such a run is judged under the harness's own sandbox and rendered *unboxed*, and it no longer says macOS and Windows adopters need a Linux box

#### Scenario: journal-and-verification and read-surfaces show the rendering
- **WHEN** the two guides are read
- **THEN** one explains the unboxed rendering and the record's `boundary`, and the other's seats table carries a `boundary` column beside `model`

#### Scenario: The layout, driver and architecture pages follow
- **WHEN** `repository-layout.md`, `driver-authoring.md` and `ARCHITECTURE.md` are read
- **THEN** the realm row names `boundary`, the contracts row names the four new files, and no page describes `driver.confine` as a working trust class

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
