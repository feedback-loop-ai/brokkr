# Change: Read the DSH session file the installed core writes (#222 follow-on)

## Why

The transcript reader that landed at `f4f4ba6` searches a DSH session
directory for exactly one filename, `session.jsonl`, and the living
specification names that one filename as the rule.

The DSH core has since versioned the file. Core 0.1.5-rc.1, which this
repository moved to during the issue #226 work, writes `session.v3.jsonl`.
Earlier cores wrote `session.jsonl`.

Measured on this host on 2026-09-13, under `~/.dsh/sessions/brokkr`:

| Filename | Sessions |
|---|---|
| `session.jsonl` | 146 |
| `session.v3.jsonl` | 17 |

So the reader reports `not-found` for every session written since the
upgrade, including tonight's Astra and DeepSeek seats, while a 4.5 MB
transcript sits beside the reference the journal recorded. The refusal is
honest — it names the reference, reports zero skipped and zero unrecognized,
and invents nothing — but it loses exactly the newest evidence, and the
count only grows as every new run uses the upgraded core.

This is not a retention question. The file is retained; the reader does not
know its name.

## What changes

`session.v3.jsonl` joins `session.jsonl` as an admitted name, in that order.
A session directory still yields at most one candidate: the first admitted
name carrying a valid header. A directory holding both therefore admits the
newer and never becomes ambiguous with itself, and a name outside the closed
set is still never read.

Nothing else about discovery moves. The retained-root restriction, the
header and delegation-depth rules, the safety checks and every failure
precedence stay exactly as specified.

## Why a closed, ordered set rather than a pattern

A glob over `session*.jsonl` would admit whatever a future core writes
without anyone reading it first, which is the opposite of how this reader
treats every other provider. The set is closed so that adding a name is a
decision with evidence behind it, and ordered so that the choice between two
present files is stated rather than incidental.
