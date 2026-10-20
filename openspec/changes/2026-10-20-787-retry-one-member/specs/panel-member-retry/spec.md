## ADDED Requirements

### Requirement: A retry names the members that run

`brokkr operator retry` SHALL accept `--member <name>`, repeatable. Each name SHALL resolve against the panel the carry applies to, spelled bare as the panel declares the member or as the tag the journal keys it with, `<step>:<member>` in a sequence, and SHALL be journaled resolved: the retry's `operator/commanded` SHALL carry `args.members`, the resolved tags in the panel's declared order, whatever order the names were given in, spelled as `carried`, `settled_members` and the member markers spell them: they are the fresh members of the plan the one rule returns, which the verb journals as it receives them, resolving nothing itself. The panel's declared order SHALL be the order in which the compiled bundle holds the panel's members, the order every list of members in this capability follows unless it names another. Two names that resolve to one member SHALL be refused as that member given twice, naming the tag. A name that is one member's tag and another member's bare name, `p:x` in a panel step `p` whose members are `x` and `p:x`, tagged `p:x` and `p:p:x`, SHALL be refused as naming two members, naming both tags in the panel's declared order, a fact of its own asked before a name is found unknown or given twice; the engine at drive time matches the journaled tags exactly and never asks it. A retry with members SHALL be disposed of as a retry is today: the fold's rule decides the acceptance, and the acceptance moves the run back to its phase's effect. A retry without `--member` SHALL journal `args: {}` and re-run every member, byte for byte as today. `--member` SHALL belong to `retry` alone and `--repo`, `--bundle`, `--recipe` and `--recipes-dir` to the member form alone, and the verb SHALL ask these two facts first, in this order, before every refusal it makes today: `--member` typed with any command word but `retry`, be it `stop`, `supersede`, `acknowledge` or a word that is no command, SHALL be refused naming `--member` as retry's alone; any of `--repo`, `--bundle`, `--recipe` and `--recipes-dir` typed without `--member` SHALL be refused naming every such argument typed as the member form's alone; both before anything is written. The refusals the verb makes today, `supersede`'s own, the `supersede`-only flags on another command, `acknowledge` and the command word, SHALL follow in their existing order and keep their texts, but for one: on `retry`, with or without `--member`, the supersede-only flags' refusal SHALL name the arguments `retry` now takes, `--findings, --by-run, --by-seq and --by-realm belong to 'supersede'; 'retry' takes --run, --reason, --realms, --db and --member, and with --member also --repo, --bundle, --recipe and --recipes-dir`, and on `stop` and every other command word it SHALL be today's text (S20). `--repo` SHALL name the operated repository the head and the tree are read from and the bundle is compiled for, defaulting as `resume`'s `--repo` defaults: the root of the folder the command runs in, else the current directory, a relative path read against the current directory. `--bundle`, `--recipe` and `--recipes-dir` SHALL find the recipe the verb compiles, each read and defaulted as `resume` reads it: the found folder's bundle when neither `--bundle` nor `--recipe` is typed, and `--recipe` resolved in `--recipes-dir`. The fold SHALL NOT read `args.members`; the engine SHALL read them for the effect the acceptance requested and for every bounded attempt of that effect, matching each tag exactly against the panel and resolving no name a second time. The acceptance SHALL be the latest `operator/accepted` before the effect's `effect/requested` whose accepted command, the `operator/commanded` bearing its `command_id`, is a `retry`, with no other `effect/requested` between that acceptance and the effect's; no such acceptance selects nothing, and the effect runs the whole panel; an `args` that does not read as the verb writes it SHALL refuse the drive as the fourth requirement's records clause refuses it, and SHALL NOT be read as the plain retry. Naming every member of the panel SHALL be admitted exactly when the plain retry is admitted, the pinned bundle compiles under `resume`'s manifest gate, the visit's `panel-carry.v1` records read, and the names resolve against it; a record of the visit that does not read SHALL refuse it, at the command and at the drive, as the fourth requirement refuses any member retry, the records being read before the bundle is compiled and before any name is resolved. When it is admitted, nothing is carried, the parked attempt's shape, the head and the tree are not asked, and the attempt runs every member as a plain retry's does, its `effect/started` carrying no `carried`; this holds wherever the parked attempt ended, a sequence parked at a step after the panel included.

#### Scenario: One stuck reviewer

- **WHEN** a four-member review panel parked because `correctness` ran to no verdict, and the operator runs `brokkr operator retry --member correctness --reason "stuck reviewer"` on an unchanged head and a clean tree
- **THEN** the journal holds an `operator/commanded` with `command: "retry"` and `args: {"members": ["correctness"]}` followed by its `operator/accepted`, and the run folds to `running` at the review phase's effect

#### Scenario: The plain retry

- **WHEN** the operator runs `brokkr operator retry` with no `--member` on the same parked run
- **THEN** `operator/commanded` carries `args: {}`, the next attempt runs every member, and its `effect/started` carries no `carried`

#### Scenario: A member named on another command

- **WHEN** the operator runs `brokkr operator stop --member correctness`
- **THEN** the command is refused naming `--member` as retry's alone, and nothing is written

#### Scenario: A member named beside a supersede-only flag

- **WHEN** the operator runs `brokkr operator stop --member correctness --findings 1`
- **THEN** the command is refused naming `--member` as retry's alone, before the text that refuses `--findings` on a `stop` today, and nothing is written

#### Scenario: A member named on a word that is no command

- **WHEN** the operator runs `brokkr operator bogus --member correctness`
- **THEN** the command is refused naming `--member` as retry's alone, before the text that names the four commands, and nothing is written

