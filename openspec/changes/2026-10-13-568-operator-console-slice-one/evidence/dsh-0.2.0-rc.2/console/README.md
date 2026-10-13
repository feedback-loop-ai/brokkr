# The console inside DSH 0.2.0-rc.2, against a real journal

- **Date:** 2026-10-13, two loads: 09:56 local (before the presentation fixes) and 10:15 local (after them).
- **DSH:** `0.2.0-rc.2`, isolated as in `../README.md`: a scratch `DSH_HOME`, scratch XDG dirs and `TMPDIR`, loopback only. The plugin was installed from `extensions/dsh/brokkr-console/` with `dsh plugin --profile web add`.
- **Brokkr:** `brokkr` built from this branch (`runs --json` reports `view_version: 19`), reading the operator's journal of six real runs.
- **Read-only.** Neither resume setting was set, so the console could send no retry or resume, and the browser script blocked any command request in the page. None was attempted.
  - `journal-before.txt` and `journal-after.txt` give the event and run counts, every run's head and the digests of `forge.db` and its WAL. All are identical.
  - Only `forge.db-shm` changed time; SQLite touches it on any read.

## Fence

`fence.txt`. Every plugin route and the WebSocket upgrade refuse a request with no login cookie (401) and one from another origin (403). They admit a same-origin request with the cookie (200, or 101 for the upgrade). The command route was never sent a request with a cookie.

## What renders

The three screenshots are from the second load:
- `shot-1-realms.png`: Realms. The brokkr card shows its counts. The three realms whose journals don't exist on that machine show as quarantined, in Brokkr's own words.
- `shot-2-run.png`: one run.
  - The step graph shows models, durations and the rule that stopped the run. The timeline is newest first.
  - Runs whose titles Brokkr cuts alike are told apart by start time.
  - There is no decision card, because no run in this journal is parked. The card is covered by the plugin's tests, not by this check.
- `shot-3-analytics.png`: Analytics. "Where runs wait for you" says nothing waited.

The host log had no errors or warnings. The browser had no console messages, page errors or failed requests.

## Tests

`plugin-tests.txt` is the output of `npm test` in `extensions/dsh/brokkr-console/` that this change was proven with, as decision 0085 ruling 5 asks until the Node CI job lands.
