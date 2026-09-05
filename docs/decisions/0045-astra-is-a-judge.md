# 0045 — Astra is a judge: the codex lane has a peer of fable, and every gate crosses the vendor line

Status: proposed
Date: 2026-09-05

## Context

On 2026-09-05 the operator named a new model, GPT-6 Astra, asked for it
to be embedded in the workflows, and asked for a full re-evaluation of
the agent roster under one premise: Astra is on par with Fable in
ability. This decision is that re-evaluation. It re-cuts the roster
decision 0041 ruling 2 tabled, keeps 0041's spend rule, and adds one
rule 0041 could not state, because 0041 had one frontier vendor and now
there are two.

What was measured before anything was written, cited:

- **The model is catalogued and served.** codex-cli 0.153.2's model
  catalog (`~/.codex/models_cache.json`) lists `gpt-6-astra` at priority
  1, visibility `list`, `supported_in_api` true, a 272k context, and
  reasoning levels `low`, `medium`, `high`, `xhigh`, `max` and `ultra`
  with `medium` as the default. The operator's own `~/.codex/config.toml`
  already names it as the default model. A probe the same day —
  `codex exec --model gpt-6-astra -c model_reasoning_effort=low` —
  answered, and the thread record under `~/.codex/sessions/` carries
  `"model":"gpt-6-astra"` on every turn context and `"effort":"low"`
  beside it: the served fact decision 0031 asks for and the configured
  fact decision 0035 asks for, both from the harness's own record.
- **The roster has one vendor at every gate's first hire.** Every gate
  in the library opens on a claude model: triage, the chief-architect,
  the analyst, the single reviewer, the review chief, the adversarial
  and security members on `fable`; the clarifier and spec-compliance
  member on `opus`. Only the correctness member and the robustness
  position open on codex's `sol`, hired at `high` as diverse members a
  chief checks. When the account's Fable limit was exhausted on
  2026-09-04, every gate that opened on fable died twice and parked —
  the agent-backed chains included, because the claude driver reports
  `accepted` at spawn and the limit refusal arrives after, which
  decision 0016 excludes from fallback. A roster whose gates all open
  on one vendor has that vendor's outage as its own.
- **The smith and its final judge share a vendor.** The implementers
  are `opus` then `sonnet`, the engine-class implementer `fable` then
  `opus`; the last judge before ship — the single reviewer in `fast`'s
  family and the self bundle, the review chief in the design and engine
  panels — is `fable`. Two models of one training house share blind
  spots the way two readers of one textbook do. Decision 0041 ruling 2
  wanted "different model families" on a panel for exactly this reason
  and then counted `fable` and `opus` as two.
- **Codex judges nothing yet, on the record.** The canonical journal
  holds 136 runs. The served models on successful effects are fable-5-1
  (98), opus-5 (37), sonnet-5 (35), haiku (12), one qwen lane and one
  opus-4.8; no `gpt-*` model has been served at any seat under decision
  0031's field, because the wager that earned codex its tier (0021's
  addendum of 2026-09-02) predates the field. Codex holds the trusted
  tier for every seat class and its adapter names `sol` as a judge, and
  no library gate has been served by it since. The premise this
  decision hires on is the operator's, confirmed by the catalog; the
  journal will confirm or refute it.
- **A codex lane can hold only a boxed or tool-less office.**
  `adapters/codex.json` declares `tool_permissions` unsupported, with
  the measured reason: codex restricts by sandbox class, not by tool
  name. The resolver refuses an agent whose `tools.allow` must map onto
  that provider. So `astra` can chain into every office that declares
  boxed hands (decision 0043) or none — the review offices, triage, the
  positions, the analyst, the clarifier, the chief-architect — and into
  none that keeps an allow-list: the implementers, intake, the
  researcher. That is not a limit this decision sets; it is one the
  tree already enforces and this decision records.
- **The vocabulary drifts at the edges.** The codex adapter's `efforts`
  carry `none` and `minimal`, which the 0.153.2 catalog lists for no
  model, and omit `ultra`, which it lists for astra, sol and terra as
  "maximum reasoning with automatic task delegation". No shipped pin
  names any of the three. The catalog also names a paid `fast` service
  tier for astra. These are measured and left alone below.

Alternatives weighed:

- **Rename `sol` to astra, as 0041 ruling 1 renamed `fable`.**
  Rejected. Ruling 1 governs generations of one family name; `sol`,
  `terra` and `luna` are codex's own family words and astra is a fourth,
  not sol's successor by name. The catalog keeps all four listed. An
  agent that hires `sol` says so legibly, which is the property ruling
  1 protects.
