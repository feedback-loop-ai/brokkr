## ADDED Requirements

### Requirement: An attempt that ran its panel and ended short of success records the head and the tree it judged and each verdict that settled in it

A panel attempt whose terminal fact is `effect/indeterminate` or `effect/failed`, written by the panel whose outcome ended it, a seat's panel body or the panel step whose outcome ended a sequence, SHALL record `tree` and `head`, and SHALL record `settled_members` when a member settled in it. `tree` SHALL hold `at_start` and `at_end`, each one of the four readings of `working-tree-reading`, `clean`, `dirty`, `refused` or `unrun`, as the one reader read the operated repository when the panel started, before any member of it spawned, and when it ended, before its terminal fact; for a seat's panel and a sequence's first step those are the attempt's own two ends, and for a later panel step the step's. `head` SHALL hold `at_start` and `at_end`, the operated repository's head as `git_head` read it at those same two moments, where the two tree readings are taken, in the commit-id shape `phase-entered-head.v1` pins, forty lowercase hex digits, which the contract pins on each end as `^[0-9a-f]{40}$`; each end SHALL be absent where the repository answered no head or answered a value outside that shape, which the writer reads as no head as `phase-entered-head.v1` reads one, and `head` SHALL be absent when it answered at neither end. A `head` end present outside the shape is therefore a record the engine never writes, and the `panel-member-retry` capability refuses it as a record that does not read, naming the attempt and the field, never as an end that recorded no head. `tree` SHALL stand on every terminal fact the panel wrote short of success and on no other, and `head` beside it wherever the repository answered a head. `tree`'s presence is one of the three marks by which an attempt its panel ended is told from one the engine or the sequence's own checks ended in its place, the others being `effect/succeeded` and the panel step's `sequence-step-finished` marker, which the `panel-member-retry` capability's three-way test reads. `settled_members` SHALL hold, for each member whose settled outcome in that attempt is a success, the member's result value as its driver reported it and the aggregate reads it, any JSON value, an object or not, since adapters repair nothing (decision 0001), keyed by the member's tag as the markers and `carried` spell it: `<member>` for a seat's panel, `<step>:<member>` for a panel step. It SHALL NOT hold a verdict the attempt only carried, and SHALL be absent when no member settled in the attempt. All three fields SHALL be absent on every attempt whose panel did not write its terminal fact: a single-driver attempt; a sequence ended by a step that is not a panel; a sequence its own checks on the panel step's aggregate ended, after the panel's outcome succeeded and before the step's marker, by the vocabulary check that fails the attempt on an aggregate outside the step's declared results, `__member-schema-invalid__` included, by the fence's refusal of the step's marker, or by the change check, which neither aggregate can trip; an attempt that started on no driver; an attempt refused admission or provisioning, whose `effect/failed` is written before any member spawns; the `effect/indeterminate` the engine writes for a restart in flight, for journal contention, or for a held outcome the fence refused; and the `GATE-MOVED-HEAD` indeterminate a gate writes in a terminal fact's place. Those payloads keep their bytes. `unresolved_members` SHALL keep its bare-name keys. `effect/succeeded` SHALL NOT change. The fold SHALL NOT read any of them.

#### Scenario: Three settled, one not

- **WHEN** a four-member panel ends indeterminate because one member ran to no verdict, on a tree clean at both ends
- **THEN** its `effect/indeterminate` carries `settled_members` with the three members' results under their bare names, `tree` reading `{"at_start": "clean", "at_end": "clean"}`, `head` reading the repository's head at both ends, and `unresolved_members` as #403 records it when any member is not proven over

#### Scenario: A panel step's keys

- **WHEN** the review sequence's `positions` step ends the attempt indeterminate with three members settled
- **THEN** `settled_members` keys them `positions:adversarial`, `positions:security` and `positions:spec-compliance`, as the attempt's markers key them

#### Scenario: A panel that failed by one member

- **WHEN** a panel attempt fails because one member's driver failed to start and the others settled
- **THEN** its `effect/failed` carries `settled_members` with the settled members' results, `tree` and `head`, beside `error` and the start-failure fields it carries today

#### Scenario: A tree edited while the panel judged

- **WHEN** a file under the operated repository is edited and left uncommitted while a panel attempt runs, and the attempt ends indeterminate
- **THEN** its terminal fact's `tree` reads `at_end` as `dirty`

#### Scenario: A work member's commit inside the attempt

