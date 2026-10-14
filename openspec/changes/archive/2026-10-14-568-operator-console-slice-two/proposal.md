# Change: DSH operator console, slice two — the assistant, "Open the spec", resume without settings (#568)

Status: proposed. Fired for delivery by the operator on 2026-10-14 ("deliver slice 2").
Change: `2026-10-14-568-operator-console-slice-two`.
Builds on: `2026-10-13-568-operator-console-slice-one` (merged as `e91d881f`), whose design D6 and D7 this slice delivers.
Parent story: [#568](https://github.com/feedback-loop-ai/brokkr/issues/568), part of #330.

## Why

Slice one put Brokkr's readouts and its four operator verbs inside DSH. It drew the assistant panels and "Open the spec" as "coming later", because nothing observed documented how a third-party DSH plugin reaches DSH's model or its Files sidebar.

That has now been measured on DSH `0.2.0-rc.2`. The evidence is under `evidence/dsh-0.2.0-rc.2/`:
- a plugin can give DSH's model tools;
- it can open a prefilled session in any directory;
- it can draw a card for its own tool's calls in the transcript;
- it can open the Files tab and a file in a session.

None of these is first-party-only.

Slice one also left three rough spots:
- **Resume needs a setting.** Retry and resume only work with `BROKKR_CONSOLE_RESUME_RECIPE` or `_BUNDLE` set, although every run's start records its recipe.
- **No CI runs the plugin's tests**, which decision 0085 ruling 5 asks for.
- **Small debt from its reviews:** the WebSocket keeps buffering after closing an oversized frame and accepts unmasked client frames; a missing state folder answers "server error" instead of "not sent"; two analytics fields carry string placeholders where a typed absence belongs.

## What Changes

### Brokkr side (Rust)

- **The run view names its recipe.**
  - `RunView` gains `recipe: Option<String>`, the `bundle_name` its `run/started` manifest recorded.
  - `inspect --json` carries it, and the view's wire version moves 19 → 20.
- **Analytics absences are typed.**
  - `ModelShare.model` and `RuleCount.rule_id` become `Option<String>` on the wire, where they were the placeholders "not reported" and the absent mark (decision 0071 ruling 3).
  - The readout's text fields keep the words a reader sees.

### DSH side (plugin, `extensions/dsh/brokkr-console/`)

- **Model tools (D6), all read-only.** They run Brokkr's own readouts, bounded as slice one's runner bounds them:
  - `brokkr_runs`
  - `brokkr_inspect`
  - `brokkr_queue`
  - `brokkr_transcript`
- **A suggestion tool.** `brokkr_suggest` names one command for a run. It answers only for a command Brokkr admits at the run's current head, and otherwise says why not. Its card in the transcript reads "Suggested · <command>", with "only if the run hasn't changed since you read this" and **Use this**. Use this fills the decision card in the Brokkr panel. The operator still presses the command.
- **Ask Brokkr and Ask about this run.**
  - The panels gain a question box again.
  - Asking opens a DSH session in a registered workspace, prefilled with the scope and the question, and sends it.
  - Ask Brokkr opens in the console's workspace. Ask about this run opens in the run's worktree when the view names one.
- **"Open the spec" (D7).** On a clarify card, it:
  - opens a DSH session in the run's worktree;
  - opens the Files tab, which is rooted there;
  - prefills the clarifier's questions with "answer these in the spec, then Retry clarify".

  Nothing is written on the operator's behalf.
- **Resume without settings.** Retry-then-resume and Resume use `--recipe <the view's recipe>` by default. The two settings remain as overrides.
- **Debt:**
  - a failure to prepare the state folder answers "not sent: unstarted";
  - the WebSocket refuses unmasked client frames and stops reading once it closes.

### CI and docs

- **CI:** a `plugin tests` job runs `npm test` in `extensions/dsh/brokkr-console/` with the pinned Node, on Linux and macOS (decision 0063, decision 0085 ruling 5).
- **Docs:** the guide, the provenance note and the status row say what slice two adds. Slice one's built tasks are ticked.

## Impact

- **Wire:** the run view's version moves to 20. The console accepts 20 only.
- **Public API:** brokkr-view's grows by the `recipe` field and the two `Option` fields. The PR names the raised snapshot with a `Ruling:` line citing this change.
- **Credentials:** none are shipped. The operator configures DSH's model in DSH. Without one, the assistant's sessions fail in DSH's own words, and the rest of the console is unaffected.
- **Not in this slice:**
  - an inline answer inside the panel without a session (the measurement shows it works, but it is unlogged and has no tools);
  - a Files tree re-rooted without a session;
  - recording a spec file's path.
