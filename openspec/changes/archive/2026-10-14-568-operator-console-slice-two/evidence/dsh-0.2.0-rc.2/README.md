# Slice-two measurement: DSH plugin APIs for the assistant (D6) and "open the spec" (D7)

- **Date:** 2026-10-14, 01:26–01:36 +03:00. The logs use UTC `Z` timestamps, so they read 2026-10-13T22:26–22:36Z.
- **DSH:** `0.2.0-rc.2`, installed with volta (`log-01-install.txt`). `$DS` below means
  `~/.volta/tools/image/packages/@deepseek-ai/dsh/lib/node_modules/@deepseek-ai/dsh/node_modules/@deepseek-ai`.
- **Node:** v22.23.2. **Browser:** headless snap Firefox, driven by `puppeteer-core@24` installed in `browser/`.
- **Isolation:** the same as in slice one. `tools/env.sh` sets `DSH_HOME`, all four `XDG_*` variables and the npm cache to folders under this one. Everything bound to loopback only: DSH on `127.0.0.1:38218` and the stub model on `127.0.0.1:38219`. `~/.dsh` mtime is unchanged at `2026-10-13 10:25:41`. Every process was killed at the end, and the Firefox profile and `$DSH_HOME/.credentials.yaml` were deleted (`log-08-stop.txt`). Launch tokens in `serve-raw*.txt` are redacted, and no cookie file was kept.
- **Model:** no real provider and no credential was used. `stub-model/` holds a test-only bundle. It declares a pi-ai route `brokkr-stub` (`api: openai-completions`, `baseURL: http://127.0.0.1:38219/v1`, `apiKeyEnv: BROKKR_STUB_KEY` set to the literal `stub-not-a-credential`) and makes it the default model. `stub-model/server.mjs` is a 100-line loopback server that speaks chat-completions SSE, logs what DSH sends, and scripts its replies: call `brokkr_runs`, then call `brokkr_suggest`, then answer in text. Every model-facing fact below was therefore observed on the wire, not inferred.

> **Redaction note.** DSH's resource addresses are written `dsh-resource://<file>/session/…` in this folder's prose and logs, where DSH writes `file` for `<file>`. The repository's private-host check reads a URL's authority as a host, and `file` is a single-label one. The probe's code builds the address as `'dsh-resource:' + '//file/session/'`, which still runs.

## Files

| Path | What |
|---|---|
| `probe/` | The probe plugin. `lib/index.js` is the host half: two model tools and fenced routes `/brokkr-slice2/{report,tools,press,ask}`. `lib/client.js` is the browser half: a main panel, a conversation-header action, and keyed tool views. `package.json` and `cordis.patch.yml` are as in slice one. |
| `stub-model/bundle/`, `stub-model/server.mjs` | Test-only default-model bundle and the loopback model server. |
| `tools/drive.mjs`, `tools/drive-cards.mjs`, `tools/drive-files.mjs`, `tools/env.sh` | Browser drivers and the isolation environment. |
| `log-01-install.txt` | `dsh --version`, then `dsh plugin --profile web add` for both bundles. |
| `log-02-host-routes.txt` | Fence (401 without cookie), global tool registry, and a one-shot model call from the host half. |
| `log-03`…`log-05` | Browser runs 1–3, each covering the whole flow. Run 1 used `sessions.create` only. Runs 2 and 3 added the Workspace route and `submit()`. |
| `log-06-browser-cards.txt` | Unfolding the cards, pressing the card button, the header "open spec" action, and the inline answer. |
| `log-07-browser-files.txt` | Client services through `inject`, and `workspaceFiles` containment (list compared with stat). |
| `probe-host-*.log`, `stub-model-*.log` | Host-half beacons and tool executions, and what the model endpoint received. `probe-host-cards-run.log` is transcribed from the terminal capture, because the next run truncated the on-disk file. Run 3's host and stub logs were overwritten in the same way; run 2 (`probe-host-run2.log`, `stub-model-run2.log`) shows the identical sequence. |
| `shots-run*/` | Screenshots. Each one is referenced below. |
| `worktree-a/`, `worktree-b/` | Fixture "worktrees". DSH was launched with cwd `worktree-a`. `worktree-b` stands in for another worktree. |

