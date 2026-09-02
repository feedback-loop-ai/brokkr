# 0021 — Model policy: the law, not the scorecard

Status: accepted — operator ruled 2026-08-31
Date: 2026-08-31

## Context

The driver fleet holds five adapters (decision 0008; Rust-only per 0009):
claude, lanetally, codex, dsh, exec. Which driver sits in which seat is
already bundle data under a pinned digest — the enforcement substrate for
model policy exists; what is missing is the vocabulary.

The heritage protocol carried a vendor-named rule — the DeepSeek boundary —
braiding three concerns: data residency toward one provider, the origin
workspace's own repository topology, and a principle about who may hold
gate seats. The first two died with the transition. The third is permanent
and deserves to live in the machine instead of the folklore: work seats
produce output the system checks; gate seats ARE the check, and nobody
stands behind the judges.

A neighboring temptation is refused at the door: cost accounting, outcome
scoring, lane economics, and promotion evidence are LaneTally's domain —
the operator's own instrument for exactly those questions. Brokkr rules on
what is permitted; LaneTally scores what it was worth. The law cites the
scorecard; it never duplicates it.

## Decision

1. **Seats divide into work and gate.** Work seats (intake, implement,
   positions and their kin) produce output the machine checks. Gate seats
   (chief, review, security review, verify, ship) are the check. The
   division is policy data, named per seat role in the bundle.

2. **Drivers carry an operator-granted trust tier.** The policy names
   tiers, not vendors: a driver's tier is data the operator rules into the
   bundle, never a constant in the engine. Gate seats require the trusted
   tier. **The bundle compiler refuses a bundle that seats an untrusted
   driver at a gate** — the refusal happens at compile time, before any
   prompt exists to leak, in the manner of a digest mismatch.

3. **Tiers are earnable, in both directions.** A promotion or demotion is
   an operator ruling recorded where rulings live, and its evidence line
   cites the scorecard — LaneTally outcome and cost reports, `compare`
   results, verify records — the way a saga cites a run id. No driver is
   trusted or distrusted by vendor name, forever, in code.

4. **Egress rights are their own axis.** Trust to judge and clearance to
   receive are different grants. A driver's egress class governs what may
   be serialized toward it; its first and sharpest consequence: **secret
   bindings (0012) are grantable per driver**, and a driver without the
   grant cannot appear in a seat that declares bindings — refused at
   compile time, fail-closed, before serialization.

5. **An unavailable driver parks the run; nothing substitutes.** A
   substituted model is a different bundle identity wearing the pinned
   digest. Substitution is the lie this machine exists to refuse; the park
   reason names the missing driver and the operator decides.

6. **Cost stays where it lives.** Seat bounds remain decision 0006's
   (attempts, deadlines); the economics of lanes — marginal cost,
   amortization, outcome value — remain LaneTally's ledger, reachable
   through the lanetally driver and the operator's own reports. This
   decision adds no accounting machinery to Brokkr.

7. **Newcomers are symmetric.** Every driver without a journaled track
   record here starts identically, whatever its vendor or flag: work
   seats freely, no secret-binding grant, no gate seat — codex and dsh
   alike. Their first outings are wagers — the same feature run under
   rival crews, compared by artifacts (`rerun`/`compare`, the mechanic
   the Edda records as the judging of the gifts). Grants and gates open
   to any driver only by ruling 3's road: evidence, then an operator's
   recorded ruling. Which jurisdictions are acceptable egress is the
   operator's threat model expressed in ruling 4's classes, never a
   default of this document.

8. **A disclosure the law keeps.** The agent that drafted this decision
   runs on a claude model. That is a conflict of interest wherever
   claude's tier is discussed, and it is exactly why rulings 2 and 3
   ground every tier in journaled evidence and operator rulings: the
   incumbent's standing rests on this shop's recorded runs, not on the
   drafting agent's provenance, and it is revocable by the same road any
   newcomer climbs.

## Consequences

Model choice becomes governable without becoming bureaucratic: the two
prohibitions that matter (untrusted judges, unsanctioned egress) are
compile-time refusals with the digest's own temperament, everything else
is operator judgment informed by the operator's own instruments, and no
vendor's name is welded into the engine. The cost is honest bookkeeping at
promotion time — a tier change is a ruling with evidence, not a config
edit — which is not a cost at all in this shop.

## Addendum — 2026-09-02, operator ruled: codex is trusted

Codex is granted **`trusted` tier for every seat class** — work seats
and gate seats alike. The evidence line: the wager (crew B under parity
rigging; the judged synthesis landed as PR #85 with crew B's recognizer
closing a residual crew A left open), the first foreign delivery's
lineage, and the operator's own standing use. This is exactly the
promotion path ruling 4 orders: a tier change is a ruling with
evidence, recorded here, enacted as data in `adapters/codex.json`.

Not granted here: the **binding grant** (secret-bearing seats) stays
`false` — a separate grant with its own evidence bar, unruled today.
The adapter's tool-permission mapping and session reuse remain the
enabling engineering, delegated to the machine as its own slice.

