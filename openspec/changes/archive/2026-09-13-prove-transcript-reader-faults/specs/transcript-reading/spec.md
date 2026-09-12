## ADDED Requirements

### Requirement: Reader failure handling is proved through a test-only fault seam

Tests SHALL prove how the shared local reader handles filesystem failures and
races by driving the production handling itself and asserting the outcome
that the other `transcript-reading` requirements already fix. A test SHALL
prefer, in this order: an ordinary input; a deterministic real filesystem
change; an injected I/O error. It SHALL use an injected error only where no
ordinary input and no real change reaches the handling. A real change that
must fall between two reader operations SHALL be timed through one narrow
fault seam at the handle-based reader boundary. The seam's targets SHALL be:

- directory enumeration;
- child open;
- handle identity;
- bounded read;
- the point inside a child open between its directory attempt and its file
  attempt;
- the point before the read-boundary acquisition re-walk.

A scripted entry SHALL name one target and the occurrence at which it fires.
Occurrences SHALL count that target's operations or points on the installing
thread since the plan was installed, starting at one. Each entry SHALL perform
exactly one action, and each target SHALL accept only its own action:

- Directory enumeration, handle identity and bounded read accept only an
  injected I/O error. The error is returned in place of that operation's
  result, and the operation is not performed. It SHALL carry
  `std::io::ErrorKind::Other`, a kind that no reader caller maps to absence.
- Child open and both points accept only a test-owned filesystem change
  inside the test's synthetic home. The change runs there, before the real
  child open or the reader's next real operation, and that real operation
  then classifies what it finds.

No other pairing SHALL be expressible. Because child open and both points
accept no error, no injected error reaches the platform code that classifies
absence, unsafety or a file type. The seam
SHALL NOT give the reader a handle, path, name, file type or identity value.
It SHALL NOT change the canonical root, the handle-relative opening, the
no-follow and non-blocking flags, or the regular-file and reparse checks that
any real operation applies. Absence, replacement and a changed identity SHALL
come only from real filesystem changes that the production code then observes.

The seam SHALL exist only in the `brokkr-cli` unit-test configuration. No
environment variable, command argument, configuration key, file, journal
value or runtime flag SHALL reach it. A build without that configuration SHALL
contain none of it, so a reference to the seam in such a build is a compile
error. That covers the release binary, packages and integration-test builds.
A scripted plan SHALL be visible only to the thread that installed it and
SHALL end with the test that installed it. An entry that never fires SHALL
fail that test. No entry SHALL be disarmed, and no test SHALL be exempted from
that check. The unix and Windows implementations SHALL share the seam
through the platform-independent helper, and each SHALL carry the point
between its directory and file attempts. No pathname fallback implementation
exists to carry it.

The unchanged exact-coverage gate SHALL count the seam's own code. A handling
arm that no ordinary input, real change or scripted I/O error can reach SHALL
be removed by restructuring, and the proof that it is unreachable SHALL be
recorded. Reachable handling SHALL NOT be removed. None of these SHALL be used
to close the gate: coverage exclusion attributes, build-configuration
carve-outs, harness-shaped file names for production code, lowered
thresholds, or edits to `scripts/coverage-exact.sh`.

#### Scenario: An enumeration failure is a discovery-stage unreadable result
- **WHEN** a test scripts an I/O error for the enumeration of a Claude projects root, a Codex sessions directory, or a DSH seat or project directory during an otherwise valid lookup
- **THEN** the shared read returns `unreadable` with null path, no turns, zero counts, no truncation and no notices, and it keeps the kind's unresolved full-session hint
- **AND** browser presentation reports the same discovery-stage `unreadable` with admission closed

#### Scenario: A failed candidate identity prevents a unique answer
- **WHEN** a scripted I/O error fails the identity check of a matching Claude, Codex or DSH candidate during discovery, whether or not another entry yields a qualifying candidate
- **THEN** the read returns discovery-stage `unreadable` and admits no candidate, rather than admitting the survivor

