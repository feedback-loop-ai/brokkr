# Task 0.2 measurement: a third-party DSH plugin can register a console panel and web routes

- **Date:** 2026-10-13, 07:51–07:57 +03:00 (the logs below use UTC `Z` and local `+03:00` timestamps)
- **DSH:** `0.2.0-rc.2`, installed through volta (`~/.volta/bin/dsh`). Package root:
  `~/.volta/tools/image/packages/@deepseek-ai/dsh/lib/node_modules/@deepseek-ai/dsh/` (written `$PKG` below; sibling packages are under `$PKG/node_modules/@deepseek-ai/`, written `$DS`).
- **Node:** v22.23.2. **pnpm:** 10.28.1 (volta). **Host:** Linux, loopback only (`127.0.0.1:38117`).
- **Probe:** `probe/` in this folder. The raw logs are `log-*.txt`, `probe-host.log` and `shot-*.png`.

## Verdict

All four points were observed on `0.2.0-rc.2`. A third-party package was installed with `dsh plugin add`, outside DSH's own package tree, into an isolated profile.

| # | Claim | Observed |
|---|---|---|
| a | `dsh --version` | `0.2.0-rc.2` |
| b | The client module registers a `main` panel and a `sidebar.panellist` entry | Yes. The bundle is in the boot graph and served under `/plugins`. In headless Firefox the sidebar row "Brokkr probe" rendered. Clicking it mounted the probe's `main` panel, and the row's icon received `active=true`. The probe's beacons confirm this in the Host log, and screenshots show it. |
| c | A plugin HTTP route is reachable | Yes. `GET /brokkr-probe/ping` → `200` with the probe's JSON, both from curl and from inside the panel. |
| d | A plugin WebSocket upgrade is reachable | Yes. `101 Switching Protocols` with a correct `Sec-WebSocket-Accept`. Node's built-in `WebSocket` got the greeting and `echo:<msg>`. The browser panel also completed the same echo. |

## Isolation

Every dsh invocation ran with:

```sh
S=<this folder>
export DSH_HOME=$S/home XDG_DATA_HOME=$S/xdg/data XDG_CACHE_HOME=$S/xdg/cache \
       XDG_CONFIG_HOME=$S/xdg/config XDG_STATE_HOME=$S/xdg/state BROKKR_PROBE_LOG=$S/probe-host.log
```

`DSH_HOME` is the documented data-root override. The precedence is explicit path, then `$DSH_HOME`, then `~/.dsh` (`$DS/dsh-home-paths/README.md`). Profiles live in `$DSH_HOME/profiles/<name>`, and credentials in `$DSH_HOME/.credentials.yaml` (`$DS/dsh-credentials-local/README.md`). The browser-auth signing secret is stored there as well. The XDG vars redirect pnpm's store, cache and state. Nothing was written to `~/.dsh`: its mtime is still 2026-10-12 23:40. The scratch home ended up holding `.anonymous-user-id`, `.credentials.yaml` (deleted afterwards), `profiles/web/` and `storages/workspace.json`.

Headless Firefox is the snap package, which cannot read `/tmp`. Its throwaway profile therefore lived in `~/snap/firefox/common/brokkr-probe-ffprofile` and was deleted afterwards. The client driver is `puppeteer-core@24`, installed locally in `browser/` with `PUPPETEER_SKIP_DOWNLOAD=1`. It drives the stock Firefox over WebDriver BiDi.

## How a third-party plugin is authored and loaded (0.2.0-rc.2)

### Package layout (the probe)

```
probe/
  package.json        dsh.bundle.patch + dsh.client, exports "./client"
  cordis.patch.yml    inserts one Loader row: { id: brokkr-console-probe, name: brokkr-console-probe }
  lib/index.js        Host (node) half: Cordis plugin { name, inject, apply }
  lib/client.js       browser half: window.__ModuleLoader__.load({ id, factory })
```

### Manifest fields (`package.json`)

- `dsh.bundle.patch: "./cordis.patch.yml"`: this makes the package a **bundle**, a patch layer that the profile's `dsh.profile.bundles` list applies. The type is `DshBundleManifest { patch: string | string[] }` in `$DS/dsh-package-manifest/lib/types/types.d.ts`. The first-party pattern to copy is `$DS/dsh-experimental-schedule-bundle/package.json` together with its `cordis.patch.yml`, which is an `- insert:` list of `{id, name}` rows.
- `dsh.client: { platform: "web", inject?: string[], external?: string[], immediately?: boolean }`: the browser declaration. `inject` holds package names that are only dependency and ordering edges, not Cordis services. The type is `DshClientManifest` in the same `types.d.ts`, and the parser is `parseDshClient` in `$DS/dsh-client-modules/lib/types/client/manifest.d.ts`.
- `exports["./client"] → ./lib/client.js` is required. `dsh-client-modules` serves the package's built `lib/client.js`, and a missing bundle fails activation (`$DS/dsh-client-modules/README.md`, "Build requirements").
- The Loader row `name` must be a **bare package specifier**. Client modules key browser modules by package name (`exactPackageSpecifier` returns `undefined` for paths; see `manifest.d.ts`). So the package must be installed into the profile; it cannot be inserted by file path.
- DSH compatibility gate: any `peerDependencies` on `@deepseek-ai/dsh*` must match the runtime version, or the row is denied. The probe declares none, which imposes no constraint (`$DS/dsh-app-boot/README.md`, "Profiles").