#### Scenario: A supersede-only flag on a member retry

- **WHEN** the operator runs `brokkr operator retry --member correctness --findings 1`
- **THEN** the member facts pass, and the command is refused with the text `--findings, --by-run, --by-seq and --by-realm belong to 'supersede'; 'retry' takes --run, --reason, --realms, --db and --member, and with --member also --repo, --bundle, --recipe and --recipes-dir`, and nothing is written

#### Scenario: A supersede-only flag on a plain retry

- **WHEN** the operator runs `brokkr operator retry --findings 1` with no `--member`
- **THEN** the command is refused with the text `--findings, --by-run, --by-seq and --by-realm belong to 'supersede'; 'retry' takes --run, --reason, --realms, --db and --member, and with --member also --repo, --bundle, --recipe and --recipes-dir`, the one text the handler gives on `retry`, and nothing is written

#### Scenario: A supersede-only flag on a stop

- **WHEN** the operator runs `brokkr operator stop --findings 1`
- **THEN** the command is refused with today's text, `--findings, --by-run, --by-seq and --by-realm belong to 'supersede'; 'stop' takes --run, --reason, --realms and --db`, and nothing is written

#### Scenario: A repository named on a plain retry

- **WHEN** the operator runs `brokkr operator retry --repo /work/tree` with no `--member`
- **THEN** the command is refused naming `--repo` as the member form's alone, and nothing is written

#### Scenario: A recipe named on a plain retry

- **WHEN** the operator runs `brokkr operator retry --recipe triage --repo /work/tree` with no `--member`
- **THEN** the command is refused once, naming `--repo` and `--recipe` as the member form's alone, and nothing is written

#### Scenario: The repository the member form reads

- **WHEN** the operator runs `brokkr operator retry --member correctness` without `--repo`, from a subdirectory of a repository that holds a `.brokkr` folder
- **THEN** the head and the tree are read at the folder's root, where `resume` operates without `--repo`, and the bundle compiled is the folder's own, the one `resume` compiles without `--bundle` or `--recipe`

#### Scenario: A name given twice

- **WHEN** the operator runs `brokkr operator retry --member correctness --member correctness`
- **THEN** the command is refused naming `correctness` as given twice, and nothing is written

#### Scenario: A name spelled two ways

- **WHEN** the parked phase is the review sequence whose first step is the panel `positions`, and the operator runs `brokkr operator retry --member correctness --member positions:correctness`
- **THEN** the command is refused naming `positions:correctness` as given twice, and nothing is written

#### Scenario: Names given out of declared order

- **WHEN** a review panel declared `adversarial`, `correctness`, `security`, `spec-compliance` parked with `correctness` and `security` indeterminate and the two others settled, and the operator runs `brokkr operator retry --member security --member correctness` on an unchanged head and a clean tree
- **THEN** `operator/commanded` carries `args: {"members": ["correctness", "security"]}`, the panel's declared order and not the order given, and the next attempt's `carried` names `adversarial` and `spec-compliance`

#### Scenario: A name that is one member's tag and another's bare name

- **WHEN** the parked phase is a synthetic sequence whose first step is the panel `p` with members `x` and `p:x`, tagged `p:x` and `p:p:x`, and the operator runs `brokkr operator retry --member p:x`
- **THEN** the command is refused naming `p:x` as naming two members, by their tags in the panel's declared order, `p:p:x` (the member `p:x`, whose key the compiled bundle holds first) then `p:x` (the member `x`), and nothing is written; `--member x` resolves to `p:x` and `--member p:p:x` to `p:p:x`, each one member

#### Scenario: The journal spells the resolved tag

- **WHEN** the parked phase is that sequence and the operator runs `brokkr operator retry --member correctness`
- **THEN** `operator/commanded` carries `args: {"members": ["positions:correctness"]}`, and the attempt's `carried` entries and its markers key every member `positions:<member>`

#### Scenario: Every member named

- **WHEN** the operator names every member of the panel on a parked attempt that records no verdict to carry, after the head moved and on a dirty tree
- **THEN** the command is admitted, `operator/commanded` carries `args.members` naming every member, every member runs, and the attempt's `effect/started` carries no `carried`

#### Scenario: Every member named beside a record that does not read

- **WHEN** the parked attempt's terminal fact carries a `tree` whose reading is a fifth word, and the operator names every member of the panel
- **THEN** the command is refused naming the attempt and the field `tree` as a record that does not read, the bundle is not compiled, no name is resolved, and nothing is written; and `brokkr resume` on an acceptance naming every member, given before the record was changed, parks the attempt the same way, naming the attempt and the field, on no driver with no member spawned

### Requirement: The carry applies to one panel

