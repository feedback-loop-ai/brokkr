# gate-boundary-policy

## Purpose

Which boundaries may hold a gate: decision 0021's gate law under the
boundary axis, the adapters' `hands.harness` and its result door, the
bundle-pinned-script reading for exec gates, the environment an unboxed
exec dispatch runs in, and the run-time argv per boundary (decision 0046
ruling 4; decision 0043 rulings 2 and 3; decision 0021 rulings 2 and 7).

## ADDED Requirements

### Requirement: An adapter declares how its harness stands under the harness boundary
The adapter loader SHALL admit, beside `hands.workspace`, an optional
`hands.harness` object with three members and no other: `gate` and
`work`, each either an argv fragment (an array of strings) or
`{"unsupported": "<measured reason>"}`, an absent member reading
unsupported without a reason, fail-closed, on the three-shape convention
`tool_permissions` uses; and `result`, optional, one of `file` and
`last-message`, absent reading `file`. `gate` is the read-only fragment
decision 0046 ruling 4 names — the harness's own sandbox with reads only
and one write door, the result path — under which a model may judge;
`work` is the harness's own writable sandbox, under which a work-class
site with hands writes the worktree as its charter requires; `result`
says how a gate seat's result reaches the engine under the `gate`
fragment: `file` when the seat writes the result file itself through a
door the fragment scopes to that path, `last-message` when the harness's
own capture writes the seat's final message to that path. A fragment
under `hands.harness` MAY carry `{result_path}`, expanded by the engine
at spawn to the seat's own result path, and `{brokkr}`; the loader SHALL
refuse `{hands_mcp_json}` and `{hands_args_toml}` there, because no
workspace tool is served under `harness`. `adapters/codex.json` SHALL
declare `gate` as `["--sandbox", "read-only", "--output-last-message",
"{result_path}"]` with `result` `last-message` — the capture flag
`codex exec` documents and the codex driver already admits on a
resume — and `work` as `["--sandbox", "workspace-write"]`;
`adapters/claude.json` SHALL declare each of `gate` and `work` only
from a measurement against the installed claude 2.1.x — a fragment,
its `gate` door scoped to `{result_path}`, or `unsupported` with the
measured reason — and SHALL leave a member undeclared until it is
measured, absence being the loader's fail-closed reading; a fragment
is never guessed; `adapters/dsh.json` and
`adapters/lanetally.json` SHALL declare no `hands.harness`. An empty
fragment is a legal declaration: it says the adapter's driver argv
already stands in that mode — claude's driver carries
`--permission-mode acceptEdits`, a candidate `work` answer — and it is
a measured declaration like any other, recorded in the guide with what
the mode denies and allows.

#### Scenario: codex declares both fragments and its door
- **WHEN** `adapters/codex.json` is loaded
- **THEN** `hands.harness.gate` is `--sandbox read-only --output-last-message {result_path}`, `hands.harness.result` is `last-message`, and `hands.harness.work` is `--sandbox workspace-write`

#### Scenario: claude's fragments are measured, not guessed
- **WHEN** `adapters/claude.json` is loaded
- **THEN** `hands.harness.gate` and `hands.harness.work` are each undeclared — not yet measured — or an argv fragment or `unsupported` with a measured reason; a `gate` fragment names `{result_path}` as its door; and the provider-adapters guide records, per member, the claude version it was measured against and what the mode denies and allows, or that it is undeclared pending the operator's measurement

#### Scenario: dsh and lanetally declare none
- **WHEN** `adapters/dsh.json` and `adapters/lanetally.json` are loaded
- **THEN** neither carries `hands.harness`, and each is refused at a `harness` gate exactly as it is refused at a boxed gate today

#### Scenario: An unknown member is refused
- **WHEN** an adapter declares `hands.harness.judge`
- **THEN** loading is refused naming the key and the three members the vocabulary admits

#### Scenario: A result outside the vocabulary is refused
- **WHEN** an adapter declares `hands.harness.result` as `stdout`
- **THEN** loading is refused naming `file` and `last-message`

#### Scenario: A workspace token in a harness fragment is refused
- **WHEN** an adapter's `hands.harness.gate` or `hands.harness.work` names `{hands_mcp_json}` or `{hands_args_toml}`
- **THEN** loading is refused saying no workspace tool is served under `harness`

