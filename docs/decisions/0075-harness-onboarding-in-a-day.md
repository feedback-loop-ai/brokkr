# 0075 — Harness onboarding in a day: a harness is a spec, a probe measures it, a new seat starts provisional, and evidence promotes it

Status: proposed
Date: 2026-09-28

## Context

The agent space moves weekly. A vendor ships a new CLI (Gemini's, Grok's,
the next one), or a much better model lands on a provider Brokkr already
reaches, and the operator wants it seated the same week, ideally the same
day. The operator raised the question on 2026-09-28: "it could be gemini,
could be grok, it is a dynamic space". What would it take to add one?

Today the answer is about a week, spread over three separate costs.

**The code.** #347 measured about 20 edits across 6 crates to add a
harness:
- an `AdapterKind` variant, whose `supports()` default claims resume nobody
  decided;
- a launch builder and argument splitters;
- an invoke arm, an event folder and a refusal classifier;
- a `BROKKR_<H>_BIN` override;
- transcript kind and home resolution;
- the runtime's `built_in_model_driver` match, without which the new
  harness's model pins go unpoliced;
- the CLI's known list, `doctor` and the UI's discovery;
- the view's transcript projector;
- and a **new seat-record contract version**, because `transcript.kind` is
  a closed enum.

Only the model catalogue and the permission flags are data. #264 is the
same failure class: harness facts that live as Rust constants.

**The facts.** Every adapter field is a dated, measured claim. For a new
CLI they are found by a hand spike, and review rounds then find the ones
that were guessed:
- how it reports usage (#402 found Claude's counts double-counted per
  stream event);
- whether it can switch off its own tools and take an MCP server;
- what native egress it carries (#462 found dsh's web tools on in every
  realm);
- whether it inherits the operator's personal configuration (#467 found
  every unboxed Claude seat and every Codex seat starting the operator's
  MCP servers, with their credentials).

**The seat.** A new harness enters the roster the way Astra did: a
measurement, a decision (0045), and a recipe edit. It is promoted, or not,
on impression.

