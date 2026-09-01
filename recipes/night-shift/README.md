# night-shift — the unattended queue

`extends: "fast"`. The same four phases and the same constitution, tuned
for the case where **nobody is awake**: one attempt per seat, a long
implement deadline, and every gate on the best judge available. Anything
unusual parks or stops for morning triage instead of retrying into the
dark.

| Phase | Model | `max_attempts` | `timeout_seconds` | Class |
|---|---|---|---|---|
| `implement` | sonnet *(see TODO below)* | **1** | 7200 | work |
| `verify` | opus | **1** | 3600 | gate |
| `review` | opus | **1** | 3600 | gate |
| `ship` | opus | **1** | 1800 | gate |

## `max_attempts: 1` everywhere — what it does and does not do

`max_attempts` is decision 0006's **per-attempt** bound: crashes,
timeouts, malformed driver output. Setting it to 1 means an effect-level
failure exhausts immediately and the run parks with its evidence,
instead of spending a second session on the same wall while nobody is
awake to read the first one.

It is **orthogonal to the phase table**. A *valid* `broken` result from
`implement` is not an effect failure — it is a typed result, and
`fast`'s inherited `IMPL-BROKEN-RETRY` rule still sends the run back
into `implement` once, exactly as it does under every other recipe here.
Night-shift needs no policy table of its own for this and ships none;
the intent "anything unusual parks for morning" comes entirely from the
attempt bound, never from a modified constitution.

The two behaviours, side by side:

| What happened | Under `fast` | Under `night-shift` |
|---|---|---|
| Driver crashed / timed out / emitted garbage | second attempt | parks with evidence |
| Seat reported `broken` (a valid typed result) | back to `implement` once | back to `implement` once |
| Review reported `security-hold` | hard stop | hard stop |

## TODO — the deepseek lane is not wired

The roster analysis called for `implement` to be driven by the **dsh**
adapter's deepseek lane. It is not, and this seat runs on
`claude`/`sonnet` instead.

**The blocker is a missing model mapping, not a trust-tier refusal.**
`adapters/dsh.json` declares `"models": {}` — an empty map, verified at
the time this recipe was written — so there is no deepseek model name to
put behind a `--model` flag; the same file declares `"model_flag":
"unsupported"`. Nothing about decision 0021 stands in the way: `dsh` is
`trust_tier: "untrusted"`, and ruling 7 admits an untrusted driver to a
**work** seat freely. `implement` is declared `class: "work"` and
carries no `secrets` key, so neither of 0021's two compile-time
prohibitions (untrusted judge, ungranted secret binding) applies to it.
The seat is lawful today; it simply has nothing to name.

The follow-up is therefore adapter work, not recipe work: give
`adapters/dsh.json` a `models` map and a usable `model_flag`, then
change this seat's `driver.command` to
`["{brokkr}", "driver", "dsh", "--", …]` and drop this section.
`adapters/*.json` is shared cross-recipe data that other bundles and the
model-policy tests key off, so it was deliberately not edited from a
recipe-authoring change: fabricating a `models` entry would have
silently changed what every other bundle sees as "the deepseek lane
exists."

Reference: [decision 0021](../../docs/decisions/0021-model-policy.md),
rulings 2 and 7.

## Cost expectations

**These are targets, not measurements. This recipe has not been run.**
Nothing in this repository's journal records a night-shift run. What is
structural fact:

- Three of four seats sit on opus. This recipe is **not** the cheap
  one — [`ember`](../ember/README.md) is. Night-shift buys *unattended
  safety*: the gates are the seats nobody will double-check before
  breakfast, so they get the best judge.
- Its intended saving was the implement seat on a cheap untrusted
  lane, and that saving is **not currently realised** — see the TODO
  above. On sonnet, the implement seat costs what any sonnet implement
  seat costs.
- `max_attempts: 1` caps the worst case at one session per seat entry
  rather than two. That is a real bound on spend, and the only one this
  recipe adds.

When night-shift has runs behind it, the figures belong in LaneTally
(decision 0021 ruling 6), and this section should cite them.

## The scheduling window — the operator's cron, not this recipe

The operator schedules night-shift runs inside DeepSeek's cheap
**01:00–04:00** pricing window.

**That sentence is operational documentation and nothing else.** This
recipe does not read a clock, does not gate on time of day, and contains
no mechanism by which it could: there is no condition in the
vocabulary for it (decision 0004 — the condition vocabulary is closed
and validated at load), and a recipe is declarative data, not
control flow (decision 0002's linear outer machine). Scheduling lives
entirely outside Brokkr, in whatever cron or timer the operator runs.

If the window moves, nothing in this directory changes. Grep
`bundle.json` and every file under `roles/` for a time and you will find
none: no seat, no charter and no driver command names an hour. The only
hours in this recipe are the two ends of the window, written in prose,
in this README, where they cannot execute.

## When to use it

- A queue of independently-framed features drained overnight.
- Work that is understood well enough to survive nobody being available
  to answer a question — because nobody will be.

**When not to.** Anything ambiguous. The implement charter is instructed
to report `blocked` rather than guess, so an underspecified feature
spends a full deadline and delivers a park. Frame it properly first, or
run it attended.

## The one-line swap property

The implement seat is one JSON object; the TODO above is literally a
one-line-plus-flag change once `adapters/dsh.json` names a model. That
is the property [`recipes/wager-harness`](../wager-harness/README.md)
turns into a procedure — and the discipline it names (same sandbox, same
tools, same repo base) is exactly what a future dsh-vs-sonnet comparison
on this seat will need.

## How the models are pinned

Inline `--model` pairs on each `driver.command`, the same mechanism and
the same trade-offs [`ember`](../ember/README.md#how-the-models-are-pinned)
documents.

## Running it

```
brokkr run --recipe night-shift --repo . --feature "<the queued task>"
```

**This recipe has not been run end to end.** It compiles under the
shipped adapters, its gate seats' trust tiers are checked at compile
time, and its manifest digest is pinned in
`crates/brokkr-runtime/tests/witness_digests.rs`; that is the claim, and
the only one.