## Verdict

| Need | API (exact names) | Observed? |
|---|---|---|
| D6a: model tools from a third-party plugin | Host: `ctx.tools.register(def)` with `inject: ['tools']` | **Yes.** The tools reached the model request, ran, and the model received their results. |
| D6b: start or prefill a chat from a panel | Client: `ctx.workspaces.create({path})` → `ctx.uiWorkspace.openWorkspace(id, beforeOpen)` → `ctx.conversation.input.for(ctx.sessions.scope(sid)).setDraft(text)` and, optionally, `.submit()` | **Yes.** The prefilled composer was seen, and a submitted prompt ran a turn. |
| D6c: inline answer in the panel | Host: `ctx.llm.stream({provider, model, system, messages})` with `ctx.agentDefaultModel.currentSelection()` | **Yes**, against the stub. |
| D6d: suggestion card in the transcript with a button | Client: the keyed slot `tool.call.toolview` with `key: '<tool name>'` | **Yes.** The card rendered, the button pressed a fenced host route, and the card re-rendered from the session log after a reload. |
| D7a: open a file in the right sidebar | Client: `ctx.sidebarRight.openResource('dsh-resource://<file>/session/<sid>/<path>')` | **Yes**, for a path relative to the Session cwd and for an absolute path outside it. |
| D7b: open the Files tab | Client: `ctx.sidebarRight.openTab('files')` | **Yes.** |
| D7c: Files tab on an arbitrary worktree | Not possible directly. The tree is rooted at the **Session's cwd**, so open a Session whose cwd is the worktree. | **Yes**, for the workaround. The root moved with the Session. A directory outside the Session cwd was refused (`workspace-file/outside-workspace`). |

Nothing needed is first-party-only. Every name above is reachable from a hand-written third-party package. The limits are listed at the end.

---

## D6a: model tools (host half)

**API.** `ctx.tools.register(definition) → dispose`, with `export const inject = ['webServer', 'connection', 'tools']`.

- Type: `$DS/dsh-tools/lib/types/index.d.ts:636`. Its doc (`:631`) reads "Register globally or in the calling agent scope. Scoped tools shadow globals." `ToolDefinition` is defined at `:105ff`, `output.presentationMeta` at `:112`, and `ToolResult.meta` lower down.
- Runtime check: `$DS/dsh-tools/lib/index.js:2878–2887`. `register` only requires `output` to be `{ schema, render, presentationMeta? }`, with the schema in the supported JSON-Schema subset. The name `run_code` is reserved.
- `defineTool` (README, "Register a tool") lives in `@deepseek-ai/dsh-tools`. A package linked from outside DSH's tree **cannot import it** (bare-specifier resolution fails), so the probe hand-writes plain objects: `{ name, description, parameters: <JSON Schema object>, output: { schema, render(args, value) → [{type:'text', text}], presentationMeta(args, value) }, isConcurrencySafe?, execute(args, exec) }`. Without `defineTool`, the plugin must validate its own arguments in `execute`, because `register()` does not.
- **Where the tools land.** The probe's row is a profile-root (host-plane) row, so its registrations go into the **global layer**. The web app moves every first-party tool into agent presets (`$DS/dsh-web-app/cordis.patch.yml:430ff`), so before any session existed the global view held only the probe's two tools: `apply: … global view now has 2 schemas; ours: brokkr_runs,brokkr_suggest` (`probe-host-run2.log`, and `GET /brokkr-slice2/tools` in `log-02`). A preset agent inherits the global layer unless something calls `ctx.tools.restrict()` on that agent's ctx. The `standard` preset does not.

**Observed.** The stub logged every model request a Session made (`stub-model-run2.log`):

