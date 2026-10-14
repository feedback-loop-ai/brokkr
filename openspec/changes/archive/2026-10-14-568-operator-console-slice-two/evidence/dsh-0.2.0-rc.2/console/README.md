# Live check: the real brokkr-console (slice two) in DSH 0.2.0-rc.2, against the operator's journal, read-only

- **Date:** 2026-10-14, 01:59–02:06 +03:00 (the logs give UTC `Z` times, 2026-10-13T22:59–23:06Z).
- **Plugin:** `~/source/feedback-loop-ai/brokkr-568-s2/extensions/dsh/brokkr-console/`, installed by absolute path with `dsh plugin --profile web add`. The worktree's uncommitted state at the time of the check was used. `plugin-files-sha256.txt` pins the files, and they were unchanged at the end of the check. Nothing in `~/source/feedback-loop-ai/` was modified, staged, committed or stashed.
- **Brokkr:** `brokkr-568-s2/target/release/brokkr` (`brokkr 0.12.0`, `runs --json` → `view_version: 20`).
  - `BROKKR_CONSOLE_BROKKR` was set to that binary and `BROKKR_CONSOLE_WORKSPACE=~/source/feedback-loop-ai/brokkr`.
  - Both `BROKKR_CONSOLE_RESUME_*` variables were unset.
  - Brokkr lists 5 realms and 17 runs: 11 in `brokkr` and 6 in `swarmail`. The other three realms are quarantined because their journals don't exist on this machine.
- **Isolation:** scratch `DSH_HOME`, `XDG_*` and `TMPDIR` folders under this folder (`tools/env.sh`). Everything bound to loopback only: DSH on `127.0.0.1:38228`, the stub on `127.0.0.1:38229`.
  - `~/.dsh` mtime is unchanged at `2026-10-13 10:25:41`.
  - Everything was killed afterwards, and the Firefox profile, the curl cookie jar and `$DSH_HOME/.credentials.yaml` were deleted (`log-07-stop.txt`).
  - The launch token is redacted in `serve-raw.txt`.
- **Model:** a loopback stub only. It received the dummy key `stub-not-a-credential`, and no real provider was contacted. The exact script is `stub-model/server.mjs`, and the route and default-model patch is `stub-model/bundle/cordis.patch.yml`. For each user turn it does four things:
  1. calls `brokkr_runs {}`;
  2. calls `brokkr_inspect {"run": RUN}`, where RUN is read from the runs answer (`landing-pr-660-at-4f6a1931-the-d-08a428d4` when listed, which it was);
  3. calls `brokkr_suggest {"run": RUN, "command": "stop", "reason": "live check"}`;
  4. answers in text.

  It logs the tool list, the asked prompt, the session workspace from DSH's runtime context, and every tool result (truncated) in `stub-model.log`.

## Read-only: the journal is byte-identical

`journal-before.txt` (01:59:47), `journal-mid.txt` (after the first browser run) and `journal-after.txt` (02:06:33) match:
- 2468 events, 11 runs and every queue table at 0.
- The same head for each of the 11 runs, and 0 operator or conclude events.
- `forge.db` sha256 `e298cc89…a499964` (4452352 bytes, mtime 2026-10-14 00:06:49). `forge.db-wal` is 0 bytes with sha256 `e3b0c442…852b855` (the empty-file digest).

Only `forge.db-shm` changed mtime; SQLite touches it on every read, including this script's.

The workspace repository was also unchanged (`workspace-before.txt` and `workspace-after.txt`, read with `GIT_OPTIONAL_LOCKS=0`): the same `M realms.json`, the same `.git/index` sha256 and the same HEAD `67a1886e`.

In the page, every `fetch` to `/brokkr-console/api/command` was wrapped to refuse before it left. The count of blocked attempts was `[]`, so none was made, and no command or decision-card button was pressed. No command request reached the network layer.

## What was observed

1. **The five tools reach the model, run against the real journal, and the refusal shows in the transcript.**
   - Every model request in a session offered `brokkr_inspect,brokkr_queue,brokkr_runs,brokkr_suggest,brokkr_transcript` among 31 tools (`stub-model.log`, every `#n brokkr tools offered`).
   - **`brokkr_runs`** returned 47,607 characters of real data: `view_version: 20`, the realms `brokkr, swarmail, lanetally, looper, xbox-inference`, and 17 runs with titles, for example `landing-pr-660-at-4f6a1931-the-d-08a428d4 | title='landing: PR #660 at 4f6a1931, the DSH operator console…'` (`log-05-session-tool-results.txt`).
   - **`brokkr_inspect`** on that run produced 123,507 bytes. It contains `"recipe":"landing"`, `"worktree":null` and `"clarify":null`, and all four commands are `refused`.
     - DSH's spill policy cut what the model saw to 49,932 characters: the head, the tail (where `recipe` sits), and the note `(Omitted 73536 bytes. Full formatted result stored at: $TMPDIR/dsh-spill-…/…-brokkr_inspect.txt. Use read with offset/limit…)`.
     - The spill file is the whole JSON. This is DSH behaviour, not the plugin's 256 KiB bound.
   - **`brokkr_suggest`** answered `{"suggested":false,"text":"refused operator stop (after_terminal): the run is not in a state this command can apply to; the refusal is journaled. Read it with: brokkr inspect --run landing-pr-660-at-4f6a1931-the-d-08a428d4"}`.
   - Unfolding "Completed in 1s" → "Called tools and searched code" shows the plugin's view, "No suggestion card: refused operator stop (after_terminal): …", with no card (`shots/shot-03-ask-brokkr-called-tools.png`, `shots-run/shot-04-ask-run-called-tools.png`).
   - The same three calls ran in all three sessions with the same results.
