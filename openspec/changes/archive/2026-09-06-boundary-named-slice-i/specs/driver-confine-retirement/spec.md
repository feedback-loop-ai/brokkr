# driver-confine-retirement

## Purpose

Decision 0008's container confinement retires into the `container`
boundary. Until slice (iii) measures that boundary the field is refused
by name, and the wrapper nobody exercised is deleted (decision 0046
ruling 5).

## ADDED Requirements

### Requirement: driver.confine is refused by name
The bundle parser SHALL refuse a `driver.confine` key at any site, in a
bundle and beside `agent:` alike, with a message that names the site,
says decision 0008's container confinement retired into the `container`
boundary declared by the realm, says it is refused until that boundary
is measured in decision 0046's slice (iii), and cites decision 0046
ruling 5. No shipped bundle declares the field, so nothing shipped moves
for this reason.

#### Scenario: An inline seat with confine is refused
- **WHEN** a seat declares `"driver": {"command": ["./drivers/x"], "confine": {"image": "img"}}`
- **THEN** compilation is refused naming the seat, the `container` boundary, and decision 0046 ruling 5

#### Scenario: An agent seat with confine is refused the same way
- **WHEN** a seat declares `agent` beside `"driver": {"confine": {"image": "img", "network": true}}`
- **THEN** compilation is refused with the same message, and `driver.confine` is no longer the one `driver` key legal beside `agent`

#### Scenario: Shipped bundles carry none
- **WHEN** every bundle under `recipes/` and `bundles/` is walked
- **THEN** no site declares `driver.confine`, and every shipped bundle still compiles

### Requirement: The docker wrapper is gone
The engine SHALL carry no `docker run` wrapper: `confined_command`, the
`Confine` type, and every `confine` field on seat bodies, panel members,
sequence steps and executable bodies SHALL be deleted;
`crates/brokkr-runtime/tests/confine_test.rs` SHALL be deleted with the
function it proved; and the machine-proof scenario that needed a working
docker SHALL be removed. The argv of every seat that never declared the
field SHALL be unchanged (decision 0046 ruling 5's enforcement binding).

#### Scenario: No docker in the runtime
- **WHEN** the runtime and cli sources are searched for `docker run`, `confined_command` and `Confine`
- **THEN** none is found outside prose that explains the retirement

#### Scenario: Every other argv is unchanged
- **WHEN** the engine composes the argv of a seat, a panel member and a sequence step that never declared `driver.confine`
- **THEN** each argv equals what the engine composed before the deletion