### Install and load

`dsh plugin --profile web add <abs path | npm spec | git>` runs pnpm in `$DSH_HOME/profiles/web`. A local path becomes a `link:` dependency, and the package name is appended to `dsh.profile.bundles` (`$DS/dsh-plugin-manager/README.md`). The installed bundle's patch rows are composed after `@deepseek-ai/dsh-base` and `@deepseek-ai/dsh-web-app`. `dsh web --dump-config` shows them under `# == brokkr-console-probe`.

### Host half: web routes (`ctx.webServer`, from `@deepseek-ai/dsh-host-webserver`)

The types are in `$DS/dsh-host-webserver/lib/types/index.d.ts`, and the README is next to them.

- `ctx.webServer.register({ kind: 'exact' | 'prefix', path, handler(req, res) }) → dispose`
- `ctx.webServer.registerUpgrade({ path, handler(req, socket: Duplex, head: Buffer) }) → dispose`. The handler owns the whole handshake. DSH does no WebSocket framing for you. The probe hand-rolls RFC 6455 with no `ws` dependency.
- Other members: `registerFallback` (already taken by the SPA server), `tapIndex`, and `webserver/index-inject` rows. `ctx.webServer.host` and `ctx.webServer.port` are getters.
- Wrap registrations in `ctx.effect(() => ctx.webServer.register(...), label)`, the way first-party plugins do (`$DS/dsh-host-open-in-app/lib/index.js` ~L1271).
- The plugin shape is `export const name`, `export const inject = ['webServer', 'connection']` and `export function apply(ctx)`.

### Browser half: slots (`ctx.slots`, from `@deepseek-ai/dsh-client-ui-slots` and `-renderer`)

`lib/client.js` must call `window.__ModuleLoader__.load({ id: '<package name>', factory: (require) => module.exports })`. The exports are `{ apply, inject: ['slots'], name }`. `require('react')` and `require('react/jsx-runtime')` resolve from the shell's frozen platform table. The probe matches what first-party `dsh-client-ui-schedule` does (`$DS/dsh-client-ui-schedule/lib/client.js` L6793–6810):

```js
ctx.slots.inject('main', () => ctx.slots.register({ name: 'main', key: PANEL_ID }, Page))
ctx.slots.inject('sidebar.panellist', () => ctx.slots.register(
  { name: 'sidebar.panellist', id: PANEL_ID, order: 50, label: () => 'Brokkr probe' }, Icon))
```

- `main` is a **keyed** root slot: "Central panel selected by sidebar entry id" (`$DS/dsh-client-ui-layout/lib/types/client/index.d.ts`).
- `sidebar.panellist` is a **list** root slot: "Each list id addresses the matching main panel". The icon component gets owner props `{ size, active }`. The row label comes from `label` and falls back to the id (`$DS/dsh-client-ui-sidebar/lib/types/client/contract/slots.d.ts`).
- Linking: the panellist `id` must equal the `main` `key`. No routing code is needed.
- `ctx.layout.selectPanel(id)` selects a panel programmatically (`$DS/dsh-client-ui-layout/lib/types/client/service.d.ts`). The probe does not use it.

## Observations

### (a) Version and install — `log-01-install.txt`

```
$ date -Is; dsh --version
2026-10-13T07:54:46+03:00
0.2.0-rc.2
$ DSH_HOME=$S/home dsh plugin --profile web add $S/brokkr-console-probe

dependencies:
+ brokkr-console-probe link:$S/brokkr-console-probe

Already up to date
Done in 348ms using pnpm v10.28.1
exit=0
```

After the install, `$S/home/profiles/web/package.json` lists `"bundles": ["@deepseek-ai/dsh-base", "@deepseek-ai/dsh-web-app", "brokkr-console-probe"]`. `dsh web --dump-config` contains:

```
# == brokkr-console-probe
- id: brokkr-console-probe
  name: brokkr-console-probe
```

### Serve — `log-02-serve.txt` (launch token redacted)

```
2026-10-13T07:54:57+03:00
$ dsh web --no-open --host 127.0.0.1 --port 38117
2026-10-13T04:54:58.441Z [brokkr-console-probe] apply: webServer listening on 127.0.0.1:38117
dsh web: http://127.0.0.1:38117/?token=<launch token redacted>
```

