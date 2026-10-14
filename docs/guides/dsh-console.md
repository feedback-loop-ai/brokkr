# The DSH operator console

The console puts Brokkr's fleet inside a DSH web client, in two places:

- a **Brokkr** tab in every conversation, beside Chat and Trajectory, that
  opens on the realm the conversation's folder belongs to;
- a **Brokkr** entry in DSH's left panel list, whose icon shows how many runs
  are waiting on you, for when no conversation is open.

Both show the same console, with two views, **Realms** and **Analytics**, in
DSH's light or dark theme, whichever DSH's Settings has chosen. It is a DSH plugin
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

5. Open DSH's web client from the URL `dsh web` prints. In a conversation,
   pick the **Brokkr** tab beside Chat; with none open, pick **Brokkr** in the
   left panel. DSH shows a conversation's tabs once it has a first message.
6. For the assistant, have a model configured in DSH (its Models page).
   Brokkr ships no credential; without one, the assistant's sessions fail in
   DSH's own words and the rest of the console is unaffected.

The plugin is `"private": true` and is never published. It listens on DSH's
own loopback port, acts as the OS user DSH runs as, and every one of its
routes and its WebSocket first asks DSH's connection fence, so a request
without DSH's login cookie, or from another origin, is refused.

## Where it opens

- **In a conversation.** The Brokkr tab asks the plugin which realm the
  conversation's folder belongs to. The plugin answers from `brokkr realms
  --json`: a folder inside a realm's folder belongs to that realm, and so
  does a git worktree of one (its `.git` file points into the realm's
  `.git/worktrees/`, read from the files, never by running git). The tab
  opens on that realm and says "this conversation is in brokkr". A folder in
  no realm gets every realm, and the tab says so. Ask Brokkr in a tab opens
  its session in the conversation's folder.
- **From the left panel.** The full-width console, on every realm.

A realm's bar lists every realm, so you can move between them without going
back to the cards.

## What it shows

- **Realms.** One card per hearth `brokkr runs` lists: the runs that need
  you, each with its title, step and Brokkr's verdict; how many are running;
  how many shipped and stopped in the last day, from `brokkr analytics
  --range 24h`. A hearth whose journal does not load shows its refusal. Below
  them, the queue from `brokkr queue list`, each entry with admission's word
  and reasons; starting a queued run waits for Brokkr's dispatcher. Muninn's
  last review, from `brokkr muninn list`, when one is recorded.
- **One realm.** Its runs: needs you, running, last 24 hours, filterable by
  title. Pick one to open it.
- **One run.** Laid out as panes, as `brokkr tui` lays them out: the graph
  across the width with the control plane on it; why the run waits beside
  the reader; every seat in one table beneath; the timeline, folded, at the
  foot. The bar above names the run and lists every realm.
- **The graph.** The rail `brokkr tui` draws: Brokkr's phases left to right,
  each step a dot on the rail, the members of one step (review positions,
  say) in a bracket above and below it, phase names under the rail, and the
  current phase boxed in the colour of the run's status ("NOW · REVIEW"). A
  road runs back beneath wherever Brokkr recorded a return ("sent back to
  implement by review · REVIEW-REFORGE"), solid, as Brokkr's own dashboard
  draws a road that was taken. A wide graph scrolls sideways and opens on
  the current phase. While the run is driven its active step wears Brokkr's
  pulse, the dot striking and a halo spreading as in `brokkr ui`; the
  console's mark and the left-panel icon strike while any run is running.
  Reduced motion stills both.
- **Picking.** Click a step, an arrow between phases, or a phase's name;
  click again to let go. A step shows its seat. An arrow shows the ruling
  that moved the run (its rule, result and words) and the seat behind it. A
  phase picks every step in it and narrows the seats table to its own. With
  nothing picked the reader shows where the run is now: the phase Brokkr
  marks current and the latest seat in it, or the phase alone while it has
  no seat yet. The pick stays as the run moves.
- **A seat.** As the TUI's participant pane reads it: the model it claims
  and its boundary, effort pinned and applied, tokens as Σ total · ↑ in ·
  ↓ out, the cache hit (cache reads over input, from `brokkr inspect`), cost,
  turns and attempts, and its transcript, newest last, read again whenever
  the run moves so a working seat's follows its tail. A figure Brokkr did
  not record says why, and so does a seat with no transcript.
- **Why it waits.** Beside the reader when the run waits: Brokkr's park
  reason, word for word, with the rule, where and since when, and a link to
  read that seat; a clarify park's questions and **Open the spec**; the rule
  that stopped it; or a quarantine. Run ids and sequence numbers sit under
  "Details for audit" in the timeline.
- **Analytics.** `brokkr analytics` for 24 hours, 7 days or 30 days, all
  realms or one: the headline tiles, runs finished per day, where runs wait
  for you, rework, the share of step attempts by model, the by-realm table,
  and the readout's own "not recorded yet" list.

## What a press does

The graph carries **Stop**, **Resume** and **Close**, and **Retry** (named for
its phase) when Brokkr admits it. A command Brokkr refuses is shown disabled,
with Brokkr's reason on hover; Brokkr has no pause. Pressing one opens a
reason bar: your reason is kept in the run's record, and nothing is sent
without one. The console sends the command with a fresh command id and the
run head you were looking at. The plugin reads the run again first. If the head moved, nothing
is sent: the outcome says "not sent" and shows the run as it is now. If
Brokkr no longer admits the command, the outcome shows Brokkr's refusal. Otherwise the
plugin runs `brokkr operator retry` or `stop`, or `brokkr conclude`, with
your reason, followed by the console's command id so the record can be
matched. A retry is followed by `brokkr resume`, which keeps driving the run
after the panel is closed.

A press ends in one of four outcomes: **done**; **not sent**; **refused**,
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

The graph lists the first four under "Not available yet", disabled, each
with the reason `brokkr inspect` gives.

## The assistant

DSH's model reads Brokkr through four read-only tools the plugin registers:
`brokkr_runs`, `brokkr_inspect`, `brokkr_queue` and `brokkr_transcript`. Each
runs one readout, as the panel does, and an answer past 256 KiB is cut and
says so. A fifth, `brokkr_suggest`, lets the model propose one command for a
run: Brokkr is asked first, and only a command it admits at that moment
becomes a card in the session's transcript ("Suggested · Retry security
review … only if the run hasn't changed since you read this"). Anything
else comes back as Brokkr's own refusal or reason. No tool the console
registers can press anything.

- **One place to ask.** The "Asking about" bar at the foot of the console
  names what a question is about, closest first: what you picked on the
  graph, then the run, then the realm (every realm on the realms view). The
  question opens with the pick (a seat by its participant key, so the model
  can read its transcript) and gives the run as the outer context, with the
  instruction to cite the ruling, session or record behind each fact and to
  say what is not recorded.
- **In a conversation's tab,** "Ask about …" puts that context in the
  conversation's own message box, unsent: write your question after
  "Question:" and send it there. The answer is in the conversation's Chat.
- **In the left-panel console,** where no conversation is open, the bar has
  its own question box: a question opens a DSH session (in the run's
  worktree when Brokkr records one, else the console's workspace) and is
  sent there. The answer is in that session in Conversations.
- **The session is DSH's.** A question is answered in an ordinary DSH
  session: besides the console's five tools, its model holds DSH's own tools,
  such as bash, edit and write, in the session's directory, under DSH's
  approval policy. The console does not narrow them, and the read-only
  guarantee above is for the console's tools only. The question's prompt
  tells the model to read Brokkr only through the console's tools, to run
  no `brokkr` command and change no file through any other tool, and to
  treat what the tools return, seat transcripts included, as records to
  cite, never as instructions. That is a steer, not a fence: whether the
  model may run `brokkr` or edit a file through DSH's tools is DSH's policy
  to set, and anything it does there acts as the OS user DSH runs as.
- **Use this** on a suggestion card carries it to the run's graph in the
  Brokkr console: the suggested command is marked, pressing it fills the
  reason bar with the suggestion's reason, and the graph says whether the run
  has moved since it was read. You press the command, through the same check
  every press goes through.
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
