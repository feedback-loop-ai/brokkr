# 0030 — Codex session resume: the cache win and the sandbox it drops

Status: proposed
Date: 2026-09-02

## Context

The 0021 addendum (operator ruled 2026-09-02) promoted `codex` to
`trusted` for every seat class and named three pieces of enabling
engineering it had not done. Two of them — the honest
`tool_permissions` declaration and the `models` map — are data edits
this slice landed. The third, session reuse, turns out to hide a
policy question, and this decision is where it goes rather than into
the driver.

Every claim below was produced by running the installed binary
(`codex-cli 0.148.0`, `/home/vyanakiev/.volta/bin/codex`) in the
implementer's own seat. Two prior attempts at this slice were denied
exec access by their sandbox and correctly declined to guess; this one
was not denied, so the guesses are replaced with measurements. Reading
`~/.codex/config.toml` remained outside the worktree-confined
filesystem and was NOT verified — nothing here depends on it.

### The mechanism exists

`codex exec` has a real resume surface:

    codex exec resume [OPTIONS] [SESSION_ID] [PROMPT]
    codex exec resume --last

`fold_codex_event` already captures the `thread_id` from
`thread.started` into `session_meta["session_id"]`, so the driver
already holds the only handle a resume could hang off. It has simply
never used it: the `AdapterKind::Codex` arm spawns
`codex exec --json -C <workdir>` cold on every attempt.

### The cache win is real, and it is large

Measured with the driver's own folded usage fields
(`input_tokens`, `cached_input_tokens` → `cache_read_tokens`), one
thread, trivial prompts, same workdir:

| run | invocation | input | cached | hit |
|-----|------------|-------|--------|-----|
| 1 | cold `exec` | 14631 | 11008 | 75.2% |
| 2 | first `resume` | 15642 | 11008 | 70.4% |
| 3 | cold `exec`, identical prompt | 14536 | 11008 | 75.7% |
| 4 | second `resume` | 15668 | 15104 | **96.4%** |
| 5 | third `resume` | 31645 | 29184 | **92.2%** |

A cold respawn plateaus at ~75% no matter how many times the identical
prefix is sent — run 3 proves the plateau is not a warm-up artefact.
Only the fixed system prefix is cached across processes; the seat's own
prompt is re-sent uncached every attempt. A resumed thread reaches
92–96%, which is the band LaneTally records for claude. This is the
first in-repo measurement of the effect; LaneTally's external ~30%
figure is neither confirmed nor contradicted here and is not cited as
if it were.

### The mechanism also drops the sandbox

`codex exec resume` accepts **neither** `-C/--cd` **nor**
`-s/--sandbox` — both are rejected as unexpected arguments. `-C` does
not matter (the driver already sets the child's `current_dir`). The
sandbox does, and it is not merely unpassable but **not inherited**:

- Cold `codex exec --sandbox read-only`, asked to write a file:
  replies `BLOCKED`, no file on disk. The class binds.
- `codex exec resume <that same thread>`, asked to write the same file:
  shell command runs, `exit_code 0`, file on disk.

So the naive wiring — carry the session id, drop the flags the resume
subcommand refuses — would take a seat that declared
`--sandbox read-only` and silently run its retry with write access.
That is precisely the "runs with MORE power than it declares" failure
`agents.rs` refuses at compile time, arriving at run time through the
back door, in exchange for a cache saving.

### There is a safe spelling

`-c/--config` IS accepted by `codex exec resume`, and the override
binds:

- `codex exec resume <id> -c sandbox_mode="read-only"`, asked to write:
  replies `BLOCKED`, no file on disk, and the turn still reads 89.0%
  cached.

So the restriction can be re-imposed on the resume path, and the cache
win survives it. The driver would have to translate its `--sandbox X`
passthrough into `-c sandbox_mode=X` for resume invocations only.

## The question this puts to the operator

Two things stand between the measurement and a wiring, and neither is
the implementer's to decide.

1. **There is no channel for a session id to reach the next attempt.**
   `run_attempt` consumes its `DriverProcess` and sends `Shutdown`, so
   the engine spawns one driver process per attempt and the adapter
   cannot remember anything across them. `Body::Resume { effect_id,
   attempt_id, session_ref }` exists in the protocol vocabulary, but the
   engine never sends it and the built-in adapters answer `supports:
   []`. Wiring resume means the engine holds a prior attempt's
   `session_ref` and hands it back — a control-plane change with
   capability-negotiation and journal-determinism consequences well past
   an adapter arm.

2. **A retry resuming the session that just failed may be wrong on
   purpose.** Brokkr retries an attempt after it failed or was killed on
   its deadline. Resuming carries the failed attempt's context — the
   confusion, the wrong turn, the half-edit — into the retry that was
   supposed to be a fresh look. The cache saving is real; so is the
   possibility that a cold retry is the more valuable one. This is a
   policy question about what a retry MEANS, not an optimisation.

The safe spelling above also has to be ruled on rather than assumed: a
translation layer from `--sandbox X` to `-c sandbox_mode=X` is a second
place where a restriction is expressed, and two places can drift.

## Proposed decision

Nothing is wired until this is ruled. The driver's codex arm keeps
spawning cold, and this slice records the measurement instead of
spending it. If the operator rules resume in, the ruling should say:

1. Whether a retry may resume, or only turns within one attempt.
2. That any resume invocation MUST re-express the seat's sandbox class
   via `-c sandbox_mode=<class>`, and that a resume path unable to
   re-express it refuses the attempt rather than proceeding — the
   fail-closed temperament of 0021 ruling 2, applied at run time.
3. Whether `session_ref` round-tripping happens through `Body::Resume`
   (the vocabulary already reserves it) or through the seat `input`.

## Consequences

Until ruled, codex seats keep paying the ~75% plateau, and the repo
holds a measured number for what that costs instead of an anecdote.
The `adapters/codex.json` gap declaration this slice landed names the
sandbox axis, so the next reader arrives at the same evidence without
re-running the binary.
