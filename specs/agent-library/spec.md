# Feature Specification: The agent library

**Feature slug**: `agent-library`
**Run**: `implement-decision-0016-the-agen-86a726e5`
**Status**: Committed (design phase ruling)
**Scope**: implements decision 0016
(`docs/decisions/0016-agent-library.md`, accepted 2026-08-29). No new
decision doc. Decision 0017 (`extends`, composition) is explicitly NOT
in this slice.
**Input positions**: `.forge/design/positions/simplicity.md`,
`.forge/design/positions/robustness.md` (run-local, uncommitted)
**Intake framing**: `.forge/tasks/agent-library.md` (run-local)

## Why

A seat inlines everything it is: charter path, driver argv, limits,
declared inputs. Measured in this tree, that duplication is real —
`shipper.md` is one text copied into four recipes, `verifier.md` one
text copied into four — and it is already drifting: `implementer.md`
exists in three distinct byte-versions, `intake.md` in two,
`review-security.md` in two. Nothing today names a model, so every run
gets whatever the provider CLI happened to default to, and no readout
records which that was.

Decision 0016 rules the fix: define an agent **once** — description,
charter, an ordered chain of abstract model names, abstract tool/MCP
configuration, 0006 limits, 0007 declared inputs — let seats reference
it by name, map it onto providers through operator-editable **data**
adapters, resolve it purely at compile time, and pin the resolution.

The ruling also fixes the acceptance bar, in the operator's words:
*"take the adapter claim with a pinch of salt."* Providers are not
interchangeable. **A slice where every test passes but a reader of the
output could believe two providers are interchangeable is a failed
slice.** This spec therefore spends its structure on the honesty rules
and buys nothing that is merely convenient.

## The design in one paragraph

A repo-root `agents/` library holds one JSON definition per agent and
the charters moved out of the three adopting recipes; a repo-root
`adapters/` directory holds one JSON file per provider declaring its
driver invocation, its abstract→concrete model mappings, how it
expresses tool permissions and MCP servers, and **which of those it
cannot express at all**. One pure function in
`crates/brokkr-runtime/src/agents.rs` turns *(library, adapters,
availability)* into a resolved seat body plus a resolution record; it
never touches PATH, the clock or the network, and `Bundle::compile`
passes availability `unspecified`, so the same bundle resolves
identically on every machine. A seat, panel member or sequence step
saying `"agent": "<name>"` resolves **before** every existing lint into
an ordinary seat body, which is what makes "reference equals inline"
true by construction rather than by assertion. The bundle manifest —
which is the run manifest — gains one `agents` key, **absent** when no
seat references an agent, carrying per invocation site the agent and
charter and adapter digests, the full chain, the chosen index, and any
notices; that key is what replaces the `manifest.files` pin a charter
loses when it leaves the recipe dir. Per-invocation provenance reaches
the journal as optional, absent-by-default structured fields on
`effect/started` and `effect/failed` at `event_schema: 1`, published as
a new extension schema and legalised by an amendment to
`contracts/README.md` made in the open. `brokkr-view` derives provenance
once; four surfaces render it, and `brokkr compare` reports a model
difference as a first-class divergence. Fallback is bounded by a
structural predicate — `Failed` **and** never `Accepted` **and** no
checkpoint — which mechanises decision 0016's mid-session boundary
instead of describing it.

## Rulings on the five open questions

### Q1 — How resolved-agent provenance reaches the run manifest

**Ruling.** `manifest_for` gains a top-level `agents` object, **absent**
when no seat references an agent (the decision-0012
`if !seat.secrets.is_empty()` precedent, applied verbatim). A new
`contracts/run-manifest.v3.schema.json` is published beside the frozen
v1 — v1's bytes plus one optional `agents` property. v1 and v2 keep
their bytes.

The record is keyed by **invocation site** (`seat`, `seat:member`,
`seat:step`, `seat:step:member` — the tags the engine already uses),
not by agent name, because that is the granularity `brokkr compare`
needs and the granularity at which a chain is actually walked:

