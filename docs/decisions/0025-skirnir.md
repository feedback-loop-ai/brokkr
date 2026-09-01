# 0025 — Skírnir and the grant: the sword is a signed loan

Status: proposed
Date: 2026-09-01

## Context

Fully autonomous delivery does not need an always-awake operator; it needs
the operator's *authority* to remain the only source of certain powers
while everything else runs around the clock. The working architecture,
proven live by the sessions that built this machine, is three loops at
three cadences with authority only in the slowest:

- **The engine's loop** (seconds–minutes): deterministic — phases, gates,
  reforging, parks. Every policy that needs no judgment sinks here (the
  merge tally became branch-protection law the day this was drafted).
- **The steward's loop** (minutes–hours, around the clock): judgment
  *inside a granted envelope* — resume determinate parks, launch
  pre-ruled slices, apply named remediation patterns, escalate the rest.
- **The operator's loop** (daily): rulings, from a briefing.

Separation of powers, one per head: **Muninn knows** and may do nothing
(0020); **Skírnir acts** and only within his grant; **the operator
rules** and needn't watch. No agent holds two of the three.

The name is the summoned-persona tier working as designed (0019 ruling
10): Skírnir is Freyr's servant, sent to act in his lord's name carrying
his lord's own sword — borrowed authority, explicit commission. The saga's
second half is the design constraint, not decoration: Freyr never got the
sword back, and at Ragnarök he fights with an antler. Delegation that
cannot be recalled is not delegation; it is abdication.

## Decision

1. **Skírnir is the standing executor.** A summoned persona invoked on
   events or cadence — never a daemon with tenure (0020 ruling 4's
   temperament): each invocation is a bounded driver-fleet seat with a
   deadline and a budget, and standing presence is repeated summons.

2. **The grant is runtime data, operator-signed.** `forge.grant/v1` is a
   versioned schema contract; a grant instance is a JSON file naming:
   the enumerated permissions (each an operator-command class or named
   action pattern), per-day and per-invocation budgets, an **expiry
   (mandatory — a grant without one is invalid)**, and a grant id. The
   file carries the operator's **GPG signature**, and the engine refuses
   an unsigned grant, a mis-signed grant, an expired grant, or a grant
   whose schema it does not know. Changing what Skírnir may do is
   editing and re-signing a file — no recompile, the operator ruled.

3. **The ceiling is constitutional and compiled.** A never-list no grant
   can override, enforced by the loader the way digests are enforced:
   no changes to decisions, policy tables, or contracts; no identity,
   visibility, or branch-protection changes; no force-push; no release
   tags; no secret-binding grants; no creating, editing, or extending
   any grant, **including his own**; no invoking another standing
   persona into being. A grant file claiming any of these is refused
   whole — fail-closed, before any action.

4. **Every exercise is journaled as the grant's.** Skírnir's operator
   commands and actions carry the grant id and the grant's content hash
   — never the operator's bare name. The journal must always answer:
   *which* authority acted, under *which* signed loan, and whether the
   loan was valid at that moment. Pinned and embedded, in the realms
   map's manner.

5. **Escalation is the default, not the exception.** Anything the grant
   does not enumerate parks where it stands and joins the operator's
   briefing — Muninn reports it, with citations; Skírnir neither
   proposes (that is Muninn's office) nor improvises. Act within the
   loan, or hand the piece back.

6. **Layering.** Per the OSS-core ruling, the grant *schema* and the
   loader's refusals may land in the open core (they are contract and
   law); the standing runner — summons, cadence, briefing assembly — is
   build-on-top territory and waits for the core's v1.0.

## Consequences

The operator's day shrinks to rulings; the machine's night runs under a
signed, expiring, revocable loan whose every use is journaled against its
hash; and the failure modes divide cleanly: a bad grant is the operator's
signature on a bad idea (visible, revocable), a grant overreach is
impossible below the compiled ceiling, and a Skírnir invocation gone
strange is a bounded seat like any other — killed by deadline, readable
in the journal. The cost is ceremony at the moments ceremony belongs:
granting, renewing, revoking. Freyr's antler is the reminder priced into
rule 2: **the sword is a loan, and the expiry field is how it comes
home.**