The panel SHALL be read from the pinned bundle, which the verb SHALL compile as `brokkr resume` compiles it (`compile_from_manifest`) and SHALL hold to the manifest the journal pins at `run/started` by the comparison `resume` makes, which refuses `ManifestMismatch`. A compile that gate refuses SHALL be refused in `resume`'s own words, the compile's own refusal or `run '<run>' pins a different bundle: <detail>`, a text no carry refusal shares, before any panel fact is asked and with nothing written. The verb SHALL NOT read the panel from the pinned manifest, which records no body for a seat that does not select. The panel SHALL be read from the body the compiled bundle holds for the parked phase, under the case the phase's latest `phase/entered` records when the seat selects: that body when it is a panel, or the sequence's first step when that step is a panel, read from the compiled bundle alone, before any name and whatever the parked attempt's shape; names SHALL resolve against that one panel: a name no member of it has, including a member of another panel step, SHALL be refused naming the panel's members in its declared order. The carry SHALL apply to that panel only when the parked attempt ended at it; the step an attempt ended at SHALL be read as the first step of the sequence with no `sequence-step-finished` marker in that attempt. A panel step after another step SHALL NOT carry: its members read `context.prior_results`, which the carrying attempt builds again and the requested digest does not cover; a sequence whose first step is no panel but a later step is SHALL be refused naming the panel step as following the first step, a fact of its own with its own text, asked after the body is found to run a panel and before any name is read. A sequence parked at a later step SHALL refuse `--member` naming the step the attempt ended at when any member of the panel is unnamed, a fact of the parked attempt's shape asked after the names; when every member is named the command SHALL be admitted as the first requirement admits it, and every step SHALL run again. At drive time the engine SHALL refuse the same facts as a carry that no longer holds is refused: the phase running no panel and the panel step following the first step SHALL be decided by the one rule both observers call, over the body the engine runs for the effect, before any journaled tag is matched. In a sequence the other steps SHALL run again, and the steps after the panel SHALL run on its aggregate.

#### Scenario: A panel that does not select

- **WHEN** a run of `recipes/panel-review`, whose `review` seat is a panel of `correctness` and `security` that selects on nothing, parked at `review` with `security` indeterminate and `correctness` settled, and the operator runs `brokkr operator retry --member security` on the head the parked attempt recorded at both ends and a clean tree
- **THEN** the verb compiles the pinned bundle under `resume`'s manifest gate, resolves `security` against the `review` panel the compiled bundle holds, and admits the command, `operator/commanded` carrying `args: {"members": ["security"]}`

#### Scenario: The pinned bundle drifted

- **WHEN** the recipe the run was started from was edited since, so that its compile no longer reproduces the manifest the journal pins, and the operator runs `brokkr operator retry --member correctness`
- **THEN** the command is refused with the text `brokkr resume` refuses the same run with, `run '<run>' pins a different bundle: ` and the difference, the panel is not read, and nothing is written

#### Scenario: A sequence's first step is the panel

- **WHEN** the parked phase is a sequence of the panel step `positions` and the single step `chief`, parked with `positions:correctness` indeterminate and the three other positions settled, and the operator names `correctness`
- **THEN** `correctness` alone runs under `positions`, the three others are carried under the tags `positions:adversarial`, `positions:security` and `positions:spec-compliance`, `chief` runs again on the step's aggregate, and the attempt journals one `panel-member-finished` marker, for `positions:correctness`, one `sequence-step-finished` marker, for `positions`, no checkpoint whose `member` is a carried member's tag, and `chief`'s result as its `effect/succeeded.result`

#### Scenario: A panel step after another step

- **WHEN** the parked phase is a sequence whose first step is a single seat and whose second step is a panel, parked with one member of that panel indeterminate, and the operator names that member
- **THEN** the command is refused naming the panel step as following the first step, whose result its members read, the name is not resolved, and nothing is written

#### Scenario: The panel facts at the drive

- **WHEN** an accepted retry's `args.members` meet, at the drive, a body that runs no panel, or a sequence whose panel step follows its first step
- **THEN** the attempt's `effect/started` names `driver: "none"` and no `carried`, no member spawns, and its `effect/indeterminate` reason names the phase as running no panel, or the panel step as following the first step, the sentence the verb refuses the same fact with

#### Scenario: A sequence parked at the chief

- **WHEN** the review sequence parked because `chief` ran to no verdict after `positions` aggregated, its `sequence-step-finished` marker naming `positions`, and the operator names a member of `positions`
- **THEN** the command is refused naming `chief` as the step the attempt ended at, nothing is written, and the plain retry is admitted

#### Scenario: Every member named on a run parked at a later step

- **WHEN** the review sequence parked because `chief` ran to no verdict after `positions` aggregated, and the operator runs `brokkr operator retry --member adversarial --member correctness --member security --member spec-compliance`
- **THEN** the command is admitted, `operator/commanded` carries `args.members` naming the four `positions:` tags in the panel's declared order, the step the attempt ended at is not asked, the next attempt runs every member and then `chief`, and its `effect/started` carries no `carried`

#### Scenario: A name two panel steps share

- **WHEN** the parked phase is a sequence of two panel steps that both declare a member `security`, parked at the first, and the operator names `security`
- **THEN** the name resolves to the first step's `security` and nothing else, and the carry proceeds for the first step's panel

### Requirement: A verdict is carried only when nothing it judged has changed