```json
"agents": {
  "design:chief": {
    "agent": "chief-architect",
    "agent_digest": "<sha256 of the canonical definition JSON>",
    "charter_digest": "<sha256 of the charter bytes>",
    "adapter_digest": "<sha256 of the adapter file bytes consulted>",
    "chain": ["fable", "opus", "sonnet"],
    "chosen_index": 0,
    "model": "fable",
    "provider": "claude",
    "skipped": [],
    "notices": []
  }
}
```

`chosen_index` is load-bearing: a `model` field alone gives honesty
rule 2 nothing to hang on — nothing in the run would say the operator
did not get their first choice. `skipped[].reason` is a closed
vocabulary (`unmapped`, `unavailable`), never prose.

**The record carries names and digests only, never resolved argv.**
This is not stylistic: `parse_command` expands `{brokkr}` to
`std::env::current_exe()`, an absolute machine-local path. Pinning
resolved argv would make `manifest_digest()` machine-dependent and
break `Engine::resume`'s `ManifestMismatch` check — the same
reproducibility failure that compile-time probing would cause, arriving
through a different door.

**Ruling on the Looper (v2) lineage — a defect found in the tree, not
in either position.** `bundle_manifest_from_run`
(`brokkr-core/src/dispatch.rs:422`) *reconstructs* the bundle manifest
from six named keys and drops everything else, and
`dispatch_from_run` re-checks `bundle_sha256` against that
reconstruction. An `agents` key would therefore be silently dropped on
the v2 round-trip, making every Looper-dispatched adopting run
unresumable with a diff that blames no file, and stripping the honesty
record at the boundary. Widening `run-manifest.v2` means changing a
contract a counterpart system reads, which is not this slice's to do
unilaterally. **Therefore `build_run_manifest_v2` REFUSES a bundle
manifest carrying `agents`, with an error naming the limitation.** A
loud refusal beats a quiet substitution; non-adopting bundles are
untouched, so every existing dispatch path stays green. Lifting this
needs a jointly agreed Looper-lineage manifest version and is named as
follow-up work, not left to be discovered.

### Q2 — How per-attempt provenance reaches the journal

**Ruling.** Optional, absent-by-default **structured payload fields** on
`effect/started` and the terminal effect events, keeping
`event_schema: 1`. Published as
`contracts/effect-provenance.v1.schema.json`, a new file beside the
frozen ones; no frozen byte is edited.

`effect/started` gains, only for agent-resolved seats:

```json
"provenance": [
  {"member": null, "agent": "implementer", "model": "fable",
   "provider": "claude", "chain_index": 0}
]
```

A **list keyed by member/step tag**, because provenance is per driver
**invocation**, not per attempt: `SeatBody::Panel` runs N members inside
one attempt and `recipes/sdd`'s `design` seat runs three claude members
and steps *and* an `exec` step inside one attempt. Today
`driver_label` is `format!("panel[{n}]:{aggregate:?}")` — a single
string that cannot represent that, and hanging one `model` beside it
would print a false statement about what ran in every readout. For a
single seat the list has one element with `member: null`.

`effect/failed` gains `start_failure: true` and the member tag when the
structural predicate of AC-14 holds; that is the fact the next chain
index folds over.

Rejected, with reasons:

- **Packing provenance into the existing `effect/started.driver`
  string.** It forces five consumers plus the engine itself to parse a
  packed grammar to make a *control* decision, and `brokkr-bridge`
  truncates strings at 256 chars, so a panel's provenance would be
  silently clipped mid-field. The engine pattern-matching its own
  display label to choose the next model is the control-plane repair
  decision 0001 forbids.
- **Riding `effect/checkpointed.checkpoint`.** Checkpoints come from the
  driver, and a fail-to-start attempt emits none — the attempts whose
  provenance matters most would carry none at all.
- **Bumping `EVENT_SCHEMA` to 2.** `manifest_for` embeds
  `"event_schema": EVENT_SCHEMA`, so the bump moves **every** manifest
  digest — including the byte-identity witnesses this slice must keep —
  and violates `{"const": 1}` in both frozen manifest schemas. The same
  argument kills a new event *type*: the `type` enum is closed under
  `additionalProperties: false`.

