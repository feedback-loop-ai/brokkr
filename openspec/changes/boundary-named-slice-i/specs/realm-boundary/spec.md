# realm-boundary

## Purpose

The boundary word: what stands between a box's hands and the machine,
named by decision 0046 ruling 1 as a closed vocabulary, declared by the
realm and never by a bundle.

## ADDED Requirements

### Requirement: The boundary vocabulary is closed and lives in one type
The system SHALL name a box's boundary with exactly one of five words —
`namespace`, `seatbelt`, `container`, `harness`, `open` — defined once as
a shared type in `brokkr-core` with a parse and display pair, so that
every crate that reads, pins, records or renders a boundary uses the same
closed vocabulary (decision 0046 ruling 1: the enumeration is frozen the
way a contract is). A sixth word SHALL be refused wherever a boundary is
parsed, with a message that lists the five words and says a new boundary
is a new decision.

#### Scenario: Each word parses and prints itself
- **WHEN** each of `namespace`, `seatbelt`, `container`, `harness` and `open` is parsed as a boundary
- **THEN** it parses, and displaying it yields the same word

#### Scenario: An unknown word is refused by name
- **WHEN** `chroot` is parsed as a boundary
- **THEN** parsing is refused with a message that lists the five words and says a new boundary is a new decision, citing decision 0046

### Requirement: The realm map declares the boundary as forge.realms/v4
`contracts/realms.v4.schema.json` SHALL be v3 plus one optional
per-realm property `boundary` whose value is the five-word enum and whose
description says that absence reads `namespace`, under the schema
constant `forge.realms/v4`. The map loader in `brokkr-core` SHALL read
v1, v2, v3 and v4 maps; SHALL refuse a realm naming `boundary` in a map
calling itself v1, v2 or v3, naming the realm, the field and the version
that would admit it, exactly as `journal`, `house` and `dialect` are held
to theirs; and SHALL keep the vocabulary closed at both levels
(decision 0046 ruling 1; decision 0023's rule that a version is a promise
about what a file may say).

#### Scenario: A v4 map with every word loads
- **WHEN** a `forge.realms/v4` map declares five realms whose `boundary` fields are the five words
- **THEN** the map loads and each realm reports the word it declared

#### Scenario: Absence reads namespace
- **WHEN** a `forge.realms/v4` map declares a realm without `boundary`
- **THEN** the realm's resolved boundary is `namespace`
- **AND** a realm in a v3 map, and a repository run with no map at all, resolve to `namespace` the same way

#### Scenario: The word under an older label is refused
- **WHEN** a map calling itself `forge.realms/v3` declares `boundary` on a realm
- **THEN** loading is refused with a message naming the realm, `boundary`, and `forge.realms/v4` as the version that admits it

#### Scenario: An unknown word in a v4 map is refused
- **WHEN** a `forge.realms/v4` map declares `"boundary": "chroot"` on a realm
- **THEN** loading is refused naming the realm and the five words

#### Scenario: The schema file is a contract beside the frozen ones
- **WHEN** the frozen-contracts test runs
- **THEN** `contracts/realms.v4.schema.json` exists with the title `Forge realms map v4`, the v1, v2 and v3 realms files keep their pinned bytes, and the v4 schema refuses a `boundary` outside the enum and a map whose `schema` is not `forge.realms/v4`

### Requirement: A run's boundary is the realm's, resolved at compile
Bundle compilation SHALL take the boundary of the realm it compiles in —
`namespace` when it compiles in no realm — and SHALL expose it on the
compiled bundle beside the manifest. `brokkr run` and `brokkr compile`
SHALL compile against the boundary of the operated repository's realm
in the discovered or named map; `brokkr resume` SHALL compile against
the realm embedded in the run's pinned world and never against the
workspace's map as it stands today, as it already does for the dialect;
`brokkr rerun` compiles as it does today, without a world and therefore
under `namespace`. Compilation consults no machine and no engine
capability: a realm declaring `seatbelt` or `container` compiles and
pins the word, and the refusal that stops a run under either comes at
start and is boundary-availability's (decision 0046 ruling 1; the
resume rule of decision 0042 ruling 1's enactment).

