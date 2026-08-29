# 0016 — The agent library: seats reference agents, adapters map them to providers

Status: accepted (operator ruling in chat, 2026-08-29)

## Context

Today a seat inlines everything it is: charter text, driver command
line, limits, declared inputs. The same chief architect written into
three recipes is three copies that drift. The operator's ask: define an
agent **once** — its description, its charter, the models it prefers
with fallbacks, its tool and MCP configuration — and have seats
reference it, with adapters mapping that one definition onto whichever
provider actually serves it.

The operator also ruled the honesty constraint explicitly: *"take the
adapter claim with a pinch of salt."* Providers are not
interchangeable. An abstraction that pretends otherwise produces runs
that look identical and are not, which is the one thing this project
exists to refuse.

## Decision

### The agent

An agent is a named, digest-pinned definition in an `agents/` library,
exactly as recipes are a library of bundles:

- `description` — what this agent is for, in a sentence. Human-facing
  and listed by `forge agents list`.
- `charter` — a path to its prompt, the text a seat used to inline.
- `models` — an ORDERED preference chain of abstract model names
  (e.g. `["fable", "qwen-3.8-max", "gpt-5.6-sol"]`). Names are opaque
  strings resolved by provider mappings; the forge validates nothing
  about them beyond that a mapping exists.
- `tools` — abstract tool and MCP configuration: the MCP servers the
  agent needs and the tool permissions it expects.
- `limits` — the decision-0006 bounds.
- `inputs` — the decision-0007 declared seat inputs.

A seat says `{"agent": "chief-architect"}` instead of inlining. Panel
members and sequence steps may each name an agent. Inline seats remain
valid — the library is an option, not a mandate.

### The provider adapter

A provider adapter is data, not code: a mapping file per provider
declaring, for that provider, how abstract things become concrete —
which driver invocation serves it, which abstract model names it can
serve and as what concrete ids, how MCP servers and tool permissions
are expressed on its command line, and **which capabilities it simply
does not have**.

### Resolution, and the pinch of salt

Resolution happens at **compile time** and is pinned into the manifest:
agent digest, chosen model, chosen provider. Same bundle, same
resolution — the reproducibility law is unchanged.

The honesty rules, which are the point of this decision:

1. **Capability gaps are errors, never silent degradation.** If an
   agent requires MCP servers and the resolved provider declares no MCP
   support, compilation FAILS naming the agent, the provider and the
   missing capability. An agent may mark a capability optional, and
   then a gap is recorded as a compile-time WARNING that also lands in
   the run manifest — never nothing.
2. **The chain is a fallback chain, not a portability claim.** The
   forge asserts only that some provider can serve some model in the
   chain. It never asserts the second choice is equivalent to the
   first.
3. **Provenance is journaled per attempt.** Every effect records which
   agent, model and provider actually served it. A run whose second
   attempt fell back to a different model must be legible as exactly
   that, in every readout — including `forge compare`, where a model
   difference is a first-class divergence, not a footnote.

### Fallback, bounded

Fallback is deliberately narrow: an attempt that **fails to start** —
the driver binary is absent, the provider rejects the model, or no
`Accepted` message ever arrives — retries on the NEXT model in the
chain, within decision 0006's existing attempt bounds, and the switch
is journaled as a fact.

Mid-session failures are NOT fallback material: a seat that ran for
forty turns and then hit a quota wall has produced work and context
that a different model does not inherit, so it follows 0006 unchanged
(retry the same way, or park). That boundary is deliberate; widening it
needs its own ruling and evidence.

## Constraints

- Adapter mappings are **data files**, operator-editable, not Rust
  match arms: adding a provider or a model must not require a release.
- The frozen v1 contracts, the corpus and the policy table are
  untouched. Bundle format gains agent references; the run manifest
  gains resolved-agent provenance.
- Resolution is a **pure function** over (agent library, adapter
  mappings, availability facts) — unit-tested without spawning
  anything.
- `forge agents list|show` mirrors `forge recipes`, and `forge doctor`
  reports which providers and models are actually available here.
- Secrets (0012) unchanged: adapter files carry names, never values.

## Consequences

- One chief architect, referenced everywhere, swappable in one edit.
- The forge can run the same recipe on different providers and
  `forge compare` can say what that changed — which is the experiment
  the recipe library was built for.
- The cost is a resolution layer that can fail loudly at compile time.
  That is the intended trade: a loud failure beats a quiet substitution.
