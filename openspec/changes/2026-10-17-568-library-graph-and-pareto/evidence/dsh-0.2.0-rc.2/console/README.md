# Live check: the Library (slice one) in DSH 0.2.0-rc.2, against the operator's journals, read-only

- **Date:** 2026-10-17, 13:52–14:07 +03:00 (the browser log gives UTC `Z` times).
- **Plugin:** this branch's `extensions/dsh/brokkr-console/`, linked into the operator's own `web` profile in place of the earlier worktree's. It is the committed tree plus the token change committed with this evidence.
- **Brokkr:** this branch's `target/release/brokkr` (`brokkr 0.12.0`), set as `BROKKR_CONSOLE_BROKKR`. It lists 5 realms: `brokkr`, `swarmail`, and three recipe libraries.
  - `lanetally`'s policy table is malformed (`no rule rules (review, residual)`), and `looper`'s adapter carries an unknown key (`binding_grant`).
  - The Library shows both refusals in Brokkr's own words rather than a graph. They are those repositories' defects, not the console's.
- **Not isolated:** unlike slice two's check, this ran in the operator's own DSH web, `127.0.0.1:3080`, with the operator watching. No model was asked anything. The Library has no assistant surface, and no command was sent.
- **Browser:** headless Firefox driven by Puppeteer, at 1600×1000 (`wide-*`) and at 1366×768 (`laptop-*`). Each shot is the console element alone, so DSH's sidebar, which holds the operator's session titles, is not in the repository.
  - `log-browser.txt` holds the Library's own requests with the launch token redacted, and the browser's console errors. There were none.

## What was observed

| Shot | What it shows |
|---|---|
| `*-1-graph-brokkr-landing` | `landing`'s machine, starting at its initial state `classify` though the composed table lists it last. Fired edges are solid with their counts; the 14 of 24 rules that never fired are dashed; edges into `stop` are in the stop tone, with `done` at the row's end. |
| `wide-2-graph-landing-review-picked` | `review` picked. The panel shows entered 26, median 4m with p90 25m, and cost $74.51. Tokens read `Σ 111.92M tok ↑ 111.89M ↓ 28k`, with cache hit 100.0% as its own fact, as the run view's seat row reads them. Below that come its rules with their counts and its seats. |
| `*-3-graph-swarmail-self` | `self`: 29 runs, 12 of 23 rules never fired, and no repair edge. Every review and verify stop goes to `stop`. |
| `*-4-pareto-swarmail` | The Graph Pareto frontier in four equal quadrants at 50% and $3.00. `self` sits in *Cheap but leaky* at 20% and $2.85; `frontier` is listed as "cost not reported". |
| `*-5-pareto-brokkr` | `landing` (74%, $3.88) in *Reliable but costly*, on the frontier; `triage` (14%, $19.88) in *Costly and leaky*. The cost axis bends at the target, so the quadrants keep one size. |

## Read-only: the journal is byte-identical

`journal-before.txt` (13:52:31) and `journal-after.txt` (after the last shot) match on:
- 7070 events, 34 runs, every queue table at 0, and 4 operator events;
- `forge.db` sha256 `ae0abd6d…5c27`, 12619776 bytes, mtime 13:23:30;
- `forge.db-wal` empty.

Swarmail's journal moved during the check, because its run `run12d` was live. The Library only reads it, and the check does not rest on it.