```
#2 POST /v1/chat/completions … model=stub-model stream=true roles=system,user,user tools(28)=ask_user_question,bash,brokkr_runs,brokkr_suggest,create_goal,edit,…,write
#2 brokkr tool schemas as sent: [{"type":"function","function":{"name":"brokkr_runs","description":"Read-only. List the Brokkr runs …","parameters":{"type":"object","properties":{"realm":{…}},"additionalProperties":false}}}, {… "brokkr_suggest" …}]
#2 reply tool_call brokkr_runs {}
#4 … roles=system,user,user,assistant,tool tools(28)=… → reply tool_call brokkr_suggest {...}
#5 … roles=system,user,user,assistant,tool,assistant,tool → reply text "Stub answer: the run "Fix the login fence" is running …"
```

Host side (`probe-host-run2.log`):

```
tool brokkr_runs execute args={} agent=present callId=call_2
tools/result name=brokkr_runs isError=false agent=present
tool brokkr_suggest execute args={"run":"Fix the login fence","verb":"open-spec","reason":"…"}
tools/result name=brokkr_suggest isError=false agent=present
```

- Under the default "Workspace Write" mode (approval policy `ask`), both tools ran **with no approval prompt**. Approval happens only when a `tools/pre-execute` listener answers `ask`, and nothing did for plugin tools.
- `exec.agent` is present, so a tool can tell which agent and Session called it.
- `ctx.on('tools/change')` fires each time a Session's agent is assembled, and `ctx.on('tools/result')` observes every outcome. Both were observed.
- Only the `standard` preset was observed. The `minimal`, `ptc` and `cordis` presets, and `DSH_TOOLS_MODE=ptc` (where tools reach the model as SDK bindings inside `run_code`), are **read-only facts, not observed**.

## D6b: "Ask about this run" — start, prefill or send a chat (browser half)

**API, observed end to end** (`probe/lib/client.js` `askAboutRun`):

```js
// module.exports.inject = ['slots','sessions','workspaces','uiWorkspace','conversation','sidebarRight', …]
const ws = await ctx.workspaces.create({ path: worktreeDir })            // idempotent per path
await ctx.uiWorkspace.openWorkspace(ws.workspaceId, (sessionId) => {      // beforeOpen: sync, before display
  const scope = ctx.sessions.scope(sessionId)                             // the session-scoped Context
  ctx.conversation.input.for(scope).setDraft(summaryText)                 // prefill
})
// later, to send it as the operator would:  ctx.conversation.input.for(scope).submit()
```

- `ctx.workspaces.create({path})`: `$DS/dsh-api-workspace-controller/lib/types/client/service.d.ts:46`. The service is named `workspaces` (`client/index.d.ts:19`).
- `ctx.uiWorkspace.openWorkspace(workspaceId, beforeOpen?)` and `openSession(target)`: `$DS/dsh-client-ui-workspace/lib/types/client/navigation.d.ts:25` and `:15`. `openWorkspace` reuses the Workspace's blank Session or creates one, and selects it in the main column.
- `ctx.sessions.create({workspaceId?, cwd?, sessionId?})` and `ctx.sessions.scope(id)`: `$DS/dsh-api-session-controller/lib/types/client/contract/sessions.d.ts:80` and `:135`.
- `ctx.conversation.input.for(actx) → SessionInput`, `setDraft(text)` and `submit(mode?)`: `$DS/dsh-client-ui-conversation/lib/types/client/contract/input.d.ts:198`, `:165` and `:176`.

**Observed** (`log-05-browser-run3.txt`, `shots-run3/shot-02-session-prefilled.png`): clicking the panel button "Ask about this run (prefill)" replaced the probe panel with the conversation of a new Session in Workspace `worktree-a`. The composer held the summary:

```
composer text after prefill: "div[ce=true]: About the Brokkr run \"Fix the login fence\" (realm brokkr, standing: running). Spec: openspec/changes/demo/spec.md. What is it waiting on?"
```