- **WHEN** a panel whose members are not all gates, so that no gate-head observation watches its effect, runs on head `H0`, a work member commits `H1`, and the attempt ends indeterminate because another member ran to no verdict
- **THEN** its terminal fact's `head` reads `{"at_start": "H0", "at_end": "H1"}`, spelled as the two commit ids, and no `GATE-MOVED-HEAD` stands for it

#### Scenario: A repository with no commit

- **WHEN** a panel attempt ends indeterminate in an operated repository that has no commit at either end
- **THEN** its terminal fact carries `tree` and no `head`

#### Scenario: A head outside the commit-id shape

- **WHEN** a panel attempt ends indeterminate in an operated repository whose `git rev-parse HEAD` answers a value that is not forty lowercase hex digits, a repository of SHA-256 object format among them
- **THEN** its terminal fact carries `tree` and no `head` at that end, as `phase-entered-head.v1` records no head for such an answer; and a carry from the attempt in the same repository, whose head now is outside the commit-id shape too, is later refused at the first head fact naming the repository as answering no head, no end compared, while an end that recorded no head is named only when the head now is in the commit-id shape, as in `panel-member-retry`'s scenario `An attempt that recorded no head`

#### Scenario: Git refuses status while the panel runs

- **WHEN** the operated repository's index file is corrupted while a panel attempt runs, overwritten with bytes that are no index, so that `git rev-parse HEAD` answers and `git status --porcelain` exits 128 printing nothing, and the attempt ends indeterminate
- **THEN** its terminal fact's `tree` reads `at_end` as `refused`

#### Scenario: A carrying attempt that settles one member

- **WHEN** a member retry names `correctness` and `security`, carries the two others, `security` settles and `correctness` runs to no verdict
- **THEN** the terminal fact's `settled_members` holds `security` alone, never the carried verdicts, and `tree` and `head` record both ends

#### Scenario: A carrying attempt the span check ended

- **WHEN** a carrying attempt's fresh member settles a verdict after a commit moved the head from the admitted `H0` to `H1` inside the attempt, so the panel ends it indeterminate for its own span (`panel-member-retry`, fifth requirement)
- **THEN** its terminal fact records `head` as `{"at_start": "H0", "at_end": "H1"}`, `tree` at both ends and the fresh verdict under `settled_members`, never a carried one, and the `panel-member-retry` capability refuses the next carry by that end

#### Scenario: The same member stuck twice

- **WHEN** a member retry names `correctness` alone, carries three verdicts, and `correctness` runs to no verdict again
- **THEN** the terminal fact carries `tree` and `head` and no `settled_members`, and the three verdicts stand only where its `effect/started.carried` names them

#### Scenario: An all-indeterminate panel

- **WHEN** every member of a panel attempt runs to no verdict
- **THEN** its terminal fact carries `tree` and `head` and no `settled_members`

#### Scenario: A single seat's attempt

- **WHEN** a single-driver attempt ends indeterminate or failed
- **THEN** its terminal fact carries none of `settled_members`, `tree` and `head`, byte for byte as before this change

#### Scenario: A sequence ended at the chief

- **WHEN** the review sequence's `positions` step aggregates and `chief` then ends the attempt indeterminate
- **THEN** the terminal fact carries none of `settled_members`, `tree` and `head`, the `sequence-step-finished` marker naming `positions` stands in the attempt, and the positions' verdicts stand only inside the aggregate the step checkpointed

#### Scenario: The sequence's checks ended the attempt at the panel step

- **WHEN** every member of the `positions` step settles, one member's result carries no `result` string, and the sequence fails the attempt on the aggregate `__member-schema-invalid__` before the step's marker
- **THEN** its `effect/failed` names the step and carries none of `settled_members`, `tree` and `head`, byte for byte as before this change, and no `sequence-step-finished` marker names `positions`

#### Scenario: An attempt that started on no driver

- **WHEN** a carry is refused at drive time, or a rebuilt input is refused
- **THEN** the attempt's terminal fact carries none of `settled_members`, `tree` and `head`

#### Scenario: An attempt refused admission

- **WHEN** a carrying attempt is refused admission, its `effect/failed` written before any member spawns
- **THEN** that `effect/failed` carries none of `settled_members`, `tree` and `head`, byte for byte as an admission refusal writes it today, and the attempt's `effect/started` keeps its `carried`

#### Scenario: The engine restarted in flight

- **WHEN** the engine restarts while a panel attempt is in flight and settles it indeterminate on the next resume
- **THEN** that `effect/indeterminate` carries none of `settled_members`, `tree` and `head`, byte for byte as before this change

