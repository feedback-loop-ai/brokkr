# operator-console-read Specification

## Purpose
Give the operator console a read of every realm's runs, the queue and the
run view as Brokkr's readouts derive them, led by human titles and
refreshed without the console deriving meaning of its own.

## Requirements

### Requirement: The console reads Brokkr only through its readouts

The DSH host plugin SHALL obtain every run, fleet, queue, Muninn, analytics
and transcript fact from Brokkr's own readouts. It SHALL NOT open a journal
or derive a standing, need, admissibility, reason or metric itself.

#### Scenario: A parked run shows Brokkr's own reason

- **WHEN** a run is parked by a rule
- **THEN** the decision card shows the park reason as the readout gives it, word for word, with the rule id
- **AND** no text on the card is computed by the client from raw events

### Requirement: The Brokkr entry and panel

The client SHALL register a **Brokkr** entry in DSH's panel list. The entry's
icon SHALL show the number of runs waiting on the operator. Selecting it
SHALL open a panel with two views: Realms and Analytics. Runs SHALL be
labelled by title. Run ids SHALL appear only under a collapsed "Details for
audit".

#### Scenario: Waiting count

- **WHEN** four runs across all realms need the operator
- **THEN** the Brokkr entry's icon shows 4 and the Realms header reads "4 waiting on you"

### Requirement: Every hearth in realms.json is shown

The Realms view SHALL show one card per realm in every hearth `realms.json`
names. Each card SHALL show:

- each run that needs the operator, with its title, step and Brokkr's
  short reason;
- the count of running runs;
- the counts shipped and stopped in the last 24 hours.

A journal that does not load SHALL show its realm as quarantined with the
refusal.

#### Scenario: A quiet run

- **WHEN** a lanetally run has recorded nothing for longer than the staleness bound
- **THEN** its card shows "no activity for 3h · last step: specify" under the realm

### Requirement: The queue shows admission, not dispatch

The Realms view SHALL list queued entries with their route and Brokkr's
admission verdict:

- could start, with the route's room;
- waiting, with its reason;
- held, with its waits and who held it.

It SHALL show no control that starts a queued run. It SHALL show the note
that starting comes with Brokkr's dispatcher.

#### Scenario: Route full

- **WHEN** an entry's route is at its host ceiling
- **THEN** it shows "waiting · route full: 3 of 3 runs on this machine"

### Requirement: The run view

Selecting a run SHALL show:

- the run list grouped needs you / running / last 24 hours, with a text
  filter;
- the run header with Brokkr's standing;
- the graph of steps with their members, the model that served each, visits
  and the return paths taken;
- the decision card when the run needs the operator;
- the timeline, newest first.

The view SHALL use only Brokkr's standings and SHALL NOT show a
queued/started/verified state machine.

#### Scenario: A review sent work back

- **WHEN** security review once ruled the run back to implement
- **THEN** the graph shows the return path with the ruling's time and rule

### Requirement: Muninn's last review is readable

The Realms view SHALL show when Muninn last reviewed the fleet and how many
runs and findings it flagged, linking to its proposal. It SHALL NOT run
Muninn.

#### Scenario: No Muninn record

- **WHEN** no Muninn record exists
- **THEN** the panel omits the Muninn line rather than showing an empty one

### Requirement: Live refresh is bounded and resumable

The host SHALL push changes only for the visible realm and the open run. A
reconnecting client SHALL receive a fresh snapshot. Closing the panel SHALL
NOT affect any run.

#### Scenario: Reconnect

- **WHEN** the client reconnects after its socket dropped
- **THEN** it receives the current snapshot, and any open command card is re-validated against the new head

## Provenance

- `2026-10-13-568-operator-console-slice-one` — folded 2026-10-14
