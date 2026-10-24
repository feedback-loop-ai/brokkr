## ADDED Requirements

### Requirement: The dispatch door reads this run's reservation under the admission lock

At every dispatch door, a seat's, a sequence step's and each panel member's, admission SHALL read the attempt's reservation only while holding the admission lock, taken before MB3's startup deadline as the driver's own settlement record is sealed under it, and SHALL release the lock once the record is read: before the writer's reach is observed again, before the artifact root's identity is compared, and before the driver's settlement record is written, which takes the lock again. Every seal of a reservation holds the lock from its first step to its last sync, so the door's read SHALL find the record that stood before a sibling's seal, the record that seal committed, or the one its undoing put back, and SHALL NOT be refused for the seal's own transient state.

#### Scenario: A sibling member seals the reservation while another member's door reads it

- **WHEN** two members of one panel attempt reach the door together, one writing its settlement record into the reservation while the other rechecks the lease, under load that deschedules the reader between the seal's link and its unlink of the prior
- **THEN** the reader waits for the seal and reads the record it committed, the attempt still standing in it; the member's driver spawns; the journal holds no member of the attempt whose driver did not spawn; and before the attempt is released its reservation holds one settlement record per member

#### Scenario: Readers beside many seals

- **WHEN** three reader threads each recheck a standing lease a fixed count of times beside one sibling thread that reseals the same reservation through the attempt's amendment a fixed count of times, all released together, every take of the lock bound to one deadline shared by the whole stress and shorter than MB3's startup bound
- **THEN** every read returns admitted and every seal reports committed and synced, so a read refused as the reservation no longer standing, as admission not completed before its deadline or as the lock not this user's own, and a seal late or not proven synced, each fail by name; every reader completes its count before the deadline, so none passes by reading less; and with the door's lock take removed the same stress refuses reads as the reservation no longer standing under load, its count recorded

#### Scenario: The attempt is truly gone

- **WHEN** the lease's reservation no longer names its attempt standing when the door reads it under the lock, or the run's directory holds no reservation that can be read
- **THEN** the door releases the lock and refuses as the attempt's reservation no longer standing, as today, and nothing is sealed

#### Scenario: What the door checks after its read runs without the lock

- **WHEN** the door has read the reservation and goes on to observe the writer's sealed reach again, compare the artifact root admitted, and write the driver's settlement record
- **THEN** the admission lock is not held across those checks; the settlement record's seal takes it again, and a sibling may seal between the two takes

### Requirement: A lock the door does not get is refused by the lock's own name

A door whose admission lock is not held exclusively by the startup deadline SHALL refuse the dispatch as admission not completed before its startup deadline, and a lock file that is not this user's own single regular file SHALL refuse it as the admission lock not this user's own, each with the text the same refusal carries at admission and at the attempt's amendment; a run directory the door cannot enter SHALL keep its own refusal too. None of these SHALL be reported as the reservation no longer standing, and no refusal the door makes today SHALL change its variant or its text. The lock is taken before the reach is observed, so a door refused by the lock's name SHALL observe nothing and spawn nothing.

#### Scenario: The lock is released before the deadline

- **WHEN** another thread holds the admission lock exclusively while the door rechecks a lease whose reservation stands, and releases it before the deadline
- **THEN** the door returns admitted only after the release, not before it, shown by the order of the two and not by the time between them

#### Scenario: The lock is held past the deadline

- **WHEN** another thread holds the admission lock exclusively and the door's deadline is spent before it is released, the lease staked with an authored reach the door would report redirected if it looked
- **THEN** the door refuses as admission not completed before its startup deadline, not as redirected; the reach is not observed, the driver does not spawn, and the reservation's bytes are unchanged

#### Scenario: The lock file carries a second link

- **WHEN** the coordination root's lock file carries a second link when the door opens it, as the admission suite makes one for admission itself
- **THEN** the door refuses as the admission lock not this user's own, nothing is read or sealed, and the reservation's bytes are unchanged; a directory, a link or another user's file at the lock's name is refused by the same open and check, proven once, at the lock

#### Scenario: The lock line removed

- **WHEN** the door's read is made without taking the lock, as a mutation of the build
- **THEN** the door returns admitted through a lock another thread holds, before the holder releases it, and the test that holds the lock fails on that order: it is the test that binds this requirement

### Requirement: No refusal of admission comes from a read of a reservation taken outside the lock

A reservation read outside the admission lock can find a sibling's seal between its link and its unlink and be refused for it, for the read rule finds no record there. Admission SHALL make no refusal of a dispatch, an amendment of the attempt, an adoption or an attestation from a read taken outside the lock: every such read SHALL hold the lock, taken the one way admission takes it. Today those reads are admission itself, the dispatch door's recheck, every amendment of the attempt (the driver's record written Settling at the door and sealed Settled at its end, the unresolved mark, the positive settlement fact and the release), the adoption of an attempt by a restarted engine, the extension of a writer's sources, and the attestation of a run's end.

#### Scenario: The readers audited

- **WHEN** the production readers of a reservation are enumerated, every read going through the root's read of a standing record
- **THEN** each holds the lock: admission under its own; the door's recheck under the lock this change takes; the amendment under the lock it takes before its read and its seal, for all five of its callers; adoption under the lock its only caller takes; the inventory's read under the lock of admission, of the sources' extension, or of attestation, taken before the reservations are gathered; no crate outside the engine reads one; and the commit that lands this change names each reader and its lock

#### Scenario: No caller of the door holds the lock

- **WHEN** a driver's spawn reaches the door from the engine's dispatch of a seat or a sequence step, from a panel member's scoped thread, or, in a test, from the restart's fence peer after the adoption released its lock
- **THEN** the calling thread holds no admission lock, for a lock held on it would contend with the door's own descriptor and wait out the deadline; admission, the sources' extension, adoption and attestation each release theirs on return, before any door
