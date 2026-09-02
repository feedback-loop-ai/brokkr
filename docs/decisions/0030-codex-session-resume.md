# 0030 — Codex session resume: the cache win and the sandbox it drops

Status: accepted (ruled 2026-09-02)
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

### Enacted — 2026-09-02, and what it measured

Wired as ruled. `Body::Resume` now leaves the engine ahead of the
`start` it belongs to; the codex arm spends it on
`codex exec resume --json -c sandbox_mode="<class>" … <thread> -`; and
the offer is made only to the same seat of the same run, resolving to
the same driver binary and the same candidate under the same pinned
bundle.

The before/after is a real two-attempt codex seat (`codex-cli 0.148.0`,
`codex` on PATH, seat argv `--sandbox read-only`), read off the driver's
own folded usage in the journal:

| attempt | launch | input | cache read | hit |
|---|---|---|---|---|
| 1 | cold `exec` | 14972 | 11008 | 73.5% |
| 2 | `exec resume`, same thread | 15956 | 14080 | **88.2%** |

The second attempt's `harness-started` checkpoint records
`launch: resumed`, the thread it rejoined, and `sandbox: read-only` —
the whole acceptance, in one journaled line. The first attempt sits on
the cold plateau this decision measured; the resumed one clears it by
fifteen points on its first resume, the same direction as the 92–96%
the Context table found on later ones.

No event payload widened to say any of this. Whether the engine offered
a session is a pure function of the journal and the pinned bundle, so
recording it would journal a derivation; what the driver did with the
offer is a fact, and the driver journals it in its own checkpoint. The
`event_schema: 1` payload vocabulary is untouched, and so is every
frozen contract — the wire already reserved `resume`.

Two of the Context's codex claims were re-verified against the installed
binary rather than inherited: `codex exec resume` still takes `-c` and
`--json` and still refuses `-C` and `-s` (`codex exec resume --help`),
and the class still does not travel by itself — a thread opened
`-s read-only`, asked on a bare resume to run `touch`, ran it and left
the file on disk, while the same thread under
`-c sandbox_mode="read-only"` answered `BLOCKED` and wrote nothing. That
is the whole reason ruling 2 exists, and it is still true.

One thing the ruling did not have to say, and the wiring found: a
decision-0016 chain fallback can never carry a session out of the effect
it happened in, because the fail-to-start predicate requires an attempt
that accepted nothing and checkpointed nothing — an attempt that opened
no session. The fallback suppression therefore only ever has to hold
across a re-entry, and it does: `chain_index` restarts per effect, so a
re-entered seat can resolve back to a link that never opened the thread
its neighbour did, and is handed nothing.

### Reforged — 2026-09-02: the argv narrowed, the machine added

A review of the enactment left two security residuals, both low, both on
paths that could hand a resume more power or a wrong session than it was
entitled to. The seat that could not run `codex` could not close them.
Both are closed now, against the installed binary.

**What may travel to a resume is now a list of what may, not a list of
what may not.** The class is re-imposed through `-c sandbox_mode=…`,
which the `--sandbox` flag outranks, so the guard against a competing
expression was load-bearing — and it was a substring search for
`sandbox`. Measured against 0.148.0: `--full-auto` does not exist in
this codex at all (the flag the review feared), and `--profile`,
`--add-dir`, `--approve-for-me` and `-C` are each rejected outright by
`codex exec resume`, so they fail closed rather than escalate. But two
holes the search could not have caught were real. `--last` and `--all`
ARE accepted by a resume and choose which session is rejoined — the
seat's own argv could have redirected an offer the engine made. And a
bare word in the seat's argv lands positionally, where
`codex exec resume [SESSION_ID] [PROMPT]` reads it as the session, ahead
of the thread the driver appends. The passthrough is now an allow-list
of seven value flags and four bare ones, each verified present on
`codex exec resume --help` and unable to reach the sandbox or the
session. Anything else — including a flag codex has not invented yet —
spawns cold with the offending part named in the checkpoint.

