# dsh boxed hands on an engine-only home (#896, decision 0098)

Date: 2026-11-01. Host: Linux 6.17.0-41-generic (x86_64), the seat's own
workstation. dsh: 0.2.1-rc.2 (`dsh --version`). Engine: this branch's
`target/debug/brokkr` ("brokkr 0.12.0") built at commit
`c21ab42d9d00c5a2dcf05e4c3ec09269d642cb32` (U9b1b, PR #933). Route:
`spark-glm/GLM-5.3-Flash-EXL3` on the operator's local vLLM, with the
placeholder `SPARK_API_KEY=placeholder` the adapter's guide says the
server does not check. No key, `.env`, credential store or operator dsh
home was read, copied or printed.

This record is the measurement decision 0098's boxed shape was waiting
for: the `cold`/`hands: boxed` ambient axis under an engine-only
`DSH_HOME`, the work seat's `replacement`/`hands: boxed` launch after a
first run in the workdir, the same launch under `setpriv
--no-new-privs`, and #914's composed-tree check on that home. The
replacement is measured because a work seat is also served its cold
replacement: without it, the boxed hands office would still refuse at
compile, by name, after the cold shape was recorded. It is measured with
the method
`adapters/dsh.json` records for the `replacement` shape (U0b D06 and pass
8): positive controls that make home- and profile-level sentinels load,
then a fresh engine-only home that starts neither while the engine's row
loads.

## Scratch and commands

Every command ran through one scratch script under `.forge/measure/`
(not committed), with all state under a fresh `mktemp -d /tmp/…`
directory removed on exit. The seat's file sandbox refuses `/var/tmp`
(read-only) and its approval channel was unavailable, so the platform
temp area `/tmp` stood in; it is still outside the worktree. The
measurement script's shape is reproduced below with `$SCRATCH` for that
directory.

The engine overlay is the one `brokkr driver dsh` writes for a boxed
hands seat: the ten disabled effectful rows, `sandbox-policy:
read-only`, and Brokkr's `dsh-mcp-client` row `id: brokkr-hands`,
`serverName: brokkr`, `transport: stdio`, `failOnStartupError: true`,
`command: $BROKKR hands serve --workdir … --artifacts … --reach … --spec
…`. The route overlay is the local `spark-glm` provider entry and the
default model row.

## (a) The cold/boxed ambient axis

### Positive control: planted home- and profile-level sentinels DO load

A fresh home was materialised once with
`dsh --profile headless --patch route.yml --dump-config` (98 shipped
rows). Two MCP sentinels were then planted: one inserted by the
home-level user layer `$DSH_HOME/cordis.patch.yml`, one by the
profile-level `$DSH_HOME/profiles/headless/cordis.patch.yml`. Each
appends its label to `starts.txt` when it starts.

```console
$ DSH_HOME=$SCRATCH/pos/home dsh --profile headless --patch route.yml --dump-config
rows=98 exit=0
$ # plant home-sentinel and profile-sentinel as above
$ DSH_HOME=$SCRATCH/pos/home dsh --profile headless --patch route.yml --dump-config
rows=100 clients=2 exit=0
$ DSH_HOME=$SCRATCH/pos/home SPARK_API_KEY=placeholder dsh --profile headless \
    --patch route.yml "Reply with exactly: POS-DONE"
run exit=0
starts:
started home-sentinel pid=65
started profile pid=64
started profile pid=66
started home-sentinel pid=67
final: POS-DONE
```

The control binds the method: a sentinel at either layer is composed
into the tree (99 + 2 rows) and started by the session, twice each
(dsh's negotiation probe, then the serving process).

### Fresh engine-only home: neither planted sentinel, the engine row loads

A second, fresh `DSH_HOME` was used directly, exactly as
`DshHome::stage` leaves an engine-only home (empty, then materialised by
the driver's own `--dump-config` probe). The overlay carried the same
`brokkr-hands` row with the sentinel standing in for the engine hands
server, so the row's start is observable without a model call.

```console
$ DSH_HOME=$SCRATCH/neg/home dsh --profile headless \
    --patch engine.yml --patch route.yml --dump-config
rows=99 clients=1 exit=0
ambient sentinel rows in dump: 0
$ DSH_HOME=$SCRATCH/neg/home SPARK_API_KEY=placeholder dsh --profile headless \
    --patch engine.yml --patch route.yml "Reply with exactly: NEG-DONE"
run exit=0
starts:
started engine pid=103
started engine pid=104
final: NEG-DONE
```

Only the engine row started. No home- or profile-level row exists in
the fresh home, and the composed tree carries exactly the measured 99
rows and one `dsh-mcp-client`.

### The real engine row

The sentinel was replaced by Brokkr's actual hands server, bound to a
fresh workdir, artifact root, reach and spec, with the local `spark-glm`
route. The model was asked to list its `mcp__` tools (a list, not a
call).

```console
$ DSH_HOME=$SCRATCH/real/home dsh --profile headless \
    --patch engine.yml --patch route.yml --dump-config
rows=99 clients=1 exit=0
$ DSH_HOME=$SCRATCH/real/home SPARK_API_KEY=placeholder dsh --profile headless \
    --patch engine.yml --patch route.yml \
    "List the exact names of every tool you have whose name begins with mcp__. ..."
run exit=0
model saw: mcp__brokkr__workspace
```

The engine's one server loaded and registered its tool. This is a tool
listing, not `mcp__brokkr__workspace` being called: the live call through
this driver remains unmeasured, as decision 0098 ruling 5 says.

### The work seat's replacement

A work seat is also served its cold replacement, so the measurement was
repeated after a first boxed hands run in the same workdir and with a
second fresh engine-only home. dsh's resume shape is unmeasured, so the
engine declines the offer and serves the cold replacement; the dsh-side
launch is the same overlay and home.

```console
$ # first: a cold boxed hands run in $SCRATCH/work
dump rows=99 clients=1; run exit=0; model saw: mcp__brokkr__workspace
$ # replacement: a fresh DSH_HOME, the same workdir
dump rows=99 clients=1; run exit=0; model saw: mcp__brokkr__workspace
first=99 replacement=99 identical=True
```

The replacement's composed tree is the same measured 99 rows and the
same one `brokkr-hands` client, and its engine row loads. The overlay is
the same boxed hands overlay, so the other three axes are the cold boxed
shape's.

## (b) The same launch under `setpriv --no-new-privs`

```console
$ DSH_HOME=$SCRATCH/nnp/home setpriv --no-new-privs dsh --profile headless \
    --patch engine.yml --patch route.yml --dump-config
rows=99 clients=1 exit=0
$ DSH_HOME=$SCRATCH/nnp/home SPARK_API_KEY=placeholder setpriv --no-new-privs \
    dsh --profile headless --patch engine.yml --patch route.yml \
    "List the exact names of every tool you have whose name begins with mcp__. ..."
run exit=0
model saw: mcp__brokkr__workspace
stderr sandbox words: 0
```

The launch is unchanged by U9b1b's `no_new_privs`: the composed tree is
still the measured 99 rows, the engine row still starts, and no
`SANDBOX_UNAVAILABLE` or partial-enforcement line is printed.

dsh's own sandbox is defence in depth under the hands rows (no native
tool that it confines remains), so the rung was exercised by the same
host with a native bash turn, normal and under `no_new_privs`:

```console
$ sysctl -n kernel.apparmor_restrict_unprivileged_userns
1
$ bwrap --ro-bind / / --dev /dev --unshare-pid --proc /proc --die-with-parent -- true
bwrap: No permissions to create new namespace, likely because the kernel does not allow non-privileged user namespaces.
  exit=1
$ setpriv --no-new-privs bwrap --ro-bind / / --dev /dev --unshare-pid --proc /proc --die-with-parent -- true
bwrap: No permissions to create new namespace, likely because the kernel does not allow non-privileged user namespaces.
  exit=1
$ landlock-run --probe
landlock: fully enforced
  exit=0
$ setpriv --no-new-privs landlock-run --probe
landlock: fully enforced
  exit=0
```

`bwrap` is unusable on this host because the host's AppArmor policy
restricts unprivileged user namespaces (`kernel.apparmor_restrict_unprivileged_userns
= 1`), not because of `no_new_privs`: it fails identically with and
without the flag. dsh's Linux chain therefore selects Landlock; its
functional probe reports full enforcement under `no_new_privs`, and the
native bash control under the flag completes:

```console
$ DSH_HOME=$SCRATCH/ctl/home SPARK_API_KEY=placeholder setpriv --no-new-privs \
    dsh --profile headless --patch route.yml --json \
    "Use the bash tool to run exactly: echo NNP-NATIVE-OK. ..."
run exit=0
{"type":"tool_result",...,"status":"completed","result":"NNP-NATIVE-OK\n"}
{"type":"final","text":"NNP-NATIVE-OK"}
stderr sandbox words: 0
```

No `SANDBOX_UNAVAILABLE`, no partial-enforcement line, and the command
ran: Landlock (the rung this host reaches) still starts under
`no_new_privs`. This host cannot show `bwrap` under `no_new_privs`
because it cannot show `bwrap` at all; that is a host fact, not a
`no_new_privs` refusal.

## (c) #914's composed-tree check on the engine-only home

The dump from (a)'s real engine row was compared row for row with the
measured `dsh-hands-round2` dump that `MEASURED_COMPOSITION` pins
(ids and package names in order):

```console
composed rows=99 measured rows=99 identical=True
duplicate ids: []
dsh-mcp-client rows: [('brokkr-hands', '@deepseek-ai/dsh-mcp-client')]
```

The engine-only home stays exactly within the measured 99 rows, the
engine's server row is the one `dsh-mcp-client`, its id is
`brokkr-hands`, and there are no duplicate ids. #914's check holds on
this home.

## The compile and launch the change proves

The measurement was taken on a scratch library under `.forge/measure/`
(not committed; the tree's `recipes/` cannot be extended in this unit),
extending `brokkr-fast` with a work-class dsh office that declares
`"hands": {"kind": "workspace", "network": false, "binds": []}` and
pins `spark-glm/GLM-5.3-Flash-EXL3`.

Before the adapter edit, the office refused at compile with the issue's
error:

```console
$ brokkr compile --bundle .forge/measure/lib/dsh-hands-office
error: bundle: bundle: seat 'implement' (office 'implement') in realm 'brokkr': provider 'dsh' has no measured strict MCP configuration for 'cold' (composed: dsh-hands-office -> brokkr-fast -> fast)
```

After recording the two measured boxed shapes it compiles:

```console
$ brokkr compile --bundle .forge/measure/lib/dsh-hands-office
exit=0
"bundle": "dsh-hands-office",
"digest": "87853bb1c130d2e1f09873bd4037f9f2116d4b656a1028a7e83da9ade38f91eb",
```

The launch itself is (a)'s real engine row, and the two measured shapes
are held by
`bundle::model_policy_tests::dsh_hands_tests::a_boxed_dsh_hands_office_compiles_past_the_lifted_fence`
and
`agents::mcp::tests::dsh_declares_only_its_own_cold_and_replacement_shapes_and_inherits_nothing`.

## Sentinels and controls

| Control | What it shows |
| --- | --- |
| home-level sentinel in `$DSH_HOME/cordis.patch.yml` | dsh composes and starts a home-level MCP row |
| profile-level sentinel in `$DSH_HOME/profiles/headless/cordis.patch.yml` | dsh composes and starts a profile-level MCP row |
| fresh engine-only home | starts neither, and carries neither row; only the engine row starts |
| planted rows in the dump | 98 → 100 rows and 0 → 2 clients, so the dump would expose an ambient row |
| real `brokkr hands serve` row | the model lists `mcp__brokkr__workspace` |
| replacement after a first workdir run | the same 99 rows and the same engine tool, no ambient row |
| `setpriv --no-new-privs` | the same tree, the same tool, no refusal |
| `landlock-run --probe` with and without the flag | `landlock: fully enforced` in both |
| native bash under the flag | completes, no `SANDBOX_UNAVAILABLE`, no partial line |

## Still unmeasured

- A live `mcp__brokkr__workspace` call through this driver: the model
  listed the tool; it did not call it.
- macOS, on every figure (decision 0063 keeps it a supported host; the
  fence stands there).
- Route scope: a shape carries no route, as the earlier shapes say.
- A `bwrap` rung under `no_new_privs`: this host refuses `bwrap`
  altogether. Landlock, the rung dsh selects here, was measured.
- dsh's native egress and any OFF control for it (decision 0065
  slice one; owed to the controller).
