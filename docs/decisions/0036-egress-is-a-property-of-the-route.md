# 0036 — Egress is a class, and it belongs to the route, not the binary

Status: accepted (ruled 2026-09-03)
Date: 2026-09-03

## Context

Decision 0021 ruling 4 named egress its own axis and said a driver's
egress **class** governs what may be serialized toward it. Ruling 7 sent
jurisdictions to the same place: "which jurisdictions are acceptable
egress is the operator's threat model expressed in ruling 4's classes."
The decision called for classes. The implementation shipped one
boolean, `binding_grant`, and no class vocabulary exists in the engine.

What the adapters declare today:

| adapter | `trust_tier` | `binding_grant` | destination |
|---|---|---|---|
| claude | trusted | yes | Anthropic |
| codex | trusted | no | OpenAI |
| dsh | untrusted | no | **three different places** |
| exec | untrusted | **yes** | this machine |
| lanetally | absent → untrusted | absent → no | Anthropic, via a local wrapper |

Two things are visible in that table. `exec` is untrusted yet holds the
grant — the only reason it can is that it runs here, which is the local
class already smuggled in as a one-off. And `dsh` carries one
declaration for a binary that fronts three destinations, measured on
this workspace on 2026-09-03:

| dsh route | endpoint | where the prompt goes |
|---|---|---|
| `spark/*` | `http://spark:30000/v1` | the operator's own DGX Spark |
| `dashscope/*` | `token-plan.ap-southeast-1.maas.aliyuncs.com` | Alibaba, Singapore |
| unprefixed (`deepseek-v4-pro`, …) | the profile's default provider route | whatever that profile resolves |

`dsh`'s `untrusted` / no-grant is therefore correct only by accident: it
is the floor across three jurisdictions. A model running on hardware in
the operator's own building is held to the clearance of a cloud API in
another country because the two share a CLI. The declaration site is the
provider binary; the fact that decides egress is the endpoint.

The cost of that mismatch is already in the journal. Run
`close-the-forge-to-brokkr-rename-5272942b` parked at seq 14 on
`MISSING_CREDENTIAL` for the `spark` route. The credential in question
is not a secret at all — the SGLang server checks no key, and the
profile comment says so: "the route requires one to be named, so
`SPARK_API_KEY` carries any value." But because `dsh` holds no binding
grant, that value may not be bound to the seat, and the only channel
left is the launching shell's ambient environment. Run
`close-the-forge-to-brokkr-rename-206fc661` succeeded for 484 turns
because the operator's shell had sourced the file; the identical pinned
bundle failed twice from a clean environment. The one channel the
machine cannot refuse at compile time, cannot record in the journal, and
cannot move a digest for is the channel a local seat is currently forced
to use.

Three alternatives were weighed and rejected. Granting `dsh` the binding
grant clears the Alibaba and DeepSeek routes at the same stroke, which
is fail-open on the axis that exists to fail closed. Splitting `dsh`
into one adapter per route duplicates the binary, the models map and the
tool grammar to express a fact about endpoints, and misstates what an
adapter is: a provider is a CLI, and this CLI genuinely has one. And
promoting the Spark to the `trusted` tier buys the grant by conflating
the judging axis with the receiving one — the precise braid 0021's
context says it untangled from the heritage DeepSeek rule.

## Ruling — 2026-09-03, operator: accepted as proposed

Accepted the day it was proposed, without amendment. The five rulings
and their enforcement bindings stand as written and are the commission
of the enactment slice, fired on acceptance. This ruling assigns no
route to a class: `spark` is not ruled `local` here, and every adapter
keeps the clearance ruling 4's migration gives it. The routes map
arrives as the place for that data, and the operator fills it by a
separate ruling — class assignment is operator data, as ruling 1 says.

2026-09-03, separately: the operator ruled the `dsh` route `spark` —
`http://spark:30000/v1`, the SGLang server on their own DGX Spark — is
class `local`, written into `adapters/dsh.json` as ruling 1 prescribes.
No other route is classed by that ruling: `dsh`'s own adapter clearance
is untouched, so unprefixed ids and the `dashscope/*` front stay
uncontracted, and every other adapter stands where ruling 4's migration
left it.

