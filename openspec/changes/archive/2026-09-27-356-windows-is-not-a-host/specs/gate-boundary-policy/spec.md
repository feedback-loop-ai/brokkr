## MODIFIED Requirements

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
the engine's own environment, each only when set there and matched by
its exact name; `CARGO_HOME`,
`RUSTUP_HOME` and `NPM_CONFIG_CACHE`, set to the operator's `~/.cargo`,
`~/.rustup` and `~/.npm` — `~` the engine's home as `expand_home` reads
it — exactly when the site's `hands.binds` declare that path, as the box
sets them, and absent otherwise, a bind's `mask` being declared and not
enforced outside a namespace; the in-box marker `BROKKR_HANDS_BOX` —
true of the child exactly when the engine itself already stands inside
a box. No other inherited name SHALL be carried, the process-startup
names a Windows host sets included, because the hosts are Linux and
macOS (decision 0063); fixed as the box
sets them, `LANG` and `LC_ALL` as `C.UTF-8`, `CI` as `true`,
`DISABLE_AUTOUPDATER` and `DISABLE_TELEMETRY` as `1`, and
`GIT_CONFIG_COUNT`, `GIT_CONFIG_KEY_0` and `GIT_CONFIG_VALUE_0` as the
`commit.gpgsign=false` triple; and the bundle's `git.identity` entries.
The engine SHALL never set the in-box marker on the dispatch, because
no box stands and the marker is what every box-building test skips on.
An inherited marker SHALL be matched by its exact name, with its value
unchanged.
The environment SHALL be composed by one pure function of the engine's
environment, the engine's home, the site's spec, the identity and the
two scratch paths, which the tests read directly; the network probe
runs in it, and the dispatch's working directory is the worktree, as
every driver's is. Clearing the environment confines nothing on disk:
an unboxed script may open any host path the operator's uid may read,
which the guide states beside the mask and the *unboxed* rendering
reports (decision 0046 ruling 4; decision 0043 ruling 1's allow-list,
from which the table is taken).

#### Scenario: The shipped verify gate under harness on a rustup machine
- **GIVEN** an engine environment of `HOME=/home/op`, `PATH=/home/op/.cargo/bin:/usr/bin:/bin`, `GH_TOKEN=secret`, `ANTHROPIC_API_KEY=secret`, `SSH_AUTH_SOCK=/run/agent` and no `CARGO_HOME`
- **WHEN** `bundles/self`'s verify seat, whose binds declare `~/.cargo` and `~/.rustup`, is composed under `harness`
- **THEN** the environment holds `PATH` verbatim, `HOME` and `TMPDIR` as the attempt's private directories, `CARGO_HOME=/home/op/.cargo` and `RUSTUP_HOME=/home/op/.rustup` from the declared binds, `LANG` and `LC_ALL` `C.UTF-8`, `CI` `true`, the two switches, the gpgsign triple and the bundle's git identity, and no `GH_TOKEN`, `ANTHROPIC_API_KEY`, `SSH_AUTH_SOCK`, `NPM_CONFIG_CACHE` or `BROKKR_HANDS_BOX`; the command is the compiled dispatch `<brokkr> driver exec -- bash <repo>/bundles/self/scripts/verify-seat.sh {prompt_file}`, behind the network prefix when the probe passes, spawned in the worktree, so rustup's cargo proxy under `~/.cargo/bin` resolves the toolchain through the operator's `~/.rustup`

#### Scenario: A planted secret is not handed over through the environment
- **GIVEN** an engine `HOME` under which `.ssh/id` and `.cargo/credentials.toml` are planted, and a site whose binds declare `~/.cargo`
- **WHEN** a dispatch of `sh -c 'cat "$HOME/.ssh/id"'` is spawned in the composed environment
- **THEN** it fails, because `HOME` is the private directory — a proof about the environment and not the filesystem: the same script naming the operator's home by its absolute path reads the file, which the guide states; and `CARGO_HOME` names the planted `.cargo`, because the bind declares it and a mask is not enforced outside a namespace, which the guide states too

#### Scenario: The locators follow the binds, not the engine's environment
- **WHEN** the engine's environment sets `CARGO_HOME`, `RUSTUP_HOME` and `NPM_CONFIG_CACHE` and the site declares no bind
- **THEN** the composed environment carries none of them; and when the site declares `~/.npm`, it carries `NPM_CONFIG_CACHE` as the engine's home joined with `.npm`

#### Scenario: The marker is inherited, never set
- **WHEN** the engine's environment carries `BROKKR_HANDS_BOX`, and again when it does not
- **THEN** the composed environment carries it in the first case and not in the second

#### Scenario: Only the named keys are carried
- **GIVEN** an engine environment that also sets `USERPROFILE`, `HOMEDRIVE`, `HOMEPATH`, `SystemRoot`, `SYSTEMDRIVE`, `windir`, `ComSpec`, `PATHEXT`, `TEMP`, `TMP`, `USERNAME`, `APPDATA`, `LOCALAPPDATA` and `PROGRAMDATA`, alongside `GH_TOKEN`, `ANTHROPIC_API_KEY` and `SSH_AUTH_SOCK`
- **WHEN** an unboxed exec dispatch is composed
- **THEN** the exact composed key set contains only this requirement's fixed, inherited, bind-gated and identity entries: none of those names travels
- **AND** a `Path`, `User`, `LogName` or `brokkr_hands_box` spelling is not the name it resembles and is not carried