### Requirement: An attempt names every verdict it carries

`effect/started` SHALL carry `carried` when the attempt carries verdicts: one entry per carried member, in the panel's declared order, with `member`, the member's tag, and `attempt_id`, the attempt the verdict settled in. An entry SHALL name the attempt the verdict settled in, never an attempt that only carried it: a verdict read through a carrying attempt's own entry keeps that entry's `attempt_id`. The field SHALL be absent on an attempt that carries nothing. A carried member SHALL appear in no other record or plan of the carrying attempt: not in `provenance`, `boundary`, the `driver` label's count, the session offers, the admission claim, nor as a `panel-member-finished` marker. `carried`, `settled_members`, `tree`, `head` and `args.members` SHALL be published as `contracts/panel-carry.v1.schema.json` under the amended rule of `contracts/README.md`: optional, absent by default, never read by the fold, moving no event type, no seat-record version and no manifest digest, each pinned to the shape its one reader reads, `tree` ends to the four readings, `head` ends to `^[0-9a-f]{40}$`, `settled_members` to an object keyed by tag whose values are any JSON value, each the member's result value as its driver reported it, `carried` to a list of entries with `member` and `attempt_id`, and `args.members` to a non-empty list of tags, each a string; the reader reads any JSON value under a tag of `settled_members`, so the engine never writes a record its own reader refuses; the file is minted under decision 0093 and SHALL land on `main` only once 0093 is accepted (decision 0082 ruling 1).

#### Scenario: Every carried verdict is named in the journal

- **WHEN** a member retry's attempt carries `adversarial`, `security` and `spec-compliance` from the parked attempt
- **THEN** its `effect/started` holds `carried` with those three tags, each naming the parked attempt's `attempt_id`, its `provenance` and `boundary` name `correctness` alone, and the attempt's markers name `correctness` alone

#### Scenario: A second park

- **WHEN** the first member retry names `correctness` and `security`, its attempt carries `adversarial` and `spec-compliance` from the first attempt, `security` settles there and `correctness` again does not, and the operator retries naming `correctness` alone
- **THEN** the third attempt's `carried` names `adversarial` and `spec-compliance` by the first attempt, the one they settled in, and `security` by the second

#### Scenario: A contract written beside the frozen ones

- **WHEN** `contracts/panel-carry.v1.schema.json` is added
- **THEN** every frozen contract's bytes are unchanged, the run manifest's digest is unchanged, a run that carries nothing journals an `effect/started` byte-identical to today's, and the contract's row names 0093 as what minted it

### Requirement: Every readout that paints participants names a carried verdict as carried

`brokkr-view` SHALL derive, from the attempt's `carried`, a participant for each carried member at status `carried` with a completed class, carrying `carried_from`, the attempt its verdict settled in, named by its `attempt_id` and by its ordinal among the phase's attempts in the journal's order, every visit counted, which the painters lead with, and SHALL derive it once; the surfaces that paint participants, `inspect`, `seats`, `watch`, the TUI, `ui.html` and the console, SHALL paint it and SHALL derive nothing of their own. `brokkr runs` and the fleet readouts, whose run rows hold no participant, SHALL show nothing of a carried verdict; of a carry they show only a refused carry's stated park reason, as the next requirement says. A carried member's participant SHALL show no model, session or cost of its own for the carrying attempt: it ran no seat, so it has no session, transcript, cost or provenance of its own, and its `working` SHALL be false. The readers of participants that paint no status SHALL read it as such a participant: `compare`'s resolutions, keyed by label with the last participant of a label in the view's order standing, SHALL skip a carried participant, so no resolution is counted for it and a label keeps the resolution of its last participant that ran; muninn's cost rows count no cost for it as they stand, since they sum only participants with no member, a carried participant is always a member, and a seat's row sums only its members' reported costs, of which a carried member has none; the transcript verb's participant selection is not changed: a carried participant's key is listed among an ambiguous label's keys as every participant's is, and an exact key naming one is refused `no-reference`, as any participant with no recorded transcript is today; and the dashboard's working checkpoints read it as they stand, since they keep only participants at work and a carried participant is not at work and has no checkpoints. The attempt's aggregate notes SHALL stay the fresh panel's. `VIEW_VERSION` SHALL move with the participant's new field.

#### Scenario: The run view after a member retry

