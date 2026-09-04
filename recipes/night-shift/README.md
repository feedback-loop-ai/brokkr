# night-shift — the unattended queue

`extends: "fast"`. The same four phases and the same constitution, tuned
for the case where **nobody is awake**: one attempt per seat, a long
implement deadline, and every gate seated from the shared roster. Anything
unusual parks or stops for morning triage instead of retrying into the
dark.

| Phase | Seat | `max_attempts` | `timeout_seconds` | Class |
|---|---|---|---|---|
| `implement` | inline deepseek-v4-flash via `dsh` | **1** | 7200 | work |
| `verify` | `verifier` | **1** | 3600 | gate |
| `review` | `reviewer` | **1** | 3600 | gate |
| `ship` | `shipper` | **1** | 1800 | gate |

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

## The deepseek lane

`implement` is driven by the **dsh** adapter: `{brokkr} driver dsh --
--model deepseek/deepseek-v4-flash`. It is lawful under decision 0021 as it
stands — `dsh` is `trust_tier: "untrusted"`, and ruling 7 admits an
untrusted driver to a **work** seat freely; `implement` is `class:
"work"` and carries no `secrets` key, so neither compile-time
prohibition (untrusted judge, ungranted secret binding) applies.

What the seat gives up, and the comparison must say: `adapters/dsh.json`
declares `tool_permissions: "unsupported"`, because the headless dsh
launcher has no allowed-tools flag. The seat runs with whatever the
harness permits, not the seven `Bash` prefixes the former inline seat named.
That asymmetry is the same one [`recipes/wager-harness`](../wager-harness/README.md)
records for its challenger arm, for the same reason.

How the pin reaches the harness: dsh has no model flag of its own; the
model is a row of its composed profile tree, and the launcher's only
override is a `--patch` overlay. The `dsh` driver turns `--model <id>`
into that overlay for the one seat and passes the rest of its arguments
through — see `crates/brokkr-protocol/src/adapters.rs`. The abstract
name `flash` in `adapters/dsh.json` is deliberately not a claude tier,
so no agent chain written for one provider lands on the other.

## Cost expectations

**These are targets, not measurements. This recipe has not been run.**
Nothing in this repository's journal records a night-shift run. What is
structural fact:

- Its intended saving is the implement seat on a cheap untrusted
  lane, which it now sits on. Whether the saving is real is what the
  first night-shift runs will show; nothing here has measured it.
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

The implement seat is one JSON object; moving it from the shared roster to the
deepseek lane was a driver name and a model id, nothing else. That is
the property [`recipes/wager-harness`](../wager-harness/README.md)
turns into a procedure — and the discipline it names (same sandbox, same
tools, same repo base) is exactly what a dsh-vs-roster comparison on
this seat will need.

## How the roster is seated

The dsh implement lane remains inline by the lane exception. The three gates
name `verifier`, `reviewer`, and `shipper`; their charters, fallback chains,
effort, tools, and default limits come from the agent library. This strategy
narrows each gate's attempt bound to one.

## Running it

```
brokkr run --recipe night-shift --repo . --feature "<the queued task>"
```

**This recipe has not been run end to end.** It compiles under the
shipped adapters, its gate seats' trust tiers are checked at compile
time, and its manifest digest is pinned in
`crates/brokkr-runtime/tests/witness_digests.rs`; that is the claim, and
the only one.