A member's settled verdict SHALL be carried into the retry's attempt only when four facts hold: the reviewed head, the working tree, the pinned bundle and the member's inputs. The reviewed head SHALL be the head recorded at both ends of the parked attempt and of every attempt a carried verdict is read from, as their terminal facts' `head` record holds it (`panel-attempt-record`), each end compared as a string with the head now of the repository `--repo` names, else the repository `resume` would operate, read by `git_head` when the command is given and when the attempt is driven; the phase's `phase/entered.head` SHALL NOT be read for it. A repository whose head cannot be read, or whose answer is not a commit id, SHALL refuse the carry naming the repository as answering no head, no recorded end SHALL be compared, and the tree SHALL NOT be asked after it. So a repository whose head stays outside the commit-id shape, a repository of SHA-256 object format among them, whose attempts recorded no head for that reason, SHALL be refused as the repository answering no head, and never naming the ends; an end that recorded no head SHALL be named only when the head now is in the commit-id shape. Every recorded end SHALL equal the head now: an end that recorded no head is unequal, and a recorded end is compared as a string and never handed to git; a recorded end present outside the commit-id shape is a record that does not read, which the next requirement refuses by name before any identity fact and never reads as no head; every unequal end SHALL be named, with the head it recorded or its absence, in one refusal, in the journal's order: each attempt a verdict is read from or through at its start and then its end, the parked attempt last among them, and the head now after them all. The working tree SHALL read `clean` at the two ends of the parked attempt and of every attempt a carried verdict is read from, as their terminal facts' `tree` record, and `clean` when the command is given and when the attempt is driven, every one of those readings taken through the one reader of `working-tree-reading`; any other reading, `dirty`, `refused` or `unrun`, at any of those observations SHALL refuse the carry, every observation that did not read `clean` named with its reading in one refusal, in the journal's order: each attempt a verdict is read from or through at its start and then its end, the parked attempt last among them, and the observation now after them all. The files the repository ignores are not read: an edit to one is the carry's accepted residual, as it is the ship gate's. The pinned bundle SHALL be held by the manifest gate `resume` applies before it builds any input, the gate the verb applies too before it reads the panel. The member's inputs SHALL be held by rebuilding the seat input for the source attempt's effect and comparing its digest with the `input_digest` that effect's `effect/requested` recorded; every source effect whose rebuilt input no longer digests to its record SHALL be named once in one refusal, in the order the carried entries first name it. The verb SHALL check the head, the tree and the journal's facts when the command is given and SHALL refuse with the reason, writing nothing. The engine SHALL check all four before anything of the attempt spawns, and a carry that no longer holds SHALL start the attempt on no driver and end it `effect/indeterminate` whose `reason` is the refusal's own sentence, so the run parks for the operator as a stated indeterminate parks today (`panel-attempt-record`). The refusal SHALL be one typed vocabulary, a `thiserror` enum in `brokkr-runtime` with one variant and one text per fact, spoken by the verb at the terminal and written by the engine as that reason, the same bytes.

#### Scenario: The head moved before the command

- **WHEN** a commit lands on the reviewed branch after the panel parked, and the operator runs `brokkr operator retry --member correctness`
- **THEN** the command is refused naming the parked attempt's start and end, at the head they recorded, and the head found now, the journal holds no `operator/commanded` for it, and `brokkr operator retry` without members is admitted and re-runs the whole panel

#### Scenario: The head moved between the acceptance and the drive

- **WHEN** the member retry was accepted on an unchanged head, a commit lands, and `brokkr resume` drives the run
- **THEN** the attempt's `effect/started` names `driver: "none"` and no `carried`, its `effect/indeterminate` reason is the sentence naming the parked attempt's start and end, at the head they recorded, and the head found, the run is `awaiting_operator` with that sentence as its stated park reason, and no member spawned

#### Scenario: The head moved during a park and was put back

- **WHEN** the review phase was entered at `H0`, its first attempt parked with `correctness` indeterminate and recorded `H0` at both ends, a commit then moved the head to `H1`, a plain retry's attempt judged `H1`, `correctness` ran to no verdict again while the three others settled, and that attempt recorded `H1` at both ends and a clean tree, the operator resets the branch to `H0`, and runs `brokkr operator retry --member correctness` on a clean tree
- **THEN** the plain retry's attempt is read as the parked attempt, the command is refused once naming that attempt's start and end as having recorded `H1` and the head now as `H0`, the first attempt's verdicts are not carried, and nothing is written

#### Scenario: A work member committed inside the attempt

- **WHEN** a panel whose members are not all gates, so that no gate-head observation watches its effect, parked with one member indeterminate after a work member committed, so that its terminal fact records `head.at_start` as `H0` and `head.at_end` as `H1`, and the operator names the indeterminate member on `H1` with a clean tree
- **THEN** the command is refused once naming that attempt's start as having recorded `H0` and the head now as `H1`, and nothing is written

#### Scenario: The tree is dirty at the command

- **WHEN** the head is unchanged, a file under the operated repository is edited and not committed, and the operator runs `brokkr operator retry --member correctness`
- **THEN** the command is refused naming the working tree as `dirty` at the command, and nothing is written

#### Scenario: The tree was dirty while the panel judged

- **WHEN** the parked attempt's terminal fact records `tree.at_end` as `dirty`, the edits were reverted after the park, and the operator runs `brokkr operator retry --member correctness` on a clean tree
- **THEN** the command is refused naming the tree as `dirty` when the parked attempt ended, and nothing is written

#### Scenario: The tree at a carrying attempt's end

- **WHEN** the parked attempt is itself a carrying attempt whose terminal fact records `tree.at_end` as `dirty`, while the attempt its entries name records `tree` clean at both ends
- **THEN** the command is refused naming the tree as `dirty` when the parked attempt ended, and nothing is written

#### Scenario: Two observations fail

- **WHEN** a file under the operated repository was edited while the panel judged and is still uncommitted, so the parked attempt's terminal fact records `tree.at_end` as `dirty` and the tree reads `dirty` now, and the operator runs `brokkr operator retry --member correctness`
- **THEN** the command is refused once, naming the tree as `dirty` when the parked attempt ended and `dirty` at the command, in that order, and nothing is written

#### Scenario: Git answers the head and refuses status

- **WHEN** the head is unchanged and recorded, the operated repository's index file is made unreadable so that `git rev-parse HEAD` answers and `git status --porcelain` exits 128 printing nothing, and the operator runs `brokkr operator retry --member correctness`
- **THEN** the command is refused naming the working tree as `refused` at the command, not the head, and nothing is written