The host beacon `ask-prefilled` shows the input state `{"draft":"About the Brokkr run …","draftRev":1,"phase":"plain",…}`. With "Ask (send)", `submit()` started a turn. The prompt reached the stub, the stub called the tools, and the final answer rendered (`shots-run3/shot-08`, `shots-run4/shot-02`). A title model call followed automatically (stub request `#3`/`#7`, "Generate the session title from this JSON array …"). It uses the same default model.

**Things that did not work, all observed:**

- `ctx.sessions.create({cwd})` followed by `openSession(id)`, **with no Workspace for that directory**, opens a blank Session in "Ungrouped" with "Choose workspace", and the **composer is inert** (`contenteditable=false`). The draft was set in state, but nothing showed (`shots-run3/shot-06-session-without-workspace.png`, run 1 `shots-run1/shot-02`). Register the directory as a Workspace first, as above.
- `scope.conversation.send(text)` threw `cannot get property "conversation" without inject` (run 1, `probe-host-run1.log`). The session scope is a child Context that has not injected `conversation`. Use `input.for(scope).submit()`. (`ctx.conversation.send` is documented to need a session-scoped caller: `service.d.ts:39`, `scopedSession` "throw on root contexts".)
- Without these service names in the client module's `inject`, `ctx.get('sessions' | 'workspaces' | 'uiWorkspace' | 'conversation' | 'sidebarRight')` were all `false` at `apply()` time. They became available later, at click time (`probe-host-run1.log`). With them listed in `inject`, all were `true` at `apply()` (`probe-host-files-run.log`). `dsh.client.inject` in `package.json` orders activation only. The Cordis `inject` export is what guarantees the services.

## D6c: an inline answer in the panel (host half calls the model)

**API.** `ctx.get('llm').stream({ provider, model, system, messages: [{ role: 'user', content: [{type:'text', text}] }], maxTokens })` yields `StreamChunk`s (`text-delta`, …, `finish`). Take the provider and model from `ctx.get('agentDefaultModel').currentSelection()`. Types: `$DS/dsh-llm/lib/types/index.d.ts:115`, and `GenerateOptions`/`RequestUserInput` at `$DS/dsh-llm/lib/types/types.d.ts:468–530` (`purpose` accepts only `'compaction' | 'session-title'`). `currentSelection()`: `$DS/dsh-agent-default-model/README.md:50`.

**Observed** (`log-02-host-routes.txt`, `probe-host-cards-run.log`, `shots-run4/shot-04-inline-answer.png`): a `POST /brokkr-slice2/ask` returned `{"answer":"Stub answer: Fix the login fence is running.","chunkTypes":["block-start","text-delta","block-end","usage","finish"],"selection":{"provider":"brokkr-stub","model":"stub-model"}}`, and the panel rendered it. This one-shot call is **not a Session**: nothing is logged in DSH's session store, and the model gets no tools unless `tools` is passed. With the shipped default (`deepseek-official/deepseek-flash`) and no credential, the call would fail. That case was **not observed**, because no real provider was contacted.

## D6d: suggestion cards in the transcript

**API.** Client: `ctx.slots.inject('tool.call.toolview', () => ctx.slots.register({ name: 'tool.call.toolview', key: '<wire tool name>' }, View))`. The slot is keyed and Session-scoped (`$DS/dsh-client-ui-tool/lib/types/client/contract/slots.d.ts:19`): "Any name is allowed, including tools registered by your package". Owner props: `phase` (`'preparing' | 'start' | 'result'`), `block` (`ToolResultNode` carries `call.argsRaw`, `content`, `isError`, `meta`; see `$DS/dsh-client-ui-conversation/lib/types/client/contract/records.d.ts:151–176`), `callId`, `toolName`, `cwd`, `openFile(path, {line})` and `useDisclosure`. `block.meta` is the tool's `output.presentationMeta(args, value)`, persisted on `tool/result`.

**Observed** (`log-06-browser-cards.txt`, `shots-run4/shot-01`, `shot-02`, `probe-host-cards-run.log`):

