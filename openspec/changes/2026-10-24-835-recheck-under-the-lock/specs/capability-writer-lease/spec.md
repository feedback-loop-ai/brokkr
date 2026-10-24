## ADDED Requirements

### Requirement: The dispatch door reads this run's reservation under the admission lock

At every dispatch door, a panel member's included, admission SHALL read the attempt's reservation only while holding the admission lock, taken before MB3's startup deadline as the driver's own settlement record is sealed under it, and SHALL release the lock once the record is read, before the writer's reach is observed again, before the artifact root's identity is compared, and before the driver's settlement record is written, which takes the lock again. A seal of the reservation holds the lock across every step from its link to its final sync, so the door's read SHALL find either the record that stood before a sibling's seal or the record that seal committed, and SHALL NOT be refused for the seal's own transient state.

#### Scenario: A sibling member seals the reservation while another member's door reads it

- **WHEN** two members of one panel attempt reach the door together, one writing its settlement record into the reservation while the other rechecks the lease, under load that deschedules the reader between the seal's link and its unlink of the prior
- **THEN** the reader waits for the seal and reads the record it committed, the attempt still standing in it, and the member's driver spawns; it is never refused as the reservation no longer standing, and both members' records are in the reservation when the attempt ends

#### Scenario: Readers beside many seals

- **WHEN** readers recheck a standing lease repeatedly beside a sibling that reseals the same reservation through the attempt's amendment, for a bounded count of seals and a bounded wall time
- **THEN** no read is refused as the reservation no longer standing, and at least the floor of reads completed, so a reader starved of the lock cannot pass by not reading

#### Scenario: The attempt is truly gone

- **WHEN** the lease's reservation no longer names its attempt standing when the door reads it under the lock, or the run's directory holds no reservation that can be read
- **THEN** the door refuses as the attempt's reservation no longer standing, as today, and nothing is sealed

#### Scenario: What the door checks after its read runs without the lock

- **WHEN** the door has read the reservation and goes on to observe the writer's sealed reach again, compare the artifact root admitted, and write the driver's settlement record
- **THEN** the admission lock is not held across those checks; the settlement record's seal takes it again, and a sibling may seal between the two takes

### Requirement: A lock the door does not get is refused by the lock's own name

A door whose admission lock is not held exclusively by the startup deadline SHALL refuse the dispatch as admission not completed before its startup deadline, and a lock file that is not this user's own single regular file SHALL refuse it as the admission lock not this user's own, each with the text the same refusal carries at admission and at the attempt's amendment. Neither SHALL be reported as the reservation no longer standing, and no refusal the door makes today SHALL change its variant or its text.

#### Scenario: The lock is released before the deadline

- **WHEN** another thread holds the admission lock exclusively while the door rechecks a lease whose reservation stands, and releases it before the deadline
- **THEN** the door returns admitted only after the release, not before it

#### Scenario: The lock is held past the deadline

- **WHEN** another thread holds the admission lock exclusively and the door's deadline is spent before it is released
- **THEN** the door refuses as admission not completed before its startup deadline, the driver does not spawn, and the reservation is unchanged

#### Scenario: The lock file is not this user's single regular file

- **WHEN** the coordination root's lock file is a directory, a symbolic link, another user's, or a file of two links when the door opens it
- **THEN** the door refuses as the admission lock not this user's own, and nothing is read or sealed

#### Scenario: The lock line removed

- **WHEN** the door's read is made without taking the lock, as a mutation of the build
- **THEN** the door returns admitted through a lock another thread holds, and the test that holds the lock fails: it is the test that binds this requirement

### Requirement: Every read of a reservation holds the admission lock

Every operation of admission that reads a run's reservation SHALL hold the admission lock throughout the read: admission itself, the dispatch door's recheck, every amendment of the attempt (the driver's settlement record, the unresolved mark, the positive settlement fact and the release), adoption of an attempt by a restarted engine, the extension of a writer's sources, and the attestation of a run's end. A reader added later SHALL take the lock the one way admission takes it, through the root's lock, before its read.

#### Scenario: The readers audited

- **WHEN** the production readers of a reservation are enumerated, every read going through the root's read of a standing record
- **THEN** each holds the lock: admission under its own; the door's recheck under the lock this change takes; the amendment under the lock it takes before its read and its seal; adoption under the lock its only caller takes; the inventory's read under the lock of admission, of the sources' extension, or of attestation taken before the reservations are gathered; and the commit that lands this change names each reader and its lock

#### Scenario: No caller of the door holds the lock

- **WHEN** a driver's spawn reaches the door from the engine's dispatch, a panel member's scoped thread, or a sequence step
- **THEN** the calling thread holds no admission lock, for a lock held on it would contend with the door's own descriptor and wait out the deadline; admission and attestation release theirs before any door
