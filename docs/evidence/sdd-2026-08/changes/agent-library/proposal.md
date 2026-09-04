# Change Proposal: agent-library

**Spec**: [`specs/agent-library/spec.md`](../../../specs/agent-library/spec.md)
· **Plan**: [`specs/agent-library/plan.md`](../../../specs/agent-library/plan.md)
· **Tasks**: [`specs/agent-library/tasks.md`](../../../specs/agent-library/tasks.md)

## Why

A seat inlines everything it is — charter path, driver argv, limits,
declared inputs — and the duplication that creates is already drifting.
Measured in this tree: `shipper.md` is one text copied into four
recipes, `verifier.md` one text copied into four, while
`implementer.md` exists in three distinct byte-versions, `intake.md` in
two and `review-security.md` in two. Nothing in any bundle names a
model, so every run takes whatever the provider CLI defaulted to that
day and no readout records which that was.

Decision 0016 (`docs/decisions/0016-agent-library.md`, accepted
2026-08-29) rules the fix: define an agent **once** — description,
charter, an ordered chain of abstract model names, abstract tool and MCP
configuration, decision-0006 limits, decision-0007 declared inputs — let
seats reference it, and map it onto providers through operator-editable
**data** adapters. The ruling's own frame is the acceptance bar, in the
operator's words: *"take the adapter claim with a pinch of salt."*
Providers are not interchangeable. A slice where every test passes but a
reader of the output could believe two providers are interchangeable is
a failed slice — so this change spends its structure on the honesty
rules and buys nothing that is merely convenient.

## What Changes

- **A repo-root `agents/` library** — one JSON definition per agent, 16
  of them over 14 charter files `git mv`'d out of `bundles/self`,
  `recipes/panel-review` and `recipes/sdd` with zero content change.
  Repo root because `manifest_for` walks the *bundle* dir; one file per
  agent because `brokkr agents list` mirrors `brokkr recipes list`, whose
  contract is *warn on a broken entry, never abort*.
- **A repo-root `adapters/` directory** — one file per provider
  (`claude`, `lanetally`, `codex`, `dsh`, `exec`) declaring the driver
  invocation, the abstract→concrete model mapping, how tool permissions
  and MCP servers are expressed, and — the load-bearing part — which of
  those the provider **cannot express**, declared explicitly as
  `"unsupported"` rather than inferred from an empty map. Composition is
  a lookup and a join: no template language, and no Rust match arm over
  provider names is ever written. `brokkr-protocol/src/adapters.rs` is
  not touched — its match arms are stream-format parsers, and everything
  after `--` is already passed through verbatim.
- **Seats reference agents.** `"agent": "<name>"` at seat, panel member
  and sequence step, as a fourth alternative in the existing
  exactly-one-of check. The reference is **total**: no seat may override
  the charter, driver, limits or inputs it names, because a seat that
  could amend its agent makes `brokkr agents show` a lie for that seat.
  Inline seats stay first-class — `recipes/sdd`'s `speckit-check` step
  stays inline and is the case that proves the library is an option.
- **One pure resolver** in `crates/brokkr-runtime/src/agents.rs` over
  *(library, adapters, availability)*, unit-tested with nothing spawned
  — guaranteed by the signature, since availability is an argument and
  the module touches no PATH, filesystem or clock. `Bundle::compile`
  passes availability `unspecified`, so the same bundle resolves
  identically on every machine; a compile that probed PATH would give
  one bundle two digests and make an in-flight run unresumable after an
  `apt install`.
- **Pinned resolution.** The bundle manifest — which is the run manifest
  — gains one `agents` key, **absent** when no seat references an agent,
  carrying per invocation site the agent, charter and adapter digests,
  the full chain, the chosen index, the skipped entries with a closed
  reason vocabulary, and any notices. `chosen_index` is what gives
  honesty rule 2 something to hang on: a `model` field alone never says
  the operator did not get their first choice. Names and digests only —
  never resolved argv, which would embed `{brokkr}`'s machine-local
  expansion into a digest. Published as a new
  `contracts/run-manifest.v3.schema.json` beside the frozen v1.
- **The honesty rules, enforced.** Capabilities split into **grants**
  (an MCP server the provider lacks — the agent gets less power, and
  `optional` is for exactly this) and **restrictions** (a tool-permission
  narrowing the provider cannot express — the agent gets *more* power).
  `optional` is structurally unrepresentable on a restriction, matching
  is per named item rather than per class, and the checks run over
  **every entry in the chain**, not just the chosen one — because a
  chain whose second link cannot express the agent's restrictions would
  silently widen its blast radius the moment it fell back.
