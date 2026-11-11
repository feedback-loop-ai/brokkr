# 0107 — A scoped gate grant: one route and model may hold a named gate office, on measured evidence

Status: proposed
Date: 2026-11-11

Built: unbuilt (#1052)

This decision acts on 0021's rulings 1 to 4 and 7, on 0036's rulings 1 to 3, on 0041's ruling 3, on 0043's ruling 3, on 0046's ruling 4 and on 0075's rulings 5 and 6. Because this story may not edit those files, the header pointers and the two back-pointers are absent until acceptance; the exact replacement sentences are in [What the older decisions say after acceptance](#what-the-older-decisions-say-after-acceptance), and the older files are not edited here.

This is a document only. No adapter, recipe, realm, agent or code file changes in this story. The grant described here is unbuilt; the fields, refusals and tests below are its commission, not its enactment. Until it is built, the answer to the ask is option A, below.

## Context

The issue is #1052, ask 3, verbatim:

> Gate seats (a security lane, a judge) refuse an untrusted provider, and dsh is untrusted. The only workaround found is a second adapter marked trusted and listed as a judge, which is a blanket trust grant rather than a scoped one. Ask: a documented, scoped way to admit a specific dsh route and model to a specific gate seat, with the measurement or evidence that has to back it. Alternatively, guidance that this should stay impossible, so downstream doesn't build the workaround.

The downstream case is concrete: a DashScope route on dsh, a Qwen Max preview model, serving a workspace-hands security review lane. In this tree that route's abstract model is `qwen-max`, mapped to `dashscope/qwen3.8-max` (`adapters/dsh.json:37`); `dashscope` is not in dsh's `routes` map (`adapters/dsh.json:5-8`), so 0036 ruling 1 reads it `uncontracted` (`crates/brokkr-runtime/src/agents/load.rs:368-380`). dsh's adapter declares `"trust_tier": "untrusted"` and `"judges": []` (`adapters/dsh.json:3`, `:46`). The compile therefore refuses the seat at the gate before any prompt exists: `refuse_untrusted_gate` reads the tier (`crates/brokkr-runtime/src/bundle/route_policy.rs:48-64`) and `refuse_unjudged_model` reads the judges list (`:66-95`), on every link of a chain (`:29-45`; `crates/brokkr-runtime/src/bundle.rs:2428-2445`).

### The two existing answers

**A work-class panel member under a trusted chief.** The controller's library seats two dsh models as panel members, `bugfix-review-adversarial-dsh` and `bugfix-review-correctness-dsflash`, each `"class": "work"` inside a `review-panel` aggregate; the panel's `chief` is `"class": "gate"` on Claude Fable 5.2, with Opus 5.5 as its fallback (the bundle text is quoted in #1052). This is lawful today: 0021 ruling 1 divides work from gate, 0021 ruling 7 lets a newcomer take work seats freely, and 0041 ruling 3 refuses only a gate link. The panel chief rules; the untrusted member contributes findings, not the verdict. The same shape ships in `recipes/panel-review/bundle.json:44-54`, where the two members are `class: gate` only because they are trusted models; a work-class member is the existing answer for an untrusted one. An untrusted route can therefore contribute at a gate today, as an advisory member a trusted chief rules on. It cannot be the gate.

**A second adapter marked trusted.** A second adapter file over the same `dsh` binary declares `"trust_tier": "trusted"` and lists the model in `judges`; the gate link then passes both checks. This is the workaround the issue names. It is a blanket grant: one line trusts the whole binary, every route and every model it can reach, now and in future route additions. It also duplicates the binary, the models map and the tool grammar to express a fact about one route and one model, which 0036 already rejected for egress: "Splitting `dsh` into one adapter per route duplicates the binary, the models map and the tool grammar to express a fact about endpoints, and misstates what an adapter is: a provider is a CLI, and this CLI genuinely has one" (`docs/decisions/0036-egress-is-a-property-of-the-route.md:61-66`). Guidance should say not to build it, for the reasons in option E below.

### Why the adapter is too coarse a unit of trust

Decision 0036 moved egress from the adapter to the route because dsh's one declaration fronts three destinations: a local SGLang server on the operator's own DGX Spark, `token-plan.ap-southeast-1.maas.aliyuncs.com` in Alibaba's Singapore region, and the profile's default provider route (`docs/decisions/0036-egress-is-a-property-of-the-route.md:34-44`). The same table is the argument here, on a different axis:

| dsh route | endpoint | where the prompt goes | what the evidence is about |
|---|---|---|---|
| `spark/*` | an SGLang server on the operator's network | the operator's own DGX Spark | local hardware; no third party |
| `dashscope/*` | `token-plan.ap-southeast-1.maas.aliyuncs.com` | Alibaba, Singapore | a third party and a jurisdiction |
| unprefixed (`deepseek-v4-pro`, …) | the profile's default provider route | whatever that profile resolves | an unknown default |

One `trust_tier` line at `adapters/dsh.json:3` answers for all three. But a gate's trust is not one fact: it is the harness release that mediates the prompt, the route's provider and jurisdiction, the abstract model's judgment, the office the seat holds, and the box around the hands. They have different evidence, and a route or a model can change while the others stand. The adapter is the right unit for a *tier* — 0021 ruling 2's one grant that a driver may judge — but it is the wrong unit for an exception that names one route and one model. The grant below borrows 0036's route granularity without moving trust onto the route.

### The law as it stands

| Question | Ruling | Text and line |
|---|---|---|
| Which seats are gates? | 0021 ruling 1 | "Work seats … produce output the machine checks. Gate seats (chief, review, security review, verify, ship) are the check." (`docs/decisions/0021-model-policy.md:32-35`) |
| Who may be a gate? | 0021 ruling 2 | "Gate seats require the trusted tier. **The bundle compiler refuses a bundle that seats an untrusted driver at a gate**" (`docs/decisions/0021-model-policy.md:37-42`) |
| Is the tier earned? | 0021 ruling 3 | "Tiers are earnable, in both directions. A promotion or demotion is an operator ruling recorded where rulings live, and its evidence line cites the scorecard" (`docs/decisions/0021-model-policy.md:44-48`) |
| Is trust the same as egress? | 0021 ruling 4 | "Trust to judge and clearance to receive are different grants." (`docs/decisions/0021-model-policy.md:50-55`) |
| What about a newcomer? | 0021 ruling 7 | "Every driver without a journaled track record here starts identically … no gate seat … Grants and gates open to any driver only by ruling 3's road: evidence, then an operator's recorded ruling." (`docs/decisions/0021-model-policy.md:68-77`) |
| Which abstract models may judge? | 0041 ruling 3 | "Each adapter declares `judges`: the abstract names that may hold a gate site. Compilation refuses a gate whose chain names any other model, on any link, naming the site and the link. … dsh and exec none." (`docs/decisions/0041-one-office-per-seat.md:201-208`) |
| Does egress belong to the route? | 0036 rulings 1 and 2 | "A concrete model id of the form `<route>/<model>` resolves to that route; an id with no prefix resolves to the adapter's own declared class and no better" (`docs/decisions/0036-egress-is-a-property-of-the-route.md:106-120`) |
| Does local confer trust? | 0036 ruling 3 | "A route is `local` because of where its endpoint runs … For the same reason `local` grants no gate seat, no trust tier and no fallback standing." (`docs/decisions/0036-egress-is-a-property-of-the-route.md:122-129`) |
| Does a new model start at a gate? | 0075 ruling 5 | "A provisional seat … never holds a gate (a review chief) or the sole implementer of main-bound work until promoted." (`docs/decisions/0075-harness-onboarding-in-a-day.md:148-156`) |
| How is it promoted? | 0075 ruling 6 | "A provisional seat is promoted after a wager (#237) on real stories. The wager compares it with the incumbent on cost per accepted change (#224) and on review holds and rounds (#436)." (`docs/decisions/0075-harness-onboarding-in-a-day.md:158-164`) |
| May a boxed command judge? | 0043 ruling 3 | "A deterministic command has no stochastic axis for a trust tier to measure; its blast radius is the box." (`docs/decisions/0043-the-hands-are-one-tool.md:118-124`) |
| Which boundaries hold a gate? | 0046 ruling 4 | "`namespace`, `seatbelt` and `container` hold any gate … `open` never holds a model gate. An exec gate under `harness` or `open` is refused as 0021 reads" (`docs/decisions/0046-the-boundary-is-named.md:179-191`) |
| Who grants a capability? | 0065 ruling 3 | "The realm grants, and only the realm. … A recipe cannot grant." (`docs/decisions/0065-capabilities-are-the-realms-to-grant.md:122-146`) |

Two addenda matter to the argument. The 2026-09-02 codex addendum shows the promotion road actually used: a wager, a recorded operator ruling, and the tier enacted as data (`docs/decisions/0021-model-policy.md:97-110`). The second addendum shows the door the tier check cannot see: "No gate-class agent is: a judge whose shell could reach an untrusted driver would take that driver's word through a door the compile-time tier check of ruling 2 cannot see" (`docs/decisions/0021-model-policy.md:112-127`). A scoped grant must not reopen that door; the box and the hands measurement are how this decision closes it.

## The trust subjects, named separately

A gate's verdict passes through five subjects. They are not interchangeable, and today's compile reads only two of them.

| # | Subject | What it decides | Where it lives today | The gap |
|---|---|---|---|---|
| i | The harness: its code and plugins, at a release | Whether the prompt and the verdict are carried faithfully; whether the harness's own tools and ambient configuration are off | The adapter's measured `mcp.shapes` and `hands` fragment (`adapters/dsh.json:60-162`); the composed-tree check and its measured 99 rows (`docs/decisions/0098-a-dsh-hands-seat-runs-on-its-composed-tree.md:37-58`; `docs/evidence/adapters/dsh-hands-round2.md:9-39`); 0075 ruling 3's probe (`docs/decisions/0075-harness-onboarding-in-a-day.md:116-129`) | `trust_tier` is one line for every release. The tree measures dsh `0.2.1-rc.2` (`adapters/dsh.json:111-154`); 1052a would measure `0.2.2-rc.1` (#1052), and no grant may run on a release the adapter does not measure. |
| ii | The route's provider and jurisdiction | Where the seat's material goes, and whose law and outage it meets | 0036's `routes` map and resolver (`crates/brokkr-runtime/src/agents.rs:407-411`; `crates/brokkr-runtime/src/agents/load.rs:368-380`); dsh declares only `spark` and `spark-glm` local (`adapters/dsh.json:5-8`) | 0036's class is clearance to *receive*, not trust to *judge* (0021 ruling 4). There is no route-level trust, and this decision does not create one. |
| iii | The abstract model | Whether the model's judgment has been measured and is allowed to hold a gate | 0041 ruling 3's `judges` (`crates/brokkr-runtime/src/agents.rs:434-437`); dsh declares none (`adapters/dsh.json:46`) | The list is adapter-wide and all-or-nothing; dsh's empty list refuses every abstract model, and a name added to it would admit that model on every route. |
| iv | The office held | Which gate the seat is: chief, review, security review, verify, ship | 0021 ruling 1's class, carried at the site as `class: gate`; 0041 ruling 2's roster (`docs/decisions/0041-one-office-per-seat.md:154-199`) | A gate is one bit. There is no named office on the grant, so a model admitted to one gate is admitted to every gate its agent is seated at. |
| v | The box | What the hands can reach when the verdict is produced | 0043's `hands`; 0046's `boundary`; `enforce_hands_boundary` (`crates/brokkr-runtime/src/bundle.rs:2553-2620`); dsh's boxed hands shape (`adapters/dsh.json:113-131`) | The gate check exempts only a boxed `exec` (`crates/brokkr-runtime/src/bundle/route_policy.rs:36-39`). A boxed model harness gets no credit for the box at a gate, even though 0046 ruling 4 already says which boundaries hold one. |

The five travel together. Today's `judge_link` reads the tier and the judges; it does not read the release, the route, the office, or the box, except to let a boxed `exec` pass. That is the right fail-closed default for an untrusted driver, and it is why a work-class panel member under a trusted chief is lawful today: the member is not the gate. A scoped grant adds the missing subjects one named exception at a time, and keeps the default for everything it does not name.

## The options

### (A) It stays impossible

The documented pattern is the work-class panel member under a trusted chief. The guidance says: an untrusted route may contribute findings to a panel; it may not be the panel's chief, a standalone review gate, a security gate, verify, ship, or any other gate. Do not build a second adapter to get around it; the reasons are option E's.

**Consequences.** The refusal stays exactly as 0021 ruling 2 writes it, with no new data, refusal, digest or readout. The downstream security lane keeps a dsh member and a trusted chief; the untrusted model never holds the verdict. The cost is that a model the operator judges qualified still cannot be the check, and the only road to that is a full tier promotion of dsh (0021 ruling 3), which trusts every route and model — exactly the blanket the issue refuses. Option A also leaves the second-adapter workaround as the only apparent way, so it needs the guidance and, if ask (e) is taken, a compile refusal to make the guidance real.

**Residual to name.** The work member still sends the worktree to DashScope, under 0036's egress class and 0065's capability grants; the chief still reads the member's findings, which may be wrong or prompt-injected. Those are the accepted costs of an advisory member, not a gate.

### (B) A scoped gate grant

The operator rules, with an evidence line (0021 ruling 3), that one (adapter, route, abstract model, concrete model id) may hold the named gate offices, and only when every condition below holds. The ruling lives where rulings live, as a dated addendum to this decision once it is accepted; the adapter data enacts it, as 0021 ruling 2 enacts a tier and 0075 ruling 6 enacts a promotion.

The conditions, all of them:

1. **The adapter is still untrusted, and the grant is not a tier.** `trust_tier` stays `untrusted`; `judges` stays empty; the grant is a separate list. The adapter is trusted for nothing else, on any route or model. A grant on an adapter already `Trusted` is refused as redundant: a tier is the broader grant, and a scoped exception beside it would misstate which one admitted the seat.
2. **The route and the concrete model are named exactly.** The grant names a route and an abstract model; the compile resolves the abstract model through `models` and requires its concrete id to have that route as its prefix (0036 ruling 2) and to equal the grant's `concrete` field. A grant whose alias has moved to another id is refused; a route the concrete id does not reach is refused. The route need not be declared in `routes`; if it is not, its class is `uncontracted` and the grant does not change that.
3. **The model is promoted, never provisional.** The downstream Qwen Max preview is a new model, and 0075 ruling 5 starts a new model provisional; the adapter must write `tier: provisional` until 0075 ruling 6 promotes it. The compile reads the written tier (`crates/brokkr-runtime/src/agents/load/models.rs:24-65`) and refuses a provisional model at a gate before the grant is read. An adapter that leaves the tier out reads promoted (`crates/brokkr-runtime/src/agents/load/models.rs:4-6`), which is the shipped default for every model that predates the tier, not a licence for a new preview. Promotion is 0075 ruling 6's wager and roster addendum, not this grant. The grant and the promotion are two rulings; both are needed, and the grant's evidence line cites the promotion.
4. **The office is named and is not a chief.** The grant names one or more of 0021 ruling 1's gate offices: review, security review, verify, ship. It never names the chief; ask (b) asks the operator to confirm that, and the recommendation is no. The office is a closed enum, not a string vocabulary; a site whose agent maps to `Chief` is refused even if a grant lists the other offices.
5. **The seat is boxed with workspace hands, on a measured release.** The site declares hands; the realm's boundary is `namespace`, `seatbelt` or `container` (0046 ruling 4), and the boundary is the realm's, not the recipe's (0046 ruling 1); the hands declare `network: false`; the adapter's hands fragment replaces the harness's own tools with the one workspace tool. The grant names the harness release, and the compile requires it to be the release the adapter measures for its boxed hands shape. The tree measures dsh `0.2.1-rc.2` today (`adapters/dsh.json:111-154`); a grant naming `0.2.2-rc.1` is refused until 1052a lands its measurement (#1052). At run start, the dsh driver already refuses a boxed hands seat whose observed `--version` is outside its measured set (`crates/brokkr-protocol/src/adapters/dsh_hands.rs:235-247`; `MEASURED_VERSIONS` at `crates/brokkr-protocol/src/adapters/dsh_hands.rs:56`); 1052a adds the new release there and to the adapter's measured shape, and a grant naming a release the driver does not measure is refused before the seat's first turn. The grant's evidence is the cold boxed shape, where the ten hands rows were re-measured (`adapters/dsh.json:111-131`); the replacement shape (`:133-154`) re-measures none of them and does not qualify on its own. The adapter's `hands.harness` is not a substitute: dsh declares none, and 0046 ruling 4 refuses it at a `harness` gate (`docs/decisions/0046-the-boundary-is-named.md:179-185`). A `network: true` box is refused: native egress is unmeasured on dsh (`adapters/dsh.json:157-158`), and the no-network box is the measured control.
6. **Every fallback link qualifies on its own.** The grant is looked up per link by that link's route, abstract model and concrete id. It is not copied from the first link to a later one. A chain whose first link is granted and whose fallback is not is refused at the fallback, naming its link. This is 0021 ruling 5's no-substitution rule read at compile time, and it is already how `judge_link` is called per link (`crates/brokkr-runtime/src/bundle.rs:2428-2445`).
7. **No secret-binding and no egress grant is implied.** The grant does not raise `egress_minimum`, does not stand in for 0021 ruling 4's binding grant, and does not grant a 0065 capability. The existing `refuse_secret_egress` still runs on the link (`crates/brokkr-runtime/src/bundle/route_policy.rs:99-166`), and a boxed site with secret bindings is refused by 0043 ruling 1. A gate may hold an `egress` capability only where the realm names that office explicitly (0065 ruling 7); a 0107 grant is not that naming. The route's class, and the realm's capability map, are read and reported, not changed.
8. **The grant does not weaken the judge check for any other model.** A link with no matching grant still faces `refuse_untrusted_gate` and `refuse_unjudged_model` in their present order. The grant is checked before them, and only a link that satisfies every condition above skips them; a link that has a grant for its route and model but misses a condition is refused with that condition named, not silently fallen back to the tier refusal. An inline site has no agent and no office; a grant never applies to it, and it faces the tier and judges as today.

**Compile refuses everything else and names the missing condition.** The refusals are in `route_policy.rs`, one variant per condition, with the operator's text preserved and the decision numbers cited. The exact messages are sketched in [Enforcement binding](#enforcement-binding-named-not-built).

**Consequences.** The downstream case can have a dsh Qwen Max security review gate, but only boxed, on a measured release, with a promoted model, at a non-chief office, with every fallback qualified, and with no secret or egress grant. The operator records the evidence once, in the ruling; the adapter carries the data; the compile refuses the rest. New data moves the adapter and bundle digests; no wire contract changes. The cost is a new adapter list, a new compile path, a new doctor readout, and a new thing to get wrong. The benefit is that the exception names exactly what it admits, and the blanket workaround is unnecessary.

**Residuals to name.** A harness compromised between measurements; a provider that changes the model behind the same concrete id; a grant that outlives its evidence; a box that is measured on one host and runs on another. The recommendation below names each and what this decision does and does not do about it.

### (C) Adapter-wide trust for dsh

Set `"trust_tier": "trusted"` on `adapters/dsh.json`, and add the model to `judges`. The gate passes.

**Consequences.** One line trusts the binary for every route: the local Spark, DashScope in Singapore, the OpenRouter routes, the DeepSeek official route, and every route the operator adds tomorrow. A route re-pointed to a new provider inherits the trust without a ruling. The `judges` list is adapter-wide, so every model in it is admitted on every route, including a model that a future adapter update marks provisional (0075 ruling 5 still refuses that one, but the grant's scope is wrong even where it is refused). It breaks 0021 ruling 7's symmetry: codex earned `trusted` by a wager recorded in the ledger (`docs/decisions/0021-model-policy.md:97-110`); dsh would get it by one edit for one route. It collapses 0036's three-destination table into one answer, which is the mistake 0036 was written to end. The compile cannot tell an operator's deliberate trust from a workaround. Rejected.

### (D) A per-route trust tier

Add a trust tier to the `routes` map, so `dashscope: trusted` while `spark: untrusted`. This follows 0036's unit and is narrower than C.

**Consequences.** It is still model-blind and office-blind: every model on `dashscope` — Qwen Max, GLM, DeepSeek, and any future alias — would hold every gate its agent is seated at, including a provisional model unless the provisional check also runs. It moves the judging axis into 0036's egress map, which is exactly the braid 0021's context untangles; 0036 ruling 3's "local grants no gate seat, no trust tier" would have to be read around a new field on the same map. It also gives a route-wide exception with no box, release or model condition. It is a legitimate fallback if the operator wants a route-wide grant, but it is not the scoped grant the issue asks for. Not recommended.

### (E) A second trusted adapter

The workaround, named: a second adapter file over the same `dsh` binary, `trust_tier: trusted`, with the model in `judges`, seated at the gate.

**Consequences.** It is C in another file: the binary is trusted for every route and model. It duplicates the models map, routes, credentials and tool grammar, and it gives one binary two provider identities in the manifest, the doctor and the journal. A route added to the real adapter and not the copy is judged by a stale map; a route added to the copy and not the real one silently inherits trust. 0036 rejected one adapter per route for the duplication (`docs/decisions/0036-egress-is-a-property-of-the-route.md:61-66`); this is worse, because the copy also carries a different trust tier. Rejected. Ask (e) asks whether the compile should refuse the same-binary, different-tier shape so the guidance is not the only guard.

## The recommendation

**B is the ruled road. A is the answer until B is built.**

Against A alone: A is safe, and it is what the tree does today, but it answers the ask with "no" and leaves the downstream case with a work member forever. It also leaves the second-adapter workaround as the only way to get the model into a gate, which is the outcome the issue exists to prevent. A is the right interim because B and its prerequisite measurement (1052a) are not built; it is not the right final answer for an operator who is willing to record the evidence B asks for.

Against C: C is the blanket the issue refuses. It trusts three jurisdictions, every model and every future route for one route's evidence. The adapter is the right unit for a tier; it is the wrong unit for this exception.

Against D and E: D is narrower than C but still model-blind, office-blind, box-blind and release-blind; E is C with duplication. B is the only option that names the route, the model, the concrete id, the office, the release and the box, and refuses each one it does not get.

Why B is lawful within the existing decisions, rather than beside them:

- 0021 ruling 2 gives the tier as the broad grant and the compile refusal as its enforcement; B is a named exception, recorded by the operator, and the amendment below writes it into that ruling rather than around it.
- 0021 ruling 3's road is evidence then an operator ruling. B uses that road for a route and a model instead of a driver; it does not create a second kind of evidence.
- 0021 ruling 4 keeps egress separate; B consumes the separation instead of merging it.
- 0021 ruling 7 keeps newcomers symmetric; B is the same road for a newcomer, scoped, and a newcomer still starts with no gate.
- 0036's route is B's scope, not B's authority. `local` still confers nothing (0036 ruling 3); a local route with a grant is admitted on the same evidence as any other, and the route's class is read and reported, not trusted.
- 0041 ruling 3's `judges` stays the adapter-wide list; B is a separate list, and a grant is never written into `judges`.
- 0075 ruling 5's "never a gate" is untouched: B refuses a provisional model before it reads the grant. 0075 ruling 6's wager is the promotion B requires.
- 0043 ruling 3's boxed `exec` is the precedent that the box changes what a gate's trust has to cover; B borrows the box, not the exemption. A model has a stochastic axis, so the box is necessary but not sufficient; the promotion evidence is the other half.
- 0046 ruling 4 already names the boundaries that may hold a gate; B requires one of them and refuses `harness` and `open`.
- 0065 ruling 3 keeps the realm the only granter of capabilities; B grants none and leaves the realm's map in force.

**The residual risks, named.**

1. **A harness compromised between measurements.** The release pin and the probe are point-in-time. A different installed release is refused at run start by the driver's measured set (`crates/brokkr-protocol/src/adapters/dsh_hands.rs:235-247`), but a binary or plugin replaced under the same version is not detected, and a supply-chain compromise can carry the prompt or the verdict. This decision does not solve that; it requires the measured release, makes doctor re-read the installed version, and leaves the binary's own integrity to the operator's install and the host's package controls. A future decision could pin the harness binary by digest; that is out of this story.
2. **A route alias re-pointed to another model.** The grant pins the concrete id, and the compile refuses an alias that no longer maps to it. But a provider can change the weights or the model behind the same published id; the concrete id is not a digest. The grant's evidence line names the model and the date, and any change to the id re-opens the grant; a change behind the id does not. The wager's run ids are the evidence, and a re-wager is the remedy.
3. **A grant outliving its evidence.** The grant is bound to the measured release, the concrete id and the hands composition. Any of those changing re-opens it; the compile refuses a release the adapter no longer measures, and a concrete id the alias no longer maps to. A grant whose evidence is stale but whose facts have not moved is not caught by a fact check; ask (c) asks whether to add a review date, and the recommendation is a recorded review, not a hard expiry.
4. **A box measured on one host and run on another.** 0098 closes the boxed dsh hands shape on Linux; macOS keeps a residual (`docs/decisions/0098-a-dsh-hands-seat-runs-on-its-composed-tree.md:3-5`). A grant used on macOS before the seatbelt measurement is not covered by the Linux evidence; doctor names the host, and the grant's evidence line names the host it was measured on.
5. **The office derivation.** The grant names an office enum; the compile maps the site's agent to it through one table. If a recipe seats a granted agent in a chief site, the table maps it to `Chief` and the grant is refused even though the grant's office list did not name chief. The build slice must use the one table, not a second name list, and test the chief case; a new gate agent with no row is refused rather than guessed.
6. **The boxed `exec` exemption is not widened or narrowed here.** 0043 ruling 3 admits a deterministic command at a gate because its blast radius is the box (`docs/decisions/0043-the-hands-are-one-tool.md:118-124`). A script that wraps a model is not a deterministic command, but the compile cannot tell one from the other; that pre-existing path is unchanged by this decision and is named as a follow-up, not fixed here.

Until B and 1052a land, the operator should use A: the dsh model as a work-class panel member, with a trusted chief. The operator should not seat a dsh gate on the strength of this proposal alone.

## The asks for the operator

Each ask is a choice with a recommended option, and each is unruled. The operator can rule each in one line; the "Ruling line" is the sentence to accept or replace.

### (a) Where the grant lives

**Option a1 — the adapter, as a new `gate_grants` list beside `routes` and `judges`.** Recommended. The route, the abstract model, the concrete id and the measured release are adapter facts; 0036 put the route class there, 0041 put `judges` there, and 0075 put the model tier there. One file, one digest, one operator edit. The list is not nested inside `routes`, because `routes` is a map from a route name to an `EgressClass` (0036 ruling 2) and a grant is not a class. A recipe cannot write it: the adapter library is the operator's data, and a recipe may not grant (0065 ruling 3). The cost is that a personal or per-realm variation needs its own adapter, which 0101's personal adapter root already allows outside the tracked tree.

**Option a2 — the realm, per office, beside `capabilities`.** The realm is already the only granter of capabilities (0065 ruling 3), and 0046 makes the boundary a realm fact. A realm entry could name the adapter, route, model and offices. The cost is that the realm would restate adapter data it does not own: the route's concrete id and the measured release live in the adapter, and a realm that names them can drift from it. A hybrid — the adapter names the route, model and release; the realm names the offices — is the strongest form of a2, but it splits one grant across two files and makes the compile read two sources for one fact.

**Recommended: a1.** It keeps 0036's route data, 0041's judges and 0075's tier in one operator file, and the realm keeps the capability and boundary facts that are already its own.

**Ruling line:** `0107(a): the grant lives in the adapter, as a gate_grants list beside routes and judges.`

### (b) Whether a grant may name the review chief

**Recommended: no.** A scoped route serves the non-chief gate offices — security review, review, verify, ship. The chief is the seat that rules on the other seats' findings; its own judgment is the last check. A model that is not trusted adapter-wide should not be the last check, and a chief on an untrusted route would make the panel's advisory pattern impossible to reason about. The chief stays a trusted adapter's judge (0041 ruling 3). A later wager may promote the route for the chief office; that is a new operator ruling expanding this grant's office enum, not a use of the present grant, and it is not available in this decision. The compile refuses a grant that names `chief`, and refuses a chief site even if a grant lists its agent.

**Alternative, not recommended:** allow the chief. It would let the downstream case replace its trusted chief, but it would also make the untrusted route the final word on every finding in the panel, and it would collapse the distinction between an advisory member and the check. The residual risk is exactly the one 0021's context names: "gate seats ARE the check, and nobody stands behind the judges" (`docs/decisions/0021-model-policy.md:16-22`).

**Ruling line:** `0107(b): no; a grant names no chief office.`

### (c) Whether a grant expires, or is re-proved when the harness release, the route's target model or the hands composition changes

**Option c1 — bound to the facts, no clock.** Recommended. The grant names the measured release, the concrete model id and the hands composition (the box, the boundary and the adapter's measured boxed hands shape). The compile refuses a release the adapter no longer measures, a concrete id the alias no longer maps to, and a grant whose office or route has moved. Any change re-opens the grant; the operator records a new evidence line. This is stronger than a calendar against a changed release, because the dangerous change is effective immediately, not at the expiry date.

**Option c2 — a calendar expiry, in 0025's manner.** 0025 ruling 2 makes an expiry mandatory for Skírnir's operator-signed grant, "a grant without one is invalid" (`docs/decisions/0025-skirnir.md:43-51`). A grant that expires after a stated period, and is re-proved at expiry, catches a grant whose facts have not moved but whose evidence is old. The cost is that a stale grant stays live until its date, and a release can change the day after it is signed. 0025's shape is cited only for the expiry idea; its grant is a different grant, for the standing executor (#476), and its signature and ceiling are not imported here.

**Recommended: c1, with a recorded review date.** The grant is invalidated by any change to its three facts; the operator records a review date in the evidence line, and `brokkr doctor` prints it, but a missed review does not keep a changed fact admitted. If the operator prefers a hard expiry as well, c2's date can be added without changing c1.

**Ruling line:** `0107(c): bound to the measured release, the concrete model id and the hands composition; any change re-opens it, with a recorded review date.`

### (d) The evidence bar

**Option d1 — the full bar.** Recommended:
- the wager under 0075 ruling 6 on real stories, comparing the route and model with the incumbent on cost per accepted change (#224) and on review holds and rounds (#436), with its run ids, including at least one story in the office the grant names;
- the conformance probe's report for the harness release (0075 ruling 3), with the version, host, date and evidence;
- the hands composition measurement (#896 for the measured release; 1052a for `0.2.2-rc.1`), on the route and concrete model the grant names: the composed tree's measured rows, one `dsh-mcp-client` and Brokkr's workspace tool, the harness's own tools off, the box, and the read-only access;
- the route's declared egress class and jurisdiction (0036), named; the grant is not an egress grant;
- the operator's evidence line, recorded in the ruling as 0021 ruling 3 requires, and the promotion as 0075 ruling 6's roster addendum.

**Option d2 — the probe and hands measurement, without a wager.** Cheaper, and enough to show the harness can run boxed; not enough to show the model's judgment is worth a gate. A preview model with no wager is exactly the case 0075 ruling 5 keeps provisional; d2 would let a model into a gate on a harness measurement alone.

**Option d3 — the operator's judgment alone, with a stated reason.** The law's floor is an operator ruling with an evidence line (0021 ruling 3); a ruling with no evidence line is not one. Not recommended.

**Recommended: d1.** The wager is what makes the model's judgment a measured claim rather than an impression; the probe and the hands measurement are what make the harness and the box measured. A grant with any of the three missing is refused by the compile for the missing data, and the operator's ruling is the place the evidence line lives.

**Ruling line:** `0107(d): the full bar — wager on real stories, probe report, hands composition on the named route and model, egress class named, operator evidence line.`

### (e) Whether compile should refuse the second-adapter workaround

**Option e1 — refuse two adapters that name the same driver binary at different tiers.** Recommended. `Adapters::load` (or the compile's adapter load) refuses the pair by name, with both files and both tiers, before any gate is judged. A repository can still copy the binary under another name, but that is a new provider with a new binary, visibly not the same adapter, and the guidance says not to. The check closes the exact workaround the issue names and makes the compile's refusal a property of the tree, not a convention.

**Option e2 — guidance alone.** Simpler, no false positives, and no new refusal. But the workaround is invisible: the second adapter passes every existing check, and the only record is a second file. The issue's downstream has already found it; guidance that the compile does not enforce is the weaker answer.

**Recommended: e1.** The refusal is a name check, not a proof; it cannot see a copy or a wrapper, and the residual is named in the consequences. It does not need a new contract: the adapter files are library data, and the refusal is a compile error naming both files.

**Ruling line:** `0107(e): yes; compile refuses two adapters naming the same driver binary at different tiers, naming both files.`

## What the older decisions say after acceptance

These are the replacement sentences, kept here so the older files are not edited. Each takes effect when this decision is accepted; the header pointers and back-pointers are added then, not in this story. No sentence in 0036, 0043, 0046, 0065 or 0075 changes; the grant is built within them, and the paragraphs below say why.

### 0021 ruling 2

The paragraph currently reads (`docs/decisions/0021-model-policy.md:37-42`):

> **Drivers carry an operator-granted trust tier.** The policy names tiers, not vendors: a driver's tier is data the operator rules into the bundle, never a constant in the engine. Gate seats require the trusted tier. **The bundle compiler refuses a bundle that seats an untrusted driver at a gate** — the refusal happens at compile time, before any prompt exists to leak, in the manner of a digest mismatch.

After acceptance it reads:

> **Drivers carry an operator-granted trust tier.** The policy names tiers, not vendors: a driver's tier is data the operator rules into the bundle, never a constant in the engine. Gate seats require the trusted tier, or a scoped gate grant that decision 0107 rules: an operator ruling, with an evidence line, naming one adapter, one route, one abstract model and its concrete id, and the non-chief gate offices it may hold. A grant is not a tier and is never adapter-wide; every gate link the grant does not name still requires the trusted tier. **The bundle compiler refuses a bundle that seats an untrusted driver at a gate** unless every condition of that grant holds, naming the missing condition — the refusal happens at compile time, before any prompt exists to leak, in the manner of a digest mismatch.

### 0041 ruling 3

The two sentences that change currently read (`docs/decisions/0041-one-office-per-seat.md:201-204`):

> Each adapter declares `judges`: the abstract names that may hold a gate site. Compilation refuses a gate whose chain names any other model, on any link, naming the site and the link.

After acceptance they read:

> Each adapter declares `judges`: the abstract names that may hold a gate site. Compilation refuses a gate whose chain names any other model, on any link, naming the site and the link, unless decision 0107's scoped gate grant names that link's route, abstract model and concrete id for that office; a grant is a separate list and is never written into `judges`.

The rest of the paragraph stands, including the run-time sentence, the shipped list (`claude`, `codex`, `lanetally`, and dsh and exec none), and the `chief-architect` sentence. A grant does not change a shipped `judges` list.

### 0036 does not change

0036 ruling 3's sentence stands: "For the same reason `local` grants no gate seat, no trust tier and no fallback standing" (`docs/decisions/0036-egress-is-a-property-of-the-route.md:128-129`). A 0107 grant is not a route class. It names the route as its scope, and the route's class is read and reported; `local` is not evidence, and a local route with a grant is admitted on the same wager and box as any other. 0036's `routes` map is not extended with a trust tier; the grant is a separate list in the adapter, and the egress axis stays 0036's.

### 0075 ruling 5 does not change

"It never holds a gate (a review chief) or the sole implementer of main-bound work until promoted" (`docs/decisions/0075-harness-onboarding-in-a-day.md:150-153`) stands exactly. The compile reads the provisional tier before the grant, and a provisional model is refused at a gate whatever the grant says. A new model starts provisional under this ruling, so the adapter writes the tier for the preview until it is promoted; an absent tier reads promoted only for the models that predate the tier. A grant on a promoted model is the promotion's companion, not a substitute for it.

### 0043, 0046 and 0065 do not change

0043 ruling 3's boxed `exec` exemption (`docs/decisions/0043-the-hands-are-one-tool.md:118-124`) and 0046 ruling 4's boundary table (`docs/decisions/0046-the-boundary-is-named.md:179-191`) are the grant's conditions, not its objects. 0065's realm grants (`docs/decisions/0065-capabilities-are-the-realms-to-grant.md:122-146`) remain the only capability grants; a 0107 grant is not one.

## Enforcement binding (named, not built)

Nothing in this section is built. It is the commission for 0107a and 0107b; the `Built:` marker stays `unbuilt (#1052)` until both land.

### The data fields

A new top-level `gate_grants` list on the adapter, beside `routes` and `judges` (`crates/brokkr-runtime/src/agents.rs:396-437`):

```json
"gate_grants": [
  {
    "route": "dashscope",
    "model": "qwen-max",
    "concrete": "dashscope/qwen3.8-max",
    "offices": ["security-review"],
    "release": "0.2.2-rc.1",
    "evidence": "#1052, 1052a and the promotion addendum"
  }
]
```

The typed shape, in a new `crates/brokkr-runtime/src/agents/load/gate_grant.rs` or an addition to the adapter loader:

- `GateGrant { route: String, model: String, concrete: String, offices: Vec<GateOffice>, release: String, evidence: String }`, parsed once at the edge with `deny_unknown_fields` (0071 ruling 3). An absent `gate_grants` is the empty list, so every adapter on disk reads exactly as it does today.
- `GateOffice` is a closed enum with `Review`, `SecurityReview`, `Verify`, `Ship` and `Chief`, written `review`, `security-review`, `verify`, `ship` and `chief`. `Chief` exists so the loader can refuse it by name; a grant may not contain it.
- `0107a` maps a gate site's office through one table in the gate law, read from the site's role and `Candidate.agent` (`crates/brokkr-runtime/src/agents.rs:738-740`): a panel's `chief` slot, and the agents `review-chief` and `chief-architect`, are `Chief`; `review-security` is `SecurityReview`; the other review agents are `Review`; `verifier` is `Verify`; `shipper` is `Ship`. An agent or role no row names is refused at a gate rather than guessed; the table is the one home of the office vocabulary (0071 ruling 5), and a roster test keeps it in step with 0041's shipped list.
- `route`, `model`, `concrete`, `release` and `evidence` are non-empty; `offices` is non-empty and deduplicated. A repeated office, an unknown office, a route that is not the concrete id's prefix, or a model not in `models` is a load refusal naming the adapter and the grant.
- The grant is part of the adapter's bytes, so it is digested with the adapter and moves every bundle that pins it. It is not a wire contract and mints no contract version.
- The `evidence` string is a pointer for doctor and for the ruling's record, not a proof; the compile cannot read a ruling, and the operator's recorded ruling is the authority 0021 ruling 3 requires.
- The loader refuses a grant whose adapter is already `Trusted`, and a grant that names a second adapter's route or model, so one fact has one home (0071 ruling 5).

### The new refusals in `route_policy.rs`

`judge_link` gains a grant lookup before `refuse_untrusted_gate` (`crates/brokkr-runtime/src/bundle/route_policy.rs:29-45`). The lookup returns a typed `GateGrantRefusal` (a `thiserror` enum in `crates/brokkr-runtime/src/bundle/grant.rs`, whose `Display` is the operator's text; 0071 ruling 8), and `judge_link` converts it to the compile's existing `Invalid` carrier. One variant per missing condition, in this order:

1. **No grant for this link.** Falls through to the present tier and judges refusals, unchanged. If the adapter has a grant for the route but none for the model, the refusal names the model; if it has one for the route and model but not the office, it names the office.
2. **`GrantRoute`:** "seat '{what}' gate link {link} names route '{route}', which no `gate_grants` entry for adapter '{adapter}' admits; a gate link without a grant requires the trusted tier (decision 0021 ruling 2) and a `judges` entry (decision 0041 ruling 3)."
3. **`GrantConcrete`:** "seat '{what}' gate link {link} seats model '{model}' as '{actual}', and the grant names '{granted}'; a grant is bound to the concrete model id, and a re-pointed alias re-opens it (decision 0107 ruling 2)."
4. **`GrantProvisional`:** "seat '{what}' gate link {link} seats model '{model}', which adapter '{adapter}' declares provisional; a provisional model never holds a gate, whatever the grant says (decision 0075 ruling 5)."
5. **`GrantOffice`:** "seat '{what}' gate link {link} holds the {office} office, which the grant for route '{route}' and model '{model}' does not name; a grant names its offices and never the chief (decision 0107 ruling 4)."
6. **`GrantUnboxed`:** "seat '{what}' gate link {link} is not boxed with workspace hands (boundary '{boundary}', hands {presence}); a scoped gate grant holds only a boxed workspace-hands seat under namespace, seatbelt or container (decision 0107 ruling 5; decision 0046 ruling 4)."
7. **`GrantNetwork`:** "seat '{what}' gate link {link} declares hands with network on; the grant requires a no-network box, because dsh's native egress is unmeasured (decision 0107 ruling 5; decision 0065 ruling 4)."
8. **`GrantRelease`:** "seat '{what}' gate link {link} names release '{granted}', and adapter '{adapter}' measures its boxed hands shape on '{measured}'; a grant is bound to a measured release, and #{issue} must measure the one it names (decision 0107 ruling 5; decision 0075 ruling 3)."
9. **`GrantFallback`:** "seat '{what}' gate link {link} has no grant of its own; every fallback link qualifies on its own, and a grant is not copied from the first link (decision 0107 ruling 6; decision 0021 ruling 5)."
10. **`GrantChief`:** "seat '{what}' gate link {link} holds a chief office; a scoped gate grant never names the chief (decision 0107 ruling 4, ask (b))."
11. **`GrantSecrets`:** the existing `refuse_secret_egress` is not bypassed; a grant does not lift the egress bar. The message is 0036's, unchanged (`crates/brokkr-runtime/src/bundle/route_policy.rs:155-165`).

The refusal order matters: the grant's condition is named before the tier, so an operator learns what the grant is missing rather than "untrusted"; a link with no grant gets the present tier refusal, so the default never regresses. A grant whose adapter is already trusted is a load refusal, not a gate refusal.

### Doctor's readout of each grant

A new `crates/brokkr-cli/src/doctor/grant.rs`, modelled on `crates/brokkr-cli/src/doctor/provisional.rs:1-55` (one line per grant, or one line saying there is none):

- each grant's adapter, route, abstract model, concrete id, offices, release and evidence reference;
- its status: `admitted` when every condition holds in the loaded tree; `alias moved` when `models[model]` differs from `concrete`; `provisional` when the model is in `provisional`; `unmeasured release` when the adapter has no boxed hands shape on that release; `unboxed` when no site is boxed; `chief` when the office is chief;
- the route's egress class, including `uncontracted` for a route not in `routes` (`crates/brokkr-runtime/src/agents/load.rs:368-380`), and the fact that the grant changes no egress or secret clearance;
- the measured release and host the evidence stands on, so a Linux measurement is not read as a macOS one.

Doctor reports; the compile refuses (the same split as `doctor/provisional.rs`). The standing is derived once by a pure `GrantStanding` function, and the compile's refusal and doctor's line both read it; the CLI only paints the line (0071 ruling 7).

### The tests a build slice must add

Each test is written to bind: the slice records a compiling change that removes the check it proves, the test fails, the change is restored, and the failing test is named in the slice's result. The build slice adds the tests; this decision alone adds none.

- `a_scoped_grant_admits_a_boxed_promoted_route_at_the_named_office` — the positive control: the dsh adapter with a grant on `dashscope/qwen3.8-max`, a boxed workspace-hands `security-review` seat, and a released measured shape compiles.
- `an_unboxed_grant_seat_is_refused_naming_the_box` — the same seat with no `hands`.
- `a_harness_or_open_boundary_grant_seat_is_refused` — `harness` and `open` each refuse; `namespace`, `seatbelt` and `container` admit.
- `a_network_on_grant_seat_is_refused` — `hands.network: true` refuses.
- `an_inline_gate_with_a_grant_is_refused` — an inline site has no office; the grant never applies, and the tier and judges refusals stand.
- `a_provisional_model_grant_is_refused` — the model in `provisional` refuses before the grant is read; the refusal names 0075 ruling 5.
- `a_repointed_alias_is_refused_naming_both_ids` — `models[model]` moved from the granted id; the refusal names both.
- `a_grant_on_a_route_the_concrete_id_does_not_reach_is_refused` — the prefix mismatch.
- `a_grant_naming_the_chief_is_refused` — `offices: ["chief"]` is a load refusal; a chief site with a grant for its agent is a compile refusal.
- `a_fallback_link_without_its_own_grant_is_refused_naming_the_link` — first link granted, second untrusted and ungranted; the second link's number is in the message.
- `a_grant_does_not_lift_the_egress_bar` — a granted seat that declares secret bindings still hits 0036's refusal.
- `a_grant_does_not_change_the_judges_or_the_tier` — a non-granted model on the same adapter still hits 0041 ruling 3 and 0021 ruling 2, unchanged.
- `the_second_adapter_workaround_is_refused` — two adapters, one binary, different tiers; the load refusal names both files (if ask (e) is ruled e1).
- `an_unknown_grant_field_is_refused_by_name` — `deny_unknown_fields`.
- `the_grant_is_in_the_adapter_digest` — the adapter's bytes and the bundle's identity move.
- `doctor_names_every_grant_and_its_missing_condition` — the readout for admitted, alias-moved, provisional, unmeasured-release and chief.

### The slices

**0107a — the data and the compile.** Files: `crates/brokkr-runtime/src/agents.rs` and `crates/brokkr-runtime/src/agents/load/adapter.rs` (the `gate_grants` field and its parse), a new `crates/brokkr-runtime/src/agents/load/gate_grant.rs` (the typed `GateGrant` and `GateOffice`), a new `crates/brokkr-runtime/src/bundle/grant.rs` (the lookup and `GateGrantRefusal`), `crates/brokkr-runtime/src/bundle/route_policy.rs` (the lookup before the tier and judges), `crates/brokkr-runtime/src/bundle.rs` (the per-link call and the site-office derivation), and the tests in `crates/brokkr-runtime/src/bundle/model_policy_tests.rs` or a new `crates/brokkr-runtime/src/bundle/tests/grant.rs`. About 180 production and 220 test lines. No `adapters/dsh.json` change in this story; the operator's grant is data for a later slice, and the tests use fixtures. No contract version; the adapter data is digested with the adapter.

**0107b — doctor and the guide.** Files: a new `crates/brokkr-cli/src/doctor/grant.rs`, `crates/brokkr-cli/src/doctor.rs` (the section call), `crates/brokkr-cli/src/doctor/provisional.rs`'s test style for the readout, and a section in `docs/guides/provider-adapters.md` that says what a grant is, what it is not, and how to read doctor's line. About 80 production and 100 test lines, plus the guide prose. The `Built:` marker moves to `built` only when both slices land; the guide is not `docs/guides/agent-library.md`, which #1037 owns.

**Order.** 0107a first; 0107b after it. 1052a's measurement is a prerequisite for a real `0.2.2-rc.1` grant, not for the compile: the compile can be built and tested against fixtures, and it refuses the real release until 1052a updates the adapter. 1052b (the per-recipe patch) is independent; it seats whatever the grant admits, and this story does not touch it.

## Consequences

- **What changes for a bundle that uses no grant:** nothing. An adapter without `gate_grants` loads as today; the tier and judges path is unchanged; the digests of every current bundle do not move.
- **What changes for a bundle that uses one:** the adapter's bytes and the bundle's identity move; the compile reads one more list; the doctor prints one more section. A grant is visible in the manifest through the adapter digest, and its refusals name the condition.
- **What does not change:** `trust_tier` and `judges` keep their meanings; 0036's egress map and classes; 0043's box; 0046's boundary; 0065's capability grants; 0075's provisional tier and promotion road; the wire protocol. No contract version is minted. No adapter, recipe or realm is edited in this story.
- **What it costs:** one more adapter list, one more compile path, one more doctor readout, and one more thing for the operator to keep true. The grant's evidence is a point-in-time measurement, and the residual risks above are the price of admitting a model that the blanket tier would have admitted for the wrong reason.

## Disclosure

In 0021 ruling 8's manner: this draft was written by a seat running on dsh (DeepSeek Flash, DeepSeek's platform), unboxed with dsh's own tools, at the operator's word (2026-10-26). That is a conflict of interest wherever dsh's route and model are discussed: the draft recommends a grant that would let a dsh route hold a gate, and the seat writing it is on that provider. The conflict is why the recommendation keeps the adapter untrusted, requires an independent wager and a measured release, names the residual risks, and leaves every choice to the operator. It is also why the interim answer is the work-class panel member under a trusted chief, not a grant on this draft's authority. The draft is reviewed by other seats; their findings and the operator's ruling, not this seat's provenance, are what make the grant law.
