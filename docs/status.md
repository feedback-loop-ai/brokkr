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
[security model](security-model.md) says what the box does and does not
contain.

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
- **Tool allow-list**: the tool names a seat's `tools.allow` may map
  onto the harness's own allow-list flag.
- **Boxed hands**: whether the harness can put its hands in Brokkr's
  box (decision 0043). **Own sandbox for** is the seat classes whose
  harness sandbox stands in for the box under the `harness` boundary
  (decision 0046 ruling 4).
- **Resume shapes**: each named shape's measured status and the version
  it was measured on, the seat classes and boundaries it covers, the
  hands mode in force, and which evidence axes it records. Only
  `supported` rejoins a session. Every other shape starts cold on a
  retry and says so in the record.

<!-- adapter-matrix:start -->
| Harness | Trust | Egress | Holds a model gate | Efforts | Tool allow-list | MCP servers | Boxed hands | Own sandbox for | Resume shapes |
|---|---|---|---|---|---|---|---|---|---|
| `claude` | trusted | contracted | yes: `fable`, `opus` | low, medium, high, xhigh, max | `cargo`, `codex`, `dsh`, `git`, `ls`, `mkdir`, `rg`, `specify`, `webfetch`, `websearch` | yes | yes | — | `boxed-workspace`: unmeasured (2.1.266); classes `work`; boundaries `namespace`, `seatbelt`, `container`; hands `boxed`; evidence `interface` |
| `codex` | trusted | uncontracted | yes: `astra`, `sol` | none, minimal, low, medium, high, xhigh, max | no (measured) | no | yes | gate, work | `work-site`: supported (0.154.0); classes `work`; boundaries `harness`, `not applicable`; hands `none`; evidence `interface`, `restrictions`, `root`, `accounting` |
| `dsh` | untrusted | uncontracted; `spark`: local; `spark-glm`: local | no: untrusted | low, medium, high, xhigh; none on `spark`, `spark-glm` | no | no | no (measured) | — | `headless-work`: unmeasured (0.1.5-rc.1); classes `work`; boundaries `not applicable`; hands `none`; evidence `interface` |
| `exec` | untrusted | contracted | no: untrusted | — | no | no | yes | — | — |
| `lanetally` | untrusted | uncontracted | no: untrusted | low, medium, high, xhigh, max | `cargo`, `git`, `ls`, `mkdir`, `rg`, `specify` | yes | no (measured) | — | `wrapper-work-site`: unmeasured (version unknown); classes `work`; boundaries `harness`, `open`, `not applicable`; hands `none`; evidence — |
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

## What each harness does in a run

This table is written by hand from the drivers' code in
`crates/brokkr-protocol/src/adapters.rs`. The test above holds its rows
to the adapter list, one row per adapter.

<!-- harness-behaviour:start -->
| Harness | Transcript | Turns and tokens | Cost in USD | Git commits in the seat | Secrets delivered | Web search |
|---|---|---|---|---|---|---|
| `claude` | The Claude Code session under `~/.claude/projects` | Turns and input, output, cache-read and cache-write tokens per turn; reasoning tokens only in the final result | Yes, as the harness reports it | Unsigned in the box, which sets `commit.gpgsign=false`. Unboxed, the seat's `git` reads the host's own configuration, signing included | Yes, through the environment, where the route meets the egress minimum. Never to a boxed seat, which compilation refuses | Only where the seat's tool list grants `websearch` or `webfetch`, which the `researcher` office's does. A boxed seat runs with `--tools ""` |
| `codex` | The Codex thread under `$CODEX_HOME` or `~/.codex` | Turns and input, cached-input, output and reasoning tokens per turn; no cache-write figure | No: Codex reports none, and the field stays absent | As `claude` | No at the default minimum: the adapter is `uncontracted`, so compilation refuses a binding | **On** in every Codex seat, boxed or not. Codex runs it server-side, outside the box |
| `dsh` | The dsh session under the seat's own `sessions/brokkr/seat-*` root | Turns and input, output, cache-read and reasoning tokens, folded from the session file the driver tails; whether they arrive per turn on a live seat is unverified | No | The driver sets `commit.gpgsign=false` and the host's identity; commits from a dsh seat fail today | Only on the `local` routes `spark` and `spark-glm` at the default minimum | **On**: dsh 0.1.5's base profile turns on `web_search` and `web_fetch` in every seat |
| `exec` | None | None; model and effort are recorded as `not applicable` | No | Unsigned: boxed or not, the driver sets `commit.gpgsign=false` | Yes, through the environment; `{{secret:NAME}}` in a command resolves to `$NAME`, never the value. Never to a boxed seat | Not applicable: no model |
| `lanetally` | As `claude` | As `claude`, and the record carries `capture: lanetally` | Yes, the harness's list price | As `claude` | No at the default minimum: the adapter declares no egress, which reads `uncontracted` | As `claude` |
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
  every seat, and no realm grant is asked
  ([#462](https://github.com/feedback-loop-ai/brokkr/issues/462)).
- **Codex seats can search the web.** Codex's server-side `web_search`
  stays on. Its off switch exists only on the unmerged decision 0065
  slice ([#319](https://github.com/feedback-loop-ai/brokkr/pull/319)).
- **A timed-out attempt's detached descendants can outlive the kill**
  ([#403](https://github.com/feedback-loop-ai/brokkr/issues/403)).
- **Dead hands servers leak their scratch trees under `/tmp`**
  ([#415](https://github.com/feedback-loop-ai/brokkr/issues/415)).
- **The docs-only preflight tier cannot be reached**
  ([#286](https://github.com/feedback-loop-ai/brokkr/issues/286),
  [#287](https://github.com/feedback-loop-ai/brokkr/issues/287)).
- **macOS has no box of Brokkr's.** `namespace` needs bubblewrap, which
  is Linux-only, and `seatbelt` is not built, so a macOS realm runs under
  `harness` and its exec scripts run unboxed
  ([#253](https://github.com/feedback-loop-ai/brokkr/issues/253),
  [#269](https://github.com/feedback-loop-ai/brokkr/issues/269)).
