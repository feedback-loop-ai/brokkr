# 0041 — One office per seat: triage rules the strategy, judges never fix, and every finding has a way back

Status: accepted (operator ruled in chat, 2026-09-04; drafted at the operator's direction 2026-09-03: "put it in 0041")
Date: 2026-09-04

## Context

On 2026-09-03 the operator asked for an honest review of the agent
roster and its effort pins, a world-class team and phase composition,
and named two hires the roster lacks: one that rules which strategy a
commission gets, and a standing operator that runs the shop around the
clock under a charter it does not compile. The second hire is decision
0025 and is not re-ruled here. The first, and the roster, are.

A note on one citation. Wherever this decision says "decision 0040" it
means the boxed-hands decision — the model's hands are one tool, and
the tool runs in an empty root — which was drafted and accepted under
that number on 2026-09-03 and was still landing when the flag-grammar
decision took 0040 on main. The hands decision landed as 0043; its
rulings are cited here by their own numbers and are unchanged by the
renumbering.

The review was made against the tree after #160 and against the
canonical journal. What it found, cited:

- **The library's `fable` is the previous generation.**
  `adapters/claude.json` maps `fable` to `claude-fable-5`;
  `brokkr init` scaffolds the same map for every adopter
  (`crates/brokkr-cli/src/init.rs`), and
  `crates/brokkr-runtime/tests/adoption.rs` pins it. Every inline recipe
  pins `claude-fable-5-1` under decision 0031, and the canonical journal
  carries that id 1,452 times and the bare one never. A probe on
  2026-09-03 (`claude --model claude-fable-5 -p`, $0.27 at list) served
  it: canonical model `claude-fable-5`, a distinct and older generation.
  The chief-architect, the one library agent that names `fable`, has
  hired the previous generation since decision 0016, and decision 0040's
  review chain would hire it at its first link.
- **Effort carries no decision.** All seventeen agent files pin `high`
  on every candidate. Decision 0035 made effort part of the hire; the
  roster hires everyone the same. Decision 0040 ruling 5 is the first
  chain in which effort varies — fable at high, then opus and sol at
  xhigh — and it varies backwards: the preferred hire at the lowest
  effort, the fallbacks compensated.
- **The strategy ladder is inverted.** `fast`, the default, hires
  fable-5-1 on all four seats. `crucible` calls itself "the heaviest
  crew in the roster" and hires opus-5 everywhere. `ember` says it
  "never buys a cheaper judge" and seats opus-5 on review.
  `night-shift` promises "every gate on the best judge available" and
  means opus-5. All three sentences became false the day 0031 re-pinned
  `fast`, and none of the three recipes has run end to end, by its own
  README. (`wager-harness`'s README says the same of itself and is
  stale: it ran on 2026-09-02.)
- **Gate chains descend.** `chief-architect` falls from fable to opus to
  sonnet; `reviewer` from opus to sonnet. The fallback is bounded
  (0016), and a judge that falls to sonnet is still the substitution
  decision 0021 ruling 5 refuses for drivers.
- **Dead policy in the library.** `verifier-speckit` and
  `shipper-speckit` name the same charters as `verifier` and `shipper`
  and exist only to carry the `specify` grant, which neither charter
  invokes. `review-security-speckit` is a near copy of
  `review-security`. `intake-speckit` describes itself as running the
  spec-kit CLI; its charter never does. Every agent grants `python3` and
  `pytest`, and the tree holds no Python outside `.forge/` scratch. The
  reviewer's own charter names dead policy as a finding.
- **The library is not portable.** The charters are this repository's
  text: the Rust-only rule, `cargo test --workspace`, the self-bundle
  compile, decision numbers. `recipes/node` copied its roles because it
  could not use them.
- **Offices are mixed.** The operator's observation the same day: some
  agents review *and* fix instead of handing the work back to the
  implementer, "and that is not single responsibility." The tree agrees
  in three places and contradicts it in one. The panel members and the
  crucible review chief are read-only by charter; decision 0022 ruling 6
  already says judges report and smiths fix. The single-seat `reviewer`
  is the contradiction: its charter says it "MAY apply small, safe
  fixes" and commit them, `fast`'s `REVIEW-CLEAN` sends that fix through
  verify and back to the same judge, and the `fixes_applied` input, the
  `REVIEW-REFORGE-EXHAUSTED-DEBT` predicate and 0039's
  `REVIEW-CLEAN-DOCS-FIXES` all exist to price the habit. It is the
  recipe the front page runs. Beyond the reviewer: `intake` has one
  result, and its charter tells it to say in the framing when a request
  is incoherent or violates a frozen contract and to let the implementer
  and reviewer act on the prose — a refusal that travels as text.
  `VERIFY-FAIL` hard-stops, so an implementer that claimed green never
  receives the red. The shipper re-checks the clean tree and the head the
  engine already gates on, then writes the ledger. The engine appends the
  seat-level result contract to every sequence step, so the chief's and
  both position charters carry a paragraph disclaiming a vocabulary that
  is not theirs.
- **Triage, and what it is not.** A first sketch put the strategy seat
  beside Muninn: invoked before a run, proposing, recorded beside the
  journals. The operator ruled otherwise: "triage is a legit phase," and
  "not Muninn — more a chief architect kinda role." The constitution
  agrees. Muninn knows and rules nothing (0020); a chief is consulted
  fresh and blind and rules within a run ("architecture within a run
  stays with the chief, fresh and blind," 0020). A seat that rules the
  strategy is a chief's office, and a gate. The constraint it works
  under is the pinning law: a run is one digest (0002, 0017), so triage
  cannot pick a recipe mid-run. It can pick a path through one table and
  a case inside one bundle, and both are journal facts.

Alternatives weighed:

- **Rename recipes to strategies.** Rejected. The rename slice closed
  under 0019 the day before; the guides already say a recipe is a
  delivery strategy written as data. `recipe` names the menu;
  `strategy` names what triage rules.
- **Triage as a pre-run advisor.** Rejected by the operator, and rightly:
  an advisor proposes and a chief rules, and the ruling belongs in the
  run it governs.
- **Triage as a meta-run that launches a second run under the chosen
  recipe.** Rejected: two journals for one commission, a pull request
  naming two runs, and the choice standing outside the run it shaped.
- **Keep the judge's small fixes, for cost.** Rejected: the judge then
  reviews its own fix, and under ruling 5 a low or info finding never
  buys a heat anyway. The cost the fixes clause saved is the cost 0039
  had to price.
- **Max effort everywhere.** Rejected by 0035's own logic: effort is
  part of the hire, and spend belongs where an error is invisible to the
  seats downstream.

## Ruling — 2026-09-04, operator: accepted as proposed

Accepted in chat the day after it was drafted ("accept 0041 and
0042"), without amendment. The eight rulings and their enforcement
bindings stand as written and are the commission of the enactment
slices, in the order the consequences record, after the boxed-hands
decision lands and moves the agent files it shares with this one. The
names left open — `triage` or `strategist`, the effort figures, the
fate of `panel-review`'s flat join — stay open, to be settled by the
enactment's evidence and the operator's later word.

## Rulings

1. **`fable` means the current generation; a superseded one gets its own
   name.** `adapters/claude.json` maps `fable` to `claude-fable-5-1`, the
   `lanetally` adapter's `fable-tallied` likewise, and `brokkr init`
   scaffolds the same map. A bare family name never points at a
   superseded generation: to hire an older one, an adapter names it
   explicitly (`fable-5`), so the choice is legible in every agent file
   that makes it.

   **Enforcement binding:** the two adapter files;
   `crates/brokkr-cli/src/init.rs`; the pin in
   `crates/brokkr-runtime/tests/adoption.rs` flips; every witness digest
   in `crates/brokkr-runtime/tests/witness_digests.rs` re-pinned with
   this ruling as the reason.

2. **One hire per office, and the library is the roster.** An office is
   hired once, in `agents/`, and every shipped recipe seats it from
   there. The exception is the wager harnesses, whose parity law needs
   inline arms (`recipes/wager-harness/README.md`). The roster:

   | Office | Hire | Fallback | Class |
   |---|---|---|---|
   | `triage` (ruling 6) | fable-5-1 xhigh | opus-5 max | gate |
   | `chief-architect` (design synthesis) | fable-5-1 max | opus-5 max | gate |
   | `reviewer`, `review-security`, `review-chief` | fable-5-1 xhigh | opus-5 xhigh, then sol xhigh | gate |
   | `review-correctness` | sol high | opus-5 high | gate |
   | `review-spec-compliance` | opus-5 high | sol high | gate |
   | `position-simplicity` | opus-5 high | sol high | work |
   | `position-robustness` | sol high | opus-5 high | work |
   | `implementer` | opus-5 high; fable-5-1 high under the `engine` class | sonnet-5 high | work |
   | `intake`, where a recipe keeps one | sonnet-5 high | opus-5 high | work |
   | `verifier`, `shipper` | boxed exec, no model (decision 0040) | — | gate |
   | `muninn` | opus-5 high | — | — |

   The logic is one sentence: spend goes to the seat whose error nobody
   downstream can see. The design chief writes the contract the
   spec-compliance reviewer judges against, so a wrong spec is invisible,
   and it is hired at max. The last judge before ship is hired at xhigh.
   Panel members are checked by a chief, so they are hired at high and,
   what matters more, from different model families, because a panel of
   one model at one effort argues with itself: every shipped panel seats
   two families by construction, and every fallback crosses a family.
   Sol earns its place as a diverse member, not as a third fallback. A
   first link is never hired at less effort than its fallback; decision
   0040 ruling 5's chain stands with its first link raised to xhigh,
   which amends that ruling in exactly one word.

   The three duplicates (`verifier-speckit`, `shipper-speckit`,
   `review-security-speckit`) are deleted; `review-chief` and `triage`
   join the library; `intake-speckit`'s description is corrected;
   `python3` and `pytest` leave every allow-list; `specify` stays only on
   agents whose charter invokes it. A recipe README states what the
   roster is, or says nothing about models.

   **Enforcement binding:** the agent files; the recipes;
   `crates/brokkr-runtime/tests/roster.rs` (new) walks the shipped
   recipes and refuses an inline model-backed site outside
   `recipes/wager-harness*` and `recipes/fast`'s quickstart seats, walks
   `agents/` and refuses a tool grant no library charter invokes, and
   pins that a chain's first link is hired at no less effort than any
   later link; the witness and compose pins.

3. **A gate hires judges only, and parks rather than descends.** Each
   adapter declares `judges`: the abstract names that may hold a gate
   site. Compilation refuses a gate whose chain names any other model,
   on any link, naming the site and the link. At run time nothing new:
   under 0016 a judge that fails to start falls to the next judge, and a
   gate whose judges are all unavailable parks — nothing substitutes
   (0021 ruling 5). Shipped: claude `["fable", "opus"]`, codex `["sol"]`,
   lanetally `["fable-tallied", "opus-tallied"]`, dsh and exec none.
   `chief-architect` and `reviewer` lose their sonnet link.

   **Enforcement binding:** the adapter files; `enforce_model_policy` in
   `crates/brokkr-runtime/src/bundle.rs`;
   `crates/brokkr-runtime/src/bundle/model_policy_tests.rs` — a gate on
   sonnet refused, on opus admitted, a chain whose third link is sonnet
   refused naming the third link, a work seat on sonnet untouched.

4. **A gate changes nothing, and the engine checks.** No gate-class site
   commits, edits the tree, or ticks a task; a gate produces a finding
   and a verdict. The `reviewer` charter loses its fixes clause and joins
   the panel members. `fixes_applied` leaves every shipped recipe's
   inputs and rules — `REVIEW-CLEAN` with fixes, the
   `REVIEW-REFORGE-EXHAUSTED-DEBT` predicate, 0039's
   `REVIEW-CLEAN-DOCS-FIXES` — while the frozen heritage table keeps its
   own words. The check is the engine's, not the charter's: the engine
   records the repository head when a gate effect starts and when it
   ends, and a head that moved parks the run with the reason
   `GATE-MOVED-HEAD`, raw evidence attached. That is a defect of the
   seat, never a result to route.

   **Enforcement binding:** the charters; the recipe tables and
   `bundles/self/policy.json`; the engine's effect start and finish
   path; `crates/brokkr-runtime/src/engine/tests.rs` drives a fake gate
   driver that commits and asserts the park; `roster.rs` asserts no
   shipped recipe declares `fixes_applied` or carries a rule that reads
   it.

5. **Every finding has an edge, and every edge a bound.** A finding
   travels to the seat that owns the defect, bounded by phase visits the
   way decision 0022 bounds reforging:

   | Finding | From | Returns to | Bound | When exhausted |
   |---|---|---|---|---|
   | the suite is red after a green claim | verify | implement | `visits_implement` below 3 | stop |
   | a residual above low, any dimension | review | implement | `visits_implement` below 3 | above medium stops; medium parks; a security residual at low or info ships as named debt |
   | the spec itself is at fault (`spec_defect`) | review | design | `visits_design` below 3 | park |
   | the work exceeds its class (`oversized`) | implement | triage | `visits_triage` below 2 | park |
   | the artifacts fail the validator | design | design | attempts, as today | stop, as today |
   | the head moved after the verdict | ship | review | as today | as today |

   In words. (a) `VERIFY-FAIL` returns to implement carrying the failing
   output as `returned_from`. (b) Reforging extends to every dimension —
   the future ruling 0022 ruling 5 reserved: a residual above low returns
   to implement whatever its dimension; a non-security residual at low
   or info ships as named debt without a return, as today; at exhaustion
   a security residual at low or info ships as named debt, because the
   smith had two heats and the note is the debt Muninn patrols, and
   `REVIEW-REFORGE-EXHAUSTED-UNFIXED` retires with the judge's fixes.
   (c) The spec-compliance member may report `spec_defect: true` — the
   implementation redefined a criterion because the criterion was wrong
   — as a declared input the `review-panel` aggregate ORs, and a review
   carrying it returns the piece to the chief, not the smith. (d) The
   implementer gains one result, `oversized`: the work exceeds the class
   triage ruled, and the run returns to triage once. (e) The docs-only
   fact decision 0039 computes is computed over a returning implement's
   delta as well, and a return whose delta lies wholly in the
   repository's docs class re-enters review without verify — the edge
   0039 priced, moved to the smith who now owns every fix. The
   constitutional lint is unchanged: no return draws an edge past review.

   **Enforcement binding:** the recipe tables and
   `bundles/self/policy.json`; `BOOLEAN_INPUTS` in
   `crates/brokkr-core/src/policy.rs` gains `spec_defect`; the aggregate
   in the engine; the engine's docs-class input at implement's ruling;
   a table test per arm in `brokkr-core`; the lint test unchanged and
   still passing on every shipped table.

6. **Triage is a phase, a gate, and a chief's office.** The seat is
   `triage`, class gate, chain as ruling 2, one attempt, 1,800 seconds.
   It reads the commission, the realm's house rules and the tree the
   commission names. It reads no journal, no fleet and no history — fresh
   and blind, 0020's temperament — and what Muninn knows reaches it only
   as text inside the commission. It writes the framing intake writes
   today and rules one class from a closed vocabulary:

   | Class | Route | What the class implies |
   |---|---|---|
   | `chore` | implement, verify, review, ship | cheap labour, a frontier judge |
   | `feature` | implement, verify, review, ship | the default crew, a panel on review |
   | `design` | design, then feature's path | a council first, spec-compliance on the panel |
   | `engine` | design, then a panel with a chief on review | the heaviest crew: core, store, contracts, policy |
   | `escalate` | park | incoherent, touches a frozen surface, or should be split |

   The table routes on the result; `escalate` parks with the reasoning as
   the park reason, so a refusal is a journal fact and not a sentence in
   a framing. Triage subsumes intake in any recipe that seats both. The
   class is a journal fact: the fold exposes `strategy`, an engine-owned
   input of enumerated kind that a rule may test as `strategy_in`; a seat
   that claims it has the claim dropped, and a bundle that declares it
   fails compilation (0007). Shipped as `recipes/triage`, the routing
   form: phases triage, design, implement, verify, review, ship, done,
   stop, with `fast`'s constitution below triage and ruling 5's edges.

   **Enforcement binding:** `agents/triage.json` and
   `agents/charters/triage.md`; `recipes/triage`;
   `crates/brokkr-runtime/tests/triage_shape.rs` — triage precedes every
   path, each class has a rule, `escalate` parks, `strategy` is
   engine-owned; the fold in `brokkr-core`; the witness pin.

7. **A seat may select its hire by the strategy.** A fourth seat body
   beside single, panel and sequence:
   `"select": {"on": "strategy", "cases": {…}, "default": {…}}`. Each
   case is a full seat body. Compilation requires every triage class to
   resolve to a case or the default, and runs every check — 0016, 0021,
   0031, 0035, 0036, 0040 and ruling 3 — over every case. The run
   manifest pins the cases as `run-manifest.v7` beside v6, absent when
   no site selects, so a bundle that selects nothing keeps its identity;
   `phase/entered` carries the case taken. With this, `ember`,
   `crucible`, `sdd` and `sdd-paranoid` become cases of `recipes/triage`
   — the review seat selects a single reviewer for `chore`, a panel for
   `feature`, a panel with a chief for `design` and `engine` — and the
   operator stops choosing a crew by hand. Recipes that differ in
   attempts or table stay recipes: `night-shift` extends `triage` with
   its limits; `fast` stays the quickstart, proof on screen in one
   command with no seat before the first; `preflight` and the wager
   harnesses stay explicit, because a wager must force the crew.

   **Enforcement binding:** the site parsers in
   `crates/brokkr-runtime/src/bundle.rs` and `compose.rs`; seat
   resolution at phase entry in the engine;
   `contracts/run-manifest.v7.schema.json` and
   `crates/brokkr-runtime/tests/frozen_contracts.rs`; the compose and
   witness pins; `brokkr compile` prints the cases.

8. **A charter is three texts, and the engine renders two of them.** The
   office lives in the library and names no repository: no toolchain
   command, no path, no decision number of this repository. The house
   rules are the realm's: `forge.realms/v3` adds one optional field per
   realm, `house`, a repository-relative path to a Markdown file the
   engine renders into every seat prompt under `## House rules`, its
   digest pinned in the run manifest beside the realms pin, absent when
   the realm names none. The result contract is rendered by the engine
   from the site's own vocabulary — a non-final sequence step sees its
   own results, the final step sees the seat's — and the three charters'
   disclaimer is deleted.

   **Enforcement binding:** `contracts/realms.v3.schema.json` and the
   frozen-contracts test; the prompt assembly in
   `crates/brokkr-protocol/src/adapters.rs`; `roster.rs` walks
   `agents/charters/` for the repository tokens it lists (`cargo`,
   `crates/`, `bundles/self`, `decision 00`) and refuses them; a test
   that a non-final step's prompt carries its own vocabulary and not the
   seat's.

## Consequences

- **What dies.** Reviewer commits, and with them the `fixes_applied`
  input, the `REVIEW-CLEAN` re-verify arm, the exhausted-unfixed arm and
  0039's review-fix arm. The sonnet link on every gate. Three agents,
  two tool grants, one false description, three README claims, three
  disclaimer paragraphs. The intake prose signal, replaced by a typed
  `escalate`. Every change inside a run becomes an implement commit,
  which makes the 0038 patch map the smith's alone and simplifies the
  drift gate to one phase that moves the head.
- **What it costs.** A finding above low buys a heat: implement, verify,
  review. A typo never does. The judge seats on fable-5-1 at xhigh cost
  roughly twice opus-5 at list on every token class, plus the reasoning
  the effort spends; on the operator's subscription the journaled figure
  is counterfactual list, as decision 0031's correction records. A
  triage seat adds one gate read at the front of every run that seats
  it, against the price of running the wrong strategy — a $33 run that
  should have been $6, or a $6 run that should have had a council.
- **Two decisions amended by one word each.** Decision 0040 ruling 5's
  first link moves from high to xhigh (ruling 2). Decision 0039 keeps
  its engine-owned input and loses its motivation: a judge that cannot
  fix has nothing to price, and the fact it computes now serves the
  smith's return (ruling 5e).
- **Enactment, in order, after the hands landing shares its agent
  files.** (i) Rulings 1 to 5: adapters, agents, charters, tables, the
  judges list, the gate-head check — one slice, and it moves every
  digest. (ii) Verifier and shipper as boxed exec, decision 0040's own
  consequence; it may ride with (i). (iii) Ruling 6, the triage agent
  and the routing recipe. (iv) Ruling 7, select seats and manifest v7.
  (v) Ruling 8, realms v3, house rules and the per-step contract. The
  SDD dialects — which artifacts the design council writes and which
  tool validates them — are their own proposal. Decision 0025's grant
  vocabulary may later enumerate the triage classes a standing executor
  may carry unattended; that is 0025's amendment, not this one's.
- **Deliberately unruled.** The agent's name (`triage`, or `strategist`
  if the operator wants the seat named by what it rules); dsh efforts,
  which decision 0035 measured as a label rather than a lever; whether
  `panel-review`'s flat join survives as a case or retires; and the
  effort figures themselves, which are pins the operator amends by
  evidence the way 0021 amends a tier.

## Addendum — 2026-09-04, the enactment's reading, confirmed by the operator

Ruling 2 classed the design chief as a gate while ruling 4 forbade gates from making the commits that the chief's accepted charter requires.
The enactment reads an artifact author as a work seat and its boxed validator as the gate, so the chief keeps its chain but the engine guards only the judging step.
Decision 0042 ruling 6 grounds this reading by assigning the `specify` and `design` artifacts and exactly those commits to the chief while forbidding every gate from touching an artifact.
The operator confirmed the reading in chat on 2026-09-04 ("confirm it"): the class column of ruling 2's table names the hiring grade, an office that authors an artifact is a work seat, and the boxed validator that follows it is the phase's gate. No ruling's text changes.

## Addendum — 2026-09-06, operator ruled: a hedge forces its crew, as a wager does

Ruling 2 makes the library the roster and ruling 7 leaves inline model
sites only where a recipe must force its crew — "a wager must force the
crew", in ruling 7's own words. On 2026-09-06 the operator asked for a
hedge: a shipped recipe that delivers without touching one vendor's
account at all, after that account's exhausted limit parked a live run
twice in one day while the other vendor was serving throughout.

A hedge forces its crew for exactly ruling 7's reason. Its purpose is
the vendor it does *not* use, and a library chain would undo that
silently at its first fallback — the fallback is the feature everywhere
else and the defect here. `recipes/standby` therefore joins the inline
exception: `fast`'s shape and contracts unchanged, its two model seats
pinned inline to the other vendor, its verify and ship gates the same
boxed exec scripts.

Two things this addendum does not do. It does not widen the exception
to any recipe that would merely prefer a vendor: a recipe that can take
a fallback belongs in the library, and the test names the hedge by name
rather than by a pattern. And it is not the end state — the reason the
hedge cannot be a library crew is that codex expresses no per-tool
allow-list, so no office holding one can resolve on it (decision 0045
ruling 4), and the smith holds `cargo` and `git`. When an implementer's
hands are boxed the way the review offices' already are, the allow-list
is not consulted at all (decision 0043 ruling 2), the smith becomes
hireable on any provider that puts its hands in the box, and this
recipe should retire into a library crew. That is its own slice.

**Enforcement binding:** `crates/brokkr-runtime/tests/roster.rs` names
`standby` in the inline exception with this addendum as the reason;
`recipes/standby` compiles and its seats keep `fast`'s result
vocabularies and inputs, so the inherited policy table rules on them
unchanged.
