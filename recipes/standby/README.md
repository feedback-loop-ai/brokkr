# standby — the crew that keeps working when an account does not

`fast`'s shape, its contracts and its exec gates, with both model seats
on the other vendor. Reach for it when the account behind the default
crew is out of limit, or when a delivery must not touch that account at
all.

## The whole diff against `fast`

Two seats, drivers only. Every result vocabulary, declared input, limit,
role charter and the policy table itself are `fast`'s, inherited:

| Seat | `fast` | `standby` |
|---|---|---|
| implement | claude `fable` @ high | codex `sol` (`gpt-6.1-sol`) @ high, sandbox `workspace-write` |
| review | claude `fable` @ high | codex `sol` (`gpt-6.1-sol`) @ high, sandbox `read-only` |
| verify, ship | exec scripts, boxed under `namespace` only | unchanged — no model, no vendor |

`sol` is a judge in `adapters/codex.json`, which is what lets it hold
the review gate (decision 0041 ruling 3). Both seats were hired on
`astra` at `xhigh` until the operator's roster ruling of 2026-09-30
(decision 0045's addendum) replaced Astra 6.0 with Sol 6.1 and capped
Sol's effort at `high`. The smith sits at that cap on the operator's
instruction of 2026-09-06: a hedge is reached for when the other crew
cannot run at all, so it is carrying work the default crew would
otherwise have done, and a returned heat costs more than the effort.

Each class is the one the engine admits for an inline Codex seat of that
class, under every boundary (`lower_inline_sandbox` in
`crates/brokkr-runtime/src/bundle.rs`), and `danger-full-access` is
admitted nowhere. The smith writes under `workspace-write`, through the
codex adapter's `hands.harness.work` fragment. The reviewer runs
`read-only`, through the `hands.harness.gate` fragment, which adds
`--output-last-message`: Codex captures the judge's final message into
the result path the engine owns, so a judge that cannot write a file
still delivers its result. Before that door, on 2026-09-06, a read-only
judge reached a full verdict and could not write the result file, so
the engine saw no result at all, and this recipe ran its smith under
`danger-full-access` and its reviewer under `workspace-write`. A gate's
discipline here is the read-only class and `fast`'s besides: the charter
forbids edits and the engine parks a gate that moves the head (decision
0041 ruling 4).

## Why it is inline, and when it should stop being

The library is the roster (decision 0041 ruling 2) and this recipe is an
exception to it, ruled on 2026-09-06 for the reason ruling 7 already
gives the wagers: a hedge must **force** its crew. Its whole purpose is
the vendor it does not use, and a library chain would undo that silently
at its first fallback.

It is inline for a second reason, and that one is a limitation rather
than a law. Codex expresses no per-tool allow-list, so no office holding
one can resolve on it (decision 0045 ruling 4), and the shipped smiths
hold `cargo` and `git`. When an implementer declares hands, as the
hands-bearing review offices already do, the allow-list is not consulted at all
(decision 0043 ruling 2) and the smith becomes hireable on any provider
that puts its hands in the box. On that day this recipe should retire
into a library crew, and its README should say so rather than this.

## What it does not give you

- **Not a wager.** A wager changes one name and holds everything else
  equal (`recipes/wager-harness`). This changes two, and the comparison
  it invites — codex smithing and codex judging in one run — is a
  different question from which crew delivers better.
- **Not a boxed smith.** The implement seat runs under codex's own
  sandbox class, not inside Brokkr's box, exactly as `fast`'s claude
  smith runs under claude's permission mode. The exec gates are `fast`'s:
  boxed under `namespace`, unboxed under `harness`.
- **No dsh arm.** dsh holds no judge (`adapters/dsh.json` declares an
  empty `judges` list), so it cannot hold this recipe's review gate. A
  dsh hedge would need a different shape and its own ruling.