- **Astra first on every gate.** Rejected: it recreates the single-
  vendor roster with the vendors swapped, and the security member's
  first hire on the other vendor is what keeps every panel diverse at
  its first hires (ruling 3) and keeps one vendor's outage from idling
  a whole panel.
- **A wager before the hire.** Rejected as the gate for this decision,
  not as an instrument. Decision 0021's wager road is for a driver's
  tier, and codex already holds the trusted tier for every seat class;
  which of a trusted driver's lanes may judge is 0041 ruling 3's
  `judges` list, an operator ruling recorded in adapter data. The
  implementer is the seat this decision does not move, and it is the
  seat the wager harness weighs; that wager is named in the
  consequences.
- **Astra as the implementer.** Deferred, for the reason above and one
  more: the implementers hold tool allow-lists and no boxed hands, so no
  codex lane can resolve there today. Boxing the smith is decision
  0043's road, not a roster line.

## Rulings

1. **`astra` is a codex lane and a judge.** `adapters/codex.json` maps
   `astra` to `gpt-6-astra` beside `sol`, `terra` and `luna`, and its
   `judges` list reads `["astra", "sol"]`. The mapping is transcribed
   from the installed catalog, as the three before it were. No other
   adapter changes: `lanetally` wraps claude and maps nothing of
   codex's.

   **Enforcement binding:** the adapter file;
   `crates/brokkr-runtime/tests/library_data.rs` pins four catalogued
   lanes and no invented ones;
   `crates/brokkr-runtime/src/bundle/model_policy_tests.rs` pins the
   judges list and the `astra` mapping; `brokkr doctor` reports the
   lane as served.

