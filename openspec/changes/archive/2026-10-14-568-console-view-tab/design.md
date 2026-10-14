# Design: the console as a conversation tab

Status: proposed. Observed on DSH `0.2.0-rc.2`, in its installed packages and in a browser (below).

## Context

- **The ring.** `ui-conversation` builds a session's view tabs from `slots.entries("conversation.view")`, each with an `id` and a `label`, in `order`. Chat is `chat` at 0, and Trajectory is `trajectory` at 10, shown only with developer tools on. `activateTarget` accepts an id with no View Definition, so a plugin entry needs no conversation data. A blank session renders no view tabs at all; DSH shows them once a conversation has a first message.
- **The folder.** `sessions.list` is a snapshot store whose `byId[sessionId].cwd` is the session's folder.
- **The theme.** DSH resolves Light, Dark or System and sets `body[data-ds-dark-theme]` for dark; its own `--dsw-*` tokens switch on that mark.
- **The composer.** DSH floats the composer over the view and publishes its live height as `--dsh-composer-height` on the scroller.

## Decisions

### D1: the conversation's realm is Brokkr's to name

The host answers `GET /brokkr-console/api/here?dir=<absolute folder>` with `{realm}`. It reads `brokkr realms --json` and resolves each realm's `path` against the console's workspace, comparing real paths, longest root first.
- A folder outside every realm is matched again through its git worktree link: the nearest `.git` *file*, `gitdir: <main>/.git/worktrees/<name>`, and that folder's `commondir`. Nothing runs git, because a repo's config can make git run commands.
- With no realms map (the readout refuses with `failed`), the workspace folder is the one hearth, `this workspace`.
- Any other refusal is reported, not guessed around.

The client matches the answer to a hearth by name; a hearth several realms share is labelled `a+b`.

### D2: one console, two places

The tab renders the same `Page` as the left-panel entry, with `inTab`, `here`, `looking` and `folder`. Both share one store and one socket. They never show at once, because opening the left-panel entry replaces the conversation.

### D3: colours are the console's, keyed to DSH's mark

The console keeps its own palettes rather than adopting `--dsw-*`, so the light look stays the boards'. The dark set takes DSH's dark neutrals (`neutral-bluish 950/875/850`, label 50/300/400) and accents (`deepseek-400`, `amber-400`, `green-400`, `red-400`), so the console sits in DSH's dark shell without a seam. A test holds every rule to a variable, and holds the two sets to the same names.

### D4: the graph is a projection of `RunView.phases`

- **Ranks.** Phases run left to right, and within a phase its columns in order. A column's nodes ran side by side and are stacked.
- **Edges.** Every node of one rank has an arrow to every node of the next.
- **Roads back.** `returns` names the phases whose ruling sent the run back. Each is drawn once, dashed amber, from the sender's last rank to the receiver's first. A phase sending the run back to itself is drawn as a loop.
- **A phase with no columns.** It is drawn as one node named for the phase.
- **Tone.** Node tone comes from the visit's `ruled` (parked or flagged amber, stopped red, returned dashed) on the phase's last rank, else from the node's `state_class`.
- **Width.** Layout is computed in the client from fixed node sizes. A graph wider than the column scrolls sideways.

## Observed

In an isolated DSH (`DSH_HOME` and XDG folders in the session scratchpad; loopback ports 38228 and 38219; a loopback stub model), started in the operator's brokkr checkout and reading its realms read-only:

- The left-panel entry and a conversation's tabs read `Chat`, `Trajectory`, `Brokkr`.
- The Brokkr tab, in a session opened in the brokkr checkout, read "this conversation is in brokkr", opened on brokkr with all five realms in the switcher, and drew `classify → verify → review → ship → done` with the ship loop.
- After choosing Dark in DSH's Settings, `body[data-ds-dark-theme]` was set and the console, in panel and tab, was dark. At 1100px the run list, the run and the assistant stacked, and the tab's bottom padding was 152px (the composer's height plus 24px).

No command was pressed and no journal was written. The isolated processes were stopped, and their browser profiles deleted.

## Second round

### D5: a pick is a phase or a node, kept by name and key

The realm keeps `{phase}` or `{phase, key}`, cleared when another run is opened. It is resolved against each new view: a phase by name, a node by `key` through `phases[].columns[].nodes[]`, its seat as the participant with that `key`. A phase's seats are the participants whose `phase` is its name, Brokkr's precomputed membership. When the view no longer holds the pick, nothing is shown and the context falls back to the run.

### D6: a seat's figures are Brokkr's cells

The pick shows `model`, `boundary`, `usage_cell`, `cache_hit_cell`, `cost_cell`, `turns_cell`, `attempts`, `effort` and `effort_pin` as they come. Brokkr had no cache-hit figure, so it gains one in `brokkr-view` beside `usage_text`: cache reads over input. `TokenUsage` already states that cache reads are a subset of input, so the share is at most a whole. The console derives nothing.

### D7: the context is a chain, closest first

`[pick, run, realm]`, shown above the question box and written into the prompt in that order: "About the seat “security” (participant key …). Its outer context is the Brokkr run “…” (…): read the run for it." The prompt's tool rules are unchanged.

### D8: the pulse is ui.html's

Same keyframes (`forge-strike` 1.8s, the halo 1.8s spreading to 3.2×), same gate: a node pulses only when its run's `summary.status` is `running` and its `state_class` is `in-active`, as ui.html gates on the selected run's status. The mark is ui.html's rail, two violet nodes and a phosphor one, and strikes in sequence while the fleet has a running run. `prefers-reduced-motion` stills both.