#### Scenario: Git refused status while the panel judged

- **WHEN** the parked attempt's terminal fact records `tree.at_start` as `refused`, and the operator runs `brokkr operator retry --member correctness` on a clean tree and an unchanged head
- **THEN** the command is refused naming the tree as `refused` when the parked attempt started, and nothing is written

#### Scenario: An ignored file edited after the park

- **WHEN** the head is unchanged, a file the repository's `.gitignore` names is edited after the park, and the operator runs `brokkr operator retry --member correctness`
- **THEN** the command is admitted: the reader reads `clean`, and the edit is the accepted residual the proposal names

#### Scenario: A member's inputs changed

- **WHEN** a file the seat input embeds, such as the realm's house rules, is edited after the park, the head and the tree are as they were, and `brokkr resume` drives an accepted member retry
- **THEN** the attempt ends `effect/indeterminate` whose reason names the source attempt's effect as the one whose input no longer rebuilds to its recorded digest, and the run parks

#### Scenario: Two source effects' inputs changed

- **WHEN** the parked attempt carried `adversarial` and `spec-compliance` from the first attempt and settled `security` itself, the realm's house rules are edited after the park, and `brokkr resume` drives an accepted retry naming `correctness`
- **THEN** the attempt ends `effect/indeterminate` whose reason names the first attempt's effect and then the parked attempt's effect, each once, as the effects whose inputs no longer rebuild to their digests

#### Scenario: Nothing changed

- **WHEN** the head, the tree, the bundle and every input are as the panel judged them, and `brokkr resume` drives the accepted member retry
- **THEN** the attempt's `effect/started` names every carried verdict, and only the named members spawn

#### Scenario: A panel in a phase that records no entry head

- **WHEN** the parked phase's latest `phase/entered` recorded no head, as the entry of a phase other than the protected one may, its parked attempt recorded the head at both ends, the head now is that head, and the tree is clean
- **THEN** the head fact holds, and the phase's entry is not asked

#### Scenario: An attempt that recorded no head

- **WHEN** the operated repository had no commit while the panel judged, so the parked attempt's terminal fact records `tree` and no `head`, a commit has since been made, and the operator runs `brokkr operator retry --member correctness`
- **THEN** `--member` is refused once naming the parked attempt's start and end as having recorded no head and the head now, and nothing is written

#### Scenario: A recorded head that is not a commit id

- **WHEN** the parked attempt's terminal fact records a `head.at_end` that reads as anything but forty lowercase hex digits, the shape `contracts/panel-carry.v1.schema.json` pins on each end
- **THEN** `--member` is refused naming the parked attempt and the field `head` as a record that does not read, before the attempt's shape and before any identity fact is asked, the recorded value is never handed to git, nothing is written, and `brokkr resume` on an acceptance given before the record was changed parks the attempt the same way

#### Scenario: A repository whose head cannot be read

- **WHEN** `--repo` names a directory that is not a git repository
- **THEN** `--member` is refused naming the repository as answering no head, the tree is not asked, and nothing is written

### Requirement: What is not carried refuses by name, in one order, before anything is written

With `--member`, the verb SHALL fold the run before it writes and SHALL ask its checks in this order, each fact with one text, the first fact that fails being the refusal reported, and a refused command writing nothing:

1. the arguments' shape, two facts in order: `--member` with a command word other than `retry`; any of `--repo`, `--bundle`, `--recipe` and `--recipes-dir` without `--member`; then the refusals the verb makes today, in their existing order and with their texts, except that the supersede-only flags' refusal on `retry` names the arguments `retry` now takes;
2. the run's state by the fold's one rule (`acceptance_refusal`: a concluded run, `after_terminal`; not parked, `run_not_awaiting_operator`; no phase to retry, `no_phase_to_retry`; in that order), reported by the verb through `brokkr_view::member_retry_refusal`, which takes the fold's word alone, `refused operator retry --member (<word>): the run is not in a state a retry can apply to; nothing was journaled`, which says nothing was journaled (S21);
3. the bundle and the panel, three facts in order: the pinned bundle compiles under `resume`'s manifest gate, else the command SHALL be refused in `resume`'s own words, as the previous requirement reads it; the parked phase's body, read from the compiled bundle as the previous requirement reads it, is a panel or a sequence with a panel step, else the command SHALL be refused naming the phase as running no panel; the panel is the body or the sequence's first step, else the command SHALL be refused naming the panel step as following the first step;
4. the names, three facts in this order, each asked over the names in the order given and refused at the first name that fails it: a name that is one member's tag and another member's bare name SHALL be refused naming both tags in the panel's declared order; each SHALL resolve to a member of that panel, else the command SHALL be refused naming the panel's members in its declared order; no member SHALL be given twice, else the command SHALL be refused naming the tag; when every member is named the command SHALL be admitted here and nothing below asked, a sequence parked at a step after the panel included;
5. the parked attempt's shape, four facts in this order, each refused naming its own fact: the phase's current visit holds no attempt its panel ended, naming the phase and its visit; the attempt's terminal fact is `effect/succeeded`, naming the park's reason as the folded state holds it; it ended at a step after the panel, naming that step; it records no verdict to carry, neither `settled_members` on its terminal fact nor `carried` on its `effect/started`, naming the attempt;
6. the unnamed members: every member not named SHALL have a verdict to carry, settled in the parked attempt or carried by it from an attempt whose `settled_members` holds it, else the command SHALL be refused naming every such member in the panel's declared order, in one refusal;
7. the head, as the previous requirement reads it;
8. the tree, as the previous requirement reads it.

