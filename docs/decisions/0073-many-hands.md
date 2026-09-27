# 0073 — Many hands: the core is complete for one operator and extensible by contract, so an enterprise layer builds on it and never forks it

Status: proposed
Date: 2026-09-27

## Context

The operator asked how a team uses Brokkr, then said what the question
really is (2026-09-27): single-operator use is right for open source, team
use is an enterprise feature, and **the architecture must be extensible in
a way that lets Brokkr go enterprise.** This decision is about that
extensibility. It does not specify enterprise features.

Brokkr already has one extension that works this way. A model harness is
not compiled into the engine: it is a driver behind a frozen, versioned
protocol (decisions 0001 and 0003, `driver-protocol.v1`), and a new
harness is new adapter data plus a driver, with no engine change.
Decision 0071 made that a principle: "the plugin boundary is a process".
The shape this decision needs is already proven. It is not yet applied
to the concerns a team or an enterprise adds.

Those concerns, taken from what a team would ask for:
- **who is acting:** named operators, roles, signed attribution of
  rulings;
- **who may act:** who may rule, apply the by-hand label, accept a
  residual, or have their queue entry started;
- **where work runs:** a shared runner host with a queue several people
  feed, and standing work under a grant (decision 0025);
- **what everyone can see:** a gathered team view, dashboards, audit
  export;
- **what it costs and on whose account:** per-person attribution, and
  which account may serve whom;
- **where secrets live:** an organization's secret store rather than a
  local one (decision 0012).

Today each of these is either absent, or hard-coded to a single local
operator in the engine's own code: the journal assumes one operator, the
queue (0068) admits by the engine's rules alone, and the secret resolver
is local. An enterprise layer written against that code today would have
to patch the core. That is the fork this decision prevents.

## Rulings

1. **The line.** The open core is complete for one operator: engine,
   gates, realms, capabilities, the queue for one person, and evidence
   through git (0033, 0038). A small team can already work on it, because
   every pull request carries a self-verifying run. Everything a team or an
   organization adds is an **extension**. It attaches only through the
   seams in ruling 3, never by patching, forking or feature-flagging core
   code.

   **Enforcement binding:** the core crates never depend on an
   extension. A dependency lint refuses any core crate importing a crate
   outside the core workspace members.

2. **An extension is a process behind a versioned contract**, as a
   driver is (0071). Each seam is a contract in `contracts/`, frozen like
   every other, with a JSON envelope over stdio or a file. An extension is
   an executable the realm names. The core invokes it, reads a typed
   verdict or record, and treats a missing, crashed or malformed
   extension as a refusal, never as permission. There are no in-process
   plugins, no dynamic loading, and no trait objects an outside crate
   implements. The same reasons 0071 gave for drivers apply: the boundary
   can be digested, replayed and pinned into a run's identity.

   **Enforcement binding:** each seam's schema with the frozen-contracts
   test, and the seam's refusal tests: absent, non-zero exit, malformed
   output and timeout, each an exact refusal.

3. **The seams.** Each seam has a core default, which is exactly today's
   single-operator behaviour. With no extension configured, nothing
   changes.

   | Seam | What it decides | Core default | An enterprise extension would |
   |---|---|---|---|
   | **identity** | who is acting (an operator id) | the single local operator | map a signing key or SSO identity to a named operator |
   | **authority** | whether an act is allowed: an operator command, accepting a ruling, the by-hand label, starting a queue entry | allow, for the single operator | enforce roles, signed rulings, two-person rules |
   | **admission** | whether a queue entry may start now (0068) | 0068's deterministic rules only | add signature, account-class and per-operator budget checks |
   | **grant** | whether standing work is covered (0025) | no standing work | verify signed, expiring grants |
   | **secrets** | where `{{secret:NAME}}` resolves (0012) | the local store | resolve from an organization's vault |
   | **events** | who may read the journal as a stream | no subscriber | feed a team view, dashboards or audit export, read-only |

   The identity seam's output, an optional `operator` id, is recorded on
   journaled operator commands and on the run manifest. When it is absent
   it means the single operator, and no readout requires it. That id is
   the one piece of team-shaped data the core stores, because attribution
   written after the fact would be worthless.

   **Enforcement binding:** one contract per seam; the optional field in
   the event and manifest contracts; and, per seam, a core test that the
   default reproduces today's behaviour exactly.

4. **Every seam is consulted at one place, and its verdict is journaled.**
   The engine calls each seam from one function, never ad hoc: the
   operator-command path, the acceptance check, the gate, the dispatcher's
   admission and the secret resolver. Each call's verdict (seam, extension
   digest, verdict, bounded reason) is recorded beside the act it
   governed. A run therefore shows which extension allowed what. The
   extension's digest joins the run's identity, as an adapter's does.

   **Enforcement binding:** a single seam-call function per seam, and a
   test that an act governed by an extension carries its verdict record
   and the extension's digest in the manifest.

5. **Evidence remains the interface between people.** Published anchors
   stay self-verifying without any server, and 0027's import stays the
   only way a run enters another journal. A team view is an events
   subscriber or a collector over imports. It is never a shared, writable
   journal, and never a network filesystem: the lock contention measured
   on 2026-09-25 and 2026-09-26 (#394) and decision 0029's single-host
   fence both rule that out.

6. **Enterprise features are out of this tree's scope.** Roles, signed
   rulings, the team view, the shared runner host, account classes,
   per-operator cost and standing grants are built outside the core,
   against the seams. This decision does not specify them, and a later
   one may, wherever they live. It **amends decision 0025 ruling 7** as
   follows. The core carries the grant *seam* and its refusals (an
   uncovered act refuses). The grant schema and its verification move to
   the extension that verifies it.

7. **Enactment, in order.** Each seam lands with its inert default, so
   every slice ships with no behaviour change:
   - (i) the extension boundary itself (ruling 2), then identity and
     authority, with the optional `operator` field;
   - (ii) admission on the dispatcher, when 0068 is built;
   - (iii) the events stream and secrets;
   - (iv) the grant seam, with 0025's standing executor.

   A reference extension that exercises every seam, and lives in tests
   only, proves each slice.

## Consequences

- The open core loses nothing and gains no configuration: a single
  operator never sees a seam.
- An enterprise layer can be a separate repository of executables that
  speak the frozen contracts. The core's release cadence does not bind it,
  and it binds nothing in the core.
- Each seam is a new frozen contract, and a new place where a refusal
  can come from. The cost is paid at slice time, with tests.
- Attribution exists from the first slice onward, even for a single
  operator, so a team's history does not start blank.

## Alternatives weighed

- **Build team features into the core behind flags.** Rejected: the
  core would carry every enterprise concept, and the line in ruling 1
  would exist only in documentation.
- **In-process plugins through traits or dynamic loading.** Rejected
  for the reasons 0071 gives: they cannot be digested or replayed, a
  crash takes the engine down, and the core's types become someone
  else's API.
- **Leave it until an enterprise customer asks.** Rejected: the one
  field the core must store (the operator id) is worthless if added
  late, and every seam retrofitted after the fact means the core is
  patched after the fact.
- **One journal on a network filesystem, or a central server as the
  store of record.** Rejected by ruling 5.

## Open questions for the operator

- Is the seam list complete? Candidates not included: notification
  (tell someone a run parked) and billing export, both of which the
  events seam may cover.
- Should the reference extension live in this repository's tests, or
  be published as an example repository for extension authors?
- Does the authority seam also govern `brokkr run` itself (who may fire
  a run on this realm), or only acts on existing runs and rulings?