The Host half loaded and `apply()` ran with `ctx.webServer` injected. `ss -ltnp` showed only `127.0.0.1:38117` (node, pid 231273).

### (c) HTTP route — `log-03-http.txt`, `log-04-auth-boot.txt`

```
$ curl -sS -i http://127.0.0.1:38117/brokkr-probe/ping
HTTP/1.1 200 OK
content-type: application/json; charset=utf-8
cache-control: no-store
...
{"probe":"brokkr-console-probe","route":"/brokkr-probe/ping","pid":231273,"at":"2026-10-13T04:55:05.593Z"}

$ curl -sS -i -X POST http://127.0.0.1:38117/brokkr-probe/ping
HTTP/1.1 405 Method Not Allowed          (probe's own method check)

$ curl -sS -i -H 'Origin: http://evil.example' http://127.0.0.1:38117/brokkr-probe/ping
HTTP/1.1 200 OK                          (no fence unless the plugin adds one)
{"probe":"brokkr-console-probe",...}

$ curl -sS -i http://127.0.0.1:38117/brokkr-probe/fenced            # no login cookie
HTTP/1.1 401 Unauthorized
$ curl -sS -i -H 'Origin: http://evil.example' http://127.0.0.1:38117/brokkr-probe/fenced
HTTP/1.1 403 Forbidden

$ curl -sS -i -c cookies.txt 'http://127.0.0.1:38117/?token=<launch token>'
HTTP/1.1 303 See Other
location: ./
set-cookie: dsh-auth-<id>=<redacted>; Max-Age=2592000; Path=/; ...; HttpOnly; SameSite=Strict
$ curl -sS -i -b cookies.txt http://127.0.0.1:38117/brokkr-probe/fenced
HTTP/1.1 200 OK
{"probe":"brokkr-console-probe","route":"/brokkr-probe/fenced","authenticated":true}
```

### (d) WebSocket upgrade — `log-07-ws.txt`

```
$ curl -sS -i --max-time 2 --no-buffer -H 'Connection: Upgrade' -H 'Upgrade: websocket' \
    -H 'Sec-WebSocket-Version: 13' -H 'Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==' \
    http://127.0.0.1:38117/brokkr-probe/ws | cat -v
HTTP/1.1 101 Switching Protocols^M
Upgrade: websocket^M
Connection: Upgrade^M
Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=^M
^M
M-^A^_hello from brokkr-console-probecurl: (28) Operation timed out after 2002 milliseconds with 33 bytes received
```

`s3pPLMBiTxaQ9kYGzzhZRbK+xOo=` is the RFC 6455 reference accept value for that key. Curl's error 28 only means the socket stayed open until `--max-time`.

```
$ node ws-client.mjs ws://127.0.0.1:38117/brokkr-probe/ws        # Node 22 built-in WebSocket
open (+7ms) readyState=1
message: hello from brokkr-console-probe
sent: ping-from-node-ws-client
message: echo:ping-from-node-ws-client
close code=1000 reason=done
$ node ws-client.mjs ws://127.0.0.1:38117/brokkr-probe/nope      # unregistered upgrade path
error Received network error or non-101 status code.
```

### (b) Client registrations for `main` and `sidebar.panellist`

**Boot graph** (`log-05-bootgraph.txt`). The authenticated `index.html` `window.__DSH_BOOT__` holds 66 entries, including:

```
boot entry: {"id":"brokkr-console-probe","url":"plugins/??probe/client.js&rev=61fdd890c4d5","rev":"61fdd890c4d5","inject":["@deepseek-ai/dsh-client-ui-layout","@deepseek-ai/dsh-client-ui-sidebar"]}
```

The probe's row sits in the `application` combo batch between `dsh-client-ui-trajectory` and `dsh-session-log-export`.

**Served bundle** (`log-06-bundle.txt`):

```
$ curl -sS -i 'http://127.0.0.1:38117/plugins/??probe/client.js&rev=61fdd890c4d5'
HTTP/1.1 200 OK
content-type: text/javascript; charset=utf-8
cache-control: public, max-age=31536000, immutable
...
window.__ModuleLoader__.load({
	id: "brokkr-console-probe",
```

**Headless browser** (`log-08-browser.txt`, `shot-1-sidebar.png`, `shot-2-main.png`):

