# 0107 — A scoped gate grant: one route and model may hold a named gate role, on measured evidence

Status: proposed
Date: 2026-11-11

Built: unbuilt (#1052)

This decision relies on 0021's rulings 1, 2, 3, 4, 5, 7 and 8, on 0036's rulings 1, 2 and 3, on 0041's ruling 3, on 0043's rulings 2 and 3, on 0046's rulings 1 and 4, on 0065's rulings 3 and 7, on 0075's rulings 3, 4, 5 and 6, and on 0098's rulings 2 and 3. Two sentences change on acceptance: 0021 ruling 2 and 0041 ruling 3. Their exact replacements are in [What the older decisions say after acceptance](#what-the-older-decisions-say-after-acceptance), and no older file is edited here.

This is a document only. No adapter, recipe, realm, agent or code file changes in this story. The grant described here is unbuilt; the fields, refusals and tests below are its commission, not its enactment. Until it is built, the answer to the ask is option A, below.

## Context

The issue is #1052, ask 3, verbatim:

> Gate seats (a security lane, a judge) refuse an untrusted provider, and dsh is untrusted. The only workaround found is a second adapter marked trusted and listed as a judge, which is a blanket trust grant rather than a scoped one. Ask: a documented, scoped way to admit a specific dsh route and model to a specific gate seat, with the measurement or evidence that has to back it. Alternatively, guidance that this should stay impossible, so downstream doesn't build the workaround.

The downstream case is concrete: a DashScope route on dsh, a Qwen Max preview model, serving a workspace-hands security review lane. In this tree that route's abstract model is `qwen-max`, mapped to `dashscope/qwen3.8-max` (`adapters/dsh.json:37`); `dashscope` is not in dsh's `routes` map (`adapters/dsh.json:5-8`), so 0036 ruling 1's absent-declaration default reads it `uncontracted` (`docs/decisions/0036-egress-is-a-property-of-the-route.md:96-104`; `crates/brokkr-runtime/src/agents/load.rs:368-380`). dsh's adapter declares `"trust_tier": "untrusted"` and `"judges": []` (`adapters/dsh.json:3`, `:46`). The compile therefore refuses the seat at the gate before any prompt exists: `refuse_untrusted_gate` reads the tier (`crates/brokkr-runtime/src/bundle/route_policy.rs:48-64`) and `refuse_unjudged_model` reads the judges list (`:66-95`), on every link of a chain (`:29-45`; `crates/brokkr-runtime/src/bundle.rs:2410-2445`).

### The two existing answers

**A work-class panel member under a trusted chief.** The controller's library seats two dsh models as panel members, `bugfix-review-adversarial-dsh` and `bugfix-review-correctness-dsflash`, each `"class": "work"`; the panel's chief is `"class": "gate"` on Claude Fable 5.2, with Opus 5.5 as its fallback (the library text is quoted in this story's commission). This is lawful today: 0021 ruling 1 divides work from gate, 0021 ruling 7 lets a newcomer take work seats freely, and 0041 ruling 3 refuses only a gate link. The panel chief rules; the untrusted member contributes findings, not the verdict. The shipped panel-review recipe is not the same shape — it seats two gate-class members and no chief (`recipes/panel-review/bundle.json:44-54`) — but a work-class member beside a gate-class chief is the existing answer for an untrusted one. An untrusted route can therefore contribute at a gate today, as an advisory member a trusted chief rules on. It cannot be the gate.

**A second adapter marked trusted.** A second adapter file over the same `dsh` driver declares `"trust_tier": "trusted"` and lists the model in `judges`; the gate link then passes both checks. This is the workaround the issue names. It is a blanket grant: one line trusts the whole binary, every route and every model it can reach, now and in future route additions. The copy cannot even keep the model's name: `Adapters::load` refuses one abstract model mapped by two adapters (`crates/brokkr-runtime/src/agents/load.rs:649-663`), so the workaround also renames the abstract model and every agent that hires it. It duplicates the driver, the models map, the credentials and the tool grammar to express a fact about one route and one model, which 0036 already rejected for egress: "Splitting `dsh` into one adapter per route duplicates the binary, the models map and the tool grammar to express a fact about endpoints, and misstates what an adapter is: a provider is a CLI, and this CLI genuinely has one" (`docs/decisions/0036-egress-is-a-property-of-the-route.md:63-66`). Guidance should say not to build it, for the reasons in option E below.

### Why the adapter is too coarse a unit of trust

Decision 0036 moved egress from the adapter to the route because dsh's one declaration fronts three destinations: a local SGLang server on the operator's own DGX Spark, `token-plan.ap-southeast-1.maas.aliyuncs.com` in Alibaba's Singapore region, and the profile's default provider route (`docs/decisions/0036-egress-is-a-property-of-the-route.md:34-44`). The same table is the argument here, on a different axis:

| dsh route | endpoint | where the prompt goes | what the evidence is about |
|---|---|---|---|
| `spark/*` | an SGLang server on the operator's network | the operator's own DGX Spark | local hardware; no third party |
| `dashscope/*` | `token-plan.ap-southeast-1.maas.aliyuncs.com` | Alibaba, Singapore | a third party and a jurisdiction |
| unprefixed (`deepseek-v4-pro`, …) | the profile's default provider route | whatever that profile resolves | an unknown default |

One `trust_tier` line at `adapters/dsh.json:3` answers for all three. But a gate's trust is not one fact: it is the harness release that mediates the prompt, the route's provider and jurisdiction, the abstract model's judgment, the role the seat holds, and the box around the hands. They have different evidence, and a route or a model can change while the others stand. The adapter is the right unit for a *tier* — 0021 ruling 2's one grant that a driver may judge — but it is the wrong unit for an exception that names one route and one model. The grant below borrows 0036's route granularity without moving trust onto the route.

### The law as it stands

| Question | Ruling | Text and line |
|---|---|---|
| Which seats are gates? | 0021 ruling 1 | "Work seats … produce output the machine checks. Gate seats (chief, review, security review, verify, ship) are the check." (`docs/decisions/0021-model-policy.md:32-35`) |
| Who may be a gate? | 0021 ruling 2 | "Gate seats require the trusted tier. **The bundle compiler refuses a bundle that seats an untrusted driver at a gate**" (`docs/decisions/0021-model-policy.md:37-42`) |
| Is the tier earned? | 0021 ruling 3 | "Tiers are earnable, in both directions. A promotion or demotion is an operator ruling recorded where rulings live, and its evidence line cites the scorecard" (`docs/decisions/0021-model-policy.md:44-48`) |
| Is trust the same as egress? | 0021 ruling 4 | "Trust to judge and clearance to receive are different grants." (`docs/decisions/0021-model-policy.md:50-55`) |
| What about a newcomer? | 0021 ruling 7 | "Every driver without a journaled track record here starts identically … no gate seat … Grants and gates open to any driver only by ruling 3's road: evidence, then an operator's recorded ruling." (`docs/decisions/0021-model-policy.md:68-77`) |
| Which abstract models may judge? | 0041 ruling 3 | "Each adapter declares `judges`: the abstract names that may hold a gate site. Compilation refuses a gate whose chain names any other model, on any link, naming the site and the link. … dsh and exec none." (`docs/decisions/0041-one-office-per-seat.md:201-208`) |
| Does egress belong to the route? | 0036 rulings 1 and 2 | "`uncontracted` is everything else, and is the value of an absent declaration" (`docs/decisions/0036-egress-is-a-property-of-the-route.md:96-99`); "A concrete model id of the form `<route>/<model>` resolves to that route; an id with no prefix resolves to the adapter's own declared class and no better" (`:106-120`) |
| Does local confer trust? | 0036 ruling 3 | "A route is `local` because of where its endpoint runs … For the same reason `local` grants no gate seat, no trust tier and no fallback standing." (`docs/decisions/0036-egress-is-a-property-of-the-route.md:122-127`) |
| Does a new model start at a gate? | 0075 ruling 5 | "A new harness, or a new model on an existing one, starts provisional. … It never holds a gate (a review chief) or the sole implementer of main-bound work until promoted." (`docs/decisions/0075-harness-onboarding-in-a-day.md:148-153`) |
| How is it promoted? | 0075 ruling 6 | "A provisional seat is promoted after a wager (#237) on real stories. The wager compares it with the incumbent on cost per accepted change (#224) and on review holds and rounds (#436)." (`docs/decisions/0075-harness-onboarding-in-a-day.md:158-161`) |
| May a boxed command judge? | 0043 ruling 3 | "A deterministic command has no stochastic axis for a trust tier to measure; its blast radius is the box." (`docs/decisions/0043-the-hands-are-one-tool.md:118-124`) |
| Which boundaries hold a gate? | 0046 ruling 4 | "`namespace`, `seatbelt` and `container` hold any gate … `open` never holds a model gate. An exec gate under `harness` or `open` is refused as 0021 reads" (`docs/decisions/0046-the-boundary-is-named.md:179-191`) |
| Who grants a capability? | 0065 ruling 3 | "The realm grants, and only the realm. … A recipe cannot grant." (`docs/decisions/0065-capabilities-are-the-realms-to-grant.md:122-146`) |

Two addenda matter to the argument. The 2026-09-02 codex addendum shows the promotion road actually used: a wager, a recorded operator ruling, and the tier enacted as data (`docs/decisions/0021-model-policy.md:97-110`). The second addendum names a different door: "No gate-class agent is: a judge whose shell could reach an untrusted driver would take that driver's word through a door the compile-time tier check of ruling 2 cannot see" (`docs/decisions/0021-model-policy.md:112-127`). That door is a gate-class agent's `tools.allow` grant of another driver's binary, and a 0107 grant opens none: it grants no tool, no shell, no capability and no egress. The door a 0107 grant does open is the granted seat's own harness and box, and conditions 5 to 7 are what bound it.

### Two words that sound alike: `office` and gate role

Today the tree gives `office` one meaning: the hired agent's name. `tier.rs:9` says "An office is an agent, by name"; `SiteAsks.office` is "the agent's name, or an inline site's authoring label" (`crates/brokkr-runtime/src/capabilities.rs:260-261`); 0065's `offices` list is an array of agent names (`docs/decisions/0065-capabilities-are-the-realms-to-grant.md:131`, where `review-security` is the agent file `agents/review-security.json`); and 0075 ruling 5's `provisional_offices` reads the same list (`docs/decisions/0075-harness-onboarding-in-a-day.md:148-156`).

0021 ruling 1's list (`chief`, `review`, `security review`, `verify`, `ship`) is a different fact: the position a site occupies in a body, and whether a later site rules on its findings. This draft calls that fact a **gate role**, and keeps `office` for the agent's name. The two are bound, not merged:

- A site's role is derived from its `SiteKey` (`crates/brokkr-runtime/src/bundle/site_key.rs:75-207`: single body, panel member, sequence step, sequence-panel member), the seat (`review`, `verify`, `ship`), its position in the sequence, and, for the security split only, the hired agent's `office`.
- The recipe's panel member key (`correctness`, `security`, …) is recipe data. It is neither the office nor the role: `site_key.rs:94-108` carries it as `member`, and `recipes/panel-review/bundle.json:45-54` chooses it. A recipe cannot grant (0065 ruling 3), so the member key cannot decide a grant either.
- An agent's office is its name; `agents/review-security.json` is the office `review-security`. The role `security-review` is a review site whose hired office is `review-security`. A `security` slot hiring `review-correctness` is role `review`, not `security-review`; a `correctness` slot hiring `review-security` is role `security-review`, because the office, not the slot, carries the security intent. A custom agent that is no roster office holds its own office and takes role `review` at a review site; it never reads `security-review` without the operator naming it so.
- `chief` is structural: the last step of a `review` sequence, when that step is a single gate-class site (a named agent or an inline command) and at least one earlier step exists. This rule takes precedence over the office split for that site. That is the work-panel-then-chief shape the controller runs (`recipes/gpt-flash/bundle.json:89-113`: a work-class `positions` panel, then a gate-class `chief`) and the gate-panel-then-chief shape. A panel member keyed `chief` inside a one-step panel is not a chief: it rules on no other step, and it is a `review` site. A gate-class step before the last is its `review` or `security-review` role, not chief; it may hold that role but not the chief's.
- Every other gate site has no role, so a grant never applies and 0021 ruling 2 and 0041 ruling 3 stand.

Ask (g) asks the operator to rule this vocabulary; the recommended option is `gate role`, keeping `office` as the agent's name.

## The trust subjects, named separately

A gate's verdict passes through five subjects. They are not interchangeable, and today's compile reads only two of them.

| # | Subject | What it decides | Where it lives today | The gap |
|---|---|---|---|---|
| i | The harness: its code and plugins, at a release | Whether the prompt and the verdict are carried faithfully; whether the harness's own tools and ambient configuration are off | The adapter's measured `mcp.shapes` and `hands` fragment (`adapters/dsh.json:60-162`); the composed-tree check and its measured 99 rows (`docs/decisions/0098-a-dsh-hands-seat-runs-on-its-composed-tree.md:40-58`; `docs/evidence/adapters/dsh-hands-round2.md:9-39`); 0075 ruling 3's probe (`docs/decisions/0075-harness-onboarding-in-a-day.md:116-129`) | `trust_tier` is one line for every release. The adapter's cold boxed shape measures dsh `0.2.1-rc.2` (`adapters/dsh.json:111-131`); 1052a would measure `0.2.2-rc.1` (#1052). The driver's own set admits `0.2.1-rc.2` and `0.2.1-rc.3` (`crates/brokkr-protocol/src/adapters/dsh_hands.rs:56`), so today an rc.2 shape can run on an installed rc.3. There are two homes for the release, and nothing binds them to agree. |
| ii | The route's provider and jurisdiction | Where the seat's material goes, and whose law and outage it meets | 0036's `routes` map and resolver (`crates/brokkr-runtime/src/agents.rs:411`; `crates/brokkr-runtime/src/agents/load.rs:368-380`); dsh declares only `spark` and `spark-glm` local (`adapters/dsh.json:5-8`) | 0036's class is clearance to *receive*, not trust to *judge* (0021 ruling 4). There is no route-level trust, and this decision does not create one. The route's endpoint is profile data, not adapter data. |
| iii | The abstract model | Whether the model's judgment has been measured and is allowed to hold a gate | 0041 ruling 3's `judges` (`crates/brokkr-runtime/src/agents.rs:437`); dsh declares none (`adapters/dsh.json:46`); 0075 ruling 5's `tier` (`crates/brokkr-runtime/src/agents/load/models.rs:24-65`) | The list is adapter-wide and all-or-nothing; dsh's empty list refuses every abstract model, and a name added to it would admit that model on every route. A bare model id reads promoted, so an absent tier cannot distinguish a grandfathered model from a new preview. |
| iv | The office held (the draft calls its structural position the gate role) | Which gate the seat is: `chief`, `review`, `security-review`, `verify`, `ship` | 0021 ruling 1's class, carried at the site as `class: gate`; 0041 ruling 2's roster (`docs/decisions/0041-one-office-per-seat.md:154-199`); the site's structural identity, `SiteKey` (`crates/brokkr-runtime/src/bundle/site_key.rs:75-207`); the hired agent's `office` (`crates/brokkr-runtime/src/capabilities.rs:260-261`) | A gate is one bit. There is no role on the grant, so a model admitted to one gate is admitted to every gate its agent is seated at. Deriving the role from the recipe's member key is wrong (the key is recipe data, and a recipe cannot grant); deriving it from the agent's name alone cannot see whether the site rules on another's findings. The role must bind the site's structure to the office, and `chief` must be structural. |
| v | The box | What the hands can reach when the verdict is produced | 0043's `hands`; 0046's `boundary`; `enforce_hands_boundary` (`crates/brokkr-runtime/src/bundle.rs:2553-2620`); dsh's boxed hands shape (`adapters/dsh.json:113-131`) | The gate check exempts only a boxed `exec` (`crates/brokkr-runtime/src/bundle/route_policy.rs:36-39`). A boxed model harness gets no credit for the box at a gate, even though 0046 ruling 4 already says which boundaries hold one. Access and binds are not part of the gate check at all. |

The five travel together. Today's `judge_link` reads the tier and the judges; it does not read the release, the route, the role, or the box, except to let a boxed `exec` pass. That is the right fail-closed default for an untrusted driver, and it is why a work-class panel member under a trusted chief is lawful today: the member is not the gate. A scoped grant adds the missing subjects one named exception at a time, and keeps the default for everything it does not name.

## The options

### (A) It stays impossible

The documented pattern is the work-class panel member under a trusted chief. The guidance says: an untrusted route may contribute findings to a panel; it may not be the panel's chief, a standalone review gate, a security gate, verify, ship, or any other gate. Do not build a second adapter to get around it; the reasons are option E's.

**Consequences.** The refusal stays exactly as 0021 ruling 2 writes it, with no new data, refusal, digest or readout. The downstream security lane keeps a dsh member and a trusted chief; the untrusted model never holds the verdict. The cost is that a model the operator judges qualified still cannot be the check, and the only road to that is a full tier promotion of dsh (0021 ruling 3), which trusts every route and model — exactly the blanket the issue refuses. Option A also leaves the second-adapter workaround as the only apparent way, so it needs the guidance and, if ask (e) is taken, a compile refusal to make the guidance real.

**Residual to name.** The work member still sends the worktree to DashScope, under 0036's egress class and 0065's capability grants; the chief still reads the member's findings, which may be wrong or prompt-injected. Those are the accepted costs of an advisory member, not a gate.

### (B) A scoped gate grant

The operator rules, with an evidence line (0021 ruling 3), that one (adapter, route, abstract model, concrete model id) may hold the named gate roles, and only when every condition below holds. The ruling lives where rulings live, as a dated addendum to this decision once it is accepted; the adapter data enacts it, as 0021 ruling 2 enacts a tier and 0075 ruling 6 enacts a promotion.

The conditions, all of them:

1. **The adapter is still untrusted, and the grant is not a tier.** `trust_tier` stays `untrusted`; `judges` stays empty; the grant is a separate list. The adapter is trusted for nothing else, on any route or model. A grant on an adapter already `Trusted` is a load refusal: a tier is the broader grant, and a scoped exception beside it would misstate which one admitted the seat.
2. **The route and the concrete model are named exactly.** The grant names a route, an abstract model and the concrete id. At load, the route must be the concrete id's prefix (0036 ruling 2) and the abstract model must be in `models`; at compile, `models[model]` must equal the grant's `concrete` exactly, or the grant refuses and names both ids. A route the concrete id does not reach is a load refusal. The route need not be declared in `routes`; if it is not, its class is `uncontracted` and the grant does not change that. A grant that has moved to another route or model refuses the same way, with both ids named.
3. **The model is explicitly promoted, never provisional.** The downstream Qwen Max preview is a new model, and 0075 ruling 5 starts a new model provisional. The adapter must write `tier: provisional` until 0075 ruling 6 promotes it; the compile reads the written tier (`crates/brokkr-runtime/src/agents/load/models.rs:24-65`) and the existing provisional gate refusal stops a provisional model before the grant is read (`crates/brokkr-runtime/src/bundle/tier.rs:78-89`). For a grant, absence is not enough: 0075 ruling 5's binding names `tier: provisional | promoted`, and 0107a makes the loader accept the written `promoted` word and records the declared tier once, as `Adapter.tiers: BTreeMap<String, ModelTier>` with `ModelTier::{Provisional, Promoted}`. That one map carries the three states — absent (legacy, reads promoted for every ordinary seat but cannot carry a grant), provisional, promoted — so the grant's requirement is not a second set beside the provisional one. An adapter that leaves the tier out still reads promoted for every ordinary seat, but it cannot carry a grant, so a bare `qwen-max` cannot become a gate by the accident of an absent field. Promotion is 0075 ruling 6's wager and roster addendum, not this grant. The grant and the promotion are two rulings; both are needed, and the grant's evidence line cites the promotion.
4. **The role is named, derived from structure and office, and is not a chief.** A gate site's **role** is derived from its `SiteKey` — the compiler's one structural identity: the seat, the selected case, and the body path (`crates/brokkr-runtime/src/bundle/site_key.rs:75-207`) — plus the hired agent's `office` (its name, `crates/brokkr-runtime/src/capabilities.rs:260-261`). It is never read from the recipe's panel member key, the charter, or a free step name. One table:
    - a single `review` seat, or a panel member of a `review` seat, whose hired office is `review-security`, is role `security-review`;
    - any other single `review` seat or panel member of a `review` seat is role `review`;
    - the last step of a `review` sequence, when that step is a single gate-class site (a named agent or an inline command) and at least one earlier step exists, is role `chief`, whatever body that single site wraps (a work-class panel followed by a gate-class chief, `recipes/gpt-flash/bundle.json:89-113`, and a gate-class panel followed by a chief are both this shape). This rule takes precedence over the office split for that site. A panel member keyed `chief` inside a one-step panel is not a chief: it rules on no other step, and it is role `review` or `security-review` by its office. A gate-class step before the last is its `review` or `security-review` role, not chief;
    - a single `verify` seat is role `verify`; a single `ship` seat is role `ship`;
    - every other gate site has no role, so a grant never applies and 0021 ruling 2 and 0041 ruling 3 stand. The clarify and analyze judges are such seats (`recipes/triage/bundle.json:84-90`, `:137-141`).

    The grant names one or more of `review`, `security-review`, `verify` and `ship`. The word `security-review` is the role 0021 ruling 1 calls "security review" (`docs/decisions/0021-model-policy.md:32-35`); it is distinct from the hired office `review-security` (`agents/review-security.json`), and the role is read only where that office is hired. The grant never names `chief`; ask (b) asks the operator to confirm that, and ask (f) asks whether a sole-check non-chief role may be named. The loader refuses a grant that writes `chief`, and the compile refuses a `chief` role even when the site's agent office appears in another grant, because the structural table, not the name alone, decides the role.
5. **The seat is boxed with workspace hands, on a measured release and host, with the office's declared read-only access and measured binds.** The site declares hands; the realm's boundary is `namespace`, `seatbelt` or `container` (0046 ruling 4), and the boundary is the realm's, not the recipe's (0046 ruling 1). The hands declare `network: false`; the hired office's declared `access` is `read-only` (0098 ruling 3); the site's whole hands spec — kind, network and binds — must digest to the value the grant names. Every explicit bind is `ro` or `overlay`; an `rw` bind is refused (`GrantBindMode`), because a grant is a read-only office and `rw` writes land on the host (0043 ruling 2). The grant names a release, and the compile requires the adapter's **cold, boxed** measured shape on that release and the boundary's host: `native_write`, `store_read`, `process_read` and `ambient` must all be `Measured` (`crates/brokkr-runtime/src/agents/mcp.rs:167-174`; 0075 ruling 3). The replacement boxed shape re-measures none of the ten hands rows (`adapters/dsh.json:133-154`) and does not qualify; the cold boxed shape is the one measured (`adapters/dsh.json:111-131`). A `seatbelt` boundary needs a macOS shape, and the adapter has only a Linux one, so a grant is admitted under `namespace` on Linux until a macOS shape is measured; `container` has no measured shape and is refused. The adapter's `hands.harness` is not a substitute: dsh declares none, and 0046 ruling 4 refuses it at a `harness` gate (`docs/decisions/0046-the-boundary-is-named.md:179-185`). A `network: true` box is refused: native egress is unmeasured on dsh (`adapters/dsh.json:157-158`), and the no-network box is the measured control.

   The grant is admitted only on a driver kind that carries an exact-release pin. The loader refuses a grant on any other kind (`GrantNoCarrier`), naming the driver and the adapter, so a grant cannot be written on an untrusted adapter whose driver cannot enforce the release at run start — `adapters/lanetally.json` writes no `trust_tier` and reads untrusted today, and its driver carries no pin. `dsh` is the one carrier today; a new carrier is a driver-code change under the harness's own module (0071 ruling 10), not adapter data.

   The grant's `release` selects the adapter's cold boxed shape, which is the compile-time home of the measured release. The driver's `MEASURED_VERSIONS` (`crates/brokkr-protocol/src/adapters/dsh_hands.rs:56`) is the run-start home. They are two homes, and 0107a adds a test that every release the adapter's cold boxed shape names is in the driver's set and every release the driver's set names has a cold boxed shape, so the two cannot drift silently. At run start the engine passes the granted release to the dsh driver as a private carrier, and the driver first refuses a version outside `MEASURED_VERSIONS`, then requires the observed `--version` to equal the granted release (`crates/brokkr-protocol/src/adapters/dsh_hands.rs:235-247`). The exact match is in addition to the measured set, not instead of it. An rc.2 grant therefore does not run on an installed rc.3; the exact-match refusal names both versions. 1052a adds `0.2.2-rc.1` to the adapter's measured shape and the driver's set; until it lands, a grant naming `0.2.2-rc.1` has no measured cold boxed shape and is refused by the compile.
6. **Every fallback link qualifies on its own.** A fallback link passes by the trusted tier and `judges`, or by its own grant; it never inherits an earlier link's grant. A link that follows a granted link and is itself untrusted and ungranted is refused at that link, naming it. This is 0041 ruling 3's "on any link" read at compile time, and it is already how `judge_link` is called per link (`crates/brokkr-runtime/src/bundle.rs:2410-2445`). A trusted-judge fallback is not refused by this condition.
7. **No secret-binding and no egress grant is implied.** The grant does not raise `egress_minimum`, does not stand in for 0021 ruling 4's binding grant, and does not grant a 0065 capability. The existing `refuse_secret_egress` still runs on the link (`crates/brokkr-runtime/src/bundle/route_policy.rs:99-167`), and a boxed site with secret bindings is refused by 0043 ruling 2's enforcement binding (`docs/decisions/0043-the-hands-are-one-tool.md:113-116`). A gate may hold an `egress` capability only where the realm names that office explicitly (0065 ruling 7); a 0107 grant is not that naming. The route's class, and the realm's capability map, are read and reported, not changed.
8. **The grant does not weaken the judge check for any other model.** A link with no matching grant and no earlier granted link still faces `refuse_untrusted_gate` and `refuse_unjudged_model` in their present order. A link that matches a grant on its abstract model is judged by the grant, and a matching grant that misses any condition is refused with that condition named, never silently fallen back to the tier refusal. An inline site has no agent and no office; a grant never applies to it, and it faces the tier and judges as today.

**Compile refuses everything else and names the missing condition.** The refusals are in `route_policy.rs`, one variant per condition, with the operator's text preserved and the decision's condition numbers cited. The exact messages are sketched in [Enforcement binding](#enforcement-binding-named-not-built).

**Consequences.** The downstream case can have a dsh Qwen Max security review gate, but only boxed, read-only, on a measured release and host, with the measured binds, an explicitly promoted model, a named non-chief role, every fallback qualified, and no secret or egress grant. The operator records the evidence once, in the ruling; the adapter carries the data; the compile refuses the rest. New data moves the adapter and bundle digests; no wire contract changes. The cost is a new adapter list, a new compile path, a new doctor readout, a new run-start carrier, and a new thing to get wrong. The benefit is that the exception names exactly what it admits, and the blanket workaround is unnecessary.

**Residuals to name.** A harness compromised between measurements; a provider that changes the model behind the same concrete id; a route's profile base URL re-pointed at another endpoint; a grant that outlives its evidence; a box that is measured on one host and runs on another; and the read-only worktree bind 0098 ruling 3 still owes. The recommendation below names each and what this decision does and does not do about it.

### (C) Adapter-wide trust for dsh

Set `"trust_tier": "trusted"` on `adapters/dsh.json`, and add the model to `judges`. The gate passes.

**Consequences.** One line trusts the binary for every route: the local Spark, DashScope in Singapore, the OpenRouter routes, the DeepSeek official route, and every route the operator adds tomorrow. A route re-pointed to a new provider inherits the trust without a ruling. The `judges` list is adapter-wide, so every model in it is admitted on every route, including a model that a future adapter update marks provisional (0075 ruling 5 still refuses that one, but the grant's scope is wrong even where it is refused). It breaks 0021 ruling 7's symmetry: codex earned `trusted` by a wager recorded in the ledger (`docs/decisions/0021-model-policy.md:97-110`); dsh would get it by one edit for one route. It collapses 0036's three-destination table into one answer, which is the mistake 0036 was written to end. The compile cannot tell an operator's deliberate trust from a workaround. Rejected.

### (D) A per-route trust tier

Add a trust tier to the `routes` map, so `dashscope: trusted` while `spark: untrusted`. This follows 0036's unit and is narrower than C.

**Consequences.** It is still model-blind, role-blind and office-blind: every model on `dashscope` — Qwen Max, GLM, DeepSeek, and any future alias — would hold every gate its agent is seated at, including a provisional model unless the provisional check also runs. It moves the judging axis into 0036's egress map, which is exactly the braid 0021's context untangles; 0036 ruling 3's "local grants no gate seat, no trust tier" would have to be read around a new field on the same map. It also gives a route-wide exception with no box, release, access, binds or model condition. It is a legitimate fallback if the operator wants a route-wide grant, but it is not the scoped grant the issue asks for. Not recommended.

### (E) A second trusted adapter

The workaround, named: a second adapter file over the same `dsh` driver, `trust_tier: trusted`, with the model in `judges`, seated at the gate, and the abstract model renamed because the loader refuses a name mapped twice (`crates/brokkr-runtime/src/agents/load.rs:649-663`).

**Consequences.** It is C in another file: the driver is trusted for every route and model. It duplicates the models map, routes, credentials and tool grammar, and it gives one driver two provider identities in the manifest, the doctor and the journal. A route added to the real adapter and not the copy is judged by a stale map; a route added to the copy and not the real one silently inherits trust. 0036 rejected one adapter per route for the duplication (`docs/decisions/0036-egress-is-a-property-of-the-route.md:63-66`); this is worse, because the copy also carries a different trust tier, and the renamed model strands every agent that hired the original name. Rejected. Ask (e) asks whether the compile should refuse the same-driver, different-tier shape so the guidance is not the only guard.

## The recommendation

**B is the ruled road. A is the answer until B is built.**

Against A alone: A is safe, and it is what the tree does today, but it answers the ask with "no" and leaves the downstream case with a work member forever. It also leaves the second-adapter workaround as the only way to get the model into a gate, which is the outcome the issue exists to prevent. A is the right interim because B and its prerequisite measurement (1052a) are not built; it is not the right final answer for an operator who is willing to record the evidence B asks for.

Against C: C is the blanket the issue refuses. It trusts three jurisdictions, every model and every future route for one route's evidence. The adapter is the right unit for a tier; it is the wrong unit for this exception.

Against D and E: D is narrower than C but still model-blind, role-blind, box-blind and release-blind; E is C with duplication and a renamed model. B is the only option that names the route, the model, the concrete id and the role, and that requires the release, the host, the office's declared read-only access and the binds, refusing each one it does not get.

Why B is lawful within the existing decisions, rather than beside them:

- 0021 ruling 2 gives the tier as the broad grant and the compile refusal as its enforcement; B is a named exception, recorded by the operator, and the amendment below writes it into that ruling rather than around it.
- 0021 ruling 3's road is evidence then an operator ruling. B uses that road for a route and a model instead of a driver; it does not create a second kind of evidence.
- 0021 ruling 4 keeps egress separate; B consumes the separation instead of merging it.
- 0021 ruling 5's no-substitution rule is a run-time park; B's per-link compile rule is 0041 ruling 3's "on any link", and it substitutes nothing.
- 0021 ruling 7 keeps newcomers symmetric; B is the same road for a newcomer, scoped, and a newcomer still starts with no gate.
- 0036's route is B's scope, not B's authority. `local` still confers nothing (0036 ruling 3); a local route with a grant is admitted on the same evidence as any other, and the route's class is read and reported, not trusted.
- 0041 ruling 3's `judges` stays the adapter-wide list; B is a separate list, and a grant is never written into `judges`.
- 0075 ruling 5's "never a gate" is untouched: the existing provisional gate refusal stops a provisional model before the grant is read. 0075 ruling 6's wager is the promotion B requires, and B adds the written `promoted` word its binding already names.
- 0043 ruling 3's boxed `exec` is the precedent that the box changes what a gate's trust has to cover; B borrows the box, not the exemption. A model has a stochastic axis, so the box is necessary but not sufficient; the promotion evidence is the other half.
- 0046 ruling 4 already names the boundaries that may hold a gate; B requires one of them and refuses `harness` and `open`.
- 0065 ruling 3 keeps the realm the only granter of capabilities; B grants none and leaves the realm's map in force.

**The residual risks, named.**

1. **A harness compromised between measurements.** The release pin and the probe are point-in-time. A different installed release is refused at run start by the exact-match carrier (condition 5), but a binary or plugin replaced under the same version is not detected, and a supply-chain compromise can carry the prompt or the verdict. This decision does not solve that; it requires the measured release, makes doctor re-read the installed version, and leaves the binary's own integrity to the operator's install and the host's package controls. A future decision could pin the harness binary by digest; that is out of this story.
2. **A route re-pointed at another model, or another endpoint.** The grant pins the concrete id, and the compile refuses an alias that no longer maps to it. But a provider can change the weights or the model behind the same published id; the concrete id is not a digest. Separately, the route name is adapter data, but the endpoint — dsh's profile base URL — is not: a profile edit can re-point `dashscope` at another host with every pinned fact unmoved (the measured evidence records the endpoint it saw, `docs/decisions/0036-egress-is-a-property-of-the-route.md:36-38`). The grant's evidence line names the endpoint and the date; any change to the id re-opens the grant, a change behind the id does not, and a profile re-point is caught only by re-reading the profile. Doctor can print the route's declared credential and class, not the profile's endpoint. The wager's run ids are the evidence, and a re-wager is the remedy.
3. **A grant outliving its evidence.** The grant is bound to the measured release, the concrete id, the host and the hands digest. Any of those changing re-opens it; the compile refuses a release the adapter no longer measures with the axes it measured, and a concrete id the alias no longer maps to. A grant whose evidence is stale but whose facts have not moved is not caught by a fact check; ask (c) asks whether to add a review date, and the recommendation is a recorded review, not a hard expiry.
4. **A box measured on one host and run on another.** 0098 closes the boxed dsh hands shape on Linux; macOS keeps a residual (`docs/decisions/0098-a-dsh-hands-seat-runs-on-its-composed-tree.md:3-5`). A grant used on macOS before the seatbelt measurement is not covered by the Linux evidence; the compile requires a measured shape on the boundary's host (condition 5), and doctor names the host the evidence stands on.
5. **The role derivation.** The grant names a structural role; the compile maps the site through `SiteKey` and the hired office. A `security` panel slot hiring `review-correctness` is role `review`, and a `security-review` grant refuses; a `correctness` slot hiring `review-security` is role `security-review`, and a `review` grant refuses; a single `review` seat and a panel member are both `review`. A non-final gate step of a `review` sequence is its `review` or `security-review` role, not chief, and is served; a gate step with no earlier site and no later chief is a sole check, which ask (f) rules. A custom agent that is no roster office is never `security-review`. A new gate agent with no role is refused rather than guessed. The residual is that `review` covers a single review seat and a panel member alike; ask (f) is where the operator rules whether a sole check may be named at all.
6. **The read-only worktree bind is owed.** 0098 ruling 3 says a read-only `HandsSpec` "does not exist in this round … the read-only hands bind stays owed" (`docs/decisions/0098-a-dsh-hands-seat-runs-on-its-composed-tree.md:73-79`). A granted `read-only` office stands on dsh's `sandbox-policy` row, the box's `ro` and `overlay` binds, and the box itself, not on a read-only bind of the worktree; the worktree stays read-write inside the box until that bind is built. A grant does not wait for it, because 0098's measured sandbox row and box are the controls the operator's evidence line names; the operator should not read the grant's `read-only` word as a read-only worktree. When the bind lands, it becomes part of the hands digest and a new grant.
7. **The boxed `exec` exemption is not widened or narrowed here.** 0043 ruling 3 admits a deterministic command at a gate because its blast radius is the box (`docs/decisions/0043-the-hands-are-one-tool.md:118-124`). A script that wraps a model is not a deterministic command, but the compile cannot tell one from the other; that pre-existing path is unchanged by this decision and is named as a follow-up, not fixed here.

Until B and 1052a land, the operator should use A: the dsh model as a work-class panel member, with a trusted chief. The operator should not seat a dsh gate on the strength of this proposal alone.

## The asks for the operator

Each ask is a choice with a recommended option, and each is unruled. The operator can rule each in one line; the "Ruling line" is the sentence to accept or replace.

### (a) Where the grant lives

**Option a1 — the adapter, as a new `gate_grants` list beside `routes` and `judges`.** Recommended. The route, the abstract model, the concrete id and the measured release are adapter facts; 0036 put the route class there, 0041 put `judges` there, and 0075 put the model tier there. One file, one digest, one operator edit. The list is not nested inside `routes`, because `routes` is a map from a route name to an `EgressClass` (0036 ruling 2) and a grant is not a class. A recipe cannot write it: the adapter library is the operator's data, and a recipe may not grant (0065 ruling 3). The cost is that a personal or per-realm variation has no supported home today: 0101 is accepted but unbuilt (`docs/decisions/0101-personal-realms-and-adapters-live-outside-the-tracked-tree.md:6`), and its ruling 4 keeps no personal adapter root in this build (`:158`, `:670`, `:735`). A variation therefore needs its own adapter in the tracked tree, or waits for 0101.

**Option a2 — the realm, per role, beside `capabilities`.** The realm is already the only granter of capabilities (0065 ruling 3), and 0046 makes the boundary a realm fact. A realm entry could name the adapter, route, model and roles. The cost is that the realm would restate adapter data it does not own: the route's concrete id and the measured release live in the adapter, and a realm that names them can drift from it. A hybrid — the adapter names the route, model and release; the realm names the roles — is the strongest form of a2, but it splits one grant across two files and makes the compile read two sources for one fact.

**Recommended: a1.** It keeps 0036's route data, 0041's judges and 0075's tier in one operator file, and the realm keeps the capability and boundary facts that are already its own.

**Ruling line:** `0107(a): the grant lives in the adapter, as a gate_grants list beside routes and judges.`

### (b) Whether a grant may name the review chief

**Recommended: no.** The chief is the seat that rules on the other gate seats' findings and absorbs their errors; its failure compounds every member's error, and it is the last word on the panel's verdict. A model that is not trusted adapter-wide should not be that seat, and a chief on an untrusted route would make the panel's advisory pattern impossible to reason about. The chief stays a trusted adapter's judge (0041 ruling 3). A later wager may promote the route for the chief role; that is a new operator ruling expanding this grant's role list, not a use of the present grant, and it is not available in this decision. The compile refuses a grant that names `chief`, and refuses a `chief` role even if a grant lists its agent office. This ask does **not** decide the standalone non-chief roles; ask (f) does.

**Alternative, not recommended:** allow the chief. It would let the downstream case replace its trusted chief, but it would also make the untrusted route the final word on every finding in the panel, and it would collapse the distinction between an advisory member and the check. The residual risk is exactly the one 0021's context names: "gate seats ARE the check, and nobody stands behind the judges" (`docs/decisions/0021-model-policy.md:16-22`).

**Ruling line:** `0107(b): no; a grant names no chief role.`

### (c) Whether a grant expires, or is re-proved when the harness release, the route's target model or the hands composition changes

**Option c1 — bound to the facts, no clock.** Recommended. The grant names the measured release, the concrete model id and the hands composition (the box, the boundary, the office's declared read-only access, the hands digest and every bind's mode). The compile refuses a release the adapter no longer measures with every axis measured, a concrete id the alias no longer maps to, a host the adapter has no shape on, a hands digest that does not match, and a bind mode that is not `ro` or `overlay`. Any change re-opens the grant; the operator records a new evidence line. This is stronger than a calendar against a changed release, because the dangerous change is effective immediately, not at the expiry date.

**Option c2 — a calendar expiry, in 0025's manner.** 0025 ruling 2 makes an expiry mandatory for Skírnir's operator-signed grant, "a grant without one is invalid" (`docs/decisions/0025-skirnir.md:43-51`). A grant that expires after a stated period, and is re-proved at expiry, catches a grant whose facts have not moved but whose evidence is old. The cost is that a stale grant stays live until its date, and a release can change the day after it is signed. 0025's shape is cited only for the expiry idea; its grant is a different grant, for the standing executor (#476), and its signature and ceiling are not imported here.

**Recommended: c1, with a recorded review date.** The grant is invalidated by any change to its facts; the operator records a review date in the evidence line, and `brokkr doctor` prints it, but a missed review does not keep a changed fact admitted. If the operator prefers a hard expiry as well, c2's date can be added without changing c1.

**Ruling line:** `0107(c): bound to the measured release, the concrete model id and the hands composition; any change re-opens it, with a recorded review date.`

### (d) The evidence bar

**Option d1 — the full bar.** Recommended:
- the wager under 0075 ruling 6 on real stories, comparing the route and model with the incumbent on cost per accepted change (#224) and on review holds and rounds (#436), with its run ids, including at least one story in the role the grant names;
- the conformance probe's report for the harness release (0075 ruling 3), with the version, host, date and evidence;
- the hands composition measurement (#896 for the measured releases; 1052a for `0.2.2-rc.1`), on the route and concrete model the grant names where the shape can be scoped to it, or on the shape the adapter measures where it cannot: the composed tree's measured rows, one `dsh-mcp-client` and Brokkr's workspace tool, the harness's own tools off, the box, the office's declared read-only access, and every bind's mode (`ro` and `overlay` only). The read-only worktree bind 0098 ruling 3 owes is not part of that measurement and the evidence line says so;
- the route's declared egress class and jurisdiction (0036), named, and the endpoint the measurement saw; the grant is not an egress grant;
- the operator's evidence line, recorded in the ruling as 0021 ruling 3 requires, and the promotion as 0075 ruling 6's roster addendum.

**Option d2 — the probe and hands measurement, without a wager.** Cheaper, and enough to show the harness can run boxed; not enough to show the model's judgment is worth a gate. A preview model with no wager is exactly the case 0075 ruling 5 keeps provisional; d2 would let a model into a gate on a harness measurement alone.

**Option d3 — the operator's judgment alone, with a stated reason.** The law's floor is an operator ruling with an evidence line (0021 ruling 3); a ruling with no evidence line is not one. Not recommended.

**Recommended: d1.** The wager is what makes the model's judgment a measured claim rather than an impression; the probe and the hands measurement are what make the harness and the box measured.

**What the compile can and cannot enforce.** The compile enforces every measurable precondition: the explicit promotion, the concrete id, the role, the box, the boundary's host, the office's declared access, the hands digest and its bind modes, the release's cold boxed shape and each of its measured axes. It cannot read a wager, a probe report or an operator ruling, and the `evidence` string is a pointer for doctor and for the ruling's record, not a proof. A grant with a measurable precondition missing is refused by the compile for the missing data; the wager, the probe report, the hands measurement and the operator's evidence line are the operator's ruling, and d1's bar is that ruling's authority, not a compile check.

**Ruling line:** `0107(d): the full bar — wager on real stories, probe report, hands composition on the named route and model, egress class and endpoint named, operator evidence line.`

### (e) Whether compile should refuse the second-adapter workaround

**Option e1 — refuse two adapters that name the same driver kind at different tiers.** Recommended. `Adapters::load` refuses the pair by name, with both files and both tiers, before any gate is judged. The key is the **driver kind** — the token after `{brokkr} driver` in the adapter's `driver` argv (`adapters/dsh.json:22-26`) — not `binary`: `binary` is only what doctor probes, while the dispatch is the driver's argv, and the shipped data already differs (`adapters/lanetally.json:27-35`: binary `claude-lanetally`, driver `lanetally`). A check on `binary` is passed by any other string. A repository can still copy the driver under another kind, but that is a new built-in driver — code, not data — visibly not the same adapter, and the guidance says not to. The check closes the exact workaround the issue names and makes the compile's refusal a property of the tree, not a convention.

**Option e2 — guidance alone.** Simpler, no false positives, and no new refusal. But the workaround is invisible: the second adapter passes every existing check except the duplicate-model one, which it sidesteps by renaming, and the only record is a second file. The issue's downstream has already found it; guidance that the compile does not enforce is the weaker answer.

**Recommended: e1.** The refusal is a name check, not a proof; it cannot see a copy or a wrapper, and the residual is named in the consequences. It does not need a new contract: the adapter files are library data, and the refusal is a load error naming both files.

**Ruling line:** `0107(e): yes; compile refuses two adapters naming the same driver kind at different tiers, naming both files.`

### (f) Whether a grant reaches a sole-check role

Ask (b) excludes the chief because the chief rules on other gate seats' findings. It does not decide whether a grant may name a **sole-check** non-chief role — a single `review` or `security-review` site, a `review` panel that no later gate step rules on, `verify`, or `ship` — where the granted model's verdict (or the panel's) is the check and no trusted chief stands behind it. The compile can tell this structurally from the same `SiteKey` and body walk: a gate site with no later gate step ruling on it is a sole check.

**Option f1 — yes, the grant names the role it admits, sole-check or panel member.** Recommended. The evidence conditions are the same and the role is named (condition 4); a sole-check role is exactly where d1's wager in that role matters most. The chief is special because it rules on other seats, not merely because it is last, and its exclusion is a separate ruling (ask (b)). The compile applies the same box, release, host, access, binds, promotion and fallback conditions, so a sole-check grant is no broader than its role.

**Option f2 — no, a grant is confined to a gate-class panel member that a trusted chief rules on.** This is the shape that is lawful today (option A) moved from work to gate. It is narrower and keeps a trusted model behind every granted verdict; the cost is that a standalone security lane, a review panel with no chief, verify or ship cannot use the grant, and the downstream case must be a panel member with a trusted chief. If the operator wants that narrower answer, the enforcement binding drops `verify`, `ship` and every `review` or `security-review` site that is a sole check (a single review, or the last gate step with no chief after it), and a "sole check" refusal names the structural reason.

**Recommended: f1.** The grant is role-scoped and the evidence bar is per role; refusing a sole-check role would leave the downstream case unserved whenever it is not a panel member, which is the ask's own case.

**Ruling line:** `0107(f): yes; a grant may name a sole-check non-chief role, with the same conditions.`

### (g) What the word for the structural gate position is

The tree already uses `office` for the hired agent's name (`crates/brokkr-runtime/src/bundle/tier.rs:9`; `crates/brokkr-runtime/src/capabilities.rs:260-261`), and 0065's `offices` and 0075's `provisional_offices` read that name. 0021 ruling 1's `chief`, `review`, `security review`, `verify`, `ship` are a different fact: the site's structural position. This draft calls that a **gate role** and binds it to the hired office (condition 4). The operator should choose the word and the field, so the same word does not carry two meanings in one realm.

**Option g1 — keep `office` for the agent's name, and name the structural fact a `gate role`.** Recommended. The grant's field is `roles`; `office` keeps its one meaning everywhere else. The role is derived from `SiteKey` plus the hired office; a recipe's panel member key is never read. The cost is two words where the issue's prose uses one, and the role `security-review` sits beside the office `review-security`; the data shape and the refusal text must keep them distinct.

**Option g2 — bind both words in the grant: name the hired `office` and the structural `role`, and require both to match.** Narrower and more explicit: a grant entry carries `office: "review-security"` and `role: "review"`, and a site must match both. The cost is a second field, and a custom agent or an operator's own agent needs an explicit entry before it can be admitted; the realm would have to map a custom office to a role for the same effect, which is a second source for one fact.

**Option g3 — keep the structural sense as `office` and rename the agent-name sense.** This is the issue's prose, but it would rewrite 0065, 0075, `SiteAsks.office` and `tier.rs`'s own sentence; it is out of 0107's scope and would be a separate semantic change.

**Recommended: g1.** It adds one word, `gate role`, to express the one fact the law did not have, and leaves every existing `office` sentence true.

**Ruling line:** `0107(g): keep office for the agent's name; the grant names a gate role, derived from the site's structure and the hired office.`

## What the older decisions say after acceptance

These are the replacement sentences, kept here so the older files are not edited. Each takes effect when this decision is accepted; the header pointers and back-pointers are added then, not in this story. No sentence in 0036, 0043, 0046, 0065, 0075 or 0098 changes; the grant is built within them, and the paragraphs below say why.

### 0021 ruling 2

The paragraph currently reads (`docs/decisions/0021-model-policy.md:37-42`):

> **Drivers carry an operator-granted trust tier.** The policy names tiers, not vendors: a driver's tier is data the operator rules into the bundle, never a constant in the engine. Gate seats require the trusted tier. **The bundle compiler refuses a bundle that seats an untrusted driver at a gate** — the refusal happens at compile time, before any prompt exists to leak, in the manner of a digest mismatch.

After acceptance it reads:

> **Drivers carry an operator-granted trust tier.** The policy names tiers, not vendors: a driver's tier is data the operator rules into the bundle, never a constant in the engine. Gate seats require the trusted tier, or a scoped gate grant that decision 0107 rules: an operator ruling, with an evidence line, naming one adapter, one route, one abstract model and its concrete id, and the gate roles it admits. A grant is not a tier and is never adapter-wide; it holds only for a boxed workspace-hands seat on a measured release and host, with the hired office's declared `read-only` access and its binds measured, with the model written `tier: promoted`, and every gate link that no grant admits still requires the trusted tier. **The bundle compiler refuses a bundle that seats an untrusted driver at a gate** unless every condition of that grant holds, naming the missing condition — the refusal happens at compile time, before any prompt exists to leak, in the manner of a digest mismatch.

The roles in that replacement follow the operator's rulings on asks (b), (f) and (g); this draft does not write them in as settled. If ask (g) rules otherwise, the words `gate roles` change in the same sentence and nowhere else.

### 0041 ruling 3

The two sentences that change currently read (`docs/decisions/0041-one-office-per-seat.md:201-204`):

> Each adapter declares `judges`: the abstract names that may hold a gate site. Compilation refuses a gate whose chain names any other model, on any link, naming the site and the link.

After acceptance they read:

> Each adapter declares `judges`: the abstract names that may hold a gate site. Compilation refuses a gate whose chain names any other model, on any link, naming the site and the link, unless decision 0107's scoped gate grant names that link's route, abstract model and concrete id for that role; a grant is a separate list and is never written into `judges`.

The rest of the paragraph stands, including the run-time sentence, the shipped list (`claude`, `codex`, `lanetally`, and dsh and exec none), and the `chief-architect` sentence. A grant does not change a shipped `judges` list.

### 0036 does not change

0036 ruling 3's sentence stands: "For the same reason `local` grants no gate seat, no trust tier and no fallback standing" (`docs/decisions/0036-egress-is-a-property-of-the-route.md:125-127`). A 0107 grant is not a route class. It names the route as its scope, and the route's class is read and reported; `local` is not evidence, and a local route with a grant is admitted on the same wager and box as any other. 0036's `routes` map is not extended with a trust tier; the grant is a separate list in the adapter, and the egress axis stays 0036's.

One comment in 0036's enforcement binding does change. `crates/brokkr-runtime/src/bundle/model_policy_tests.rs:1644-1651` says "The gate refusal reads `trust_tier` and nothing else"; after 0107 that sentence is false for a model a grant admits. The test's own assertion still holds, because `a_local_route_earns_no_gate_seat` seats no grant and still refuses on the tier. 0107a restates the comment — the refusal reads the tier, and a 0107 grant where one matches — and leaves the test's assertion as it is; the build slice names that edit.

### 0075 ruling 5 does not change

"It never holds a gate (a review chief) or the sole implementer of main-bound work until promoted" (`docs/decisions/0075-harness-onboarding-in-a-day.md:151-152`) stands exactly. The compile reads the provisional tier before the grant, and a provisional model is refused at a gate whatever the grant says. A new model starts provisional under this ruling, so the adapter writes the tier for the preview until it is promoted; an absent tier still reads promoted for the models that predate the tier, but a grant requires the written `promoted` word 0075's binding names, so absence cannot admit a grant. A grant on a promoted model is the promotion's companion, not a substitute for it.

### 0043, 0046 and 0065 do not change

0043 ruling 2's hands-and-secrets refusal (`docs/decisions/0043-the-hands-are-one-tool.md:113-116`), 0043 ruling 3's boxed `exec` exemption (`:118-124`) and 0046 ruling 4's boundary table (`docs/decisions/0046-the-boundary-is-named.md:179-191`) are the grant's conditions, not its objects. 0065's realm grants (`docs/decisions/0065-capabilities-are-the-realms-to-grant.md:122-146`) remain the only capability grants, and 0065 ruling 7's gate-egress naming (`:199-207`) is untouched; a 0107 grant is not one.

## Enforcement binding (named, not built)

Nothing in this section is built. It is the commission for 0107a and 0107b; the `Built:` marker stays `unbuilt (#1052)` until both land. The exact-release pin is folded into 0107a rather than left for 0107b, so no build admits a grant before its release condition is enforceable. Each condition assumes the operator's ruling on the asks; an ask ruled otherwise changes the named condition and its refusal, not the rest.

### The data fields

A new top-level `gate_grants` list on the adapter, beside `routes` and `judges` (`crates/brokkr-runtime/src/agents.rs:396-437`), and the loader's `ADAPTER_KEYS` gains `gate_grants` (`crates/brokkr-runtime/src/agents/load.rs:707-728`):

```json
"gate_grants": [
  {
    "route": "dashscope",
    "model": "qwen-max",
    "concrete": "dashscope/qwen3.8-max",
    "roles": ["security-review"],
    "release": "0.2.2-rc.1",
    "hands": "sha256:<the canonical hands spec's digest>",
    "evidence": "#1052, 1052a, the promotion addendum and the wager run ids"
  }
]
```

There is no `access` field: the grant has no access of its own, and the compile reads the hired office's declared `read-only` (0098 ruling 3). The example's release is the one 1052a measures; today the adapter measures only `0.2.1-rc.2`, so a `0.2.2-rc.1` grant is refused until 1052a writes that shape.

The typed shape, in a new `crates/brokkr-runtime/src/agents/load/gate_grant.rs`:

- `GateGrant { route: String, model: String, concrete: String, roles: Vec<GateRole>, release: String, hands: HandsDigest, evidence: String }`, parsed once at the edge with `deny_unknown_fields` (0071 ruling 3). An absent `gate_grants` is the empty list, so every adapter on disk reads exactly as it does today.
- `GateRole` is a closed enum with `Review`, `SecurityReview`, `Verify`, `Ship` and `Chief`, written `review`, `security-review`, `verify`, `ship` and `chief`. `Chief` is a known word so the loader can refuse it with the operator's text (`GateGrantChief`) instead of "unknown role"; a grant may not contain it. The site's own role reads the same enum plus a `None` value for a gate seat outside the five.
- `HandsDigest` is a newtype, not a raw `String`: it parses `sha256:` followed by 64 lowercase hex, and its parse failure names the canonicalisation. The bytes are `brokkr_core::canonical::sha256_hex(HandsSpec::to_value())` for the site's resolved hands (`crates/brokkr-protocol/src/hands.rs:148-217`), the same canonical value the manifest pins (`crates/brokkr-runtime/src/bundle.rs:4228-4236`); no untyped string reaches the check.
- `Adapter` gains `tiers: BTreeMap<String, ModelTier>`, with `ModelTier::{Provisional, Promoted}`, replacing the existing `provisional: BTreeSet<String>`. The three states are one typed fact: absent (legacy, reads promoted for every ordinary seat and cannot carry a grant), `Provisional`, and `Promoted`. The existing provisional check reads `ModelTier::Provisional`, and the grant check requires `ModelTier::Promoted`.
- `route`, `model`, `concrete`, `release` and `evidence` are non-empty; `roles` is non-empty and deduplicated. A repeated role, an unknown role, a route that is not the concrete id's prefix, a model not in `models`, a route and model granted twice, or a grant on a `Trusted` adapter is a load refusal naming the adapter and the grant.
- A grant is a load refusal on a driver kind with no exact-release carrier (`GrantNoCarrier`), naming the driver and the adapter. `dsh` is the one carrier today; `adapters/lanetally.json` writes no `trust_tier`, reads untrusted, and carries no pin, so a grant there is refused.
- Two adapters that name the same driver kind at different `trust_tier`s are a load refusal naming both files and both tiers (ask (e)'s e1).
- The grant is part of the adapter's bytes, so it is digested with the adapter and moves every bundle that pins it. It is not a wire contract and mints no contract version.
- The `evidence` string is a pointer for doctor and for the ruling's record, not a proof; the compile cannot read a ruling, and the operator's recorded ruling is the authority 0021 ruling 3 requires. It is non-empty by load, so doctor has no "missing" state for it.

### The one rule, and the refusals

A gate link passes when one of these holds: it is a boxed `exec` (0043 ruling 3, unchanged); or its adapter is `Trusted` and its abstract model is in `judges` (0021 ruling 2; 0041 ruling 3, unchanged); or it matches one grant on its abstract model and every condition of that grant holds. The grant is looked up by abstract model, then judged in this order:

1. **`GrantConcrete`:** the resolved concrete id must equal the grant's `concrete`; a moved alias or route names both ids.
2. **`GrantPromotion`:** the model must read `ModelTier::Promoted` in the adapter's `tiers`; a bare or absent tier names 0075 ruling 6.
3. **`GrantRole`:** the site's derived role must be in the grant's `roles`, and never `chief`.
4. **`GrantUnboxed`:** the site must declare hands and the realm's boundary must be `namespace`, `seatbelt` or `container`.
5. **`GrantNetwork`:** the hands must declare `network: false`.
6. **`GrantAccess`:** the hired office's declared access must be `read-only`.
7. **`GrantBindMode`:** every explicit bind must be `ro` or `overlay`; an `rw` bind is refused.
8. **`GrantBinds`:** the site's whole hands spec must digest to the grant's `hands`.
9. **`GrantHost`:** the boundary's host must have a measured shape; `namespace` is Linux, `seatbelt` is macOS, `container` has none.
10. **`GrantRelease`:** the adapter must have a cold boxed shape at the grant's `release` on that host.
11. **`GrantAxes`:** that shape's `native_write`, `store_read`, `process_read` and `ambient` must all be `Measured`, and the refusal names the first that is not.

A link with **no matching grant** is a separate state. If it is trusted and its model is in `judges`, it passes. If it is trusted and misses `judges`, the existing `refuse_unjudged_model` refuses it and names the link. If it is untrusted and an earlier link of the same chain passed by grant, `GrantFallback` refuses it and names the link. If it is untrusted and no earlier link passed by grant, the existing `refuse_untrusted_gate` refuses it, unchanged.

A link that matches a grant and misses a condition is refused with that condition named; it does not fall back to the tier refusal. `refuse_secret_egress` runs on every link after the gate check, unchanged, and a grant never bypasses it.

The messages, one per variant, in the operator's text:

- `GrantConcrete`: "seat '{what}' gate link {link} seats model '{model}' as '{actual}', and the grant names '{granted}'; a grant is bound to the concrete model id, and a re-pointed alias or route re-opens it (decision 0107 condition 2)."
- `GrantPromotion`: "seat '{what}' gate link {link} seats model '{model}', whose adapter '{adapter}' does not write `tier: promoted`; a grant requires the 0075 ruling 6 promotion to be written on the model, not inferred from an absent tier (decision 0107 condition 3; decision 0075 ruling 5)."
- `GrantRole`: "seat '{what}' gate link {link} holds the {role} role, which the grant for route '{route}' and model '{model}' does not name; a grant names its roles and never the chief (decision 0107 condition 4, ask (b))."
- `GrantUnboxed`: "seat '{what}' gate link {link} is not boxed with workspace hands (boundary '{boundary}', hands {presence}); a scoped gate grant holds only a boxed workspace-hands seat under namespace, seatbelt or container (decision 0107 condition 5; decision 0046 ruling 4)."
- `GrantNetwork`: "seat '{what}' gate link {link} declares hands with network on; the grant requires a no-network box, because dsh's native egress is unmeasured (decision 0107 condition 5; decision 0075 ruling 4)."
- `GrantAccess`: "seat '{what}' gate link {link} hires office '{office}', whose declared access is '{access}'; the grant requires 'read-only', because the measured hands rows and the store mask stand on a read-only office (decision 0107 condition 5; decision 0098 ruling 3)."
- `GrantBindMode`: "seat '{what}' gate link {link} binds '{path}' as '{mode}'; a scoped gate grant admits only 'ro' and 'overlay' binds, because 'rw' writes land on the host (decision 0107 condition 5; decision 0043 ruling 2)."
- `GrantBinds`: "seat '{what}' gate link {link} seats hands digest '{actual}', and the grant names '{granted}'; a grant is bound to the measured box, binds included (decision 0107 condition 5)."
- `GrantHost`: "seat '{what}' gate link {link} runs under boundary '{boundary}', whose measured host is {host}, and adapter '{adapter}' has no cold boxed shape there; it measures {measured} (decision 0107 condition 5; decision 0046 ruling 4)."
- `GrantRelease`: "seat '{what}' gate link {link} names release '{granted}', and adapter '{adapter}' has no cold boxed shape at that release; it measures {versions} (decision 0107 condition 5; decision 0075 ruling 3)."
- `GrantAxes`: "seat '{what}' gate link {link} names release '{release}', whose cold boxed shape records {axis} as {state}; the grant requires every isolation axis measured (decision 0107 condition 5; decision 0075 ruling 3)."
- `GrantFallback`: "seat '{what}' gate link {link} has no grant of its own; a link that follows a granted link qualifies on its own by the trusted tier and judges or by its own grant, and a grant is not copied (decision 0107 condition 6; decision 0041 ruling 3)."

The load refusals (`LibraryError`, naming the adapter and the entry): a grant naming `chief`; a grant on a `Trusted` adapter; a route that is not the concrete id's prefix; a model not in `models`; an empty or repeated field; an unknown key; a grant on a driver kind with no release carrier; and two adapters naming one driver kind at different tiers. `GateGrantChief` names ask (b); `GateGrantTrusted` names condition 1; `GateGrantConcrete` names 0036 ruling 2; `GrantNoCarrier` names condition 5's carrier sentence; the duplicate-names refusal names 0071 ruling 5.


### Doctor's readout of each grant

A new `crates/brokkr-cli/src/doctor/grant.rs`, modelled on `crates/brokkr-cli/src/doctor/provisional.rs:1-54` (one line per grant, or one line saying there is none):

- each grant's adapter, route, abstract model, concrete id, roles, release, hands digest and evidence reference;
- the adapter-level status it can read: `ready` when the alias, the written promotion, the release's cold boxed shape, the host and every axis read as the grant says; `alias moved`; `not explicitly promoted`; `unmeasured release`; `unmeasured axis`; `host not measured`;
- the route's egress class, including `uncontracted` for a route not in `routes` (`crates/brokkr-runtime/src/agents/load.rs:368-380`), reported beside the grant, not as a grant fact; the grant changes no egress or secret clearance and has no egress field to read;
- the measured release and host the evidence stands on, so a Linux measurement is not read as a macOS one;
- a sentence that the role, the office's declared access, the box and the binds are compile checks of a site, not facts of a grant, so doctor does not report them as if it could.

Doctor reports; the compile refuses (the same split as `doctor/provisional.rs`). The adapter-level standing is derived once by a pure `GateGrantStatus` in `crates/brokkr-runtime/src/agents/load/gate_grant.rs`, which doctor paints and the compile's adapter-level refusals read; the site-level checks (role, office access, box, bind mode, hands digest, fallback) are the compile's alone and doctor says so. The CLI only paints the line (0071 ruling 7 — the home is the runtime, because the compile is the admitting authority and may not read a display crate).

### The tests a build slice must add

Each test is written to bind: the slice records a compiling change that removes the check it proves, the test fails, the change is restored, and the failing test is named in the slice's result. The build slice adds the tests; this decision alone adds none.

- `a_scoped_grant_admits_a_boxed_promoted_route_at_the_named_role` — the positive control: the dsh adapter with a grant on `dashscope/qwen3.8-max`, an explicitly promoted model, a read-only boxed workspace-hands `security-review` seat whose hands digest matches, `ro` and `overlay` binds only, and a cold boxed measured shape on the granted release compiles.
- `an_unboxed_grant_seat_is_refused_naming_the_box` — the same seat with no `hands`.
- `a_harness_or_open_boundary_grant_seat_is_refused` — `harness` and `open` each refuse; `namespace` admits; `seatbelt` and `container` refuse on the host or the missing shape.
- `a_network_on_grant_seat_is_refused` — `hands.network: true` refuses.
- `a_writes_grant_seat_is_refused` — the hired office declares `access: writes` and refuses; an absent access reads writes (0098 ruling 3).
- `an_rw_grant_bind_is_refused_naming_the_mode` — an explicit `rw` bind refuses at `GrantBindMode` even when the grant's digest names it; `ro` and `overlay` admit.
- `a_grant_seat_with_an_unmeasured_bind_set_is_refused` — a bind added or removed moves the digest and refuses at `GrantBinds`.
- `a_replacement_boxed_shape_does_not_carry_a_grant` — a shape recorded only as `replacement` refuses at `GrantRelease`.
- `an_unmeasured_axis_refuses_and_names_it` — `store_read: unmeasured` on the granted release refuses at `GrantAxes`.
- `a_macos_boundary_without_a_macos_shape_is_refused` — `seatbelt` refuses at `GrantHost`; a fixture macos cold boxed shape admits.
- `a_granted_release_runs_only_on_the_observed_release` — a driver input naming `0.2.1-rc.2` with an observed `0.2.1-rc.3` refuses at run start; the observed rc.2 passes; no grant keeps `MEASURED_VERSIONS`; an unmeasured version still refuses at the set before the exact match.
- `the_granted_release_homes_agree` — every release the adapter's cold boxed shape names is in the driver's `MEASURED_VERSIONS`, and every release there has a cold boxed shape; the two homes cannot drift silently.
- `a_grant_on_a_driver_without_a_release_carrier_is_refused` — a grant on an untrusted adapter whose driver kind carries no exact-release pin refuses at load, naming the driver.
- `an_inline_gate_with_a_grant_is_refused` — an inline site has no role; the grant never applies, and the tier and judges refusals stand.
- `a_provisional_model_grant_is_refused` — the model reads `Provisional` and refuses before the grant is read; the refusal names 0075 ruling 5.
- `a_bare_model_grant_is_refused_for_the_missing_promotion` — `qwen-max` with no written tier refuses at `GrantPromotion`.
- `a_repointed_alias_is_refused_naming_both_ids` — `models[model]` moved from the granted id, or the granted route moved; the refusal names both.
- `a_grant_on_a_route_the_concrete_id_does_not_reach_is_refused` — the prefix mismatch is a load refusal.
- `a_grant_naming_the_chief_is_refused` — `roles: ["chief"]` is a load refusal.
- `the_role_is_structural_and_office_bound` — a `security` panel slot hiring `review-correctness` is `review`, and a `security-review` grant refuses; a `correctness` slot hiring `review-security` is `security-review`, and a `review` grant refuses; a final gate-class `review` step after a work-class `positions` panel is `chief`, and a grant naming its agent office refuses; a gate-class step before the chief is its `review` or `security-review` role, not chief; a panel member keyed `chief` in a one-step panel is `review`; a `review` panel with no later gate step is a sole check; the clarify and analyze judges have no role.
- `an_untyped_hands_digest_is_refused` — a bare hex string, or `sha256:` with a short or uppercase hex, refuses at load; the canonical `sha256:<64 lowercase hex>` of `HandsSpec::to_value()` admits.
- `a_fallback_link_without_its_own_grant_is_refused_naming_the_link` — first link granted, second untrusted and ungranted; the second link's number is in the message; a trusted-judge fallback passes.
- `a_grant_does_not_lift_the_egress_bar` — a granted seat that declares secret bindings still hits 0043 ruling 2's hands-and-secrets refusal and 0036's egress refusal.
- `a_grant_does_not_change_the_judges_or_the_tier` — a non-granted model on the same adapter still hits 0041 ruling 3 and 0021 ruling 2, unchanged.
- `the_second_adapter_workaround_is_refused` — two adapters, one driver kind, different tiers and different model names; the load refusal names both files. Two adapters with different driver kinds are not refused, even when they share a `binary`; the lanetally adapter differs in both `binary` and driver kind (`adapters/lanetally.json:27-35`), so it is not an example of the shared-binary shape, and the test's own fixture supplies one.
- `an_unknown_grant_field_is_refused_by_name` — `deny_unknown_fields`.
- `the_grant_is_in_the_adapter_digest` — the adapter's bytes and the bundle's identity move.
- `doctor_names_every_grant_and_its_data_status` — the readout for ready, alias-moved, not-explicitly-promoted, unmeasured-release, unmeasured-axis and host-not-measured, the route's egress class reported beside the grant, and the sentence that site conditions are compile checks.

### The slices

**0107a — the data, the compile and the exact-release pin.** Files: `crates/brokkr-runtime/src/agents.rs` (the `Adapter` `tiers` and `gate_grants` fields), `crates/brokkr-runtime/src/agents/load.rs` (`ADAPTER_KEYS` gains `gate_grants`; the same-driver-different-tier load check beside the existing duplicate-model refusal at `:649-663`; the driver-kind carrier refusal), `crates/brokkr-runtime/src/agents/load/adapter.rs` (the `gate_grants` parse), a new `crates/brokkr-runtime/src/agents/load/gate_grant.rs` (the typed `GateGrant`, `GateRole`, `HandsDigest` and `GateGrantStatus`), `crates/brokkr-runtime/src/agents/load/models.rs` (the written `promoted` tier and the one `tiers` map), a new `crates/brokkr-runtime/src/bundle/grant.rs` (the lookup and `GateGrantRefusal`), `crates/brokkr-runtime/src/bundle/route_policy.rs` (the lookup before the tier and judges), `crates/brokkr-runtime/src/bundle.rs` (the per-link call, the earlier-grant state and the structural role derived from `SiteKey`), `crates/brokkr-protocol/src/adapters/dsh_hands.rs` (the `GRANT_RELEASE` carrier and the exact-match branch of `measured`, after the set check), `crates/brokkr-runtime/src/engine/marks.rs` (write the carrier from the site's compiled grant release), the bundle site facts that carry it, and the tests in `crates/brokkr-runtime/src/bundle/model_policy_tests.rs` or a new `crates/brokkr-runtime/src/bundle/tests/grant.rs`, plus the driver's own tests. About 330 production and 410 test lines. No `adapters/dsh.json` change in this story; the operator's grant is data for a later slice, and the tests use fixtures. No contract version; the adapter data is digested with the adapter. The exact-release pin is in this slice, so no build admits a grant whose release the driver does not enforce. The slice also restates the comment at `crates/brokkr-runtime/src/bundle/model_policy_tests.rs:1644-1651` (the gate refusal reads the tier, and a 0107 grant where one matches), leaving `a_local_route_earns_no_gate_seat`'s assertion as it is.

**0107b — doctor and the guide.** Files: a new `crates/brokkr-cli/src/doctor/grant.rs`, `crates/brokkr-cli/src/doctor.rs` (the section call), `crates/brokkr-cli/src/doctor/provisional.rs`'s test style for the readout, and a section in `docs/guides/provider-adapters.md` that says what a grant is, what it is not, how to compute the hands digest, and how to read doctor's line. About 80 production and 110 test lines, plus the guide prose. The `Built:` marker moves to `built` only when both slices land; the guide is not `docs/guides/agent-library.md`, which #1037 owns.

**Order.** 0107a first; 0107b after it. 1052a's measurement is a prerequisite for a real `0.2.2-rc.1` grant, not for the compile: the compile can be built and tested against fixtures, and it refuses the real release until 1052a updates the adapter. 1052b (the per-recipe patch) is independent; it seats whatever the grant admits, and this story does not touch it.

## Consequences

- **What changes for a bundle that uses no grant:** nothing. An adapter without `gate_grants` loads as today; the tier and judges path is unchanged; the digests of every current bundle do not move.
- **What changes for a bundle that uses one:** the adapter's bytes and the bundle's identity move; the compile reads one more list and one more model fact; the driver reads one more carrier; the doctor prints one more section. A grant is visible in the manifest through the adapter digest, and its refusals name the condition.
- **What does not change:** `trust_tier` and `judges` keep their meanings; 0036's egress map and classes; 0043's box and its hands-and-secrets refusal; 0046's boundary; 0065's capability grants; 0075's provisional tier and promotion road; the wire protocol. No contract version is minted. No adapter, recipe or realm is edited in this story.
- **What it costs:** one more adapter list, one more compile path, one more run-start carrier, one more doctor readout, and one more thing for the operator to keep true. The grant's evidence is a point-in-time measurement, and the residual risks above are the price of admitting a model that the blanket tier would have admitted for the wrong reason.

## Disclosure

In 0021 ruling 8's manner: this draft was written by a seat running on dsh (DeepSeek Flash, DeepSeek's platform), unboxed with dsh's own tools, at the operator's word (2026-10-26). That is a conflict of interest wherever dsh's route and model are discussed: the draft recommends a grant that would let a dsh route hold a gate, and the seat writing it is on that provider. The conflict is why the recommendation keeps the adapter untrusted, requires an independent wager and a measured release, names the residual risks, and leaves every choice to the operator. It is also why the interim answer is the work-class panel member under a trusted chief, not a grant on this draft's authority. The draft is reviewed by other seats; their findings and the operator's ruling, not this seat's provenance, are what make the grant law.
