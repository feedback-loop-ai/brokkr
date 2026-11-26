# dsh native web, measured ON and OFF (2026-11-24)

The controller's live measurement of dsh 0.2.1-rc.2's native web tools, for
fetch and search separately. It is the evidence decision 0126 ruling 6's
change declares in dsh's adapter, first reported on
[#1329](https://github.com/feedback-loop-ai/brokkr/issues/1329#issuecomment-7142489320).

**Setup.**
- dsh 0.2.1-rc.2 on a Linux workstation, route
  `deepseek-official/deepseek-flash`.
- Every run used a scratch `DSH_HOME` (headless profile, nothing of the
  operator's), `--json`, and the overlay in
  [no-shell.yml](dsh-native-web-2026-11-24/no-shell.yml), which disables
  dsh's shell tool. The shell is the operator-accepted residual
  (#1329, "accept it as known risk for now"), so it is kept out of this
  measurement and a "no tool" answer cannot come from `curl`.
- "Catalog" is the session's tool list as dsh recorded it.

| Run | Overlay beside no-shell.yml | Task | web_fetch in catalog | web_search in catalog | Tool call | Answer |
|---|---|---|---|---|---|---|
| fetch-on | none | fetch example.com's title | yes | yes | web_fetch (HTTP 200) | Example Domain |
| fetch-off | [web-off.yml](dsh-native-web-2026-11-24/web-off.yml) | the same | no | no | none | NO-WEB-TOOL |
| search-on | none | search "IANA example domain" | yes | yes | web_search | an iana.org URL |
| search-off | [web-off.yml](dsh-native-web-2026-11-24/web-off.yml) | the same | no | no | none | NO-SEARCH-TOOL |
| only-fetch2 | [fetch-only.yml](dsh-native-web-2026-11-24/fetch-only.yml) | fetch | yes | no | web_fetch | Example Domain |
| only-fetch2-search | [fetch-only.yml](dsh-native-web-2026-11-24/fetch-only.yml) | search | yes | no | none | NO-SEARCH-TOOL |
| only-search2 | [search-only.yml](dsh-native-web-2026-11-24/search-only.yml) | search | no | yes | web_search | an iana.org URL |
| only-search2-fetch | [search-only.yml](dsh-native-web-2026-11-24/search-only.yml) | fetch | no | yes | none | NO-WEB-TOOL |

Each run's stream is `<run>.ndjson`, and its session record is
`<run>.session.ndjson` (dsh's JSONL session file, renamed so that the shelf's
typos exclusion covers it). All of them are in
[dsh-native-web-2026-11-24](dsh-native-web-2026-11-24/).

**What it settles.**
- **One control per capability.** The tool-web row's `fetch` and `search`
  config keys each remove one tool from the catalog. That is the per-capability
  control.
- **Backends are not a control.** Catalog presence follows those keys, not the
  backends. With both search provider rows disabled and `search` left on,
  web_search stayed in the catalog. Disabling a backend row is therefore not
  a control.

**What it does not settle.**
- **Children:** whether dsh's `subagent`, `subagent_fork` or `workflow` tools
  (still in the catalog) reach the web through a child.
- **Releases:** any dsh release other than 0.2.1-rc.2.
- **The shell:** the accepted residual.

## Key-only compositions (2026-11-26)

For a dsh seat without a grant, brokkr composes OFF as the tool-web row's two keys and nothing else: no backend row is disabled. The row it writes is the one in
`crates/brokkr-protocol/src/native_controls/dsh.rs`.

Every OFF run above also disabled backend rows, so this shape rested on inference. The review chief of 0126b's round 4 (run
story-0126b-round-4-for-issue-13-62ca24ad) raised that as finding 1.

The controller measured the shape on 2026-11-26. The setup was the same: dsh 0.2.1-rc.2, the scratch `DSH_HOME`, no-shell.yml, and the same two tasks.

| Run | Overlay beside no-shell.yml (backend rows all enabled) | Task | Web tools in catalog | Answer |
|---|---|---|---|---|
| keyoff-fetch | [key-off.yml](dsh-native-web-2026-11-24/key-off.yml) | fetch | none | NO-WEB-TOOL |
| keyoff-search | [key-off.yml](dsh-native-web-2026-11-24/key-off.yml) | search | none | NO-SEARCH-TOOL |
| keyfetch-search | [key-fetch.yml](dsh-native-web-2026-11-24/key-fetch.yml) | search | web_fetch only | NO-SEARCH-TOOL |
| keysearch-fetch | [key-search.yml](dsh-native-web-2026-11-24/key-search.yml) | fetch | web_search only | NO-WEB-TOOL |

**What the four runs show.**
- **No run called a tool.**
- **The catalogs match.** key-off's catalog is the same 22 tools as web-off's. Each one-key overlay's catalog is key-off's plus exactly the tool its key keeps.
- **The keys alone are the control.** With every backend row enabled, `fetch: false` and `search: false` each remove their own tool. So the composed OFF, and each composition that keeps one capability, holds as measured.

**Scrubbing.** The files hold no key, no home path and no host name. Their
only path is the measurement's scratch working directory under
`/var/tmp/cap-gates/`.

**Cost:** on 2026-11-24, ten deepseek-flash calls and two DeepSeek search requests; on 2026-11-26, four deepseek-flash calls and no search request.
