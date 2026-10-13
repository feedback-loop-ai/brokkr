# The DSH operator console

The console puts Brokkr's fleet inside a DSH web client: a **Brokkr** entry
in DSH's left panel list, whose icon shows how many runs are waiting on you,
and a panel with two views, **Realms** and **Analytics**. It is a DSH plugin
kept in this repository at `extensions/dsh/brokkr-console/` (decision 0084,
proposed). It targets DSH `0.2.0-rc.2`.

The console paints Brokkr's own readouts and presses Brokkr's own verbs. It
decides nothing: every standing, park reason, admitted command, refusal,
"not available yet" reason and figure on screen is text a `brokkr ... --json`
readout printed. It never opens a journal.

## Install it into a DSH profile

1. Have `brokkr` on the `PATH` DSH starts with, or name the binary in
   `BROKKR_CONSOLE_BROKKR`.
2. Start DSH in the workspace whose `realms.json` names your hearths, or name
   that directory in `BROKKR_CONSOLE_WORKSPACE`. Every readout runs there, as
   you would run it by hand.
3. Name the bundle runs are resumed under, in `BROKKR_CONSOLE_RESUME_BUNDLE`
   (a bundle directory) or `BROKKR_CONSOLE_RESUME_RECIPE` (a recipe name).
   Brokkr's readouts do not say which bundle a run was started under, and a
   retry is followed by `brokkr resume`, which needs one. Without either, the
   console sends no retry and no resume, and says why. `resume` refuses a
   bundle that does not compile to the run's pinned manifest, so a wrong one
   is refused, never run.
4. Add the plugin to a profile by its absolute path:

   ```sh
   dsh plugin --profile web add "$PWD/extensions/dsh/brokkr-console"
   ```

5. Open DSH's web client from the URL `dsh web` prints, and pick **Brokkr**.

The plugin is `"private": true` and is never published. It listens on DSH's
own loopback port, acts as the OS user DSH runs as, and every one of its
routes and its WebSocket first asks DSH's connection fence, so a request
without DSH's login cookie, or from another origin, is refused.

## What it shows

- **Realms.** One card per hearth `brokkr runs` lists: the runs that need
  you, each with its title, step and Brokkr's verdict; how many are running;
  how many shipped and stopped in the last day, from `brokkr analytics
  --range 24h`. A hearth whose journal does not load shows its refusal. Below
  them, the queue from `brokkr queue list`, each entry with admission's word
  and reasons; starting a queued run waits for Brokkr's dispatcher. Muninn's
  last review, from `brokkr muninn list`, when one is recorded.
- **One realm.** The run list (needs you, running, last 24 hours), filterable
  by title. For the selected run: its header, the steps it went through with
  the model and rule of each visit, the decision card, and the timeline,
  newest first.
- **The decision card.** Brokkr's park reason, word for word, and the rule
  that parked the run. A clarify park lists the clarifier's open questions
  and the run's worktree. A button for each command `brokkr inspect` admits,
  named for its effect: "Retry review", "Stop the run", "Resume", "Close it".
  Each press needs a reason, recorded with the command. Run ids and sequence
  numbers sit under "Details for audit".
- **Analytics.** `brokkr analytics` for 24 hours, 7 days or 30 days, all
  realms or one: the headline tiles, runs finished per day, where runs wait
  for you, rework, the share of step attempts by model, the by-realm table,
  and the readout's own "not recorded yet" list.

## What a press does

The card sends the command with a fresh command id and the run head you were
looking at. The plugin reads the run again first. If the head moved, nothing
is sent: the card says "not sent" and shows the run as it is now. If Brokkr
no longer admits the command, the card shows Brokkr's refusal. Otherwise the
plugin runs `brokkr operator retry` or `stop`, or `brokkr conclude`, with
your reason, followed by the console's command id so the record can be
matched. A retry is followed by `brokkr resume`, which keeps driving the run
after the panel is closed.

The card ends in one of four outcomes: **done**; **not sent**; **refused**,
with Brokkr's text verbatim; or **checking**, when Brokkr's answer was lost.
A lost answer is settled from the run's record by the command id, and the
command is never sent twice. Pressing again with the same command id returns
the first outcome.

## Not available yet, and why

| Action | Why not |
|---|---|
| Answer a clarify question in the console | No channel carries an answer to a run; write it into the spec and retry clarify |
| Talk to the reviewer | Review gates start fresh each time (decision 0056) |
| Dismiss a finding on a parked run | Superseding a finding applies to finished runs only (decision 0047) |
| Hold a parked run | `brokkr queue hold` acts on queue entries |
| Start a queued run | The dispatcher is not built ([#430](https://github.com/feedback-loop-ai/brokkr/issues/430)) |

The card shows the first four disabled, each with the reason `brokkr inspect`
gives.

This slice also leaves three things out:

- **The assistant panels.** "Ask Brokkr" and "Ask about this run" are drawn,
  and say they are not connected: the plugin does not yet hand DSH's model
  Brokkr's readouts.
- **Opening the spec.** The clarify card shows the run's worktree; opening
  its spec in DSH's Files sidebar is not built.
- **Quiet runs.** `brokkr runs --json` does not carry the staleness the
  terminal fleet shows, and the console derives none, so a run nothing
  drives still reads as running. Resume and Close it are offered whenever
  Brokkr admits them.