2. **The roster, re-cut: the last judge before ship is hired across
   the vendor line from the smith.** Decision 0041's spend rule stands
   — spend goes to the seat whose error nobody downstream can see — and
   this one sits beside it. The smiths are claude, so the offices that
   give the final verdict on a smith's work open on astra. The offices
   that read before the smith, or judge beside another judge, keep
   fable first and take astra as their first fallback, so every chain's
   first step down crosses the vendor line. Efforts are 0041's; a first
   link is still never hired below its fallback, with triage's ruled
   exception.

   | Office | Hire | Fallback | Class |
   |---|---|---|---|
   | `reviewer` (the last judge in a recipe without a panel) | astra xhigh | fable xhigh, then opus xhigh | gate |
   | `review-chief` (the last judge over a panel) | astra xhigh | fable xhigh, then opus xhigh | gate |
   | `review-adversarial` | astra xhigh | fable xhigh, then opus xhigh | gate |
   | `review-security` | fable xhigh | astra xhigh, then opus xhigh | gate |
   | `triage` | fable xhigh | astra xhigh, then opus max | gate |
   | `chief-architect` (an author, by 0041's addendum) | fable max | astra max, then opus max | gate-grade work |
   | `analyst` (decision 0042's drift judge) | fable xhigh | astra xhigh, then opus xhigh | gate |
   | `clarifier` | opus xhigh | sol xhigh | gate |
   | `review-correctness` | sol high | opus high | gate |
   | `review-spec-compliance` | opus high | sol high | gate |
   | `position-simplicity` | opus high | sol high | work |
   | `position-robustness` | sol high | opus high | work |
   | `implementer`, `implementer-sdd` | opus high | sonnet high | work |
   | `implementer-engine` | fable high | opus high | work |
   | `intake`, `intake-sdd` | sonnet high | opus high | work |
   | `researcher` | fable high | opus high | work |
   | `verifier`, `shipper` | boxed exec, no model | — | gate |
   | `muninn` | opus high | — | — |

   The rows below `clarifier` are unchanged from 0041 and are tabled so
   the roster is read in one place. `sol` keeps the seats it earned as
   the cheap diverse member a chief checks; astra takes the seats where
   an error is invisible downstream and the smith is claude.

   **Enforcement binding:** the seven moved agent files
   (`reviewer`, `review-chief`, `review-adversarial`, `review-security`,
   `triage`, `chief-architect`, `analyst`);
   `crates/brokkr-runtime/tests/adoption.rs` pins the self bundle's
   review site on `gpt-6-astra` with codex's boxed hands in its argv;
   the witness and compose pins move for the bundles that hire a moved
   office; the `brokkr agents list` transcript in
   `docs/guides/agent-library.md`.

3. **Panel diversity is vendor diversity, counted at the first hire.**
   Every shipped panel seats two adapters' models among its members'
   first hires. Two models of one house at one effort argue with
   themselves; a fallback that might cross the line is not a member
   who did. This reads 0041 ruling 2's "two model families" the way
   its own reasoning meant it, and it is why the security member stays
   on fable while the chief moves to astra: the feature panel is sol
   and fable, the design panel adds opus, the engine panel adds astra,
   the design council is opus and sol.

   **Enforcement binding:** `crates/brokkr-runtime/tests/roster.rs`
   keys each member's first hire by the adapter that maps it — never
   by a substring of the name — and refuses a panel whose first hires
   sit under one provider; the inline arm of a panel names its driver
   after `driver` and counts as that provider.

4. **A codex lane is chained only into boxed or tool-less offices.**
   Recorded, not invented: codex maps no tool names, so an office with
   a tool allow-list cannot resolve on it and the compiler refuses the
   first bundle that hires it. The roster never writes such a chain.
   The road for the smiths is decision 0043's — an implementer with
   boxed hands can be hired on any provider that puts its hands in the
   box — and that is a slice with its own evidence, not a line here.

   **Enforcement binding:** the existing resolver refusal in
   `crates/brokkr-runtime/src/agents.rs`; `roster.rs` pins that every
   agent naming a codex lane declares no `tools`, and that the roster
   chains codex into at least eight offices.

5. **The hire rests on the premise; the journal is the scorecard.**
   The operator's word that astra is fable's peer, and the catalog that
   ranks it first, are the evidence line for the `judges` entry today.
   What confirms or refutes it is what decision 0021 ruling 3 already
   names: served astra gates in the canonical journal, their verdicts
   against the operator's own reading, their cost beside fable's at the
   same effort, and `compare` across runs. No number of runs is fixed
   here; the operator reads the ledger and amends this table the way
   0021 amends a tier.

   **Enforcement binding:** none of its own — judgment guidance, cited
   when the next roster amendment is argued. The instruments exist:
   `brokkr costs`, `brokkr seats`, `brokkr compare`, and the served
   `model` field every codex record now carries.

## Consequences

- **What moves.** Every bundle that hires a moved office changes
  identity: `recipes/triage` and everything that extends it,
  `recipes/night-shift`, `recipes/panel-review` (its security member),
  and `bundles/self`, whose review seat now opens on codex with boxed
  hands in its argv. The inline recipes (`fast`, `node`, `preflight`,
  the wager harnesses) and `bundles/verify` pin only the claude adapter
  at their gates and do not move. The self-forge loop's own reviews are
  the first astra gates the journal will carry, which is the embedding
  the operator asked for: the shop's next pull requests are judged
  across the vendor line.
- **What it costs.** Unmeasured here. No list price for astra was
  fetched, and the journal's counterfactual figures (decision 0031's
  correction) will carry the answer beside fable's at the same effort.
  The reasoning meter of decision 0035 ruling 4 is what settles it.
- **What survives an outage.** A fable limit no longer stops every
  gate: the single reviewer, the chief and the adversarial member open
  on the other vendor. It still stops the security member, triage, the
  analyst and the chief-architect at their first link, because the
  driver's accepted-then-failed report cannot advance a chain; the
  honest fix stays the one `docs/guides/agent-library.md` names as
  limit 3 — classify a pre-session refusal as a determinate start
  failure at the driver — and it is not ruled here.
- **The wager that follows.** The wager harness's codex arm stays
  pinned at `gpt-5.6-sol`, because its README's claim is the
  reproduction of the recorded wager. The lawful astra wager is a
  scratch copy of that recipe with one token changed, run against
  `fast` on the same feature and compared; its outcome is the evidence
  for moving the implementer, and until then the implementers are
  claude's by ruling 4's fact.
- **Two decisions read, none amended.** 0041 ruling 2's table is
  superseded by ruling 2's here, in the rows it names; its rules and
  bindings stand. 0043's chain sentence ("the review gates hire
  fable@high → opus@xhigh → sol@xhigh") was already amended by 0041
  and is superseded again; the index row records it.
- **Deliberately unruled.** The effort figures, still pins the operator
  amends by evidence. `sol`'s future once astra's record exists.
  Whether the positions or the clarifier take astra. The codex
  vocabulary's `none` and `minimal`, which the catalog lists for no
  model, and `ultra`, which delegates inside a judge and wants the
  hands question answered first. The `fast` service tier, a price
  lever and not a hire. The inline quickstart gates, which 0041 ruling
  7 keeps explicit and which still cannot fall back.
