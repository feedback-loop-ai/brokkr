## MODIFIED Requirements

### Requirement: The run view

Selecting a run SHALL show:

- the run list grouped needs you / running / last 24 hours / older, every run
  in exactly one group (older holding acknowledged quarantines and every run
  that finished before the last day), with a text filter;
- the run header with Brokkr's standing and, for a run entered at a gate, the
  gate it entered at and the runs it names as continued, each led by its
  feature with its id after, as the run view derives them from the run's own
  start (`run-continuation`), claiming nothing beyond the naming;
- the graph of steps with their members, the model that served each, visits
  and the return paths taken;
- the control plane on the graph, and why the run waits when it does;
- the timeline, newest first.

The view SHALL use only Brokkr's standings and SHALL NOT show a
queued/started/verified state machine.

#### Scenario: A review sent work back

- **WHEN** security review once ruled the run back to implement
- **THEN** the graph shows the return path with the ruling's time and rule

#### Scenario: A run entered at verify continuing two runs

- **WHEN** the console shows a run whose `run/started` records an entry at `verify` continuing runs `a` and `b`
- **THEN** the header names `verify` and the two runs by their features, ids after, in the order named, and derives nothing the run view did not
