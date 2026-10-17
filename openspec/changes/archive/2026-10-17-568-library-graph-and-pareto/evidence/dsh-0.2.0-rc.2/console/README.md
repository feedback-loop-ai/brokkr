# Live check: the Library (slice one) in DSH 0.2.0-rc.2, against the operator's journals, read-only

- **Date:** 2026-10-17. First check 13:52–14:07 +03:00. Re-taken at 15:10–15:12 after the graph was reworked into links with outcome chips (design D3) and the recipe dropdown added (D7). The shots and journal records here are the re-take; the browser log gives UTC `Z` times.
- **Plugin:** this branch's `extensions/dsh/brokkr-console/`, linked into the operator's own `web` profile in place of the earlier worktree's. It is this branch's tree as committed with this evidence.
- **Brokkr:** this branch's `target/release/brokkr` (`brokkr 0.12.0`), set as `BROKKR_CONSOLE_BROKKR`. It lists 5 realms: `brokkr`, `swarmail`, and three recipe libraries.
  - `lanetally`'s policy table is malformed (`no rule rules (review, residual)`), and `looper`'s adapter carries an unknown key (`binding_grant`).
  - The Library shows both refusals in Brokkr's own words rather than a graph. They are those repositories' defects, not the console's.
- **Not isolated:** unlike slice two's check, this used the operator's own DSH profile, `web`. The first check ran in their running DSH web on `127.0.0.1:3080`. The re-take ran in a second DSH web of the same profile on `127.0.0.1:3091`, so the operator's session was left alone. No model was asked anything, the question boxes are hidden (D10), and no command was sent.
- **Browser:** headless Firefox driven by Puppeteer, at 1600×1000 (`wide-*`) and at 1366×768 (`laptop-*`). Each shot is the console element alone, so DSH's sidebar, which holds the operator's session titles, is not in the repository.
  - `log-browser.txt` holds the Library's own requests with the launch token redacted, and the browser's console errors. There were none.

## What was observed

| Shot | What it shows |
|---|---|
| `*-1-graph-brokkr-landing` | `landing`'s machine, starting at its initial state `classify` though the composed table lists it last. One thin line per pair of states, with the outcomes that fired as chips (`code 28`, `pass 26`, `residual 11 · clean 10`, `blocked 4`). Lines whose rules never fired are faint and dashed, without chips. The recipe dropdown reads `landing · 28 runs`, with 18 recipes in the realm. |
| `wide-2-graph-landing-review-picked` | `review` picked. The panel shows entered 26, median 4m with p90 25m, and cost $74.51. Tokens read `Σ 111.92M tok ↑ 111.89M ↓ 28k`, with cache hit 100.0% as its own fact, as the run view's seat row reads them. Below that come its rules with their counts and its seats. |
| `*-3-graph-swarmail-self` | `self`: 30 runs, 12 of 23 rules never fired, and no repair edge. `review → stop` is one line with `residual 12 · security-hold 2`, and `verify → stop` reads `fail 4`. |
| `*-4-pareto-swarmail` | The Graph Pareto frontier in four equal quadrants at 50% and $3.00. `self` sits in *Cheap but leaky* at 20% and $2.88; `frontier` is listed as "cost not reported". |
| `*-5-pareto-brokkr` | `landing` (71%, $3.74) in *Reliable but costly*, on the frontier; `triage` (14%, $19.88) in *Costly and leaky*. The cost axis bends at the target, so the quadrants keep one size. |

## Read-only: the journal is byte-identical

`journal-before.txt` and `journal-after.txt`, taken around the re-take, match on:
- 7086 events, 35 runs, every queue table at 0, and 6 operator events (two operator stops recorded earlier that afternoon, before the re-take);
- `forge.db` sha256 `16404dd8…0123e`;
- `forge.db-wal` empty.

Swarmail's journal moved during the check, because its run `run12d` was live. The Library only reads it, and the check does not rest on it.
