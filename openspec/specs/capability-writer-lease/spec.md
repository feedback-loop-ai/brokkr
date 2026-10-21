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

## Provenance

- `2026-10-20-772-foreign-reservations` — folded 2026-10-20