### Requirement: The gate law reads the boundary for sites that declare hands
`enforce_model_policy` SHALL apply decision 0021's gate refusals as
today under every boundary and, for a gate-class site that declares
hands, SHALL additionally rule by the boundary the bundle compiles
under: under `namespace`, `seatbelt` and `container` a model gate is
admitted as today and a boxed exec gate as decision 0043 ruling 3
reads — the compile defines the identity now, and a run under
`seatbelt` or `container` refuses at start until its slice lands, which
is boundary-availability's rule and not this one's; under `harness` a
model gate is admitted only when every link of its resolved chain
declares `hands.harness.gate` as a fragment, and refused otherwise
naming the link, the provider and the missing declaration; under `open`
a model gate is refused naming decision 0046 ruling 4. A work-class site
with hands under `harness` SHALL be refused as a capability gap when a
link declares no `hands.harness.work` fragment. A gate-class site
without hands has no box whose boundary could be named and SHALL compile
as it does today under every boundary (decision 0046 ruling 4; decision
0021 rulings 2 and 7; decision 0041 ruling 3).

#### Scenario: A harness gate on a provider that declares the fragment is admitted
- **WHEN** a gate-class agent site with hands resolves to a trusted judging provider whose adapter declares `hands.harness.gate`, and the bundle compiles under `harness`
- **THEN** compilation succeeds

#### Scenario: A harness gate on a provider that declares none is refused
- **WHEN** the same site resolves to a provider whose adapter declares no `hands.harness`, and the bundle compiles under `harness`
- **THEN** compilation is refused naming the provider, `hands.harness.gate` and decision 0046 ruling 4

#### Scenario: A fallback link without the fragment refuses the chain
- **WHEN** a gate's chain has a first link whose adapter declares `hands.harness.gate` and a second whose adapter does not, under `harness`
- **THEN** compilation is refused naming the second link

#### Scenario: An open model gate is refused
- **WHEN** a gate-class agent site with hands compiles under `open`, whatever its adapter declares
- **THEN** compilation is refused naming decision 0046 ruling 4

#### Scenario: A seatbelt gate is admitted at compile
- **WHEN** a gate-class agent site with hands compiles under `seatbelt`, and again under `container`
- **THEN** compilation succeeds exactly as under `namespace` and the manifest pins the word, whatever the compiling machine holds

#### Scenario: A harness work seat without a work fragment is refused
- **WHEN** a work-class agent site with hands resolves to a provider whose adapter declares no `hands.harness.work`, under `harness`
- **THEN** compilation is refused as a capability gap naming the provider and `hands.harness.work`

#### Scenario: A gate without hands is untouched
- **WHEN** an inline trusted model gate with a tool list and no hands compiles under `open`
- **THEN** it is admitted exactly as under `namespace`

#### Scenario: namespace is exactly today
- **WHEN** every shipped bundle compiles under `namespace`
- **THEN** every refusal and admission is what it was before this change

