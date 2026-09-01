# ember — the frugal daily driver

`extends: "fast"`. The same constitution, with an intake seat in front
of it and a deliberately cheap roster behind it: docs, chores, small
fixes — the work that makes up most days and does not deserve the
heaviest crew in the shop.

| Phase | Model | `max_attempts` | `timeout_seconds` | Class |
|---|---|---|---|---|
| `intake` | haiku | 2 | 1800 | work |
| `implement` | sonnet | 2 | 3600 | work |
| `verify` | sonnet | 2 | 1800 | gate |
| `review` | **opus** | 2 | 1800 | gate |
| `ship` | haiku | 2 | 1800 | gate |

One seat is not economized on: `review`. Ember buys cheaper *labour*,
never a cheaper *judge* — the reforging ladder, the security-hold hard
stop and the protected review gate are `fast`'s, inherited rule for
rule, and the seat that produces the verdict they act on runs on the
best model in the roster.

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

- Four of ember's five seats run on haiku or sonnet where `fast` runs
  whatever `claude` defaults to. Whether that is cheaper in practice
  depends on the default, which is a machine-local fact this bundle
  does not pin.
- Only one seat — `review` — sits on opus.
- Timeouts are cut roughly in half against `fast` (`implement` 3600 vs
  5400; every other seat 1800 vs 3600/1800). A timeout is a bound, not a
  spend: cutting it does not make a run cheaper, it makes a runaway seat
  stop sooner. The saving is in the models, not the clocks.
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

Every seat here is one JSON object. Moving `implement` from sonnet to
opus is one token in `bundle.json`; so is putting `verify` on haiku, or
pointing `implement` at a different provider's driver entirely. The
digest moves when you do it — that is the point. See
[`recipes/wager-harness`](../wager-harness/README.md) for the swap
turned into a procedure.

## How the models are pinned

Two mechanisms were available (see
[recipe-authoring.md](../../docs/guides/recipe-authoring.md)): an inline
`driver.command` carrying an explicit `--model` pair, or new
single-entry agent-library entries. **This recipe uses the inline
form**, and so do `crucible`, `night-shift` and `wager-harness`, for
consistency:

```json
"--allowedTools", "Bash(cargo:*),…",
"--model", "claude-haiku-4-5-20251001"
```

The reasoning: `fast` — the base all four extend — is already inline
`role`+`driver`, so this keeps the diff against it readable, and every
existing agent-library entry declares an ordered *fallback chain*
(`["opus", "sonnet"]`), not the single pin these recipes need. The cost
of the choice, stated plainly: the concrete model ids are duplicated
here from `adapters/claude.json`'s `models` map, so a rename there does
not reach these bundles. The compiler does not check the pair — a
`--model` argument is argv, forwarded to the driver, and nothing
validates it against the adapter's map. A model id typo in this file is
a run-time failure, not a compile-time refusal.

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
`override.seats` because all four are redefined — each one pins a model
and re-cuts its limits.

## Running it

```
brokkr run --recipe ember --repo . --feature "<the chore>"
```

**This recipe has not been run end to end.** It compiles under the
shipped adapters, its gate seats' trust tiers are checked at compile
time, and its manifest digest is pinned in
`crates/brokkr-runtime/tests/witness_digests.rs`; that is the claim, and
the only one.