2. **Ask Brokkr** (Realms view). Typing "Which runs need me today?" and pressing **Ask** opened a session in Workspace `brokkr` (left sidebar) and sent it. The stub received it with `session workspace (from DSH's runtime context): ~/source/feedback-loop-ai/brokkr`. The user message is the scope line, the tools line and the question:

   ```
   About Brokkr across every realm the console reads.

   Read Brokkr with the brokkr_runs, brokkr_inspect, brokkr_queue and brokkr_transcript tools. Cite the ruling (by its sequence number), session or record each fact comes from, and say plainly what is not recorded instead of guessing. You may suggest one command with brokkr_suggest; only the operator can press it.

   Question: Which runs need me today?
   ```

   Evidence: `shots/shot-01-realms.png`, `shot-02-ask-brokkr-answer.png`, `shot-03`, `log-03-browser.txt`, and stub `#1`–`#5`.
3. **Ask about this run.** Selecting the run "landing: PR #660 at 4f6a1931, the DSH operator console…" (shown shipped, with its step graph and timeline, `shots-run/shot-02-run-view.png`) and asking "Why did this run finish, and what did it change?" opened and sent a session. The prompt opens with `About the Brokkr run “landing: PR #660 at 4f6a1931, the DSH operator console…” (realm brokkr, run id landing-pr-660-at-4f6a1931-the-d-08a428d4).`, followed by the tools line and the question (`log-04-browser-run.txt`, stub `#11`–`#15`). The run's view has `worktree: null`, so the session's cwd is the workspace, `~/source/feedback-loop-ai/brokkr`, which the stub confirms. The first run (`log-03`, `shots/shot-04`–`06`) also asked about "Deliver OpenSpec change…" (`deliver-openspec-change-2026-10--38009879`) with the same result. In both runs the driver's check for the `run <id> · head seq` line timed out. The selected row's title and the run id in the prompt identify the run.
4. **No clarify card.** No run in this journal is parked at clarify: `NEEDS YOU · 0`, every `inspect` has `clarify: null`, and the run view has no "Open the spec", "questions read at seq" or decision card (`runFacts` in `log-03` and `log-04`). "Open the spec" therefore could not be shown live.
5. **The fence still holds** (`fence.txt`):

   | Route | No cookie | Cookie, `Origin: http://evil.example` | Cookie, same origin |
   |---|---|---|---|
   | `console.css` | 401 | 403 | 200 |
   | `api/analytics` | 401 | 403 | 502 (see below) |
   | `api/command` (body `not-json`) | 401 | 403 | never sent |
   | WebSocket upgrade | 401 | 403 | 101 |

6. **Errors.** The browser had no console messages, page errors, failed requests or 4xx/5xx responses during the Ask flows (`problems seen in the page: none`). The DSH host output has only its URL line.
   - **One defect found:** the **Analytics tab fails against this journal**. `GET /brokkr-console/api/analytics` returns 502 `{"error":"models[4].model is not text"}`, and the tab shows that text (`shots/shot-07-analytics.png`, `log-06-browser-analytics.txt`).
   - Cause: `brokkr analytics --json` (view 20) emits `{"model": null, "attempts": 3, "percent": 4, "text": "not reported 4%"}`, but `lib/host/readouts.js:347` parses `models[].model` with `text()` rather than `optText()` (`log-02-analytics-502.txt`).
   - The 502 is not logged on the host side (a `ReadoutRefused` is answered without `console.error`).
   - **Wording to check:** Brokkr's refusal text says "the refusal is journaled", but the suggestion only read `brokkr inspect`, and nothing was journaled, as the identical journal shows. The tool passes the `commands[].text` that `inspect` precomputes. Shown in a "No suggestion card" note, it can read as though a refusal was recorded.

## Files

| File | What |
|---|---|
| `tools/env.sh`, `tools/journal.sh`, `tools/workspace.sh` | Isolation environment, journal snapshot, workspace-repo snapshot. |
| `tools/drive.mjs`, `tools/drive-analytics.mjs`, `tools/session-tools.py` | Browser drivers (with the command guard), and the session-log summary. |
| `stub-model/server.mjs`, `stub-model/bundle/` | The exact stub script, and the test-only default-model bundle. |
| `log-01-install.txt` | Versions, view version, realms, both installs. |
| `fence.txt` | Fence re-check. |
| `log-02-analytics-502.txt` | The analytics 502, and the readout row that causes it. |
| `log-03-browser.txt`, `shots/` | Realms, Ask Brokkr, Ask about this run ("Deliver OpenSpec change…"), and the Analytics screenshot `shot-07`. |
| `log-04-browser-run.txt`, `shots-run/` | Ask about this run on `landing-pr-660-at-4f6a1931-the-d-08a428d4`. |
| `log-05-session-tool-results.txt` | What each session recorded for each tool. |
| `log-06-browser-analytics.txt` | Analytics tab text. |
| `stub-model.log` | Everything the model endpoint received, with tool results truncated. |
| `journal-before.txt`, `journal-mid.txt`, `journal-after.txt`, `workspace-before.txt`, `workspace-after.txt` | The read-only evidence. |
| `log-07-stop.txt` | Teardown. |
| `home/`, `tmp/` | Scratch DSH home (three sessions) and TMPDIR (DSH spill files). |

## Fixed after this check

- **The analytics defect** (`log-02-analytics-502.txt`): `lib/host/readouts.js` read `models[].model` as required text, so view 20's `null` model was refused and the tab answered 502. It now reads optional text. The analytics fixture carries a `null` model row, and the client test fails with the old parse.
- **The refusal's wording:** "the refusal is journaled" is Brokkr's text for a pressed command. A suggestion only reads the run, so `brokkr_suggest` now answers "If pressed now, Brokkr would refuse it: …", followed by Brokkr's words.