### Requirement: An exec gate under harness or open holds only for pinned bytes
An exec gate that declares hands, compiled under `harness` or `open`,
SHALL be admitted only when its command is the bundle's own pinned
script, checked by construction on the raw command, before
`expand_command` erases the `./` spelling: after the `--` that ends the
`{brokkr} driver exec` dispatch, zero or more bare interpreter names —
no path separator, no leading `-`, so `bash -c '…'` is refused — then
exactly one script token, then arguments. The script token is
`./`-relative, every component after `./` a plain name — no `..`, no
`.`, no empty component, no `\`, no drive or UNC prefix — and, joined to
the directory of the layer that declared the seat (the bundle's own
directory, or the ancestor's that wrote the seat under composition), a
regular file at compile by `metadata`, following a symlink as the
manifest walk does, and not a key the walk skips (`realms.json`,
`dialects/…`), the exclusion being one function shared with the walk.
That directory is the one the compiler already expands `./` against and
the one the manifest walk digests, so a token that passes is pinned by
the manifest of the layer that declared it, and no path is canonicalised
and no two spellings are compared (design DD9). The tokens
after the script are its arguments and are not judged, which is how the
shipped ship gate hands `{brokkr}` to its own script. A command with no
such script — a bare program, a `{brokkr}` verb, an absolute path, a
`\`-spelled or `/private/var`-spelled token, a `../` that escapes, or a
`./` token naming no file — SHALL be refused naming decision 0046
ruling 4 and decision 0021 and, for a spelling, the spelling. A dialect
validate or check step, whose argv is the dialect's own and pinned by
the dialect's content digest in the run manifest beside the tool's
declared name and version, SHALL be admitted on the same environment
and network terms, the run marked unboxed all the same; the tool's
binary is not pinned by digest, as `bash` and `cargo` are not pinned
under a bundle-pinned script — both readings pin a declaration and run
a host tool — and the reading is recorded in the proposal's D6 and the
design's DD8 for the operator to confirm or refuse (decision 0046 ruling 4; decision 0042 rulings 1 and
4).

#### Scenario: The shipped verifier under open is admitted
- **WHEN** `bundles/self`, whose verify seat is `["{brokkr}","driver","exec","--","bash","./scripts/verify-seat.sh","{prompt_file}"]` with hands, compiles under `open`
- **THEN** it is admitted, because `bundles/self/scripts/verify-seat.sh` is a file the bundle's own manifest walk pins

#### Scenario: A brokkr-external command under open is refused
- **WHEN** an exec gate with hands whose command is `["{brokkr}","driver","exec","--","true"]` compiles under `open`
- **THEN** compilation is refused naming decision 0046 ruling 4

#### Scenario: An escaping, absolute or platform-spelled script is refused
- **WHEN** the script token is `./../outside.sh`, `/usr/bin/true`, `.\scripts\s.sh` or `/private/var/b/scripts/s.sh` under `harness`
- **THEN** compilation is refused naming the token, and no path is compared to judge it

#### Scenario: A pinned-looking token that names no file is refused
- **WHEN** the script token is `./scripts/missing.sh` and no such file exists under the declaring layer's directory
- **THEN** compilation is refused naming the token and the directory searched

#### Scenario: An option before the script is refused
- **WHEN** the command is `["{brokkr}","driver","exec","--","bash","-c","./scripts/s.sh"]` under `harness`
- **THEN** compilation is refused naming `-c` as an option token before the script

#### Scenario: A file the walk skips is refused as unpinned
- **WHEN** the script token is `./dialects/run.sh` and such a file exists under the declaring layer's directory
- **THEN** compilation is refused naming the token as a path the manifest walk does not pin

#### Scenario: An inherited seat resolves against the layer that wrote it
- **WHEN** `recipes/wager-harness`, which inherits its verify seat from `recipes/fast`, compiles under `harness`
- **THEN** the verify seat is admitted, the file checked being `recipes/fast/scripts/verify-seat.sh`

#### Scenario: The shipped ship gate under harness is admitted
- **WHEN** the shipped ship seat, `["{brokkr}","driver","exec","--","bash","./scripts/ship-seat.sh","{prompt_file}","{brokkr}"]` with hands, compiles under `harness`
- **THEN** it is admitted, because the tokens after the script are its arguments and the `{brokkr}` among them names no command

#### Scenario: A dialect step under harness is admitted
- **WHEN** a fixture bundle with an artifact phase, its chief seated on a fixture provider that declares both `hands.harness` members as fragments, compiles under `harness` in a realm that declares a dialect
- **THEN** its synthetic validate and check steps are admitted and the bundle compiles

### Requirement: Every shipped bundle compiles under harness once the fragments are measured
Decision 0046 ruling 6 promises that after this slice a macOS operator
runs every shipped bundle, the review offices under their harness's own
sandbox. The work offices that declare hands are the chief architect,
which chains claude and codex (fable, astra, opus) and is seated by
`recipes/triage`'s specify and design steps and, through inheritance,
by `recipes/night-shift`, and the sdd intake, which chains claude alone
(sonnet, opus) and is hired by no shipped bundle — `recipes/sdd` folded
into `recipes/triage`'s strategy select in #176 and no longer exists.
So the promise holds exactly when `adapters/codex.json` and
`adapters/claude.json` declare
`hands.harness.gate` and `hands.harness.work` as fragments, each `gate`
with a measured door. Every bundle under `recipes/` and `bundles/` SHALL
compile under `harness` on that condition. If the measurement finds a
claude mode that cannot be declared as a fragment, the refusal SHALL
name the adapter, the member and the site, and the implementation SHALL
report ruling 6's promise as unmet for the operator to rule on — never
widen the rule, seat another provider or declare an unmeasured fragment
to make the shipped bundles compile. The implementing seat cannot make
the claude measurement — its tool grant is `cargo` and `git` — so until
the operator records it the members are undeclared and the same refusal
applies by name; the implementation SHALL finish every other task and
commit, SHALL NOT report itself blocked for want of the measurement,
and SHALL name the measurement as the operator's in its completion note
with the recipe, the candidates and the version. The tree's own proof
of the promise SHALL therefore run in a scratch copy of the shipped
adapter library with the two members planted as fragments, and a second
test SHALL pin, against the shipped adapters as they stand, exactly
which shipped bundles refuse and why — a pin that moves when the
measurement lands (decision 0046 rulings 4 and 6; decision 0042's
addendum, ruling 1: a decision is amended only by a decision).

#### Scenario: The shipped bundles compile under harness
- **GIVEN** an adapter library in which the codex and claude adapters declare both `hands.harness` members as fragments — the shipped files once the measurement lands, a scratch copy with the members planted until then
- **WHEN** every bundle under `recipes/` and `bundles/` compiles under `harness` against it, in a realm that declares the openspec dialect
- **THEN** each compiles, and each hands site's manifest `boundary` entry reads `harness`

#### Scenario: The measurement is not reachable from the implementing seat
- **WHEN** the shipped adapters are loaded as they stand, `adapters/claude.json` declaring no `hands.harness` member because no claude the implementing seat may run was reachable, and every bundle under `recipes/` and `bundles/` compiles under `harness`
- **THEN** exactly the bundles that seat an agent with hands whose chain reaches claude — `bundles/self`, `recipes/panel-review`, `recipes/triage` and `recipes/night-shift` — refuse naming `claude`, the member and the site; every other shipped bundle compiles; and the implementation completes and commits every other task, reports nothing blocked, and names the measurement in its completion note as the operator's with the recipe, the candidates and the version

#### Scenario: A measured gap is reported, not papered over
- **WHEN** the measurement declares claude's `work` member unsupported
- **THEN** `recipes/triage` and `recipes/night-shift` — every shipped bundle under `recipes/` and `bundles/` that seats the chief architect — refuse under `harness` naming `claude`, `hands.harness.work` and the site; every other shipped bundle still compiles; and the implementation reports the unmet promise instead of amending the adapter, the roster or the rule

### Requirement: The argv of a site with hands follows the boundary and the class
At run time the engine SHALL compose the argv of every site with hands
from the boundary the bundle was compiled under, which is the realms map
the run was started with: under `namespace` exactly today's path — the
adapter's `hands.workspace` fragment with `{hands_mcp_json}`,
`{hands_args_toml}` and `{brokkr}` expanded, and `brokkr hands exec`
around an exec dispatch; under `harness` the adapter's
`hands.harness.gate` fragment for a gate-class site and
`hands.harness.work` for a work-class site, with `{result_path}`
expanded to the seat's own result path and `{brokkr}` to this binary,
no workspace tool served and no box built; under `open` no fragment of
Brokkr's at all. Under `harness` and `open` the site's `hands.network`
and `hands.binds` stay pinned in the manifest as declared and are
enforced by nothing of Brokkr's: the harness's own sandbox decides what
the hands may reach, which is the fact the *unboxed* rendering states.
Under `harness` and `open` alike an exec dispatch — an exec gate's or
a dialect step's — SHALL be the compiled command itself, spawned by
the engine through `DriverProcess::spawn` with no verb of Brokkr's
around it: `{brokkr}` and `./` were expanded at compile by
`expand_command` against the declaring layer's directory, and
`{prompt_file}` stays literal for the exec driver to expand when it
stages the prompt. It SHALL start in the environment the next
requirement lists and, on Linux only, behind a network prefix when a
probe at spawn passes — `unshare --map-root-user --net -- sh -c 'ip
link set lo up && exec unshare --map-user=<uid> --map-group=<gid> --
"$@"' sh`, `<uid>` and `<gid>` being the engine's own ids: a user
namespace with the engine mapped to root, so the exec'd `sh` keeps the
capability to bring the loopback up, then a second user namespace
mapping root back to the operator, so the dispatch runs as the
operator with its capabilities dropped on exec and the network
namespace inherited. Every layer replaces itself by exec, so the PID
the engine holds is the driver's and the deadline kill reaches it as
today. The probe SHALL be that prefix around `true`, run in the
dispatch's environment against its search path: with no `unshare` on
the path nothing is spawned and the answer is no; with a non-zero exit
the prefix is skipped and the dispatch runs with the network on. The
prefixed argv SHALL be one pure function of the dispatch, the probe's
answer and the ids, which the argv tests read directly; the probe's
answer is not journaled, and the record marks the run unboxed all the
same. A model site's process inherits the engine's environment under
every boundary, because its harness needs the operator's keys. An
inline model site with hands under `harness` or `open` SHALL be
refused at compile naming the repair, because its argv is the author's
and carries the box's own tokens.
`seatbelt` and `container` never reach composition: the engine refuses
them at its entry before any journal row (boundary-availability), and
composition is written over the three boundaries this engine builds
(decision 0046 rulings 1 and 4; decision 0043 rulings 1 and 3).