#### Scenario: A failed DSH opening-header read is discovery-stage unreadable
- **WHEN** a scripted I/O error fails the bounded opening-header read of the only DSH session candidate
- **THEN** the read returns `unreadable` with null path and null DSH hint, no turns and zero counts

#### Scenario: An entry that disappears before its open is not a candidate
- **WHEN** a test-owned change removes the only matching Codex rollout after the reader has enumerated its directory and before the handle-relative open
- **OR** a test-owned change removes a matching Claude session file after the reader's directory attempt on it fails and before its file attempt
- **THEN** the reader classifies the real absent answer, admits no candidate and returns `not-found` with null path
- **AND** it reports neither `unsafe-path` nor `unreadable` and reads no content

#### Scenario: A changed acquisition fails closed before any byte is read
- **WHEN**, after discovery has admitted a unique safe source, a test-owned change at the point before the read-boundary re-walk either renames that file to another qualifying location or renames a new file over its path
- **THEN** the re-walk finds a different path or identity and refuses the read with body-stage `unreadable`, no turns, zero counts and no truncation
- **AND** no prose from either file appears on any surface

#### Scenario: A failed retained-handle check or bounded read refuses without prose
- **WHEN**, after discovery has admitted a source, a scripted I/O error fails either the retained handle's identity recheck or its bounded source read
- **THEN** the read returns body-stage `unreadable` with no turns, zero counts and no truncation, and the retained file keeps its bytes

#### Scenario: A scripted fault never leaks beyond its test
- **WHEN** one test thread has installed a plan while another test thread reads its own fixture, and later the installing test ends
- **THEN** the other thread's read sees only real filesystem results, and no entry of the plan outlives the installing test

#### Scenario: A scripted fault that never fires fails its proof
- **WHEN** a test scripts an entry for an occurrence of an operation that the reader never reaches
- **THEN** that test fails instead of passing on a path it did not exercise

#### Scenario: A scripted entry accepts only its target's action
- **WHEN** a test tries to script an I/O error at a child open, at the point between a child open's directory and file attempts or at the point before the re-walk, or a filesystem change at an enumeration, an identity check or a bounded read
- **THEN** the seam offers no constructor for that pairing, so the entry cannot be written and no read runs under it
- **AND** a scripted error at enumeration, identity or bounded read carries `std::io::ErrorKind::Other`, and no read reports `not-found` or `unsafe-path` because of it

#### Scenario: The seam leaves the boundary's checks in force
- **WHEN** a test installs a plan whose only entry is an enumeration error at the occurrence that falls in a separate, otherwise valid lookup made after the asserted reads
- **AND** the reader first reads a symlinked project, a symlinked candidate, a FIFO candidate and a regular candidate
- **THEN** each asserted read keeps its canonical root, handle-relative opening, no-follow, non-blocking and regular-file checks, and those fixtures keep their `unsafe-path` and readable outcomes
- **AND** the later lookup returns discovery-stage `unreadable`, so the entry fired and the unfired-entry check passes unchanged
- **AND** an occurrence that drifts into an asserted read changes that read's outcome, and one that drifts past the later lookup never fires, so either drift fails the test

#### Scenario: A release build has no fault switch
- **WHEN** brokkr is built without the unit-test configuration, as for the release binary, a package or an integration-test build
- **THEN** none of the seam's code is compiled, and a reference to it in that build is a compile error
- **AND** no environment variable, argument, configuration key, file or journal value can make a reader operation fail or run a scripted change

#### Scenario: Unreachable handling is removed with its proof
- **WHEN** an uncovered arm can be reached by no ordinary input, real filesystem change or scripted I/O error, such as an emptiness check after percent-decoding that the check before decoding already implies
- **THEN** that arm is removed by restructuring and the unreachability argument is recorded
- **AND** every arm that one of those routes does reach keeps its handling and gains a test asserting its specified outcome

