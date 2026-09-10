# realm-crossing Specification

## Purpose
A crossing is a contract between realms: a realm publishes a FILE it
owns, and the realm that depends on it pins the bytes. The vocabulary,
the refusals, the loader that resolves and verifies at world load, the
`run-manifest/v10` recording of what a run stood on, and the readouts
that report the crossings — decision 0054, built by Phase 2 slices
(ii)–(vi).

## Requirements

### Requirement: The crossing vocabulary is forge.realms/v5
`contracts/realms.v5.schema.json` SHALL be v4 plus exactly two optional
per-realm properties. `publishes` SHALL be a list of `{name, path}`: a
file the realm owns, named repository-relative on the same terms `house`
and `dialect` are named, its bytes the contract. `consumes` SHALL be a
list of `{name, realm, sha256}`: a crossing named by its publishing
realm and pinned by a lowercase 64-character hex sha256 over the
published file's RAW bytes, never a canonical form (decision 0054
rulings 1 and 2).

#### Scenario: A v5 map declares and pins a crossing
- **WHEN** a v5 map names a realm that publishes `orders.api` at `contracts/orders.v1.schema.json` and another realm that consumes `orders.api` from it with the file's raw-bytes sha256
- **THEN** the map loads and both properties are read as declared

#### Scenario: Absent both properties a v5 map is a v4 map
- **WHEN** a v5 map carries neither `publishes` nor `consumes`
- **THEN** it loads with the same realms, journals, houses, dialects and boundaries a v4 map would resolve

#### Scenario: Every earlier map keeps loading
- **WHEN** a v1, v2, v3 or v4 map is loaded
- **THEN** it loads exactly as it did before this capability existed

### Requirement: A malformed crossing is refused by shape alone
`brokkr-core` SHALL perform no I/O (decision 0003) and SHALL refuse,
from the map's text alone, a map whose crossing vocabulary is malformed:
a `consumes` entry naming a realm this world does not hold; a `consumes`
entry naming a crossing its publisher does not publish; one realm
publishing or consuming the same crossing name twice; a pin that is not
64 lowercase hex characters; either word written under a label older
than v5; a realm consuming its own published crossing; or either word
written as `null`. Every refusal SHALL name what it read (decision 0054
ruling 3).

#### Scenario: A consumer names an unknown realm
- **WHEN** a `consumes` entry names a realm the map does not hold
- **THEN** the map is refused before any file is opened

#### Scenario: A consumer names an unpublished crossing
- **WHEN** a `consumes` entry names a crossing its named publisher does not publish
- **THEN** the map is refused

#### Scenario: A crossing name is used twice
- **WHEN** one realm publishes or consumes the same crossing name twice
- **THEN** the map is refused

#### Scenario: A pin is not a sha256
- **WHEN** a pin is not exactly 64 lowercase hex characters
- **THEN** the map is refused

#### Scenario: The word appears under an older label
- **WHEN** `publishes` or `consumes` appears in a map calling itself v1, v2, v3 or v4
- **THEN** the map is refused naming the version that forbids the word

#### Scenario: A realm consumes its own crossing
- **WHEN** a realm's `consumes` entry names that same realm as publisher
- **THEN** the map is refused, because a crossing is between realms

#### Scenario: A written null is not an absence
- **WHEN** either property is written as `null`
- **THEN** the map is refused rather than read as though the word had never been written