#### Scenario: Run resolves the operated realm's boundary
- **WHEN** a `forge.realms/v4` map declares the operated repository's realm with `"boundary": "harness"` and `brokkr run` compiles a bundle in it
- **THEN** the compiled bundle's boundary is `harness` and its manifest's `boundary` map says so for every hands site

#### Scenario: Resume reads the pinned world, not the file
- **GIVEN** a run started under a map that declared no boundary
- **WHEN** the workspace's `realms.json` now declares `harness` and `brokkr resume` compiles the run's bundle
- **THEN** the bundle compiles under `namespace`, the pinned manifest matches, and the resume proceeds under the boundary the run was started with

#### Scenario: Compile without a map is namespace
- **WHEN** a bundle compiles through `Bundle::compile_with` with no realm context
- **THEN** its boundary is `namespace` and its manifest is what this tree's witness table pins

#### Scenario: A seatbelt realm compiles and pins the word
- **WHEN** a `forge.realms/v4` map declares the operated repository's realm with `"boundary": "seatbelt"` and `brokkr compile` runs over a boxed bundle on a machine without `sandbox-exec`
- **THEN** the bundle compiles and its manifest's `boundary` map says `seatbelt` for every hands site; the refusal that stops a run under it comes at start and names slice (ii)

### Requirement: A bundle never names the boundary
The bundle parser SHALL refuse a `boundary` key at any site — a seat, a
panel member, a sequence step, a selected case body — and inside any
`hands` object, in a bundle and in an agent file alike, with a message
that names the site, says the boundary is declared by the realm
(`realms.json`, `forge.realms/v4`) and never by a bundle, and cites
decision 0046 ruling 1. The refusal SHALL name the realm as the field's
home rather than reporting an unknown key, so the author is told where
the word lives.

#### Scenario: A seat-level boundary is refused
- **WHEN** a seat declares `"boundary": "harness"` beside `hands`
- **THEN** compilation is refused naming the seat, the realm map as the field's home, and decision 0046

#### Scenario: A member, a step and a case cannot name it either
- **WHEN** a panel member, a sequence step, or a selected case body declares `boundary`
- **THEN** compilation is refused with the same message naming that site

#### Scenario: A boundary inside hands is refused
- **WHEN** a seat declares `"hands": {"kind": "workspace", "boundary": "open"}`
- **THEN** compilation is refused naming the realm as the field's home rather than as an unknown `hands` key

#### Scenario: An agent file cannot name it
- **WHEN** an agent's `hands` object carries `boundary`
- **THEN** loading the library refuses the agent with the same message

### Requirement: brokkr compile prints the boundary under each hands site
`brokkr compile` SHALL print the compiled bundle's manifest, as it does
today, and under v9 that manifest SHALL carry `boundary` keyed by the
same site labels as `hands`, so an operator reading the printout finds
each boxed site's boundary beside its box spec. The printout SHALL gain
no second copy of the map: the manifest is the one derivation of the
bundle's identity, and a copy printed beside it could drift from it. A
bundle with no hands site SHALL print a manifest with neither key
(decision 0046 ruling 1's enforcement binding; decision 0013).

#### Scenario: A boxed bundle prints its sites' boundary
- **WHEN** `brokkr compile --bundle bundles/self` runs in this repository
- **THEN** the printed JSON's `manifest.boundary` carries the value `namespace` for every key of `manifest.hands` and no other key, and the printed JSON carries no `boundary` outside the manifest

#### Scenario: A plain bundle prints none
- **WHEN** `brokkr compile` runs over a bundle with no hands site
- **THEN** the printed manifest carries neither `hands` nor `boundary`