## MODIFIED Requirements

### Requirement: Transcript prose stays local and inert

Reading SHALL open the journal read-only and SHALL append no events or
checkpoints. Prompt text, reasoning, tool arguments and output SHALL not
enter inspect/seats/watch JSON, exports, dossiers, result telemetry, the
journal or a new persistent transcript cache. Only the explicit transcript
command and existing local transcript surfaces SHALL expose their requested
prose. Plain-text terminal rendering SHALL sanitize control/escape sequences
using the same terminal safety rules as the existing TUI; JSON SHALL preserve
content as JSON-escaped strings. Reading or displaying full-session hints
SHALL execute no provider command, tool call, URL, shell fragment or file
contents, and SHALL not change adapter resumption or sandbox behavior.

The read-only journal open SHALL write no journal content. When SQLite reads
a quiescent write-ahead-log journal that has no sidecars, it may create an
empty `-wal` and a `-shm` shared-memory index beside it. This is SQLite's own
read-only behavior, and those files carry no journal content. A read SHALL
NOT:

- change the database bytes, the event count or the hash;
- write a frame into any `-wal`;
- change an existing `-wal`;
- open the journal read-write, migrate, repair or checkpoint it.

A `-wal` that an earlier read left behind is an existing `-wal`, so a later
read SHALL leave it as it found it. An existing `-shm` is SQLite's
shared-memory index of the `-wal`. Readers coordinate through it, so its
bytes may change during a read, and it carries no journal content. A missing
journal SHALL gain no file of any kind: no database, no `-wal` and no `-shm`.

#### Scenario: A tool output cannot become terminal control or a journal event
- **WHEN** a transcript contains an ANSI escape, a shell command, a credential-shaped sentinel and tool output
- **THEN** explicit local JSON contains the requested content as escaped strings, text/TUI output neutralizes terminal controls, and the journal event count/hash and its exports gain none of that transcript content

#### Scenario: Inspect remains a reference-only readout
- **WHEN** inspect, seats, watch or a dossier is derived for a seat whose transcript can now be read
- **THEN** its common reference and existing accounting remain unchanged and no transcript body is attached to the journal-derived view

#### Scenario: A read-only open leaves no journal content behind
- **WHEN** a test runs the transcript command's reads in sequence over one quiescent write-ahead-log journal, either a successful read and then a refusal or repeated reads after the retained source grows
- **AND** the test takes its before-state once, before the first read, when the journal has no `-wal` or `-shm`
- **THEN** after each read the database bytes, event count and hash equal that before-state, and no event, checkpoint or migration was written
- **AND** a `-wal` that appears during the first read is empty or holds no frame, and a `-shm` index may appear
- **AND** each later read finds the `-wal` an earlier read left and leaves it with the digest it had just before that read, so it still holds no frame; the `-shm` is not compared

#### Scenario: A read leaves an existing write-ahead log as it found it
- **WHEN** the same three reads open a quiescent journal whose `-wal` holds at least one committed frame, because the writer that appended the seat's transcript reference stays open and idle across the reads
- **AND** the test has confirmed that the `-wal` holds a frame before the first read
- **THEN** each read resolves that reference, that `-wal` keeps its digest and the database bytes are unchanged
- **AND** the existing `-shm` is not compared, because it is SQLite's shared-memory index and not journal content

#### Scenario: A missing journal gains no file on any read surface
- **WHEN** the transcript command is given `--db` naming a journal that does not exist
- **OR** the browser server answers `/api/runs`, `/api/run/<run>`, `/api/view/<run>` and `/api/presentation/<run>/<key>`, and one poll of its run stream `/sse/<run>`, over that path
- **OR** the TUI is started with `--db` on that path
- **THEN** the command and the TUI refuse, each route answers not found, and the run stream reports head sequence zero
- **AND** after each of them that path, its `-wal` and its `-shm` still do not exist
