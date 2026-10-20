## ADDED Requirements

### Requirement: An attempt that ends short of success records each settled verdict

A panel attempt whose terminal fact is `effect/indeterminate` or `effect/failed` SHALL record `settled_members`: for each member whose outcome is a settled success, keyed by the member's tag as `unresolved_members` and the marker key it, the member's result object as the seat-record fence admitted it. The field SHALL be absent when no member settled and on every attempt that ran no panel, so those payloads keep their bytes. `effect/succeeded` SHALL NOT change. The fold SHALL NOT read the field.

#### Scenario: Three settled, one not

- **WHEN** a four-member panel ends indeterminate because one member ran to no verdict
- **THEN** its `effect/indeterminate` carries `settled_members` with the three members' results under their tags, and `unresolved_members` as #403 records it when any member is not proven over

#### Scenario: A panel that failed by one member

- **WHEN** a panel attempt fails because one member's driver failed to start and the others settled
- **THEN** its `effect/failed` carries `settled_members` with the settled members' results, beside `error` and the start-failure fields it carries today

#### Scenario: A single seat's attempt

- **WHEN** a single-driver attempt ends indeterminate or failed
- **THEN** its terminal fact carries no `settled_members`, byte for byte as before this change

### Requirement: An attempt names every verdict it carries

`effect/started` SHALL carry `carried` when the attempt carries verdicts: one entry per carried member, in the panel's declared order, with `member`, the member's tag, and `attempt_id`, the attempt the verdict settled in. An entry SHALL name the attempt the verdict settled in, never a carrying attempt. The field SHALL be absent on an attempt that carries nothing. `carried`, `settled_members` and `args.members` SHALL be published as `contracts/panel-carry.v1.schema.json` under the amended rule of `contracts/README.md`: optional, absent by default, never read by the fold, moving no event type, no seat-record version and no manifest digest. A carried member SHALL write no `panel-member-finished` marker in the carrying attempt.

#### Scenario: Every carried verdict is named in the journal

- **WHEN** a member retry's attempt carries `adversarial`, `security` and `spec-compliance` from the parked attempt
- **THEN** its `effect/started` holds `carried` with those three tags, each naming the parked attempt's `attempt_id`, and the attempt's markers name `correctness` alone

#### Scenario: A second park

- **WHEN** the member retry's attempt also ends short of success and the operator retries that member again
- **THEN** the third attempt's `carried` names each verdict carried through the second attempt by the attempt it settled in, the first, and a member that settled in the second attempt by the second

#### Scenario: A contract written beside the frozen ones

- **WHEN** `contracts/panel-carry.v1.schema.json` is added
- **THEN** every frozen contract's bytes are unchanged, the run manifest's digest is unchanged, and a run that carries nothing journals the payloads it journals today

### Requirement: Every readout names a carried verdict as carried

`brokkr-view` SHALL derive, from the attempt's `carried`, a participant for each carried member at status `carried` with a completed class, carrying `carried_from`, the attempt its verdict settled in, and SHALL derive it once; `brokkr runs`, `inspect`, `seats`, the TUI, `ui.html` and the console SHALL paint it and SHALL derive nothing of their own. The attempt's aggregate notes SHALL stay the fresh panel's. `VIEW_VERSION` SHALL move with the participant's new field.

#### Scenario: The run view after a member retry

- **WHEN** the run view reads a journal where the retry's attempt carried three verdicts and ran one member to a verdict
- **THEN** the three carried members' participants stand at status `carried` naming the parked attempt, the fresh member's at `succeeded`, and the panel's notes read one line per member as a fresh panel's would

#### Scenario: A plain retry's view

- **WHEN** the run view reads a journal where the retry's attempt carried nothing
- **THEN** no participant stands at `carried`, and every readout reads as it did before this change