### Requirement: The loader resolves and verifies crossings at world load
`brokkr-runtime` SHALL, when it loads a world, read every published
crossing from the publishing realm's own tree and hold every consumer's
pin to the raw bytes actually found there. A pin that matches SHALL be
accepted. A pin whose publisher's bytes differ SHALL become a refusal
naming the CONSUMING realm, the crossing, the publishing realm, the
pinned digest, the observed digest and the file. A published file that
cannot be read SHALL be charged to the PUBLISHING realm, and its
consumers' pins SHALL be recorded as unchecked — never matching
(decision 0054; decision 0023 ruling 4; decision 0046's Addendum).

#### Scenario: A matching pin is accepted
- **WHEN** the published file's raw bytes hash to the consumer's pin
- **THEN** the world loads and the crossing is resolved

#### Scenario: A moved pin names what moved
- **WHEN** the published file's bytes no longer hash to the consumer's pin
- **THEN** the load refuses with a message naming the consuming realm, the crossing, the publishing realm, the pinned digest, the observed digest and the file

#### Scenario: An unreadable publication charges the publisher
- **WHEN** the publishing realm's file cannot be read
- **THEN** the publisher's report carries the failure, no consumer pin is called matching, and each consumer's pin is recorded as unchecked

### Requirement: A moved crossing refuses the verbs that start or continue a run
`run`, `rerun`, `resume` and `compile` SHALL refuse a world whose
crossing moved before any seat spawns or any prompt exists, reading the
one refusal the loader built, and `resume` SHALL re-read the disk as a
fence over the world it rehydrated from its run manifest (decision
0046's Addendum; decision 0054; commits `b9b2ee6` and `ca0c765`).

#### Scenario: run refuses before a seat spawns
- **WHEN** `run` opens a world whose consumed pin moved
- **THEN** it exits nonzero, starts no run and prints the loader's refusal

#### Scenario: compile refuses before a prompt exists
- **WHEN** `compile` opens a world whose consumed pin moved
- **THEN** it refuses and writes no bundle

#### Scenario: resume fences the disk
- **WHEN** `resume` rehydrates a world from a run manifest and the published bytes have since moved
- **THEN** it refuses with the same wording before continuing the run

### Requirement: A run records the crossings it stood on as run-manifest/v10
`contracts/run-manifest.v10.schema.json` SHALL be v9 plus one optional
`crossings` property: per publishing realm, per crossing name, the
resolved source path and the sha256 the loader observed on disk. The
property SHALL be a SIBLING of the map pin, absent when no realm
publishes or consumes a crossing, and SHALL carry the loader's
observation rather than a second copy of the map's declaration
(decision 0023 ruling 4; decision 0054 ruling 4; commit `9111ee2`).

#### Scenario: A world with a publication records what it stood on
- **WHEN** a run starts in a world that publishes a crossing
- **THEN** `run/started` carries `crossings` keyed by publishing realm and name, and the export answers offline from the journal alone

#### Scenario: A world with no crossing writes the exact prior shape
- **WHEN** a run starts in a world that draws no crossing
- **THEN** its manifest carries no `crossings` key and stores and exports the exact v9 shape

#### Scenario: A crossing moves no bundle digest
- **WHEN** the same bundle compiles in a world with a crossing and in a world without one
- **THEN** the bundle digest is the same, because a crossing is workspace data and not bundle data

### Requirement: brokkr doctor reports every crossing and refuses nothing
`brokkr doctor` SHALL print one line per realm that draws a crossing: a
sound realm's counts of published files and matching pins; the
publisher's own failure when a published file is unreadable; a moved
pin's failure naming the CONSUMING realm and the crossing; and a warning
naming the publishing realm when a consumer's pin could not be checked.
An unchecked pin SHALL NOT be counted among the matching ones, and a
world that draws no crossing SHALL gain no line (decision 0046's
Addendum; decision 0054; commit `ca0c765`).

#### Scenario: A sound crossing is reported
- **WHEN** every published file is readable and every pin matches
- **THEN** doctor prints an ok line naming the realm and the counts

#### Scenario: A moved pin is the consumer's line
- **WHEN** a consumed pin no longer matches
- **THEN** doctor prints a missing line naming the consuming realm and the crossing, with the loader's own words

#### Scenario: An unreadable publication is the publisher's line
- **WHEN** a published file cannot be read
- **THEN** doctor prints the publisher's missing line and a warning naming the publisher for each consumer's unchecked pin, and claims no pin matching

#### Scenario: A world with no crossing gains no line
- **WHEN** a world that draws no crossing is diagnosed
- **THEN** doctor prints nothing about crossings

### Requirement: brokkr realms reads each realm's crossings out
`brokkr realms` SHALL read, under each realm that draws a crossing, the
files it publishes and the crossings it consumes, each consumed entry
naming its publishing realm and whether the pin is `matching`, `moved`
(with the loader's own refusal words) or `unchecked`. It SHALL emit the
same three states as values under `--json`, SHALL render a world with no
crossing exactly as it did before, and SHALL remain a read surface with
no writes (decision 0023 ruling 6; decision 0054).

#### Scenario: The three pin states render distinctly
- **WHEN** a world draws a matching, a moved and an unchecked pin
- **THEN** the text output spells `matching`, `moved` and `unchecked` under the realms that draw them, and `--json` carries the same words beside each consuming realm

#### Scenario: A moved pin is reported, never refused
- **WHEN** `brokkr realms` opens a world whose pin moved
- **THEN** it prints the moved state and exits successfully, having written no journal

#### Scenario: A world with no crossing is byte-identical
- **WHEN** `brokkr realms` reads a world that draws no crossing
- **THEN** its output is exactly the output it produced before crossings existed

### Requirement: brokkr muninn run carries crossings into the dossier
`brokkr muninn run` SHALL read the world's one crossing report into the
fleet dossier: per realm, what it publishes and consumes, with a moved
pin raised as a FINDING under the CONSUMING realm and citable by that
realm and the crossing's name. An invented crossing, or one charged to
the wrong realm, SHALL be refused rather than recorded; a matching or
unchecked pin SHALL NOT be a finding; and the flight SHALL write no run
journal (decision 0020 ruling 3; decision 0026 ruling 3; decision 0054).

#### Scenario: A moved pin is a citable finding of the consumer
- **WHEN** the dossier states a moved pin
- **THEN** a finding names the consuming realm and the crossing, and a proposal citing that pair validates

#### Scenario: An invented crossing citation is refused
- **WHEN** a report cites a crossing the dossier does not state as a finding, or states it under another realm
- **THEN** the report is refused and nothing is recorded

#### Scenario: A matching pin is not a finding
- **WHEN** every crossing matches
- **THEN** the dossier states the crossings per realm and raises no crossing finding

#### Scenario: A world with no crossing hands the seat the same dossier
- **WHEN** the world draws no crossing
- **THEN** the dossier carries no crossing key and no crossing finding, and no journal is written

## Provenance

- `2026-09-10-crossing-enactment` — folded 2026-09-10
