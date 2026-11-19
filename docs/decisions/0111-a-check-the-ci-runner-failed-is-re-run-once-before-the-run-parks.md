# 0111 — A check the CI runner failed, not the change, is re-run once before the run parks

Status: proposed
Date: 2026-11-18

Built: unbuilt (#1093)

## Context

Issue #1093, under the adopter-findings epic #1094 (delivery cost and recovery), carries an
adopter's figures, and they are theirs: on brokkr 0.12.0 with GitHub-hosted runners, three parks
in one week, each on a runner fault. In one, a compose service could not bind its host port
(`address already in use`), so the job's tests never ran. In another, a timing-sensitive test
failed once and passed on re-run. Each, they report, cost a seat attempt to diagnose, plus the
wall clock until a person re-ran the jobs. Their exact failing steps, those steps' names and
their log excerpts were not observed here. Every claim below about them is **unmeasured**.

The house has its own instance. #1113 is RustSec's `update-a-check-run` "Not Found" transient on a
pull request, fixed by a re-run. Another session holds it; this decision names it as an instance
and designs no fix for it.

Decision 0106 (accepted 2026-11-16) gave every `run/parked` a closed `cause` under
`park-cause/v1`, and 0106a built it: `ruled`, `attempt-cap`, `infrastructure` (with `retryable`,
true only on positive non-execution evidence) and `provider-limit`
(`crates/brokkr-core/src/park.rs:32-59`). Its slice 0106d, unbuilt, is the dispatcher's declared
retry budget, spent only on an `infrastructure` park with that evidence
(`docs/decisions/0106-a-park-names-its-cause-a-held-wait-and-a-budget.md:647-699`, `:1212-1226`).
This decision builds on both and weakens neither.

**The issue asks for a re-run, and this decision builds none.** The heading keeps the issue's
ask, because a decision's heading is never edited; what the decision rules is narrower. A re-run
is a forge write, and the forge cannot confine one. GitHub's two re-run verbs,
`POST /repos/{owner}/{repo}/actions/jobs/{job_id}/rerun` and
`POST /repos/{owner}/{repo}/actions/runs/{run_id}/rerun-failed-jobs`
(<https://docs.github.com/en/rest/actions/workflow-runs>), take only `enable_debug_logging` in
their body, and the job form also `enable_debugger`. Neither takes an attempt number, an
expected state or an `If-Match` precondition, and each re-runs the named jobs "and its dependent
jobs". So no request can be confined, at the moment it is sent, to the attempt and the closure
the engine classified. The controller read that page on 2026-11-18; this seat holds no web
capability and did not, and the forge's live behavior is **unmeasured**. Fix round 1093r2's
review held as a security risk a design that sent the request anyway, and on 2026-11-18 the
operator ruled ruling 1 (a): Brokkr observes, classifies and parks, and a person re-runs.

What this decision holds fixed: 0071's rules (one home per rule, a closed vocabulary matched
without a wildcard, observe before decide, every guard bound by a test); 0082 (a contract version
is minted only under an accepted decision, so every version below is proposed and none is
minted); no model decides a routing, and a seat's "no code change applies" is prose, not a
classifier; 0106's cause and budget stand; 0051's landing and the landing recipe stand; and 0063:
Linux and macOS only.

### What #1093's acceptance becomes

Narrowed by ruling 1 (a), the issue's acceptance is this: **a port-bind failure during service
boot parks with cause `ci_infrastructure` and no diagnosis seat attempt.** The observation is
itself an exec attempt: the engine journals `effect/started` with an attempt id before the `ci`
gate's driver starts (`crates/brokkr-runtime/src/engine/dispatch.rs:147-165`), and ruling 8 says
that attempt executed. What the park saves is the model seat the adopter counted, the one that
diagnoses; a person still re-runs. The engine draws the class
`ci-infrastructure` (ruling 3), the table matches it as the flag `ci_infrastructure` (ruling 10),
and the park is written `infrastructure` with `kind: ci-infrastructure` (ruling 7). These are one
fact in three places, each in the spelling of the surface it lives on.

What is not delivered, and why:

- **The re-run once.** No forge write is sent, because none can be confined to the authorized
  attempt and closure when it is sent (<https://docs.github.com/en/rest/actions/workflow-runs>).
  A person re-runs.
- **The wall clock.** The wait until a person re-runs remains. What is gone is the diagnosing
  seat attempt the issue counts.
- **The timing-sensitive test.** It stays the change's, as today. Flaky-test handling is a
  follow-up (ruling 4).

### The gate grants nothing

This is the frame every ruling below is read in, and this is its one home.

**No class the `ci` gate draws lets anything reach a later phase, or a merge, that landing
without the gate would not.** `green` proceeds to `classify`, exactly as landing's own entry
does today (`recipes/landing/bundle.json:7-10` overrides `initial`; `recipes/landing/README.md:5`).
No later rule, seat, script or check reads the gate's result as evidence of CI's verdict. The
forge's required checks at merge stay the only CI authority, unchanged.

Why that holds at HEAD:

- landing reads no forge check ([below](#nothing-in-the-engine-or-the-house-recipes-reads-a-forge-check));
- its ship seat merges nothing and pushes nothing ("never commit or push",
  `recipes/fast/scripts/ship-seat.sh:2`); it leaves a clean tree, the gated head, the ledger and
  the anchor (`recipes/landing/README.md:35`) for a person to "review, push, and merge next"
  (`ship-seat.sh:73`, `:91`);
- "the rest of the checks main requires stay CI's" (`recipes/landing/README.md:9-11`), and the
  forge enforces them at merge (`.github/branch-protection.json:5-20`);
- the contribution gate reads, of a run's journal, only its `ship` decision and a preflight's
  `review` decision (`scripts/delivered-by-brokkr.sh:197-200`, `:265-267`), never a gate's
  inputs, so a `ci` decision in a shipped run's journal moves no tier.

So `green` is not a verdict. It is the absence of the gate's two new outcomes, a typed park for
a person and one bounded return to `implement`. Every proof the gate takes below is therefore a
precision guard on the `ci-infrastructure` class, and each fails closed to "not
infrastructure" (ruling 3).

**Every outcome of a wrong or forged observation is bounded:**

| A wrong … | Costs |
|---|---|
| `green` | landing as it is today: review, ship, and the forge's required checks at merge |
| `ci-infrastructure` | a typed park a person reads before re-running |
| `change` | one implement visit, under `fast`'s existing implement bound (ruling 6) |
| `refused` | a `ruled` park a person reads |

**What this is not.** A change that weakens CI's own verdict, such as `deny.toml` widened to
admit a licence, is green on the forge as well. That is landing's existing exposure, and review
and the forge hold it, unchanged. It is not a residual of this gate, and this decision does not
claim to close it.

**Enforcement binding (0111e):**
`after_ci_pass_the_composed_table_is_landings_own_rule_for_rule` (the composed `landing-ci`
table, less its four `CI-` rules, its `initial` and its `ci` phase, equals `landing`'s composed
table rule for rule); and `nothing_in_the_recipes_or_seat_scripts_reads_the_ci_phases_result`
(a search of every recipe's bundle, policy and scripts, and of `scripts/`, for `ci_class`, the
three class flags and a `from: ci` decision finds only the four `CI-` rules).

## What main holds at HEAD

Read at `e667a04c2`.

### Nothing in the engine or the house recipes reads a forge check

Two searches were run over the tree. The first,
`grep -rnE "gh (pr checks|run (rerun|view|watch|list))|statusCheckRollup|check-runs|check_runs|checks --watch|mergeStateStatus|--required" recipes crates scripts`,
printed only `recipes/preflight/README.md:48` and
tool-grant strings in tests (`crates/brokkr-runtime/src/agents/tests.rs:4219`,
`crates/brokkr-runtime/tests/library_data.rs:745`, `crates/brokkr-runtime/tests/capability_launch.rs:5747`,
`crates/brokkr-protocol/src/native_controls/tests.rs:6434`). The second, for any `gh` subcommand in
recipe scripts, recipe data, agents and adapters, printed only the two tool grants
`gh-pr-view` and `gh-run-view` (`adapters/claude.json:69-70`, `adapters/lanetally.json:67-68`)
and the agent that hires them (`agents/verify-reviewer.json:7`). A search of non-test Rust under
`crates/` for `"gh"` as a command, `api.github.com`, `/check-runs` or `/actions/runs` printed no
reader.

`recipes/preflight/README.md:48` is one table row with two cells. Preflight's cell says it has
no `gh`. The other cell says the `verify` recipe's driver tools include `gh pr view` and
`gh run view`. Those are the grants of the reviewer `verify` hires: its `review` seat names the
agent `verify-reviewer` (`recipes/verify/bundle.json:43`). A grant is not an observed use.
Whether any run's reviewer called either tool, and what it read, was not looked for in any
journal here: **unmeasured**.

So Brokkr reads no check. What a model seat may do is call `gh run view` under its grant and
write prose about it; nothing types what it read.

No house recipe has a CI phase. `landing`'s phases are `classify` → `verify` → `review` →
`ship` → `done`, and "the rest of the checks main requires stay CI's"
(`recipes/landing/README.md:5-11`). That is an absence of a CI reader, not an ordering. CI runs
on a pull request's `opened`, `synchronize`, `reopened`, `labeled` and `unlabeled` events
(`.github/workflows/ci.yml:3-17`), so on a pull request already open, CI can start before a
landing run does, and run beside it. When a person or the merge queue reads a red check, and
which external actors take part, is **unmeasured**. `fast`'s two exec seats, the verifier and the
shipper, run with `"network": false` (`recipes/fast/bundle.json:46`, `:91`). #1113 is a red check
read outside any run.

### What a failed check becomes, and through which rule it parks

For Brokkr itself: nothing, because no run observes it.

For the adopter: unmeasured. Their 0.12.0 recipe is not in this tree, so whether they read checks
through an exec gate of their own or a model seat with `gh run view` is not observed. In a table
shaped like `fast`'s, the two roads a red gate can take are these:

- the gate returns `fail`, and `VERIFY-FAIL` returns it to `implement`
  (`recipes/fast/policy.json:87-94`), spending an implement visit to diagnose; after three visits
  `VERIFY-FAIL-EXHAUSTED` stops the run (`:76-86`);
- the implementer, finding nothing to change, returns `blocked` with `blocked_by: environment`,
  and `IMPL-BLOCKED-ENVIRONMENT` parks (`:39-50`, decision 0092). `blocked_by` is a seat-owned
  typed choice (`crates/brokkr-core/src/policy/inputs.rs:106-115`), matched by `blocked_by_in`
  (`recipes/fast/policy.json:39-57`). It is not prose: a result that names the environment in
  its notes but carries no `blocked_by` input does not match `IMPL-BLOCKED-ENVIRONMENT`, and falls
  to `IMPL-BLOCKED`'s stop. What is the seat's own is the judgment behind the typed word: no
  forge fact backs it.

A rule park is written `ruled` with the matched rule id
(`crates/brokkr-runtime/src/engine/cursor.rs:362-369`, `:386`). No writer at HEAD passes
`retryable: true`: `grep -rn "infrastructure(true" crates --include=*.rs`, filtered to non-test
lines, counted 0.

### What the journal records of it

The gate's result and notes, sealed as its seat record, and the decision's seven fields
(`crates/brokkr-core/src/decision.rs:86`), whose `inputs` carry engine- and seat-owned inputs; the
park's reason and its `ruled` cause. Nothing typed names a job, a step or a forge run.

### The forge's shape

This shell has no forge client and this seat holds no web capability, so the forge's shape below
is from GitHub's public documentation by URL and was **not read in this session: unmeasured
until slice 0111a records real payloads.**

- A workflow run's jobs, each with `id`, `name`, `status`, `conclusion` and `steps[]` of
  `{number, name, status, conclusion}`:
  <https://docs.github.com/en/rest/actions/workflow-jobs>.
- A job's log: <https://docs.github.com/en/rest/actions/workflow-jobs#download-job-logs-for-a-workflow-run>.
- Check runs and their conclusions: <https://docs.github.com/en/rest/checks/runs>.
- The re-run verbs, which this decision never calls (ruling 1):
  <https://docs.github.com/en/rest/actions/workflow-runs>.

Unmeasured in particular: the closed list of `conclusion` words (the documentation lists
`success`, `failure`, `neutral`, `cancelled`, `skipped`, `timed_out`, `action_required`, and for a
run `startup_failure`); how a job log is cut into steps; and whether the API says who cancelled a
job. **A step has no `kind` field**: whether a step is setup, a service or a test is not a forge
fact, so any step role below is recipe data.

## The rulings

Each ruling gives its options, what each costs, refuses and changes, and one recommended option.
Ruling 1 is the operator's, ruled on 2026-11-18. For every other ruling the operator may answer
"accept all" or pick.

### 1. Brokkr makes no forge write

**Ruled by the operator on 2026-11-18: (a).** The operator answered this choice in chat with
"a for 1093". Accepting the rest of this decision is still the operator's.

- **(a) Ruled by the operator: observe, classify and park.** The engine and the recipes observe
  CI, classify what they observed, and park a runner fault with typed evidence and no
  diagnosis seat attempt. They send no re-run request and hold no write credential. A typed
  `ci_infrastructure` park carries the evidence, and a person reads it and re-runs.
  - Cost: the issue's "re-run once" is not built. The wall-clock wait for a person remains, but
    the diagnosing seat attempt the issue counts is gone.
  - It refuses every forge write. A mistyped failure costs a person's look, never a pass or a
    re-run (ruling 4).
- **(b) An automated re-run on GitHub, as the forge offers it.** The engine would send one of
  the two verbs above. The residual, stated plainly:
  - **No attempt precondition.** A request names a job or a run, never the attempt the engine
    classified. A re-run another actor starts between the engine's read and its write is re-run
    or widened again, and the forge cannot be asked to refuse it.
  - **Dependents re-run, an audit among them.** Each verb re-runs "and its dependent jobs". A
    dependent that passed, or an audit job, re-runs with the target, and its new result replaces
    the one the engine classified.
  - **A concurrent person's re-run cannot be told apart.** A new attempt carries no mark of which
    request made it, so the engine cannot attribute a new attempt to its own request.
  - **A lost receipt is indeterminate.** A request the forge accepted while the effect died before
    recording it cannot be told from one never sent. Sending again may double it; not sending may
    leave none.

  The operator did not take it.
- **(c) An automated re-run only on a forge that gives an attempt-bound, closure-bound write.**
  That is a forge whose re-run takes the attempt, refuses when the attempt moved, and re-runs
  exactly the named jobs. None is measured. It is the follow-up decision this one names, and that
  decision would rule the durable intent, its reconciliation and the scope rules a write needs.

**What leaves with the write.** Fix round 1093r2 drafted them, and each answered a finding that
held only because a write was sent:

- the credentialed writer seat and its write token;
- the durable intent and its reconciliation;
- the multi-target re-run rounds and the per-head re-run count;
- the re-run routes, and the classes and park kinds only they drew;
- the rule that let an audit job be re-run.

**Enforcement binding:** 0111d's overlay test refuses any seat in the CI overlay that declares a
write binding, and the observer's credential is read-only (ruling 2).

### 2. What reads the checks

**Recommended: (a) a recipe-declared exec gate observes, and the engine classifies.**

- **(a) An exec gate, `ci`, observes; core classifies; the table routes.** The gate runs
  `brokkr driver exec` over a script, as `fast`'s verifier does, with no model. It waits for the
  gated head's checks to settle and writes what the forge said as a typed observation (ruling 3).
  The engine classifies that observation with a pure core function (ruling 4) and the table
  routes on the engine-owned result. Observing and deciding stay apart (0071 ruling 10). Cost: one
  script, one core module, one engine input. It refuses forge-specific network code in the
  engine. It serves an adopter's exec gate and a future house one through the same contract.
- **(b) A built-in engine effect that calls the forge.** One reader for every recipe, and no
  child process whose launch must be proven. But the engine holds no forge client at HEAD: its
  one HTTP client, `ureq`, is in `brokkr-bridge`, which speaks to Looper, not to a forge
  (`crates/brokkr-bridge/src/lib.rs:1-7`, `:210-219`). So (b) builds an HTTP client, a
  credential path and a forge vocabulary into the runtime, a second effect kind for one forge.
  Not recommended now; the observation contract in (a) is what (b) would write, so (b) stays
  open, as a slice of its own if ruled.
- **(c) A model seat reads `gh run view` and says.** Refused: a model's reading is prose, and
  no model decides a routing.

**Enforcement binding:** the bundle compile refuses a `ci` declaration (ruling 5) on a seat
whose adapter is not `exec`.

**The observer is pinned outside the change.** What the observer runs, and what it resolves,
must not be anything the gated change can edit:

- **Its script.** A `./`-prefixed driver entry is bundle-relative, not worktree-relative
  (`crates/brokkr-runtime/src/bundle/expand.rs:12-13`, `:21-33`), and the compile's walk pins
  the script it admits into the manifest's `files`
  (`crates/brokkr-runtime/src/bundle/model_policy_tests.rs:2598-2622`). So `fast`'s
  `./scripts/verify-seat.sh` (`recipes/fast/bundle.json:33-41`) is the bundle layer's copy,
  digested at compile. It is the change's only when the operator compiles the bundle from the
  change's own tree; in this house `recipes/fast/` is repository code. This decision therefore
  requires that the observer's layer lie outside the gated worktree: run start refuses a `ci`
  seat whose bundle layer is inside it. Whether run start can see both paths today is
  unmeasured; slice 0111c's test settles it.
- **Its launch.** Checking where `gh` resolves from inside the script proves nothing about the
  process that runs the script. An inherited `BASH_ENV` naming a worktree file runs before the
  script's first line, and `bash` itself resolves through `PATH`, which 0049 ruling 3 leaves
  unpinned under `harness` and `open` (quoted at
  `crates/brokkr-runtime/src/bundle/model_policy_tests.rs:2595-2596`). So the launch is ruled
  below, and the engine proves it before it spawns anything.

**The observer's launch is trusted, or the observation refuses.** **Recommended: (a) an
engine-built launch, proven before the spawn.** The process that observes CI, read-only as it
is, is launched with all four of these, or the engine refuses before the spawn, parks `ruled`
with the reason `launch-untrusted` naming the condition that failed, and observes nothing:

- **A pinned executable identity outside the change, executed as the bytes it verified.** The
  `ci` seat's interpreter and each tool it calls (`gh`, `jq`) are declared in ruling 5's file by
  absolute path and SHA-256 digest. The argv's first word is the pinned interpreter, never a
  bare name, and it runs as `bash --noprofile --norc` over the pinned script. The tools reach the
  script as absolute paths in its argv, never through `PATH`. Hashing a path and then executing
  by that path is not enough: HEAD starts every subprocess by path
  (`crates/brokkr-protocol/src/adapters/exec.rs:33-42`, `adapters.rs:756-770`, `:839-845`), and
  a directory outside the worktree that another seat under the same account can write may change
  between the two. So the rules below bind what runs to what was hashed.
- **A working directory outside the worktree.** The launcher creates an empty directory of its
  own, outside every worktree, runs the observer there, and removes it after. No git
  configuration, `.envrc` or tool file in the worktree is beneath it.
- **A cleared, allow-listed environment.** The environment is cleared, then holds only a closed
  list: the read-only token's one variable, `HOME`, `XDG_CONFIG_HOME` and `GH_CONFIG_DIR` set to
  empty directories the launcher made, `LC_ALL=C`, and an empty `PATH`. There is no `BASH_ENV`,
  no `ENV` and no shell startup file. The compile refuses any other key.
- **No tool configuration the change controls.** `gh` reads its configuration only from the
  launcher's empty directory, and is told the repository by `--repo` from the pinned list, so it
  never reads the worktree's remotes. Nothing the observer opens is under a worktree.

**The observer executes the bytes it verified.** **Recommended: (a) open once, verify the open
file, execute that file; and no path anyone but root can change.**

- **No path that resolves into a worktree, and no symlink.** The launcher resolves each pinned
  path with every symlink followed, and refuses when the resolved path differs from the declared
  one, or lies under any worktree the run holds. So an absolute path that is a link into the
  worktree refuses, as does a directory in its chain that is one.
- **No path a seat can change.** The file and every ancestor directory must be owned by root
  and writable by no group and no other, so no process under the run's account, another
  unboxed seat among them, can replace an entry between verification and use. A user-installed
  tool (under `~/.local/bin` or a Homebrew prefix the user owns) refuses. This is the one rule
  that holds the tools `bash` itself later starts by their absolute paths.
- **The interpreter runs from the file descriptor it hashed.** The launcher opens the
  interpreter once, with no symlink followed, hashes the bytes read from that descriptor, and
  executes that same descriptor. On Linux that is `execveat` with `AT_EMPTY_PATH`, or `fexecve`
  (<https://man7.org/linux/man-pages/man2/execveat.2.html>; not read in this session). HEAD's
  spawn names its program to `Command::new` (`crates/brokkr-protocol/src/process.rs:295-296`).
  It already runs `pre_exec` hooks (`crates/brokkr-protocol/src/process/attempts.rs:129`) and
  hands descriptors into a namespace (`crates/brokkr-protocol/src/hands/namespace/workspace.rs:240`),
  but no site executes a descriptor, so 0111c builds that.
- **The script never passes through a file.** The engine reads the pinned script from the
  compiled bundle once, checks its digest against the manifest's, and hands those bytes to the
  interpreter over a pipe it owns, as `bash --noprofile --norc /dev/fd/3`. Nothing outside the
  engine holds the pipe's other end, so no one can change the script after it is checked.
- **macOS.** Darwin offers no `execveat`, and whether its `fexecve` is usable is **unmeasured**
  here. So on macOS the interpreter is executed by its path, and the root-owned chain above is
  what holds it; the pipe holds the script on both systems. Where the chain does not hold, the
  launch refuses; it never falls back to an unchecked path.

  The alternatives: **(b)** copy the verified bytes into an engine-owned 0700 directory outside
  the worktree and execute from there. Not recommended: a 0700 directory still admits every
  process under the same account, which is the case this rule exists for, so it shortens the
  window and does not close it. **(c)** the engine makes the read-only forge call itself (ruling
  2 (b)): no child process, so no executable to bind, no launch to prove and no handoff. Its
  dependency cost is an HTTP client with TLS, a credential path and a GitHub vocabulary in the
  runtime: the one client HEAD holds is `ureq` in `brokkr-bridge`, which speaks to Looper
  (`crates/brokkr-bridge/src/lib.rs:1-7`, `:210-219`). It stays open as that option's slice.

**The result comes back through the engine, never the worktree.** HEAD hands every seat a result
path under the run-local results directory (`crates/brokkr-runtime/src/engine/input.rs:69-82`),
which is the folder's state or, by default, `<workdir>/.forge`
(`crates/brokkr-runtime/src/engine.rs:1047-1050`, `crates/brokkr-runtime/src/places.rs:687-690`).
In the default form that is under the worktree, where the change and every other seat can write.
So a `ci` seat's observation is never read from a file. The engine captures the observer's
stdout through its own pipe, bounded at 1 MiB, and an observer that writes more refuses as
`observation-too-large`. HEAD's exec adapter already captures stdout into its invocation
(`crates/brokkr-protocol/src/adapters/exec.rs:42`, `:50`, `:71`), but decodes it lossily; the
`ci` path parses the raw bytes, and refuses bytes that are not UTF-8. A file under `.forge/results`
for a `ci` seat is ignored, and its presence is journaled as a refusal word, `result-file-ignored`,
never read. The observer writes nothing to stderr but its own diagnostics, and the `ci` site
journals stderr's length and digest only.

**Live admission is refused until the launcher exists.** 0111b builds the classifier and the
compile's `ci` declaration, but its compile refuses every bundle that declares a `ci` seat, a
`ci_observation` input or a `ci-checks.json`, a custom exec recipe's included, with the reason
that the trusted launch is unbuilt. 0111c lifts that refusal in the same change that lands the
launcher. 0111b's tests drive the classifier through builders and test-only bundles, never a
live compile.

What HEAD lacks for this form, each built by slice 0111c:

- **A cleared environment for a site without hands.** Such a site spawns in the engine's own
  environment: `SiteSpawn::inherit` sets `SpawnEnv::Inherit`
  (`crates/brokkr-runtime/src/engine.rs:2040-2050`), and `harness_command` adds the bindings
  without clearing what it inherited (`crates/brokkr-protocol/src/adapters.rs:756-770`). The
  cleared form exists: `SpawnEnv::Exactly` (`crates/brokkr-protocol/src/process.rs:85-94`,
  applied with `env_clear` at `:303-305`). Today only an unboxed exec dispatch with hands uses
  it (`crates/brokkr-runtime/src/engine.rs:2871-2880`).
- **A working directory other than the worktree.** The exec adapter runs its command from the
  workdir it is handed (`crates/brokkr-protocol/src/adapters/exec.rs:33-42`), and no site
  declares another.
- **A pinned executable identity.** None exists for any exec site (0049 ruling 3).
- **Execution of the verified bytes.** No site executes a descriptor or feeds a script over a
  pipe, and none checks a root-owned chain.
- **A result channel the engine owns.** Every seat's result is a file under the run-local
  results directory, by default the worktree's `.forge` (above).

The other forms:

- **(b) The engine observes through a forge client.** It has no child process to prove, but
  HEAD holds no forge client (ruling 2 (b)), so it is that option's cost.
- **(c) A boxed observer under `namespace`.** The box clears the environment and starts from `/`
  (`crates/brokkr-protocol/src/hands.rs:764`), and supplies the filesystem and `PATH` (0049,
  `docs/decisions/0049-the-platform-classes.md:43-46`). But HEAD refuses any hands site with a
  secret binding (`crates/brokkr-runtime/src/bundle.rs:3940-3967`), so a boxed observer could
  read only with no credential: a public repository, under the forge's unauthenticated rate
  limit (unmeasured), and on Linux only. Not recommended.

**The observer's credential is read-only.** A forged observation reaches no further than the
bounded table in [The gate grants nothing](#the-gate-grants-nothing): a forged `green` is landing
as it is today. What the launch rules guard is the precision of `ci-infrastructure` and the
read-only token itself. The
credential is a sealed secret binding (0012), resolved by the runner into the one allow-listed
variable and never into argv; `gh` reads a token from its environment, an env-only consumer
0012's compile lint already admits (`crates/brokkr-runtime/src/bundle.rs:4149-4156`). Its scope
is read-only: Actions, checks, commit statuses, pull requests and metadata, and administration
only where ruling 3's forge-required set is read. The forge's permission names, and whether
Brokkr can read a token's scopes, are unmeasured; so a token with write scopes is the operator's
misconfiguration, and the observer never sends a write whatever its token holds.
- **Its word is data.** The engine re-checks the observation's binding itself (ruling 3): it
  compares the head to its own git head and the repository to the pinned one. It never takes
  them from the observer.

### 3. The observation, the classification, and where the journal keeps them

**Recommended: (a) two inputs, owned apart.**

- **The observation is the seat's.** It is a seat-owned typed record input,
  `ci_observation`, under `ci-observation/v1`, which the `ci` seat declares in its `inputs`.
- **The classification is the engine's alone.** It is an engine-owned record input,
  `ci_class`, under `ci-classification/v1`, written on the gate's `transition/decided`.

They must be two inputs. The first draft made one `ci` input engine-owned and had the seat
write it, which HEAD refuses twice over. A seat that declares an engine-owned input fails the
compile (`crates/brokkr-runtime/src/bundle/assemble.rs:327-332`). And a seat's payload loses
every engine-owned key, and every undeclared one, before the table sees it
(`crates/brokkr-runtime/src/engine/decide.rs:73-82`). So the observation must be seat-owned to
reach the engine at all, and the classification engine-owned so no seat can reach it. A
seat-owned **record** input is new: every record input in the registry today is engine-owned
(`crates/brokkr-core/src/policy/inputs.rs:116-118`). Slice 0111b adds the first, parsed at
decide under its contract and refused whole on any unknown field.

The observation names its binding:

- the forge (`github`, the one word v1 admits);
- the repository, by owner, name and numeric id;
- the pull request's number, and its base commit's full id;
- the pull request's head commit as the forge reports it
  (<https://docs.github.com/en/rest/pulls/pulls#get-a-pull-request>, its `head.sha`), which must
  be the engine's own git head, or the head is unpublished (ruling 6);
- the gated head's full commit id.

It then lists every check the forge reports for that head, with no filtering. Checks outside
Actions are included:

- a check run carries its id, its app (slug and numeric id), its name, its `head_sha`, its
  status and its conclusion
  (<https://docs.github.com/en/rest/checks/runs#list-check-runs-for-a-git-reference>);
- a check from Actions also carries its producer: its workflow run's id, number and attempt, its
  workflow file's path, its triggering event, the revision the run tested, and its job's id,
  name and steps (<https://docs.github.com/en/rest/actions/workflow-runs>,
  <https://docs.github.com/en/rest/actions/workflow-jobs>);
- every workflow run the forge lists for the head, with its path, event, number, attempt,
  status, `head_sha` and its pull-request association (below), so the engine can tell which
  run is current and whose it is, and every job of each current run with its steps, so a
  required aggregate's dependencies are seen (ruling 4);
- every open pull request the forge lists for the head commit
  (<https://docs.github.com/en/rest/commits/commits#list-pull-requests-associated-with-a-commit>),
  so a head two pull requests share is seen;
- a commit status carries its context, its state and its creator's login and numeric id
  (<https://docs.github.com/en/rest/commits/statuses#get-the-combined-status-for-a-specific-reference>),
  so it can be refused by name; it is never admitted as required evidence (below).

It is parsed once at the edge, with unknown fields refused (0071 ruling 3). An unknown
conclusion word refuses by name. Whether the forge reports a workflow job's key, as the workflow
file spells it, is unmeasured until 0111a's evidence; if it does, the observation carries it.

**Every admitted execution is this pull request's.** HEAD's workflow keys its concurrency by
ref (`.github/workflows/ci.yml:19-27`) and reads pull-request-specific data, the body and the
labels, into a required job (`:78-102`, where the `by-hand` label changes its outcome). So two
pull requests that share a head can each produce a run on it, with different results, and a run
keyed only by workflow path, event and head may be the other one's. **Recommended: (a) an
Actions execution is admitted only when the forge positively associates it with this pull
request.** The field is the workflow run's `pull_requests` array, each element naming a pull
request's `number` and its `head` and `base` with `ref` and `sha`
(<https://docs.github.com/en/rest/actions/workflow-runs#get-a-workflow-run>); a check run
carries the same array (<https://docs.github.com/en/rest/checks/runs#get-a-check-run>). The
field is named from that documentation; this seat read neither page, so its live contents are
**unmeasured**. In particular, whether it is empty for a run from a fork, or for any other case,
is unmeasured until 0111a's evidence, and an empty association is never read as this pull
request's. An execution is admitted only when:

- its `pull_requests` names this pull request's number;
- the element's `head.sha` equals the gated head, and its `base.sha` the base the observation
  names;
- where the forge lists more than one open pull request for the head, the execution's
  association names this one and no other.

Otherwise the entry has no admitted execution, and the observation refuses with
`association-unproven`, naming the run, or, where two open pull requests share the head and an
execution's association cannot tell them apart, `head-shared`. Refused alternatives: **(b)**
matching by workflow path, event and head alone, which is fix round 1093r3's key and admits the
other pull request's run; and **(c)** matching by the merge commit's parents, which cannot tell
two pull requests with the same base and head apart.

**Commit statuses are not admitted as required evidence.** A status names a context, a state
and a creator, and attests no revision: nothing in it says what its producer tested
(<https://docs.github.com/en/rest/commits/statuses>). HEAD's required checks are all Actions
jobs: each of the twelve contexts `.github/branch-protection.json:7-20` records (observed
2026-09-26, its `:3`) is a job's display name in `.github/workflows/ci.yml` or, for
`mutants in the diff: brokkr-core`, in `.github/workflows/mutants.yml:42`. So `required_checks` admits no `status` entry: the compile refuses one,
and where the forge's own required set names a status context, the observation refuses with
`status-required`, for a person. Admitting statuses is a follow-up that needs a revision
attestation the forge does not give a status.

**The required set.** **Recommended: (a) a recipe-declared `required_checks` list, pinned by
the compiled bundle outside the change, each entry bound to its producer.**

- **(a) Recipe-declared.** `required_checks` lives in ruling 5's file, which the operator's
  compile pins. It must be non-empty, or the compile refuses. Each entry names its producer, and
  a check matches it only when every part of that producer matches:
  - **an Actions job** is bound to its app, its workflow file's path, its job (the key the
    workflow declares, and the display name the forge reports for it), and its triggering event.
    For this house the audit entry is job `dependency-audit` in `.github/workflows/ci.yml`,
    displayed `RustSec dependency audit` (`:673-674`), from `github-actions`, on `pull_request`.
    The required test jobs are named by system: `test (ubuntu-latest)` and
    `test (macos-latest)`, job `engine` over its matrix (`:315-322`). Each is its own entry.
  - **No commit status** is an entry, as above.

  Where the observer can also read the forge's own required set for the base branch
  (branch protection, <https://docs.github.com/en/rest/branches/branch-protection>, or rulesets,
  <https://docs.github.com/en/rest/repos/rules>), every check the forge requires must be in the
  declared list, or the observation refuses with `required-set-disagrees`. The forge's set can
  only widen what a pass needs, never narrow it. The token scope that read needs is unmeasured;
  where it is unread, the observation records `forge_required: unread`, and the pinned list
  alone governs. Cost: one more list for the operator to keep.
- **(b) The forge's branch protection alone.** It is outside the change, but a repository
  administrator can change it at any time, the read needs an administration scope (unmeasured),
  and an unprotected branch yields an empty set, which must refuse. So an adopter without
  protection cannot use the gate. Not recommended.
- **(c) Whatever checks the forge reports for the head.** Refused: a missing check is invisible
  to it, so an absent audit would pass. That is the fault this ruling closes.

**A change that touches CI's machinery draws no `ci-infrastructure`.** A change to CI can keep a
required check's name while substituting what produces its result, and it need not touch the
workflow to do it. HEAD's required `test (…)` verdicts are an aggregate: job `engine` runs one
assertion over `needs.engine-machines.result` (`.github/workflows/ci.yml:315-329`), and
`engine-machines` runs `scripts/test-shards.sh` (`:255`). That script sources
`scripts/test-summary.sh` (`scripts/test-shards.sh:444-445`), and runs `scripts/parallel-heaps.py`
and `scripts/shard-runner.py`, consuming their status (`:553-602`). And a verdict need not be
executed to be moved: the required `dependency licenses (cargo-deny)` job
(`.github/branch-protection.json:14`) applies `deny.toml`'s policy (`deny.toml:1-5`;
`.github/workflows/ci.yml:559-573`), and `non-Rust lints` runs `typos`, which reads
`_typos.toml` (`scripts/lint-non-rust.sh:17-24`). Under the frame such a change buys nothing a
`green` buys ([The gate grants nothing](#the-gate-grants-nothing)); what it could buy is a
failure of its own dressed as a setup fault. So the guard is a precision guard on that one class.

**Fix round 1093r4's machinery park is revised.** r4 refused every machinery change as
`machinery-changed` and parked it `ruled` for a person. That park is removed, and with it r4's
stated cost, "every change to `scripts/` or `quality/` now gets a person's pass": no person's
pass is spent on a machinery change. **A change touching the machinery never draws
`ci-infrastructure`: a red observation is `change`, and an all-success one is `green`, which
proceeds** as the frame says. The withholding is named `machinery-touched`, with the paths, in
the class's sealed reason (ruling 4).

**Recommended: (a) the set fails closed: `ci-checks.json` declares the change classes, and
every other path is machinery.** `change_classes` (ruling 5) lists the repository paths whose
change is the change under test. Every other changed path is machinery, `deny.toml`,
`_typos.toml` and any root configuration among them, a new one the change adds included. For
this house, from `git ls-tree --name-only e667a04c2`, the story's base (39 entries: nineteen
change classes and twenty machinery paths):

- **change classes:** `crates/`, `docs/` and `Cargo.lock`; and `adapters/`, `agents/`,
  `capabilities/`, `contracts/`, `dialects/`, `extensions/`, `fixtures/`, `openspec/`, `policy/`,
  `recipes/`, `reference/`, `specs/`, `realms.json`, `README.md`, `ARCHITECTURE.md` and
  `CONTRIBUTING.md`: the product's code, data and prose, which the required jobs build, compile,
  read or test (`.github/workflows/ci.yml:157-160`, `:305-308`), and which review reads;
- **machinery, everything else:** `.cargo/`, `.github/`, `.gitattributes`, `.gitignore`,
  `Cargo.toml` (its workspace lint tables are what clippy enforces, `Cargo.toml:25-41`),
  `_typos.toml`, `clippy.toml`, `deny.toml`, `cargo-llvm-cov-version.txt`, `rust-toolchain.toml`,
  `rust-nightly-version.txt`, `flake.nix`, `LICENSE-APACHE`, `LICENSE-MIT`, `assets/`, `ops/`,
  `packaging/`, `quality/`, `scripts/` and `tests/`.

A path the list does not name is machinery, so a path added at the root is machinery until the
operator's compile names it. Cost, said plainly: a change to `Cargo.toml`, a dependency bump among
them, can never park as `ci-infrastructure`; a setup fault beside it costs one implement visit,
as today.

**The closure is checked at the base revision.** The change classes are only as good as the
machinery's closure, so the engine tests it before it trusts it, over the base revision the
observation names (ruling 3), read through 0109's verified reader (below) and never from the
change's tree:

- Every required entry's workflow file is parsed at the base into a typed subset: each job's
  `needs`, its `strategy.matrix` and its steps' `uses` and `run`. The closure starts from each
  required job and every job it `needs`, transitively.
- A **reference** is a local file a reached step or file executes: a `uses: ./…` (and that
  action's `action.yml`, whose `runs` are read the same way); in a `run:` line, or in a reached
  script, the operand of `bash`, `sh`, `source`, `.`, `exec`, `python3`, `python`, `node`,
  `perl` or `ruby`, and any command word that is itself a repository path. Every reached file is
  scanned for references in turn.
- **Every file a required job executes at the base lies outside the change classes,** or
  `machinery-escapes` withholds the class, naming the reference and the file that made it.
- **A reference the test cannot resolve fails closed,** as `machinery-unresolved`: an operand or
  command word built by an expansion, a command substitution, `eval`, or a glob, unless it is a
  variable the same file assigns only from literal words. Whether HEAD's own scripts resolve
  under this rule is **unmeasured**; 0111b's test runs it over HEAD's base and records the
  answer. Where they do not, the house's gate never draws `ci-infrastructure` until the scripts
  are written resolvably, which fails closed.

Why the base is enough: any change that could add a reference from the machinery into a change
class must edit a machinery file, which already withholds. So an untouched machinery whose
closure holds at the base is the machinery the head ran. The parse needs a YAML reader the
workspace does not hold at HEAD (`Cargo.lock` names no `yaml` crate); 0111b adds one at the
runtime's edge, into a typed subset, under the house's licence and audit jobs.

**The change set is read through 0109's verified object and range reader,** never by git in the
operated repository. HEAD's `changed_paths` runs `diff-tree` with the operated repository as its
working directory (`crates/brokkr-runtime/src/anchor.rs:62-65`, `:156-175`), so its replace refs
are honored. Decision 0109 observed a replace ref planted there shorten the repository's own
`diff-tree` to one path, while a scratch over the same objects listed both
(`docs/decisions/0109-a-gate-s-re-visit-reads-what-changed.md:986-994`). So the range is read by
0109b's `git_query` in an engine-made source scratch, every object hashed and no ref of the
operated repository resolved (0109 `:975-994`), over the two commit ids the binding names. Where
that read cannot establish equivalent provenance and a complete, byte-exact path list (a
`read_failed` or `delta_incomplete` end, or 0109b unbuilt), `range-unproven` withholds the
class. A replaced-object view therefore never hides a machinery path: the path is found, or no
`ci-infrastructure` is drawn.

What the classes do not cover, said plainly: the change's own code under test. A required job
that runs `cargo test` builds and runs the change's crates, their manifests and their build
scripts; that is what CI measures, not its machinery. A change that rigs its own test to pass is
a change review reads, and every landing path reaches review (ruling 6).

- **(b) A conservative machinery list,** `.github/`, `scripts/`, `quality/` and the toolchain
  files, as fix round 1093r4 drafted. Refused: it is open, so `deny.toml`, `_typos.toml` and any
  root configuration it did not name were the change's.
- **(c) A per-entry definition list,** as fix round 1093r3 drafted. Refused: it missed HEAD's
  own helpers.

**The tested revision is established, or the pass refuses.** HEAD's pull-request workflow tells
three revisions apart: `delivered by brokkr` checks out the trusted base and the pull request's
head by explicit ref (`.github/workflows/ci.yml:49-64`), and the ratchet job asserts that its
checkout is the pull request's merge commit (`:663-671`). A checkout with no `ref`, as the audit
job's (`:681-683`), tests the forge's generated merge revision on a `pull_request` event
(<https://docs.github.com/en/actions/writing-workflows/choosing-when-your-workflow-runs/events-that-trigger-workflows#pull_request>;
unmeasured here). And a run's association SHA is not proof of what it checked out: a job in a
run on the head may check out the base (`:49-55`). So each required Actions entry declares
`tested`, one of two words, and the tested revision must be the exact one:

- `head`: the job tested the gated head itself. The run's `head_sha` and its association's
  `head.sha` must equal the gated head, and the pinned workflow at the base, read by ruling 3's
  closure parse, must check that SHA out in the job: an `actions/checkout` step whose `ref` is
  the head-SHA expression `${{ github.event.pull_request.head.sha }}`, as `:61-64` spells it.
  A `head` entry whose job checks out no such ref, or more than one ref it could run from,
  refuses as `revision-unestablished`.
- `merge`: the job tested the forge's merge revision, `refs/pull/<number>/merge`. The pinned
  workflow's checkout in the job must take that ref, by default or by naming it. The
  observation must name the exact merge commit the run tested, as the run records it, and that
  commit's two parents. Its first parent must be the base the observation names, which must be
  the pull request's current base, and its second the gated head.

**Disagreement refuses.** Where the run's `head_sha`, its association's `head.sha`, the merge
commit's second parent and the gated head are not all one commit, or the association's
`base.sha` is not the merge commit's first parent, the observation refuses with
`revision-disagrees`, naming the values. It is never resolved by preferring one source.

The declaration is the operator's statement about a pinned workflow, and `machinery-touched`
withholds the class whenever that workflow moved from the base's bytes. Whether the forge's run payload names the merge commit
it tested is **unmeasured**: the run's `head_sha` is the pull request's head, not the merge
commit (unmeasured until 0111a). Until 0111a's evidence shows a forge field that records the
tested merge commit, every `merge` entry refuses as `revision-unestablished`, and that fails
closed. Every required entry in this house's list is `merge` (ruling 5), so until then this
house's gate never passes on its own.

**When `green` is drawn.** `green` grants nothing
([The gate grants nothing](#the-gate-grants-nothing)); the rule below says only when the gate
proceeds without either new outcome. The engine draws `green` only when all of these hold:

- the observation's repository, pull request and head equal the run's pinned repository, the
  pull request the gate names and the engine's own git head;
- every entry of `required_checks` is matched by exactly one settled check from its bound
  producer, in that producer's current execution, and that execution is admitted as this pull
  request's;
- each matched Actions entry's tested revision is established, and its sources agree, as above;
- each matched check's conclusion is `success`.

The **current execution** of an Actions entry is the workflow run with the highest run number
among those the forge lists for its workflow path, its event, the head and this pull request's
association, read at its highest attempt. Any other run for that key still queued or in
progress, or a run list the observer could not read whole, refuses as `execution-ambiguous`.

Nothing short of that is `green`. **Every binding and completeness refusal comes before any
classification.** The engine checks each row below in order, records every one that holds, and
classifies a failure only when none does. So a required check that is missing beside an
otherwise classifiable failure refuses as `missing-required`; it is never typed.

| Refusal | When |
|---|---|
| `unrelated` | the repository, the pull request or the head differs from the binding |
| `head-unpublished` | the forge's head for the pull request is not the engine's git head: the head the run would gate is not the one pushed |
| `empty` | the forge reports no check for the head |
| `producer-mismatch` | a check carries a required entry's name from another app or workflow path; the refusal names it, so a substitute is never silently passed over |
| `association-unproven` | a required entry's execution is not positively associated with this pull request, its head and its base; the refusal names the run |
| `head-shared` | another open pull request shares the head and an execution's association cannot tell them apart |
| `status-required` | the forge's required set names a commit status context, which is never admitted |
| `missing-required` | a required entry matches no check from its bound producer; the refusal names the entry |
| `ambiguous-required` | a required entry matches more than one check in its current execution |
| `execution-ambiguous` | a required entry's current execution cannot be told, as above |
| `revision-unestablished` | a required entry's tested revision is not established, as above |
| `revision-disagrees` | the association, the run's head, the tested commit and the gated head are not one commit; the refusal names the values |
| `not-settled` | a required check is still queued or in progress when the observer's own polling bound ends, short of the seat's deadline |
| `required-set-disagrees` | the forge requires a check the pinned list lacks |

One more refusal follows the pass and precedes classification: `non-success`, when no required
check failed, and one concluded `neutral`, `skipped`, `stale` or `action_required`. It names the
check and the word.

A refusal never passes. The run parks `ruled`, with the rule's id and the refusals' words.

**What withholds `ci-infrastructure`.** A withholding is not a refusal: it parks nothing and
stops nothing landing would do. Each is a precision guard on the one class, and fails closed to
"not infrastructure": where one holds, a red observation is `change` and an all-success one is
`green`. The engine checks every row and records each that holds in the class's sealed reason.

| Withholding | When |
|---|---|
| `machinery-touched` | a changed path lies outside every change class; it names the paths |
| `machinery-escapes` | at the base, a file a required job executes lies inside a change class |
| `machinery-unresolved` | at the base, a reached file holds a reference the closure test cannot resolve |
| `range-unproven` | 0109's verified reader cannot establish the range's provenance and its complete, byte-exact path list |
| `range-too-large` | the range's changed paths exceed the bound the classification record carries |
| `basis-too-large` | the classifier's sealed basis would exceed its bound |

The gate's word is bound to the engine's class: `pass` is admitted only on `green`, and `fail`
with a `green` class is refused. A refused word is never rewritten (the same binding 0109
ruling 2 gives a verdict).

The classification is engine-written: an `Owner::Engine` record input, as `reviewed_heads` is
(`crates/brokkr-core/src/policy/inputs.rs:116`), so a seat may neither declare nor overwrite it,
as the engine-owned strategy is held (`:87-95`). Both inputs ride in the decision's `inputs`
(`crates/brokkr-core/src/decision.rs:86`). So the journal holds three things, in the order they
happened, with no new event: the observation the seat sealed, the class the engine drew from it,
and the rule that routed on it.

**The classification replays from what the journal seals, and nothing else.** A replay cannot
read the pinned list out of the run's manifest: HEAD's manifest records each bundle file as a
digest string, never its bytes (`crates/brokkr-runtime/src/bundle.rs:4214-4239`), so a digest
can confirm a list but cannot rebuild one. So the engine's `ci_class` record carries a
**basis**: every input the class was drawn from that the sealed observation does not hold, as
bounded, typed data under `ci-classification/v1`:

- the digest of the sealed observation it read;
- **the pinned required list**, every `required_checks` entry as the compiled `ci-checks.json`
  declares it, with that file's digest beside it;
- **the declared literal set**: each setup step (workflow, job, step name) and each pattern (id,
  steps, literal bytes);
- **the change classes** and what ruling 3's closure test took at the base: its verdict,
  and, for each required job and each job it needs, transitively, its `needs` and its checkout
  ref;
- **the comparison values** the binding read: the repository's numeric id, the pull request's
  number, the base commit, the merge base, the gated head, the engine's git head and the forge's
  head for the pull request;
- the range's two commit ids, and the changed-path list 0109's verified reader gave for that
  range.

A replay is `classify(observation, basis)` over the sealed observation and that record alone,
and must equal the recorded class. It reads no bundle, no manifest file, no checkout, no Git
range and no forge, so a replay after the bundle is recompiled, the worktree is gone and the
forge's runs have expired still holds. The bounds: the compile admits at most 64 required
entries, 64 setup steps and 64 patterns, so the list and the literals are at most 32 KiB; the
closure's job map is at most 256 jobs; the path list is at most 4,096 paths and 256 KiB. A
basis beyond its bound withholds the class as `basis-too-large`, and a range beyond its own as
`range-too-large`, with ruling 3's withholdings; neither is ever truncated.

- **(b) A new `ci/classified` event.** Moves the event envelope, every manifest and every fold
  arm for a fact an input can carry. Refused, as 0106 ruling 1 refused `run/park-caused`.
- **(c) The excerpt kept outside the journal by digest.** Refused: the journal would point at
  log bytes kept somewhere else, and a replay would need them. No log byte is kept at all
  (below).

**No raw log byte enters the journal.** The journal records only what matched. An excerpt is
read only for the first failed step of a failed required job, or of a failed dependency of a
required aggregate (ruling 4), and only when that step is one ruling 4's list declares as
setup. It lives only in the engine's memory, between the observer's stdout (ruling 2) and the
edge parse:

- the observer reads each excerpt from the step log's tail, at most 8 KiB, and at most four of
  them, so 32 KiB in total; a fifth candidate refuses the observation rather than dropping one
  silently;
- at the edge, before anything is sealed, a pure core function matches each excerpt against
  that step's declared literals and replaces it with the set of literal ids that occur in it;
- the excerpt's bytes are then dropped. The sealed observation holds, for each such step, the
  job's id, the step's number and name, the run's id and attempt, and the matched literal ids,
  possibly none. The `ci_class` basis holds the literal bytes the ids name, from the pinned
  list (above), never from a log;
- no excerpt reaches a seat, a model, a notes field or the stderr tail (ruling 2's `ci` site
  journals stderr's length and digest only).

So a replay re-derives the class from the matched ids, which is all the classifier reads, and
the person reads the log itself on the forge, from the run, attempt and job ids the park names.
Because no log byte is durable, the forge's secret masking
(<https://docs.github.com/en/actions/security-for-github-actions/security-guides/using-secrets-in-github-actions>)
is no longer a precondition of ruling 4's pattern arm, as fix round 1093r3 made it; whether the
forge masks every secret stays **unmeasured**, and nothing here relies on it. What is durable is
bounded by the compile: a literal id is kebab-case and at most 64 bytes, and a matched set holds
at most the step's declared literals.

### 4. What evidence types a park, and the classifier

**Recommended: (a) recipe-declared literals over declared setup steps type a person's park, and
nothing a failure draws reaches a pass or a re-run.**

**A classification of a failure chooses between two outcomes, and neither is a pass.** Under
ruling 1 no write is sent, so a failed observation can only be:

- **the change's**: a diagnosis seat attempt, as today; or
- **a person's `ci_infrastructure` park**, with the evidence and no diagnosis seat attempt.

A mistyped code failure therefore costs a person's look, not a laundered pass. That is why
evidence a change can produce is admissible here, where fix round 1093r2 refused it: there, a
literal could buy a re-run, and a re-run could buy a pass. A pass is drawn only by ruling 3's
rule, which reads every required check's own `success` and never reads a log.

- **(a) Recommended: declared literals over declared setup steps.** The list declares setup
  steps, each by workflow, job and step name, and the literals each may match. A failed required
  job, or a failed dependency of a required aggregate, is typed setup only when its first failed
  step is a declared setup step and that step's bounded excerpt holds one of that step's
  literals, and the observation parks only when its whole failure set is setup (the classifier,
  below). The issue's port-bind case is one:
  the step that boots the compose services, declared setup, with the literal
  `address already in use`. The adopter's step name and excerpt are unmeasured.
  - **The bounded cost, said plainly:** a change that makes its own failure print a declared
    literal in a declared setup step reaches a person's park, not a pass. Both an unchanged
    setup step that runs repository code, such as a service bootstrap that starts the
    repository's own compose file, and HEAD's checkout-local action
    `uses: ./.github/actions/setup-cargo-audit` (`.github/workflows/ci.yml:681-689`) can print
    it. The person reads the evidence and decides; nothing passes.
- **(b) Nothing types a park.** Every failure is the change's, as today. Cost: the narrowed
  acceptance is not delivered, and the port bind still costs a diagnosis seat attempt. It no longer
  depends on secret masking: no log byte is journaled under (a) either (ruling 3).
- **(c) A forge-proven non-execution set as a precondition**, as fix round 1093r2 recommended.
  Removed. One observed payload establishes a reported value, not its semantics, and not that
  the change could not cause it. The set is left **unmeasured and empty**; no classifier arm
  reads it, so a job-level `startup_failure` or `timed_out` is untyped, and the change's.
- **(d) Any class drawn from a pattern that reaches a pass or a re-run.** Refused by ruling 1
  and by ruling 3's pass rule.

**Timing-sensitive tests stay the change's.** A flaky test fails in the step that runs the tests,
and a test step is not a setup step. The compile cannot tell a test step from a setup step, so
that is the operator's declaration; a literal declared over a test step costs the same bounded
person's park and never a pass. Flaky-test handling is out of scope; it is a follow-up for a
decision of its own.

**The classifier.** One pure function over the observation and its basis (ruling 3: the
declared list, the literals, the closure, the comparison values and the change's paths), with a
closed result, failing closed.

`classify(observation, basis) -> Classification` lives in `brokkr-core`, reads no clock,
environment or network (0071 ruling 1), and is total. The engine builds the basis before it
calls it, and seals the same basis it called it with, so live and replay take one path.

The changed paths are the engine's, read through 0109b's verified reader (ruling 3), not
through HEAD's `crate::anchor::changed_paths` (`crates/brokkr-runtime/src/anchor.rs:171`), which
diffs in the operated repository and which 0039's docs-only flag reads
(`crates/brokkr-runtime/src/engine.rs:3158-3173`). Nor is the range the docs flag's. The docs
flag diffs the latest phase entry's head against the phase's end, so it sees only that phase's
own commits. The CI gate needs the whole change, so its range is pinned: from the merge base of
the pull request's base commit, as the observation names it, to the gated head, the merge base
taken by 0109's ancestry walk over verified commits. Both ends are checked as full commit ids
before the reader sees them. A range the reader cannot take (a base it does not have, no merge
base, an object that does not verify) withholds the class as `range-unproven`. It is never read
as an empty change. In order:

1. Any binding or completeness refusal of ruling 3 holds: **`refused`**, naming every one that
   holds. Nothing below runs, so no refusal is ever outranked by a classifiable failure.
2. The observation passes ruling 3's rule for `green`: **`green`**. One that cannot and holds no
   failed, timed-out, unstarted or cancelled required check is **`refused`** as `non-success`.
3. **Any withholding of ruling 3 holds: `change`,** with the reason `withheld`, naming every
   withholding that holds. No pattern is read, so nothing below can draw `ci-infrastructure`.
4. **The whole failure set is typed.** The failure set is every required job whose conclusion is
   not `success`, each instance of a matrix job on its own. Every element gets one closed type,
   matched without a wildcard:
   - **setup-pattern**: conclusion `failure`, its first failed step a declared setup step, and
     that step's sealed match naming one of that step's literals, with the pattern's id;
   - **setup-via-needs**: a required aggregate (below) with conclusion `failure`, whose failed
     dependencies are each, in turn, setup-pattern or setup-via-needs, and at least one failed;
   - **cancelled**: conclusion `cancelled`, the required job's own or a dependency's through an
     aggregate. Who cancelled is unmeasured, though on a pull request a newer push cancels the
     older run (`.github/workflows/ci.yml:25-27`);
   - **untyped**: anything else: a `failure` no declared literal matches, `startup_failure` and
     `timed_out` (a hang may be the change's), and `neutral`, `skipped`, `stale` or
     `action_required` beside a failure, so a skipped required check never rides along with a
     setup fault into a park.

   A failure ruling 3 refuses to admit (an unproven association, a disagreeing revision, a
   status) never reaches this step: step 1 has refused the whole observation for a person, so it
   is never counted toward a `ci_infrastructure` park.
5. **Only an all-setup failure set parks.** Every element setup-pattern or setup-via-needs, and
   the set non-empty: **`ci-infrastructure`**, naming each job, its step and its pattern id,
   with the workflow run's id and attempt a person needs to re-run it.
6. Otherwise **`change`**, with a closed reason naming the first element that is not setup, in
   the set's order: `cancelled`, or `untyped-failure`. One such element beside any number of
   setup faults makes the whole observation the change's.

**A required aggregate is followed through its pinned `needs`.** HEAD's required `test (…)`
jobs are an aggregate: job `engine` `needs: engine-machines`, runs `always()`, and its one step
asserts `needs.engine-machines.result` is `success` (`.github/workflows/ci.yml:315-329`). A
port bind during service boot in an `engine-machines` instance therefore fails `engine` in a
step that prints no setup literal, and read alone it is `untyped`. So a failed required job
whose job, in the base revision's pinned workflow, declares `needs` is typed through them: the
classifier reads the `needs` that ruling 3's closure parse took from the base workflow, finds
those jobs in the same workflow run and attempt, and types each that did not succeed by the
same rules, recursively. A dependency the observation does not carry, or one in another run or
attempt, is `untyped`. This is safe because the base workflow is the one the run used (ruling 3
withholds the class on a changed one as `machinery-touched`), and because the type only parks: a
mistyped aggregate costs a person's look, never a pass. The aggregate's own failed step is taken
as the dependency's consequence, not read; that is the bounded cost.

A change that touches CI's machinery never reaches step 4: step 3 makes it the change's, as
`machinery-touched`, and no pattern over its steps is read. It parks nothing for a person.

A failure the classifier cannot type is the change's, as today.

**Audit jobs need no rule of their own.** Fix round 1093r2 kept an audit job out of any re-run
unless the forge proved its steps never ran, so a re-run could not launder an advisory. With no
write, no class reaches a pass but `green`, `green` grants nothing, and the forge still requires
the audit's own `success` at merge ([The gate grants nothing](#the-gate-grants-nothing)). An audit
failure is typed like any other: the change's, or a person's park. #1113's "Not
Found" is a failed step inside the `RustSec dependency audit` job
(`.github/workflows/ci.yml:673-695`), in the step that runs the audit and reports it
(`:693-695`), which is not a setup step. This decision declares nothing for it and designs no
fix for it; it stays the change's, as today.

Also refused, under every option: a step's role alone, with no pattern, since a setup step that
installs the change's dependencies fails for the change's reasons too; and failing open on an
unread conclusion or a missing excerpt (0004).

**Enforcement binding:** a table test over recorded observations, one per arm and reason; and
`classify` matches the conclusion enum without a wildcard (0071 ruling 2).

### 5. The runner-fault list: its home and who declares it

**Recommended: (a) recipe data, `ci-checks.json` beside the policy, under `ci-checks/v1`,
compiled into the bundle and pinned with it.**

```json
{
  "schema": "ci-checks/v1",
  "repository": {"owner": "example", "name": "app", "id": 1},
  "observer": {
    "interpreter": {"path": "/usr/bin/bash", "sha256": "<64 hex>"},
    "tools": {"gh": {"path": "/usr/bin/gh", "sha256": "<64 hex>"},
              "jq": {"path": "/usr/bin/jq", "sha256": "<64 hex>"}}
  },
  "change_classes": ["crates/", "docs/", "Cargo.lock", "recipes/", "README.md"],
  "required_checks": [
    {"actions_job": {
      "app": "github-actions", "workflow": ".github/workflows/ci.yml",
      "job": "engine", "name": "test (ubuntu-latest)", "event": "pull_request",
      "tested": "merge"}},
    {"actions_job": {
      "app": "github-actions", "workflow": ".github/workflows/ci.yml",
      "job": "dependency-audit", "name": "RustSec dependency audit", "event": "pull_request",
      "tested": "merge"}}
  ]
}
```

The lists are cut short: the house's full `change_classes` is the nineteen paths ruling 3
lists, and its full `required_checks` the twelve required contexts above. The app's
numeric id is pinned beside its slug once 0111a's evidence shows it. The compile refuses a
`status` entry.

Under ruling 4 (a), two more keys declare the evidence that types a park:

```json
{
  "setup_steps": [
    {"id": "compose-up", "workflow": ".github/workflows/ci.yml", "job": "engine-machines", "step": "start services"}
  ],
  "patterns": [
    {"id": "port-in-use", "steps": ["compose-up"], "literal": "address already in use"}
  ]
}
```

The job is HEAD's: `engine-machines`, which the required `test (…)` aggregate needs
(`.github/workflows/ci.yml:315-329`). The step is illustrative: HEAD's `engine-machines` boots no
compose service, and the adopter's step name and excerpt are unmeasured. A setup step's job must
be a required entry's job, or a job a required aggregate needs at the base, transitively; the
check runs with ruling 3's closure parse, and a setup step outside both withholds the class as
`machinery-unresolved`. Under ruling 4 (b) the compile refuses either key. A pattern is a
literal: 1 to 200 printable ASCII bytes, matched as an exact, case-sensitive byte substring. No
regex, glob or wildcard is admitted, so matching is linear and the same on every host. Each `id`
is kebab-case and unique; every step a pattern names must be in `setup_steps`, and every setup
step's workflow must be the workflow of a required entry, so `machinery-touched` withholds the
class whenever it moved from the base's bytes.

The list has two syntaxes, and they never mix. A **path** in `change_classes` is a
repository-relative path. It matches a changed path that equals it, or, when it ends in `/`,
any changed path under that directory. It is not a glob, and not the docs class's regular
expressions (`crates/brokkr-runtime/src/engine.rs:3217`). A **literal** in `patterns` is matched
only against log bytes, never against a path. Each field admits only its own syntax: the
compile refuses a `*`, `?` or `[` in a path, so a glob cannot pass for one, while a literal's
bytes are all literal.

`required_checks` must be non-empty, every Actions entry must name all of its producer's parts
and its `tested` word, and `change_classes` must be non-empty and hold no path that is, or lies
under, `.github/` or any required entry's workflow. The compile refuses a list that leaves any
of them to a default. A path `change_classes` does not name is machinery; there is no machinery
list to keep. `repository` pins the binding ruling 3 checks.

The operator declares it, as any recipe or `extends` overlay (0017). No seat writes it: the
run reads the list its compiled bundle pinned at start, never the change's tree, so a branch
that edits `ci-checks.json` changes the next run's list, by the operator's compile, not its own.

- **(b) Host data under `forge.host`.** One list for every recipe; but runner faults are a
  property of a workflow, not a machine. Refused.
- **(c) Engine defaults.** A second home for a fact the recipe owns (0071 ruling 5). Refused.

### 6. Each outcome, and every route

**Recommended: (a) a pass proceeds; a runner fault parks typed for a person; anything else is
the change's, under the shared implement bound.**

The class a table sees is closed: `green`, `ci-infrastructure`, `change` or `refused`. A seat
cannot write any of them.

- **`green`**: the gate's `pass`; the run proceeds to `classify`, exactly as landing's own entry
  does, and nothing later reads it ([The gate grants nothing](#the-gate-grants-nothing)).
- **`ci-infrastructure`**: the run parks, typed (ruling 7), with the faults as its evidence, and
  no diagnosis seat attempt is spent beyond the `ci` gate's own exec attempt. A person reads
  the evidence and re-runs.
- **`change`**: routed exactly as a red gate is today, to `implement`.
- **`refused`** (ruling 3): parks `ruled`.

**A deterministic match.** Under ruling 4 (a), where several of a step's literals matched, the
classifier takes the first matched pattern in the list's declaration order. The fault carries
that pattern's id, so one step yields one fault.

**Where the gate sits: where the head is already published.** Fix round 1093r3 put an
exact-head CI gate before review in `fast` and `verify`. But `fast`'s implement completes with
a local commit (`recipes/fast/policy.json:69-75`), and pushing is a person's act after review:
the shipper's notes say "review, push, and merge next"
(`recipes/fast/scripts/ship-seat.sh:73`, `:91`). A head no one has pushed has no remote checks,
so that gate could only refuse or wait. **Recommended: (a) the gate is the `landing` recipe's
check-watching step, its entry phase.** The issue names the step ("a land run that watches pull
request checks", as the commission quotes it; the issue was not read in this seat). Landing runs
over a branch that already exists (`recipes/landing/README.md:5-8`), and at its entry no seat
has committed, so the head is the one the person brought. Whether they pushed it is checked, not
assumed: the observation names the pull request's head as the forge reports it, and the engine
refuses with `head-unpublished` when that is not its own git head (ruling 3). A person pushes,
and the operator's retry observes again.

The overlay, `landing-ci`, extends `landing` and overrides its table's `initial` to `ci` and
its `phases` to add `ci`; `landing` overrides the same keys over `brokkr-fast`
(`recipes/landing/bundle.json:7-10`). Its rules precede landing's
(`crates/brokkr-runtime/src/bundle/compose.rs:871-921`), and no rule of landing's or `fast`'s
changes.

**Every route and cap, from the incoming edge.** First match wins, in the order listed. The
`When` column names the class; ruling 10 gives its schema-legal spelling.

| Rule | From | Result | When | Next |
|---|---|---|---|---|
| (entry) | | | `initial: ci` | `ci` |
| `CI-REFUSED` | `ci` | `fail` | class `refused` | park, `ruled`, naming the refusals |
| `CI-INFRASTRUCTURE` | `ci` | `fail` | class `ci-infrastructure` | park, `infrastructure` `ci-infrastructure` |
| `CI-PASS` | `ci` | `pass` | class `green` | `classify` |
| `CI-FAIL` | `ci` | `fail` | | `implement`, `flagged` |

Then landing's own two rules, `CLASSIFY-DOCS` to `review` and `CLASSIFY-CODE` to `verify`
(`recipes/landing/policy.json:9-21`), and every `fast` rule, unchanged.

**The CI loop keeps a bound.** `ci` is entered only as the run's `initial`, and again only by
the operator's retry of its own park; the compile refuses any rule whose `next` is `ci`. So
`CI-FAIL` fires at most once a run, and the implement visit it buys is counted by `fast`'s one
implement bound, `visits_implement_gte: 3` (`recipes/fast/policy.json:76-94`;
`visits_<phase>_gte` counts every entry of the phase, whatever routed into it,
`contracts/phase-machine.v2.schema.json:73`). No loop runs through `ci`.

**What this does not gate.** A head the run itself makes, by an implement after `CI-FAIL` or a
review return, is local, and no gate in this run reads its CI: it is verified locally, judged,
and shipped for a person to push, and the pull request's CI then runs on it as it does today.
Gating a head the run made needs a publish-and-resume handoff: a park that names the head to
push, and a resume that refuses any head but that one. That is a follow-up, and the pre-review
interception in `fast` and `verify` that fix round 1093r3 drafted, removed here, waits on it.

**Refused routes**, each because it would unbound a loop, hide a failure, read an unpublished
head or write to the forge:

- any route to a re-run, under every class (ruling 1);
- any rule whose `next` is `ci`: it would gate a head the run made, which is unpublished;
- any rule into `classify` but `CI-PASS`: it would skip the gate;
- `ci-infrastructure` routed to `implement`: a diagnosis seat attempt is what the class exists
  to save;
- a refusal routed to `implement`; a refusal parks;
- `CI-PASS` on any class but `green`; the word binding refuses it (ruling 3).

Each is refused by the overlay test, over the complete composed table (ruling 10). The
scope-widened reconciliation park fix round 1093r2 promised went with the write (ruling 1); no
route or cause remains for it.

- **(b) A runner fault routed to `implement`, with the class in the commission.** It spends the
  diagnosis seat attempt the issue counts, to tell a model what the engine already typed. Not
  recommended.
- **(c) The pre-review gate in `fast` and `verify`, as fix round 1093r3 drafted.** Refused for
  now: its head is unpublished by construction. It returns with the follow-up handoff above.

### 7. The park's cause

**Recommended: (b) `infrastructure` with a closed `kind`, under `park-cause/v2`.**

- **(a) A new `ci-infrastructure` cause.** A fifth written word. Every match over `ParkCause`
  gains an arm (`crates/brokkr-core/src/park.rs:234`, `:264`, `:276`), 0106 ruling 1's four-word
  table becomes five, and 0106d's "only an `infrastructure` park spends" leaves it out by
  construction. Cost: a wider change to an accepted ruling's table. It refuses nothing (b) admits.
- **(b) `infrastructure` with `kind`.** Recommended. `kind` is one of:
  - `engine`, every writer today;
  - `ci-infrastructure`, with `faults`, each naming its workflow, job, step, kind and pattern id.

  A v1 cause with no `kind` reads `engine`. The cause stays four words, the readouts already
  paint `infrastructure`, and 0106d's rule reads unchanged: it spends only where `retryable` is
  true (ruling 8). Cost: one closed enum inside one variant, matched without a wildcard.

**The typed park matches its own rule explicitly.** A park is not typed because its inputs
happen to carry the class. The compile marks, in a table whose recipe declares a `ci` seat,
exactly one rule as the CI-infrastructure park: the park rule `from` the `ci` phase whose `when`
is exactly `{"ci_infrastructure": true}`. It refuses a table with none, or with more than one.
The engine's park arm (`crates/brokkr-runtime/src/engine/cursor.rs:361-371`) gains one case. It
writes `infrastructure { kind: ci-infrastructure, retryable: false }` instead of `ruled` only
when all three hold:

- the park is a table park, `ParkReason::Stated` with a matched rule id (`:368`);
- the matched rule is the one the compile marked;
- the deciding inputs' `ci_class` is `ci-infrastructure`.

A different table park that carries the same class does not write it. An operator's rule placed
above the marked one, such as a park on `ci_infrastructure: true` with a counter beside it, stays
`ruled`, and so does every refusal. A closed indeterminate end keeps HEAD's `infrastructure` arm
(`:364-366`). The rule id stays in the decision beside the cause, where
0106 says a rule park's rule already lives.

On acceptance, 0106's header and this one's gain the pointer pair for ruling 1's cause gaining
`kind`, under the errata rule for back-pointers.

### 8. How a CI park reads in 0106d's budget

**Recommended: (a) a `ci-infrastructure` park is `retryable: false`, and spends nothing.**

0106 ruling 4 asks for positive evidence that the last attempt never executed. Here the last
attempt is the `ci` gate's, and it executed: it read the forge and returned a typed result. The
forge's statement about the change's checks is a fact about CI, not about the attempt, and a
pattern match is evidence that a step ran. So the rule, read literally, gives `false`. And a
budget retry, as 0106d builds it, re-enters the parked phase as a new effect: it would observe
the same settled runs and park again, spending the budget for nothing. A person's re-run is what
changes the forge's state, and the operator's retry after it re-observes.

- **(b) `retryable: true`.** Spends a retry that cannot help, as above. Refused.

**Enforcement binding:** `a_ci_infrastructure_park_is_never_retryable` (0111d). 0106d's
non-retryable infrastructure test, `a_non_retryable_infrastructure_park_spends_nothing`
(`docs/decisions/0106-a-park-names-its-cause-a-held-wait-and-a-budget.md:1224`), is extended
with one `ci-infrastructure` park, spending nothing. 0106d's
`only_an_infrastructure_park_spends_the_budget` is unchanged: it reads `retryable`, not `kind`.

### 9. The readouts

**Recommended: (a) one derivation in `brokkr-view`, painted by `inspect` and the fleet.**

`inspect` names each observation's class, its refusal or its faults, and the evidence a person
needs to re-run: the workflow run's id and attempt and each failed job's id, from one view model
(0071 ruling 7). The fleet paints the park's cause through `ParkCause::line`
(`crates/brokkr-core/src/park.rs:275`), which the TUI and `inspect` already paint
(`crates/brokkr-cli/src/tui/columns.rs:375`, `crates/brokkr-cli/src/render/inspect.rs:36`; the
fleet's row field is `crates/brokkr-view/src/fleet.rs:339`). It reads
`infrastructure (ci-infrastructure, not retryable): …` with the faults' job and pattern ids.

### 10. How a table spells the class

**Recommended: (a) engine-owned flags, one per class word, legal under phase-machine v2 with no
version.**

The first draft wrote `ci_fault_in: [runner-fault]`. That fails the frozen v2 schema: `when`'s
values are a boolean, a number or a string, never an array
(`contracts/phase-machine.v2.schema.json:71-77`). HEAD's runtime does read choice arrays
(`crates/brokkr-core/src/policy/conditions.rs:43-75`), but the published contract does not admit
them, and 0082 mints no version outside an accepted decision.

- **(a) Engine-owned flags.** `ci_green`, `ci_infrastructure` and `ci_refused`, each an
  `Owner::Engine` `Flag`. All three are projected from the one `ci_class` enum at decide, so at
  most one is true, and `change` is all three false. A boolean input by name is what v2's `when`
  admits (`:73`). Ruling 6's `When` column reads `ci_refused: true`, `ci_green: true` and
  `ci_infrastructure: true`. Cost: three names for one enum. That is a projection, not a second
  home, because the enum stays the one fact and the flags are derived in one function. A table
  could also write two of them in one rule, which would never match; the overlay test refuses
  that at compile. It mints no contract.
- **(b) A table contract of this decision's own, admitting a `<choice>_in` array.** Refused.
  Phase-machine v3 is decision 0109's, which is accepted, and its v3 is proposed in its slice
  0109c (`docs/decisions/0109-a-gate-s-re-visit-reads-what-changed.md:1500-1520`, `:1830`). This
  decision claims no table version of its own. What it needs from v3 is the ask below.
- **(c) `ci_fault_in` under v2, as drafted.** Refused: it is not legal under the schema it
  claims.

**One result per row.** Each rule's `result` is one string under v2
(`contracts/phase-machine.v2.schema.json:31`, `:50`), and the engine matches it by exact
equality (`crates/brokkr-core/src/policy.rs:264-273`). Ruling 1 removed the two re-run words
fix round 1093r2 put on one row. **Each row of ruling 6's landing table, checked against v2:**

- `CI-REFUSED` and `CI-INFRASTRUCTURE`: one result, `fail`; a `when` of one boolean input,
  `ci_refused: true` or `ci_infrastructure: true` (`:73`); `park: true`, with no `next`, no
  severity and no `requires_artifacts`, as v2's park admits (`:53-57`). Legal.
- `CI-PASS`: one result, `pass`; `when` `ci_green: true`; `next: classify`, a declared phase.
  Legal.
- `CI-FAIL`: one result, `fail`; no `when`; `next: implement` with severity `flagged`. Legal,
  and not made unreachable by an unconditional rule above it for the same `(from, result)`
  (`:28`), since the two parks above it are conditional.
- The overlay's `initial: ci` and its `phases`, which add `ci` to landing's seven, are one
  string and a list of strings (`:16-20`). Legal.

So every row 0111 adds is v2-legal on its own.

**The whole composed table validates, not the overlay alone.** A recipe's table is composed: an
overlay's rules precede its base's, and an overlay rule that overrides a base rule's id replaces
it (`crates/brokkr-runtime/src/bundle/compose.rs:871-921`). The layers of one composition share
one schema (`:826-836`). So what must validate is the complete materialized table. Every
delivering recipe extends `fast`, directly or through `brokkr-fast`
(`recipes/brokkr-fast/bundle.json:4`), and `fast`'s `IMPL-BLOCKED-ENVIRONMENT` matches
`blocked_by_in: ["environment"]` (`recipes/fast/policy.json:40-50`). That array fails v2's
scalar-only `when` (`:71-77`), and HEAD's runtime spells a choice only as an array
(`crates/brokkr-core/src/policy/conditions.rs:43-75`). So no fast-derived composed table that
keeps the environment park validates whole under v2. A `jq` scan of every
`recipes/*/policy.json` for an array-valued condition printed `blocked_by_in` for `fast`,
`drift_in` and `strategy_in` for `triage`, and nothing for any other table.

**But the composed landing table does not validate whole under v2, so 0111 still needs 0109's
v3.** `landing` extends `brokkr-fast`, which extends `fast` (`recipes/landing/bundle.json:5`,
`recipes/brokkr-fast/bundle.json:4`), so `landing-ci`'s composed table carries `fast`'s
`IMPL-BLOCKED-ENVIRONMENT` and its array. That is true of `landing` today, before 0111 adds a
row; 0111 adds no array and inherits the one there. Fix round 1093r3's v2 route was the
`verify`-based gate, and that gate goes with the pre-review interception (ruling 6). No
composition that holds the landing gate and keeps the environment park validates under v2, so
the ask below stays.

**Recommended: (a) the landing gate on 0109's v3.**

- **(a) Recommended.** `landing-ci` lands only after 0109c moves `fast` to v3, and only if the
  operator grants the ask below. Its composed table then validates whole under v3. Cost, said
  plainly: no live producer exists before 0109c. 0111a to 0111c land and test offline, and live
  telemetry waits on 0109c.
- **(b) An engine-owned boolean projection of `blocked_by`,** such as `blocked_by_environment`,
  so `fast`'s park spells as a scalar under v2 and `landing-ci` validates under v2 now. It edits
  `fast`'s table, which every delivering recipe composes, and adds a second name for a fact the
  choice already holds. Not recommended; it is the operator's way to land the gate before 0109c.
- **(c) Validate the overlay's own rules alone.** Refused: the table the engine runs is the
  composed one.

**The ask on 0109's proposed v3, for the operator.** That `forge.phase-machine/v3` admit a
`<choice>_in` key as a non-empty array of words from that choice's closed vocabulary, which is
the shape HEAD's runtime already reads (`conditions.rs:43-75`). `fast`'s `blocked_by_in` and
`triage`'s `drift_in` and `strategy_in` would then validate whole, and so would `landing` and
`landing-ci`. 0111 mints nothing for it; the ask is 0109's to rule.

**Enforcement binding:** 0111e validates the complete materialized table `landing-ci` composes,
not the overlay's file, against v3. A test asserts that exactly one flag is true for each
non-`change` class.

## What stays out

- Any forge write: ruling 1. No re-run is requested, and no seat holds a write credential.
- No model decides: the gate is an exec seat, the classifier is core.
- No required check is skipped, marked passed or written.
- No refusal is weakened; `retryable`'s positive-evidence rule stands as 0106 wrote it.
- No Windows code (0063).
- #1113's own fix.
- The dispatcher's retry budget, beyond ruling 8.
- Flaky-test handling of any kind, quarantine included: a follow-up (ruling 4).
- `fast`'s `blocked_by_in` array outside v2's published schema: a follow-up (ruling 10).
- A CI gate on a head the run itself made, before review in `fast` or `verify`: a follow-up
  that needs a publish-and-resume handoff (ruling 6).
- Commit statuses as required evidence: a follow-up that needs a revision attestation (ruling 3).
- A forge other than GitHub: `forge` is a closed enum of one word in v1.
- An automated re-run: the follow-up decision ruling 1 (c) names.

## Consequences

A runner fault the classifier can type costs no model diagnosis attempt: the run parks with a
cause a script can read and evidence a person can act on, and the person re-runs. Every other
failure is the change's, as today. Brokkr sends the forge nothing, and no class the gate draws
lets anything reach a later phase or a merge that landing without it would not, so the gate is
not a CI authority: the forge's required checks at merge stay the only one. A wrong class costs
at most a person's look or one bounded implement visit. A recipe with no list behaves
exactly as now. The cost is an exec observer per recipe that wants it, one read-only token, and
a list an operator keeps. Four contract versions are proposed: `ci-observation/v1`,
`ci-classification/v1`, `ci-checks/v1` and `park-cause/v2`. Each is minted only when its slice
lands under this decision's acceptance (0082), and the first three only after 0111a's evidence.
This decision proposes no table contract: the gate is `landing`'s entry, over a published head,
and it waits on 0109's v3 and ruling 10's ask.

## The slices

Smallest first; each lands on its own. Evidence comes before any contract, because 0082 forbids
fixing a minted version in place: a shape the evidence would change is changed in this decision,
while it is proposed, never in a minted file. Each piece of lifecycle and readout work is in the
slice that produces it, and each typed assertion is in the slice that builds its writer.

1. **0111a — the forge's evidence, before any contract.** The operator captures, by hand, outside
   any run, and with a read-only token, representative payloads for this repository's own pull
   requests: the check runs and the commit statuses for a head; the workflow runs that produced
   them, with path, event, number, attempt, head and whatever the forge says of the revision it
   tested; their jobs with steps; one step log's bounded tail, cut to what 0111b's matcher reads;
   and the base branch's required set. A failed setup step is added if one recurs. The log sample
   is reviewed by the operator before it is committed, as any test data is. It needs no claim
   that the forge masked a secret, because under ruling 3 no log byte reaches a journal; the
   sample is a matcher's test input, not a journal row. Files: the captured payloads, redacted, as test data under
   `crates/brokkr-core/src/ci/evidence/` with a note of how each was taken; one test that pins
   their digests, so no later slice regenerates them. About 40 test lines and the data. No
   contract and no production code. It settles, or marks still unmeasured:
   - whether the forge reports a job's key, and the app's numeric id (ruling 3);
   - whether a run names the merge commit it tested, which decides whether any `merge` entry can
     pass (ruling 3);
   - what a run's and a check run's `pull_requests` association holds, and in which cases it is
     empty, a fork's pull request among them (ruling 3);
   - how a matrix job's name is spelled;
   - the closed list of conclusion words, and how a job log is cut into steps.

   Secret masking is no longer among them: ruling 4's pattern arm does not depend on it.

   Where a fact differs from what this decision assumes, the decision is revised before it is
   accepted, or before 0111b mints. Test: `the_ci_evidence_files_are_pinned_by_digest`. With
   this slice the classifier's first consumer exists: 0111b's tests read it, so the next sweep
   can count, offline, what a typed park would have saved.
2. **0111b — the classifier, its record and its readout, over 0111a's evidence.** Files: a new
   `crates/brokkr-core/src/ci.rs` with `ci/observation.rs`, `ci/checks.rs`, `ci/classify.rs`,
   `ci/closure.rs` (ruling 3's machinery closure over typed workflow data) and
   `ci/tests.rs`; `crates/brokkr-core/src/lib.rs`; the base workflow's parse into that typed
   subset at the runtime's edge, with its YAML reader added to the workspace; `crates/brokkr-core/src/policy/inputs.rs` (the
   seat-owned `ci_observation` record; the engine-owned `ci_class` record and ruling 10's three
   flags); `crates/brokkr-runtime/src/bundle.rs` (load and pin `ci-checks.json`, refuse it on a
   non-exec seat); `crates/brokkr-runtime/src/engine/decide.rs` (take the bounded range through
   0109b's verified reader, classify at decide, and record the class with its sealed basis); and
   the readout it produces,
   `crates/brokkr-view/src/` (the CI view model: class, refusals, faults, and the run's id and
   attempt and each failed job's id a person needs) with `crates/brokkr-cli/src/render/inspect.rs`.
   About 600 production and 900 test lines; it may split at triage into the observation and its
   pass (with the wire test) and the classifier. Contracts, minted here and only after 0111a:
   `ci-observation/v1`, `ci-checks/v1` and `ci-classification/v1`. No live producer yet: its
   observations are 0111a's evidence and builders over it, and its compile refuses every live
   `ci` declaration until 0111c (ruling 2):
   `a_bundle_declaring_a_ci_seat_refuses_to_compile_until_the_trusted_launch_lands`, a custom
   exec recipe's declaration included. Tests: one per arm and reason of
   ruling 4, over observations built by a shared builder from 0111a's evidence. The wire, end to
   end, over 0111a's evidence in `ci-observation/v1`'s shape:
   `a_recorded_observation_is_sealed_classified_replayed_and_inspected`. That test hands decide a
   recorded observation through a test-built bundle the live compile never admits, and 0111c
   repeats it over the observer's stdout. It asserts that the sealed seat record holds
   it; that the decision's inputs hold `ci_class` and its one true flag; that a fold rebuilt from
   the journal re-derives the same class; and that `inspect` paints it. The ownership tests:
   `a_seat_declaring_ci_class_or_a_class_flag_fails_the_compile`;
   `a_seat_payload_carrying_ci_class_is_stripped_before_the_table`;
   `an_observation_with_an_unknown_field_refuses_whole`. The range and the list's syntax:
   `the_change_range_is_the_merge_base_to_the_gated_head_not_the_phase_entry`;
   `a_range_the_verified_reader_cannot_take_withholds_as_range_unproven`;
   `a_replaced_object_view_whose_diff_omits_a_machinery_path_still_finds_it_or_withholds`
   (a replace ref planted in the operated repository shortens its own `diff-tree`; the verified
   read lists the machinery path, or the range withholds as `range-unproven`, never `ci-infrastructure`);
   `a_glob_character_in_a_change_class_path_refuses_to_compile`;
   `a_directory_path_matches_only_under_its_slash`. The replay (ruling 3):
   `a_replay_with_the_bundle_the_checkout_and_the_forge_all_gone_re_derives_the_class`
   (the journal exported, then the bundle, the worktree and the forge stub removed);
   `a_replay_reads_the_carried_path_list_not_git`;
   `the_basis_carries_the_required_list_the_literals_the_closure_and_the_comparison_values`;
   `the_live_class_and_the_replayed_class_take_one_function_over_one_basis`;
   `a_basis_beyond_its_bound_refuses_as_basis_too_large`;
   `a_range_beyond_the_path_bound_refuses_as_range_too_large`. The evidence (ruling 4):
   `a_port_bind_literal_in_a_declared_setup_step_draws_ci_infrastructure`;
   `a_declared_literal_in_an_undeclared_step_is_the_changes`;
   `a_literal_printed_by_the_change_in_a_declared_setup_step_draws_a_park_never_green`;
   `no_class_a_failure_draws_is_green`; `a_startup_failure_or_timed_out_job_is_the_changes`;
   `several_literals_in_one_excerpt_match_the_first_declared`;
   `a_pattern_naming_an_undeclared_step_refuses_to_compile`;
   `a_setup_step_outside_every_required_workflow_refuses_to_compile`;
   `a_fifth_excerpt_refuses_the_observation`; the journal's bytes (ruling 3):
   `no_log_byte_reaches_the_sealed_observation_or_the_basis`;
   `a_sealed_observation_names_only_the_matched_literal_ids_and_the_job_step_and_attempt`;
   `an_unmasked_secret_in_an_excerpt_never_reaches_the_journal`;
   `a_timing_sensitive_test_failure_is_the_changes`; `a_cancelled_job_is_the_changes`;
   `one_untyped_job_makes_the_whole_change`; `an_unknown_conclusion_refuses_by_name`;
   the whole failure set (ruling 4):
   `a_cancelled_required_job_beside_a_setup_failure_is_the_changes`;
   `an_unclassifiable_failure_beside_a_setup_failure_is_the_changes`;
   `a_skipped_required_check_beside_a_setup_failure_is_the_changes`;
   `an_unadmitted_failure_beside_a_setup_failure_refuses_and_never_parks_typed`;
   `a_port_bind_in_engine_machines_through_the_engine_aggregate_draws_ci_infrastructure`;
   `an_aggregate_with_one_untyped_dependency_is_the_changes`;
   `an_aggregate_whose_dependency_is_in_another_attempt_is_the_changes`;
   `the_aggregate_reads_its_needs_from_the_base_workflow_never_the_head`;
   `a_setup_step_in_a_job_no_required_aggregate_needs_refuses`;
   `a_stale_head_observation_does_not_read`; `a_pass_with_a_failed_job_is_refused`;
   `a_seat_cannot_write_the_classification`; `a_ci_list_on_a_model_seat_refuses_to_compile`.
   The producer, the association and the tested revision (ruling 3):
   `a_same_named_green_check_from_another_workflow_refuses_as_producer_mismatch`;
   `a_green_check_on_another_event_does_not_match_its_entry`;
   `a_green_run_on_the_same_head_for_another_pull_request_refuses_as_association_unproven`;
   `a_run_with_an_empty_association_refuses_as_association_unproven`;
   `two_open_pull_requests_sharing_the_head_refuse_as_head_shared_unless_each_association_names_one`;
   `an_association_and_a_checkout_that_disagree_refuse_as_revision_disagrees`;
   `a_head_entry_whose_pinned_job_checks_out_another_ref_refuses_as_revision_unestablished`;
   `a_status_entry_in_the_required_list_refuses_to_compile`;
   `a_status_context_in_the_forges_required_set_refuses_as_status_required`;
   the machinery, each asserting the exact class and withholding (ruling 3):
   `a_deny_toml_only_change_with_a_red_setup_only_observation_is_change_not_ci_infrastructure`;
   `a_typos_toml_only_change_with_a_red_setup_only_observation_is_change_not_ci_infrastructure`;
   `a_change_to_ci_yml_with_a_red_setup_only_observation_is_change_as_machinery_touched`;
   `a_change_to_the_local_action_a_required_job_runs_is_change_as_machinery_touched`;
   `a_helper_only_change_to_test_summary_sh_with_a_red_setup_only_observation_is_change`;
   `a_helper_only_change_to_shard_runner_py_with_a_red_setup_only_observation_is_change`;
   `a_machinery_change_with_an_all_success_observation_is_green_and_parks_nothing`;
   `a_new_root_file_no_change_class_names_is_machinery`;
   `a_change_class_only_change_with_a_red_setup_only_observation_draws_ci_infrastructure`;
   `a_base_file_a_required_job_executes_inside_a_change_class_withholds_as_machinery_escapes`;
   `a_reference_built_by_expansion_withholds_as_machinery_unresolved`;
   `the_closure_follows_needs_uses_and_sourced_helpers_transitively`;
   `the_closure_reads_the_base_revision_never_the_changes_tree`;
   `an_empty_change_class_list_refuses_to_compile`;
   `a_change_class_under_github_or_a_required_workflow_refuses_to_compile`;
   `the_closure_test_over_heads_own_base_records_its_answer`;
   `an_ambiguous_tested_revision_refuses_as_revision_unestablished`;
   `a_merge_revision_whose_parents_are_not_the_base_and_the_head_refuses`;
   `two_unsettled_runs_for_one_entry_refuse_as_execution_ambiguous`;
   `missing_required_beside_a_classifiable_failure_refuses_and_is_never_typed`. The pass's
   refusals, one test each, every one asserting the refusal's variant and that the class is not
   `green`: `an_observation_with_no_check_refuses_as_empty`;
   `a_required_audit_missing_from_the_observation_refuses_as_missing_required`;
   `an_observation_of_another_head_repository_or_pull_request_refuses_as_unrelated`;
   `a_skipped_or_neutral_required_check_refuses_as_non_success`;
   `a_required_check_still_in_progress_refuses_as_not_settled`;
   `two_matches_for_one_required_check_refuse_as_ambiguous`;
   `a_forge_required_check_the_list_lacks_refuses`; `an_empty_required_list_refuses_to_compile`.
   The readout it produces: `inspect_names_the_class_the_refusals_and_the_faults`;
   `inspect_names_the_run_attempt_and_job_ids_a_person_re_runs`.
3. **0111c — the trusted launch and a GitHub observer script.** Files: the launch in
   `crates/brokkr-runtime/src/engine.rs` (a `ci` site's cleared `SpawnEnv::Exactly` table, its
   own working directory, created before the spawn and removed after, its digest-checked
   executables, the script fed over the engine's pipe, and the observation captured from
   stdout, bounded) and `crates/brokkr-protocol/src/adapters/exec.rs` (a working directory the
   site names, and the raw stdout channel); `crates/brokkr-protocol/src/process.rs` (execution
   of an opened descriptor on Linux, and the root-owned chain check on both systems);
   `crates/brokkr-runtime/src/bundle.rs` (the pinned interpreter and tools, the allow-list, and
   the lift of 0111b's live-admission refusal); run start's refusal of a `ci` seat whose bundle
   layer is inside the worktree; a house observer script, with its polling bound inside the
   seat's deadline, and its shellcheck clean. About 350 production, 150 script and 550 test
   lines. No contract change. Tests:
   `an_interpreter_replaced_after_verification_never_runs` (Linux: the hashed descriptor runs;
   both systems: a path whose chain the run's account can write refuses before the spawn);
   `a_tool_whose_directory_the_runs_account_can_write_refuses_before_the_spawn`;
   `a_pinned_path_that_is_a_link_into_the_worktree_refuses_before_the_spawn`;
   `a_pinned_path_whose_ancestor_is_a_link_refuses_before_the_spawn`;
   `the_script_reaches_the_interpreter_over_the_engines_pipe_as_the_bytes_the_manifest_digests`;
   `a_replaced_result_file_under_forge_results_is_ignored_and_the_stdout_observation_is_read`;
   `an_observation_over_the_stdout_bound_refuses_as_observation_too_large`;
   `a_non_utf8_observation_refuses_and_is_never_decoded_lossily`;
   `the_ci_site_journals_no_stderr_bytes`;
   `a_recorded_observation_over_stdout_is_sealed_classified_replayed_and_inspected`;
   `a_ci_seat_whose_layer_is_inside_the_worktree_refuses_at_start`;
   `an_inherited_bash_env_never_reaches_the_observer`;
   `an_observer_interpreter_whose_digest_moved_refuses_before_the_spawn`;
   `an_observer_tool_under_the_worktree_refuses_before_the_spawn`;
   `a_bare_interpreter_name_refuses_to_compile`;
   `the_observer_runs_from_a_directory_outside_the_worktree`;
   `the_launch_directory_is_removed_after_the_observer_ends`;
   `an_environment_key_outside_the_allow_list_refuses_to_compile`;
   `an_untrusted_launch_parks_ruled_as_launch_untrusted_and_observes_nothing`;
   `the_observer_reads_no_gh_configuration_from_the_worktree_or_home`;
   `the_observer_names_the_repository_and_never_reads_the_worktrees_remotes`;
   `the_observer_lists_every_check_for_the_head_unfiltered` (a stub `gh` given by absolute
   path, its output held to `ci-observation/v1` and to 0111a's evidence);
   `the_observer_refuses_as_not_settled_when_its_polling_bound_ends`;
   `the_observer_holds_no_write_binding`.
4. **0111d — the typed park and its marked rule.** Files: `crates/brokkr-core/src/park.rs` and
   its `parse` and `forms` modules; `crates/brokkr-runtime/src/engine/cursor.rs` (the marked
   rule's typed park); `crates/brokkr-runtime/src/bundle/` (the compile's marking of that rule,
   and the refusals of ruling 6's routes); and the readout it produces, the fleet's and
   `inspect`'s paint of the cause. About 150 production and 400 test lines, over test-built
   tables; no recipe yet. Contract: `park-cause/v2`. Tests:
   `a_runner_fault_parks_with_no_diagnosis_seat_attempt` (the typed park, asserted here with its
   writer: the `ci` gate's one exec attempt is journaled, and no `implement` or other model seat
   attempt follows it);
   `a_v1_infrastructure_cause_reads_kind_engine`; `a_ci_infrastructure_park_is_never_retryable`;
   `a_runner_fault_park_writes_the_ci_infrastructure_cause_and_keeps_the_rule_in_the_decision`;
   `a_refusal_park_carrying_a_ci_class_stays_ruled`;
   `a_different_table_park_carrying_the_ci_infrastructure_class_stays_ruled`;
   `a_table_with_no_or_two_marked_ci_infrastructure_parks_refuses_to_compile`;
   `exactly_one_class_flag_is_true_for_each_class_but_change`;
   `a_rule_naming_two_class_flags_refuses_to_compile`;
   `a_rule_whose_next_is_ci_refuses_to_compile`;
   `a_rule_into_classify_but_ci_pass_refuses_to_compile`;
   `no_rule_routes_to_a_rerun`; `a_ci_overlay_seat_declaring_a_write_binding_refuses_to_compile`;
   `the_fleet_paints_the_ci_infrastructure_cause`; and, when 0106d lands, its
   `a_non_retryable_infrastructure_park_spends_nothing` (0106:1224) extended with a
   `ci-infrastructure` park (ruling 8).
5. **0111e — the gate on `landing`, over a published head.** The first slice with a live
   producer. Lands only after 0109c moves `fast` to v3, and only if the operator grants ruling
   10's ask (or rules its option (b)). Files: `recipes/landing-ci/`, the overlay declaring ruling
   6's table, and its compile tests. About 100 data and 400 test lines. No contract of this
   decision's own. **Live read-only telemetry starts here**: from the first landing the operator
   drives under this overlay on a real pull request, each observation, class and park is
   journaled, and a sweep can count typed parks against diagnosis seat attempts. The park's
   lifecycle is this slice's too: a person re-runs, then the operator retries the parked `ci`
   phase, which observes again. Tests: the frame's two
   ([The gate grants nothing](#the-gate-grants-nothing)),
   `after_ci_pass_the_composed_table_is_landings_own_rule_for_rule` and
   `nothing_in_the_recipes_or_seat_scripts_reads_the_ci_phases_result`;
   `a_green_observation_passes_to_classify`;
   `a_change_failure_returns_to_implement_once`;
   `a_refusal_parks_ruled_and_never_passes`;
   `a_ci_infrastructure_class_never_routes_to_implement`;
   `an_unpushed_head_refuses_as_head_unpublished`;
   `a_pushed_head_after_a_head_unpublished_park_observes_again_on_retry`;
   `a_head_the_run_made_is_never_gated_by_ci`;
   `the_environment_park_survives_the_ci_overlay`;
   `an_operator_retry_after_a_person_re_runs_observes_again`;
   `the_composed_landing_ci_table_validates_whole_against_phase_machine_v3`.

## Evidence

Commands run in this seat at `e667a04c2`, each named above where it grounds a claim: the two
`grep` inventories in [What main holds at HEAD](#what-main-holds-at-head); the count of
`infrastructure(true` writers; and reads of `crates/brokkr-core/src/park.rs`,
`crates/brokkr-core/src/policy/inputs.rs`, `crates/brokkr-runtime/src/engine/cursor.rs`,
`recipes/fast/policy.json`, `recipes/fast/bundle.json`, `recipes/landing/README.md`,
`adapters/exec.json` and `contracts/phase-machine.v2.schema.json`. The forge's shape was not read.

The revision of fix round 1093r2 answered the review positions of run
`design-story-for-issue-1093-epic-93cc5725`. It read, at the same base, each file:line it newly
cites:

- `.github/workflows/ci.yml:3-26` and `:673-695`;
- `recipes/fast/bundle.json:25-100`, `recipes/fast/policy.json:30-100` and
  `recipes/verify/bundle.json:40-46`;
- `crates/brokkr-runtime/src/bundle/expand.rs:1-60` and
  `crates/brokkr-runtime/src/bundle/model_policy_tests.rs:2595-2625`;
- `crates/brokkr-runtime/src/bundle/assemble.rs:320-342` and
  `crates/brokkr-runtime/src/engine/decide.rs:68-86`;
- `crates/brokkr-runtime/src/engine.rs:3150-3180` and `:3217-3229`, and
  `crates/brokkr-runtime/src/anchor.rs:165-175`;
- `crates/brokkr-runtime/src/engine/cursor.rs:355-390`;
- `crates/brokkr-core/src/policy/conditions.rs:35-80` and `policy/inputs.rs:75-190`;
- `contracts/phase-machine.v2.schema.json:55-95` and
  `crates/brokkr-runtime/tests/frozen_contracts.rs:290-320`;
- decision 0106 (`:1205-1235`).

The revision of fix round 1093r3 answered the review positions of run
`fix-round-1093r2-for-issue-1093--9fd4243a` and the operator's ruling of 2026-11-18. The
re-run verbs' parameters are the controller's reading of
<https://docs.github.com/en/rest/actions/workflow-runs> on 2026-11-18; this seat read no forge
page in any round. Every forge fact cited by URL is unmeasured until 0111a. It read, at the
story's base, each file:line it newly cites:

- `.github/workflows/ci.yml:1-70`, `:300-325` and `:655-700`, and a `grep -n` of its job
  names, local actions and scripts;
- `recipes/fast/policy.json` (every rule, by `jq`), `recipes/verify/policy.json`,
  `recipes/brokkr-fast/bundle.json:4`, and a `jq` scan of every `recipes/*/policy.json` for an
  array-valued condition;
- `crates/brokkr-protocol/src/adapters/exec.rs:25-50`, `crates/brokkr-protocol/src/adapters.rs:756-853`
  and `crates/brokkr-protocol/src/process.rs:85-100`, `:295-310`;
- `crates/brokkr-runtime/src/engine.rs:2040-2056` and `:2860-2885`;
- `crates/brokkr-runtime/src/bundle.rs:3938-3968` and `:4147-4157`;
- `crates/brokkr-runtime/src/bundle/compose.rs:815-840` and `:868-922`, and
  `compose_tests.rs:600-650`;
- `crates/brokkr-core/src/policy.rs:258-278`, `:471-545` and `policy/conditions.rs:30-85`;
- `crates/brokkr-core/src/park.rs:25-70`, and `crates/brokkr-bridge/src/lib.rs:1-30`, `:210-229`;
- `contracts/phase-machine.v2.schema.json:25-80`;
- decisions 0049 (`:30-60`) and 0109 (`:1500-1530`, `:1830`).

The revision of fix round 1093r4 answered the review positions of run
`fix-round-1093r3-for-issue-1093--413225f3`. This seat again read no forge page or issue: the
`pull_requests` association, the pull request's `head.sha`, the commit-pulls listing,
`execveat` and statuses' lack of revision attestation are named from their documentation by URL
and stay unmeasured until 0111a. It read, at the story's base, each file:line it newly cites:

- `.github/workflows/ci.yml:25-27`, `:40-105` and `:225-330`, with a `grep -n` of its
  `pull_request.head.sha` and `pull_request.base.sha` references and of every job's display name
  in it and in `.github/workflows/mutants.yml`;
- `.github/branch-protection.json` whole, and `git ls-files .cargo .config .github
  rust-toolchain.toml rust-nightly-version.txt`;
- `scripts/test-shards.sh:19-47`, `:440-450` and `:553-602`, with a `grep -noE` of every local
  path it, `scripts/test-summary.sh`, `scripts/shard-runner.py` and `scripts/parallel-heaps.py`
  name; and `grep -n 'name = ".*yaml' Cargo.lock`, which printed nothing;
- `recipes/landing/README.md`, `recipes/landing/bundle.json`, `recipes/landing/policy.json` (by
  `jq`), `recipes/brokkr-fast/bundle.json`, and `recipes/fast/scripts/ship-seat.sh`'s `push`
  lines;
- `crates/brokkr-protocol/src/adapters/exec.rs:30-75`, `adapters.rs:756-770`, `:791-847` and
  `process.rs:295-310`, with a `grep` of `pre_exec`, `fexecve` and `execveat` under `crates/`;
- `crates/brokkr-runtime/src/engine/input.rs:66-84`, `engine.rs:1044-1052`,
  `places.rs:684-692`, `engine/dispatch.rs:145-166` and `bundle.rs:4212-4240`.