#### Scenario: namespace is byte-identical to today
- **WHEN** a boxed model site and a boxed exec site are composed under `namespace`
- **THEN** their argv equal what `hands_command` produced before this change, token for token

#### Scenario: A harness gate takes the read-only fragment and its door
- **WHEN** a gate-class agent site with hands on codex, whose result path is `<workdir>/.forge/results/<effect>.json`, is composed under `harness`
- **THEN** its argv carries `--sandbox read-only --output-last-message <workdir>/.forge/results/<effect>.json`, no `mcp_servers.brokkr` entry, no `{hands_mcp_json}` expansion, and no literal `{result_path}`

#### Scenario: A harness work seat takes the work fragment
- **WHEN** a work-class agent site with hands on codex is composed under `harness`
- **THEN** its argv carries `--sandbox workspace-write` and no MCP server

#### Scenario: An open site takes nothing
- **WHEN** a work-class agent site with hands is composed under `open`
- **THEN** its argv is the adapter's base driver argv with the model and effort pins and nothing else

#### Scenario: The hands policy is declared, not enforced, under harness
- **WHEN** a site declaring `"hands": {"kind": "workspace", "network": false, "binds": []}` is composed under `harness`
- **THEN** its manifest `hands` entry still says `network` false, its argv carries no network switch of Brokkr's, and the run is rendered *unboxed*