`contracts/README.md`'s prose — *"a new field is a v2 event"* — is
**amended in the same commit, with its reason**, to an enforceable rule:

> Additive payload fields that are optional, absent by default, and
> published as a numbered extension schema are permitted at
> `event_schema: 1`. A field that changes the meaning of an existing
> field, or that a v1 consumer must read to fold to correct state, is a
> v2 event.

That amendment is honest here only because `fold` never reads the new
fields: the next chain index is derived by the engine scanning the
effect's own events, the way it already scans them for the last error,
and `RunState` gains nothing. AC-13 proves it.

`brokkr-bridge`'s `normalize_payload` allowlist is **deliberately not
extended**: provenance does not cross to Looper in this slice, asserted
by a test that names this ruling. Absence is not a false claim; a
half-updated boundary would be.

### Q3 — Where the agent library lives

**Ruling.** Repo root, flat, one file per agent:

```
agents/<name>.json          # the definition
agents/charters/<file>.md   # the prompt text, git mv'd out of the recipes
adapters/<provider>.json    # one per provider
```

Repo root, because `manifest_for` walks the *bundle* dir: a library
inside a bundle would be digested by whichever bundle sat above it, and
a shared library walked into `manifest.files` would couple every recipe
to every agent — editing an agent nobody references would move
`recipes/fast`'s digest. The `agents` manifest key is the pin instead,
and it covers exactly the transitively referenced agents.

One file per agent rather than one index file, because
`brokkr agents list` mirrors `brokkr recipes list`, whose contract is
*warn on a broken entry, never abort the listing* — a single index file
makes one syntax error fatal to the whole library. `brokkr agents show
<name>` then reads exactly one file.

Paths are resolved against the library root, canonicalised, and must be
**contained** within it; `parse_role`'s bare `dir.join(rel)` is
tolerable inside an operator's own bundle and is not tolerable for a
shared library joined on behalf of every recipe. Agent, model, provider
and MCP server names match `^[a-z][a-z0-9-]*$` and are unique
case-insensitively (the precedent is sequence step names). The
`secrets.env` refusal in `manifest_for` is extended to the library and
adapter trees.

### Q4 — How the diverging charters and argv are honestly modelled

**Ruling.** Distinct texts and distinct tool sets are **distinct,
honestly named agents**. There is no charter composition, no tool merge,
no seat-level override of anything the agent defines.

An agent reference is **total**. If a seat could amend the agent it
names, `agent: implementer` would stop being a complete statement about
what ran and `brokkr agents show implementer` would be a lie for that
seat — inlining with extra steps, and drift with a name on it. So
`role`, `driver`, `limits` and `inputs` are all **forbidden** alongside
`agent:`.

Three keys remain legal beside `agent:` because they are bindings the
*seat* provides rather than statements about what the agent is, and
`brokkr agents show` never claims to show them: `results` (phase/policy
coupling, required per seat), `secrets` (0012 environment binding), and
`confine`. Agents carry no secrets and no confine in this slice.

The measured consequence, stated plainly: the library ships **16
definitions over 14 charter files**, several near-identical, and this
slice therefore does **not** deliver "one implementer everywhere". It
delivers one definition per distinct thing that actually runs, which is
the only version of that promise that is true. The `-speckit` suffix is
ugly on purpose — it names *why* the variant exists (it carries
`Bash(specify:*)` for the spec-kit CLI) and puts the drift on the
surface where `brokkr agents list` shows it every time. Collapsing
near-duplicates is decision 0017's job; smuggling a merge rule in here
to get a prettier library is how 0016 and 0017 both end up half-built.

`speckit-check` does **not** become an agent: it is
`driver exec -- bash recipes/sdd/drivers/speckit_check.sh` — a shell
script with no model and no charter-as-prompt semantics. It stays
inline, and it is the case that proves the library is an option and not
a mandate.

### Q5 — The shape of `brokkr agents show`

