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

## Addendum — 2026-11-22, operator ruled: dsh is trusted, to judge with qwen4-max only

dsh is granted the **`trusted` tier**, and its `judges` list names
**`qwen4-max` alone**: Qwen 4.0 Max, pinned `dashscope/qwen4-max` on the
Model Studio route the adapter's other `dashscope/` aliases use. This is
ruling 3's road, taken in the order the operator ruled it, and it is
enacted as data in `adapters/dsh.json` for 0.16.0.

**The trust is granted for the tool-less shape only.** A dsh gate seat
runs with every native tool row of its measured release disabled through
the `--patch` overlay, dsh's own sandbox `read-only`, and no MCP server,
and it declares no hands. So the judge can neither read the secrets store
nor write the worktree it judges. The gate class decides this, not
`DSH_PERMISSION_MODE`, and the compiler refuses any other shape (see
"The tool-less gate" below).

The rulings:

- **2026-11-19/20:** a trust wager for Qwen 4.0 Max as review chief ("ok,
  a wager then for qwen-4.0 max?").
- **2026-11-20, after the wager:** option 1, "1, this session does it": a
  lane trial first, then this repository change and this addendum, for
  0.16.0.
- **2026-11-21:** "the qwen4-max trust PR - let's wait for 22nd": the
  escalation replay first, then this change, reviewed with an Astra chief.
- **2026-11-22, after that review held the first round:** "go with all
  recommended". A dsh gate seat is admitted only in a measured
  configuration that keeps the secrets store out of reach and the
  worktree unwritten: the tool-less shape above.

The evidence line:

1. **The wager, 2026-11-20.** Eight recorded review visits were replayed
   exactly: the same commission, head, implement and verify results, and
   the same panel members' positions, byte for byte. Qwen 4.0 Max sat as
   chief, at `high`. The Codex scorers, gpt-6.2-sol and gpt-6.2-astra,
   are neutral between the Qwen and Fable families, and they scored each
   chief against the incumbent. The bar was met.
2. **The lane trial, 2026-11-20,** on a lane library where dsh was trusted
   with `judges: ["qwen4-max"]` only. There were five live reviews: three
   on #1187's 1187a and two on 1187b.
   - The panel's floor held every time, and no severity was lowered.
   - Every citation checked was correct.
   - One side-correction was wrong: it said decision 0071 has rulings 1
     to 10, when it has 11.
   - It raised no finding of its own.
   - The negative controls refused: an untrusted dsh as a gate driver,
     and a `judges` list without the pin.
3. **The escalation replay, 2026-11-22.** The trial never saw a
   chief-raised hold, so the ground-truth case was replayed through the
   wager harness: #1137's run 1137a at its decision 653. Its panel was
   residual at `high` with no security flag, and the incumbent chief
   raised it to REVIEW-SECURITY-HOLD. Qwen 4.0 Max as chief raised
   SECURITY-HOLD on the same confirmed high finding, that `group_gone`
   reads EPERM as gone, which fails open. Its words: "the charter forbids
   a high security finding becoming a residual".

**`judges` confines the gate.** The tier admits the provider, and
decision 0041 ruling 3 admits the abstract hire. So a gate seat on dsh
compiles on `qwen4-max` and is refused on any other dsh model, in that
ruling's existing words: "names model '…', which driver 'dsh' does not
declare in 'judges'". The check runs on every fallback link, so a chain
cannot fall back to another dsh model at a gate. The bundle tests in
`model_policy_tests/dsh_gate_tests.rs` hold the admission, the refusal
of each of the other sixteen models the adapter maps, and a removal
control for each half of the grant.

**The tier's reach.** The tier is read in one place: ruling 2's compile
check on a gate link (`bundle/route_policy.rs`). The engine, the view
and the doctor do not read it, and neither does any seat record. What
the grant does and does not reach:

- **Gate eligibility.** A gate seat, or any link of a gate's chain, may
  name dsh, but only on `qwen4-max`.
- **Egress, unchanged.** dsh stays `uncontracted`, with `spark` and
  `spark-glm` ruled `local` as before. A seat that binds a secret is
  refused on dsh's cloud routes wherever the bundle's `egress_minimum` is
  above `uncontracted`; a bundle whose operator rules the minimum
  `uncontracted` admits that binding, as it does on every uncontracted
  route (ruling 4 as enacted by decision 0036 ruling 4, where the minimum
  is the operator's). Trust to judge is not clearance to receive.
- **The tool-less gate.** The tier grants no hands, and a dsh gate is
  served none of dsh's own tools either:
  - The engine marks every gate-class site (`gate_seat`), and for a seat
    so marked `brokkr driver dsh` ignores `DSH_PERMISSION_MODE` and
    stages no runner. Its overlay is the shape #896 round 2 measured on
    dsh 0.2.1-rc.2 and rc.3: each effectful row disabled, each reading
    `disabled: true` in `--dump-config`, the remaining bookkeeping tools
    reaching only an MCP server in scope. The gate has no server in
    scope, so they reach nothing. Its sandbox is `read-only`.
  - Before the session the driver composes the seat's tree and refuses
    any row outside the release's measured set, any pinned row left
    enabled, and any `dsh-mcp-client` row at all.
  - The compiler refuses a dsh gate that declares hands, under every
    boundary: under `namespace` by this addendum's words, under
    `harness` and `open` by the hands law, which speaks first.
  - The record this shape answers is the adapter's own: dsh's cold shape
    without hands records its native shell reading a 0600 store canary
    (`store_read: unsupported`), and its native web tools are unmeasured.
    The tool-less gate holds neither the shell nor the web row.
- **Identity.** The adapter's digest is pinned by every bundle that
  consults it, so the manifests of six recipes moved with this edit.
- **Other drivers' shells, unchanged.** No gate-class agent gains
  `Bash(dsh:*)`. The second addendum of 2026-09-02 kept that grant to the
  implementers, and nothing here widens it.
- **The scaffold, unchanged.** The dsh adapter `brokkr init` writes is
  its own declaration. It maps no `qwen4-max`, stays untrusted and names
  no judges, so a dsh scaffold still keeps its reviewer on claude.

**The limits, stated plainly.**

- **The evidence measured a different seat.** The wager and the lane
  trial seated a Qwen chief that had dsh's native shell: in the trial's
  third review it re-ran cargo itself. The tool-less judge's quality is
  not yet measured. The controller replays the wager's items in the
  tool-less shape before this change lands.
- dsh seats are not boxed on the hosts that ran this evidence (#1158).
  The boxed judge with a read-only worktree bind is owed to decision 0098
  ruling 3 and #1158; until it is built, the tool-less shape is the only
  one a dsh gate runs in.
- The provider's token plan is capped, and a quota refusal parks the run.
  A gate seat on qwen4-max never steps down to another model (ruling 5).
- The trial measured the chief's judgment, not its discovery: it raised
  no finding of its own, and one side-correction was wrong.