- **Bounded fallback on a structural predicate**: `Failed` **and** never
  `Accepted` **and** no checkpoint. No stderr sniffing, no
  "model not found" regex — the engine never pattern-matches a
  provider's prose to make a control decision. The predicate *is* the
  mid-session boundary mechanised: once `Accepted` arrives, fallback is
  unreachable by construction. The chain index is derived from journaled
  facts, so a crash between attempts cannot change which model runs
  next, and `max_attempts` is untouched.
- **Provenance in every readout.** Optional, absent-by-default
  structured payload fields at `event_schema: 1`, published as a new
  extension schema, with `contracts/README.md`'s prose rule amended in
  the open and with its reason — narrowed to fields that are optional,
  absent by default, and never read by `fold`. Provenance is a **list
  keyed by invocation site**, because a panel runs N models inside one
  attempt and `recipes/sdd`'s design seat spans `claude` and `exec`;
  "per-attempt" taken literally would print a false model in every
  readout. `brokkr-view` derives it once, four surfaces render it, and
  `brokkr compare` gains a first-class `resolution_divergence` reported
  unconditionally — including when `same_recipe` is `true`.
- **CLI.** `brokkr agents list` and `brokkr agents show <name>` mirroring
  `brokkr recipes`; `brokkr doctor` reports providers, binaries, probe
  results and declared models **read from the adapter files**, and per
  agent which model would be chosen on this machine — computed by the
  same resolver, which is what gives its availability tri-state a real
  consumer.

## Impact

**Byte-identity holds where it must.** `recipes/fast` and
`bundles/verify` adopt nothing; their `manifest_digest()` is pinned by a
golden landed **before** any production edit, their `agents` key is
absent, and their journals gain no payload field. Adopting recipes'
digests do move — a charter leaving a recipe dir leaves
`manifest.files`, and `charter_digest` in the new manifest key is the
pin that replaces it, so adoption does not lose content pinning.

**Adopting recipes will run a different model on purpose.** They pass no
`--model` today and take an invisible provider default; resolved seats
pin an explicit concrete id. That is strictly more honest and it is the
point of the feature.

**Two named limits ship with this change, in writing.** First, a
Looper-dispatched run cannot adopt agents: `bundle_manifest_from_run`
reconstructs the bundle manifest from six named keys and drops the rest,
so an `agents` key would be silently dropped on the v2 round-trip and
every adopting Looper run would become unresumable with a diff blaming
no file. Rather than widen a contract a counterpart system reads,
`build_run_manifest_v2` **refuses** an adopting bundle with an error
naming the limitation; lifting it needs a jointly agreed v2-lineage
manifest version. Provenance likewise does not cross the bridge in this
slice — asserted by a test that names the ruling, because a
half-updated boundary is how a surface quietly stops telling the truth.

**Second, one third of the fallback trigger is deliberately not
delivered.** Decision 0016 names three fail-to-start cases. The
structural predicate covers two — a driver that fails to spawn, and a
driver reporting a determinate failure before accepting, which is the
shape a model rejection takes for every driver we own. The third, a
driver that exits without accepting and without a result, is
`AttemptOutcome::Indeterminate` today, and decision 0003 rules that it
parks because the forge cannot tell "did nothing" from "already opened a
billed session". Reclassifying it to satisfy 0016 would make an
indeterminate-shaped condition auto-retry behind a feature flag, and a
bound that applies "unless a new feature is in play" has stopped being a
bound. The honest mitigation is at the driver: drivers we own should
report a provider's pre-session model rejection as a determinate
failure, which converts case 3 into case 2.

**What this change does not deliver is stated as plainly as what it
does.** The library ships 16 definitions, several near-identical,
because the charters and argv they replace are genuinely different
today. This slice reduces *definition* duplication, not file count;
collapsing the near-duplicates is decision 0017's job. The `-speckit`
suffix is ugly on purpose — it puts the drift on the surface where
`brokkr agents list` shows it every time, instead of laundering it
through a merge system that could rewrite an adopting recipe's prompt
bytes silently.

**Untouched**: the frozen v1 contracts' bytes, `run-manifest.v1` and
`v2`, `fixtures/evaluator/corpus.ndjson`, `policy/phase-machine.json`,
`reference/`, the secrets machinery, `brokkr-protocol/src/adapters.rs`,
and `brokkr-store`. No new dependency. The workspace suites and the exact
100% line/branch/function coverage gate stay green.
