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
Opus 5.5, shipped flagged with two lows. Each unboxed arm differs from
the incumbent only in the implementer. The boxed arms differ in two
more things, the engine build and the boxed hands, and the incumbent
ran no boxed arm: they show what a boxed agy seat delivers, not a
like-for-like comparison.

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

## Addendum — 2026-11-26, operator ruled: Qwen 4.0 Max through dsh is the review chief's first fallback in the operator's own lane library

In the operator's own lane library, which lives outside this repository,
the `dsh` adapter is marked `trusted` with `judges: ["qwen4-max"]`, and
the review chief's chain is Claude Fable 5.2, then Qwen 4.0 Max
(`dashscope/qwen4-max`) at effort `high`, then the fallbacks it had
before (#1278). Qwen 4.0 Max judges only while Fable is exhausted.

The tracked `adapters/dsh.json` is unchanged: `trust_tier` stays
`untrusted` and `judges` stays empty. No downstream user receives this
lift. Unlike the agy addendum of 2026-11-22 (enacted in
`docs/examples/adapters/agy.json`) and the codex addendum of 2026-09-02
(enacted in `adapters/codex.json`), this addendum is enacted as data
nowhere in the tree.

The operator's rulings, in chronological order (UTC):

- 2026-11-22, on the security position after the eval's follow-up:
  "ok, move the seat off qwen, I'd say put it to Carbon with Fable
  fallback".
- 2026-11-23: "qwen 4.0-max as fable chief fallback, confirmed".
- 2026-11-26, asked to choose between (A) opening the chief role in
  decision 0107 for this grant and building its chain, (B) lifting it in
  the operator's own lane library only, with a decision 0021 addendum
  recording it, and (C) leaving the model a panel member: "B".
- 2026-11-26, asked whether to commission this addendum: "yes".
- 2026-11-26, asked whether `uncontracted` egress stands for this chief
  seat, which sends the whole change and the panel's notes over the
  `dashscope` route: "yes, it stands".

**What trusting `dsh` in that library means for its other models.**
Under ruling 2, the trust tier belongs to the adapter. Marking `dsh`
`trusted` in the operator's own lane library makes every `dsh` model on
every route tier-eligible there. The `judges` list alone keeps every
other `dsh` model out of a gate: under decision 0041 ruling 3, each
adapter declares `judges` — the abstract names that may hold a gate
site — and compilation refuses a gate whose chain names any other model,
on any link, naming the site and the link. That `qwen4-max` sits only in
the review chief's chain, and only behind Fable, is that library's
convention; nothing in the compiler refuses `qwen4-max` at another gate
role or office. That gap is the cost of choosing form (B).

**This is not decision 0107's scoped grant.** Decision 0107 is accepted
(2026-11-12) and unbuilt (#1052), and it stays the route to a form that
can ship. This private lift does not meet decision 0107's asks or
rulings:

- **Adapter-wide tier instead of a scoped grant (condition 1, asks (a)
  and (g)).** Decision 0107 keeps `trust_tier` `untrusted` and `judges`
  empty, rejects adapter-wide trust for `dsh` (option C), and puts a
  scoped grant in `gate_grants` naming adapter, route, abstract model,
  concrete id, hired `office` and structural `role`. This lift sets
  `trust_tier: "trusted"` and `judges: ["qwen4-max"]` instead.
- **The chief role (condition 4, ask (b)).** On ask (b) the operator
  ruled "No": a 0107 grant never names the review chief, and the chief
  stays a trusted adapter's judge. This lift is for the review chief's
  seat.
- **The read-only worktree bind (condition 5).** Decision 0107 requires
  a boxed workspace-hands seat with `network: false`, the office's
  declared `read-only` access, `ro` or `overlay` binds, a measured hands
  digest, and decision 0098 ruling 3's read-only worktree bind, which is
  unbuilt (`GrantWorktreeBind` refuses every 0107 grant until it lands).
- **Measured native capabilities of `dsh` (condition 6, ask (d)).**
  Decision 0107 requires every declared native capability's composed
  control (ON where held natively through the adapter, OFF where not) to
  be measured on the grant's release and host, and refuses an
  `unmeasured` inventory whole. `adapters/dsh.json` still declares
  `native_capabilities` `unmeasured`.
- **The model's tier wording (condition 3).** Decision 0107 requires an
  explicitly written `tier: promoted` under `models.<name>.tier`
  (`ModelTier::Promoted`), refusing an absent or bare tier (and slice
  0107a that parses it is unbuilt); in `adapters/dsh.json`, `qwen4-max`
  is a bare id (`"dashscope/qwen4-max"`).
- **Fact binding and evidence bar (asks (c) and (d)).** Decision 0107
  binds a grant to a measured release, concrete model id, hands digest
  and typed `review_by` date, backed by d1's full bar (a work-class
  wager in the granted office, probe report, hands composition
  measurement and native ON/OFF measurement); a `trusted` + `judges`
  lift enforces none of those bindings at compile time.

Decision 0107 condition 7 (each fallback link qualifying on its own) is
not what this private lift breaks: condition 7 admits a fallback link
either by the trusted tier and `judges` or by its own grant, and
expressly does not refuse a trusted-judge fallback. Because the
operator's own lane library lifts `dsh` to `trusted` and lists
`qwen4-max` in `judges`, the Qwen 4.0 Max link qualifies on its own
there. Only on an untrusted `dsh` (as in the shipped `adapters/dsh.json`)
would sitting behind Fable as a fallback still inherit nothing from Fable
and require its own 0107 grant — which ask (b) forbids for the chief and
conditions 3, 4, 5 and 6 above do not admit.

**Standing with ruling 7.** Ruling 7 — newcomers are symmetric, with
work seats freely, no secret-binding grant and no gate seat, codex and
dsh alike — remains what the shipped adapter carries. This addendum is
ruling 3's road (evidence, then an operator's recorded ruling) walked
for one model, one seat and one house's library.