#### Scenario: The shipped verify seat under harness on Linux with the probe passing
- **WHEN** `bundles/self`'s verify seat, compiled in this repository so that `{brokkr}` is `<brokkr>` and `./scripts/verify-seat.sh` is `<repo>/bundles/self/scripts/verify-seat.sh`, is composed under `harness` on Linux with the probe passing and the engine's ids `<uid>` and `<gid>`
- **THEN** its argv is exactly, token by token: `unshare`, `--map-root-user`, `--net`, `--`, `sh`, `-c`, `ip link set lo up && exec unshare --map-user=<uid> --map-group=<gid> -- "$@"`, `sh`, `<brokkr>`, `driver`, `exec`, `--`, `bash`, `<repo>/bundles/self/scripts/verify-seat.sh`, `{prompt_file}` — no `hands` verb, no `/runtime/bundle` path, and the literal `{prompt_file}` left for the exec driver

#### Scenario: The same seat with the probe failing, and off Linux
- **WHEN** the same seat is composed under `harness` on Linux with the probe failing, and again on macOS and on Windows
- **THEN** its argv is exactly `<brokkr>`, `driver`, `exec`, `--`, `bash`, `<repo>/bundles/self/scripts/verify-seat.sh`, `{prompt_file}` — the compiled command untouched, spawned in the fixed environment with the network on — and the same holds under `open`

#### Scenario: The probe is the prefix around true
- **WHEN** the probe runs
- **THEN** the command it spawns is the eight-token prefix followed by `true`, in the dispatch's environment, and nothing it learns is journaled

#### Scenario: The probe's arms on a planted search path
- **WHEN** the probe is given a search path with no `unshare`, then one whose `unshare` is a planted executable exiting non-zero, then one exiting zero
- **THEN** it answers no without spawning anything, no, and yes, in that order; and on macOS and Windows it is never consulted

