# What works today

This page says what each harness does on `main` today: claude, codex or
dsh, plus `lanetally` (Claude Code through LaneTally's session-capture
wrapper) and `exec` (a deterministic command with no model). It states
what the code does, not what a decision intends. Where an adapter lacks a
capability, its [measured gaps](#measured-gaps) quote the reason its file
records. Where an open issue or pull request owns a fix, the
[known limitations](#known-limitations) link it. A measured gap with no
entry there, such as Codex's tool allow-list or LaneTally's boxed hands,
is the adapter's own measured record and names no owner here. The
[security model](security-model.md) says what Brokkr enforces around a
seat and what it does not.

## What each adapter declares

The adapter files under [`adapters/`](../adapters/) are the engine's own
record of each harness. The table below is those files rendered. A test
(`crates/brokkr-cli/tests/status_pages.rs`) loads them through the
engine's loader and holds this block to its rendering byte for byte, so
a change to a rendered value shows here. A field the loader grows fails
the test's compile until the matrix renders it or names it as not a
capability. A resume shape's `limitations` are dated prose notes, and
they are read in the adapter file, not here.

- **Trust**: only a `trusted` adapter may seat a model at a gate
  (decision 0021 ruling 2). An adapter that declares no tier is
  untrusted.
- **Holds a model gate**: trust and the abstract models the adapter
  names in `judges` (decision 0041 ruling 3). An `exec` seat is not a
  model. It may hold a gate when it declares hands (decision 0043
  ruling 3), and under the `harness` or `open` boundary its script then
  runs unboxed.
- **Egress**: where the adapter's material goes (decision 0036). A seat
  may bind a secret only on a route that meets the bundle's minimum,
  which defaults to `contracted`.
- **Tool allow-list**: the tool names a seat's typed `tools.allow` may
  map onto the harness's own allow-list flag. On claude that flag is
  `--allowedTools`, which pre-approves the tools it names and removes
  none, so it does not bound an unboxed seat. A name that maps to a tool
  of a [native power](#native-powers) is refused at compile, because
  only the realm grants that power (decision 0065 ruling 3). On a
  harness whose native inventory is unmeasured, a typed list is refused
  at compile (the operator's ruling R5 of 2026-09-29). Codex maps no
  tool name; its typed `tools.sandbox` names a sandbox class instead.
- **MCP flag Brokkr passes**: the flag through which Brokkr passes a
  seat MCP servers, not whether the seat reaches MCP servers. Under
  decision 0065 no agent names a server, so the only one Brokkr passes
  is a boxed seat's `workspace`, through its hands fragment. The
  operator's own configuration still
  reaches every Codex seat, boxed or not, through
  `~/.codex/config.toml`, and every unboxed claude seat, through their
  Claude Code configuration. Only the boxed claude fragment passes
  `--strict-mcp-config`, which shuts those out.
- **Boxed hands**: whether the harness can put its hands in Brokkr's
  box (decision 0043). A boxed Codex seat keeps Codex's native shell,
  read-only and outside the box, so it can still read the host,
  credential files included; only its writes go through the box. A
  boxed claude seat has `workspace` and only the tool of a native power
  the realm grants it, but Claude Code runs
  outside the box and loads the operator's user-scope settings,
  `CLAUDE.md` and auto-memory, and runs their hooks, on the host
  ([security model](security-model.md#what-brokkr-does-not-enforce)).
  **Own sandbox for** is the seat classes whose
  harness sandbox stands in for the box under the `harness` boundary
  (decision 0046 ruling 4).
- **Resume shapes**: each named shape's measured status and the version
  it was measured on, the seat classes and boundaries it covers, the
  hands mode in force, and which evidence axes it records. Only
  `supported` rejoins a session. Every other shape starts cold on a
  retry and says so in the record.

<!-- adapter-matrix:start -->
| Harness | Trust | Egress | Holds a model gate | Efforts | Tool allow-list | MCP flag Brokkr passes | Boxed hands | Own sandbox for | Resume shapes |
|---|---|---|---|---|---|---|---|---|---|
| `claude` | trusted | contracted | yes: `fable`, `opus` | low, medium, high, xhigh, max | `cargo`, `codex`, `dsh`, `gh-pr-view`, `gh-run-view`, `git`, `ls`, `mkdir`, `node`, `npm`, `npx`, `rg`, `specify`, `webfetch` (refused: native `web-fetch`), `websearch` (refused: native `web-search`) | `--mcp-config` | yes | — | `boxed-workspace`: unmeasured (2.1.266); classes `work`; boundaries `namespace`, `seatbelt`, `container`; hands `boxed`; evidence `interface` |
| `codex` | trusted | uncontracted | yes: `astra`, `sol` | none, minimal, low, medium, high, xhigh, max | no (measured) | none | yes | gate, work | `work-site`: supported (0.154.0); classes `work`; boundaries `harness`, `not applicable`; hands `none`; evidence `interface`, `restrictions`, `root`, `accounting` |
| `dsh` | untrusted | uncontracted; `spark`: local; `spark-glm`: local | no: untrusted | low, medium, high, xhigh; none on `spark`, `spark-glm` | no | none | no (measured) | — | `headless-work`: unmeasured (0.1.5-rc.1); classes `work`; boundaries `not applicable`; hands `none`; evidence `interface` |
| `exec` | untrusted | contracted | no: untrusted | — | no | none | yes | — | — |
| `lanetally` | untrusted | uncontracted | no: untrusted | low, medium, high, xhigh, max | no: refused while the native inventory is unmeasured | `--mcp-config` | no (measured) | — | `wrapper-work-site`: unmeasured (version unknown); classes `work`; boundaries `harness`, `open`, `not applicable`; hands `none`; evidence — |
<!-- adapter-matrix:end -->

### Measured gaps

What each adapter records about a capability it lacks, quoted from its
file.

<!-- adapter-gaps:start -->
- `claude` resume `boxed-workspace`: main does not perform this rejoin: the 2.1.266 interface is captured and partial controller and September 10 probes observed resumed behavior, but complete rejoin enforcement, the filesystem boundary and flag precedence remain unmeasured on installed 2.1.270, so this new shape stays disabled
- `codex` tool allow-list: codex-cli 0.148.0 restricts by sandbox CLASS, not by tool name: `codex exec -s|--sandbox` takes read-only|workspace-write|danger-full-access and there is no per-tool allow-list flag (no --allowedTools analogue) to map a seat's declared tools onto; --ask-for-approval exists on the interactive `codex` but not on `codex exec`
- `dsh` effort on `spark`: dsh 0.1.5-rc.1 refuses reasoningEffort at every level tried (low, none — runs task-framing-doctor-warns-when-a-088cb62b, task-framing-doctor-warns-when-a-b2b4fdc4, 2026-09-11); seats on this route pin no effort and record not applicable (decision 0035 addendum 2026-09-11)
- `dsh` effort on `spark-glm`: dsh 0.1.5-rc.1 refuses reasoningEffort on spark-glm/GLM-5.3-Flash-EXL3 (low — UNSUPPORTED_REASONING_EFFORT, measured 2026-09-16); the model entry states no reasoningEfforts so the lane materialises as non-reasoning, seats on this route pin no effort and record not applicable (decision 0035 addendum 2026-09-11)
- `dsh` boxed hands: dsh 0.1.0-rc.6 replaces its tool surface only through a profile plugin (presentAs/restrict/guard/register on the scoped tools service); no CLI flag disables its shell and file tools or adds an MCP server, so the one boxed tool cannot be expressed without a plugin. Re-measured on 0.1.2-rc.1 (2026-09-04): the launcher and headless flags are unchanged; the tools plugin reads a DSH_TOOLS_MODE environment value whose vocabulary the release does not document, so it is not relied on. Re-measured on 0.1.5-rc.1 (2026-09-12; launcher 0.1.5-rc.1, installed @deepseek-ai/dsh-tools and @deepseek-ai/dsh-headless 0.1.5-rc.2): the launcher flags (-V/--profile/--patch/--dump-config/--dump-default-config, web/plugin) and the headless flags (-h alone; reasoning streams to stderr) are unchanged, still no CLI flag swaps the tool surface or adds an MCP server. The tools row reads DSH_TOOLS_MODE into tools.mode (dsh-headless/cordis.patch.yml:16, same seam as dsh-web-app/cordis.patch.yml:34) with documented vocabulary native|ptc|both (@deepseek-ai/dsh-tools/README.md:64-74); those are presentation modes for the visible schemas, not a restriction of the underlying shell and file tools, so hands stays unsupported. The plugin is the next slice.
- `dsh` resume `headless-work`: main does not perform this rejoin: the forward-pinned route (@deepseek-ai/dsh 0.1.5-rc.1 plus the repository-owned six-file adaptation of dsh-plugin-cli-session 0.2.0 under extensions/dsh/plugin-cli-session/) is selected but disabled, because exact-root, restriction, multi-message accounting and composite-digest proof remain unmeasured
- `lanetally` boxed hands: the LaneTally session-capture wrapper forwards argv to claude verbatim and positionally — --mcp-config, --tools and --strict-mcp-config all reach the child unchanged (measured 2026-09-19, task 10.8). Forwarding is not confinement: whether the boxed work shape those flags describe actually holds through the wrapper is unmeasured, and the wrapper additionally owns the per-session --settings layer and refuses any settings-source flag in the child argv. Unsupported until confinement itself is measured.
- `lanetally` resume `wrapper-work-site`: main does not perform this rejoin: a wrapper is qualified on its own wrapper, never on what it wraps; claude's captured interface says nothing about what this wrapper forwards, and support by analogy is exactly what this field refuses, so the unknown wrapper identity and forwarding stay unmeasured
<!-- adapter-gaps:end -->

## Native powers

A native power is one the provider or the harness runs itself, such as
Codex's server-side web search or Claude Code's `WebSearch` and
`WebFetch`, so no box confines it. Decision 0065 ruling 4 turns each one
off in every seat the realm does not grant it. A realm grants one in
`realms.json`'s `forge.realms/v6` `capabilities` map, through a tool
dialect under [`dialects/tools/`](../dialects/tools/). A grant with no
`offices` list reaches every office that asks, one with `"offices": []`
reaches none, and one with a list reaches only the offices it names
(`CapabilityGrant::reaches`, `realms.rs`). A recipe or an agent only asks, by name, under
`capabilities`. This repository's own `realms.json` grants nothing.

The same test renders this block from the adapters' `native_capabilities`
and the shipped dialects, through the engine's own loaders. For a power
the adapter knows, a seat that does not hold it is launched with its OFF
composed into the command, boxed or not, and the
[security model](security-model.md#the-final-launch-check) says how the
launch proves it. A seat whose OFF is `unsupported` or `unmeasured` is
refused (decision 0066 ruling 1). An `unmeasured` inventory switches
nothing off and claims no denial: what dsh, LaneTally and an `exec`
command reach on their own, Brokkr does not bound. The notes below say
what each declaration's evidence covers. Only Codex's OFF was measured
live, and only on a cold `codex exec`; Claude's is composition.

<!-- native-powers:start -->
| Harness | Native power | Tools | Off, in every seat the realm does not grant it | On, where the realm grants it | Shipped dialect a realm grants it through | Restriction transport |
|---|---|---|---|---|---|---|
| `claude` | `web-fetch` | `WebFetch` | deny `WebFetch` in `--disallowedTools` | include `WebFetch` in `--tools`, allow `WebFetch` in `--allowedTools` | `claude-native-fetch` | unsupported |
| `claude` | `web-search` | `WebSearch` | deny `WebSearch` in `--disallowedTools` | include `WebSearch` in `--tools`, allow `WebSearch` in `--allowedTools` | `claude-native-search` | unsupported |
| `codex` | `web-search` | `web_search` | `-c web_search="disabled"` | the harness's default, with no flag | `codex-native-search` | unsupported |
| `dsh` | unmeasured | — | nothing is switched off, and no denial is claimed | nothing can be granted | — | — |
| `exec` | unmeasured | — | nothing is switched off, and no denial is claimed | nothing can be granted | — | — |
| `lanetally` | unmeasured | — | nothing is switched off, and no denial is claimed | nothing can be granted | — | — |
<!-- native-powers:end -->

### Notes on native powers

<!-- native-notes:start -->
- `claude` `web-fetch` restrictions: no native transport for a restriction on Claude Code's WebFetch has been established; a WebFetch(domain:…) permission pattern was not measured as a host allowlist
- `claude` `web-fetch` evidence: adapter data and argv composition only; no live denial or enablement of WebFetch has been measured
- `claude` `web-search` restrictions: no native transport for a restriction on Claude Code's WebSearch has been established
- `claude` `web-search` evidence: adapter data and argv composition only; no live denial or enablement of WebSearch has been measured
- `codex` `web-search` restrictions: no native transport for a restriction on codex's server-side search has been established; the measurement covers only the key's "disabled" value
- `codex` `web-search` ON: codex-cli 0.154.0 cold `codex exec` has server-side web search ON with no flag: the default run of the 2026-09-21 controller measurement issued a web_search item and answered with a cited version. No explicit ON value is declared because none was measured.
- `codex` `web-search` evidence: codex-cli 0.154.0, cold `codex exec` only: with `-c web_search="disabled"` the model answered NO SEARCH TOOL; without it the tool ran
- `dsh` native inventory: dsh declares mcp and tool_permissions unsupported, which establishes only that Brokkr cannot narrow or extend its tool surface from the command line; it does not establish that dsh has no native egress of its own. On the contrary, recipes/research-dsh/README.md records that since dsh 0.1.2-rc.1 the headless profile ships web-fetch-http with page fetch on and a keyed search, so native egress is likely present and no OFF control for it has been declared or measured. Its native inventory, and any ON or OFF control for it, remain unmeasured: nothing is granted through dsh and no native denial is claimed (decision 0065 slice one; owed to the controller)
- `exec` native inventory: exec runs whatever command the bundle names; the engine cannot certify what an arbitrary child program reaches on its own, so its native inventory is unmeasured. Its hands, boundary and command authority are decisions 0043 and 0046's, unchanged
- `lanetally` native inventory: the LaneTally wrapper forwards argv to claude, and forwarding is not confinement: whether Claude Code's native WebSearch and WebFetch controls hold through the wrapper, which owns its own per-session settings layer, has not been verified. Claude's declarations and evidence are not inherited; this wrapper's native inventory and controls are unmeasured on their own terms (decision 0065 slice one; owed to the controller)
<!-- native-notes:end -->

The limitations of each declaration's evidence, such as a resumed Codex
session, are dated prose in the adapter file. `brokkr doctor` prints them
beside each power the realm does not grant, and the
[provider adapters guide](guides/provider-adapters.md#native-capabilities)
explains the declaration.

## What each harness does in a run

This table is written by hand from the drivers' code in
`crates/brokkr-protocol/src/adapters.rs`. The test above holds its rows
to the adapter list, one row per adapter, and renders the claude row's
web cell from the adapter and the agent library.

<!-- harness-behaviour:start -->
| Harness | Transcript | Turns and tokens | Cost in USD | Git commits in the seat | Secrets delivered | Web search and fetch |
|---|---|---|---|---|---|---|
| `claude` | The Claude Code session under `~/.claude/projects` | Turns and input, output, cache-read and cache-write tokens per turn; reasoning tokens only in the final result | Yes, as the harness reports it | Unsigned in the box, which sets `commit.gpgsign=false`. Unboxed, the seat's `git` reads the host's own configuration, signing included | Yes, through the environment, where the route meets the egress minimum. Never to a boxed seat, which compilation refuses | Off unless the realm grants it: each of `web-fetch`, `web-search` that a seat does not hold is denied by name in its command ([native powers](#native-powers)), which is composition, not a live measurement. An agent asks for one under `capabilities`, as `researcher` (`web-fetch` wants, `web-search` wants) does. An agent that lists no tools and declares no hands, as `muninn`, `position-robustness`, `position-simplicity`, `triage` do, runs unboxed with no tool list, so Claude Code's other default tools and the operator's MCP servers reach it ([#467](https://github.com/feedback-loop-ai/brokkr/issues/467)) |
| `codex` | The Codex thread under `$CODEX_HOME` or `~/.codex` | Turns and input, cached-input, output and reasoning tokens per turn; no cache-write figure | No: Codex reports none, and the field stays absent | As `claude` | No at the default minimum: the adapter is `uncontracted`, so compilation refuses a binding | Off unless the realm grants `web-search`: a seat that does not hold it is launched with `-c web_search="disabled"`, boxed or not. That switch was measured on codex-cli 0.154.0's cold `codex exec`; on a resumed session it is composed but unmeasured ([native powers](#native-powers)) |
| `dsh` | The dsh session under the seat's own `sessions/brokkr/seat-*` root | Turns and input, output, cache-read and reasoning tokens, folded from the session file the driver tails; whether they arrive per turn on a live seat is unverified | No | The driver sets `commit.gpgsign=false` and the host's identity; commits from a dsh seat fail today | Only on the `local` routes `spark` and `spark-glm` at the default minimum | **On**: dsh's native inventory is unmeasured, so Brokkr switches nothing off, and dsh 0.1.5's base profile turns on `web_search` and `web_fetch` in every seat ([#462](https://github.com/feedback-loop-ai/brokkr/issues/462)) |
| `exec` | None | None; model and effort are recorded as `not applicable` | No | Unsigned: boxed or not, the driver sets `commit.gpgsign=false` | Yes, through the environment; `{{secret:NAME}}` in a command resolves to `$NAME`, never the value. Never to a boxed seat | No model. Its inventory is unmeasured, so nothing is claimed about what its command reaches |
| `lanetally` | As `claude` | As `claude`, and the record carries `capture: lanetally` | Yes, the harness's list price | As `claude` | No at the default minimum: the adapter declares no egress, which reads `uncontracted` | Not switched off: LaneTally's native inventory is unmeasured, so no denial is composed, and whether Claude Code's controls hold through the wrapper is unmeasured |
<!-- harness-behaviour:end -->

## Recipes that do not run today

`night-shift` and `wager-harness-dsh` compile, but are **unavailable**
until [#264](https://github.com/feedback-loop-ai/brokkr/issues/264) is
fixed. Each pins its dsh implement seat to `deepseek/deepseek-flash`.
The dsh adapter maps no `deepseek/` route, and #264 records dsh refusing
that prefix with `NO_ADAPTER` at launch.

## Known limitations

- **Resume is live for one shape.** Only Codex's harness work seat
  rejoins its session on a retry. The claude, dsh and LaneTally shapes
  are unmeasured, so their retries start cold, and `exec` has no session
  ([#226](https://github.com/feedback-loop-ai/brokkr/issues/226)).
- **dsh seats cannot commit**
  ([#282](https://github.com/feedback-loop-ai/brokkr/issues/282)).
- **dsh per-turn telemetry is unverified**
  ([#281](https://github.com/feedback-loop-ai/brokkr/issues/281)).
- **`night-shift` and `wager-harness-dsh` do not run**
  ([#264](https://github.com/feedback-loop-ai/brokkr/issues/264)).
- **dsh seats can search and fetch the web.** dsh 0.1.5's base profile
  turns on `web_search` (DeepSeek's own search) and `web_fetch` in
  every seat. Its adapter declares its native inventory unmeasured, so
  Brokkr switches neither off and no realm grant is asked
  ([#462](https://github.com/feedback-loop-ai/brokkr/issues/462)).
- **Seats inherit the operator's harness configuration.** Brokkr
  switches off only the native powers it names. A claude seat whose
  agent lists no tools and declares no hands, such as the `triage`
  gate, keeps Claude Code's other default tools under the operator's
  own permission settings. An unboxed claude seat starts every MCP
  server in the operator's user-scope configuration, with its
  credentials. A Codex seat, boxed or not, starts the MCP servers in
  `~/.codex/config.toml`
  ([#467](https://github.com/feedback-loop-ai/brokkr/issues/467)).
- **Seats can receive cross-session messages.** On a host that runs
  other interactive Claude Code sessions, a claude seat is reachable by
  their cross-session messages, an input channel outside the journal
  ([#505](https://github.com/feedback-loop-ai/brokkr/issues/505)).
- **A boxed exec attempt can still park `indeterminate`, rarely.** A
  single-shot exec box writes its overlays to RAM, so its init no longer
  outlives the kill syncing an upper layer on disk, the cause of 6 of
  the 7 parks in 40 stressed runs. One run in 40 still parked because its
  driver was not reaped after the kill. Retry it
  ([#504](https://github.com/feedback-loop-ai/brokkr/issues/504)).
- **A dead hands server's scratch tree waits for the next run.** The
  start of every run, resume and rerun removes each `brokkr-hands-*` tree
  under the temporary directory whose recorded owner is dead and whose
  lock no process holds, and names it on stderr. It keeps a locked tree,
  and keeps and names a tree whose lock cannot be probed. Until 0.13.0 it
  also removes a lockless tree left by a server from before the lock,
  even one a live server in another pid namespace still uses
  ([#415](https://github.com/feedback-loop-ai/brokkr/issues/415)).
- **The docs-only preflight tier cannot be reached**
  ([#286](https://github.com/feedback-loop-ai/brokkr/issues/286),
  [#287](https://github.com/feedback-loop-ai/brokkr/issues/287)).
- **macOS has no box of Brokkr's.** `namespace` needs bubblewrap, which
  is Linux-only (0.11 or newer for a seat that binds an overlay, as the
  shipped recipes' cache binds do; `doctor` warns on an older one),
  and `seatbelt` is not built, so a macOS realm runs under
  `harness` and its exec scripts run unboxed
  ([#253](https://github.com/feedback-loop-ai/brokkr/issues/253),
  [#269](https://github.com/feedback-loop-ai/brokkr/issues/269)).