```
$ node tools/drive.mjs 'http://127.0.0.1:38117/?token=<launch token>' ~/snap/firefox/common/brokkr-probe-ffprofile shot
04:56:46.584Z landed http://127.0.0.1:38117/
04:56:46.701Z sidebar.panellist icon present: true
04:56:46.707Z panel row element label/aria: "Brokkr probe"
04:56:48.315Z dismissed modal via button: "Continue"
04:56:49.294Z main panel mounted: true
04:56:49.356Z DOM brokkr-probe-icon[active=true]: B
04:56:49.356Z DOM brokkr-probe-ping: ping: {"probe":"brokkr-console-probe","route":"/brokkr-probe/ping","pid":231273,"at":"2026-10-13T04:56:49.187Z"}
04:56:49.357Z DOM brokkr-probe-ws: ws: hello from brokkr-console-probe | echo:from-main-panel
exit=0
```

**Host-side beacons from the browser half** (`probe-host.log`, same run):

```
04:56:45.618Z client-report event=client-apply
04:56:45.619Z client-report event=client-registered detail=main+sidebar.panellist
04:56:45.857Z client-report event=sidebar-icon-mounted detail=size=16
04:56:49.186Z client-report event=main-mounted detail=brokkr-probe
04:56:49.186Z GET /brokkr-probe/ping host=127.0.0.1:38117 origin=-
04:56:49.188Z UPGRADE /brokkr-probe/ws host=127.0.0.1:38117 origin=http://127.0.0.1:38117 upgrade=websocket key=present
04:56:49.241Z ws message: from-main-panel
04:56:49.261Z client-report event=main-ws-echo detail=echo:from-main-panel
```

`shot-1-sidebar.png` shows the row "B Brokkr probe" under the first-party "Plugins" row. `shot-2-main.png` shows the selected row and the probe page in the main column, with the ping JSON and the WS echo.

### Stop — `log-09-stop.txt`

```
$ date -Is; kill -TERM -231270   # process group of the volta shim (231270) and node dsh (231273)
2026-10-13T07:57:10+03:00
dsh web exited
port 38117 closed
no dsh/firefox probe processes
```

My first stop attempt did not kill anything, because the saved PID was the already-exited `setsid` wrapper. The second attempt, shown above, stopped the server. Puppeteer closed Firefox itself, and the Firefox profile directory was removed afterwards.

## Divergences and things a plugin author must know

1. **Plugin routes have no auth by default.** `dsh-host-webserver` has "no server-wide TLS, authentication, or origin policy" (README, Known Limitations). `/brokkr-probe/ping` answered 200 to a forged `Origin: http://evil.example` and with no cookie. To fence a route, call `ctx.connection.requestRejection(req)` from `@deepseek-ai/dsh-client-connection`. It returns `403` for an untrusted Host or Origin, `401` without the `dsh-auth-*` login cookie, and `undefined` when the request is allowed. All three outcomes were observed. This is the first-party pattern (`dsh-host-open-in-app`). Add `'connection'` to `inject`.
2. **Upgrades are not fenced either.** The registered upgrade route accepted connections with no Origin and no cookie. The browser does send `Origin: http://127.0.0.1:38117` and its cookie (`SameSite=Strict`, same origin). Calling `requestRejection(req)` from the upgrade handler should give the same fence. **Not measured:** I did not run a fenced upgrade variant.
3. **Unregistered upgrade paths are closed, not answered 404.** The Node client saw a network error.
4. **Port and host.** The web profile defaults to `127.0.0.1:3080` (`ctx.webStartup.port ?? 3080`). `--port` and `--host` override it, and `--port 0` gives an OS-assigned port. The webserver accepts only `127.0.0.1` or `0.0.0.0`. Plugin routes share the one origin with `/api`, `/plugins` and the SPA, so the browser half can use same-origin relative URLs and the cookie flows automatically.
5. **Browser login.** The launch prints `http://127.0.0.1:<port>/?token=<launch token>`. That URL 303-redirects and sets an HttpOnly `dsh-auth-*` cookie, valid for 30 days. Without the cookie the index is 401. The token is per process.
6. **First-launch modal.** On a fresh `DSH_HOME` a "Preview Notice" dialog, with a "Continue" button, covers the page and intercepts clicks. The first browser run timed out on the panel click because of it. A "Unable to create default workspace" toast also appeared because the scratch home has no workspace; it has nothing to do with the probe.
7. **Client bundle format is not documented for hand authoring.** First-party bundles are tsdown output in the `window.__ModuleLoader__.load({id, factory})` CJS-factory shape. The probe hand-writes that shape, and it loaded with no sourcemap and no build step. A real plugin should probably use the same tsdown preset; **the preset is not shipped in the installed package.**
8. **No HMR used.** The web template enables `dsh-hmr`, so `dsh plugin add` against a running profile should hot-apply. This was not tested; the probe was installed before launch.
9. **API names matched expectations.** `register`, `registerUpgrade`, `ctx.slots.register({name:'main', key})` and `ctx.slots.register({name:'sidebar.panellist', id, order, label})` are as described. Separately, schedule's first-party `client.js` also passes `locale: <namespace>`. The probe omitted it, and the registration worked without it.
