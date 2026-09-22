# 0068 — The dispatcher: runs wait in a queue the engine owns, and start only when a provider, the host and the queue's own order all allow it

Status: proposed
Date: 2026-09-23

## Context

Brokkr decides everything inside a run and nothing between runs. Which run
starts next, whether a provider can take another seat, whether the host has
room for another box, and what a run is waiting for are all decided today by
whoever is sitting at the bench — in practice the controller, by hand, with a
shell script.

The journal shows what that costs. Across its first month (2026-08-22 to
2026-09-22):

- **Provider limits park runs after the work is paid for.** Fourteen parks in
  ten runs read `rate_limit`: a seat was dispatched, the provider refused the
  first turn twice, and the run stopped mid-phase to wait for a person. On
  2026-09-22 one repair run of decision 0065 sat parked on a rate limit for
  eighteen hours before anyone looked.
- **The host fills up and fails the work that is running.** Ten parks were
  host-resource failures — no space left on the `/tmp` filesystem, a seat that
  wrote no result file, an executable the kernel refused as busy under load —
  and the exact coverage gate is known to starve when several seats build at
  once. On 2026-09-23 the controller reaped sixteen gigabytes of dead seat
  trees by hand.
- **Shared local hardware is treated like a cloud endpoint.** The DGX Spark
  runs the operator's other workloads; a seat routed to it inherited a
  one-hour limit sized for a cloud API and was killed with nothing written.
  The operator ruled that Spark seats never carry that limit.
- **The order of work lives in a script.** The operator's rules for a queue —
  "start a new task only when the previous is fully complete", "queue it
  before the rest", "highest priority" — were implemented as a shell loop
  under `systemd-run` that polled `brokkr runs` every minute. It waited forever
  on a run that ended `stopped`, because a stopped run never reads
  `completed`; it had to be edited and restarted by hand four times in two
  days as priorities moved.

Decision 0025 names Skírnir, the standing executor who acts under a signed
grant; it is unbuilt, and it is a persona that makes operator-class moves. What
is missing here is smaller and must be deterministic: a queue and an admission
rule. The operator ruled on 2026-09-23 that it is a separate decision from
0067, which bounds the work inside a run.

## Rulings

1. **The queue is operator data, and the engine owns its state.** A
   commission becomes an entry — recipe, repository, feature text, and any
   realm map — in a queue held in the journal's own database, so its state
   survives the session that wrote it. `brokkr queue add`, `list`, `move`,
   `hold`, `release` and `drop` are the only ways it changes, each journaled
   as an operator command with its reason. A run started from the queue
   carries its entry id in its manifest.

2. **Order is declared, not implied.** Each entry has a priority and may name
   entries it waits for, with the condition it waits on: `completed`, `ended`
   (any terminal status), or, once decision 0064 is built, a named ending.
   The operator's "start the next only when the previous is fully complete" is
   `after: <entry>, on: completed`; "queue it before the rest" is a priority.
   A waiting condition that can no longer be met — the awaited run ended
   `stopped` when the entry waits for `completed` — holds the entry and says
   so; it never waits forever in silence.

3. **Admission is deterministic and checked before dispatch.** An entry starts
   only when all of these hold, and the dispatcher records which one did not:
   - its waits are satisfied;
   - every provider its compiled bundle will seat has capacity — a concurrency
     ceiling per provider and per route, declared in the realm's host
     configuration — and is not in a cool-down;
   - the host has room: free space on the seat scratch filesystem above a
     declared floor, and the number of concurrent boxed builds below a
     declared ceiling.
   Nothing about admission asks a model.

4. **Provider refusals become cool-downs, not parks.** When a provider refuses
   before the first turn with a rate limit, the adapter already fails the
   attempt; the dispatcher records a cool-down for that provider and route,
   holds new admissions to it, and — when the cool-down ends and a one-turn
   probe answers — retries the refused effect under decision 0006's attempt
   rules. The run's journal shows the wait as a wait, with the provider's own
   words, not as a park for a person. A run parks for the operator only when
   the provider's refusal is not a rate limit or the cool-down exceeds a
   declared maximum.

5. **Local hardware declares itself.** A route may be declared `shared-local`
   in the realm's host configuration, with its own concurrency ceiling and a
   seat limit default that replaces the cloud-sized one. The operator's Spark
   ruling becomes data instead of a remembered rule.

6. **The host is swept, not reaped by hand.** Seat scratch trees carry the
   effect that made them. When that effect is terminal and no process holds the
   tree, the dispatcher removes it on its next pass and journals what it
   removed and how much space it freed. A tree whose effect is still open is
   never touched, whatever its age.

7. **The dispatcher is a deterministic loop, not a persona.** `brokkr dispatch`
   runs as one process the operator starts, or one pass on demand; it starts
   runs, records cool-downs and sweeps, and does nothing else. It never
   retries a run the policy parked, never concludes or stops a run, and never
   changes an entry's order — those remain operator commands. When decision
   0025's Skírnir is built, moving the queue is an operator-command class a
   grant may enumerate, like any other.

## What this decision does not do

It does not bound or split the work inside a run: that is decision 0067. It
does not run anything in parallel that a recipe or the host does not allow. It
does not decide priorities; the operator does. It does not replace
`brokkr run`, which still starts a run directly, outside the queue.

## Consequences

The chain script, the hand-kept notes of what waits on what, the midnight
retry after a rate limit and the manual `/tmp` sweep become engine state an
operator can read with one command. The operator's scheduling rulings — one at
a time, this one first, Spark is shared — are written once, as data, and hold
while nobody is watching. Rate-limited runs lose their eighteen-hour silent
parks. The price is a new standing process and a queue table beside the
journal, both of which must meet the store's durability rules.

Builds toward decision 0025 without enacting it. Uses decision 0064's endings
as wait conditions once 0064 is built, and decision 0006's attempt limits for
every retry it makes.

## Evidence

Counted from the-forge's journal on 2026-09-23 (runs started 2026-08-22 to
2026-09-22): `run/parked` reasons containing `rate_limit`: 14 in 10 runs;
host-resource parks (no space, no result file, text file busy): 10. The
eighteen-hour park is run `build-decision-0065-slice-one-th-80bfd784`, parked at
02:55Z and retried at 20:33Z on 2026-09-22. The chain script and its restarts
are the controller's, recorded in the session that ran them. The counts will
drift as the journal grows.
