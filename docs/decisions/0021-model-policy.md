# 0021 — Model policy: the law, not the scorecard

Status: accepted — operator ruled 2026-08-31
Date: 2026-08-31

Built: built
Amended by: 0036, 0107

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

## Addendum — 2026-09-02, operator ruled: the other drivers are the implementer's hands

`adapters/claude.json` may express `Bash(codex:*)` and `Bash(dsh:*)`,
and the two implementer agents (`implementer`, `implementer-speckit`)
are granted both. No gate-class agent is: a judge whose shell could
reach an untrusted driver would take that driver's word through a door
the compile-time tier check of ruling 2 cannot see, so the verifier,
reviewer and shipper keep the hands they had. The grant is data — a
name in the adapter's map, an entry in each agent's `allow` — and the
bundles that adopt those agents changed identity with it.

The reverse direction needs no grant and admits no cage: `codex` and
`dsh` declare `tool_permissions: "unsupported"`, so their seats already
run whatever their harness permits, `claude` included. A comparison
across drivers must say so rather than claim a symmetry the data cannot
express.

## Addendum — 2026-11-22, operator ruled: agy is trusted, its four models are promoted, and Neon and Carbon are its judges

Antigravity CLI (`agy`) is granted the **`trusted` tier**, its four
models (`agy-neon`, `agy-carbon`, `agy-hydrogen`, `agy-argon`) are
**promoted**, and **`agy-neon` and `agy-carbon` are its judges**
(#1252). The operator ruled it in three steps:

- 2026-11-20, commissioning the agy work: "ultra terms justify trusted
  and contracted, but run an eval, Neon, Carbon, Hydrogen, Argon". The
  terms are the Google AI Ultra subscription's, which the adapter's
  `egress: contracted` already rests on (#1200).
- 2026-11-22, asked which agy models may judge and on what evidence:
  "neon and carbon are the judges".
- 2026-11-22, asked which of the four models leave `provisional`: "all
  models leave provisional".

The evidence line is two wagers, which the controller ran and wrote up
on #1200.

Wager one is round six of #1200, the proof that an agy seat's check
binds, rerun from the lane's own run
`finish-issue-1200-the-proof-that-0fd42b65`. Its incumbent implementer,
Opus 5.5, shipped flagged with two lows. Each of its arms differs from
the incumbent only in the implementer.

- Unboxed, on an engine built from `ffcba90b` and agy 1.5.13: Neon
  `finish-issue-1200-the-proof-that-acc0addd`, Carbon
  `finish-issue-1200-the-proof-that-c79707a6`, Hydrogen
  `finish-issue-1200-the-proof-that-40a78ba2` and Argon
  `finish-issue-1200-the-proof-that-94f43065`. All four shipped
  flagged, each with the one low the story inherits. Carbon was sent
  back once on a flaky test and diagnosed it.
- Boxed, holding the engine's hands tool and no native tool, on an
  engine built from `1acdeeaa` and agy 1.5.14: Carbon
  `finish-issue-1200-the-proof-that-98da3360`, Hydrogen
  `finish-issue-1200-the-proof-that-791ff00c` and Argon
  `finish-issue-1200-the-proof-that-85fbb096` each shipped flagged with
  one low. Neon `finish-issue-1200-the-proof-that-403a9ca2` completed its
  change, and its run stopped on two verify failures that were the
  host's (#1239).

Wager two is round five of #1200, production code that moves the
workspace check to directly before the spawn, rerun from
`finish-issue-1200-an-agy-seat-s--35ff49bc`. Its incumbent, Opus 5.5,
took three implement rounds and parked on a medium. Its unboxed arms
differ from the incumbent in the implementer; its boxed arms differ in
the box as well.

- Unboxed, on agy 1.5.14: Neon `finish-issue-1200-an-agy-seat-s--d9aa31f5`
  and Carbon `finish-issue-1200-an-agy-seat-s--f592bebe` each took three
  rounds and parked on a medium, as the incumbent did. Argon
  `finish-issue-1200-an-agy-seat-s--e1b5c47c` failed verify three times
  on the host's open-file limit (#1239) and was never reviewed. The
  controller stopped Hydrogen `finish-issue-1200-an-agy-seat-s--b3ef774d`:
  agy gives an unboxed command a terminal, and its test run waited on a
  test that reads stdin (#1245). These two arms are not counted.
- Boxed, on agy 1.5.14: Hydrogen
  `finish-issue-1200-an-agy-seat-s--e646f969` shipped flagged with two
  lows after three rounds. Argon `finish-issue-1200-an-agy-seat-s--501e9e9d`
  took three rounds and parked on a medium, as the incumbent did.

| Model | Wager one, unboxed | Wager one, boxed | Wager two, unboxed | Wager two, boxed |
|---|---|---|---|---|
| Opus 5.5 (incumbent) | shipped flagged, two lows | — | three rounds, parked on a medium | — |
| Neon | shipped flagged, one low | change complete, stopped on the host (#1239) | three rounds, parked on a medium | — |
| Carbon | shipped flagged, one low | shipped flagged, one low | three rounds, parked on a medium | — |
| Hydrogen | shipped flagged, one low | shipped flagged, one low | stopped by the controller (#1245), not counted | shipped flagged, two lows, three rounds |
| Argon | shipped flagged, one low | shipped flagged, one low | failed verify on the host (#1239), not counted | three rounds, parked on a medium |

A dash marks an arm the evidence does not include.

What the evidence shows: on two real stories, each of the four models
delivered work that the same panel judged as it judged the incumbent's,
or better, and no review of any arm drew a security finding.

What it does not show. No agy model sat as a judge in any run, so when
the operator named the judges, no agy model had a judging record: that
ruling rests on the implementer wagers and the operator's judgment, not
on a record of judging. Agy reports no price, so cost per accepted
change is not measured. Only tokens are, and on wager two they ran
between 1.4 and 6 times the incumbent's, mostly cache reads. Each arm is
one run. The boxed and unboxed arms of wager two are not like for like.

This is the road rulings 3, 4 and 7 order: evidence first, then the
operator's recorded ruling, on the trust axis and the egress axis
apart. The egress class stays `contracted`, as ruled for #1200. The
promotion of the four models is the one decision 0075 ruling 6 leaves
to the operator, recorded here with the wagers' run ids. It is enacted
as data in `docs/examples/adapters/agy.json`: `trust_tier` is
`trusted`, every model is a bare id, and `judges` lists `agy-carbon` and
`agy-neon`.

Not granted here: Hydrogen and Argon are promoted and are **not
judges**. A gate seat on either is still refused, now because it is not
in `judges` (decision 0041 ruling 3), no longer because of its tier.
The **binding grant** is not granted, as for codex.

**Accepted exposure: an agy judge in an unboxed seat can edit the
tree.** An agy judge may sit in a boxed seat or an unboxed one. In an
unboxed seat agy's own write tools are on, as a claude gate's are under
`acceptEdits`. The engine's check on a gate compares the repository
head (decision 0041 ruling 4), so an edit a judge leaves uncommitted is
not caught. The operator accepted this for agy on 2026-11-22 as the
state the house already has for claude, choosing to accept it as it
stands for claude, with the exposure stated plainly: "the decision -
1." Closing it for every harness is #1260.