Before any fact of items 3 to 8 is asked, after item 2 and before the pinned bundle is compiled, whatever names are given, every member named included, the journal's records of the parked phase's visit SHALL be read once into typed records: a `carried`, `settled_members`, `tree` or `head` present on an attempt that does not read as `contracts/panel-carry.v1.schema.json` publishes it SHALL refuse the command naming the attempt and the field, with nothing written, and SHALL refuse the drive the same way, as a carry that no longer holds is refused; a malformed record is never read as an absent one. The schema pins each field's shape and the reader reads exactly that shape: each `tree` end one of the four readings; each `head` end forty lowercase hex digits, `^[0-9a-f]{40}$`, as `phase-entered-head.v1` pins it; `settled_members` an object keyed by tag whose value under each tag is any JSON value, the member's result value as its driver reported it, an object or not, since adapters repair nothing (decision 0001); `carried` a list of entries each with `member` and `attempt_id`; `settled_members` or `head` on a terminal fact that carries no `tree` does not read either. So a `head` end present outside the shape is a record that does not read, refused here, and only an absent end is a head that recorded none, refused at item 7; and the reader reads any JSON value under a tag of `settled_members`, so the engine never writes a record its own reader refuses, and a carried value that is no object poisons the aggregate exactly as the same value in a fresh panel would. At the drive the accepted retry's `operator/commanded.args` SHALL be read first, before the visit's records: an absent `args` or `{}` is the plain retry, and `args` that do not read as the verb writes them, a key other than `members`, a `members` that is not a list of strings, or an empty `members` list, which the verb never writes since the plain retry journals `{}`, SHALL refuse the drive as a record that does not read, naming the command by its `operator/commanded` sequence number and the field `args`, as a carry that no longer holds is refused, and SHALL NOT be read as the plain retry. On a run whose record does not read and whose bundle has also drifted, the record SHALL be the refusal reported and the bundle SHALL NOT be compiled. The observations the facts need, the records, the head now and the tree now, are taken before the rule decides, observing apart from deciding; the order of the items is the order the rule asks its facts and reports the first that fails, not the order the world is read. The parked attempt SHALL be the latest attempt of the parked phase's current visit, before the retry's own effect, that its panel ended, and a three-way test alone SHALL decide it: its terminal fact is `effect/succeeded`; or its terminal fact is an `effect/indeterminate` or `effect/failed` that records `tree`, which only the panel writes; or, in a sequence, the attempt journals the panel step's `sequence-step-finished` marker. An attempt that bears any of the three marks SHALL NOT be passed over, whatever ended it after its panel: a sequence's attempt whose panel step's marker stands and which a later step, a restart in flight, journal contention, a held outcome the fence refused or a gate moving the head then ended, or which was journaled before this change, is the parked attempt when it is the latest so marked, and the shape fact of item 5 SHALL refuse it naming the step it ended at, the attempt before it unread. Only an attempt that bears none of the three marks SHALL be passed over: one that started on no driver; one refused admission or provisioning before any member spawned; one the engine ended for a restart in flight, for journal contention or for a held outcome the fence refused before its panel ended; one a gate ended by moving the head before its panel ended, its `GATE-MOVED-HEAD` written in the place of the panel's terminal fact or before the panel step's marker; one a sequence's own checks on the panel step's aggregate ended before the step's marker, its terminal fact the sequence's; and one journaled before this change that neither succeeded nor journaled the panel step's marker. At the command the parked attempt is read among every attempt of the visit; at drive time it is read among the attempts before the effect the acceptance requested, so every bounded attempt of that effect reads the one its first attempt read. Once the command is written only the fold's race refusal, a lost fence, SHALL follow, journaled as today. The engine SHALL refuse the same facts at drive time, in the same order with the inputs last, as the previous requirement refuses a carry that no longer holds: item 3's two panel facts are decided there by the same rule over the body the engine runs, the compile excepted, which `resume` asked before the engine was built; and item 4 is asked of the journaled tags, matched exactly, so its first fact cannot fail there.

#### Scenario: A member retry on a run that is not parked

- **WHEN** the operator runs `brokkr operator retry --member correctness` on a running run
- **THEN** the command is refused with the text `refused operator retry --member (run_not_awaiting_operator): the run is not in a state a retry can apply to; nothing was journaled`, and the journal holds no `operator/commanded` for it, where the plain retry would journal its command and its `operator/rejected` and print `operator_refusal`'s text, which is unchanged

#### Scenario: A member retry on a concluded run

- **WHEN** the operator runs `brokkr operator retry --member correctness` on a run that completed or was stopped
- **THEN** the command is refused with the text `refused operator retry --member (after_terminal): the run is not in a state a retry can apply to; nothing was journaled`, and nothing is written

#### Scenario: The parked phase runs no panel

- **WHEN** the run parked at a single-driver seat, or at a sequence of single steps, and the operator runs `brokkr operator retry --member correctness`
- **THEN** the command is refused naming the phase as running no panel, nothing is written, and the plain retry is admitted

#### Scenario: A name the panel does not have

- **WHEN** the operator names `chief`, the review sequence's single step, or a name no member has
- **THEN** the command is refused naming the panel's members, and the parked attempt's shape is not asked

#### Scenario: No attempt its panel ended

- **WHEN** the parked phase's current visit holds no attempt its panel ended: its attempts started on no driver, were refused admission before any member spawned, or were journaled before this change and neither succeeded nor journaled the panel step's marker
- **THEN** `--member` is refused naming the phase's visit as holding no attempt its panel ended, and nothing is written

