# capability-writer-lease Specification

## Purpose
Keep every managed writer on the host in one protected inventory of run
reservations (decision 0065 slice two, U8a2a), and read every other run's
reservation, whichever Brokkr build wrote it, through what admission compares
and recovers by (decision 0090).

## Requirements

### Requirement: This run's own reservation is read whole

Admission SHALL read the reservation in the run directory named by the claim's worktree key and run id only as this build's whole record, refusing any field it does not know. A record there that it cannot read whole SHALL refuse dispatch with MB3's identity cause, naming the file, before anything of the run is sealed.

#### Scenario: An earlier attempt of the same run was written by another build

- **WHEN** this run's own reservation holds a boxed writer whose `sealed` is the list an older build wrote, or names a `format`
- **THEN** dispatch is refused as an unknown participant at that file, and nothing of the run is sealed

### Requirement: Every other run's reservation is read through the participation view

Admission SHALL read every other run's reservation whole where this build can, and otherwise through a view holding only the worktree key, the run, its artifact root, its journal, its settled attempts, and each standing attempt's `protects`, `cleanup`, `serves` and writers, each writer by its site and its reach: unbounded, or boxed to its writable roots beside the sources it binds apart. The view SHALL ignore every other field. It SHALL read a `protects` it does not know as evidence at stake, and a `cleanup` it does not know as not proven settled. A missing field the view needs, a reach of a kind it does not know, or a `format` other than 1 SHALL refuse dispatch with MB3's identity cause, naming the file. A `format` refusal SHALL name the format too. Every writer a record read through the view reserves SHALL be compared as any other participant's is.

#### Scenario: A live run of an older build, kept apart

- **WHEN** another worktree's run, written by a build whose `sealed` is a list, holds a standing attempt whose writers are boxed apart from this run's artifact root and the coordination root
- **THEN** this run is admitted, with nothing at stake, with evidence at stake on either side, and with the fence lifted, and the other run's file is unchanged

#### Scenario: A live run of an older build, reaching what this run protects

- **WHEN** that run's writable root covers this run's protected artifact root, or the coordination root once the fence is lifted, or its writer is unbounded beside evidence
- **THEN** dispatch is refused with CR2's cause, naming that run's site and run

#### Scenario: A record the view cannot read

- **WHEN** another run's reservation is not JSON, lacks its attempts, its worktree key or a boxed writer's writable roots, or names a reach of an unknown kind
- **THEN** dispatch is refused as an unknown participant at that file, and nothing of this run is sealed

### Requirement: A record read only through the view is never sealed again

Recovery SHALL release a standing attempt of a record read only through the view only on the same durable facts as any other attempt: its positive settlement fact and a certified end in a journal no participant's writer can reach. It SHALL release that attempt for the one admission only, and SHALL NOT write the record.

#### Scenario: Another build's attempt proven settled

- **WHEN** another build's run on this worktree holds an attempt whose cleanup is `settled` and whose journal certifies its end
- **THEN** this run is admitted, and the other run's file holds the bytes it held before

#### Scenario: Another build's attempt not proven

- **WHEN** that attempt's cleanup is still settling, or a value this build does not know
- **THEN** dispatch is refused because that run holds the worktree, and its file is unchanged

### Requirement: The dispatch door reads this run's reservation under the admission lock

At every dispatch door, a seat's, a sequence step's and each panel member's, admission SHALL read the attempt's reservation only while holding the admission lock, taken before MB3's startup deadline as the driver's own settlement record is sealed under it, and SHALL release the lock once the record is read: before the writer's reach is observed again, before the artifact root's identity is compared, before the door's later checks of the launch record, the charter and the re-walk, and before the driver's settlement record is written, which takes the lock again. Every seal of a reservation holds the lock from its first step to its last sync, so the door's read SHALL find the record that stood before a sibling's seal, the record that seal committed, or the one its undoing put back, and SHALL NOT be refused for the seal's own transient state.

#### Scenario: A sibling member seals the reservation while another member's door reads it

- **WHEN** two members of one panel attempt reach the door together, one writing its settlement record into the reservation while the other rechecks the lease, under load that deschedules the reader between the seal's link and its unlink of the prior
- **THEN** the reader waits for the seal and reads the record it committed, the attempt still standing in it; the member's driver spawns; the journal holds no member of the attempt whose driver did not spawn; and before the attempt is released its reservation holds one settlement record per member

#### Scenario: A seal in flight is waited for

- **WHEN** another thread holds the admission lock exclusively, as every seal of a reservation holds it from its first step to its last sync, and the reservation stands as a reader descheduled between a sibling's link and its unlink composes it, the record of two links and no prior, which the read rule refuses; then the record is made whole again and the lock released
- **THEN** the door answers nothing while the lock is held, and admits once it is released, shown by the order of the two and not by the time between them; the hold bounds only how long a door that reads without the lock is given to answer, and a door that waits on the lock cannot fail by it

#### Scenario: The attempt is truly gone

- **WHEN** the lease's reservation no longer names its attempt standing when the door reads it under the lock, or the run's directory holds no reservation that can be read
- **THEN** the door releases the lock and refuses as the attempt's reservation no longer standing, as today, and nothing is sealed

#### Scenario: What the door checks after its read runs without the lock