**Ruling.** Pretty-printed canonical JSON: the definition as written,
plus a `resolution` block computed by the same pure function the
compiler calls — for each chain entry, the provider that maps it or
`unmapped`, and the capability check result. Machine-readable without a
`--json` flag, cannot drift from the data, and shorter than a
formatter. An unknown name errors, naming the known set.
`brokkr agents list` prints one tab-separated line —
`name ⇥ chain ⇥ description` — warning without aborting on a broken
entry.

## Further rulings this design makes

### One model name, one provider

An abstract model name mapped by **two** adapter files is a compile
error naming both. Resolution is then unambiguous by construction, with
no provider-preference tiebreak to reason about, and agents stay
provider-free. An operator wanting a lanetally-wrapped `fable` names it
distinctly — which is honest, because a wrapped invocation with session
capture is not the same invocation.

### Capability checks apply to EVERY entry in the chain

Capabilities split into **grants** and **restrictions**:

- An MCP server the provider cannot serve is a **grant** gap: the agent
  gets *less* power than declared. Visible degradation — this is what
  `optional` is for.
- A tool-permission narrowing is a **restriction**. A provider that
  cannot express it gives the agent **more** power, not less. `exec` has
  no notion of tool permissions at all.

So: `optional` is **structurally unrepresentable** on a restriction —
`tools.allow` is a plain ordered array of names, there is no key to set
— and unknown keys in agent and adapter files are rejected outright.
A restriction the provider cannot express is always a hard compile
failure naming agent, provider and capability.

Matching is **per named item, never per class**: "provider supports MCP"
does not satisfy "agent needs the `github` MCP server". Otherwise the
agent runs, finds no tools, and reports a *content* failure for a
*configuration* cause — the forge diagnosing itself wrong, which
decision 0001 exists to prevent.

