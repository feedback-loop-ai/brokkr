# What it does

You hand Brokkr a feature and a **recipe** (a delivery strategy:
policy table, seats, charters, limits, drivers — reviewable text,
identified by content digest). The engine drives real agent sessions
through the recipe's phases — implement, verify, review, ship — ruling
on each typed result with a pinned first-match-wins policy. Unknowns
never advance: schema violations, unmatched results, exhausted retries,
and security findings park or stop the run with raw evidence attached.
The operator's judgment enters only as signed journal events.

```
brokkr run --recipe fast --repo . --feature "…"     # deliver
brokkr watch --run <id>                             # watch it live, in the terminal
brokkr tui                                          # explore the fleet with the keyboard
brokkr ui                                           # watch it live, in a browser
brokkr rerun --run <id> --recipe panel-review       # swap the strategy
brokkr compare <a> <b>                              # journal-backed A/B
```

- **Live telemetry**: Claude and Codex seats stream bounded per-turn/item
  checkpoints; DSH emits an explicit harness lifecycle. Checkpoints retain
  only bounded turn/tool/usage fields or a file-path-only target—never prose,
  commands, or reasoning; the Looper bridge hashes the target before export.
  The journal is evidence, not transcript. Parallel
  review panels stream both members
  side by side. The full session transcript stays one
  `claude --resume <session_id>` away.
- **The strategy loop** (decisions 0010/0017): a library of recipes
  (`brokkr recipes list|add|show`, installable from git), swap by name,
  re-run a past feature under another recipe, compare outcomes —
  decision trails with first divergence, per-seat costs, verdict
  deltas. A pure read over two journals; works on live runs.
  Recipes **compose**: `night-shift` extends `triage`, overriding only
  its attempt limits and dsh implement seat. A derived selector may
  override one named case with `override.cases`.
  Named things merge by name; redefining one the base has needs an
  explicit marker, so an accidental collision fails compilation instead
  of silently winning. Composition resolves at compile time into ONE
  flat bundle — no inheritance at run time — and the run manifest
  records the chain, so a run states what it was composed from.
- **Bounded autonomy** (decisions 0006/0007): per-seat attempt limits
  and deadlines; determinate failures retry, indeterminate outcomes
  always park; every evaluation input is engine-computed or
  seat-declared — everything else is dropped before it reaches the
  table or the record.
- **Self-hosting**: Brokkr forges its own changes (`bundles/self`)
  and verifies every delivered slice with its own adversarial agents
  (`bundles/verify`) — which have hard-stopped their author's work on
  real security findings, twice. The operator keeps push and merge
  authority.

- **Foreign delivery, proven**: Brokkr's first delivery to a repository
  that is not its own is merged —
  [looper#346](https://github.com/feedback-loop-ai/looper/pull/346), a
  CI-gate fix driven end to end through `recipes/node` in the target
  repo's own worktree, and accepted under that repository's own
  candidate-bound evidence law before landing. The
  [adoption guide](adopting-a-node-repo.md) is the path
  that run took.

[ARCHITECTURE.md](../../ARCHITECTURE.md) is the deep dive: crates, journal,
effect discipline, verification layers. This guide stays the tour.

## Determinism laws

1. **Decisions are pure.** Given the same journal and pinned bundle,
   the next action is always the same. Transition logic is a data table
   evaluated first-match-wins by `brokkr-core`; changing a ruling is a
   reviewed one-line diff.
2. **State is derived, never mutated.** `state = fold(events)`; resume
   is replay; counters, drift, and reviewed heads are journal-computed,
   never accepted from a caller.
3. **No LLM repair of the control plane** (decision 0001). Invalid
   results park in `awaiting_operator` with the raw evidence — never
   guessed at, coerced, or handed to a model to fix.
4. **Human gates are control states.** Parks exit only through operator
   events; approval is a journal entry, not a prose convention.

The paradigm behind these laws is argued in long form in
[the essays](../essays/): *[The Model Is a Detail](../essays/the-model-is-a-detail.md)*
(why the model belongs in leaf position, with this repository's own
history as the receipts) and
*[Decisions as Code](../essays/decisions-as-code.md)* (the
deliberate → rule → encode → enforce → evidence → amend lifecycle the
[decision record](../decisions/) lives by), and
*[The Lore](../essays/the-lore.md)* (why the
[Edda](../lore/edda.md) is an engineering decision, not decoration),
*[The Machine That Argued With Itself](../essays/the-machine-that-argued-with-itself.md)*
(five arguments from the journals, and what makes them argument
instead of theater), *[The Wager](../essays/the-wager.md)* (two
crews, one commission, and why fair model comparison is an
architecture property), and
*[One Job, Three Hires](../essays/one-job-three-hires.md)* (a phase
held by a model is a job with a description and a price tag; three
hires into one job, thirty to one on price, the same gates for all).
