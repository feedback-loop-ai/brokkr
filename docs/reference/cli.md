# CLI reference

<!-- Rendered from the clap definitions by crates/brokkr-cli/src/cli_reference_tests.rs; do not edit by hand. Regenerate with: BROKKR_REGENERATE_CLI_REFERENCE=1 cargo test -p brokkr-cli --lib cli_reference -->

Every `brokkr` verb and argument with its default, and every exit code, as the binary defines them; `brokkr <verb> --help` prints the same text. An argument marked **yes** under Selector takes a full run id, a unique prefix of one, or `latest`, the run created most recently (decision 0015, and for the write paths its proposed 2026-09-28 addendum); one marked **with a journal** does so only when the workspace journal is there, and otherwise takes the id literally and refuses `latest`. Every verb also takes clap's own `-h`/`--help`, and `brokkr` itself `-V`/`--version`; the tables leave them out.

- [`brokkr init`](#brokkr-init): Scaffold a minimal reviewable bundle and prove it compiles
- [`brokkr costs`](#brokkr-costs): Per-seat cost and session accounting from journal checkpoints — the LaneTally join surface (stable seat ids, journal-derived)
- [`brokkr ledger`](#brokkr-ledger): Render the shipper's delivery ledger from journal and repository evidence
- [`brokkr anchor`](#brokkr-anchor): Anchor a run's journal head in refs/forge/&lt;run&gt; (tamper evidence), or verify the existing anchor with --check
- [`brokkr keep-refs`](#brokkr-keep-refs): Keep-refs (decision 0028): the git objects a run's journal cites, held by refs/forge/keep/&lt;run&gt;/&lt;sha&gt; so a squash-merge, a branch delete and a gc cannot collect the evidence the journal names. Runs plant these themselves at conclusion; these verbs cover the runs that concluded before the mechanism existed, and the deliberate letting-go that is the operator's alone
- [`brokkr keep-refs plant`](#brokkr-keep-refs-plant): Plant a keep-ref for every object the run's journal cites. Idempotent: replanting moves nothing
- [`brokkr keep-refs list`](#brokkr-keep-refs-list): Which runs hold which exhibits — one `for-each-ref`, no journal needed. `--run` narrows the listing to one run
- [`brokkr keep-refs delete`](#brokkr-keep-refs-delete): Let one run's exhibits go: remove refs/forge/keep/&lt;run&gt;/*. The operator's decision alone — nothing in the engine ever deletes a keep-ref, and the objects are then as mortal as gc leaves them
- [`brokkr ui`](#brokkr-ui): Serve the embedded read-only surface on loopback
- [`brokkr tui`](#brokkr-tui): Explore runs interactively in the terminal: the fleet as a navigable table, one run's phase graph, seats and decision trail, and one seat's own checkpoint and session stream. Read-only like every other readout (decision 0014) — it issues no operator commands and writes nothing to the journal
- [`brokkr doctor`](#brokkr-doctor): Verify tools, drivers, the workspace database, and optionally a bundle, without executing any agent
- [`brokkr compile`](#brokkr-compile): Validate a bundle and print its pinned manifest and digest
- [`brokkr run`](#brokkr-run): Start a new run and drive it until it parks or finishes
- [`brokkr resume`](#brokkr-resume): Resume an existing run under its exact pinned bundle
- [`brokkr rerun`](#brokkr-rerun): Re-run a past run's feature as a NEW run under another bundle or recipe, so outcomes can be compared by run id. No stored linkage
- [`brokkr compare`](#brokkr-compare): Compare two runs' aligned outcomes: decision trails, first divergence, phases visited, per-seat costs. Read-only
- [`brokkr recipes`](#brokkr-recipes): The recipe library: bundle directories as named, swappable delivery strategies
- [`brokkr recipes list`](#brokkr-recipes-list): List recipes under --dir plus the built-in bundles; broken ones print a warning line, never abort the listing
- [`brokkr recipes add`](#brokkr-recipes-add): Install a recipe from a local path or a git URL into &lt;dir&gt;/&lt;name&gt;
- [`brokkr recipes show`](#brokkr-recipes-show): Print one recipe's RESOLVED bundle and, when it extends another, the composition chain it was resolved from (decision 0017)
- [`brokkr agents`](#brokkr-agents): The agent library (decision 0016): one definition per agent — description, charter, an ordered chain of abstract model names, abstract tool/MCP configuration — that seats reference by name
- [`brokkr agents list`](#brokkr-agents-list): One line per agent — name, model chain, description. A broken definition prints a warning line and never aborts the listing
- [`brokkr agents show`](#brokkr-agents-show): The definition as written, plus the per-chain-entry resolution the compiler would compute. An unknown name errors naming the known set
- [`brokkr muninn`](#brokkr-muninn): The standing overseer (decision 0020): read the fleet, propose to the operator, execute nothing. It opens the journal read-only, issues no operator command, starts no run, and records every proposal — with the run ids and sequence numbers it was derived from — in its own append-only file beside the journal
- [`brokkr muninn run`](#brokkr-muninn-run): Derive the fleet dossier, ask one bounded seat for proposals, and record them. Nothing is executed: a proposal becomes an action only when the operator issues the command themselves
- [`brokkr muninn list`](#brokkr-muninn-list): Read the record back: every proposal, with the run ids and sequence numbers it cited
- [`brokkr secrets`](#brokkr-secrets): Manage the operator-side secrets store (decision 0012): bundles and journals carry NAMES only; values live in this env-format file outside version control. There is no value-printing verb
- [`brokkr secrets set`](#brokkr-secrets-set): Bind NAME to a value read from STDIN (never argv — the CLI obeys its own injection discipline). Creates the store 0600
- [`brokkr secrets list`](#brokkr-secrets-list): Print bound names, one per line — names, never values
- [`brokkr secrets remove`](#brokkr-secrets-remove): Remove NAME from the store
- [`brokkr conclude`](#brokkr-conclude): Close a stopped or parked run from its journal alone — no bundle, no recipe, no effect. `resume` compiles the exact pinned recipe and refuses on any drift, which is right for the branches that spend money but leaves a run from a moved engine with no lawful ending. This appends the operator stop conclusion and nothing else, so it needs no pinned recipe to be honest about what it wrote. It cannot retry: that re-enters the policy loop, and the policy loop needs the bundle by construction. For a run believed dead: every write is fenced, so a journal that moves beneath the conclusion — something still driving the run — refuses instead of being closed over (decision 0029). Check `brokkr runs` first
- [`brokkr operator`](#brokkr-operator): Record an operator command (retry \| stop \| supersede) as journal events
- [`brokkr queue`](#brokkr-queue): The dispatcher's queue (decision 0068): runs waiting to start, in the journal's own database. Add, list, judge, move, hold, release, re-pin and drop entries; each change is journaled with its reason
- [`brokkr queue add`](#brokkr-queue-add): Queue a new run, taking `brokkr run`'s arguments, at the end of the queue. Nothing starts: the entry waits for the dispatcher
- [`brokkr queue list`](#brokkr-queue-list): The queue in order: each waiting entry's place, state, priority, waits, launch and admission (admissible, or why it waits or is held), then the entries that started a run, with it. `--json` emits the view model for scripts
- [`brokkr queue judge`](#brokkr-queue-judge): Judge the queue as `list` shows it, and first latch on each waiting entry the realm drift found: from then on it is held until the operator re-pins, re-queues or drops it, whatever the map comes to. Each latch is journaled with the reason
- [`brokkr queue move`](#brokkr-queue-move): Put an entry at another place in the queue
- [`brokkr queue hold`](#brokkr-queue-hold): Keep an entry in its place, not to be started until released
- [`brokkr queue release`](#brokkr-queue-release): Let a held entry be started again
- [`brokkr queue repin`](#brokkr-queue-repin): Re-pin a waiting entry to the realms map that would govern it now: how the operator releases an entry `judge` latched a realm-drift hold on, accepting the differences it found. Refused when the map on disk is not the one the latch found; judge the queue again
- [`brokkr queue drop`](#brokkr-queue-drop): Take an entry out of the queue. One that started a run cannot be
- [`brokkr inspect`](#brokkr-inspect): Explain a run: header, ruling, seats, decision trail, and the phase graph as a tree. `--phase` and `--seat` are the scoping verbs the console's clicks became; `--json` emits the view model
- [`brokkr transcript`](#brokkr-transcript): Read one participant's retained local transcript — Claude, Codex or DSH — through the same bounded local derivation the TUI uses. The verb never launches, retries or resumes a provider and writes nothing to the journal
- [`brokkr seats`](#brokkr-seats): The seats of a run: the seats block `inspect` renders — every seat's model with the boundary its hands stood behind beside it (decision 0046 ruling 3) — from the same view. `--json` prints that view verbatim, the bytes `inspect --json` prints
- [`brokkr watch`](#brokkr-watch): Watch a run live: redraw the graph, seats, last ruling and seat activity whenever the journal head moves; exit when the run reaches a terminal status. Read-only, like every other readout
- [`brokkr replay`](#brokkr-replay): Rebuild state from the journal twice and verify determinism
- [`brokkr export`](#brokkr-export): Write the canonical NDJSON journal and pinned manifest
- [`brokkr import`](#brokkr-import): Adopt an exported run into this journal, byte-identically — the verb paired with `export`. Journals never merge; one run relocates. Nothing lands unless the whole chain verifies, the events fold, the run_id does not already exist here, and the export is not a redacted derivative
- [`brokkr verify-run`](#brokkr-verify-run): Verify an exported journal offline: chain, envelopes, fold
- [`brokkr bridge`](#brokkr-bridge): Synchronize a Looper-bound run over the authenticated producer API. The API key is read from an environment variable and is never stored
- [`brokkr runs`](#brokkr-runs): List runs in the workspace database: one clamped line per run, newest first. `--json` emits the view model for scripts
- [`brokkr realms`](#brokkr-realms): List the world (decision 0023): each realm with its path, default branch and current HEAD, and the journal the world writes. Read-only, like every other readout
- [`brokkr driver`](#brokkr-driver): Run a built-in forge-driver/v1 adapter (claude \| lanetally \| codex \| dsh \| exec). Bundles reference these as {brokkr} driver &lt;kind&gt; -- &lt;extra args&gt;
- [`brokkr hands`](#brokkr-hands): The model's hands are one tool, and the tool runs in an empty root (decision 0043): serve the `workspace` tool over MCP on stdio, or run one command whole inside the same box
- [`brokkr hands serve`](#brokkr-hands-serve): Serve the one `workspace` tool over MCP (newline-delimited JSON-RPC on stdio); every call runs `bash -lc <command>` inside the box
- [`brokkr hands exec`](#brokkr-hands-exec): Run one command whole inside the box with stdio passed through — how a deterministic `exec` seat holds a gate. Exits with the command's own code
- [`brokkr probe`](#brokkr-probe): Measure an agent CLI for its adapter's facts, spending its bound credentials: host only, never CI
- [`brokkr probe harness`](#brokkr-probe-harness): Run one agent CLI headless against a scratch repository and HOME, and report each fact its adapter must declare as measured, unmeasured or unsupported, beside the adapter's own fields and the seat eligibility the facts derive
- [Exit codes](#exit-codes)

## brokkr init

Scaffold a minimal reviewable bundle and prove it compiles

```text
Usage: brokkr init <DIR>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `<DIR>` |  |  | The directory the bundle is written into; one that already holds a bundle.json or a realms.json is refused, never overwritten |

## brokkr costs

Per-seat cost and session accounting from journal checkpoints — the LaneTally join surface (stable seat ids, journal-derived)

```text
Usage: brokkr costs [OPTIONS] --run <RUN>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--run <RUN>` |  | yes | The run whose seats are accounted: a full run id, a unique run-id prefix, or `latest` |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default ./realms.json when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, .forge/forge.db as always |

## brokkr ledger

Render the shipper's delivery ledger from journal and repository evidence

```text
Usage: brokkr ledger [OPTIONS] --run <RUN>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--run <RUN>` |  | yes | The run whose ledger is rendered: a full run id, a unique run-id prefix, or `latest` |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default ./realms.json when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, .forge/forge.db as always |
| `--repo <REPO>` |  |  | Write `.forge/ledger/<run>.md` here; without it, print the ledger |

## brokkr anchor

Anchor a run's journal head in refs/forge/&lt;run&gt; (tamper evidence), or verify the existing anchor with --check

```text
Usage: brokkr anchor [OPTIONS] --run <RUN>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--run <RUN>` |  | yes | Full run id, a unique run-id prefix, or `latest` |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default ./realms.json when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, .forge/forge.db as always |
| `--repo <REPO>` | `.` |  | The git repository whose refs/forge/&lt;run&gt; holds the anchor |
| `--check` |  |  | Verify instead of writing a new anchor |

## brokkr keep-refs

Keep-refs (decision 0028): the git objects a run's journal cites, held by refs/forge/keep/&lt;run&gt;/&lt;sha&gt; so a squash-merge, a branch delete and a gc cannot collect the evidence the journal names. Runs plant these themselves at conclusion; these verbs cover the runs that concluded before the mechanism existed, and the deliberate letting-go that is the operator's alone

```text
Usage: brokkr keep-refs <COMMAND>
```

## brokkr keep-refs plant

Plant a keep-ref for every object the run's journal cites. Idempotent: replanting moves nothing

```text
Usage: brokkr keep-refs plant [OPTIONS] --run <RUN>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--run <RUN>` |  | yes | Full run id, a unique run-id prefix, or `latest` |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default ./realms.json when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, .forge/forge.db as always |
| `--repo <REPO>` | `.` |  | The git repository the keep-refs are planted in |

## brokkr keep-refs list

Which runs hold which exhibits — one `for-each-ref`, no journal needed. `--run` narrows the listing to one run

```text
Usage: brokkr keep-refs list [OPTIONS]
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--run <RUN>` |  | with a journal | A run id; with the workspace journal there, also a unique prefix or `latest`, and without it the id is taken literally and `latest` is refused. Omitted, every run holding keep-refs in this repository |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default ./realms.json when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, .forge/forge.db as always |
| `--repo <REPO>` | `.` |  | The git repository whose keep-refs are listed |

## brokkr keep-refs delete

Let one run's exhibits go: remove refs/forge/keep/&lt;run&gt;/*. The operator's decision alone — nothing in the engine ever deletes a keep-ref, and the objects are then as mortal as gc leaves them

```text
Usage: brokkr keep-refs delete [OPTIONS] --run <RUN>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--run <RUN>` |  | with a journal | A run id; with the workspace journal there, also a unique prefix or `latest`, and without it the id is taken literally and `latest` is refused |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default ./realms.json when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, .forge/forge.db as always |
| `--repo <REPO>` | `.` |  | The git repository the keep-refs are removed from |

## brokkr ui

Serve the embedded read-only surface on loopback

```text
Usage: brokkr ui [OPTIONS]
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default ./realms.json when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, .forge/forge.db as always |
| `--port <PORT>` | `8383` |  | The loopback port the surface is served on |
| `--open` |  |  | Open the system browser after binding |

## brokkr tui

Explore runs interactively in the terminal: the fleet as a navigable table, one run's phase graph, seats and decision trail, and one seat's own checkpoint and session stream. Read-only like every other readout (decision 0014) — it issues no operator commands and writes nothing to the journal

```text
Usage: brokkr tui [OPTIONS]
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--run <RUN>` |  | yes | Full run id, a unique run-id prefix, or `latest`; opens directly at that run's level |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default ./realms.json when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, .forge/forge.db as always |

## brokkr doctor

Verify tools, drivers, the workspace database, and optionally a bundle, without executing any agent

```text
Usage: brokkr doctor [OPTIONS]
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--bundle <BUNDLE>` |  |  | A bundle directory to compile and check as well; without it, no bundle is checked |
| `--realms <REALMS>` |  |  | The world's map whose realm house declarations doctor checks (default ./realms.json when present) |
| `--db <DB>` |  |  | The workspace journal whose database doctor opens. Outranks the map's journal; without either, .forge/forge.db as always |
| `--secrets-file <SECRETS_FILE>` | `.forge/secrets.env` |  | Operator-side secrets store, so doctor can say which declared credentials a route is taking from the ambient environment instead (decision 0036 ruling 5) |

## brokkr compile

Validate a bundle and print its pinned manifest and digest

```text
Usage: brokkr compile --bundle <BUNDLE>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--bundle <BUNDLE>` |  |  | The bundle directory to validate, compiled against the workspace |

## brokkr run

Start a new run and drive it until it parks or finishes

```text
Usage: brokkr run [OPTIONS] --feature <FEATURE> <--bundle <BUNDLE>|--recipe <RECIPE>>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--bundle <BUNDLE>` |  |  | The bundle directory to deliver under; this or `--recipe` is required. `resume` compiles it to the run's pinned manifest and refuses any drift |
| `--recipe <RECIPE>` |  |  | Named recipe, resolved to &lt;recipes-dir&gt;/&lt;name&gt; |
| `--recipes-dir <RECIPES_DIR>` | `recipes` |  | The recipe library `--recipe` is resolved in |
| `--secrets-file <SECRETS_FILE>` |  |  | Operator-side secrets store for seats with declared bindings (default &lt;workdir&gt;/.forge/secrets.env) |
| `--feature <FEATURE>` |  |  | The feature the run delivers, as text: recorded when the run starts and handed to its seats |
| `--realms <REALMS>` |  |  | The world's map: realms and the journal they share (decision 0023). Defaults to ./realms.json when there is one; a map named here and missing or malformed is a refusal, never a silent fallback |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, .forge/forge.db as always |
| `--repo <REPO>` |  |  | The repository the run operates on: the bundle is compiled against its realm and the engine works in it. Without it, the workspace (the current directory) is compiled against, and the engine gets no repository override, so it works there too |
| `--dispatch <DISPATCH>` |  |  | Canonical forge-dispatch/v2 JSON. When present the run id, Looper/grant correlation, recipe, repository, budget, and producer bounds are pinned into an immutable run-manifest/v2 |
| `--no-view` |  |  | On a terminal, print the plain lines and the summary instead of opening the run view. Off a terminal they always print |

## brokkr resume

Resume an existing run under its exact pinned bundle

```text
Usage: brokkr resume [OPTIONS] --run <RUN> <--bundle <BUNDLE>|--recipe <RECIPE>>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--bundle <BUNDLE>` |  |  | The bundle directory to deliver under; this or `--recipe` is required. `resume` compiles it to the run's pinned manifest and refuses any drift |
| `--recipe <RECIPE>` |  |  | Named recipe, resolved to &lt;recipes-dir&gt;/&lt;name&gt; |
| `--recipes-dir <RECIPES_DIR>` | `recipes` |  | The recipe library `--recipe` is resolved in |
| `--secrets-file <SECRETS_FILE>` |  |  | Operator-side secrets store for seats with declared bindings (default &lt;workdir&gt;/.forge/secrets.env) |
| `--run <RUN>` |  | yes | Full run id, a unique run-id prefix, or `latest` |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default ./realms.json when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, .forge/forge.db as always |
| `--repo <REPO>` |  |  | The repository the resumed run operates on. Without it, the engine gets no repository override and works in the current directory |
| `--no-view` |  |  | On a terminal, print the plain lines and the summary instead of opening the run view. Off a terminal they always print |

## brokkr rerun

Re-run a past run's feature as a NEW run under another bundle or recipe, so outcomes can be compared by run id. No stored linkage

```text
Usage: brokkr rerun [OPTIONS] --run <RUN> <--bundle <BUNDLE>|--recipe <RECIPE>>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--run <RUN>` |  | yes | The source run whose feature is re-run |
| `--bundle <BUNDLE>` |  |  | The bundle directory to deliver under; this or `--recipe` is required. `resume` compiles it to the run's pinned manifest and refuses any drift |
| `--recipe <RECIPE>` |  |  | Named recipe, resolved to &lt;recipes-dir&gt;/&lt;name&gt; |
| `--recipes-dir <RECIPES_DIR>` | `recipes` |  | The recipe library `--recipe` is resolved in |
| `--secrets-file <SECRETS_FILE>` |  |  | Operator-side secrets store for seats with declared bindings (default &lt;workdir&gt;/.forge/secrets.env) |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default ./realms.json when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, .forge/forge.db as always |
| `--repo <REPO>` |  |  | The repository the new run operates on: the bundle is compiled against its realm and the engine works in it. Without it, the workspace (the current directory) is compiled against, and the engine gets no repository override, so it works there too |

## brokkr compare

Compare two runs' aligned outcomes: decision trails, first divergence, phases visited, per-seat costs. Read-only

```text
Usage: brokkr compare [OPTIONS] <RUN_A> <RUN_B>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `<RUN_A>` |  | yes | The first run: a full run id, a unique run-id prefix, or `latest` |
| `<RUN_B>` |  | yes | The second run, named the same ways |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default ./realms.json when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, .forge/forge.db as always |

## brokkr recipes

The recipe library: bundle directories as named, swappable delivery strategies

```text
Usage: brokkr recipes <COMMAND>
```

## brokkr recipes list

List recipes under --dir plus the built-in bundles; broken ones print a warning line, never abort the listing

```text
Usage: brokkr recipes list [OPTIONS]
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--dir <DIR>` | `recipes` |  | The recipe library directory |

## brokkr recipes add

Install a recipe from a local path or a git URL into &lt;dir&gt;/&lt;name&gt;

```text
Usage: brokkr recipes add [OPTIONS] --name <NAME> <SOURCE>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `<SOURCE>` |  |  | A local bundle directory or a git URL |
| `--name <NAME>` |  |  | The name the recipe is installed under |
| `--dir <DIR>` | `recipes` |  | The recipe library directory it is installed into |

## brokkr recipes show

Print one recipe's RESOLVED bundle and, when it extends another, the composition chain it was resolved from (decision 0017)

```text
Usage: brokkr recipes show [OPTIONS] <NAME>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `<NAME>` |  |  | The recipe's name, resolved to &lt;dir&gt;/&lt;name&gt; |
| `--dir <DIR>` | `recipes` |  | The recipe library directory |

## brokkr agents

The agent library (decision 0016): one definition per agent — description, charter, an ordered chain of abstract model names, abstract tool/MCP configuration — that seats reference by name

```text
Usage: brokkr agents <COMMAND>
```

## brokkr agents list

One line per agent — name, model chain, description. A broken definition prints a warning line and never aborts the listing

```text
Usage: brokkr agents list [OPTIONS]
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--agents-dir <AGENTS_DIR>` | `agents` |  | The agent library directory |

## brokkr agents show

The definition as written, plus the per-chain-entry resolution the compiler would compute. An unknown name errors naming the known set

```text
Usage: brokkr agents show [OPTIONS] <NAME>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `<NAME>` |  |  | The agent's name in the library |
| `--agents-dir <AGENTS_DIR>` | `agents` |  | The agent library directory |
| `--adapters-dir <ADAPTERS_DIR>` | `adapters` |  | The adapter library the chain's models are resolved against |

## brokkr muninn

The standing overseer (decision 0020): read the fleet, propose to the operator, execute nothing. It opens the journal read-only, issues no operator command, starts no run, and records every proposal — with the run ids and sequence numbers it was derived from — in its own append-only file beside the journal

```text
Usage: brokkr muninn <COMMAND>
```

## brokkr muninn run

Derive the fleet dossier, ask one bounded seat for proposals, and record them. Nothing is executed: a proposal becomes an action only when the operator issues the command themselves

```text
Usage: brokkr muninn run [OPTIONS]
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the fleet this reading covers (default ./realms.json when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, .forge/forge.db as always |
| `--agents-dir <AGENTS_DIR>` | `agents` |  | The agent library Muninn's own seat is hired from |
| `--adapters-dir <ADAPTERS_DIR>` | `adapters` |  | The adapter library that seat's model is resolved against |
| `--record <RECORD>` | `.forge/muninn.ndjson` |  | The append-only file each proposal is recorded in |

## brokkr muninn list

Read the record back: every proposal, with the run ids and sequence numbers it cited

```text
Usage: brokkr muninn list [OPTIONS]
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--record <RECORD>` | `.forge/muninn.ndjson` |  | The record file to read |
| `--json` |  |  | Emit the recorded entries verbatim — this is what scripts read |

## brokkr secrets

Manage the operator-side secrets store (decision 0012): bundles and journals carry NAMES only; values live in this env-format file outside version control. There is no value-printing verb

```text
Usage: brokkr secrets <COMMAND>
```

## brokkr secrets set

Bind NAME to a value read from STDIN (never argv — the CLI obeys its own injection discipline). Creates the store 0600

```text
Usage: brokkr secrets set [OPTIONS] <NAME>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `<NAME>` |  |  | The name a seat's declared binding asks for |
| `--secrets-file <SECRETS_FILE>` | `.forge/secrets.env` |  | The store file, created 0600 when absent |

## brokkr secrets list

Print bound names, one per line — names, never values

```text
Usage: brokkr secrets list [OPTIONS]
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--secrets-file <SECRETS_FILE>` | `.forge/secrets.env` |  | The store file to read |

## brokkr secrets remove

Remove NAME from the store

```text
Usage: brokkr secrets remove [OPTIONS] <NAME>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `<NAME>` |  |  | The bound name to remove |
| `--secrets-file <SECRETS_FILE>` | `.forge/secrets.env` |  | The store file to remove it from |

## brokkr conclude

Close a stopped or parked run from its journal alone — no bundle, no recipe, no effect. `resume` compiles the exact pinned recipe and refuses on any drift, which is right for the branches that spend money but leaves a run from a moved engine with no lawful ending. This appends the operator stop conclusion and nothing else, so it needs no pinned recipe to be honest about what it wrote. It cannot retry: that re-enters the policy loop, and the policy loop needs the bundle by construction. For a run believed dead: every write is fenced, so a journal that moves beneath the conclusion — something still driving the run — refuses instead of being closed over (decision 0029). Check `brokkr runs` first

```text
Usage: brokkr conclude [OPTIONS] --run <RUN> --reason <REASON>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--run <RUN>` |  | yes | Full run id, a unique run-id prefix, or `latest` |
| `--reason <REASON>` |  |  | Why the run is closed, recorded with the stop conclusion |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default ./realms.json when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, .forge/forge.db as always |

## brokkr operator

Record an operator command (retry | stop | supersede) as journal events

```text
Usage: brokkr operator [OPTIONS] --run <RUN> --reason <REASON> <COMMAND>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--run <RUN>` |  | yes | Full run id, a unique run-id prefix, or `latest` |
| `<COMMAND>` |  |  | "retry" re-runs the current phase; "stop" ends the run; "supersede" records that residual findings on a run that has already finished are closed by another run (decision 0047) |
| `--reason <REASON>` |  |  | Why the operator issued the command, recorded with it |
| `--findings <FINDINGS>...` |  |  | supersede only: the residual findings this closes, by the sequence number of the ruling each was read from. Repeatable, or one comma-separated list |
| `--by-run <BY_RUN>` |  |  | supersede only: the run that closed them |
| `--by-seq <BY_SEQ>` |  |  | supersede only: the `transition/decided` in that run which closed them |
| `--by-realm <BY_REALM>` |  |  | supersede only: the realm that run was read in. Omitted for the workspace journal, which is every one-hearth world |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default ./realms.json when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, .forge/forge.db as always |

## brokkr queue

The dispatcher's queue (decision 0068): runs waiting to start, in the journal's own database. Add, list, judge, move, hold, release, re-pin and drop entries; each change is journaled with its reason

```text
Usage: brokkr queue <COMMAND>
```

## brokkr queue add

Queue a new run, taking `brokkr run`'s arguments, at the end of the queue. Nothing starts: the entry waits for the dispatcher

```text
Usage: brokkr queue add [OPTIONS] --feature <FEATURE> --reason <REASON> <--bundle <BUNDLE>|--recipe <RECIPE>>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--bundle <BUNDLE>` |  |  | The bundle directory to deliver under; this or `--recipe` is required. `resume` compiles it to the run's pinned manifest and refuses any drift |
| `--recipe <RECIPE>` |  |  | Named recipe, resolved to &lt;recipes-dir&gt;/&lt;name&gt; |
| `--recipes-dir <RECIPES_DIR>` | `recipes` |  | The recipe library `--recipe` is resolved in |
| `--secrets-file <SECRETS_FILE>` |  |  | Operator-side secrets store for seats with declared bindings (default &lt;workdir&gt;/.forge/secrets.env) |
| `--feature <FEATURE>` |  |  | The feature the run delivers, as text: recorded when the run starts and handed to its seats |
| `--realms <REALMS>` |  |  | The world's map: realms and the journal they share (decision 0023). Defaults to ./realms.json when there is one; a map named here and missing or malformed is a refusal, never a silent fallback |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, .forge/forge.db as always |
| `--repo <REPO>` |  |  | The repository the run operates on: the bundle is compiled against its realm and the engine works in it. Without it, the workspace (the current directory) is compiled against, and the engine gets no repository override, so it works there too |
| `--dispatch <DISPATCH>` |  |  | Canonical forge-dispatch/v2 JSON. When present the run id, Looper/grant correlation, recipe, repository, budget, and producer bounds are pinned into an immutable run-manifest/v2 |
| `--priority <PRIORITY>` | `0` |  | The entry's priority: operator data, weighed at admission |
| `--after <ENTRY:CONDITION>...` |  |  | An earlier entry this one waits for, and on what: `3:completed` (its run completed) or `3:ended` (its run ended at all). Repeatable |
| `--reason <REASON>` |  |  | Why, journaled with the command |

## brokkr queue list

The queue in order: each waiting entry's place, state, priority, waits, launch and admission (admissible, or why it waits or is held), then the entries that started a run, with it. `--json` emits the view model for scripts

```text
Usage: brokkr queue list [OPTIONS]
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default ./realms.json when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, .forge/forge.db as always |
| `--json` |  |  | Emit the view model verbatim — this is what scripts read |

## brokkr queue judge

Judge the queue as `list` shows it, and first latch on each waiting entry the realm drift found: from then on it is held until the operator re-pins, re-queues or drops it, whatever the map comes to. Each latch is journaled with the reason

```text
Usage: brokkr queue judge [OPTIONS] --reason <REASON>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--reason <REASON>` |  |  | Why, journaled with each latch |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default ./realms.json when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, .forge/forge.db as always |
| `--json` |  |  | Emit the view model verbatim — this is what scripts read |

## brokkr queue move

Put an entry at another place in the queue

```text
Usage: brokkr queue move [OPTIONS] --reason <REASON> --to <TO> <ENTRY>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `<ENTRY>` |  |  | The entry's id, as `brokkr queue list` prints it |
| `--reason <REASON>` |  |  | Why, journaled with the command |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default ./realms.json when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, .forge/forge.db as always |
| `--to <TO>` |  |  | The place to put it at, from 1 |

## brokkr queue hold

Keep an entry in its place, not to be started until released

```text
Usage: brokkr queue hold [OPTIONS] --reason <REASON> <ENTRY>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `<ENTRY>` |  |  | The entry's id, as `brokkr queue list` prints it |
| `--reason <REASON>` |  |  | Why, journaled with the command |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default ./realms.json when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, .forge/forge.db as always |

## brokkr queue release

Let a held entry be started again

```text
Usage: brokkr queue release [OPTIONS] --reason <REASON> <ENTRY>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `<ENTRY>` |  |  | The entry's id, as `brokkr queue list` prints it |
| `--reason <REASON>` |  |  | Why, journaled with the command |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default ./realms.json when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, .forge/forge.db as always |

## brokkr queue repin

Re-pin a waiting entry to the realms map that would govern it now: how the operator releases an entry `judge` latched a realm-drift hold on, accepting the differences it found. Refused when the map on disk is not the one the latch found; judge the queue again

```text
Usage: brokkr queue repin [OPTIONS] --reason <REASON> <ENTRY>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `<ENTRY>` |  |  | The entry's id, as `brokkr queue list` prints it |
| `--reason <REASON>` |  |  | Why, journaled with the command |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default ./realms.json when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, .forge/forge.db as always |

## brokkr queue drop

Take an entry out of the queue. One that started a run cannot be

```text
Usage: brokkr queue drop [OPTIONS] --reason <REASON> <ENTRY>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `<ENTRY>` |  |  | The entry's id, as `brokkr queue list` prints it |
| `--reason <REASON>` |  |  | Why, journaled with the command |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default ./realms.json when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, .forge/forge.db as always |

## brokkr inspect

Explain a run: header, ruling, seats, decision trail, and the phase graph as a tree. `--phase` and `--seat` are the scoping verbs the console's clicks became; `--json` emits the view model

```text
Usage: brokkr inspect [OPTIONS] --run <RUN>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--run <RUN>` |  | yes | Full run id, a unique run-id prefix, or `latest` |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default ./realms.json when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, .forge/forge.db as always |
| `--json` |  |  | Emit the view model verbatim — this is what scripts read |
| `--phase <PHASE>` |  |  | Scope the readout to one phase |
| `--seat <SEAT>` |  |  | Scope the readout to one seat, by label or participant key |

## brokkr transcript

Read one participant's retained local transcript — Claude, Codex or DSH — through the same bounded local derivation the TUI uses. The verb never launches, retries or resumes a provider and writes nothing to the journal

```text
Usage: brokkr transcript [OPTIONS] --run <RUN> --seat <SEAT>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--run <RUN>` |  | yes | Full run id, a unique run-id prefix, or `latest` |
| `--seat <SEAT>` |  |  | A participant key, or a label that is unique within the run |
| `--turn <TURN>` |  |  | One-based displayed-turn index; omitted reads the whole transcript |
| `--json` |  |  | Emit the `brokkr.transcript/v1` document |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default ./realms.json when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, .forge/forge.db as always |

## brokkr seats

The seats of a run: the seats block `inspect` renders — every seat's model with the boundary its hands stood behind beside it (decision 0046 ruling 3) — from the same view. `--json` prints that view verbatim, the bytes `inspect --json` prints

```text
Usage: brokkr seats [OPTIONS] --run <RUN>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--run <RUN>` |  | yes | Full run id, a unique run-id prefix, or `latest` |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default ./realms.json when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, .forge/forge.db as always |
| `--json` |  |  | Emit the view model verbatim — `inspect --json`'s own bytes |

## brokkr watch

Watch a run live: redraw the graph, seats, last ruling and seat activity whenever the journal head moves; exit when the run reaches a terminal status. Read-only, like every other readout

```text
Usage: brokkr watch [OPTIONS] --run <RUN>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--run <RUN>` |  | yes | Full run id, a unique run-id prefix, or `latest` |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default ./realms.json when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, .forge/forge.db as always |
| `--once` |  |  | Print one frame and exit |
| `--interval <INTERVAL_MS>` | `750` |  | Poll interval in milliseconds (floored at 100) |
| `--no-view` |  |  | On a terminal, redraw the text frames instead of opening the run view. Off a terminal, and with `--once`, they always print |

## brokkr replay

Rebuild state from the journal twice and verify determinism

```text
Usage: brokkr replay [OPTIONS] --run <RUN>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--run <RUN>` |  | yes | Full run id, a unique run-id prefix, or `latest` |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default ./realms.json when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, .forge/forge.db as always |

## brokkr export

Write the canonical NDJSON journal and pinned manifest

```text
Usage: brokkr export [OPTIONS] --run <RUN>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--run <RUN>` |  | yes | Full run id, a unique run-id prefix, or `latest` |
| `--out <OUT>` | `.` |  | The directory `<run>.ndjson` and `<run>.manifest.json` are written into, created when absent |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default ./realms.json when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, .forge/forge.db as always |
| `--redact` |  |  | Also write a sanitized copy for publishable fixtures — `<run>.redacted.ndjson` and `<run>.redacted.manifest.json` — with every absolute path in event payloads rewritten to a stable placeholder. The verbatim pair is written unchanged; the redacted copy's recorded hashes no longer verify, and its manifest says so |

## brokkr import

Adopt an exported run into this journal, byte-identically — the verb paired with `export`. Journals never merge; one run relocates. Nothing lands unless the whole chain verifies, the events fold, the run_id does not already exist here, and the export is not a redacted derivative

```text
Usage: brokkr import [OPTIONS] --from <FROM>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--from <FROM>` |  |  | The exported `<run>.ndjson`. Its `<run>.manifest.json` sidecar is read from beside it and must be there |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default ./realms.json when present) |
| `--db <DB>` |  |  | The destination journal. Outranks the map's journal; without either, .forge/forge.db as always |

## brokkr verify-run

Verify an exported journal offline: chain, envelopes, fold

```text
Usage: brokkr verify-run <FILE>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `<FILE>` |  |  | The exported `<run>.ndjson` journal to verify |

## brokkr bridge

Synchronize a Looper-bound run over the authenticated producer API. The API key is read from an environment variable and is never stored

```text
Usage: brokkr bridge [OPTIONS] --run <RUN> --looper-url <LOOPER_URL>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--run <RUN>` |  | yes | Full run id, a unique run-id prefix, or `latest` |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default ./realms.json when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, .forge/forge.db as always |
| `--looper-url <LOOPER_URL>` |  |  | The base URL of the Looper producer API |
| `--token-env <TOKEN_ENV>` | `LOOPER_API_KEY` |  | The environment variable the API key is read from; unset or empty is refused |
| `--follow` |  |  | Keep tailing the verified journal and command feed |
| `--interval-ms <INTERVAL_MS>` | `750` |  | With `--follow`, the pause between syncs in milliseconds (floored at 100) |

## brokkr runs

List runs in the workspace database: one clamped line per run, newest first. `--json` emits the view model for scripts

```text
Usage: brokkr runs [OPTIONS]
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--realms <REALMS>` |  |  | The world's map — the journal it names is the one opened (default ./realms.json when present) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, .forge/forge.db as always |
| `--json` |  |  | Emit the view model verbatim — this is what scripts read |

## brokkr realms

List the world (decision 0023): each realm with its path, default branch and current HEAD, and the journal the world writes. Read-only, like every other readout

```text
Usage: brokkr realms [OPTIONS]
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--realms <REALMS>` |  |  | The map to read (default ./realms.json) |
| `--db <DB>` |  |  | The workspace journal. Outranks the map's journal; without either, .forge/forge.db as always |
| `--json` |  |  | Emit the view model verbatim — this is what scripts read |

## brokkr driver

Run a built-in forge-driver/v1 adapter (claude | lanetally | codex | dsh | exec). Bundles reference these as {brokkr} driver &lt;kind&gt; -- &lt;extra args&gt;

```text
Usage: brokkr driver <KIND> [ARGS]...
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `<KIND>` |  |  | The adapter to run: claude, lanetally, codex, dsh or exec |
| `<ARGS>...` |  |  | Arguments after -- pass to the agent CLI (claude/lanetally/codex/dsh) or form the command template (exec) |

## brokkr hands

The model's hands are one tool, and the tool runs in an empty root (decision 0043): serve the `workspace` tool over MCP on stdio, or run one command whole inside the same box

```text
Usage: brokkr hands <COMMAND>
```

## brokkr hands serve

Serve the one `workspace` tool over MCP (newline-delimited JSON-RPC on stdio); every call runs `bash -lc <command>` inside the box

```text
Usage: brokkr hands serve [OPTIONS] --workdir <WORKDIR>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--workdir <WORKDIR>` |  |  | The worktree, bound read-write at its own path |
| `--spec <SPEC>` | `"workspace"` |  | The box spec as JSON: {"kind":"workspace","network":…,"binds":[…]} |

## brokkr hands exec

Run one command whole inside the box with stdio passed through — how a deterministic `exec` seat holds a gate. Exits with the command's own code

```text
Usage: brokkr hands exec [OPTIONS] --workdir <WORKDIR> <COMMAND>...
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--workdir <WORKDIR>` |  |  | The worktree, bound read-write at its own path |
| `--bundle-root <BUNDLE_ROOT>` |  |  | Strategy root, bound read-only at /runtime/bundle |
| `--spec <SPEC>` | `"workspace"` |  | The box spec as JSON, as `serve` takes it |
| `<COMMAND>...` |  |  | The command and its arguments, run inside the box |

## brokkr probe

Measure an agent CLI for its adapter's facts, spending its bound credentials: host only, never CI

```text
Usage: brokkr probe <COMMAND>
```

## brokkr probe harness

Run one agent CLI headless against a scratch repository and HOME, and report each fact its adapter must declare as measured, unmeasured or unsupported, beside the adapter's own fields and the seat eligibility the facts derive

```text
Usage: brokkr probe harness [OPTIONS] --adapter <ADAPTER>
```

| Argument | Default | Selector | Description |
| --- | --- | --- | --- |
| `--adapter <ADAPTER>` |  |  | The adapter whose harness is probed: claude, codex or dsh |
| `--cli <CLI>` |  |  | The CLI to launch (default: the adapter's binary, found on PATH) |
| `--out <OUT>` |  |  | Write the report here. A report already here is read first, and every reading that moved since it is reported as drift |
| `--credential <CREDENTIALS>...` |  |  | A credential every credentialed launch is given, bound by name from the secrets store (decision 0012). Repeatable |
| `--secrets-file <SECRETS_FILE>` | `.forge/secrets.env` |  | The secrets store each `--credential` is read from |
| `--adapters-dir <ADAPTERS_DIR>` | `adapters` |  | The directory holding the adapter files `--adapter` names |

## Exit codes

Every code the binary exits with, from `crates/brokkr-cli/src/exit.rs`.

| Code | Name | Meaning |
| --- | --- | --- |
| 0 | completed | The command did what it was asked; a driven run completed. |
| 1 | failed | An error, a refused operator command, an unhealthy `doctor`, an unreadable transcript, or a Muninn reading with nothing usable to record. |
| 1 | running | The run was still running when the command stopped following it. |
| 2 | parked | The run parked and awaits the operator. |
| 2 | usage | The command line did not parse (clap's own code, shared with `parked`: stderr tells them apart). |
| 3 | stopped | The run stopped. |
| 4 | contended | A peer held the shared journal's write lock. Nothing was written; the same command run again is likely to land. |
| 127 | runner failed | The dsh sandbox runner could not build or start bubblewrap. |
| its own | boxed | `hands exec`: the boxed command's own exit code, passed through. A box a signal ended has no code of its own, and exits 1 (`failed`). |
| 128 + signal | signalled | `hands serve` ended by a termination signal: its session tree is removed, and it exits 128 plus the signal's number, as a shell reports a signal death (143 for SIGTERM). |