```
toolview-mounted {"tool":"brokkr_runs","phase":"preparing"…} → "start" → "result","hasMeta":true      (live, run 2)
toolview-mounted {"tool":"brokkr_runs","phase":"result","callId":"call_6","hasMeta":true}           (after a page reload: replayed from the session log)
DOM card brokkr-suggest-card[result] 769x92: Brokkr suggests: open-spec on "Fix the login fence" …  Press open-spec
DOM pressed: 200 {"pressed":"open-spec","run":"Fix the login fence"}
host: press verb=open-spec run="Fix the login fence" from=suggestion-card
```

- The card is a real React component, so its button can call the plugin's own fenced route. The press is a user action. The model cannot press it; it can only call `brokkr_suggest`, which renders the card.
- **Placement:** DSH folds a finished turn's tool calls under "Completed in Ns" → "Called tools". The cards are therefore collapsed by default and the operator unfolds them (`shots-run3/shot-08-transcript-collapsed.png` against `shot-09`). The final assistant text stays visible.
- A card **only exists as the view of a tool call**. Practice note `$DS/dsh-agent-preset/skills/cordis-plugin-development/references/practices.md:21` says not to append session events with a new `type` (the Session would refuse to reopen). The other route, `ctx.uiConversation.events.register()` plus `conversation.chat.node` (`:37`), renders existing event kinds. It was not probed.
- Other places a plugin can put UI in a Session: `conversation.session.header.actions` (list, Session-scoped; `$DS/dsh-client-ui-conversation/lib/types/client/contract/slots.d.ts:155`) was observed with the probe's "Brokkr: open spec" button (`shots-run4/shot-02`). It renders only once the Session is non-blank; a blank hero has no header. `conversation.input.dock` and `conversation.composer.dock` are declared but were not probed.

## D7a: "Open the spec" in the right sidebar

**API.** `ctx.sidebarRight.openResource(address, { params?: { line }, kind?, paneId?, preferNewPane?, revealIfOpened? })`: `$DS/dsh-client-ui-sidebar-right/lib/types/client/service.d.ts:161`. The address grammar comes from `sessionFileAddress` (`$DS/dsh-util-workspace-path/lib/index.js:28`): `dsh-resource://<file>/session/<encodeURIComponent(sid)>/<path, segment-encoded>`. A relative path resolves against the Session cwd. An absolute path keeps its leading `/`, so the address has `//`. The probe restates the five-line encoder, because the package cannot import DSH's helper. The on-screen Session is `ctx.sidebarRight.mounted.getSnapshot()` (`service.d.ts:148`). Inside Session-scoped slots it is the `sessionId` prop.

**Observed:**
- From the conversation-header action, the relative path `openspec/changes/demo/spec.md` produced `open-spec-ok` with address `…/session/session-1a86…/openspec/changes/demo/spec.md`. The right sidebar expanded and rendered the Markdown "Demo change: fix the login fence" (`shots-run4/shot-03-header-open-spec.png`).
- The absolute path `$S/worktree-b/specs/other.md`, **outside the Session cwd**, opened in the same Session and rendered (`shots-run3/shot-04-open-spec-outside-cwd.png`). File reads are not workspace-contained. `stat` on `/etc/hostname` also succeeded (`log-07-browser-files.txt`), which matches `$DS/dsh-api-workspace-files/README.md:12`: "File reads and watches may target paths outside the workspace; directory listing and watches remain workspace-scoped."
- The probe's own panel is a global `main` panel. While it shows, there is **no Session on screen** (`mounted` is `undefined`). The probe guards that case (`open-spec-error … "no Session on screen"`). The sidebar README says `openResource` itself throws without an on-screen Session (`$DS/dsh-client-ui-sidebar-right/README.md`, `ctx.sidebarRight` section). That throw was **read, not observed**: the probe's guard fired first. "Open the spec" from the Brokkr panel therefore has to open a Session first (D6b route), then call `openResource`.

## D7b/c: the Files tab, and pointing it at a worktree

**API.** `ctx.sidebarRight.openTab('files')` (`service.d.ts:168`). The tab type is kind `files` from `dsh-client-ui-sidebar-files`. Its root is `useSessions().byId[sessionId].cwd` (`$DS/dsh-client-ui-sidebar-files/README.md`, "The tree"). It takes no root parameter. Listing goes through `remote.workspaceFiles.list(sessionId, absolutePath)`.

