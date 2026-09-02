# The agent library

This guide owns the complete tour of reusable agents, resolution and
fallback.

## The agent library

A seat used to inline everything it was: charter text, driver argv,
limits, declared inputs. Decision 0016 lets it name an agent instead.

```
$ brokkr agents list
chief-architect	fable → opus → sonnet	Synthesises the panel's positions into the committed spec, plan and tasks, and rules on the open questions.
implementer	opus → sonnet	Builds the framed task to the repository's conventions and commits the work with its tests.
intake	sonnet → opus	Frames a raw request into a recorded, actionable task before any code is written.
review-security	opus → sonnet	Review panel member: the adversarial security read of the change.
reviewer	opus → sonnet	The single-seat reviewer: correctness and security in one pass, for recipes without a review panel.
shipper	sonnet → opus	Closes a delivery out: ledger, gates, and the report the operator reads before merging.
verifier	sonnet → opus	Runs the suites and gates and reports pass or fail on evidence, never on intent.
…
```

```
brokkr agents show <name>    # the definition, plus its per-entry resolution
brokkr doctor                # which providers and models are actually here
```

An agent is one file in `agents/`: a description, a charter, an ORDERED
preference chain of abstract model names, abstract tool and MCP
configuration, its decision-0006 limits and its decision-0007 declared
inputs. A seat, panel member or sequence step says `"agent": "<name>"`.
Inline seats stay first-class — `recipes/sdd`'s `speckit-check` step is
a shell script with no model, and it stays inline.

Resolution happens at compile time, is pinned into the run manifest, and
is a pure function of *(library, adapters, availability)* — availability
that `Bundle::compile` deliberately supplies none of, so one bundle
cannot resolve two ways on two machines.

**The honesty rules are the point, and they are enforced rather than
documented.** A tool restriction the provider cannot express fails
compilation naming the agent, the provider and the capability — the
agent would run with MORE power than it declares, so `optional` is
structurally unrepresentable there. An MCP server the provider cannot
serve fails the same way unless the agent marked it optional, and then
it is a notice that lands in the run manifest and in every readout —
never nothing. Both checks run over **every** entry in the chain, so a
chain that would widen an agent's blast radius the moment it fell back
fails at design time rather than at 2am. The chain is a fallback chain,
not a portability claim: Brokkr never says the second choice equals
the first, and `brokkr compare` reports a model difference as a
first-class divergence.

Fallback is narrow on purpose. An attempt that FAILS TO START — the
driver binary is absent, or the provider rejects the model before
accepting — retries on the next model in the chain, inside decision
0006's existing attempt bounds, journaled as a fact. A mid-session
failure is not fallback material: a seat that ran for forty turns and
then hit a wall produced work a different model does not inherit, so it
follows 0006 unchanged. The predicate is structural — `Failed`, never
`Accepted`, no checkpoint — so once a session opens, fallback is
unreachable by construction rather than by convention.

**Three limits ship with it, stated as limits.**

1. **A Looper-dispatched run cannot adopt agents.** The v2 run-manifest
   lineage reconstructs a bundle manifest from six named keys and would
   silently drop the `agents` pin, making the run unresumable with a
   diff that blames no file. `build_run_manifest_v2` refuses instead.
   Lifting it needs a jointly agreed v2-lineage manifest version.
2. **Provenance does not cross the Looper bridge.** The bridge's payload
   allowlist drops it, asserted by a test rather than assumed.
3. **"No `Accepted` ever arrives" parks, it does not fall back.** That
   shape is `indeterminate` today, and decision 0003 rules that it parks
   because Brokkr cannot tell "did nothing" from "already opened a
   billed session". The honest fix is at the driver — report a
   provider's pre-session model rejection as a determinate failure — not
   at the engine, because a bound that applies "unless a new feature is
   in play" has stopped being a bound.