- **WHEN** the door has read the reservation and goes on to observe the writer's sealed reach again, compare the artifact root admitted, judge the launch record, read the charter, re-walk the script directory and write the driver's settlement record
- **THEN** the admission lock is not held across those checks, whose order, variants and texts are as today; the settlement record's seal takes it again, and a sibling may seal between the two takes

### Requirement: A lock the door does not get is refused by the lock's own name

A door whose admission lock is not held exclusively by the startup deadline SHALL refuse the dispatch as admission not completed before its startup deadline, and a lock file that is not this user's own single regular file SHALL refuse it as the admission lock not this user's own, each with the text the same refusal carries at admission and at the attempt's amendment; a run directory the door cannot enter SHALL keep its own refusal too. None of these SHALL be reported as the reservation no longer standing. The door's set of refusals SHALL be unchanged: no variant added or removed, and no variant's text changed. The lock SHALL be the door's first take, once a spawn composed refused is turned away: before the run's directory is entered, the reservation read, the writer's reach observed or the artifact root compared, and before every check the door makes after its read, today the launch record's, the charter's two and the re-walk's, which run ahead of the door's one lock take today, the settlement record's. So a lease refused twice over, by the lock and by any later check of the door, SHALL report the lock's refusal where today the door reports that check's and reaches no lock: the coordination root not protected, the reservation no longer standing, the reach redirected or the artifact root replaced, from the recheck; or the launch record, the charter text carried, the charter moved or the re-walk refused, from the door after it. Only a spawn refused as composed SHALL keep its place ahead of the lock. No refusal's variant or text SHALL change; what moves is which of its two refusals a doubly-refused lease reports, and a door refused by the lock's name SHALL observe nothing and spawn nothing.

#### Scenario: The lock is held past the deadline

- **WHEN** another thread holds the admission lock exclusively past the deadline of a take admission makes, the one way every take is made
- **THEN** that take refuses as admission not completed before its startup deadline, proven where the lock is proven, at admission with the lock held; the door's take is the same open, the same wait and the same conversion, its deadline MB3's startup bound, so the door inherits that proof and its own proof at the door is the lock file's; no reach is observed and no driver spawns

#### Scenario: The lock file carries a second link

- **WHEN** the coordination root's lock file carries a second link when the door opens it, as the admission suite makes one for admission itself
- **THEN** the door refuses as the admission lock not this user's own and nothing is read or sealed, and with the link gone the same lease is admitted; a directory, a link or another user's file at the lock's name is refused by the same open and check, proven once, at the lock

#### Scenario: A lease the lock refuses that the door would also refuse after its read

- **WHEN** the door's lock is refused, held past the deadline or carrying a second link, for a lease whose spawn the door would also refuse after the recheck: an input carrying a launch record the engine sealed none for, charter text arriving before the door read one, a charter moved since the compile, or a script directory moved since the walk
- **THEN** the door refuses by the lock's name, as admission not completed before its startup deadline or as the admission lock not this user's own, where today it reports the later check's refusal and reaches no lock; that check is not run, no record judged, no charter read, no directory re-walked, nothing sealed and nothing spawned; a lease whose lock is held past the deadline is refused when the deadline is spent, not at once, as a lease that passed those checks already is at the settlement record's seal

#### Scenario: The lock line removed

- **WHEN** the door's read is made without taking the lock, as a mutation of the build
- **THEN** the seal in flight is read through the lock another thread holds and refused at once as the reservation no longer standing, before the holder releases it, and a lock file carrying a second link admits; the two tests fail on that answer and on that admission: they are the tests that bind this requirement and the first

### Requirement: No refusal of admission comes from a read of a reservation taken outside the lock

A reservation read outside the admission lock can find a sibling's seal between its link and its unlink and be refused for it, for the read rule finds no record there. Admission SHALL make no refusal of a dispatch, an amendment of the attempt, an adoption or an attestation from a read taken outside the lock: every such read SHALL hold the lock, taken the one way admission takes it. Today those reads are admission itself, the dispatch door's recheck, every amendment of the attempt (the driver's record written Settling at the door and sealed Settled at its end, the unresolved mark, the positive settlement fact and the release), the adoption of an attempt by a restarted engine, the extension of a writer's sources, and the attestation of a run's end.

#### Scenario: The readers audited

- **WHEN** the production readers of a reservation are enumerated, every read going through the root's read of a standing record
- **THEN** each holds the lock: admission under its own; the door's recheck under the lock this change takes; the amendment under the lock it takes before its read and its seal, for all five of its callers; adoption under the lock its only caller takes; the inventory's read under the lock of admission, of the sources' extension, or of attestation, taken before the reservations are gathered; no crate outside the engine reads one; and the commit that lands this change names each reader and its lock

#### Scenario: No caller of the door holds the lock

- **WHEN** a driver's spawn reaches the door from the engine's dispatch of a seat or a sequence step, from a panel member's scoped thread, or, in a test, from the restart's fence peer after the adoption released its lock
- **THEN** the calling thread holds no admission lock, for a lock held on it would contend with the door's own descriptor and wait out the deadline; admission, the sources' extension, adoption and attestation each release theirs on return, before any door

## Provenance

- `2026-10-20-772-foreign-reservations` — folded 2026-10-20
- `2026-10-24-835-recheck-under-the-lock` — folded 2026-10-25