#### Scenario: The shape facts in order

- **WHEN** the parked attempt's panel succeeded and a rule parked the run, and the operator names a member
- **THEN** the command is refused naming the attempt as succeeded and the park's reason as the folded state holds it, the step and the record are not asked, nothing is written, and the plain retry is admitted

#### Scenario: The sequence's checks ended the attempt at the panel step

- **WHEN** every member of the review sequence's `positions` step settled, `correctness`'s result carried no `result` string, so the step's aggregate was `__member-schema-invalid__` and the sequence failed the attempt before the step's marker, the effect's bounded attempts ended the same way and the run parked, and the operator runs `brokkr operator retry --member correctness`
- **THEN** every such attempt is passed over; with no earlier attempt of the visit its panel ended, the command is refused naming the phase's visit as holding no attempt its panel ended, nothing is written, and the plain retry is admitted

#### Scenario: A seat's panel with the same member

- **WHEN** the same member in a seat's panel body carries no `result` string, so the attempt's `effect/succeeded` carries `__member-schema-invalid__` and `decide` parks the run by decision 0001's result check, and the operator names that member
- **THEN** the command is refused naming the attempt as succeeded and the park's reason as the folded state holds it, and the plain retry is admitted

#### Scenario: A gate moved the head in the carrying attempt

- **WHEN** a member retry's attempt carried three verdicts and the retried member, a gate, committed, so the attempt ended with the `GATE-MOVED-HEAD` indeterminate, and the operator runs `brokkr operator retry --member correctness` on the moved head
- **THEN** the gate's attempt is passed over, the attempt before it that its panel ended is read, and the command is refused naming that attempt's start and end, at the head they recorded, and the head found now

#### Scenario: The head is put back after a gate moved it

- **WHEN** the operator resets the branch, after that park, to the head the attempt before the gate's recorded at both ends, the tree is clean, and runs `brokkr operator retry --member correctness`
- **THEN** the gate's attempt is passed over, the command is admitted, and the next attempt's `carried` names the attempt before the gate's

#### Scenario: A carrying attempt that ended at the chief after the positions aggregated

- **WHEN** the review sequence's first attempt parked with `positions:correctness` indeterminate, a member retry's attempt carried the three other positions from it and ran `correctness` to a verdict, the `positions` step aggregated and its `sequence-step-finished` marker stands, and `chief` then ended that attempt, by committing so that it ended with the `GATE-MOVED-HEAD` indeterminate, or in flight when the engine restarted, and the operator runs `brokkr operator retry --member correctness`
- **THEN** the carrying attempt bears the panel step's marker, so it is read as the parked attempt and is not passed over; the command is refused naming `chief` as the step the attempt ended at, the first attempt is not read and no verdict is carried from it, nothing is written, and the plain retry is admitted

#### Scenario: A carrying attempt refused admission

- **WHEN** a member retry's attempt journaled `carried` on its `effect/started` and was then refused admission, its `effect/failed` written before any member spawned, the effect's bounded attempts were refused the same way and the run parked, and the operator runs `brokkr operator retry --member correctness` again once the refusal is put right
- **THEN** every refused attempt is passed over, the attempt before them that its panel ended is read, the command is admitted, and the next attempt's `carried` names that attempt, the one the refused attempts' entries also named

#### Scenario: The bounded attempt after a refused admission

- **WHEN** a carrying effect's first attempt is refused admission under a seat whose `max_attempts` is 2
- **THEN** the effect's second attempt reads the parked attempt among the attempts before the effect's request, carries the same entries from the same source attempts, and runs the named members again

#### Scenario: The engine restarted during the carrying attempt

- **WHEN** a member retry's attempt was in flight, before its panel ended, when the engine restarted, so its terminal fact is the restart's `effect/indeterminate` with no `tree` and it journals no panel step's marker, and the operator runs `brokkr operator retry --member correctness` on an unchanged head and a clean tree
- **THEN** the restarted attempt is passed over, the command is admitted, and the next attempt's `carried` names the attempt the restarted one carried from

#### Scenario: A journal written before this change

- **WHEN** the parked phase's only attempt is a panel attempt written before this change, whose terminal fact is no `effect/succeeded` and carries no `tree`, and which journals no panel step's marker
- **THEN** `--member` is refused naming the phase's visit as holding no attempt its panel ended, and nothing is written

#### Scenario: A record that does not read

- **WHEN** the parked attempt's terminal fact carries a `settled_members` that is not an object, a `tree` whose reading is a fifth word, or a `head` end outside the commit-id shape, and the operator names a member
- **THEN** `--member` is refused naming the attempt and the field that does not read, before the bundle is compiled and before the attempt's shape is asked, nothing is written, and `brokkr resume` on an acceptance given before the record was changed parks the attempt the same way, naming the attempt and the field

#### Scenario: A record that does not read beside a drifted bundle

- **WHEN** the recipe the run was started from was edited since, so that its compile would not reproduce the pinned manifest, the parked attempt's terminal fact carries a `tree` whose reading is a fifth word, and the operator names a member
- **THEN** the command is refused naming the attempt and the field `tree`, in the record's text and not `resume`'s, the bundle is not compiled, and nothing is written

#### Scenario: Retry arguments that do not read

