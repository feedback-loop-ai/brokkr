# The agent library

A seat used to inline everything it was: charter text, driver argv,
limits, declared inputs. Decision 0016 lets it name an agent instead.

Every model prompt is assembled from three texts with separate owners. The
library charter under `agents/charters/` is the portable office: what the seat
does and must not do. The realm may name repository-specific Markdown through
`house` in `forge.realms/v3`; the engine inserts it once under `## House rules`
after the charter. Finally, the engine renders the result-file contract from
the current site. A non-final sequence step receives its own `results`; the
final step and an ordinary single seat receive the enclosing seat's results.
House and dialect declarations and their content digests ride inside the
realms manifest pin, so changing a house changes the run identity without
changing an agent or recipe digest.

```
$ brokkr agents list
chief-architect	fable → opus	Synthesises the panel's positions into the committed spec, plan and tasks, and rules on the open questions.
implementer-engine	fable → opus	Engine-class implementer: builds core, store, contract, and policy work selected by triage.
implementer-speckit	opus → sonnet	Implementer for spec-driven delivery: builds to the committed spec, ticking tasks.md as it goes.
implementer	opus → sonnet	Builds the framed task to the repository's conventions and commits the work with its tests.
intake	sonnet → opus	Frames a raw request into a recorded, actionable task before any code is written.
intake-speckit	sonnet → opus	Intake for spec-driven delivery: frames the request before the design phase opens the feature's spec directory.
muninn	opus	Reads the fleet dossier and proposes operator actions; issues none.
position-robustness	sol → opus	Design panel member: argues the failure modes the simple design would leave open, with evidence.
position-simplicity	opus → sol	Design panel member: argues the simplest design that meets the ruling, and names what it gives up.
review-adversarial	fable → opus → sol	Review panel member: tries to break the delivered change with concrete adversarial cases.
review-chief	fable → opus → sol	Review chief: checks the panel's findings and rules the protected phase without lowering its verdict.
review-correctness	sol → opus	Review panel member: does the change do what it claims, and does the evidence support it?
review-security	fable → opus → sol	Review panel member: the adversarial security read of the change.
review-spec-compliance	opus → sol	Review panel member: does the delivered change satisfy the committed spec's acceptance criteria?
reviewer	fable → opus → sol	The single-seat reviewer: correctness and security in one pass, for recipes without a review panel.
triage	fable → opus	Rules the commission's delivery class from a closed vocabulary, fresh and blind.
```

```
brokkr agents show <name>    # the definition, plus its per-entry resolution
brokkr doctor                # which providers and models are actually here
```

An agent is one file in `agents/`: a description, a charter, an ORDERED
preference chain of abstract model names, abstract tool and MCP
configuration, its decision-0006 limits and its decision-0007 declared
inputs. A seat, panel member or sequence step says `"agent": "<name>"`.
Inline seats stay first-class. Dialect validators such as `recipes/triage`'s
`validate` step are also model-free execs, but their checked argv comes from
the realm's pinned dialect rather than from an agent definition.

Verifier and shipper are deliberately absent from the agent library.
They are boxed, inline `exec` scripts with no model: verification runs a
recipe's fixed checks, and shipping renders journal evidence through
`brokkr ledger` before confirming the recorded head and clean tree.

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

## Hands

An agent may declare `"hands"` instead of relying on its `tools.allow`
list (decision 0043). The harness keeps its credential and its network;
what the model asks to run goes through one MCP tool, `workspace`, served
by `brokkr hands serve`, and every call executes inside an empty-root
bubblewrap namespace holding the worktree read-write and the host
toolchain read-only. `binds` add host paths — the Rust toolchain cache as an
overlay (the box may write to it, the host never sees the writes), its
credentials masked, rustup read-only:

```json
"hands": {
  "kind": "workspace",
  "network": false,
  "binds": [
    {"path": "~/.cargo", "mode": "overlay", "mask": ["credentials.toml", "credentials"]},
    {"path": "~/.rustup", "mode": "ro"}
  ]
}
```

With hands, the adapter's per-tool map is not consulted; what the adapter
must express is how its harness's own tools are replaced by the one boxed
tool (`hands` in the adapter file, or `"unsupported"` with the reason).
The review agents declare hands, which is what lets `sol` on codex be
their third link.