#### Scenario: A dialect step takes the same road
- **WHEN** a dialect validate step is composed under `open` on Linux with the probe passing
- **THEN** its argv is the prefix followed by `<brokkr>`, `driver`, `exec`, `--`, `openspec`, `validate`, `<change>`, `--strict`, `--no-interactive`, in the fixed environment

#### Scenario: A model site keeps the engine's environment
- **WHEN** a gate-class agent site with hands on codex is composed under `harness`
- **THEN** it is spawned with the engine's own environment, exactly as under `namespace`

#### Scenario: An inline model site with hands under harness is refused
- **WHEN** an inline seat whose command is a `{brokkr} driver claude` dispatch declares `hands` and compiles under `harness`
- **THEN** compilation is refused naming the seat and the repair

### Requirement: An unboxed exec dispatch runs in a fixed environment
Under `harness` and `open` the engine SHALL start an exec dispatch —
through `DriverProcess::spawn`, which takes the environment the child
starts with: the engine's own, today's behaviour and every model site's
under every boundary, or exactly a composed table — from an empty
environment and set exactly these keys and no other, the box's own
table with the paths a namespace would remap replaced by the paths that
stand outside one (design DD10): `HOME` and `TMPDIR`, two private
directories created for the attempt under the run's scratch and never
the operator's; `PATH`, `USER` and `LOGNAME`, inherited verbatim from
the engine's own environment, each only when set there; `CARGO_HOME`,
`RUSTUP_HOME` and `NPM_CONFIG_CACHE`, set to the operator's `~/.cargo`,
`~/.rustup` and `~/.npm` — `~` the engine's home as `expand_home` reads
it — exactly when the site's `hands.binds` declare that path, as the box
sets them, and absent otherwise, a bind's `mask` being declared and not
enforced outside a namespace; the in-box marker `BROKKR_HANDS_BOX` —
true of the child exactly when the engine itself already stands inside
a box; and, on Windows only, `USERPROFILE`, `HOMEDRIVE`, `HOMEPATH`,
`SYSTEMROOT`, `SYSTEMDRIVE`, `WINDIR`, `COMSPEC`, `PATHEXT`, `TEMP`,
`TMP`, `USERNAME`, `APPDATA`, `LOCALAPPDATA` and `PROGRAMDATA`,
verbatim, without which no Windows process starts; fixed as the box
sets them, `LANG` and `LC_ALL` as `C.UTF-8`, `CI` as `true`,
`DISABLE_AUTOUPDATER` and `DISABLE_TELEMETRY` as `1`, and
`GIT_CONFIG_COUNT`, `GIT_CONFIG_KEY_0` and `GIT_CONFIG_VALUE_0` as the
`commit.gpgsign=false` triple; and the bundle's `git.identity` entries.
The engine SHALL never set the in-box marker on the dispatch, because
no box stands and the marker is what every box-building test skips on.
The environment SHALL be composed by one pure function of the engine's
environment, the engine's home, the site's spec, the identity and the
two scratch paths, which the tests read directly; the network probe
runs in it, and the dispatch's working directory is the worktree, as
every driver's is. A dialect step under `harness` or `open` runs in the
same environment (decision 0046 ruling 4; decision 0043 ruling 1's
allow-list, from which the table is taken).

#### Scenario: The shipped verify gate under harness on a rustup machine
- **GIVEN** an engine environment of `HOME=/home/op`, `PATH=/home/op/.cargo/bin:/usr/bin:/bin`, `GH_TOKEN=secret`, `ANTHROPIC_API_KEY=secret`, `SSH_AUTH_SOCK=/run/agent` and no `CARGO_HOME`
- **WHEN** `bundles/self`'s verify seat, whose binds declare `~/.cargo` and `~/.rustup`, is composed under `harness`
- **THEN** the environment holds `PATH` verbatim, `HOME` and `TMPDIR` as the attempt's private directories, `CARGO_HOME=/home/op/.cargo` and `RUSTUP_HOME=/home/op/.rustup` from the declared binds, `LANG` and `LC_ALL` `C.UTF-8`, `CI` `true`, the two switches, the gpgsign triple and the bundle's git identity, and no `GH_TOKEN`, `ANTHROPIC_API_KEY`, `SSH_AUTH_SOCK`, `NPM_CONFIG_CACHE` or `BROKKR_HANDS_BOX`; the command is the compiled dispatch `<brokkr> driver exec -- bash <repo>/bundles/self/scripts/verify-seat.sh {prompt_file}`, behind the network prefix when the probe passes, spawned in the worktree, so rustup's cargo proxy under `~/.cargo/bin` resolves the toolchain through the operator's `~/.rustup`

