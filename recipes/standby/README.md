# standby — the crew that keeps working when an account does not

`fast`'s shape, its contracts and its boxed gates, with both model seats
on the other vendor. Reach for it when the account behind the default
crew is out of limit, or when a delivery must not touch that account at
all.

## The whole diff against `fast`

Two seats, drivers only. Every result vocabulary, declared input, limit,
role charter and the policy table itself are `fast`'s, inherited:

| Seat | `fast` | `standby` |
|---|---|---|
| implement | claude `fable` @ high | codex `astra` @ xhigh, sandbox `danger-full-access` |
| review | claude `fable` @ high | codex `astra` @ xhigh, sandbox `read-only` |
| verify, ship | boxed exec scripts | unchanged — no model, no vendor |

`astra` is a judge in `adapters/codex.json`, which is what lets it hold
the review gate (decision 0041 ruling 3). The smith is hired at `xhigh`
rather than `fast`'s `high` on the operator's instruction of 2026-09-06:
a hedge is reached for when the other crew cannot run at all, so it is
carrying work the default crew would otherwise have done, and a returned
heat costs more than the effort. The reviewer reads under
codex's own `read-only` class; the smith writes under
`danger-full-access`, as the wager harness's codex arm does, because a
smith that cannot write is not a smith.

## Why it is inline, and when it should stop being

The library is the roster (decision 0041 ruling 2) and this recipe is an
exception to it, ruled on 2026-09-06 for the reason ruling 7 already
gives the wagers: a hedge must **force** its crew. Its whole purpose is
the vendor it does not use, and a library chain would undo that silently
at its first fallback.

It is inline for a second reason, and that one is a limitation rather
than a law. Codex expresses no per-tool allow-list, so no office holding
one can resolve on it (decision 0045 ruling 4), and the shipped smiths
hold `cargo` and `git`. When an implementer's hands are boxed the way
the review offices' already are, the allow-list is not consulted at all
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
  smith runs under claude's permission mode. The gates that *are* boxed
  stay boxed.
- **No dsh arm.** dsh holds no judge (`adapters/dsh.json` declares an
  empty `judges` list), so it cannot hold this recipe's review gate. A
  dsh hedge would need a different shape and its own ruling.
