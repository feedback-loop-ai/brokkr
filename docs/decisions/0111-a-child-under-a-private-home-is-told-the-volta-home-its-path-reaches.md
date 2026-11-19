# 0111 — A child under a private `HOME` is told the volta home its `PATH` reaches

Status: proposed
Date: 2026-11-19

Built: built

## Context

The unboxed exec dispatch (`hands::unboxed_environment`, under 0046
ruling 4's "environment cleared") and the probe's launches
(`probe/observe.rs`, `Runner::launch`, under 0012's "nothing of the
operator's") each clear the environment, keep the engine's `PATH` and
set a private `HOME`. Neither carried any name that says where a
volta-installed CLI lives.

A volta shim finds its home as `$VOLTA_HOME`, else `$HOME/.volta`. Under
the private `HOME` the second is an empty directory: the shim finds no
tool, "delegates to system", takes only that empty home's `bin` out of
`PATH`, finds the operator's `~/.volta/bin/<tool>` (itself) next, and
recurses until fork fails. On the ripper on 2026-11-17 a lane whose
`PATH` listed `~/.volta/bin` left about 180,000 `dsh --version`
processes and a load average over 130,000. Reproduced on volta 2.0.2
under `systemd-run --user --scope -p TasksMax=30`: `env -i HOME=<empty>
PATH=$HOME/.volta/bin:/usr/bin:/bin dsh --version` delegates 15 times
and fails on fork; with `VOLTA_HOME=$HOME/.volta` it prints the version.

Alternatives weighed:

- **Take `~/.volta/bin` out of the child's `PATH`.** The child would then
  not find the CLI it was asked to run at all.
- **Always set `VOLTA_HOME=~/.volta`.** That names a locator no shim on
  the child's `PATH` would read, which the toolchain homes already avoid:
  `CARGO_HOME`, `RUSTUP_HOME` and `NPM_CONFIG_CACHE` are set only where a
  bind declares them.
- **Pass `VOLTA_HOME` through only when the engine has it.** The
  operator's own shell usually lacks it (volta's installer adds the
  `bin` to `PATH` and relies on `$HOME/.volta`), so the recursion would
  stay on exactly the host that met it.

## Rulings

1. **One resolver names the volta home.** `hands::environment::volta_home`
   is the one function both the unboxed dispatch and the probe's launches
   read (0071 ruling 5). It returns the engine's own `VOLTA_HOME`
   verbatim when set; else the operator's `~/.volta`, only when the
   operator's home is absolute and the inherited `PATH` lists that home's
   `bin` exactly; else nothing.

   **Enforcement binding:** `hands::environment::tests::a_private_home_is_told_the_volta_home_a_shim_on_path_reads`,
   `hands::environment::tests::the_unboxed_dispatch_names_the_volta_home_its_path_reaches`
   and `probe::tests::a_launch_is_told_the_volta_home_its_path_reaches`;
   the requirement and scenario in
   `openspec/specs/gate-boundary-policy/spec.md`.

2. **`VOLTA_HOME` is a locator, not configuration.** It names where an
   installed CLI is, not how it is configured, so carrying it leaves the
   probe's config isolation (#467) and the unboxed table's credential
   exclusion as they stand. No other name is added to either allow-list.

   **Enforcement binding:** the unboxed table's exact-set tests and the
   spec's "No other inherited name SHALL be carried".

3. **The workspace box is unchanged.** Its fixed `PATH` lists no volta
   shim, and `init` already names `VOLTA_HOME` for it.

   **Enforcement binding:** `box_environment`'s existing tests.

## Consequences

- A volta-installed CLI on the engine's `PATH` runs under the unboxed
  dispatch and the probe instead of recursing.
- A `PATH` that spells the shim directory differently from `HOME` (a
  symlink alias, `..`, a literal `~`) gets no `VOLTA_HOME`, and the
  recursion stays on such a host; a relative engine `VOLTA_HOME` is
  handed down verbatim to a child with a different working directory.
  Both are judgment the operator may revisit.
- The unboxed dispatch composes from the engine's environment as UTF-8
  strings (`engine/seat_tree.rs` collects `std::env::vars()`, which
  panics on a non-UTF-8 entry), so a non-UTF-8 `PATH` or `VOLTA_HOME`
  never reaches the resolver there; the probe hands OS strings. That
  limit predates this decision and is a follow-up of its own.
