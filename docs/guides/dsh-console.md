# The DSH operator console

The console puts Brokkr's fleet inside a DSH web client: a **Brokkr** entry
in DSH's left panel list, whose icon shows how many runs are waiting on you,
and a panel with two views, **Realms** and **Analytics**. It is a DSH plugin
kept in this repository at `extensions/dsh/brokkr-console/` (decision 0085,
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
3. Nothing to name for resuming: a retry is followed by `brokkr resume`
   under the recipe the run's start recorded (`brokkr inspect --json`'s
   `recipe`), as you would type `--recipe` by hand. To resume every run
   under one bundle instead, name it in `BROKKR_CONSOLE_RESUME_BUNDLE` (a
   bundle directory) or `BROKKR_CONSOLE_RESUME_RECIPE` (a recipe name). A
   run whose start recorded no recipe, with neither set, gets no retry and
   no resume, and the console says why. `resume` refuses a bundle that does
   not compile to the run's pinned manifest, so a wrong one is refused,
   never run.
4. Add the plugin to a profile by its absolute path:

   ```sh
   dsh plugin --profile web add "$PWD/extensions/dsh/brokkr-console"
   ```

5. Open DSH's web client from the URL `dsh web` prints, and pick **Brokkr**.
6. For the assistant, have a model configured in DSH (its Models page).
   Brokkr ships no credential; without one, the assistant's sessions fail in
   DSH's own words and the rest of the console is unaffected.

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

## The assistant

DSH's model reads Brokkr through four read-only tools the plugin registers:
`brokkr_runs`, `brokkr_inspect`, `brokkr_queue` and `brokkr_transcript`. Each
runs one readout, as the panel does, and an answer past 256 KiB is cut and
says so. A fifth, `brokkr_suggest`, lets the model propose one command for a
run: Brokkr is asked first, and only a command it admits at that moment
becomes a card in the session's transcript ("Suggested · Retry security
review … only if the run hasn't changed since you read this"). Anything
else comes back as Brokkr's own refusal or reason. No tool can press
anything.

- **Ask Brokkr** and **Ask about this run** each open a DSH session and send
  your question in it, prefilled with the scope and with the instruction to
  cite the ruling, session or record behind each fact and to say what is not
  recorded. Ask Brokkr opens in the console's workspace; Ask about this run
  opens in the run's worktree when Brokkr records one. The answer, and every
  tool call, is in that session in Conversations.
- **Use this** on a suggestion card carries it to the run's decision card in
  the Brokkr panel: the suggested command leads, its reason fills the reason
  box, and the card says whether the run has moved since it was read. You
  press the command, through the same check every press goes through.
- DSH folds a finished turn's tool calls under "Called tools", so a
  suggestion card is one click away in the transcript.

## Opening the spec

A run parked by clarify shows the clarifier's questions and **Open the
spec**. It opens a DSH session in the run's worktree, opens the Files tab on
that tree, and puts the questions in the composer as a draft, unsent: write
your answers into the spec, then press Retry clarify. Nothing is written for
you. Brokkr records the worktree but not which spec file a run reads, so the
console opens the tree rather than guessing a file. A run with no recorded
worktree shows the button disabled, with "no worktree recorded for this run".

## Left out

- **Quiet runs.** `brokkr runs --json` does not carry the staleness the
  terminal fleet shows, and the console derives none, so a run nothing
  drives still reads as running. Resume and Close it are offered whenever
  Brokkr admits them.