## Rulings

1. **Egress is a named class, and the vocabulary is closed and ordered.**
   The classes are `local`, `contracted` and `uncontracted`. `local`
   means the endpoint runs on hardware the operator controls and the
   serialized material crosses no network boundary they do not own.
   `contracted` means a third party the operator has ruled acceptable in
   a recorded ruling. `uncontracted` is everything else, and is the
   value of an absent declaration. The vocabulary is the engine's; the
   assignment of any route to a class is operator data, exactly as 0021
   ruling 2 holds for tiers.

   **Enforcement binding:** an `EgressClass` enum in
   `brokkr-runtime::agents` beside `TrustTier`, parsed in
   `agents::load` with the same closed-vocabulary refusal naming the
   file and the key, defaulting to `uncontracted` on absence.

2. **The class is declared on the route, and a route is the prefix of
   the concrete model id.** An adapter gains a `routes` map from route
   name to its declared class. A concrete model id of the form
   `<route>/<model>` resolves to that route; an id with no prefix
   resolves to the adapter's own declared class and no better, because
   an unprefixed id reaches whatever default the harness profile
   resolves and the machine cannot know where that is. Adapters that
   front a single destination declare one class at the adapter and no
   routes, unchanged.

   **Enforcement binding:** `Adapter` carries `routes: Map<String,
   EgressClass>` and a route resolver in `agents::load`; driver
   conformance tests cover the prefixed, unprefixed and undeclared-route
   cases, and the unprefixed case asserts it does not inherit a better
   class than the adapter's own.

3. **Local is structural, not earned, and it confers nothing on the
   judging axis.** A route is `local` because of where its endpoint
   runs, so it needs no journaled track record and no promotion ruling
   under 0021 ruling 3 — topology is not a reputation. For the same
   reason `local` grants no gate seat, no trust tier and no fallback
   standing. A model on the operator's own hardware may be the most
   private worker in the fleet and remain the least qualified to be the
   check.

   **Enforcement binding:** the gate-seat refusal at
   `bundle.rs:1056` continues to read `trust_tier` alone; a test seats a
   `local`, `untrusted` route at a gate and asserts the refusal is
   unchanged.

4. **Secret bindings key off the class, and the minimum is the
   operator's.** A seat declaring bindings (0012) compile-refuses unless
   its resolved route's class meets a minimum the operator rules into
   the bundle. `binding_grant` is superseded: a `true` grant reads as
   `contracted`, a `false` or absent grant as `uncontracted`, so every
   adapter on disk keeps its present clearance until the operator rules
   otherwise.

   **Enforcement binding:** the existing 0021 ruling 4 compile refusal
   is rewritten against the class comparison and keeps its fail-closed
   tests; a migration test asserts the five adapters as they stand today
   resolve to exactly the clearances they have now.

5. **A credential reaching a seat from ambient environment is reported,
   not silently used.** Where a route resolves a credential the bundle
   did not bind, `doctor` says so by route name. This decision does not
   yet forbid the ambient channel — that would strand every route whose
   class the operator has not yet ruled — but it stops the channel being
   invisible.

   **Enforcement binding:** a `doctor` line per route whose declared
   credential variable is satisfied from the process environment rather
   than the bindings store, named as a `warn`.

## Consequences

The Spark can hold a binding without clearing Alibaba, and the operator
can rule `spark` local without touching the tier that decides who
judges. `exec`'s standing stops being an anomaly and becomes the first
member of a named class. `dsh` keeps one adapter, one binary and one
models map, and gains a routes map that says the thing that was
previously unsayable.

Bundle digests move only where a route class is declared. No journal is
rewritten: existing rows carry no egress fact and their absence stays
absent, as 0031 ruling 3 holds for models. The wire protocol does not
change — egress is compile-time policy and never reaches
`forge-driver/v1`.

Decision 0021 is amended in its ruling 4 only, and by enactment rather
than reversal: the axis it named keeps its meaning and finally gets the
vocabulary it specified. Rulings 2, 3 and 7 stand untouched, and this
decision rules no jurisdiction acceptable — it builds the place where
the operator says so.