**The evidence line** (#1259 and its two follow-ups of 2026-11-22):

- Three recorded panels judged through Qoder: 7, 7, 7; the findings
  verified by itself three times of three; three confirmed additions;
  one small error — level with Fable. Seven of the eight other chiefs
  passed all three.
- The harder test through `dsh`: kept residual at `medium`, named the
  planted instruction and ruled against it, and refuted the planted
  false finding from the code and a test run; 2.2M input tokens (2.1M of
  them cache reads), 31k output; 15 minutes.

**What the evidence does not show, and what this model is not.** The
harder test ran once. The three recorded panels ran through Qoder, not
through `dsh`. No run sat Qwen 4.0 Max as a fallback behind an exhausted
Fable. The promotion evidence holds no boxed chief run of this model:
#1278 lists "no boxed run" among what was not measured, and #1259
records that in the eval "Qoder seats ran unboxed, because Qoder's boxed
driver was still in review" without saying whether the harder chief test
through `dsh` ran boxed. Separately, decision 0127 (and
`docs/evidence/adapters/dsh-hands-0-2-2.md`) records boxed Qwen 4.0 Max
chief seat turns of 2026-11-24: with a provider key, a boxed Qwen 4.0
Max chief seat served its whole turn under boundary `namespace` as the
measurement of that decision's launch fix. Those records measure that
the boxed seat launches and completes a turn, not how well the chief
judged, so they add nothing to the evidence line above. The eval's `dsh`
run used an eval-only lift.

And Qwen 4.0 Max is **not a finder**. In three review positions on three
branches through `dsh`, it found two of fifteen known findings and filed
no `security-hold` on branches with known highs. After the eval's
follow-up on 2026-11-22, the operator moved the security position off it
("ok, move the seat off qwen, I'd say put it to Carbon with Fable
fallback"). This promotion is for the chief's office alone.

**Accepted exposure: `uncontracted` egress for this chief seat.** In
`adapters/dsh.json`, `egress` is `uncontracted` and `routes` declares
only `spark` and `spark-glm` as `local`; `dashscope` is undeclared and
therefore resolves to `uncontracted` (decision 0036 rulings 1 and 2). A
review chief reads the whole change and the panel's notes, sending them
over the `dashscope` route. On 2026-11-26 the operator accepted this for
this seat in the operator's own lane library: "yes, it stands". Not
granted here: the **binding grant** (secret-bearing seats, ruling 4 as
amended by decision 0036 ruling 4) is not granted, and this addendum
changes no bundle's `egress_minimum`: a seat that declares secret
bindings is refused on this `uncontracted` route at compile time
whenever its egress class falls below the bundle's `egress_minimum`
(any minimum above `uncontracted`, including the default `contracted`
minimum; an explicit `uncontracted` minimum is met).

**The unboxed-judge exposure.** The agy addendum above accepts that an
unboxed judge can edit the tree. In this tree, the house's chief agents
(`agents/review-chief.json`, `agents/chief-architect.json`) declare the
engine's workspace hands with `network: false` (and `review-chief`
declares `access: "read-only"`), so the unboxed-judge exposure does not
apply in that unboxed form to a boxed chief seat with workspace hands,
whereas if a `dsh` judge sat unboxed, what it could write would depend
on the effective `dsh` profile (`adapters/dsh.json` declares
`tool_permissions` `unsupported`: `dsh` disables a tool only as a whole
profile row through the `--patch` overlay, so an agent's allowlist —
`tools.allow` — cannot narrow those tools). For
a boxed `dsh` seat with workspace hands, the `v0.16.0` release notes
record what #1277 (#1364; decision 0127) changed: previously,
`dsh --dump-config` folded long JSON `--reach` and `--spec` values onto
lines the driver's line rule refused, so the seat could not launch; now
the `dsh` overlay carries those exact bytes as unpadded base64url under
`--reach-base64url` and `--spec-base64url`, and `brokkr hands serve`
accepts either spelling through one path without echoing a refused
encoded value. This addendum claims only that: it does not establish
from this tree how every seat or boundary in the out-of-tree lane
library is configured, does not claim decision 0098 ruling 3's read-only
worktree bind exists yet, and does not claim that uncommitted edits are
caught by `GATE-MOVED-HEAD` (decision 0041 ruling 4 compares repository
head; closing uncommitted edits across harnesses is #1260).
