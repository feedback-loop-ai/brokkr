# ember — the frugal daily driver

`extends: "fast"`. The same constitution, with an intake seat in front
of it and the shared roster behind it: docs, chores, small fixes — the
work that makes up most days.

| Phase | Office | `max_attempts` | `timeout_seconds` | Class |
|---|---|---|---|---|
| `intake` | `intake` | 2 | 1800 | work |
| `implement` | `implementer` | 2 | 5400 | work |
| `verify` | boxed `verify-seat.sh` | 2 | 3600 | gate |
| `review` | `reviewer` | 2 | 3600 | gate |
| `ship` | boxed `ship-seat.sh` | 2 | 1800 | gate |

## When to use it

- A documentation change, a README correction, a comment that lies.
- A chore: a dependency bump, a rename the compiler can check, a lint
  fix, a test that needs one more assertion.
- A small, well-understood bug fix whose blast radius is one file.

**When not to.** Anything under `crates/brokkr-runtime`,
`crates/brokkr-core` or `crates/brokkr-store` whose behaviour changes;
anything touching `contracts/`, `policy/schemas/`, an event shape or a
pinned digest. That is `crucible`'s work. Ember's intake charter is
instructed to say so in the framing when a request turns out to be
larger than it looked — the operator reads that and decides whether to
re-run the feature under a heavier recipe.

## Cost expectations

**These are targets, not measurements. This recipe has not been run.**
Nothing in this repository's journal records an ember run, so there is
no cost figure here that any evidence supports, and one will not be
invented. What can be stated as fact is the *structure* the cost
follows:

- The intake phase is an added seat, so ember runs *one more* session
  than `fast` does. It buys a framing the implementer would otherwise
  re-derive, which is a bet, not a certainty.

When ember has runs behind it, the honest figures belong in
LaneTally — cost accounting is its domain, not this engine's
(decision 0021 ruling 6) — and this section should cite them.

## `max_attempts` stays at 2

The roster analysis asked for "limits tuned small," and named only
timeouts. `max_attempts` is therefore left at `fast`'s 2 rather than
silently picked: attempts are decision 0006's crash/timeout/malformed-
output retries, and dropping to 1 would change what *parks* the run, not
what it costs. That is a behavioural change, and it belongs to
`night-shift`, which asks for it explicitly.

## The one-line swap property

Every seat here is one agent name. Changing the office seated by a phase is
one token in `bundle.json`, while changing an office's hire belongs in its
single library entry. The digest moves when either changes — that is the point. See
[`recipes/wager-harness`](../wager-harness/README.md) for the swap
turned into a procedure.

## How the roster is seated

Every phase names the agent shown in the table above. The library owns each
office's charter, fallback chain, effort, tools, and default limits; this recipe owns
only its phase shape and result vocabularies.

## Composition mechanics

`fast` has no `intake` phase and starts at `implement`. Ember's own
`policy.json` adds the phase and one rule as plain additions — no
collision, no marker needed — but `initial` and `description` are table
scalars the base already sets, so replacing them requires saying so:

```json
"override": {
  "seats": ["implement", "verify", "review", "ship"],
  "table":  ["description", "initial"]
}
```

Without the `table` marker, compilation refuses the bundle naming the
collision (decision 0017). All four of `fast`'s seats are listed under
`override.seats` because all four are redefined — each one seats a roster
agent.

## Running it

```
brokkr run --recipe ember --repo . --feature "<the chore>"
```

**This recipe has not been run end to end.** It compiles under the
shipped adapters, its gate seats' trust tiers are checked at compile
time, and its manifest digest is pinned in
`crates/brokkr-runtime/tests/witness_digests.rs`; that is the claim, and
the only one.

Its verifier and shipper are boxed exec scripts inherited in contract
from `fast`. Cargo runs offline from the bound registry cache; an
uncached dependency fails closed and the verifier quotes Cargo's
decisive offline/cache line in its `fail` notes.