- **WHEN** the accepted retry's `operator/commanded` carries `args` with a key other than `members`, a `members` that is not a list of strings, or `{"members": []}`, and `brokkr resume` drives the effect the acceptance requested
- **THEN** the attempt's `effect/started` names `driver: "none"` and no `carried`, no member spawns, its `effect/indeterminate` reason names the command by its `operator/commanded` sequence number and the field `args` as a record that does not read, the run is `awaiting_operator` with that sentence as its stated park reason, and the attempt is not run as the plain retry; an accepted retry whose `args` is `{}` or absent runs every member as today

#### Scenario: An all-indeterminate panel

- **WHEN** every member of the parked attempt ran to no verdict, so its terminal fact carries `tree` and no `settled_members`, and the operator names one member
- **THEN** `--member` is refused naming the parked attempt as recording no verdict to carry, and nothing is written

#### Scenario: An unnamed member that never settled

- **WHEN** `correctness` and `security` both ran to no verdict and the operator names only `correctness`
- **THEN** the command is refused naming `security` as having no verdict to carry, and nothing is written

#### Scenario: Several unnamed members never settled

- **WHEN** `adversarial`, `correctness` and `security` all ran to no verdict, `spec-compliance` settled, and the operator names `correctness`
- **THEN** the command is refused once, naming `adversarial` and `security` in the panel's declared order as having no verdict to carry, and nothing is written

#### Scenario: The same member stuck twice

- **WHEN** the first attempt parked with `correctness` indeterminate, the member retry's attempt carried the three others from it and `correctness` ran to no verdict again, and the operator runs `brokkr operator retry --member correctness` on an unchanged head and a clean tree
- **THEN** the carrying attempt is read as the parked attempt, the three verdicts are found through its `carried` entries, the head and the tree are checked at the carrying attempt's two ends and at the first attempt's, the command is admitted, and the third attempt's `carried` names the three by the first attempt

#### Scenario: A refused carry is not forfeited

- **WHEN** a member retry's drive ended refused because the tree was dirty, the operator cleans the tree, and runs `brokkr operator retry --member correctness` again
- **THEN** the refusing attempt is passed over, the carry reads the attempt before it, the command is admitted, and the next attempt's `carried` names that earlier attempt

#### Scenario: Several refusals hold

- **WHEN** the operator names a member the panel does not have on a run whose head has also moved
- **THEN** the command is refused for the name alone, and the head fact is not asked

### Requirement: The carrying attempt runs the named members and aggregates as a fresh panel

In the retry's attempt only the named members SHALL run; every other member's verdict SHALL be carried from the attempt it settled in, read from the parked attempt alone: settled there, its bytes in the parked attempt's `settled_members`, or named by that attempt's own `carried` entry, its bytes in the named attempt's `settled_members`. The declared aggregate SHALL join carried and fresh verdicts exactly as it joins a fresh panel's, over the members' bare names, so the attempt's result, its `inputs` and its `notes` SHALL equal the bytes a fresh panel aggregates over the same verdicts, and policy SHALL route them unchanged. Every per-site record and plan of the carrying attempt SHALL cover the members that run and no other: `effect/started.provenance`, `effect/started.boundary`, the `driver` label, the session offers, the admission claim and `panel-member-finished`. A fresh member that is indeterminate SHALL park the attempt and one that fails SHALL fail it; the bounded retry of that effect SHALL read the parked attempt among the attempts before the effect's request, SHALL run the named members again with the same carried entries from the same source attempts, and SHALL NOT carry a verdict that settled in an earlier bounded attempt of the same effect. The carry SHALL be the same inside one effect as a fresh panel's (decision 0002).

#### Scenario: The same aggregate as a full re-run

- **WHEN** a review panel of `adversarial`, `correctness`, `security` and `spec-compliance` parked with `correctness` indeterminate and the three others settled, and the member retry runs `correctness` to a verdict
- **THEN** the attempt's `effect/succeeded.result` equals, byte for byte, the result a fresh panel aggregates over the same four verdicts: the worst verdict, the joined `inputs`, and `notes` holding every member's notes and verdict

#### Scenario: A residual from the retried member routes as today

- **WHEN** the retried member reports `residual` naming a security residual at severity `low`, the carried verdicts are `clean`, and the run's table is decision 0022's reforging table, whose `REVIEW-REFORGE` sends a security residual back to `implement`
- **THEN** the attempt's result is `residual` with `has_security_residual` `true` and `max_residual_severity` `low`, the bytes a fresh panel aggregates over the same verdicts, `REVIEW-REFORGE` routes the run to `implement` and then `verify` as today, and the review the run re-enters has no acceptance before its effect's request, so its attempt runs every member fresh and its `effect/started` carries no `carried`

#### Scenario: The fresh member fails and the effect retries

- **WHEN** the named member's driver fails to start in the retry's first attempt, under a seat whose `max_attempts` is 2
- **THEN** that attempt's `effect/failed` records no settled member, its `effect/started` having named the carried verdicts, and the effect's second attempt runs the named member again with `carried` entries naming the same source attempts

#### Scenario: A member that settled in the first bounded attempt is not carried into the second

- **WHEN** the member retry names `correctness` and `security`, `security` settles in the effect's first attempt and `correctness`'s driver fails, under a seat whose `max_attempts` is 2
- **THEN** the effect's second attempt runs `correctness` and `security` again, its `carried` naming the two others alone, by the attempts the first attempt named

#### Scenario: Only the members that run are recorded

- **WHEN** the member retry's attempt carries three verdicts and runs `correctness`
- **THEN** its `effect/started.provenance` and `boundary` hold `correctness` alone, its `driver` label counts one member, one session is offered, admission claims one site, and one `panel-member-finished` is journaled