And the checks run over **every candidate in the chain, not just the
chosen one**. A chain whose second link cannot express the agent's
restrictions is a chain that would silently widen the agent's blast
radius the moment it fell back; that is a compile error at design time,
not a surprise at 2am. This is the pinch of salt made mechanical, and
it is why the operator's literal example — `fable → qwen-3.8-max →
gpt-5.6-sol` — will fail compilation for a tool-restricted agent until
the `dsh` and `codex` adapters can express that agent's restrictions.
**That failure is the feature.** An agent may opt out by omitting
`tools.allow` entirely, which declares *no* restriction and is recorded
as `tool_restriction: none`; `"allow": []` is rejected as ambiguous
between "no restriction" and "restrict to nothing".

### Availability is a parameter; compile supplies none

`resolve` takes availability as an argument — provider →
`available | unavailable | unknown`, absent entries `unknown` — so
"unit-tested without spawning anything" is guaranteed by the *type*, not
by discipline. `Bundle::compile` passes `Availability::unspecified()`:
compile-time resolution depends on exactly two digested inputs, the
library and the adapters. A `Bundle::compile` that probed PATH would
give the same bundle different digests on two machines, make an
in-flight run unresumable after an `apt install`, and degrade the
reproducibility law to "same bundle, same laptop, same afternoon".

`brokkr doctor` is the real consumer of the non-`unknown` arms: it probes
this machine, calls the **same** `resolve`, and reports per agent which
model would be chosen here. Named limit: because compile supplies no
facts, this slice does not fail compilation for a chain that is mapped
but universally uninstalled. `brokkr doctor` catches that before a run
and bounded fallback catches it during one.

### Model pinning changes what adopting recipes run, on purpose

Adopting seats today pass no `--model` and get whatever the provider CLI
defaults to — an unpinned, invisible, undated choice. Resolved seats
pin an explicit concrete model id. Adopting recipes' argv therefore
changes; that is strictly more honest and it is the point of the
feature. Non-adopting recipes' argv does not change at all.

## Acceptance Criteria

Each criterion is a behaviour a reviewer can run. AC-1..AC-12 correspond
to the intake framing's twelve tests; AC-13..AC-22 are this design's
own.

- **AC-1 — Resolution is pure and deterministic.** `resolve` is called
  twice in two processes with the same inputs and returns byte-identical
  output including key order; nothing is spawned, no PATH, filesystem or
  clock access occurs. The first mapped candidate wins; a later entry is
  chosen only when an earlier one is unmapped or `unavailable`. A model
  mapped by no adapter is a compile error naming the model and the
  adapter files consulted. A model mapped by two adapters is a compile
  error naming both.
- **AC-2 — A required capability gap FAILS compilation**, and the
  message names the agent, the provider and the missing capability.
  Asserted on message content, not error kind. Holds for a
  *non-chosen* chain entry as well as the chosen one.
- **AC-3 — An optional capability gap WARNS and lands in the
  manifest.** The `notices` entry is asserted in
  `manifest["agents"][site]`, not merely on stderr: a warning that only
  reaches stderr is "nothing" by the ruling's own words. It surfaces in
  `brokkr inspect` as a run-level notice too (AC-17).
- **AC-4 — Byte-identity for non-adopters.** A golden test pins
  `manifest_digest()` for `recipes/fast` and `bundles/verify`; the
  `agents` key is absent from both manifests; every recipe in the tree
  still compiles.
- **AC-5 — Reference equals inline.** An agent-resolved seat produces
  the same `role_path`, argv (element for element, including
  `--allowedTools` ordering), limits and declared inputs as the
  equivalent inline seat, compared as resolved `SeatBody` values.
- **AC-6 — Fail-to-start falls back, bounded.** The first candidate's
  binary is absent → the next candidate in the chain is attempted, the
  switch is journaled (`start_failure: true`, then a new
  `chain_index`), and the whole sequence stays inside the seat's
  `max_attempts`. Exhausting the bound parks with the last error.
- **AC-7 — Mid-session failure does NOT fall back.** A driver that emits
  `Accepted`, checkpoints, then fails retries on the **same** candidate;
  `effect/indeterminate` still never auto-retries.
- **AC-8 — Provenance in every readout.** `brokkr inspect --json`,
  `brokkr tui`, the `brokkr ui` console and `brokkr compare` all show
  agent/model/provider from the single `brokkr-view` derivation; no
  surface formats it itself.
- **AC-9 — Adapters are data.** A test adds a NEW provider and a NEW
  model by writing an adapter file into a fixture directory and
  compiling a bundle against it, with **no Rust edit in the test's
  diff** — the executable form of "no code change, no release".
- **AC-10 — CLI.** `brokkr agents list` lists every agent and warns
  without aborting on a broken file; `brokkr agents show <name>` prints
  the definition plus its per-entry resolution; an unknown name errors
  naming the known set; `brokkr doctor` reports providers, their
  binaries, their probe results and their declared models **read from
  the adapter files**, not from a hardcoded list.
- **AC-11 — Secrets unchanged (0012).** A `secrets.env` inside `agents/`
  or `adapters/` fails compilation, mirroring `manifest_for`'s existing
  refusal. A `{{secret:` reference in an adapter driver template is
  scanned by the existing `scan_secret_refs` lint against the
  referencing seat's declared secrets and fails when undeclared. No
  agent or adapter file may carry a secret value; a test asserts a
  value-bearing adapter file is rejected.
- **AC-12 — Gates.** The 97-case differential corpus and the
  machine-proof suite pass unmodified; canonical formatting;
  warning-free clippy across all targets and features; the exact nonzero
  100% line/branch/function coverage gate
  (`scripts/coverage-exact.sh`) — which refuses `coverage(off)`
  outright, so new code lands with its tests.
- **AC-13 — Fold is unchanged.** `fold` never reads a provenance field;
  folding an adopting run's journal yields the same `RunState` a v1
  consumer would derive. A golden test asserts a non-adopting recipe's
  journal is byte-identical — no new payload field appears at all.
- **AC-14 — The fail-to-start predicate is structural.** It is exactly
  `Failed` **and** no `Accepted` ever received **and** no checkpoint
  emitted — no stderr sniffing, no "model not found" regex. Two cases in
  one test file: a driver failing before `Accepted` falls back; a driver
  that emits `Accepted` and then fails does not.
- **AC-15 — The chain index survives a restart.** Fold the journal,
  restart the engine between attempts, and the same candidate is
  selected: the index is derived from journaled facts, never from
  memory or a re-probe.
- **AC-16 — Per-invocation provenance.** A panel of two members on two
  providers produces two provenance records in one attempt, and
  `recipes/sdd`'s `design` sequence reports `claude` and `exec`
  separately. Each invocation site walks its **own** chain index.
- **AC-17 — `chosen_index > 0` is surfaced, not merely stored.** A
  fallback selection and an optional-capability gap both appear as a
  run-level notice in `brokkr inspect`, `brokkr tui` and the console — a
  notice that only lands in JSON nobody reads is the ruling's "never
  nothing" defeated.
- **AC-18 — `brokkr compare` reports resolution divergence.** A
  first-class `resolution_divergence` field, per seat and member,
  computed from the single derivation over what **actually ran**
  (comparing pinned plans would hide precisely the fallback this exists
  to expose), reported unconditionally — including when `same_recipe`
  is `true`.
- **AC-19 — The Looper lineage refuses rather than truncates.**
  `build_run_manifest_v2` returns an error naming the limitation when
  the bundle manifest carries `agents`; non-adopting bundles dispatch
  exactly as today. `brokkr-bridge`'s allowlist is asserted to still drop
  provenance, with the ruling named in the test.
- **AC-20 — Name grammar and path containment.** A charter path
  escaping the library root (`../`), a name outside
  `^[a-z][a-z0-9-]*$`, and a case-duplicate agent name are each
  rejected with a message naming the offending file and key.
- **AC-21 — The agent reference is total.** A seat combining `agent:`
  with `role`, `driver`, `limits` or `inputs` fails compilation naming
  the conflicting key; `results`, `secrets` and `confine` remain legal
  beside it.
- **AC-22 — The resolved seat faces every existing lint.** Resolution
  happens before the 0007 provenance lint, the 0012 secret-reference
  lint, results-covered-by-a-rule and protected-phase reachability;
  each is proven to fire on an agent-resolved seat exactly as on an
  inline one.

## Non-goals

Decision 0017 (`extends`, composition, merge rules). Widening fallback
to mid-session failures. Any claim that a model "is real", any runtime
capability probing used as a gate, any auto-selection framed as
quality-equivalent. New provider drivers — the five adapter kinds are
the set. New dependencies. Touching
`fixtures/evaluator/corpus.ndjson`, `policy/phase-machine.json`,
`reference/`, or the bytes of any frozen v1 contract. Making agents
mandatory. Changing the secrets machinery. Rewriting `recipes/fast` or
`bundles/verify` — they are the byte-identity witnesses.

## Named limits this slice ships with

Written here because an undocumented gap is the quiet substitution
wearing the opposite mask.

1. **"No `Accepted` ever arrives" does not fall back — it parks.**
   Decision 0016 names three fail-to-start triggers. The structural
   predicate of AC-14 covers two: a driver that fails to spawn, and a
   driver that reports a determinate failure before accepting (the
   shape a provider's model rejection takes for every driver we own).
   The third — a driver that exits without accepting and without a
   determinate result — is `AttemptOutcome::Indeterminate` today
   (`process.rs:158` → `fold.rs:275`), and decision 0003 rules that it
   parks because the forge cannot distinguish "did nothing" from
   "already opened a billed session". Reclassifying it to satisfy 0016
   would make an indeterminate-shaped condition auto-retry behind a
   feature flag — a bound that applies "unless a new feature is in
   play" has stopped being a bound. **The honest mitigation is at the
   driver, not the engine**: drivers we own should report a provider's
   pre-session model rejection as a determinate `Result{status:failed}`,
   which converts case 3 into case 2. Where a driver cannot, the run
   parks and the operator decides.
2. **Looper-dispatched runs cannot adopt agents** (Q1). Lifting it needs
   a jointly agreed v2-lineage manifest version.
3. **Provenance does not cross the Looper bridge** (Q2), asserted rather
   than assumed.
4. **Compilation does not fail for a mapped-but-uninstalled chain**,
   because compile supplies no availability facts. `brokkr doctor` before,
   bounded fallback during.
5. **The library holds near-duplicate entries** (Q4). This slice reduces
   *definition* duplication, not file count.