**The fast lane.** For a new *model*, rather than a new harness, it exists
already: a model reachable through OpenRouter or an OpenAI-compatible
endpoint becomes a dsh route in hours. But dsh declares
`hands: unsupported`, so such a seat records the `not applicable`
boundary and cannot hold a boxed office (#290).

**Alternatives weighed:**
- *Keep adding harnesses by hand.* This is proven, and it costs a week and
  a review arms race each time. It scales with the space's pace only if
  the operator stops wanting new things.
- *Route every new model through one multi-provider harness only.* This is
  the fastest, but it forgoes a vendor CLI that is materially better than
  the generic loop, and the space's best agents have so far been vendor
  CLIs.
- *Make a harness data, measure it with a probe, and seat it provisionally
  until evidence promotes it.* This is the one ruled here. It costs a
  refactor on the critical path (#226 → #348 → #347) and one new tool (the
  probe, #484), and it turns each later harness into a day's work.

## Rulings

1. **A harness is a declarative spec plus at most one module.** The adapter
   file declares everything today's code restates per harness: the launch
   grammar, the mapping from the CLI's events to the journal, its refusal
   classes, effort, the tool-restriction and MCP mechanism, the transcript
   location and kind, and the resume shape. Code is allowed only where data
   cannot express the harness, and then in one module under the harness's
   name, with no edit elsewhere. A default that claims a capability (resume,
   hands, effort) is refused: every capability is declared.
   *Binding:* #347's acceptance. A test enumerates every `AdapterKind` and
   refuses a harness fact kept as a Rust constant outside its module (#264's
   class). The adapter loader refuses a harness missing a spec field.

2. **The transcript kind is open.** A seat record names its transcript kind
   from the harness's adapter, validated against the adapters loaded, not
   against a closed enum in a frozen contract. So a new harness needs no
   contract version. The change is minted through the contract lifecycle
   (#361), once.
   *Binding:* the seat-record schema accepts a declared kind; a test seats a
   fixture harness unknown to the schema's author and its record verifies.

3. **Every adapter fact is measured by the conformance probe.** The probe
   (#484) runs a CLI against a scratch repository and records each fact the
   spec declares, with the CLI's version, the host, the date and the
   evidence. A fact the probe cannot measure is `unmeasured`, as dsh's
   native capabilities are today, never assumed. The probe is proven against
   the three shipped harnesses, reproducing their adapters' measured fields,
   before it is trusted on a new one. Live probing needs real credentials,
   bound through decision 0012. It is an operator host step, never CI, and
   the no-real-providers gate still holds for the repository.
   *Binding:* a test that every adapter field carries a measurement reference
   to a committed probe report. Re-running the probe on a new CLI version
   reports drift against that report.

4. **Eligibility is derived, not argued.** From the probe's measured facts:
   - a harness whose own tools can be switched off and which takes an MCP
     server may hold boxed offices (decisions 0043 and 0046);
   - one that cannot may hold unboxed or tool-less offices only;
   - one whose native egress has no measured off switch is refused in every
     realm that has not granted that capability (decision 0065 ruling 4,
     restated here for new harnesses);
   - one whose user-scope configuration cannot be isolated (#467) is
     refused.
   *Binding:* the existing boundary and capability compile refusals consume
   the adapter's measured fields; a fixture harness of each kind is refused
   or admitted exactly as stated.

5. **A new harness, or a new model on an existing one, starts provisional.**
   A provisional seat may hold only the offices the operator lists for
   provisional seats, for example review-panel positions, research, and
   wager arms. It never holds a gate (a review chief) or the sole implementer
   of main-bound work until promoted. Its per-token price rows exist before
   its first seat, so its cost is visible from the first run.
   *Binding:* an adapter or route field `tier: provisional | promoted`; a
   compile refusal for a provisional seat in a gate or sole-implementer
   office; LaneTally's unpriced audit shows no provisional route unpriced.

6. **Promotion is on evidence, by the operator.** A provisional seat is
   promoted after a wager (#237) on real stories. The wager compares it with
   the incumbent on cost per accepted change (#224) and on review holds and
   rounds (#436). The promotion is a roster addendum, not a new decision per
   harness.
   *Judgment-guidance:* the operator's ruling, recorded in the roster
   addendum with the wager's run ids. No mechanism promotes automatically.

7. **The fast lane stays fast, and gains hands.** A new model reachable
   through an existing multi-provider harness is added as a route:
   - its alias and effort table;
   - one `BROKKR_BLESS=1` re-pin (#358);
   - its price rows;
   - a probe run;
   - and the provisional tier.

   The target is hours. dsh gains hands (#290), so that this lane can reach
   boxed offices.
   *Binding:* #290's acceptance, and ruling 4's eligibility applied to dsh's
   measured facts.

## Consequences

- **Targets:**
  - A new model on an existing harness is seatable, provisionally, in
    **hours**.
  - A new CLI whose shape matches one already supported is seatable,
    provisionally, in **one day**. The probe writes its facts, and no crate
    changes beyond its one module.
  - A new shape takes longer, and pays for that shape once.
- **Critical path:** #226 (safe resumption across harnesses) → #348 (split
  brokkr-protocol) → #347 (a harness is one module). Rulings 1 and 2 wait on
  it. Rulings 3 to 7 do not: the probe (#484), config isolation (#467),
  native-capability declarations (#319, #462), dsh's hands (#290) and the
  provisional tier can start now.
- **What it costs:**
  - one new compile concept (the tier);
  - a probe that needs live credentials on the operator's host;
  - a contract version for the open transcript kind, minted once.
- **What it changes for existing seats:** nothing until each ruling lands.
  claude, codex and dsh are promoted today by their current rosters.
- **Tracking:** epic #485.
- **Related decisions:** 0012 (secret bindings), 0036 (egress is a property
  of the route), 0043 and 0046 (hands and the boundary), 0045 (the Astra
  roster, the pattern this replaces), 0065 (capabilities are the realm's to
  grant), 0071 (the architectural principles).
