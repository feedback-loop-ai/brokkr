## ADDED Requirements

### Requirement: A run enters at a gate of its table

`brokkr run` SHALL accept `--enter <phase>`, and a run started with it SHALL enter the named phase first, instead of the table's `initial`. The bundle it runs under SHALL be the one the same recipe compiles to without an entry: the table, its `initial`, the manifest, its digest and the witnesses do not move, because an entry is a fact of the run, not of the recipe. A recipe whose own `initial` is a gate SHALL start as it always did when no entry is named. A Looper-bound run (`--dispatch`) SHALL refuse an entry by name, as it refuses a named map, until the v2 manifest lineage carries one.

#### Scenario: A finished branch is verified, judged and shipped without a work seat

- **WHEN** a `fast`-shaped recipe is run with `--enter verify` over a repository whose branch already holds the implementation, and its verify, review and ship seats report `pass`, `clean`, `ready` and `shipped`
- **THEN** the run completes by `SHIP-COMPLETE`; its journal enters `verify` first and never enters `intake`, `design` or `implement`; no effect is requested of a work seat; and the run manifest equals the manifest a run of the same recipe without an entry pins

#### Scenario: A landing starts where it always did

- **WHEN** `brokkr run --recipe landing` is started without `--enter`
- **THEN** the run enters `classify` first and its `run/started` carries no `entry`

#### Scenario: A Looper-bound run takes no entry

- **WHEN** `brokkr run` is given both `--dispatch` and `--enter`
- **THEN** the launch is refused naming both flags before anything is compiled, opened or written

### Requirement: Entry is admitted only at a gate from which the protected phase is unavoidable

An entry SHALL be admitted when the compiled table and the entry satisfy all three: the phase is one of the table's phases and is seated; every driver-bearing site of that seat is a gate (its class under decision 0021 ruling 1 is `gate`; a sequence counts by its steps, a dialect step being a gate, and a panel by its members); and the protected phase is the entry itself or is unavoidable from it, by the walk the compiler already makes from `initial`, started at the entry. Any other entry SHALL be refused when the recipe and the entry are compiled, before a run id is minted or an event is written, by a typed error that names the phase, which of the three it failed, and the entries the table admits, derived by the same admission. The engine SHALL hold no roster of which phases may be entered: the three facts are the table's and its seats'.

#### Scenario: A work phase is refused

- **WHEN** a `fast`-shaped recipe is run with `--enter implement`
- **THEN** no run is created, and the refusal names `implement` as a work phase and lists `review` and `verify` as the entries the table admits

#### Scenario: A gate that reaches done without the protected phase is refused

- **WHEN** a `fast`-shaped recipe is run with `--enter ship`
- **THEN** no run is created, and the refusal names `ship` as a gate from which `done` is reached without `review`, and lists `review` and `verify`

#### Scenario: The protected phase itself is an entry

- **WHEN** a `fast`-shaped recipe is run with `--enter review` and review reports `clean`
- **THEN** the run enters `review` first, and ship's drift check reads the head review was entered at

#### Scenario: A phase the table lacks, or a terminal, is refused

- **WHEN** a recipe is run with `--enter intake` and its table has no `intake`, or with `--enter done`
- **THEN** no run is created, and the refusal names the phase as absent from the table, or as a terminal with no seat, and lists the entries the table admits

#### Scenario: Every shipped table's entries are pinned

- **WHEN** every recipe under `recipes/` is compiled and each of its phases is tried as an entry
- **THEN** the admitted set of each is pinned by a test: `verify` and `review` in every `fast`-shaped table, with `triage`, `clarify` and `analyze` added in the `triage`-shaped ones and `classify` in `landing`; and `implement`, `ship`, `intake` and every dialect phase that writes are refused in every table that has them

### Requirement: A phase the entry took as done is returned to

In an entered run, the phases before the entry in the table's order are the work the entry took as done on the branch. A seat of such a phase SHALL receive `returned_from`, the ruling's phase and the result that sent the run there, on its first visit exactly as any seat does on a revisit, so the smith reads the failing verify or the review's findings it is sent to answer. A phase after the entry is reached forward and SHALL receive nothing new on its first visit. No seat input of a run without an entry SHALL move.

#### Scenario: The entered run's first return carries the finding

- **WHEN** a `fast`-shaped recipe is entered at `verify` and verify reports `fail`
- **THEN** the implementer's first visit receives `returned_from` with phase `verify` and the verify seat's result, and its `context.results` holds no result of a phase this run never ran

#### Scenario: A forward phase is not a return

- **WHEN** a `triage`-shaped recipe is entered at `analyze` and analyze finds the artifacts consistent
- **THEN** the implementer's first visit receives no `returned_from`, as it does in a run that began at `triage`
