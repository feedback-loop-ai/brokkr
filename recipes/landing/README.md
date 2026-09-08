# landing — the machine finishes what you wrote by hand

`fast`'s crew, entered at a gate that reads instead of writes:
`classify` → `verify` (code only) → `review` → `ship` → `done`, with
`implement` standing behind them as the remediation seat. The branch
already exists — you wrote it, a session wrote it with you, a ruling put
it there — so nothing is commissioned and nothing is implemented first.
`classify` answers in seconds whether the branch is prose or code; code
is verified by the checks CI will run, prose goes straight to the judge;
`review` judges the diff; a failure or a finding above low sends the
smith in to answer exactly what was named, bounded as decision 0022
bounds every return; a clean judgment ships, and the shipped anchor
carries the patch map that lets the contribution gate vouch for the pull
request at tier `vouched` (decision 0038). No `by-hand` label.

```
brokkr run --recipe landing --repo . --feature "landing: <what the branch is, and why>"
```

The feature text is the commission of record: say what the branch is
and why, in a sentence, the way you would in the pull request. A seat
that returns reads the finding in `returned_from`; the feature text tells
it what not to reinvent.

## What each seat does

| Seat | Class | Results | Runs |
|---|---|---|---|
| `classify` | `gate` | `docs`, `code` | `scripts/classify-seat.sh`, boxed, no model: every path the branch changes against the default branch is read against the repository's own docs class, `.github/delivery-classes.json` — the file the contribution gate cuts its tiers by (decision 0038 ruling 3). All prose → `docs`; anything else, or anything it cannot establish → `code`. |
| `verify` | `gate` | `pass`, `fail` | `fast`'s boxed exec verifier: format, clippy, the workspace suite, both bundle compiles, the exact-coverage script. `fail` returns to `implement` with the decisive output. |
| `review` | `gate` | `clean`, `residual`, `security-hold` | `fast`'s adversarial read of the diff against the base: correctness, fit, security. A residual above low returns to `implement`; at or below low ships as named debt; `security-hold` stops. |
| `implement` | `work` | `complete`, `broken`, `blocked`, `oversized` | Entered only on a return. `fast`'s implementer charter already says it: "answer the finding in `returned_from`; that finding is the work this visit owns." |
| `ship` | `gate` | `ready`, `shipped` | `fast`'s boxed exec shipper: a clean tree, the head the engine gated on, the ledger, the anchor with the per-file patch map. |

The delivery rules are `fast`'s, rule for rule. This recipe adds the two
`classify` arms in front of them and changes one word of the table,
`initial`; that word is what makes it a landing rather than a delivery:
the run begins by reading what is there instead of by writing something.

## Prose is judged, not built

A docs-only branch enters at `review`. That is decision 0039's reasoning
— a review that fixed only prose does not buy the whole verify again —
applied at the front of a run instead of on a return, and it is what
makes a decision, a guide or a research sweep cost one judge's read
rather than a build. The platform suite on the pull request remains the
backstop for the tests that read prose: the decision index, the rename
guard, the guide tables, the diagrams. A branch that mixes one code path
into its prose is code, whole; the classify seat carries no pattern of
its own and cannot be argued with.

## Where it differs from `preflight`

| | `preflight` | `landing` |
|---|---|---|
| Ends at | `review` — findings only, nothing changes, nothing merges | `ship` — a vouched head, or a stop with the reason |
| On a finding | reports it | returns it to the smith, twice, then the ladder |
| The gate reads it as | a `Brokkr-Preflight` beside a delivery run's docs delta | a `Brokkr-Run`: the run that judged and shipped the head |
| Cost | two gates | a read, a build for code, a judge, and the smith only when there is something to fix |

A preflight is the right tool when you want to know; a landing is the
right tool when you want to land. The gate refuses a preflight named as
a `Brokkr-Run` — it "never ruled shipped" — which is why every branch
that only had a preflight also needed the label.

## What a landing costs

`classify` is seconds. `verify` is an exec seat — minutes, no model, and
only for code. `review` is one judge's read of the diff. `implement` is
spent only on a finding. On the journal's own figures, the shipped runs
of the first week of September cost a median of nine dollars end to end,
implement included; a landing with nothing to fix is the review seat
alone.

## What a landing cannot give you

It judges what is on the branch against the base; it does not judge
whether the branch should exist. That question is the pull request's
prose and the operator's read, as it always was. And a landing whose
review stops the run — a security hold, a residual above medium at the
bound — has told you the branch is not ready, with the reason in the
journal. That is the answer, not an obstacle to route around.