- **WHEN** the run view reads a journal where the retry's attempt carried three verdicts and ran one member to a verdict
- **THEN** the three carried members' participants stand at status `carried` naming the parked attempt by its id and its ordinal, with no model of their own and `working` false, the fresh member's at `succeeded`, and the panel's notes read one line per member as a fresh panel's would

#### Scenario: Compare and the transcript verb after a member retry

- **WHEN** `compare` and `brokkr transcript` read that journal
- **THEN** `compare`'s resolution for each carried member's label is the one its source attempt's participant resolved, and `brokkr transcript` given a carried member's label lists every participant's key of that label, the carried participant's among them, as it lists an ambiguous label today, and given the carried participant's key is refused `no-reference`

#### Scenario: A plain retry's view

- **WHEN** the run view reads a journal where the retry's attempt carried nothing
- **THEN** no participant stands at `carried`, and every readout reads as it did before this change

### Requirement: A carry refused at drive time parks as a stated indeterminate

The `effect/indeterminate` that ends a refused carry SHALL carry, as its `reason`, the refusal's own sentence: the typed refusal's text, pinned once in `brokkr-runtime`, the same text `brokkr operator retry --member` refuses with for the same fact, naming the fact that refused: the repository as answering no head; every attempt end whose recorded head is not the head now, with the head it recorded or its absence, in the journal's order with the head now last; the tree, every observation at which it did not read `clean` with its reading, a source attempt's start or end naming that attempt, the parked attempt's start or end, the command, or the drive, in the journal's order with now last; every effect whose input no longer rebuilds to its digest, once each; a record that does not read, naming the attempt and the field, or, for the accepted retry's `args`, a fact only the drive asks, the command by its `operator/commanded` sequence number and the field `args`; or the shape fact or members the verb would have named, a succeeded attempt's park reason read, at the drive as at the command, from the `run/parked` that follows that attempt in the journal. A carrying attempt the panel's span check ended (`panel-member-retry`, fifth requirement) parks the same way: its `effect/indeterminate` carries, as its `reason`, the same typed refusal's sentence, naming the attempt's two ends with their readings and the admitted head, and nothing else of this requirement differs for it. The reason SHALL carry no marker and no JSON. The fold's `close` arm SHALL read it as it reads every unmarked indeterminate reason, to the stated park reason `effect <effect_id> indeterminate: <sentence>`, and the `run/parked` the engine then writes SHALL carry that stated reason with `evidence` `{}`, as a stated park does today; no arm of the fold, no `ParkReason` variant and no codec SHALL be added, and `brokkr-core` SHALL NOT change. Every readout that shows a park reason, `brokkr runs`, `inspect`, `compare`, muninn, the fleet's summary row, the console and the journal timeline's rows, SHALL paint that stated reason as it paints every stated reason today, and derive nothing of its own. The `GATE-MOVED-HEAD` park, its `run/parked` bytes and every other `Stated` reason SHALL read as they do today.

#### Scenario: The park folds through run/parked

- **WHEN** a member retry's drive ended refused because the head moved, and the journal is folded past the `run/parked` the engine wrote
- **THEN** the state's park reason is the stated reason `effect <effect_id> indeterminate: ` followed by the sentence naming the parked attempt's two ends, at the head they recorded, and the head found; `run/parked.reason` carries that text and `run/parked.evidence` is `{}`; and `brokkr inspect`'s `park_reason` is that text

#### Scenario: The readouts after a drive-time refusal

- **WHEN** that journal is read by `brokkr runs`, `inspect`, `compare`, muninn, the fleet readouts and the console
- **THEN** each shows that stated reason as it shows a stated park's, and the journal keeps the sentence as the `effect/indeterminate`'s `reason`

#### Scenario: The timeline after a drive-time refusal

- **WHEN** the console's and the TUI's journal timeline paint that journal
- **THEN** the `run/parked` row and the `effect/indeterminate` row read as every stated park's rows read today, and a `GATE-MOVED-HEAD` row reads as it does today

#### Scenario: The sentence is the verb's

- **WHEN** the head moved before the command and `brokkr operator retry --member correctness` refuses at the terminal
- **THEN** its refusal text is, byte for byte, the sentence a drive-time `effect/indeterminate` carries as its `reason` for the same fact, which the stated park reason carries after `effect <effect_id> indeterminate: `

#### Scenario: Every other park reads as before

- **WHEN** a run parks by `GATE-MOVED-HEAD`, by a rule, or by an attempt's stated indeterminate reason
- **THEN** its park reason, its `run/parked` bytes and every readout of it are byte for byte as before this change