**A session handle no longer crosses a machine.** Decision 0027 made
runs portable and made an adopted run deliberately indistinguishable
from a native one *inside the chain*; a run exported mid-flight and
resumed elsewhere therefore agreed on every journaled fact the offer
rested on, and would have been handed a thread opened under another
machine's credential. Codex refuses such a thread today because its
rollouts are local, but the offer had already been made, and a driver
whose provider keeps sessions server-side would not be refused. Since no
comparison of journaled facts can answer this, the store answers it: an
`origin_host` column beside the chain — 0027's own pattern, written only
by `create_run`, left NULL by `import_run`, and absent from an export,
which carries events. `Store::started_here` gates the offer, and the
gate is closed for an adopted run, a copied journal, and an
installation that cannot identify itself. The fingerprint is a hash of
the machine id and the account's home, so the journal file holds an
equality token rather than an operator's hostname. Nothing was added to
any event: a start payload still reads `effect_id`, `attempt_id`,
`driver`, and the witness golden that pins that still passes untouched.

Two facts settled by measurement rather than argument while closing
these. A successful `codex exec resume --json` emits `thread.started`
carrying the same thread id it was handed, before any turn begins — so
the structural refusal predicate (non-zero exit, no thread ever
announced) cannot misfire on a rejoin that actually ran, and no seat is
billed twice for one attempt. And the numbers reproduce: an independent
cold/resume pair on 0.148.0 read 11008/14628 input tokens cached cold
(75.2%) against 14080/15404 resumed (**91.4%**) — the same sixteen-point
step the enactment measured, on a different pair of turns.

The offer also survives a park: an operator's `retry` is a second engine
process by definition, and it still rejoins, because everything the
offer rests on is durable. That is now pinned by a test that drives a
run to a park, retries it, and reads the wire.

**Known and bounded.** The engine's check is journaled facts plus one
local fingerprint. It cannot ask a provider who owns a session, so it
cannot see a re-pointed `CODEX_HOME` or a rotated token between two
attempts on one machine. `docs/guides/driver-authoring.md` now says
exactly that, and puts the last step where it can be taken: a driver
fails closed on a handle its own harness does not recognise. An adopted
run's codex seats also spawn cold for good — `origin_host` stays NULL
after an import — which costs cache and nothing else.

## Ruling — 2026-09-02, operator: resume is in; it is a cost win

Accepted. The measured gap (a ~75% cold plateau against 92–96% on a
resumed thread) is a cost the fleet pays on every codex seat, and the
safe spelling exists. The three questions above are answered so:

1. **A retry may resume, and so may a re-entered seat.** A second
   attempt of the same seat within a run resumes the thread its first
   attempt opened; a seat the phase machine sends back into (review →
   implement) resumes the thread it last held. A new run never resumes
   a thread from another run — the base has moved, and a thread that
   remembers a different tree is the wrong context, cheap or not.
2. **The sandbox travels, or the resume does not happen.** Every resume
   invocation re-expresses the seat's declared sandbox class as
   `-c sandbox_mode=<class>`. Where the driver cannot express the class
   on the resume path it spawns cold instead and records why in the
   checkpoint — ruling 2's fail-closed temperament applied at run
   time, never a silent escalation.
3. **The thread id travels in the protocol's own vocabulary.** The
   driver already folds `thread_id` into `session_meta`; the engine
   hands it back on the next attempt through `Body::Resume`, which
   forge-driver/v1 reserves for this, not through the seat's `input`.
   A resume that codex refuses (unknown or expired thread) is a cold
   spawn with the refusal journaled, not an attempt failure.
4. **Only the same adapter instance resumes the same session, and it
   resumes that session itself.** What is resumed is the provider's own
   session — the codex thread, the claude session, the dsh session —
   by the id the driver captured when it opened it: never a fork, a
   copy, or a new session seeded with the old transcript. It is resumed
   by the provider that opened it, through the same adapter declaration
   (the digest the run manifest pins) and the same driver binary, on
   the same seat of the same run — nothing else. A
   chain fallback to another model or provider (decision 0016), an
   adapter edit, or an engine upgrade between attempts spawns cold. A
   thread is one model's memory of one tree; handing it to anything
   else would be handing one model another's memory, and no cache win
   pays for that. It is also the providers' policy, not only ours: a
   session belongs to the credential and client that opened it, and
   resuming it from any other instance is a terms violation that gets
   the account blacklisted. Cost never outranks that.

Enacted by the machine as its own slice, with the before/after cache
figures recorded in the commit from the driver's own folded usage.