**Observed:**
- With the Session in Workspace `worktree-a`, the Files tab showed `…/dsh-slice2/worktree-a` with `openspec/` and `README.md` (`shots-run3/shot-05-files-tab-worktree-a.png`).
- A Session created with `cwd = worktree-b` showed `…/worktree-b` with `specs/` and `README.md` (`shots-run3/shot-07-files-tab-worktree-b.png`). **The Files tree follows whatever cwd the Session has.** "Point Files at worktree X" therefore means opening a Session in X, ideally through `workspaces.create({path: X})` + `openWorkspace`, so the composer works too.
- The same `workspaceFiles` Remote, called directly by the plugin (`log-07-browser-files.txt`):
  `list($S/worktree-a/openspec)` → `ok` · `list($S/worktree-b)` → `{"code":"workspace-file/outside-workspace"}` · `stat($S/worktree-b/specs/other.md)` → `ok` · `stat(/etc/hostname)` → `ok`.
  To reach the Remote the client module needs **both** `'remote'` and `'remote.workspaceFiles'` in `inject`. With only `remote.workspaceFiles`, `ctx.remote` threw `cannot get property "remote" without inject`. This is the first-party pattern (`$DS/dsh-client-ui-sidebar-files/lib/client.js:939–940`).
- A plugin can register its **own** sidebar tab type: `ctx.sidebarRightTabs.register({ id, kind, patterns?, title, guide? })` plus the keyed slot `sidebar.right.pane.tab`, described in `$DS/dsh-client-ui-sidebar-right/README.md` under "Extension seats". That would allow a "Brokkr worktree" tree served from the plugin's own host route. It was **read, not observed**.

## Limits and what is impossible (0.2.0-rc.2)

1. **No `defineTool` for third-party packages.** Hand-written `ToolDefinition` objects work, and argument validation becomes the plugin's job. Registration is global (host plane), so every preset's agents get the tools. A per-preset tool would need a preset row, which is a profile patch of `@deepseek-ai/dsh-agent-preset` that replaces the whole child list.
2. **Transcript cards only as tool views.** A plugin cannot inject an arbitrary card into the transcript without a tool call, because new session event types break reopening. Cards sit inside the folded "Called tools" group by default.
3. **A real model needs a provider credential.** Everything was observed against a loopback stub. The shipped default route is `deepseek-official`. Brokkr cannot ship a credential, and the operator configures the model in DSH's Models page. Not observed: behaviour with a missing credential (README: `MISSING_CREDENTIAL`), real-provider latency and tool-choice quality.
4. **The Files tree has one root, the Session cwd.** The Files tab cannot be re-rooted. Directory listing outside the Session cwd is refused (`outside-workspace`). Opening a **file** by absolute path works anywhere the Host user can read.
5. **File reads are wide.** Any authenticated browser on the DSH origin can `stat` or `read` any file the Host user can read, through any Session (observed for `/etc/hostname`). This is DSH's behaviour, not something the console adds, but D7 should not treat a Session as a containment boundary.
6. **Sidebar actions need a Session on screen.** From the Brokkr `main` panel, "open the spec" must first open a Session, which is the D6b route. A Session without a Workspace has an inert composer.
7. **Activation services.** Client services (`sessions`, `workspaces`, `uiWorkspace`, `conversation`, `sidebarRight`, `remote` + `remote.workspaceFiles`) must be in the module's Cordis `inject` export. The session scope from `ctx.sessions.scope(id)` has not injected `conversation`, so use `ctx.conversation.input.for(scope)` rather than `scope.conversation`.
8. **Not tested:** HMR re-application (every change here was applied by restarting DSH), the `ptc`, `minimal` and `cordis` presets, mobile or fullscreen sidebar presentations, `conversation.input.dock` and `composer.dock`, and `ctx.uiConversation.events.register`.
