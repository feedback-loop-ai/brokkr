# CLI reference

<!-- Rendered from the clap definitions by crates/brokkr-cli/src/cli_reference_tests.rs; do not edit by hand. Regenerate with: BROKKR_REGENERATE_CLI_REFERENCE=1 cargo test -p brokkr-cli --lib cli_reference -->

Every `brokkr` verb and argument with its default, and every exit code, as the binary defines them; `brokkr <verb> --help` prints the same text. An argument marked **yes** under Selector takes a full run id, a unique prefix of one, or `latest`, the run created most recently (decision 0015, and for the write paths its proposed 2026-09-28 addendum); one marked **with a journal** does so only when the workspace journal is there, and otherwise takes the id literally and refuses `latest`. Every verb also takes clap's own `-h`/`--help`, and `brokkr` itself `-V`/`--version`; the tables leave them out.

The verbs nest under thirteen roots: decision 0113's twelve and decision 0119's `completions`. Every old flat name, such as `brokkr inspect` for `brokkr runs inspect` or `brokkr verify-run` for `brokkr journal verify`, still parses as a hidden alias until 0.17, and says its new spelling on stderr.

- [`brokkr init`](#brokkr-init): Scaffold a minimal reviewable bundle and prove it compiles
- [`brokkr run`](#brokkr-run): Start a new run and drive it until it parks or finishes
- [`brokkr resume`](#brokkr-resume): Resume an existing run under its exact pinned bundle
- [`brokkr watch`](#brokkr-watch): Watch a run live: redraw the graph, seats, last ruling and seat activity whenever the journal head moves; exit when the run reaches a terminal status. Read-only, like every other readout
- [`brokkr tui`](#brokkr-tui): Explore runs interactively in the terminal: the fleet as a navigable table, one run's phase graph, seats and decision trail, and one seat's own checkpoint and session stream. Read-only like every other readout — it issues no operator commands and writes nothing to the journal
- [`brokkr doctor`](#brokkr-doctor): Verify tools, drivers, the workspace database, and optionally a bundle, without executing any agent
- [`brokkr completions`](#brokkr-completions): Print the shell completion script for bash, zsh, fish, powershell or elvish: every visible verb, flag and fixed value. `source <(COMPLETE=bash brokkr)` registers the live completer instead, which also offers run ids, phases, seats, recipes, agents, realms and queue entries
- [`brokkr runs`](#brokkr-runs): The runs in the journal: list them (the default), read one, compare two, re-run, close or command one, and the overseer that reads them all
- [`brokkr runs list`](#brokkr-runs-list): List runs in the workspace database: one clamped line per run, the fleet's sections in order and each by last activity, read a page at a time. `--json` emits the view model for scripts
- [`brokkr runs inspect`](#brokkr-runs-inspect): Explain a run: header, ruling, seats, decision trail, and the phase graph as a tree. `--phase` and `--seat` are the scoping verbs the console's clicks became; `--json` emits the view model, or with `--capability-call` the call and its verified response
- [`brokkr runs seats`](#brokkr-runs-seats): The seats of a run: the seats block `runs inspect` renders — every seat's model with the boundary its hands stood behind beside it — from the same view. `--json` prints that view verbatim, the bytes `runs inspect --json` prints
- [`brokkr runs transcript`](#brokkr-runs-transcript): Read one participant's retained local transcript — Claude, Codex or DSH — through the same bounded local derivation the TUI uses. The verb never launches, retries or resumes a provider and writes nothing to the journal
- [`brokkr runs compare`](#brokkr-runs-compare): Compare two runs' aligned outcomes: decision trails, first divergence, phases visited, per-seat costs. Read-only
- [`brokkr runs replay`](#brokkr-runs-replay): Rebuild state from the journal twice and verify determinism
- [`brokkr runs rerun`](#brokkr-runs-rerun): Re-run a past run's feature as a NEW run under another bundle or recipe, so outcomes can be compared by run id. No stored linkage
- [`brokkr runs conclude`](#brokkr-runs-conclude): Close a stopped or parked run from its journal alone — no bundle, no recipe, no effect. `resume` compiles the exact pinned recipe and refuses on any drift, which is right for the branches that spend money but leaves a run from a moved engine with no lawful ending. This appends the operator stop conclusion and nothing else, so it needs no pinned recipe to be honest about what it wrote. It cannot retry: that re-enters the policy loop, and the policy loop needs the bundle by construction. For a run believed dead: every write is fenced, so a journal that moves beneath the conclusion — something still driving the run — refuses instead of being closed over. Check `brokkr runs` first
- [`brokkr runs operator`](#brokkr-runs-operator): Record an operator command (retry \| stop \| release \| supersede \| classify \| repair) as journal events, or acknowledge a quarantined run beside its journal
- [`brokkr runs muninn`](#brokkr-runs-muninn): The standing overseer: read the fleet, propose to the operator, execute nothing. It opens the journal read-only, issues no operator command, starts no run, and records every proposal — with the run ids and sequence numbers it was derived from — in its own append-only file beside the journal
- [`brokkr runs muninn run`](#brokkr-runs-muninn-run): Derive the fleet dossier, ask one bounded seat for proposals, and record them. Nothing is executed: a proposal becomes an action only when the operator issues the command themselves
- [`brokkr runs muninn list`](#brokkr-runs-muninn-list): Read the record back: every proposal, with the run ids and sequence numbers it cited
- [`brokkr journal`](#brokkr-journal): The journal itself: export, import and verify one run, back up and restore the whole journal, and the evidence that outlives it
- [`brokkr journal export`](#brokkr-journal-export): Write the canonical NDJSON journal and pinned manifest
- [`brokkr journal import`](#brokkr-journal-import): Adopt an exported run into this journal, byte-identically — the verb paired with `journal export`. Journals never merge; one run relocates. Nothing lands unless the whole chain verifies, the events fold, the run_id does not already exist here, and the export is not a redacted derivative
- [`brokkr journal verify`](#brokkr-journal-verify): Verify an exported journal offline: chain, envelopes, fold
- [`brokkr journal backup`](#brokkr-journal-backup): Back up a whole journal — one consistent snapshot taken while writers may run, plus the versioned inventory that declares its boundary and makes the backup complete
- [`brokkr journal restore`](#brokkr-journal-restore): Restore a journal backup into a new location, after the inventory, the snapshot's digest, SQLite's integrity, every chain and the counts all pass. It starts nothing
- [`brokkr journal anchor`](#brokkr-journal-anchor): Anchor a run's journal head in refs/forge/&lt;run&gt; (tamper evidence), or verify the existing anchor with --check
- [`brokkr journal keep-refs`](#brokkr-journal-keep-refs): Keep-refs: the git objects a run's journal cites, held by refs/forge/keep/&lt;run&gt;/&lt;sha&gt; so a squash-merge, a branch delete and a gc cannot collect the evidence the journal names. Runs plant these themselves at conclusion; these verbs cover the runs that concluded before the mechanism existed, and the deliberate letting-go that is the operator's alone
- [`brokkr journal keep-refs plant`](#brokkr-journal-keep-refs-plant): Plant a keep-ref for every object the run's journal cites. Idempotent: replanting moves nothing
- [`brokkr journal keep-refs list`](#brokkr-journal-keep-refs-list): Which runs hold which exhibits — one `for-each-ref`, no journal needed. `--run` narrows the listing to one run
- [`brokkr journal keep-refs delete`](#brokkr-journal-keep-refs-delete): Let one run's exhibits go: remove refs/forge/keep/&lt;run&gt;/*. The operator's decision alone — nothing in the engine ever deletes a keep-ref, and the objects are then as mortal as gc leaves them
- [`brokkr journal evidence`](#brokkr-journal-evidence): List every historical evidence root, or retire one under the admission lock once its evidence is archived or discarded. The sealed retirement sits beside the historical record, which stays
- [`brokkr journal evidence list`](#brokkr-journal-evidence-list): List every historical evidence root: its path, whether it exists, its digest count and bytes, its retirement and its superseded history. A root whose own counts cannot be read still prints, with its problem, and makes the verb exit non-zero after every row
- [`brokkr journal evidence retire`](#brokkr-journal-evidence-retire): Retire one historical root, under the admission lock, after archiving its evidence or discarding it
- [`brokkr library`](#brokkr-library): The libraries a bundle is built from: recipes and agents, and the compiler that validates a bundle
- [`brokkr library recipes`](#brokkr-library-recipes): The recipe library: bundle directories as named, swappable delivery strategies
- [`brokkr library recipes list`](#brokkr-library-recipes-list): List recipes under --dir, then those on the personal shelf, each with its origin, project or personal; broken ones print a warning line, never abort the listing
- [`brokkr library recipes add`](#brokkr-library-recipes-add): Install a recipe from a local path or a git URL into the shelf at the repository's root, .brokkr/recipes/&lt;name&gt; where the folder is, else recipes/&lt;name&gt;, with the agents and charters it names that the agent library lacks
- [`brokkr library recipes show`](#brokkr-library-recipes-show): Print one recipe's RESOLVED bundle and, when it extends another, the composition chain it was resolved from
- [`brokkr library recipes usage`](#brokkr-library-recipes-usage): Print what each recipe's attempts used, per phase, as the engine measured their process trees: the minimum, average and maximum of peak and mean memory, peak and mean load, CPU time and wall time, over the drivers measured. An attempt journaled without usage is counted as unmeasured, never as zero
- [`brokkr library agents`](#brokkr-library-agents): The agent library: one definition per agent — description, charter, an ordered chain of abstract model names, abstract tool/MCP configuration — that seats reference by name
- [`brokkr library agents list`](#brokkr-library-agents-list): One line per agent — name, model chain, description. A broken definition prints a warning line and never aborts the listing
- [`brokkr library agents show`](#brokkr-library-agents-show): The definition as written, plus the per-chain-entry resolution the compiler would compute. An unknown name errors naming the known set
- [`brokkr library compile`](#brokkr-library-compile): Validate a bundle and print its pinned manifest and digest
- [`brokkr queue`](#brokkr-queue): The dispatcher's queue: runs waiting to start, in the journal's own database. Add, list, judge, move, hold, release, re-pin and drop entries; each change is journaled with its reason. `dispatch` starts what admission admits, and `sweep` frees what the finished runs left behind
- [`brokkr queue add`](#brokkr-queue-add): Queue a new run, taking `brokkr run`'s arguments, last among the entries of its priority. Nothing starts: the entry waits for the dispatcher
- [`brokkr queue list`](#brokkr-queue-list): The queue in order (priority, higher first, then place): each waiting entry's place, state, priority, waits, launch and admission (admissible, or why it waits or is held), then the entries that started a run, with it. `--json` emits the view model for scripts
- [`brokkr queue judge`](#brokkr-queue-judge): Judge the queue as `list` shows it, and first latch on each waiting entry the realm drift found: from then on it is held until the operator re-pins, re-queues or drops it, whatever the map comes to. Each latch is journaled with the reason
- [`brokkr queue move`](#brokkr-queue-move): Put an entry at another place in the queue, among the entries of its own priority: priority always wins, so a place above a higher priority or below a lower one leaves it first or last among its own. Says the place it holds after
- [`brokkr queue hold`](#brokkr-queue-hold): Keep an entry in its place, not to be started until released
- [`brokkr queue release`](#brokkr-queue-release): Let a held entry be started again
- [`brokkr queue repin`](#brokkr-queue-repin): Re-pin a waiting entry to the realms map that would govern it now: how the operator releases an entry `judge` latched a realm-drift hold on, accepting the differences it found. Refused when the map on disk is not the one the latch found; judge the queue again
- [`brokkr queue drop`](#brokkr-queue-drop): Take an entry out of the queue. One that started a run cannot be
- [`brokkr queue merged`](#brokkr-queue-merged): Record that an entry's work merged, journaled with the reason (name the pull request and its merge commit): an entry waiting on ENTRY:merged may start from then on. Brokkr reads no code host, so this record is the merge's one witness. Only an entry that started a run can be recorded merged
- [`brokkr queue dispatch`](#brokkr-queue-dispatch): Start what admission admits from the journal's queue, each run handed to a driver of its own: one pass with `--once` under the journal's dispatch lock, the bound's command and the holder's readout
- [`brokkr queue dispatch set-bound`](#brokkr-queue-dispatch-set-bound): Set the journal's bound on the refusals counted against one entry before the dispatcher holds it, a whole number of at least 1, journaled as the operator's command
- [`brokkr queue dispatch status`](#brokkr-queue-dispatch-status): Say whether a dispatcher holds the journal, the bound in force, and each entry left launching or linked with no `spawn` row. Opens the journal read-only and writes nothing
- [`brokkr queue sweep`](#brokkr-queue-sweep): Remove the run-local seat scratch trees the journal proves free and record what each complete removal freed, and reap the hands session trees whose owners are dead. One pass under the journal's dispatch lock; `--dry-run` lists what a pass would remove, with sizes, and writes nothing
- [`brokkr stats`](#brokkr-stats): What the runs did, read across them: analytics, a recipe's graph, a run's costs and its delivery ledger
- [`brokkr stats analytics`](#brokkr-stats-analytics): What the runs did across every hearth, for a range and an optional realm: shipped, start to end, waiting on you, rework, cost and served models. `--json` emits the view model
- [`brokkr stats graph`](#brokkr-stats-graph): One recipe's state machine and its traffic over a range. `--json` emits the view model
- [`brokkr stats costs`](#brokkr-stats-costs): Per-seat cost and session accounting from journal checkpoints — the LaneTally join surface (stable seat ids, journal-derived)
- [`brokkr stats ledger`](#brokkr-stats-ledger): Render the shipper's delivery ledger from journal and repository evidence
- [`brokkr system`](#brokkr-system): This host and its world: the realms map, the secrets store, a harness probe and the read-only web surface
- [`brokkr system realms`](#brokkr-system-realms): List the world: each realm with its path, default branch and current HEAD, and the journal the world writes. Read-only, like every other readout
- [`brokkr system secrets`](#brokkr-system-secrets): Manage the operator-side secrets store: bundles and journals carry NAMES only; values live in this env-format file outside version control. There is no value-printing verb
- [`brokkr system secrets set`](#brokkr-system-secrets-set): Bind NAME to a value read from STDIN (never argv — the CLI obeys its own injection discipline). Creates the store 0600
- [`brokkr system secrets list`](#brokkr-system-secrets-list): Print bound names, one per line — names, never values
- [`brokkr system secrets remove`](#brokkr-system-secrets-remove): Remove NAME from the store
- [`brokkr system host`](#brokkr-system-host): Manage the machine's host configuration
- [`brokkr system host upgrade`](#brokkr-system-host-upgrade): Upgrade the machine's host configuration to forge.host/v5
- [`brokkr system probe`](#brokkr-system-probe): Measure an agent CLI for its adapter's facts, spending its bound credentials: host only, never CI
- [`brokkr system probe harness`](#brokkr-system-probe-harness): Run one agent CLI headless against a scratch repository and HOME, and report each fact its adapter must declare as measured, unmeasured or unsupported, beside the adapter's own fields and the seat eligibility the facts derive
- [`brokkr system ui`](#brokkr-system-ui): Serve the embedded read-only surface on loopback
- [Exit codes](#exit-codes)

## brokkr init

Scaffold a minimal reviewable bundle and prove it compiles

```text
Usage: brokkr init [OPTIONS] <DIR>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `<DIR>` |  |  | The repository root whose .brokkr/ folder is written; a folder that already holds more than moved state is refused, never overwritten |
| `--cli <CLI>` |  |  | The agent CLI the seats are hired from (default: the first of claude, codex and dsh on PATH); scaffolded for even where PATH lacks it |

## brokkr run

Start a new run and drive it until it parks or finishes

```text
Usage: brokkr run [OPTIONS] --feature <FEATURE>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--enter <ENTER>` |  |  | Enter at an admitted gate over work already on the branch |
| `--continues <CONTINUES>...` |  | yes | Earlier concluded run continued by this run (id, prefix, or latest); repeat in order |
| `--bundle <BUNDLE>` |  |  | The bundle directory to deliver under; without it or `--recipe`, the discovered .brokkr/ folder. `resume` compiles it to the run's pinned manifest and refuses any drift |
| `--recipe <RECIPE>` |  |  | Named recipe, resolved to &lt;recipes-dir&gt;/&lt;name&gt; |
| `--recipes-dir <RECIPES_DIR>` |  |  | The recipe library `--recipe` is resolved in (default the found .brokkr/recipes, else recipes) |
| `--secrets-file <SECRETS_FILE>` |  |  | Operator-side secrets store for seats with declared bindings (default .brokkr/state/secrets.env, or &lt;workdir&gt;/.forge/secrets.env) |
| `--feature <FEATURE>` |  |  | The feature the run delivers, as text: recorded when the run starts and handed to its seats |
| `--realms <REALMS>` |  |  | The world's map: realms and the journal they share. Defaults to .brokkr/realms.json, else ./realms.json; a map named here and missing or malformed is a refusal, never a silent fallback |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, the found folder's .brokkr/state/forge.db, or a library's .forge/forge.db |
| `--repo <REPO>` |  |  | The repository the run operates on: the bundle is compiled against its realm and the engine works in it. Without it, the workspace (the current directory) is compiled against, and the engine gets no repository override, so it works there too |
| `--dispatch <DISPATCH>` |  |  | Canonical forge-dispatch/v2 JSON. When present the run id, Looper/grant correlation, recipe, repository, budget, and producer bounds are pinned into an immutable run-manifest/v2 |
| `--no-view` |  |  | On a terminal, print the plain lines and the summary instead of opening the run view. Off a terminal they always print |

## brokkr resume

Resume an existing run under its exact pinned bundle

```text
Usage: brokkr resume [OPTIONS] --run <RUN>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--bundle <BUNDLE>` |  |  | The bundle directory to deliver under; without it or `--recipe`, the discovered .brokkr/ folder. `resume` compiles it to the run's pinned manifest and refuses any drift |
| `--recipe <RECIPE>` |  |  | Named recipe, resolved to &lt;recipes-dir&gt;/&lt;name&gt; |
| `--recipes-dir <RECIPES_DIR>` |  |  | The recipe library `--recipe` is resolved in (default the found .brokkr/recipes, else recipes) |
| `--secrets-file <SECRETS_FILE>` |  |  | Operator-side secrets store for seats with declared bindings (default .brokkr/state/secrets.env, or &lt;workdir&gt;/.forge/secrets.env) |
| `--run <RUN>` |  | yes | Full run id, a unique run-id prefix, or `latest` |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default the found folder's .brokkr/realms.json, or a library's ./realms.json, when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, the found folder's .brokkr/state/forge.db, or a library's .forge/forge.db |
| `--repo <REPO>` |  |  | The repository the resumed run operates on. Without it, the engine gets no repository override and works in the current directory |
| `--no-view` |  |  | On a terminal, print the plain lines and the summary instead of opening the run view. Off a terminal they always print |

## brokkr watch

Watch a run live: redraw the graph, seats, last ruling and seat activity whenever the journal head moves; exit when the run reaches a terminal status. Read-only, like every other readout

```text
Usage: brokkr watch [OPTIONS] --run <RUN>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--run <RUN>` |  | yes | Full run id, a unique run-id prefix, or `latest` |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default the found folder's .brokkr/realms.json, or a library's ./realms.json, when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, the found folder's .brokkr/state/forge.db, or a library's .forge/forge.db |
| `--once` |  |  | Print one frame and exit |
| `--interval <INTERVAL_MS>` | `750` |  | Poll interval in milliseconds (floored at 100) |
| `--no-view` |  |  | On a terminal, redraw the text frames instead of opening the run view. Off a terminal, and with `--once`, they always print |

## brokkr tui

Explore runs interactively in the terminal: the fleet as a navigable table, one run's phase graph, seats and decision trail, and one seat's own checkpoint and session stream. Read-only like every other readout — it issues no operator commands and writes nothing to the journal

```text
Usage: brokkr tui [OPTIONS]
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--run <RUN>` |  | yes | Full run id, a unique run-id prefix, or `latest`; opens directly at that run's level |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default the found folder's .brokkr/realms.json, or a library's ./realms.json, when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, the found folder's .brokkr/state/forge.db, or a library's .forge/forge.db |

## brokkr doctor

Verify tools, drivers, the workspace database, and optionally a bundle, without executing any agent

```text
Usage: brokkr doctor [OPTIONS]
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--bundle <BUNDLE>` |  |  | A bundle directory to compile and check as well (default the found .brokkr/ folder, compiled once for the whole report and its folder line) |
| `--realms <REALMS>` |  |  | The world's map whose realm house declarations doctor checks (default the found folder's .brokkr/realms.json, or a library's ./realms.json, when present) |
| `--db <DB>` |  |  | The workspace journal whose database doctor opens. Outranks the map's journal; without either, the found folder's .brokkr/state/forge.db, or a library's .forge/forge.db |
| `--secrets-file <SECRETS_FILE>` |  |  | Operator-side secrets store, so doctor can say which declared credentials a route is taking from the ambient environment instead (default .brokkr/state/secrets.env, else .forge/secrets.env) |

## brokkr completions

Print the shell completion script for bash, zsh, fish, powershell or elvish: every visible verb, flag and fixed value. `source <(COMPLETE=bash brokkr)` registers the live completer instead, which also offers run ids, phases, seats, recipes, agents, realms and queue entries

```text
Usage: brokkr completions <SHELL>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `<SHELL>` |  |  | The shell the script is written for |

## brokkr runs

The runs in the journal: list them (the default), read one, compare two, re-run, close or command one, and the overseer that reads them all

```text
Usage: brokkr runs [OPTIONS]
       brokkr runs <COMMAND>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default the found folder's .brokkr/realms.json, or a library's ./realms.json, when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, the found folder's .brokkr/state/forge.db, or a library's .forge/forge.db |
| `--json` |  |  | Emit the view model verbatim — this is what scripts read |
| `--limit <N>` |  |  | Print one page of at most N runs, and where the next page starts (`next`) |
| `--after <CURSOR>` |  |  | Print the page after this cursor: a page's `next` |
| `--run <RUN>` |  |  | Print one run's row: the same row this listing gives it, and no other. Refused by name when the id names no run |
| `--ending <ENDING>` |  |  | List only the runs that ended so: one of the twelve endings |
| `--needs-operator` |  |  | List only the runs that want the operator: a ruling (`!`) or an action (`>`) |
| `--hide-settled` |  |  | Leave out the settled runs: every run that neither wants the operator nor is live |

## brokkr runs list

List runs in the workspace database: one clamped line per run, the fleet's sections in order and each by last activity, read a page at a time. `--json` emits the view model for scripts

```text
Usage: brokkr runs list [OPTIONS]
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default the found folder's .brokkr/realms.json, or a library's ./realms.json, when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, the found folder's .brokkr/state/forge.db, or a library's .forge/forge.db |
| `--json` |  |  | Emit the view model verbatim — this is what scripts read |
| `--limit <N>` |  |  | Print one page of at most N runs, and where the next page starts (`next`) |
| `--after <CURSOR>` |  |  | Print the page after this cursor: a page's `next` |
| `--run <RUN>` |  |  | Print one run's row: the same row this listing gives it, and no other. Refused by name when the id names no run |
| `--ending <ENDING>` |  |  | List only the runs that ended so: one of the twelve endings |
| `--needs-operator` |  |  | List only the runs that want the operator: a ruling (`!`) or an action (`>`) |
| `--hide-settled` |  |  | Leave out the settled runs: every run that neither wants the operator nor is live |

## brokkr runs inspect

Explain a run: header, ruling, seats, decision trail, and the phase graph as a tree. `--phase` and `--seat` are the scoping verbs the console's clicks became; `--json` emits the view model, or with `--capability-call` the call and its verified response

```text
Usage: brokkr runs inspect [OPTIONS] --run <RUN>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--run <RUN>` |  | yes | Full run id, a unique run-id prefix, or `latest` |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default the found folder's .brokkr/realms.json, or a library's ./realms.json, when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, the found folder's .brokkr/state/forge.db, or a library's .forge/forge.db |
| `--json` |  |  | Emit the view model verbatim — this is what scripts read |
| `--phase <PHASE>` |  |  | Scope the readout to one phase |
| `--seat <SEAT>` |  |  | Scope the readout to one seat, by label or participant key |
| `--capability-call <CALL_ID>` |  |  | Open the retained response the call this id names in the run cites, verified against its recorded digest, never a path; with --json, beside the call's provenance |
| `--repo <REPO>` |  |  | The operated repository whose run-local directory holds the call's retained response, as `stats ledger --repo` resolves it; without it, the workspace's own |

## brokkr runs seats

The seats of a run: the seats block `runs inspect` renders — every seat's model with the boundary its hands stood behind beside it — from the same view. `--json` prints that view verbatim, the bytes `runs inspect --json` prints

```text
Usage: brokkr runs seats [OPTIONS] --run <RUN>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--run <RUN>` |  | yes | Full run id, a unique run-id prefix, or `latest` |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default the found folder's .brokkr/realms.json, or a library's ./realms.json, when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, the found folder's .brokkr/state/forge.db, or a library's .forge/forge.db |
| `--json` |  |  | Emit the view model verbatim — `runs inspect --json`'s own bytes |

## brokkr runs transcript

Read one participant's retained local transcript — Claude, Codex or DSH — through the same bounded local derivation the TUI uses. The verb never launches, retries or resumes a provider and writes nothing to the journal

```text
Usage: brokkr runs transcript [OPTIONS] --run <RUN> --seat <SEAT>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--run <RUN>` |  | yes | Full run id, a unique run-id prefix, or `latest` |
| `--seat <SEAT>` |  |  | A participant key, or a label that is unique within the run |
| `--turn <TURN>` |  |  | One-based displayed-turn index; omitted reads the whole transcript |
| `--json` |  |  | Emit the `brokkr.transcript/v1` document |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default the found folder's .brokkr/realms.json, or a library's ./realms.json, when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, the found folder's .brokkr/state/forge.db, or a library's .forge/forge.db |

## brokkr runs compare

Compare two runs' aligned outcomes: decision trails, first divergence, phases visited, per-seat costs. Read-only

```text
Usage: brokkr runs compare [OPTIONS] <RUN_A> <RUN_B>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `<RUN_A>` |  | yes | The first run: a full run id, a unique run-id prefix, or `latest` |
| `<RUN_B>` |  | yes | The second run, named the same ways |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default the found folder's .brokkr/realms.json, or a library's ./realms.json, when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, the found folder's .brokkr/state/forge.db, or a library's .forge/forge.db |

## brokkr runs replay

Rebuild state from the journal twice and verify determinism

```text
Usage: brokkr runs replay [OPTIONS] --run <RUN>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--run <RUN>` |  | yes | Full run id, a unique run-id prefix, or `latest` |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default the found folder's .brokkr/realms.json, or a library's ./realms.json, when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, the found folder's .brokkr/state/forge.db, or a library's .forge/forge.db |

## brokkr runs rerun

Re-run a past run's feature as a NEW run under another bundle or recipe, so outcomes can be compared by run id. No stored linkage

```text
Usage: brokkr runs rerun [OPTIONS] --run <RUN>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--run <RUN>` |  | yes | The source run whose feature is re-run |
| `--bundle <BUNDLE>` |  |  | The bundle directory to deliver under; without it or `--recipe`, the discovered .brokkr/ folder. `resume` compiles it to the run's pinned manifest and refuses any drift |
| `--recipe <RECIPE>` |  |  | Named recipe, resolved to &lt;recipes-dir&gt;/&lt;name&gt; |
| `--recipes-dir <RECIPES_DIR>` |  |  | The recipe library `--recipe` is resolved in (default the found .brokkr/recipes, else recipes) |
| `--secrets-file <SECRETS_FILE>` |  |  | Operator-side secrets store for seats with declared bindings (default .brokkr/state/secrets.env, or &lt;workdir&gt;/.forge/secrets.env) |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default the found folder's .brokkr/realms.json, or a library's ./realms.json, when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, the found folder's .brokkr/state/forge.db, or a library's .forge/forge.db |
| `--repo <REPO>` |  |  | The repository the new run operates on: the bundle is compiled against its realm and the engine works in it. Without it, the workspace (the current directory) is compiled against, and the engine gets no repository override, so it works there too |

## brokkr runs conclude

Close a stopped or parked run from its journal alone — no bundle, no recipe, no effect. `resume` compiles the exact pinned recipe and refuses on any drift, which is right for the branches that spend money but leaves a run from a moved engine with no lawful ending. This appends the operator stop conclusion and nothing else, so it needs no pinned recipe to be honest about what it wrote. It cannot retry: that re-enters the policy loop, and the policy loop needs the bundle by construction. For a run believed dead: every write is fenced, so a journal that moves beneath the conclusion — something still driving the run — refuses instead of being closed over. Check `brokkr runs` first

```text
Usage: brokkr runs conclude [OPTIONS] --run <RUN> --reason <REASON>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--run <RUN>` |  | yes | Full run id, a unique run-id prefix, or `latest` |
| `--reason <REASON>` |  |  | Why the run is closed, recorded with the stop conclusion |
| `--cause <CAUSE>` |  |  | Required: how the run ended, one of superseded, infrastructure, delivered-elsewhere, withdrawn, exhausted or operator-stopped |
| `--by-run <BY_RUN>` |  |  | With --cause superseded, required: the run that replaced this one, read as written |
| `--by-realm <BY_REALM>` |  |  | With --cause superseded: the realm that run was read in. Omitted for the workspace journal, which is every one-hearth world |
| `--by-ref <BY_REF>` |  |  | With --cause delivered-elsewhere, required: the commit or pull request the work landed by, recorded as written and not resolved |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default the found folder's .brokkr/realms.json, or a library's ./realms.json, when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, the found folder's .brokkr/state/forge.db, or a library's .forge/forge.db |

## brokkr runs operator

Record an operator command (retry | stop | release | supersede | classify | repair) as journal events, or acknowledge a quarantined run beside its journal

```text
Usage: brokkr runs operator [OPTIONS] --run <RUN> --reason <REASON> <COMMAND>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--member <MEMBER>...` |  |  | retry only: run this panel member; repeat to run several. Sequence members use &lt;step&gt;:&lt;member&gt; |
| `--repo <REPO>` |  |  | With --member: the operated repository, defaulting as resume does |
| `--bundle <BUNDLE>` |  |  | The bundle directory to deliver under; without it or `--recipe`, the discovered .brokkr/ folder. `resume` compiles it to the run's pinned manifest and refuses any drift |
| `--recipe <RECIPE>` |  |  | Named recipe, resolved to &lt;recipes-dir&gt;/&lt;name&gt; |
| `--recipes-dir <RECIPES_DIR>` |  |  | The recipe library `--recipe` is resolved in (default the found .brokkr/recipes, else recipes) |
| `--secrets-file <SECRETS_FILE>` |  |  | Operator-side secrets store for seats with declared bindings (default .brokkr/state/secrets.env, or &lt;workdir&gt;/.forge/secrets.env) |
| `--run <RUN>` |  | yes | Full run id, a unique run-id prefix, or `latest` |
| `<COMMAND>` |  |  | "retry" re-runs the current phase; "stop" ends the run, naming its cause; "release" attests that every process the run's ended attempts owned has ended, and frees the worktree their writer lease holds; "supersede" records that residual findings on a run that has already finished are closed by another run; "classify" names how a stopped run ended; "repair" names the late tail an event after a run's terminal one left, so the run folds again as it ended, the tail kept and never applied; "settle" records that a quarantined run's writers are proven ended, so its worktree is released; "acknowledge" lists a quarantined run under older until its journal moves |
| `--reason <REASON>` |  |  | Why the operator issued the command, recorded with it |
| `--cause <CAUSE>` |  |  | stop and classify, and required there: how the run ended, one of superseded, infrastructure, delivered-elsewhere, withdrawn, exhausted or operator-stopped |
| `--findings <FINDINGS>...` |  |  | supersede only: the residual findings this closes, by the sequence number of the ruling each was read from. Repeatable, or one comma-separated list |
| `--by-run <BY_RUN>` |  |  | supersede, stop or classify only: with supersede, the run that closed the findings; with stop or classify --cause superseded, the run that replaced this one. Read as written, never resolved through the selector |
| `--by-seq <BY_SEQ>` |  |  | supersede only: the `transition/decided` in that run which closed them |
| `--by-realm <BY_REALM>` |  |  | supersede, stop or classify only: the realm that run was read in. Omitted for the workspace journal, which is every one-hearth world |
| `--by-ref <BY_REF>` |  |  | stop or classify only: with delivered-elsewhere, the commit or pull request the work landed by, recorded as written and not resolved |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default the found folder's .brokkr/realms.json, or a library's ./realms.json, when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, the found folder's .brokkr/state/forge.db, or a library's .forge/forge.db |

## brokkr runs muninn

The standing overseer: read the fleet, propose to the operator, execute nothing. It opens the journal read-only, issues no operator command, starts no run, and records every proposal — with the run ids and sequence numbers it was derived from — in its own append-only file beside the journal

```text
Usage: brokkr runs muninn <COMMAND>
```

## brokkr runs muninn run

Derive the fleet dossier, ask one bounded seat for proposals, and record them. Nothing is executed: a proposal becomes an action only when the operator issues the command themselves

```text
Usage: brokkr runs muninn run [OPTIONS]
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the fleet this reading covers (default .brokkr/realms.json, else ./realms.json, when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, .brokkr/state/forge.db, or .forge/forge.db in a library |
| `--agents-dir <AGENTS_DIR>` |  |  | The agent library Muninn's own seat is hired from (default .brokkr/agents, else agents) |
| `--adapters-dir <ADAPTERS_DIR>` |  |  | The adapter library that seat's model is resolved against (default .brokkr/adapters, else adapters) |
| `--record <RECORD>` |  |  | The append-only file each proposal is recorded in (default .brokkr/state/muninn.ndjson, or a library's .forge/muninn.ndjson) |

## brokkr runs muninn list

Read the record back: every proposal, with the run ids and sequence numbers it cited

```text
Usage: brokkr runs muninn list [OPTIONS]
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--record <RECORD>` |  |  | The record file to read (default .brokkr/state/muninn.ndjson, or a library's .forge/muninn.ndjson) |
| `--json` |  |  | Emit the recorded entries verbatim — this is what scripts read |

## brokkr journal

The journal itself: export, import and verify one run, back up and restore the whole journal, and the evidence that outlives it

```text
Usage: brokkr journal <COMMAND>
```

## brokkr journal export

Write the canonical NDJSON journal and pinned manifest

```text
Usage: brokkr journal export [OPTIONS] --run <RUN>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--run <RUN>` |  | yes | Full run id, a unique run-id prefix, or `latest` |
| `--out <OUT>` | `.` |  | The directory `<run>.ndjson` and `<run>.manifest.json` are written into, created when absent |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default the found folder's .brokkr/realms.json, or a library's ./realms.json, when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, the found folder's .brokkr/state/forge.db, or a library's .forge/forge.db |
| `--redact` |  |  | Also write a sanitized copy for publishable fixtures — `<run>.redacted.ndjson` and `<run>.redacted.manifest.json` — with every absolute path in event payloads rewritten to a stable placeholder. The verbatim pair is written unchanged; the redacted copy's recorded hashes no longer verify, and its manifest says so |

## brokkr journal import

Adopt an exported run into this journal, byte-identically — the verb paired with `journal export`. Journals never merge; one run relocates. Nothing lands unless the whole chain verifies, the events fold, the run_id does not already exist here, and the export is not a redacted derivative

```text
Usage: brokkr journal import [OPTIONS] --from <FROM>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--from <FROM>` |  |  | The exported `<run>.ndjson`. Its `<run>.manifest.json` sidecar is read from beside it and must be there |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default the found folder's .brokkr/realms.json, or a library's ./realms.json, when present) |
| `--db <DB>` |  |  | The destination journal. Outranks the map's journal; without either, the found folder's .brokkr/state/forge.db, or a library's .forge/forge.db |

## brokkr journal verify

Verify an exported journal offline: chain, envelopes, fold

```text
Usage: brokkr journal verify <FILE>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `<FILE>` |  |  | The exported `<run>.ndjson` journal to verify |

## brokkr journal backup

Back up a whole journal — one consistent snapshot taken while writers may run, plus the versioned inventory that declares its boundary and makes the backup complete

```text
Usage: brokkr journal backup [OPTIONS] --to <TO>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--to <TO>` |  |  | The directory the snapshot and its inventory are written into; it must be absent or empty, and nothing in it is overwritten |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default the found folder's .brokkr/realms.json, or a library's ./realms.json, when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, the found folder's .brokkr/state/forge.db, or a library's .forge/forge.db |

## brokkr journal restore

Restore a journal backup into a new location, after the inventory, the snapshot's digest, SQLite's integrity, every chain and the counts all pass. It starts nothing

```text
Usage: brokkr journal restore --from <FROM> --to <TO>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--from <FROM>` |  |  | The backup directory holding the snapshot and its inventory |
| `--to <TO>` |  |  | The new journal path. It must not exist; a live journal is never a restore's destination |

## brokkr journal anchor

Anchor a run's journal head in refs/forge/&lt;run&gt; (tamper evidence), or verify the existing anchor with --check

```text
Usage: brokkr journal anchor [OPTIONS] --run <RUN>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--run <RUN>` |  | yes | Full run id, a unique run-id prefix, or `latest` |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default the found folder's .brokkr/realms.json, or a library's ./realms.json, when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, the found folder's .brokkr/state/forge.db, or a library's .forge/forge.db |
| `--repo <REPO>` | `.` |  | The git repository whose refs/forge/&lt;run&gt; holds the anchor |
| `--check` |  |  | Verify instead of writing a new anchor |

## brokkr journal keep-refs

Keep-refs: the git objects a run's journal cites, held by refs/forge/keep/&lt;run&gt;/&lt;sha&gt; so a squash-merge, a branch delete and a gc cannot collect the evidence the journal names. Runs plant these themselves at conclusion; these verbs cover the runs that concluded before the mechanism existed, and the deliberate letting-go that is the operator's alone

```text
Usage: brokkr journal keep-refs <COMMAND>
```

## brokkr journal keep-refs plant

Plant a keep-ref for every object the run's journal cites. Idempotent: replanting moves nothing

```text
Usage: brokkr journal keep-refs plant [OPTIONS] --run <RUN>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--run <RUN>` |  | yes | Full run id, a unique run-id prefix, or `latest` |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default the found folder's .brokkr/realms.json, or a library's ./realms.json, when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, the found folder's .brokkr/state/forge.db, or a library's .forge/forge.db |
| `--repo <REPO>` | `.` |  | The git repository the keep-refs are planted in |

## brokkr journal keep-refs list

Which runs hold which exhibits — one `for-each-ref`, no journal needed. `--run` narrows the listing to one run

```text
Usage: brokkr journal keep-refs list [OPTIONS]
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--run <RUN>` |  | with a journal | A run id; with the workspace journal there, also a unique prefix or `latest`, and without it the id is taken literally and `latest` is refused. Omitted, every run holding keep-refs in this repository |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default the found folder's .brokkr/realms.json, or a library's ./realms.json, when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, the found folder's .brokkr/state/forge.db, or a library's .forge/forge.db |
| `--repo <REPO>` | `.` |  | The git repository whose keep-refs are listed |

## brokkr journal keep-refs delete

Let one run's exhibits go: remove refs/forge/keep/&lt;run&gt;/*. The operator's decision alone — nothing in the engine ever deletes a keep-ref, and the objects are then as mortal as gc leaves them

```text
Usage: brokkr journal keep-refs delete [OPTIONS] --run <RUN>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--run <RUN>` |  | with a journal | A run id; with the workspace journal there, also a unique prefix or `latest`, and without it the id is taken literally and `latest` is refused |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default the found folder's .brokkr/realms.json, or a library's ./realms.json, when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, the found folder's .brokkr/state/forge.db, or a library's .forge/forge.db |
| `--repo <REPO>` | `.` |  | The git repository the keep-refs are removed from |

## brokkr journal evidence

List every historical evidence root, or retire one under the admission lock once its evidence is archived or discarded. The sealed retirement sits beside the historical record, which stays

```text
Usage: brokkr journal evidence <COMMAND>
```

## brokkr journal evidence list

List every historical evidence root: its path, whether it exists, its digest count and bytes, its retirement and its superseded history. A root whose own counts cannot be read still prints, with its problem, and makes the verb exit non-zero after every row

```text
Usage: brokkr journal evidence list [OPTIONS]
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--json` |  |  | Print the roots as a JSON array |

## brokkr journal evidence retire

Retire one historical root, under the admission lock, after archiving its evidence or discarding it

```text
Usage: brokkr journal evidence retire [OPTIONS] --root <ROOT> --reason <REASON> <--archived-to <ARCHIVED_TO>|--discarded>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--root <ROOT>` |  |  | The artifact root the historical record names, as `journal evidence list` prints it |
| `--archived-to <ARCHIVED_TO>` |  |  | Archive every artifact into this absent or empty directory, write the inventory last, and keep the root. The destination is resolved by the filesystem, so a link is followed before `..`, and it must resolve outside the coordination root and the root being retired; the resolved path is the one recorded. A root that does not stand cannot be archived: `--discarded` is the record for an absent root |
| `--discarded` |  |  | Discard the evidence: remove the artifact root by a walk that follows no link and crosses no filesystem |
| `--reason <REASON>` |  |  | Why the operator retires it, in their words |
| `--json` |  |  | Print the retirement as JSON |

## brokkr library

The libraries a bundle is built from: recipes and agents, and the compiler that validates a bundle

```text
Usage: brokkr library <COMMAND>
```

## brokkr library recipes

The recipe library: bundle directories as named, swappable delivery strategies

```text
Usage: brokkr library recipes <COMMAND>
```

## brokkr library recipes list

List recipes under --dir, then those on the personal shelf, each with its origin, project or personal; broken ones print a warning line, never abort the listing

```text
Usage: brokkr library recipes list [OPTIONS]
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--dir <DIR>` |  |  | The recipe library directory (default the found .brokkr/recipes, listing nothing while it is absent, else recipes) |

## brokkr library recipes add

Install a recipe from a local path or a git URL into the shelf at the repository's root, .brokkr/recipes/&lt;name&gt; where the folder is, else recipes/&lt;name&gt;, with the agents and charters it names that the agent library lacks

```text
Usage: brokkr library recipes add [OPTIONS] --name <NAME> <SOURCE>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `<SOURCE>` |  |  | A local bundle directory or a git URL |
| `--name <NAME>` |  |  | The name the recipe is installed under |
| `--path <PATH>` |  |  | With a git URL, the bundle's directory inside the clone, a relative path of plain names (`recipes/fast`), walked without following a link; without it, the clone's root or its one subdirectory holding a bundle.json. The bases its recipe extends come from beside it |
| `--personal` |  |  | Install onto the personal shelf instead, $XDG_CONFIG_HOME/brokkr/recipes/&lt;name&gt;, else $HOME/.config/brokkr/recipes/&lt;name&gt;, beside host.json and in no repository; it copies no agent or charter |

## brokkr library recipes show

Print one recipe's RESOLVED bundle and, when it extends another, the composition chain it was resolved from

```text
Usage: brokkr library recipes show [OPTIONS] <NAME>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `<NAME>` |  |  | The recipe's name, resolved to &lt;dir&gt;/&lt;name&gt;, else to the personal shelf's; a name on both refuses |
| `--dir <DIR>` |  |  | The recipe library directory (default the found .brokkr/recipes, else recipes) |

## brokkr library recipes usage

Print what each recipe's attempts used, per phase, as the engine measured their process trees: the minimum, average and maximum of peak and mean memory, peak and mean load, CPU time and wall time, over the drivers measured. An attempt journaled without usage is counted as unmeasured, never as zero

```text
Usage: brokkr library recipes usage [OPTIONS] [NAME]
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `<NAME>` |  |  | One recipe, by the name its runs started under (default every recipe the journal has runs of) |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default the found folder's .brokkr/realms.json, or a library's ./realms.json, when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, the found folder's .brokkr/state/forge.db, or a library's .forge/forge.db |
| `--json` |  |  | Print the rows as JSON |

## brokkr library agents

The agent library: one definition per agent — description, charter, an ordered chain of abstract model names, abstract tool/MCP configuration — that seats reference by name

```text
Usage: brokkr library agents <COMMAND>
```

## brokkr library agents list

One line per agent — name, model chain, description. A broken definition prints a warning line and never aborts the listing

```text
Usage: brokkr library agents list [OPTIONS]
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--agents-dir <AGENTS_DIR>` |  |  | The agent library directory (default .brokkr/agents, else agents) |

## brokkr library agents show

The definition as written, plus the per-chain-entry resolution the compiler would compute. An unknown name errors naming the known set

```text
Usage: brokkr library agents show [OPTIONS] <NAME>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `<NAME>` |  |  | The agent's name in the library |
| `--agents-dir <AGENTS_DIR>` |  |  | The agent library directory (default .brokkr/agents, else agents) |
| `--adapters-dir <ADAPTERS_DIR>` |  |  | The adapter library the chain's models are resolved against (default .brokkr/adapters, else adapters) |

## brokkr library compile

Validate a bundle and print its pinned manifest and digest

```text
Usage: brokkr library compile --bundle <BUNDLE>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--bundle <BUNDLE>` |  |  | The bundle directory to validate, compiled against the workspace |

## brokkr queue

The dispatcher's queue: runs waiting to start, in the journal's own database. Add, list, judge, move, hold, release, re-pin and drop entries; each change is journaled with its reason. `dispatch` starts what admission admits, and `sweep` frees what the finished runs left behind

```text
Usage: brokkr queue <COMMAND>
```

## brokkr queue add

Queue a new run, taking `brokkr run`'s arguments, last among the entries of its priority. Nothing starts: the entry waits for the dispatcher

```text
Usage: brokkr queue add [OPTIONS] --feature <FEATURE> --reason <REASON>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--bundle <BUNDLE>` |  |  | The bundle directory to deliver under; without it or `--recipe`, the discovered .brokkr/ folder. `resume` compiles it to the run's pinned manifest and refuses any drift |
| `--recipe <RECIPE>` |  |  | Named recipe, resolved to &lt;recipes-dir&gt;/&lt;name&gt; |
| `--recipes-dir <RECIPES_DIR>` |  |  | The recipe library `--recipe` is resolved in (default the found .brokkr/recipes, else recipes) |
| `--secrets-file <SECRETS_FILE>` |  |  | Operator-side secrets store for seats with declared bindings (default .brokkr/state/secrets.env, or &lt;workdir&gt;/.forge/secrets.env) |
| `--feature <FEATURE>` |  |  | The feature the run delivers, as text: recorded when the run starts and handed to its seats |
| `--realms <REALMS>` |  |  | The world's map: realms and the journal they share. Defaults to .brokkr/realms.json, else ./realms.json; a map named here and missing or malformed is a refusal, never a silent fallback |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, the found folder's .brokkr/state/forge.db, or a library's .forge/forge.db |
| `--repo <REPO>` |  |  | The repository the run operates on: the bundle is compiled against its realm and the engine works in it. Without it, the workspace (the current directory) is compiled against, and the engine gets no repository override, so it works there too |
| `--dispatch <DISPATCH>` |  |  | Canonical forge-dispatch/v2 JSON. When present the run id, Looper/grant correlation, recipe, repository, budget, and producer bounds are pinned into an immutable run-manifest/v2 |
| `--priority <PRIORITY>` | `0` |  | The entry's priority, which orders the queue first: a higher priority is judged and started first, and `move` orders an entry only among those of its own priority |
| `--after <ENTRY:CONDITION>...` |  |  | An earlier entry this one waits for, and on what: `3:completed` (its run completed), `3:ended` (its run ended at all), or a named ending, as `3:shipped` (its run is over and ended so; one that ended otherwise holds this entry). The named endings are shipped, shipped-flagged, refused, failed, blocked, exhausted, superseded, infrastructure, delivered-elsewhere, withdrawn and operator-stopped. Or `3:merged` (the operator recorded its work merged with `brokkr queue merged`; a run over with no such record holds this entry). Repeatable |
| `--reason <REASON>` |  |  | Why, journaled with the command |

## brokkr queue list

The queue in order (priority, higher first, then place): each waiting entry's place, state, priority, waits, launch and admission (admissible, or why it waits or is held), then the entries that started a run, with it. `--json` emits the view model for scripts

```text
Usage: brokkr queue list [OPTIONS]
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default the found folder's .brokkr/realms.json, or a library's ./realms.json, when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, the found folder's .brokkr/state/forge.db, or a library's .forge/forge.db |
| `--json` |  |  | Emit the view model verbatim — this is what scripts read |

## brokkr queue judge

Judge the queue as `list` shows it, and first latch on each waiting entry the realm drift found: from then on it is held until the operator re-pins, re-queues or drops it, whatever the map comes to. Each latch is journaled with the reason

```text
Usage: brokkr queue judge [OPTIONS] --reason <REASON>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--reason <REASON>` |  |  | Why, journaled with each latch |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default the found folder's .brokkr/realms.json, or a library's ./realms.json, when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, the found folder's .brokkr/state/forge.db, or a library's .forge/forge.db |
| `--json` |  |  | Emit the view model verbatim — this is what scripts read |

## brokkr queue move

Put an entry at another place in the queue, among the entries of its own priority: priority always wins, so a place above a higher priority or below a lower one leaves it first or last among its own. Says the place it holds after

```text
Usage: brokkr queue move [OPTIONS] --reason <REASON> --to <TO> <ENTRY>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `<ENTRY>` |  |  | The entry's id, as `brokkr queue list` prints it |
| `--reason <REASON>` |  |  | Why, journaled with the command |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default the found folder's .brokkr/realms.json, or a library's ./realms.json, when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, the found folder's .brokkr/state/forge.db, or a library's .forge/forge.db |
| `--to <TO>` |  |  | The place to put it at, from 1, as `brokkr queue list` shows it |

## brokkr queue hold

Keep an entry in its place, not to be started until released

```text
Usage: brokkr queue hold [OPTIONS] --reason <REASON> <ENTRY>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `<ENTRY>` |  |  | The entry's id, as `brokkr queue list` prints it |
| `--reason <REASON>` |  |  | Why, journaled with the command |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default the found folder's .brokkr/realms.json, or a library's ./realms.json, when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, the found folder's .brokkr/state/forge.db, or a library's .forge/forge.db |

## brokkr queue release

Let a held entry be started again

```text
Usage: brokkr queue release [OPTIONS] --reason <REASON> <ENTRY>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `<ENTRY>` |  |  | The entry's id, as `brokkr queue list` prints it |
| `--reason <REASON>` |  |  | Why, journaled with the command |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default the found folder's .brokkr/realms.json, or a library's ./realms.json, when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, the found folder's .brokkr/state/forge.db, or a library's .forge/forge.db |

## brokkr queue repin

Re-pin a waiting entry to the realms map that would govern it now: how the operator releases an entry `judge` latched a realm-drift hold on, accepting the differences it found. Refused when the map on disk is not the one the latch found; judge the queue again

```text
Usage: brokkr queue repin [OPTIONS] --reason <REASON> <ENTRY>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `<ENTRY>` |  |  | The entry's id, as `brokkr queue list` prints it |
| `--reason <REASON>` |  |  | Why, journaled with the command |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default the found folder's .brokkr/realms.json, or a library's ./realms.json, when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, the found folder's .brokkr/state/forge.db, or a library's .forge/forge.db |

## brokkr queue drop

Take an entry out of the queue. One that started a run cannot be

```text
Usage: brokkr queue drop [OPTIONS] --reason <REASON> <ENTRY>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `<ENTRY>` |  |  | The entry's id, as `brokkr queue list` prints it |
| `--reason <REASON>` |  |  | Why, journaled with the command |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default the found folder's .brokkr/realms.json, or a library's ./realms.json, when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, the found folder's .brokkr/state/forge.db, or a library's .forge/forge.db |

## brokkr queue merged

Record that an entry's work merged, journaled with the reason (name the pull request and its merge commit): an entry waiting on ENTRY:merged may start from then on. Brokkr reads no code host, so this record is the merge's one witness. Only an entry that started a run can be recorded merged

```text
Usage: brokkr queue merged [OPTIONS] --reason <REASON> <ENTRY>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `<ENTRY>` |  |  | The entry's id, as `brokkr queue list` prints it |
| `--reason <REASON>` |  |  | Why, journaled with the command |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default the found folder's .brokkr/realms.json, or a library's ./realms.json, when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, the found folder's .brokkr/state/forge.db, or a library's .forge/forge.db |

## brokkr queue dispatch

Start what admission admits from the journal's queue, each run handed to a driver of its own: one pass with `--once` under the journal's dispatch lock, the bound's command and the holder's readout

```text
Usage: brokkr queue dispatch [OPTIONS]
       brokkr queue dispatch <COMMAND>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--once` |  |  | Run one pass and return without waiting on the drivers it started |
| `--every <SECONDS>` | `60` |  | Without `--once`, the seconds from one pass's start to the next, from 5 to 3600. The dispatcher stands until SIGINT or SIGTERM |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default the found folder's .brokkr/realms.json, or a library's ./realms.json, when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, the found folder's .brokkr/state/forge.db, or a library's .forge/forge.db |

## brokkr queue dispatch set-bound

Set the journal's bound on the refusals counted against one entry before the dispatcher holds it, a whole number of at least 1, journaled as the operator's command

```text
Usage: brokkr queue dispatch set-bound [OPTIONS] --reason <REASON> <BOUND>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `<BOUND>` |  |  | The bound: counted refusals of one entry before it is held |
| `--reason <REASON>` |  |  | Why the bound is set, journaled with it |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default the found folder's .brokkr/realms.json, or a library's ./realms.json, when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, the found folder's .brokkr/state/forge.db, or a library's .forge/forge.db |

## brokkr queue dispatch status

Say whether a dispatcher holds the journal, the bound in force, and each entry left launching or linked with no `spawn` row. Opens the journal read-only and writes nothing

```text
Usage: brokkr queue dispatch status [OPTIONS]
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default the found folder's .brokkr/realms.json, or a library's ./realms.json, when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, the found folder's .brokkr/state/forge.db, or a library's .forge/forge.db |

## brokkr queue sweep

Remove the run-local seat scratch trees the journal proves free and record what each complete removal freed, and reap the hands session trees whose owners are dead. One pass under the journal's dispatch lock; `--dry-run` lists what a pass would remove, with sizes, and writes nothing

```text
Usage: brokkr queue sweep [OPTIONS]
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--dry-run` |  |  | List what a pass would remove, with sizes, and write nothing: no tree-sweep row, no dispatch holder and no migration |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default the found folder's .brokkr/realms.json, or a library's ./realms.json, when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, the found folder's .brokkr/state/forge.db, or a library's .forge/forge.db |

## brokkr stats

What the runs did, read across them: analytics, a recipe's graph, a run's costs and its delivery ledger

```text
Usage: brokkr stats <COMMAND>
```

## brokkr stats analytics

What the runs did across every hearth, for a range and an optional realm: shipped, start to end, waiting on you, rework, cost and served models. `--json` emits the view model

```text
Usage: brokkr stats analytics [OPTIONS]
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--realms <REALMS>` |  |  | The world's map — every hearth it names is read (default the found folder's .brokkr/realms.json, or a library's ./realms.json, when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, the found folder's .brokkr/state/forge.db, or a library's .forge/forge.db |
| `--range <RANGE>` | `7d` |  | How far back from now: 24h, 7d or 30d |
| `--realm <REALM>` |  |  | Only this realm's runs |
| `--json` |  |  | Emit the view model verbatim — this is what the console reads |

## brokkr stats graph

One recipe's state machine and its traffic over a range. `--json` emits the view model

```text
Usage: brokkr stats graph [OPTIONS]
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--recipe <RECIPE>` |  |  | The recipe whose graph is drawn, by name: on the realm's shelf, or a bundle directory in the realm whose bundle.json names it (default the realm's recipe with the most runs in the range, else the first it knows by name) |
| `--realm <REALM>` |  |  | The realm the recipe resolves in and whose runs are counted (default the workspace's realm, else the map's first) |
| `--range <RANGE>` | `7d` |  | How far back from now: 24h, 7d or 30d |
| `--realms <REALMS>` |  |  | The world's map — every hearth it names is read (default the found folder's .brokkr/realms.json, or a library's ./realms.json, when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, the found folder's .brokkr/state/forge.db, or a library's .forge/forge.db |
| `--json` |  |  | Emit the view model verbatim — this is what the console reads |

## brokkr stats costs

Per-seat cost and session accounting from journal checkpoints — the LaneTally join surface (stable seat ids, journal-derived)

```text
Usage: brokkr stats costs [OPTIONS] --run <RUN>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--run <RUN>` |  | yes | The run whose seats are accounted: a full run id, a unique run-id prefix, or `latest` |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default the found folder's .brokkr/realms.json, or a library's ./realms.json, when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, the found folder's .brokkr/state/forge.db, or a library's .forge/forge.db |

## brokkr stats ledger

Render the shipper's delivery ledger from journal and repository evidence

```text
Usage: brokkr stats ledger [OPTIONS] --run <RUN>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--run <RUN>` |  | yes | The run whose ledger is rendered: a full run id, a unique run-id prefix, or `latest` |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default the found folder's .brokkr/realms.json, or a library's ./realms.json, when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, the found folder's .brokkr/state/forge.db, or a library's .forge/forge.db |
| `--repo <REPO>` |  |  | Write `<run>.md` under this repository's `.brokkr/state/ledger/`, or `.forge/ledger/` with no folder; without it, print the ledger |

## brokkr system

This host and its world: the realms map, the secrets store, a harness probe and the read-only web surface

```text
Usage: brokkr system <COMMAND>
```

## brokkr system realms

List the world: each realm with its path, default branch and current HEAD, and the journal the world writes. Read-only, like every other readout

```text
Usage: brokkr system realms [OPTIONS]
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--realms <REALMS>` |  |  | The map to read (default .brokkr/realms.json, else ./realms.json) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, the found folder's .brokkr/state/forge.db, or a library's .forge/forge.db |
| `--json` |  |  | Emit the view model verbatim — this is what scripts read |

## brokkr system secrets

Manage the operator-side secrets store: bundles and journals carry NAMES only; values live in this env-format file outside version control. There is no value-printing verb

```text
Usage: brokkr system secrets <COMMAND>
```

## brokkr system secrets set

Bind NAME to a value read from STDIN (never argv — the CLI obeys its own injection discipline). Creates the store 0600

```text
Usage: brokkr system secrets set [OPTIONS] <NAME>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `<NAME>` |  |  | The name a seat's declared binding asks for |
| `--secrets-file <SECRETS_FILE>` |  |  | The store file, created 0600 when absent (default .brokkr/state/secrets.env, or a library's .forge/secrets.env) |

## brokkr system secrets list

Print bound names, one per line — names, never values

```text
Usage: brokkr system secrets list [OPTIONS]
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--secrets-file <SECRETS_FILE>` |  |  | The store file to read (default .brokkr/state/secrets.env, or a library's .forge/secrets.env) |

## brokkr system secrets remove

Remove NAME from the store

```text
Usage: brokkr system secrets remove [OPTIONS] <NAME>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `<NAME>` |  |  | The bound name to remove |
| `--secrets-file <SECRETS_FILE>` |  |  | The store file to remove it from (default .brokkr/state/secrets.env, or a library's .forge/secrets.env) |

## brokkr system host

Manage the machine's host configuration

```text
Usage: brokkr system host <COMMAND>
```

## brokkr system host upgrade

Upgrade the machine's host configuration to forge.host/v5

```text
Usage: brokkr system host upgrade [OPTIONS]
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--check` |  |  | Print the diff and return a failure status if an upgrade is needed |

## brokkr system probe

Measure an agent CLI for its adapter's facts, spending its bound credentials: host only, never CI

```text
Usage: brokkr system probe <COMMAND>
```

## brokkr system probe harness

Run one agent CLI headless against a scratch repository and HOME, and report each fact its adapter must declare as measured, unmeasured or unsupported, beside the adapter's own fields and the seat eligibility the facts derive

```text
Usage: brokkr system probe harness [OPTIONS] --adapter <ADAPTER>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--adapter <ADAPTER>` |  |  | The adapter whose harness is probed: claude, codex or dsh |
| `--cli <CLI>` |  |  | The CLI to launch (default: the adapter's binary, found on PATH) |
| `--out <OUT>` |  |  | Write the report here. A report already here is read first, and every reading that moved since it is reported as drift |
| `--credential <CREDENTIALS>...` |  |  | A credential every credentialed launch is given, bound by name from the secrets store. Repeatable |
| `--credential-file <CREDENTIAL_FILES>...` |  |  | A file under HOME the CLI keeps its login in, such as .codex/auth.json, linked at the same place under the scratch HOME for every credentialed launch, so a login the CLI refreshes is written to this file. Its content is never read. Repeatable |
| `--secrets-file <SECRETS_FILE>` |  |  | The secrets store each `--credential` is read from (default .brokkr/state/secrets.env, or a library's .forge/secrets.env) |
| `--adapters-dir <ADAPTERS_DIR>` |  |  | The directory holding the adapter files `--adapter` names (default .brokkr/adapters, else adapters) |

## brokkr system ui

Serve the embedded read-only surface on loopback

```text
Usage: brokkr system ui [OPTIONS]
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default the found folder's .brokkr/realms.json, or a library's ./realms.json, when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, the found folder's .brokkr/state/forge.db, or a library's .forge/forge.db |
| `--port <PORT>` | `8383` |  | The loopback port the surface is served on |
| `--open` |  |  | Open the system browser after binding |

## Exit codes

Every code the binary exits with, from `crates/brokkr-cli/src/exit.rs`.

| Code | Name | Meaning |
| --- | --- | --- |
| 0 | completed | The command did what it was asked; a driven run completed. |
| 1 | failed | An error, a refused operator command, an unhealthy `doctor`, an unreadable transcript, or a Muninn reading with nothing usable to record. |
| 1 | running | The run was still running when the command stopped following it. |
| 2 | parked | The run parked and awaits the operator. |
| 2 | usage | The command line did not parse, or omitted the `--cause` its verb requires (clap's own code, shared with `parked`: stderr tells them apart). |
| 3 | stopped | The run stopped. |
| 4 | contended | A peer held the shared journal's write lock. Nothing was written; the same command run again is likely to land. |
| 127 | runner failed | The dsh sandbox runner could not build or start bubblewrap. |
| its own | boxed | `hands exec`: the boxed command's own exit code, passed through. A box a signal ended has no code of its own, and exits 1 (`failed`). |
| 128 + signal | signalled | `hands serve` ended by a termination signal: its session tree is removed, and it exits 128 plus the signal's number, as a shell reports a signal death (143 for SIGTERM). |