#### Scenario: A planted secret in the operator's home is out of reach
- **GIVEN** an engine `HOME` under which `.ssh/id` and `.cargo/credentials.toml` are planted, and a site whose binds declare `~/.cargo`
- **WHEN** a dispatch of `sh -c 'cat "$HOME/.ssh/id"'` is spawned in the composed environment
- **THEN** it fails, because `HOME` is the private directory; and `CARGO_HOME` names the planted `.cargo`, because the bind declares it and a mask is not enforced outside a namespace, which the guide states

#### Scenario: The locators follow the binds, not the engine's environment
- **WHEN** the engine's environment sets `CARGO_HOME`, `RUSTUP_HOME` and `NPM_CONFIG_CACHE` and the site declares no bind
- **THEN** the composed environment carries none of them; and when the site declares `~/.npm`, it carries `NPM_CONFIG_CACHE` as the engine's home joined with `.npm`

#### Scenario: The marker is inherited, never set
- **WHEN** the engine's environment carries `BROKKR_HANDS_BOX`, and again when it does not
- **THEN** the composed environment carries it in the first case and not in the second

#### Scenario: Windows starts its processes
- **WHEN** the environment is composed on Windows with `SYSTEMROOT`, `COMSPEC`, `PATHEXT`, `USERPROFILE` and `TEMP` set
- **THEN** each is carried verbatim, and on Linux and macOS the Windows names are not consulted

#### Scenario: A dialect step gets the same environment
- **WHEN** a dialect validate step is composed under `open`
- **THEN** its environment is the same table, and its dispatch is `<brokkr> driver exec -- openspec validate <change> --strict --no-interactive` behind the same prefix on the same terms

### Requirement: A judge under harness still delivers its result file
Under `harness` a gate-class model site SHALL be able to deliver exactly
its result: with `result` `file`, the adapter's `gate` fragment, as
measured, leaves the one write door the expanded `{result_path}` names
and the seat writes the file as the result contract says; with `result`
`last-message`, the fragment's capture flag carries the expanded path,
the seat input carries `result_delivery: last-message`, and the prompt's
result contract tells the seat that its final message must be exactly
the result object, which the harness writes to the path. The engine
reads the result file as today in both cases; a final message that is
not the bare object is a missing result, exactly as a malformed file is.
An adapter whose measured read-only mode leaves no such door SHALL
declare `hands.harness.gate` as `{"unsupported": "<measured reason>"}`
and is refused at a `harness` gate. The result file stays the only
channel the engine reads (decision 0046 ruling 4; decision 0043 as
amended by the boxed-marker fix).

#### Scenario: The door points at the seat's own result path
- **WHEN** a gate-class site on codex whose result path is `P` is composed under `harness`
- **THEN** its argv carries `--output-last-message P`, its input carries `result_delivery: last-message`, and its prompt says the final message must be exactly the result object and names `P`

#### Scenario: A file door names the path and changes nothing else
- **WHEN** a gate-class site on an adapter with `result` absent and a `gate` fragment carrying `{result_path}` is composed under `harness`
- **THEN** the expanded argv names the seat's result path where the token stood, the input carries no `result_delivery`, and the prompt's result contract is today's

#### Scenario: The measured door is recorded
- **WHEN** an adapter declares `hands.harness.gate` as a fragment
- **THEN** the provider-adapters guide records, for that fragment, the measurement that showed a seat under it delivering its result and nothing else, or — where the fragment is the decision's own word and the door the tool's documented capture, as codex's is — that the door is declared from the tool's record with the measurement of the capture under the read-only class named as the operator's and pending until it is recorded

#### Scenario: No door means unsupported
- **WHEN** a harness's read-only mode is measured to leave no way to deliver the result
- **THEN** its adapter declares `hands.harness.gate` unsupported with that reason, and a `harness` gate on it is refused at compile
