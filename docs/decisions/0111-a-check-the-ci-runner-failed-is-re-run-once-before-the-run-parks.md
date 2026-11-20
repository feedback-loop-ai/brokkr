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
and the park is written `infrastructure` with `origin: ci` (ruling 7). These are one
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

**The outcomes beyond ordinary landing, stated once.** The gate adds three, and only three:

- **one bounded return to `implement`**, on `change`;
- **a typed park** for a person, `infrastructure` with `origin: ci`, on `ci-infrastructure`;
- **a `ruled` park** for a person: on `refused`, an observation that cannot be read as this pull
  request's settled checks (ruling 3), and on `launch-untrusted`, a launch the engine refuses
  before the spawn (ruling 2).

So `green` is not a verdict. It is the absence of those three. Every other part of this
decision points here rather than restating them. Every proof the gate takes below is a
precision guard on the `ci-infrastructure` class, and each fails closed to "not
infrastructure" (ruling 3); a refusal fails closed to a person's look.

**Every outcome of a wrong or forged observation is bounded:**

| A wrong … | Costs |
|---|---|
| `green` | landing as it is today: review, ship, and the forge's required checks at merge |
| `ci-infrastructure` | a typed park a person reads before re-running |
| `change` | one implement visit, under `fast`'s existing implement bound (ruling 6) |
| `refused` | a `ruled` park a person reads |

No row reaches a seat's input as authority either: later seats receive no projection of the
observation (ruling 6's projection).

**What this is not.** A change that weakens CI's own verdict, such as `deny.toml` widened to
admit a licence, is green on the forge as well. That is landing's existing exposure, and review
and the forge hold it, unchanged. It is not a residual of this gate, and this decision does not
claim to close it.

**Enforcement binding (0111e):**
`after_ci_pass_the_composed_table_is_landings_own_rule_for_rule` (the composed `landing-ci`
table, less its four `CI-` rules, its `initial` and its `ci` phase, equals `landing`'s composed
table rule for rule); and `nothing_in_the_recipes_or_seat_scripts_reads_the_ci_phases_result`
(a search of every recipe's bundle, policy and scripts, and of `scripts/`, for `ci_class`, the
three class flags and a `from: ci` decision finds only the four `CI-` rules). A text search
cannot prove what a seat reads, because a seat reads its built input, not the tree; so the
consumer input is tested too, over the engine's own input builder, by ruling 6's projection
tests.

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
write binding, and 0111c's compile refuses an observer credential declared with any `access`
but `read` (ruling 2).

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
  own, outside every worktree, by the walk [below](#the-account-is-the-trust-root) names, runs the observer
  there, and removes it after. No git configuration, `.envrc` or tool file in the worktree is
  beneath it.
- **A cleared, allow-listed environment.** The environment is cleared, then holds only a closed
  list: the read-only token's one variable, `GH_TOKEN` (below), `HOME`, `XDG_CONFIG_HOME` and `GH_CONFIG_DIR` naming
  empty directories the launcher made by that walk and holds by descriptor, `LC_ALL=C`, and an
  empty `PATH`. There is no `BASH_ENV`,
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
  interpreter over a pipe it owns, as `bash --noprofile --norc /dev/fd/3`. No boxed seat can
  reach the pipe's other end, and the change's tree holds no path to it, so neither can change
  the script after it is checked. A process of the account can; that is the trust root's reach,
  bounded below ([The account is the trust root](#the-account-is-the-trust-root)).
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

#### The account is the trust root

This is the one home of the rule; every other mention points here.

**The launch claims no isolation of the observer from any process of the account, whether
Brokkr launched it or not.** Fix round 1093r5 admitted the live launch only where this bundle's
sites were boxed and no realm in this invocation's map was unboxed. Both review positions of run
`fix-round-1093r5-for-issue-1093--a8bd2af6` showed that neither check sees a second Brokkr run
under the same UID: one started with another map (`--realms`,
`crates/brokkr-cli/src/cli_args.rs:220-225`; `World::discover`,
`crates/brokkr-runtime/src/realms.rs:409-412`), or one in the same `namespace` realm with a
site that has no hands, which HEAD spawns unboxed
(`crates/brokkr-runtime/src/engine/dispatch.rs:555-559`;
`crates/brokkr-runtime/src/engine.rs:2867-2869`, `:2040-2050`). In a read-only Linux probe, the
adversarial review of run `fix-round-1093r4-for-issue-1093--b1cf18e6` had such a same-UID
sibling open another process's anonymous pipe write endpoint through procfs
(`/proc/<pid>/fd/<n>`). The correctness position added that the r5 rule contradicted itself:
it required every site boxed, while a boxed observer cannot hold the credential binding (form
(c) below) and the plan launched the observer without hands.

Fencing is the wrong frame, for three reasons:

- **The account already holds what the fence would protect.** A process of the account Brokkr
  did not launch, such as the operator's own shell or another session, holds everything the
  engine holds, its journal and its secret store included. Decision 0046's ruling 4 puts every
  unboxed seat on the same footing: it runs with the engine UID's authority
  (`docs/decisions/0046-the-boundary-is-named.md:179-191`). The observer's credential is a
  sealed binding resolved from the operator-side store, an env-format file the account owns
  (`docs/decisions/0012-sealed-secret-bindings.md:33-37`). So an unboxed sibling can read the
  token from the store without going near the observer.
- **No check can enumerate the account.** A host registry of every run, realm and site
  sharing the UID is something Brokkr does not keep, and it would still not see the operator's
  own processes.
- **Within its own run, the observer has no neighbour.** `ci` is `landing-ci`'s entry phase
  (ruling 6): it runs before any seat of its run, and again only on the operator's retry of its
  own park, after every earlier seat has ended. No seat of the run runs beside it.

**What bounds the account's reach through the observer:**

- **The frame.** A forged or tampered observation costs at most a person's park or one bounded
  implement visit, never a merge ([The gate grants nothing](#the-gate-grants-nothing)); and no
  later seat's input carries the observation (ruling 6's projection), so a forged one reaches no
  model as text.
- **The credential.** It is read-only, scoped to the one repository's checks and Actions runs,
  and a sealed binding (0012), declared with a closed `access: read` the compile admits alone
  (ruling 5). It is never broader than what the account's unboxed seats can already read from
  the same store.

So a process of the account gains, through the observer, nothing beyond what it already holds
and that table.

**What the remaining launch rules guard: what is outside the account.** That is two things: the
change's tree, which the operator's push brings and anyone may have written; and boxed seats,
which HEAD's box confines with `--unshare-pid` among its isolation flags
(`crates/brokkr-protocol/src/hands/namespace/isolation.rs:4-23`), a `/proc` of its own
(`crates/brokkr-protocol/src/hands/namespace.rs:161`) and a cleared environment (`:154`). So a
boxed seat's `/proc` lists only its own PID namespace's processes, and the engine's and the
observer's `/proc/<pid>/fd` entries are not in it. That is read from HEAD's argv and was not
exercised in this session: **unmeasured** until 0111c's test, and if that test can open the
observer's endpoint from inside the box, the launch refuses under a boxed boundary until it
cannot. Rule by rule:

| Rule | Guards against |
|---|---|
| the cleared, allow-listed environment | the change's tree: no `BASH_ENV`, `ENV` or startup file names a worktree file; and no key a boxed seat could have planted reaches the observer |
| the verified bytes: pinned digests, the root-owned chain, the interpreter run from the descriptor it hashed, the script over the engine's pipe | the change's tree: no observer byte is a path the change controls; and a boxed seat whose binds can write a directory on that path |
| the working directory and `gh`'s configuration outside the worktree | the change's tree: no git configuration, `.envrc`, remote or tool file the change controls is read |
| the engine-owned result channel, the observer's stdout | the change's tree and boxed seats, which can write `.forge/results` under the worktree; neither can reach the pipe |
| the directories made and held by 1091a's walk | the change's tree, and a boxed seat whose binds reach a parent of them |

The directories: the observer's `HOME`, `XDG_CONFIG_HOME`, `GH_CONFIG_DIR` and working directory
are made by the same anchored, no-follow, one-descriptor-per-component 0700 walk 1091a builds
(#1091; in fix round at the story's base, so 0111c lands after it), and are held by descriptor
through use: the working directory is set by `fchdir` before the exec, and each configuration
directory reaches the observer as an inherited descriptor named `/dev/fd/<n>`, so no path is
resolved again after the walk. Whether `gh` accepts a `/dev/fd` directory for its
configuration is **unmeasured**; where it does not, the launch refuses.

**Recommended: (a) the account is the trust root, and the live launch is admitted on the same
terms under every boundary.** Under `harness`, as under `namespace`, the launch is admitted when
every rule above holds, unless the verified-bytes rule refuses on that system. The `ci` site has
no hands, so 0046's hands law, which judges only a site that declares hands, does not judge it
(`crates/brokkr-runtime/src/bundle.rs:2436-2443`). What macOS offers is the bytes rule's own
statement above: no `execveat` and an unmeasured `fexecve`, so the interpreter is executed by
its path under the root-owned chain, and the script over the pipe; where the chain does not
hold, the launch refuses. The offline classification (0111b) runs on both systems (0063).

- **(b) An account-wide fence: refuse while any unboxed site of any run shares the UID.** Not
  taken. It needs a host registry of runs and sites that Brokkr does not keep, and it still
  cannot see the operator's own processes, which hold the same store.
- **(c) Fix round 1093r5's `namespace`-only admission.** Removed, with its refusal word
  `launch-unboxed-peer`. Its checks could not see another run's map or a site without hands in
  another bundle, and it required every site boxed while the observer itself has no hands.
- **(d) Linux non-dumpable processes.** `PR_SET_DUMPABLE` 0 on the observer and on the
  engine's pipe holder makes their `/proc/<pid>/fd` entries and `ptrace` closed to a same-UID
  process without `CAP_SYS_PTRACE` (<https://man7.org/linux/man-pages/man2/prctl.2.html>; not
  read in this session). Under the trust root it buys nothing the store does not already give
  away. Costs: the engine's own diagnostics, no core dump and no `gdb`, `strace` or
  `/proc/<pid>/fd` read of the engine by the operator; and the attribute is expected to reset on
  `execve` (unmeasured), so `bash` cannot hold it. Not recommended; macOS has no equivalent and
  0063 needs both.

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

**Live admission is refused until the first live producer lands, and this is the one statement
of the order.** 0111b builds the classifier and the compile's `ci` declaration, but its compile
refuses every bundle that declares a `ci` seat, a `ci_observation` input or a `ci-checks.json`, a
custom exec recipe's included, with the reason that the gate is unbuilt. 0111c lands the
launcher and 0111d the typed park, each tested through builders and test-only bundles the live
compile never admits. **0111e lifts the refusal**, for `landing-ci` and a custom exec recipe
alike, in the same change that lands the first live producer, so no live compile admits a `ci`
seat before the launcher, the typed park and a composed table that validates whole all exist.
Refused alternative: lifting it in 0111c, which would admit a live `ci` seat whose
`ci-infrastructure` class has no typed park to write (0111d) and whose table validates under no
published schema (ruling 10).

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
- **Directories held by descriptor.** The descriptor-held 0700 walk is 1091a's, not yet on
  main. No boundary admission is needed: the account is the trust root
  ([above](#the-account-is-the-trust-root)).

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
as it is today. What the launch rules guard is the precision of `ci-infrastructure` against the
change's tree and boxed seats; what bounds the account is the trust root
([above](#the-account-is-the-trust-root)). The
credential is a sealed secret binding (0012), resolved by the runner into the one allow-listed
variable and never into argv; `gh` reads a token from its environment, an env-only consumer
0012's compile lint already admits (`crates/brokkr-runtime/src/bundle.rs:4149-4156`).
**The binding is named `GH_TOKEN`, and the compile refuses any other name.** HEAD's runner
exports a binding under its own name, with no mapping (`bind_environment`,
`crates/brokkr-protocol/src/secret.rs:123-130`), and `GH_TOKEN` is the variable the lint's own
comment names as `gh`'s input (`crates/brokkr-runtime/src/bundle.rs:4153-4155`). So the name
the operator declares is the variable `gh` reads; a binding called anything else would reach
`gh` as a variable it ignores, beside an empty configuration directory, and the observer would
read unauthenticated. No mapping layer is added. Its scope
is read-only and one repository's: Actions runs and jobs, checks, commit statuses, pull requests
and metadata, and administration only where ruling 3's forge-required set is read. It comes from
the same operator-side store every unboxed seat of the account can already read, so it is never
broader than what they hold. It is declared in ruling 5's file as `observer.credential`, a
binding name with `"access": "read"`, a closed enum of one word: the compile refuses any other
word, and any further secret binding on the `ci` seat. The compile checks the declaration, not
the token: the forge's permission names, and whether Brokkr can read a token's scopes, are
unmeasured; so a token with write scopes behind a `read` declaration is the operator's
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

**`ci-infrastructure` needs this pull request's run.** HEAD's workflow keys its concurrency by
ref (`.github/workflows/ci.yml:19-27`) and reads pull-request-specific data, the body and the
labels, into a required job (`:78-102`, where the `by-hand` label changes its outcome). So two
pull requests that share a head can each produce a run on it, with different results, and a run
keyed only by workflow path, event and head may be the other one's. Under the frame, another
pull request's run buys nothing a `green` buys; what it could buy is a park drawn from someone
else's setup fault. So the association is a precision guard on that one class, and a
withholding when unproven, never a refusal. **Recommended: (a) an Actions execution counts
toward `ci-infrastructure` only when the forge positively associates it with this pull request,
and with no other.** The field is the workflow run's `pull_requests` array, each element naming a pull
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
- **the array names this pull request and no other.** An association array that names more
  than one pull request is ambiguous, whether or not the others are still open: a pull request
  closed since the run cannot be told apart from the one that triggered it by the array alone
  (fix round 1093r4's rule read only the open list, so an array naming A and a since-closed B
  fitted A).

Otherwise the execution is not admitted, and `association-unproven` withholds the class, naming
the run, or `association-ambiguous`, naming every pull request the array names. A failure in
an unadmitted execution is still a failure: it makes the observation `change` (ruling 4), and
an all-success observation is still `green`. Refused alternatives: **(b)**
matching by workflow path, event and head alone, which is fix round 1093r3's key and admits the
other pull request's run; and **(c)** matching by the merge commit's parents, which cannot tell
two pull requests with the same base and head apart.

**Commit statuses are not admitted as required evidence.** A status names a context, a state
and a creator, and attests no revision: nothing in it says what its producer tested
(<https://docs.github.com/en/rest/commits/statuses>). HEAD's required checks are all Actions
jobs: each of the twelve contexts `.github/branch-protection.json:7-20` records (observed
2026-09-26, its `:3`) is a job's display name in `.github/workflows/ci.yml` or, for
`mutants in the diff: brokkr-core`, in `.github/workflows/mutants.yml:42`. So `required_checks` admits no `status` entry: the compile refuses one,
and where the forge's own required set names a status context, `status-required` withholds
`ci-infrastructure`: the gate cannot read that requirement, so the failure set it sees is not
known to be complete. Admitting statuses is a follow-up that needs a revision
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
  declared list, or `required-set-disagrees` withholds `ci-infrastructure`, for the same reason.
  The forge's set can only widen what the class needs, never narrow it. The token scope that read needs is unmeasured;
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

**The closure reads two sources, each the one GitHub ran.** The change classes are only as
good as the machinery's closure, so the engine tests it before it trusts it, over what the job
actually ran, read through 0109's verified reader (below) and never from the worktree. What ran
has two parts, and each has exactly one source:

- **The schedule comes from the event workflow.** Every step, `uses`, `run`, `env`,
  `working-directory`, `needs` and `strategy.matrix` the closure, the aggregate parse and the
  instance table read is parsed only from the **event workflow**: the workflow file at the
  revision GitHub ran for that run, as the run reports it. For `pull_request` the forge
  documents that revision as the merge revision
  (<https://docs.github.com/en/actions/writing-workflows/choosing-when-your-workflow-runs/events-that-trigger-workflows#pull_request>;
  unmeasured until 0111a), so for this house it is the merge commit, whichever tree a job then
  checks out. Its identity is the run's reported revision, the workflow's path, and the blob id
  0109's verified reader finds at that path in that revision. The checked-out tree's copy of the
  workflow is never parsed: it is a file the job may hold, not the schedule GitHub ran.
- **Local files come from the executed checkout.** Every script, local action (`uses: ./…` and
  its `action.yml`), implicit input, environment-selected file, read operand and include the
  closure follows is resolved in the job's **closure tree**, the commit its one proven checkout
  (below) holds: for a `merge` entry, the merge commit the run records, whose first parent is
  the base the observation names; for a `head` entry, the gated head.

Both identities are sealed, the event workflow's per run and the closure tree per judged job, in
the basis's instance table (below), so a replay reads which schedule and which tree were judged.
**When either cannot be established, the observation refuses as `source-unestablished`**,
naming the run or the job and which source ([No refusal weakens](#no-refusal-weakens)): the
expected instances `instance-duplicated` reads come from the event workflow, so an unknown
schedule leaves that refusal undecided. Where this decision reads the closure, the aggregate
parse or a matrix "at the base", it reads the schedule from the event workflow and the files
from that job's closure tree.

Why it matters: let the event workflow add a dependency job key, under a required aggregate,
that the checked-out head's copy of the workflow lacks, and let the forge report that key twice
in one attempt, beside a single failed required aggregate. The event workflow expects the key,
so `instance-duplicated` holds and the run refuses, `CI-REFUSED`. Parsed from the head instead,
the duplicate is an unexpected instance, which draws `change` with `matrix-incomplete` and routes
`CI-FAIL`: a person's ruled park would have become an implement visit.

- Every workflow file of an entry in the entry set (ruling 6's one derivation) is parsed, as its
  event workflow, into a typed subset: each job's
  `needs`, its `strategy.matrix`, its steps' `uses`, `run`, `working-directory` and `env`, the
  workflow's and the job's `env`, and the
  workflow's and the job's `defaults.run.working-directory`
  (<https://docs.github.com/en/actions/writing-workflows/workflow-syntax-for-github-actions#defaultsrunworking-directory>;
  not read in this session). The closure starts from each required job and every job it
  `needs`, transitively.
- A **reference** is a local file a reached step or file executes: a `uses: ./…` (and that
  action's `action.yml`, whose `runs` are read the same way); in a `run:` line, or in a reached
  script, the operand of `bash`, `sh`, `source`, `.`, `exec`, `python3`, `python`, `node`,
  `perl` or `ruby`, and any command word that is itself a repository path. Every reached file is
  scanned for references in turn.
- **A reached step's effective working directory lies in the machinery.** It is the step's own
  `working-directory`, else the job's default, else the workflow's, else the workspace root,
  `GITHUB_WORKSPACE`, moved by any `cd` the step runs. The workspace root is the checkout's
  root only when the checkout names no `path`: a checkout into `candidate/` leaves an ordinary
  step that names no directory in `GITHUB_WORKSPACE`, outside it. HEAD's `delivered by brokkr`
  checks out into `base-verifier` and `pr`, and its steps name those directories in their
  operands, never by default (`.github/workflows/ci.yml:49-77`). A `cd` whose operand is not a
  literal is unresolved. A
  directory that is, or lies under, a change class withholds as `machinery-escapes`, naming the
  directory and the step. A program run there reads what it finds there.
- **So do each command word's implicit inputs.** A program reads files no operand names: `make`
  reads `GNUmakefile`, `makefile` or `Makefile` in its working directory; `docker compose` reads
  `compose.yaml`, `compose.yml`, `docker-compose.yaml` or `docker-compose.yml`; `npm` reads
  `package.json` and `.npmrc`. A pinned table in `ci/closure.rs` names, for every command word
  and every remote action (`owner/repo`) the closure admits, its implicit inputs relative to the
  effective working directory, possibly none (`echo`, `test`, `printf`, `mkdir`), and, apart,
  its **product inputs**, the inputs that are the change under test (for `cargo`, the
  workspace members' manifests, build scripts and sources, and `Cargo.lock`; its own
  configuration, the root `Cargo.toml`, `.cargo/` and `rust-toolchain.toml`, is an implicit
  input). Every implicit input
  lies outside the change classes, or `machinery-escapes`
  withholds, naming the word, the input and the step. **A command word or a remote action the
  table does not name withholds as `machinery-unresolved`**: its inputs are not known, so they
  are not known to be machinery. The table is the engine's, one home, and a word it lacks fails
  closed; the closure verdict it gave is sealed in the basis, so a later table never moves a
  replay.
- **Product inputs are admitted only where evidence cannot type setup.** A product input may
  lie in a change class only for a step whose failure can never be setup evidence: a step that
  is not a declared setup step (ruling 4) and that supplies nothing a declared setup step
  consumes. A step supplies a declared setup step when it precedes it in the same job, or runs
  in a job the setup step's job `needs`, transitively, since an earlier step's files, outputs
  and environment, and a needed job's outputs and artifacts, are what a later step finds. So a
  declared setup step, and every step that supplies one, holds its product inputs to the same
  rule as its implicit inputs: each lies outside the change classes, or `machinery-escapes`
  withholds, naming the word, the product input (the changed path, where the range holds one,
  else the member's directory) and the step. A remote action that restores state another job
  of the run may write (`actions/cache`, `actions/download-artifact`) in a step that supplies a
  declared setup step withholds as `machinery-unresolved`: its writer is not bounded. A
  `cargo build` declared setup at the workspace root therefore withholds, and a changed member
  build script can no longer print the declared literal into a park. A `cargo test` after the
  last declared setup step of its job, in a job no setup step's job needs, keeps the exception:
  its failure is typed by its own step, which is not setup, so it is `untyped` and the change's.
- **So do the files a command word's environment selects.** A program also reads files its
  environment names: `make` reads every makefile `MAKEFILES` lists; `docker compose` reads the
  files `COMPOSE_FILE` lists, split by `COMPOSE_PATH_SEPARATOR` (`:` on Linux and macOS), and
  `COMPOSE_ENV_FILES`; `node` loads the `--require`, `-r` and `--import` operands of
  `NODE_OPTIONS`; `bash` sources `BASH_ENV`. The same pinned table names, for every command word
  it admits, every environment key that word reads as selecting a file, and the grammar of its
  value; any other key is one the table states the word does not read for a file. A command's
  **effective environment** takes each key from the strongest source that sets it. The order,
  stated once, strongest first:
  1. the command's own `NAME=word` assignment prefix, which binds that command alone;
  2. the step's `env:`;
  3. the job's `env:`;
  4. the workflow's `env:`;
  5. a literal `NAME=word` line an earlier step of the job wrote to `$GITHUB_ENV`, which reaches
     later steps only, never the step that wrote it
     (<https://docs.github.com/en/actions/writing-workflows/choosing-what-your-workflow-does/workflow-commands-for-github-actions#setting-an-environment-variable>;
     not read in this session, so GitHub's order among 2 to 5 is unmeasured until 0111b's
     fixtures pin it from a recorded run).

  A literal `export NAME=word` earlier in the same shell, the run line's or a script it sources,
  overrides 2 to 5 for every later command in that shell, and never 1: a command-local
  assignment outranks every export, for its own command. (The chief's shell probe in run
  `fix-round-1093r7-for-issue-1093--c1fa7d61` confirmed the local assignment wins; this seat's
  shell held no grant to repeat it.) An export in a script run as a child reaches only that
  script's own later commands. Each file a table-named key selects, normalized against the
  command's effective working directory into a repository-relative path, is an implicit input:
  it lies outside the change classes, or `machinery-escapes` withholds, naming the word, the
  key, the normalized file and the step; and it is
  scanned in turn as an implicit input is (below). A table-named key whose value is not a
  literal word (a `${{ … }}` expression, a `$VAR`, a substitution), whose value the key's
  grammar cannot split, or that an assignment form the grammar does not name sets (`env -S`,
  `declare -x`, an `export` of a non-literal, a `$GITHUB_ENV` write that is not one literal
  `NAME=word` line), withholds as `machinery-unresolved`, naming the key and the step. So does
  a `$GITHUB_PATH` write before a reached step, since it moves which file a command word names,
  unless it is a literal directory outside the change classes. `MAKEFLAGS` is table-named for
  `make`, and any value but the empty word is unresolved, since it may carry `-f`, `-C` or
  `-I`.
- **So do each command word's data operands.** A program also reads files its operands name
  without executing them: `test -f crates/ci/ready` run from the workspace root reads the
  change's tree as surely as a `make` there does. The same table names, for every command word it
  admits, which operand positions are paths it reads (for `test`, the operand of `-f`, `-e`,
  `-d`, `-s` and `-r`; for `cat`, `cmp` and `diff`, every operand; for `echo` and `printf`,
  none). Each such operand, taken relative to the effective working directory and resolved
  through verified tree entries (below), lies outside the change classes, or `machinery-escapes` withholds, naming the word, the operand and the step;
  one that is not a literal word is `machinery-unresolved`.
- **And so does everything an implicit input includes.** An implicit input is a reached file:
  it is scanned in turn, by the table's grammar for its format, for what it pulls in. A
  `Makefile`'s `include`, `-include` and `sinclude` operands, and each recipe line as a `run:`
  line; a compose file's `include`, `extends.file` and `env_file`. So a machinery `Makefile`
  that includes `crates/ci/setup.mk` escapes, as `machinery-escapes` naming the `Makefile`, the
  include and the step. An included path that is not a literal word, an implicit input whose
  format the table gives no grammar for, and a read or include form the grammar does not name
  (a `$(shell …)` or `$(wildcard …)` in a `Makefile`) each withhold as `machinery-unresolved`:
  what they read is not known, so it is not known to be machinery.
- **Every file a required job executes, reads as an implicit input, a product input of a step
  that supplies setup, an environment-selected file, a read operand or an include, or runs
  from, in its closure tree,
  lies outside the change classes,** or `machinery-escapes` withholds the class, naming the
  reference and the file that made it.
- **Every path resolves through verified tree entries, symlinks included.** A path's spelling
  is not what a program opens: `test -f ops/ci/ready` follows an unchanged tracked symlink
  `ops/ci/ready` to `../../crates/ci/ready`, so a change to the target alone reaches the step
  while every spelling, directory and machinery check passes. So each path the closure classes
  (an effective working directory, a reference, an implicit or product input, an
  environment-selected file, a read operand, an include) is resolved component by component in
  the job's closure tree through 0109's verified reader, never by the spelling and never on a
  filesystem: a tree entry of mode `120000` is a symlink, whose blob is its target, substituted
  relative to the entry's directory before the next component. Every path the walk visits, the
  spelled path, each link and each target, is classed: one that is, or lies under, a change
  class withholds as `machinery-escapes`, naming the spelled path, each link and the resolved
  target. A walk that leaves the closure tree, meets an absolute target, a gitlink (mode
  `160000`) or a component the tree does not hold (a file a step creates at run time), or
  follows more than 40 links, withholds as `machinery-unresolved`, naming the path and where
  the walk stopped. Cost, said plainly: a step that reads a directory or file an earlier step
  created, outside the tree, is unresolved, so it never types setup.
- **A reference the test cannot resolve fails closed,** as `machinery-unresolved`: an operand or
  command word built by an expansion, a command substitution, `eval`, or a glob, unless it is a
  variable the same file assigns only from literal words.

**The house's own closure does not resolve, so the house does not park today.** HEAD's required
`test (…)` aggregate needs `engine-machines`, which runs `scripts/test-shards.sh`
(`.github/workflows/ci.yml:255`). That script builds each shard's argv at run time, from
non-literal words (`"$bound"`, `"$work/$index.bound"`, `"${prefix[1]#+}"`), prepends a runner to
an argv another function filled, and executes the array itself as its command word:
`{ "${argv[@]}" >"$work/$index.log" 2>&1; }` (`scripts/test-shards.sh:558-575`). Each of those
is the unresolved category above. So the closure from `engine` through `engine-machines` to
`scripts/test-shards.sh` withholds as `machinery-unresolved`, naming that line, and **this
house's gate never draws `ci-infrastructure`** while it stands: a red house observation is
`change`, and an all-success one `green`. This decision does not claim the house parks.

It is not the only blocker. The required `mutants in the diff: brokkr-core`
(`.github/workflows/mutants.yml:42`) runs `bash scripts/mutants.sh gate "$BASE" brokkr-core`
(`:63`), and that script's line 31 is `cd "$(git rev-parse --show-toplevel)"`: a `cd` whose
operand is a command substitution, so its effective working directory, and every relative
reference after it, is unresolved. That closure withholds as `machinery-unresolved` too, naming
`scripts/mutants.sh:31`.

- **House typed parking is deferred** until both are written so the house's whole eligible
  closure resolves: each shard's command word and the files it executes in
  `scripts/test-shards.sh` spelled as literal words, or as variables the same file assigns only
  from literal words; and `scripts/mutants.sh`'s working directory set by a literal (the job's
  checkout root) rather than a substitution. Those rewrites are one follow-up issue under
  #1094, outside this decision and outside its slices, and it names every closure the house's
  integration case still finds unresolved when it is opened, not only these two.
- The synthetic matrix test (0111b) stays the classifier's positive case for a verdict-only
  aggregate over nine instances, over a workflow whose closure resolves, until the house's whole
  eligible closure resolves; it does not stand for the house.
- 0111b's house integration cases assert the exact withholding over HEAD's base, one per
  blocker.

The narrowed acceptance ([above](#what-1093s-acceptance-becomes)) is therefore delivered for an
adopter whose closure resolves, and for this house only after the follow-up.

Why the executed tree, and not the current base: the base moves. Start a pull request on A,
advance its base to B, where a setup helper is replaced with one whose reads close, and let the
pull request change only a product file A's helper reads. `merge-base..head` holds no machinery
edit, so `machinery-touched` is silent; a `head` entry's job checks out the head, which still
holds A's helper, and runs it. A scan of B passes while the executed helper reads the changed
file. Scanned on the head's tree, the closure finds A's helper and its read, and withholds as
`machinery-escapes`. A `merge` entry's tree has the base as its first parent, so with the
machinery untouched its machinery is the base's byte for byte, and the scan agrees with it.
Any change that could add a reference from the machinery into a change class must edit a
machinery file, which already withholds. The parse needs a YAML reader the
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
a change review reads, and every landing path reaches review (ruling 6). It never reaches a
declared setup step's evidence: the product-input exception above holds only for a step whose
failure cannot type setup.

- **(b) A conservative machinery list,** `.github/`, `scripts/`, `quality/` and the toolchain
  files, as fix round 1093r4 drafted. Refused: it is open, so `deny.toml`, `_typos.toml` and any
  root configuration it did not name were the change's.
- **(c) A per-entry definition list,** as fix round 1093r3 drafted. Refused: it missed HEAD's
  own helpers.

**`ci-infrastructure` needs the revision the run tested, established.** HEAD's pull-request workflow tells
three revisions apart: `delivered by brokkr` checks out the trusted base and the pull request's
head by explicit ref (`.github/workflows/ci.yml:49-64`), and the ratchet job asserts that its
checkout is the pull request's merge commit (`:663-671`). A checkout with no `ref`, as the audit
job's (`:681-683`), tests the forge's generated merge revision on a `pull_request` event
(<https://docs.github.com/en/actions/writing-workflows/choosing-when-your-workflow-runs/events-that-trigger-workflows#pull_request>;
unmeasured here). And a run's association SHA is not proof of what it checked out: a job in a
run on the head may check out the base (`:49-55`). So each required Actions entry declares
`tested`, one of two words, and the tested revision must be the exact one:

- `head`: the job tested the gated head itself. The run's `head_sha` and its association's
  `head.sha` must equal the gated head, and the event workflow, read by ruling 3's closure
  parse, must check that SHA out in the job: an `actions/checkout` step whose `ref` is
  the head-SHA expression `${{ github.event.pull_request.head.sha }}`, as `:61-64` spells it.
  A `head` entry whose job checks out no such ref, or more than one ref it could run from, has
  no established closure tree, and refuses as `source-unestablished`.
- `merge`: the job tested the forge's merge revision, `refs/pull/<number>/merge`. The event
  workflow's checkout in the job must take that ref, by default or by naming it. The
  observation must name the exact merge commit the run tested, as the run records it, and that
  commit's two parents. Its first parent must be the base the observation names, which must be
  the pull request's current base, and its second the gated head. The merge commit is a
  distinct commit from the head, as a valid merge always is.

**The house's three shapes, each specified so a valid case reaches classification:**

- **An aggregate with no checkout of its own.** HEAD's required `test (ubuntu-latest)` and
  `test (macos-latest)` are job `engine`, whose one step reads `needs.engine-machines.result`
  and checks nothing out (`.github/workflows/ci.yml:315-329`). Its tested revision is its pinned
  dependencies' exact revisions: every expected instance of each job it `needs` (ruling 4's
  matrix rule), each established by its own entry's rule above, and all one commit. A
  dependency whose revision is unestablished, or that disagrees with a sibling's, withholds the
  class for the aggregate. `engine-machines` checks out with no `ref` (`:228-231`), so it is
  `merge`. Note that `needs.engine-machines.result` is the result over all nine instances, both
  systems' (`:197-215`), so each system's `test (…)` verdict reads the other system's machines
  too.
- **A merge-mode run.** The merge commit and the head are compared separately, never as one
  commit: the run's `head_sha` and the association's `head.sha` against the gated head; the
  merge commit's second parent against the gated head; its first parent against the
  association's `base.sha` and the observation's base; and the merge commit against the one the
  run records.
- **`delivered by brokkr`,** with its trusted-base verifier and its explicit pull-request head
  (`:49-102`), is not in the entry set at all (ruling 6): its evidence is this landing's own
  output.

**Disagreement withholds.** Where any one of those comparisons fails, `revision-disagrees`
withholds the class, naming the two values that differ. It is never resolved by preferring one
source. (Fix round 1093r4's row demanded that the association, the run's head, the tested
commit and the gated head all be one commit, which no valid merge is; that row is replaced.)

**Which checkout a verdict-bearing step used.** A checkout the change's own code moves during a
job, such as a test that checks out another commit, is the change under test, which review reads:
it is not CI's machinery and this decision does not guard it. A checkout moved by machinery, a
workflow step or a script outside the change classes, is ruling 3's: if the change touched that
machinery, `machinery-touched` already withholds; if it did not, the event workflow's schedule
over the closure tree's files is what ran, and ruling 3's closure judges exactly those. What the forge would have to give, to prove beyond that which revision a verdict-bearing
step used, is a field naming the commit the run checked out. The run's `head_sha` is the pull
request's head, not the merge commit, and whether any field of the run or its jobs records the
merge commit tested is **unmeasured** until 0111a. **Where the forge exposes no such field, the
job's closure tree is not established, and the observation refuses as `source-unestablished`**
(ruling 3's two sources). This decision claims no proof GitHub does not give.

**One proven checkout, and every relevant step inside it.** A proven checkout proves the
revision of one directory, not of the step. A runner may hold other checkouts already: a
self-hosted runner's earlier workspace, or another `actions/checkout` into a `path` of its own,
as HEAD's `delivered by brokkr` checks out the base and the head into two directories
(`.github/workflows/ci.yml:49-64`). A step whose `working-directory` points at one of those ran
on its revision, and every comparison above can still pass. So, in the event workflow, by
ruling 3's closure parse:

- the rule judges a job that executes checkout contents: every job an entry or its aggregate is
  drawn through, **except a verdict-only aggregate** (ruling 4), which by its definition has no
  checkout and no `uses` step, and executes nothing but an assertion over `needs`. Its revision
  is not its own: it is derived recursively from every expected instance of every job it
  `needs`, each judged by this rule (or, if itself verdict-only, derived in turn), and all of
  them must be one commit, or `revision-disagrees` withholds, naming the two instances;
- each job the rule judges has exactly one `actions/checkout` step,
  and its `path` (the workspace root when it names none) is **the proven checkout**; a job with
  two, or with a `path` built by an expression, has no one closure tree, and refuses as
  `source-unestablished`;
- **the checkout completed before any step claimed to run in it.** The checkout step precedes,
  in the job's step list, every reached step the rule judges inside it, and the sealed
  observation reports that step concluded `success` in the same job attempt, its number and
  conclusion sealed with the job. A literal working directory inside the checkout's `path`
  proves no order: a step listed before the checkout ran in whatever that directory held, a
  runner's earlier workspace among them. A step listed before it, or a checkout step the
  observation does not report as succeeded, is unproven;
- every reached step of that job, the matched setup step and every step before it among them,
  has an effective working directory (ruling 3's rule: its own, else the job's default, else
  the workflow's, else the workspace root, moved by any literal `cd`, and resolved through
  verified tree entries, a symlinked directory as its target, by ruling 3's path rule) that is,
  or lies under, the proven checkout. An absolute directory, a `..` component, or one built by an expression
  (`${{ runner.temp }}`, `$HOME`) is not; nor is the workspace root, reached by default, when the
  proven checkout has a `path` of its own;
- an operand naming a file outside the proven checkout is not resolved by the closure at all,
  and withholds there.

Otherwise, where the job's one closure tree is established but a step is not proven inside it,
the entry is `revision-unmeasured`, naming the step and its directory, and no
`ci-infrastructure` is drawn.

The declaration is the operator's statement about a pinned workflow, and `machinery-touched`
withholds the class whenever that workflow moved from the base's bytes. Every required entry in
this house's list is `merge` (ruling 5), so until 0111a's evidence shows a forge field that
records the tested merge commit, and the revision a run's workflow was read from, every
observation over a `merge` entry refuses as `source-unestablished`, for this house or an
adopter, and the narrowed acceptance ([above](#what-1093s-acceptance-becomes)) waits on that
evidence; this house waits on the closure follow-up as well (above). If 0111a
finds no such field, the decision is revised before acceptance, and the acceptance is not
delivered on GitHub. That fails closed: every such observation is a person's `ruled` park,
never a weaker class.

**When `green` is drawn.** `green` grants nothing
([The gate grants nothing](#the-gate-grants-nothing)); the rule below says only when the gate
proceeds without either new outcome. The engine draws `green` only when all of these hold:

- the observation's repository, pull request and head equal the run's pinned repository, the
  pull request the gate names and the engine's own git head;
- every entry of `required_checks` in the entry set (ruling 6) is matched by exactly one settled
  check from its bound producer, in that producer's current execution;
- each matched check's conclusion is `success`;
- **no required failure is observed inadmissible** (below): no check that carries the name of
  an entry in the entry set from another producer, and no context the forge's own required set
  names, other than one an `"after": "ship"` entry declares, has concluded red.

**Present, absent and inadmissible are told apart.** A required entry is **present** only by an
**eligible current match**: a check from its bound producer (its app, workflow path, job and
event) in that producer's current execution, the one run attempt defined below. Nothing else
makes it present. A check from the same producer in another run, or in another attempt of the
current run, is **stale**, whatever it concluded: a stale `success` never satisfies the entry,
and a stale red is never an observed inadmissible failure (below), so neither moves the entry
from absent. A required entry with no eligible current match is **absent**, whatever else
carries its name, and so is a forge-required context no current observation reports. A
required failure is
**observed but inadmissible** when a check or status the gate cannot admit carries the
required name: a check under the
entry's name from another app, workflow path or event (`producer-mismatch`); a commit status
the forge requires (`status-required`); a context the forge requires that the pinned list lacks
(`required-set-disagrees`). An inadmissible check whose conclusion is red, `failure`,
`cancelled`, `timed_out` or `startup_failure`, is an **observed inadmissible failure**, and it
outranks both `green` and `missing-required`: the observation is `change`, never `green` and
never refused for absence. So a red commit status in the forge's required set beside every
pinned entry green is `change`; and a sole red check under a required entry's name from another
workflow is `change` with `producer-mismatch`, never `missing-required`. An inadmissible check
that succeeded changes nothing: the entry is still matched or absent on its own.

The association and the tested revision's comparisons are not in that rule: they are precision
guards on `ci-infrastructure` alone, and an all-success observation whose association is
unproven, or whose revisions disagree, proceeds as landing does without the gate. The two sources
are not guards: an observation whose event workflow or closure tree is not established refuses
as `source-unestablished` ([No refusal weakens](#no-refusal-weakens)).

The **current execution** of an Actions entry is the workflow run with the highest run number
among those the forge lists for its workflow path, its event and the head, read at its highest
attempt. Any other run for that key still queued or in
progress, or a run list the observer could not read whole, refuses as `execution-ambiguous`.
The sealed observation holds each check's run id and attempt and each key's whole run list, so
a replay re-derives which match is current, and which is stale, from the journal alone.

Nothing short of that is `green`.

#### No refusal weakens

This is the one home of the rule; every refusal below, and every step of ruling 4, cites it.
Fix rounds 1093r6 and 1093r7 each found a case where a fact a refusal should read was lost or
read from the wrong source (a stale green, then the head's matrix in place of the event
workflow's), so the run fell through to a weaker class and a person's ruled park became an
implement visit. Fixing each case alone leaves the next open, so the class is designed out once.
**Recommended: (a) refusals first, and an unestablished fact refuses.**

- **The refusal predicates are evaluated first,** every row of the table below, in its order,
  over facts sealed inside their bounds (the observation, and the basis's required list,
  comparison values and instance table). Every row that holds is named.
- **A fact a refusal predicate reads that is absent, overflowed, unproven, ambiguous, or parsed
  from a source other than the established one makes that predicate hold.** The run refuses,
  routed `CI-REFUSED`, with the row's word and a typed `unestablished` qualifier naming the fact
  and its state, one of `absent`, `overflowed`, `unproven`, `ambiguous` or `wrong-source`. It
  never falls through to a weaker class.
- **A weaker class, `change`, `ci-infrastructure` or `green`, is drawn only when every refusal
  predicate is established false:** every fact its row names is sealed, inside its bound, from
  the source its row names, and the predicate evaluates false over them.

So the observation's contract carries each fact a row reads as a value, or as the typed `unread`
with the observer's reason, never a default; a paged list carries whether it was read to its last
page; and each parsed fact carries the identity of the source it was parsed from, so a replay
checks the source as well as the value. An observation that lacks a field its contract requires
never reaches the table: the edge refuses it (ruling 3's parse), and the engine draws `refused`
with the edge's word, `observation-too-large`, or `observation-unreadable` naming the field, the
unknown word or the fifth excerpt. Those are journaled as the gate's refusal, as
`launch-untrusted` is, and a replay reads them there.

The withholdings are not refusals, and this rule does not reach them: an unestablished fact a
withholding reads already withholds, and fails closed to "not infrastructure". What this rule
guards is the one direction the frame does not: a refusal turned into `change`, which buys an
implement visit where a person's park was owed.

Cost, said plainly: an observation refuses wherever a fact is unknown, `green` included. Until
0111a measures how the forge names a matrix instance, and which fields record the revision a
run's workflow was read from and the commit a job checked out, every observation whose entry set
reaches a matrix job, or a `merge` entry, refuses; no live compile admits a `ci` seat before
0111e, which follows 0111a, and if 0111a finds either unmeasurable the decision is revised before
acceptance. An adopter whose
matrix is built by an expression cannot use the gate. **(b)** keeping each case's own fix, as fix rounds
1093r6 and 1093r7 did, is not taken: it leaves the next case open.

**The order, stated once.** `classify` takes these in this order, live and on replay alike, over
the one sealed observation and basis; ruling 4's numbered steps are this order and no other:

1. **every refusal predicate** of the table below, `missing-required` and `non-success`
   included: **`refused`**, naming every one that holds, each observed or `unestablished`;
2. **the explicit exception**, an observed inadmissible failure: **`change`**, with
   `inadmissible-failure`, because it is observed and red. It outranks no refusal:
   `missing-required`'s own predicate is false for an entry under whose name it is reported,
   and `non-success`'s is false wherever a red is observed;
3. then the classes: `green`, the withholdings, and the failure set.

So a required check that is missing beside an otherwise classifiable failure refuses as
`missing-required`; it is never typed. A required audit with no check in its current run, an
older same-producer `success` on the same head, and a second, eligible required job with an
ordinary failed test is `refused`, `missing-required`, routed `CI-REFUSED`: the stale success
never makes the audit present, and the test failure is never reached. The same holds when the
second job's failure is all-setup: `refused`, never `ci-infrastructure`.

The refusal predicates, each with the facts it reads, where each is sealed, and its unestablished
outcome. *Observation* is the sealed `ci_observation`; *basis* is the `ci_class` record's basis.

| Refusal | Holds when | Facts it reads, and where each is sealed | Unestablished |
|---|---|---|---|
| `unrelated` | the repository, the pull request or the head differs from the binding | the repository's owner, name and id, the pull request's number and the gated head (observation); the run's pinned repository, the gate's pull request and the engine's git head (basis, comparison values) | `unrelated`, naming the fact |
| `head-unpublished` | the forge's head for the pull request is not the engine's git head: the head the run would gate is not the one pushed | the forge's `head.sha` (observation); the engine's git head (basis) | `head-unpublished`, naming the fact |
| `empty` | the forge reports no check for the head | the check list, and whether it was read to its last page (observation) | `empty`, `absent`, when no check was read and the list was not read whole |
| `missing-required` | a required entry has no eligible current match, a check from its bound producer in its current run attempt, and no observed inadmissible failure is reported under its name; a stale same-producer check of any conclusion never counts; it names the entry | the pinned required list (basis); the check list, read whole, each check's producer, run id and attempt, and each key's whole run list (observation) | `missing-required`, naming the entry and the fact |
| `ambiguous-required` | a required entry matches more than one check in its current execution | the same facts as `missing-required` | `ambiguous-required`, naming the entry and the fact |
| `execution-ambiguous` | a required entry's current execution cannot be told, as above | each key's whole run list, with each run's status and attempt (observation) | `execution-ambiguous`, naming the key |
| `source-unestablished` | a source the classifier parses or resolves against is not established as the one GitHub ran (ruling 3's two sources): a current run's event workflow, or a judged job's closure tree | each current run's reported workflow revision, its workflow path and that file's blob id, and each judged job's one proven checkout and its commit (observation; sealed again in the basis's instance table) | `source-unestablished`, naming the run or the job and which source |
| `instance-duplicated` | one expected matrix instance's key is observed more than once in one run attempt; it names the key and the jobs | each matrix dependency's expected instance keys, parsed from the event workflow, with that source's identity, and the forge's instance naming (basis, instance table); each observed job's name, run id and attempt (observation) | `instance-duplicated`, naming the dependency and the fact: an unexpandable matrix, unmeasured naming, or an instance table parsed from another source |
| `instances-too-large` | the instance table exceeds its bound, so `instance-duplicated` cannot be decided; it names the measured count | the instance table's overflow witness (basis) | this row is the `overflowed` state of `instance-duplicated`'s fact, and holds whenever the witness does |
| `not-settled` | a required check is still queued or in progress when the observer's own polling bound ends, short of the seat's deadline | each matched check's status (observation) | `not-settled`, naming the check |
| `non-success` | no check in the failure set and no observed inadmissible failure is red, and a matched required check concluded `neutral`, `skipped`, `stale` or `action_required`; it names the check and the word | each matched check's conclusion, and every red the observation holds (observation) | `non-success`, naming the check |

A refusal never passes. It is the frame's `ruled` park
([stated once](#the-gate-grants-nothing)), with the rule's id and the refusals' words.

**What withholds `ci-infrastructure`.** A withholding is not a refusal: it parks nothing and
stops nothing landing would do. Each is a precision guard on the one class, and fails closed to
"not infrastructure": where one holds, a red observation is `change` and an all-success one is
`green`. The engine checks every row and records each that holds in the class's sealed reason,
with the named values the basis's reason table seals (below), so every reason replays; a reason
whose values an overflow hid is recorded `unavailable`, by the same rule live and on replay.

| Withholding | When |
|---|---|
| `machinery-touched` | a changed path lies outside every change class; it names the paths |
| `machinery-escapes` | in a job's closure tree, under its event workflow's schedule, a file a required job executes, a command word's implicit input or read operand, a file its effective environment selects, a product input of a declared setup step or of a step that supplies one, a file an implicit input includes, or a reached step's effective working directory lies inside a change class, on any path its tree-entry walk visits, a symlink's target included |
| `machinery-unresolved` | in a job's closure tree, under its event workflow's schedule, a reached file holds a reference, read operand or include the closure test cannot resolve, a file-selecting environment key or assignment the table cannot resolve, a state-restoring action supplies a setup step, a path's tree-entry walk cannot complete, or a command word, remote action or implicit input format the pinned table does not name |
| `range-unproven` | 0109's verified reader cannot establish the range's provenance and its complete, byte-exact path list |
| `range-too-large` | the range's changed paths exceed the bound the classification record carries |
| `basis-too-large` | the classifier's sealed basis would exceed its bound |
| `association-unproven` | an execution is not positively associated with this pull request, its head and its base; it names the run |
| `association-ambiguous` | an execution's association array names more than one pull request, open or closed; it names them |
| `revision-disagrees` | one of the separate comparisons above fails; it names the two values |
| `revision-unmeasured` | a relevant step's effective working directory is not inside the job's one proven checkout, or the step precedes that checkout's observed success; it names the step. A closure tree or event workflow that is not established is not this row's: it refuses as `source-unestablished` |
| `producer-mismatch` | a check carries a required entry's name from another app or workflow path; it names it, so a substitute is never silently passed over |
| `status-required` | the forge's required set names a commit status context, which is never admitted |
| `required-set-disagrees` | the forge requires a check the pinned list lacks |
| `matrix-incomplete` | a verdict-only aggregate's dependency is missing an expected instance, from its event workflow's matrix (ruling 4), or holds one whose values match no expected entry; it names them. A matrix that cannot be expanded, or whose instances cannot be named, is not this row's: it leaves `instance-duplicated` unestablished, and refuses ([No refusal weakens](#no-refusal-weakens)) |

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
- **the change classes** and what ruling 3's closure test took at each closure tree: its verdict,
  and, for each required job and each job it needs, transitively, its `needs`, its checkout
  ref, its closure tree's commit, its proven checkout's path and step number, and each reached
  step's effective working directory;
- **each such job's aggregate kind**, as ruling 4's pinned parse derived it: `verdict-only`,
  with the dependency each of its steps asserts and the `env` binding that ties the assertion to
  `needs.<dep>.result`; or `independent`, naming its first independent step; or the typed
  `kind-unmeasured`, naming the step the parse could not read;
- **the instance table**, sealed apart from the closure's job map and inside its own bound
  (below): **the two sources** (ruling 3), each current run's event workflow identity (its
  reported revision, path and blob id) and each judged job's closure-tree commit, or the typed
  `unestablished` naming which, so `source-unestablished` replays; and **each matrix
  dependency's expected instance identities**, the event workflow's
  `strategy.matrix` expanded into its fixed list of instance keys, with the event workflow
  identity it was parsed from, each the matrix values in
  their declared order, each with the revision its own entry rule established for it, so a
  verdict-only aggregate's derived revision replays, with the forge's name for it once 0111a
  measures how the forge names an
  instance; or the typed `matrix-unexpandable`, naming the expression or `fromJSON` that built
  it; or `instance-naming-unmeasured` while 0111a has not measured the naming. Each of the
  latter two leaves `instance-duplicated` unestablished, so a replay draws `refused`,
  `instance-duplicated`, `unestablished`, from the outcome alone
  ([No refusal weakens](#no-refusal-weakens));
- **the comparison values** the binding read: the repository's numeric id, the pull request's
  number, the base commit, the merge base, the gated head, the engine's git head and the forge's
  head for the pull request;
- **the reason table**, sealed apart from the path list and the closure's job map and inside its
  own bound (below): the facts every named withholding needs, so an overflow of either list
  never takes a reason's names with it. Its rows: **the machinery paths**, every changed path
  outside every change class, which the verified reader collects as it streams the range, so
  `machinery-touched` names its paths even when the full list overflows; and **the closure's
  findings**, one row per `machinery-escapes`, `machinery-unresolved` or `revision-unmeasured`
  the closure test found, each with the word, key, file, link, directory and step it names. A
  `revision-disagrees` needs no row: its values are the observation's and the instance table's
  (ruling 3's two sources);
- the range's two commit ids, and **the range read's outcome**, one of three closed forms:
  `verified`, with the changed-path list 0109's verified reader gave for that range, possibly
  empty; `overflowed`, with the overflow witness below; or `unproven`, naming the reader's end
  (`read-failed`, `delta-incomplete`, or `reader-unbuilt` while 0109b is not built) and no path
  list. So an unreadable range and a verified empty one, over the same two commit ids, seal
  different bases: the first replays `range-unproven`, and the second replays as a change that
  touched no path, never the other.

A replay is `classify(observation, basis)` over the sealed observation and that record alone,
and must equal the recorded class. It reads no bundle, no manifest file, no checkout, no Git
range and no forge, so a replay after the bundle is recompiled, the worktree is gone and the
forge's runs have expired still holds. The bounds: the compile admits at most 64 required
entries, 64 setup steps and 64 patterns, so the list and the literals are at most 32 KiB; the
closure's job map, its aggregate kinds included, is at most 256 jobs
**and 128 KiB** in its canonical JSON; the instance table is at most 1,024 expected instances,
each job at most 64, **and 64 KiB**; the reason table is at most 1,024 rows **and 64 KiB**;
the path list is at most 4,096 paths and 256 KiB. The closure has a byte bound as well as its count, so 256 jobs
with long step lists cannot carry an unbounded basis. A basis beyond its bound withholds the
class as `basis-too-large`, and a range beyond its own as `range-too-large`, with ruling 3's
withholdings; neither is ever truncated. An instance table beyond its bound refuses as
`instances-too-large` (ruling 3), because without it `instance-duplicated` cannot be decided.

**What an overflow keeps, and the precedence over it.** Every check ruling 3's order ranks above
`basis-too-large` (every refusal predicate, the explicit exception and `green`) reads only the sealed
observation, the pinned required list, the comparison values and the instance table, never the
closure's job map. So a closure overflow replaces the job map alone with its witness (below);
the instance table stays sealed whole beside it, inside its own bound. `instance-duplicated`
therefore refuses ahead of `basis-too-large`, live and on replay alike: an observation over an
over-128-KiB closure that holds an expected instance's key twice in one attempt is `refused`,
`instance-duplicated`, routed `CI-REFUSED`, from the instance table the replay reads, and never
`change` with `basis-too-large`.

**Every withholding reason replays, overflow included.** A withholding is never a refusal, so
none outranks another: where any holds, the class is `change`, `withheld`, and the reason lists
every one, size withholdings included, each in one of two closed forms:

- `holds`, with its named values, read from the reason table, the observation or the instance
  table, never from a list an overflow replaced;
- `unavailable`, naming the witness that hid its values and, where the stream found any before
  it stopped, the values it found, marked a lower bound. That happens only when the reason table
  itself overflows (its witness is `reasons`), or when the range read reached its hard cap, so
  the machinery paths past the cap were never read.

Live and replay take the one function over the one basis, so a reason the replay cannot name was
not named live either. So a 4,097-path range that includes `deny.toml`, beside a red all-setup
observation, is `change`, `withheld`, with `range-too-large` (the `paths` witness, `measured`)
and `machinery-touched` `holds` naming `deny.toml`, from the reason table, live and from the
journal alone. And a closure over 128 KiB beside one dependency instance whose closure tree is
another commit is `change`, `withheld`, with `basis-too-large` (the `closure` witness) and
`revision-disagrees` `holds` naming both instances and both commits, from the instance table.
Under ruling 3's refusal rule none of this reaches a refusal: every fact a refusal reads is in
the observation, the required list, the comparison values or the instance table, and an
`instances` witness refuses.

So a replay of a valid all-setup aggregate reads its kind, `verdict-only`, and its nine expected
keys from the basis, finds all nine in the sealed observation, and re-draws
`ci-infrastructure`; and a replay of the same observation less one instance finds eight of the
nine sealed keys and re-draws `change` with `matrix-incomplete`, naming the missing key. Neither
reads the workflow, the bundle or the forge.

**A size withholding replays from the journal too.** A list over its bound cannot be sealed whole
under the rule that forbids truncating it, so the basis seals, in its place, a bounded, typed
**overflow witness**, never the list:

- which bound it is: `paths` (the range's changed-path list), `closure` (the closure's job
  map), `instances` (the instance table) or `reasons` (the reason table), the four parts of the
  basis the compile does not already bound;
- the limit, as a count and in bytes (4,096 paths and 256 KiB; 256 jobs and 128 KiB; 1,024
  instances and 64 KiB; 1,024 rows and 64 KiB);
- the measurement, one of two closed forms:
  - `measured`: the full count and bytes, and the SHA-256 digest of the full list in its
    canonical form (for paths, the verified reader's sorted list with each path's bytes
    NUL-terminated; for the closure and the instance table, its canonical JSON);
  - `overflowed-unmeasured`: the count and bytes read when the hard cap stopped the read, as a
    lower bound, and no digest.

**How an overflow is measured without unbounded work.** The verified reader never holds an
overflowing list. Past the sealing limit it keeps reading, streaming each path, or each closure
job's or expected instance's canonical bytes, into a running count, byte total and SHA-256, and
holds nothing else. That stream has its own hard read cap, a fixed multiple of the limit: 65,536
paths and 4 MiB for paths, 4,096 jobs and 2 MiB for the closure, 16,384 instances and 1 MiB for
the instance table, 16,384 rows and 1 MiB for the reason table. A list that ends within the cap seals
`measured`. A list that reaches the cap stops there and seals `overflowed-unmeasured`, so no
rejected input costs more than the cap. The sorted order a `measured` path digest needs is the
reader's own order: 0109b's `diff-tree` over verified trees walks paths in tree order, which is
sorted (unmeasured until 0109b's tests pin it); where the reader cannot give sorted output
without holding the list, the path witness is `overflowed-unmeasured` past the limit, and that
fails closed the same way.

Everything else the basis holds is sealed as usual. `classify` reads a witness as its
withholding or refusal, at its own place in ruling 3's order: an `instances` witness is the
step 1 refusal `instances-too-large`, a `paths` or `closure` witness draws
`range-too-large` or `basis-too-large` at step 4, after every check above it has read what the
witness left whole, and a `reasons` witness draws `basis-too-large` at step 4 with each reason
it hid `unavailable`. So the
replay re-derives the class without the list, and a `measured` digest lets a person who
still has the range confirm which list it was. A witness whose measured count and bytes, or
whose lower bound, are both within its limit is malformed and refuses the replay, so a witness
cannot stand in for a list that fit. The witness is at most a few hundred bytes. Fix round 1093r4 specified only the in-bound replay; with
the witness, every class replays from the journal alone, both size withholdings included.

Ruling 2's launch refusal, `launch-untrusted`, is not `classify`'s: it holds before any
observation exists. It is the engine's pre-spawn fact, journaled as the park's reason with the
condition that failed, and a replay reads it there.

- **(b) A new `ci/classified` event.** Moves the event envelope, every manifest and every fold
  arm for a fact an input can carry. Refused, as 0106 ruling 1 refused `run/park-caused`.
- **(c) The excerpt kept outside the journal by digest.** Refused: the journal would point at
  log bytes kept somewhere else, and a replay would need them. No log byte is kept at all
  (below).

**No raw log byte enters the journal.** The journal records only what matched. An excerpt is
read only for a failed step of a failed required job, or of a failed dependency of a required
aggregate (ruling 4), and only when that step is one ruling 4's list declares as setup; a
failed step that is not declared setup is typed `untyped` by its declaration alone, and no log
of it is read. It lives only in the engine's memory, between the observer's stdout (ruling 2)
and the edge parse:

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
  job, or a failed dependency of a required aggregate, is typed setup only when every one of its
  failed steps is a declared setup step whose bounded excerpt holds one of that step's
  literals, and the observation parks only when its whole failure set is setup (the classifier,
  below). The issue's port-bind case is one:
  the step that boots the compose services, declared setup, with the literal
  `address already in use`. The adopter's step name and excerpt are unmeasured.
  - **The bounded cost, said plainly:** ruling 3's closure keeps the change out of a declared
    setup step's inputs, its product inputs, its environment and the steps that supply it
    included, so a change reaches that step's evidence only where the pinned table is wrong
    about what a program reads. Such a change, making its own failure print a declared literal
    in a declared setup step, reaches a person's park, not a pass. A service bootstrap that
    starts a compose file in a change class, or a `cargo build` declared setup, withholds
    instead; HEAD's checkout-local action `uses: ./.github/actions/setup-cargo-audit`
    (`.github/workflows/ci.yml:681-689`) is machinery, so a change to it is `machinery-touched`.
    The person reads the evidence and decides; nothing passes.
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
as an empty change. In ruling 3's one order:

1. Any refusal predicate of ruling 3's table holds, observed or unestablished
   ([No refusal weakens](#no-refusal-weakens)), `missing-required` and `non-success` among them:
   **`refused`**, naming every one that holds. Nothing below runs, so no refusal is ever
   outranked by a classifiable failure. A stale success never satisfies `missing-required`.
2. **An observed inadmissible failure** (ruling 3: a red check under an entry-set entry's name
   from another producer, or a red context the forge requires that the gate cannot admit and no
   after-ship entry declares; never a stale check of the entry's own producer):
   **`change`**, with the reason `inadmissible-failure`, naming the check and its withholding.
   It outranks `green` below, because it is observed and red. It outranks no refusal: step 1
   established each false.
3. The observation passes ruling 3's rule for `green`: **`green`**.
4. **Any withholding of ruling 3 holds: `change`,** with the reason `withheld`, naming every
   withholding that holds. No pattern is read, so nothing below can draw `ci-infrastructure`.
5. **The whole failure set is typed.** The failure set is every job of an entry in the entry
   set (ruling 6) whose conclusion is not `success`, each instance of a matrix job on its own. Every element gets one closed type,
   matched without a wildcard:
   - **setup-pattern**: conclusion `failure`, **at least one** failed step reported, and
     **every** failed step of the job a declared setup step whose sealed match names one of that
     step's literals, with the pattern's id. A failed job that reports no failed step satisfies
     no setup predicate vacuously: it is `untyped`, and beside a matched fault in another job it
     makes the observation `change`, `untyped-failure`.
     That includes a later step that runs after the first failure under `if: always()` or
     `if: failure()`, as HEAD's mutants artifact upload does
     (`.github/workflows/mutants.yml:64-68`): if it fails and is not a matched setup step, the
     job is `untyped`;
   - **setup-via-needs**: a pinned verdict-only aggregate (below) with conclusion `failure`,
     whose dependencies' expected instances are all observed, every one that did not succeed is,
     in turn, setup-pattern or setup-via-needs, and at least one failed;
   - **cancelled**: conclusion `cancelled`, the required job's own or a dependency's through an
     aggregate. Who cancelled is unmeasured, though on a pull request a newer push cancels the
     older run (`.github/workflows/ci.yml:25-27`);
   - **untyped**: anything else: a `failure` no declared literal matches, `startup_failure` and
     `timed_out` (a hang may be the change's), and `neutral`, `skipped`, `stale` or
     `action_required` beside a failure, so a skipped required check never rides along with a
     setup fault into a park.

   **Inadmissible evidence has one route, `change`.** A red check ruling 3 cannot admit under a
   required name is step 2's; a failure in an execution ruling 3 does not admit (an unproven or
   ambiguous association, an unestablished, disagreeing or unmeasured revision) is step 4's.
   Neither reaches this step, and either way the observation is `change`, exactly, never
   `refused`, never `green` and never `ci-infrastructure`. (Fix round 1093r4 parked that case
   `ruled`; that route is removed.)
6. **Only an all-setup failure set parks.** Every element setup-pattern or setup-via-needs, and
   the set non-empty: **`ci-infrastructure`**, naming each job, its steps and their pattern ids,
   with the workflow run's id and attempt a person needs to re-run it.
7. Otherwise **`change`**, with a closed reason naming the first element that is not setup, in
   the set's order: `cancelled`, or `untyped-failure`. One such element beside any number of
   setup faults makes the whole observation the change's.

**An aggregate is followed only where it is verdict-only.** HEAD's required `test (…)` jobs are
an aggregate: job `engine` `needs: engine-machines`, runs `always()`, and its one step asserts
`needs.engine-machines.result` is `success` (`.github/workflows/ci.yml:315-329`). A port bind
during service boot in an `engine-machines` instance therefore fails `engine` in a step that
prints no setup literal, and read alone it is `untyped`. So such a job is typed through its
`needs`, but only when, in its event workflow as ruling 3's closure parse took
it, it is a **verdict-only aggregate**: it has no `uses` step and no checkout, and every one of
its steps is a single assertion that a dependency's result is `success`, of the form
`test "$NAME" = success` with `NAME` bound in that step's `env` to exactly
`${{ needs.<dep>.result }}` for a `<dep>` in its `needs`, as HEAD's one-step `engine` job is.
A job with any independent step is not one: each of its own failed steps is typed by the rules
above, so an independent assertion that fails is `untyped` and makes the observation `change`
whatever its dependencies did. Fix round 1093r4 followed any job with `needs` and discarded its
own failed step; that is removed.

The classifier then finds the aggregate's dependencies in the same workflow run and attempt,
and types each instance that did not succeed by the same rules, recursively. **Every expected
instance is accounted for.** A dependency's expected instances come from its event workflow's
`strategy.matrix` (ruling 3's two sources), never from a checked-out copy: for HEAD's `engine-machines`, its nine `include` entries, five Linux and four
macOS machines (`:192-215`). An observation missing any
expected instance, or holding one whose matrix values match no expected entry, withholds the
class as `matrix-incomplete`, so a sparse observation cannot look all-setup. **The binding is
one-to-one**: an expected instance's key observed more than once in one run attempt refuses the
observation as `instance-duplicated` (ruling 3), so a contradictory duplicate cannot stand
beside its sibling, whichever one the classifier would have read. A cancelled sibling
is observed, and is `cancelled`, which makes the observation `change`. A matrix the parse cannot
expand into a fixed list, because it is built by an expression or `fromJSON`, leaves the expected
keys unknown, so `instance-duplicated` cannot be established false and the observation refuses
([No refusal weakens](#no-refusal-weakens)). How the forge names an instance, and so how an
observed job is matched to its expected entry, is **unmeasured** until 0111a; until it is
measured every observation that reaches a matrix dependency refuses the same way.

This is safe because the event workflow is the one GitHub ran, by its sealed identity (ruling 3
refuses where it is not established, and withholds the class on a changed one as
`machinery-touched`), and because the type only parks: a mistyped aggregate
costs a person's look, never a pass. A verdict-only aggregate's own failed step is taken as its
dependencies' consequence, not read, because it holds nothing else.

A change that touches CI's machinery never reaches step 5: step 4 makes it the change's, as
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
              "jq": {"path": "/usr/bin/jq", "sha256": "<64 hex>"}},
    "credential": {"binding": "GH_TOKEN", "access": "read"}
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
be a required entry's job, or a job a required aggregate needs in its event workflow, transitively; the
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

`required_checks` must be non-empty and hold at least one entry with no `"after"` (ruling 6's
entry set is never empty), every Actions entry must name all of its producer's parts
and its `tested` word, and may carry `"after": "ship"` (ruling 6's entry set; the only word
v1 admits, so the field is a closed enum of one), and `change_classes` must be non-empty and hold no path that is, or lies
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

These are the frame's three outcomes beyond ordinary landing
([stated once](#the-gate-grants-nothing)), and `green`.

**The `ci` result's projection.** A rule is not the only reader of a phase's result: a seat
reads its built input. HEAD hands every seat but triage its run context: `last_decision`, the
decision that brought it here; `results`, the result and inputs of every phase earlier in the
table's order; and, on a return, `returned_from` (`crates/brokkr-runtime/src/engine/input.rs:110-168`).
A phase result carries its decision's inputs (`crates/brokkr-core/src/decision.rs:113-126`).
Unchanged, that would hand `ci_observation`, `ci_class` and its flags to `classify` (its
`last_decision` after `CI-PASS`) and to `verify`, `review` and `ship` (`results.ci`, because
`ci` precedes them in the order above), where a model reviewer could weigh a `pass` as CI's
verdict. **Recommended: (a) the projection is fixed, in the one input builder:**

- `results` never holds `ci`, for any seat;
- a `last_decision` or `returned_from` whose phase is `ci` carries its rule id and result word,
  and no `ci_*` input;
- with one exception: `implement`, entered by `CI-FAIL`, receives `ci_class` alone, the class
  `change`, its closed reason, and the workflow run's id and attempt and each failed job's id, so
  it knows what to read on the forge. That is a reason to work, not a verdict to rely on, and it
  is typed and bounded;
- no seat ever receives `ci_observation`. The review seat receives nothing of `ci` on any road.

So the `ci` result has exactly one projection, the class to `implement` after `CI-FAIL`, and
nothing a later seat reads can stand for CI's verdict. **(b)** projecting it unchanged: a model
seat would read the observation and a `green` it could take as authority. Not recommended.
**(c)** projecting nothing, `implement` included: the implementer would diagnose a red CI
without the job ids a person would give it. Not recommended.

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

**Where `ci` sits in the phase order.** The order is not cosmetic. HEAD orders every transition
by phase position (`crates/brokkr-core/src/policy/direction.rs:10-19`), and the entry check
refuses an entry from which a forward rule reaches a work seat
(`crates/brokkr-runtime/src/bundle/entry.rs:166-170`, `:186-209`). Landing's order is
`classify`, `implement`, `verify`, `review`, `ship`, `done`, `stop` (`recipes/landing/policy.json`).
Prepended, as fix round 1093r5 had it, `ci` would make `CI-FAIL` (`ci` to `implement`) a forward
path to a work seat, so `--enter ci` would refuse as `ForwardWork`. **Recommended: (a) the
overlay's materialized order is `classify`, `implement`, `ci`, `verify`, `review`, `ship`,
`done`, `stop`**: `ci` placed after `implement` and before `verify`, landing's own phases in
their own order. Then:

- `CI-FAIL` (`ci` to `implement`) and `CI-PASS` (`ci` to `classify`) are both returns, so the
  forward walk from `ci` reaches nothing, and `ForwardWork` admits it;
- the protected walk from `ci` reaches no terminal but `stop` around `review`
  (`crates/brokkr-runtime/src/bundle/phases.rs:146-157` skips `stop`): every road from `ci` runs
  through `classify` or `implement`, and landing's own rules reach `ship` only through `review`;
- the visit check still finds `implement` reachable from `ci` without a visit-reading rule, by
  `CI-FAIL`, so fast's `visits_implement_gte` groups are admitted, as from landing's `classify`;
- no landing rule changes direction, because no rule joins `ci`'s neighbours across it: the
  only pairs whose order moved involve `ci` itself.

Costs, said plainly: the rail, which reads the recorded order (`crates/brokkr-view/src/rail.rs:9-12`),
paints both `CI-` roads as roads back; and on an explicit `--enter ci`, HEAD's seat input gives
`classify` a `returned_from` naming `ci` and its result word
(`crates/brokkr-runtime/src/engine/input.rs:151-168`), which landing's classify script does not
read (`recipes/landing/scripts/classify-seat.sh`). **(b)** `implement`, `ci`, `classify`, …: `CI-PASS` is forward, but landing's
own order moves, and `implement` stops receiving `classify`'s result on its later visits. Not
recommended. **(c)** an engine rule exempting `ci`'s edges from `ForwardWork`: no order needs
it, so no engine change is proposed.

**CI is the only way in.** A table's `initial` is not the only entry: an explicit `--enter`
phase is admitted on its own, through the entry check, and `initial` is used only when no entry
is named (`crates/brokkr-runtime/src/bundle/entry.rs:139-150`). The check refuses an absent,
terminal or work phase, and one from which a path reaches a terminal around the protected phase
(`:152-184`); `fast`'s protected phase is `review` (`recipes/fast/bundle.json:6`). Landing
documents `--enter review` and `--enter classify --continues <run>`
(`recipes/landing/README.md:91-99`), and both pass that check. So, as fix round 1093r4 left it,
`landing-ci --enter review` would skip `ci`. Under the frame that is landing as it is today, not
a pass the gate grants; it is still designed out, so the overlay's typed outcomes are reached on
every run that names it. **Recommended: (a) `landing-ci`'s entry admission admits `ci` alone,
through the existing entry check.** The check's chain of refusals gains one arm: in a bundle
whose table declares a `ci` seat, every phase but that one refuses as a new `EntryRefusal`
variant, `BeforeCi`, naming the `ci` phase, and `admitted()` lists `ci` alone. The arm sits
after the work-seat arm and before the protected arm, so `ci` itself still meets every existing
check: protected, forward work, strategy, change and visits, which the materialized order above
admits. `--enter ci`, with or without `--continues`, stays admitted, and the default start uses
`initial`, `ci`, as HEAD's `first_phase` does (`entry.rs:139-150`). The protected-phase arm is
unchanged, so `review` stays unavoidable before `ship`. Refused alternatives: **(b)**
a table rule alone, which the explicit entry never reads; **(c)** removing `--enter` from the
CLI for every recipe, which takes landing's documented continuation away from recipes that have
no `ci` phase.

**The entry set holds only checks that can be green before landing.** HEAD's required
`delivered by brokkr` (`.github/branch-protection.json:8`) fails until landing publishes its
shipped evidence: with no `by-hand` label, a body with no `Brokkr-Run:` line fails, and a named
run must have ruled `shipped` (`scripts/delivered-by-brokkr.sh:189-204`), which only this landing's
ship produces (decision 0051 ruling 2). At entry it fails on every first landing without the
label, and would send a sound change to `implement`. **Recommended: (a) the entry set excludes
every required check whose evidence is this landing's own output.** The recipe data says which:
a `required_checks` entry may carry `"after": "ship"`, a closed one-word field. **An after-ship
check is in the observed inventory and excluded from entry classification**, stated here once:
the observer lists it with every other check, unfiltered, and the sealed observation holds it;
no failure predicate, closure, revision check or completeness test reads it; and the forge's
required-set comparison counts it as declared. The rule for the recipe data: an entry whose producer reads anything this run's ship
seat writes (its anchor, its journal, its ledger) carries `"after": "ship"`. For the house that
is `delivered by brokkr`, and only it. The forge's requirement stays at merge, unchanged. The
declaration is the operator's: the compile cannot see what a check reads, and an entry left
undeclared costs one implement visit (the frame's table), never a pass. Refused alternative:
**(b)** observing it and treating its failure as untyped, which is fix round 1093r4's behaviour
and spends the implement visit on every first landing.

**The entry set is derived once, and every reader but the inventory reads it.** One function
over the pinned list gives the entry set: the entries with no `"after"`. The failure predicate
(ruling 3's `green` rule and its observed inadmissible failures, and ruling 4's failure set),
the closure parse, the producer, association and revision checks, and the setup steps' jobs all
read that set and nothing wider, so an after-ship check, its producer's checkouts and its red
are never a failure, an inadmissible failure, a closure root or a revision to prove. The full
declared list is read only to reconcile the inventory: ruling 3's comparison with the forge's
required set counts an after-ship entry as declared, so `required-set-disagrees` still finds an
undeclared required context, and the sealed observation still lists every check unfiltered.
A red `delivered by brokkr` beside every eligible entry green is therefore `green`, and beside a
valid setup failure is `ci-infrastructure`.

**The entry set is never empty.** A declared list whose every entry carries `"after": "ship"`
filters to nothing, and an observation with no entry to match would draw `green` on whatever
unrelated checks it saw. So the compile requires at least one entry-eligible check, one entry
without `"after"`, and refuses the list otherwise (ruling 5).

**Every route and cap, from the incoming edge.** First match wins, in the order listed. The
`When` column names the class; ruling 10 gives its schema-legal spelling.

| Rule | From | Result | When | Next |
|---|---|---|---|---|
| (entry) | | | `initial: ci`; an explicit entry admits `ci` alone | `ci` |
| `CI-REFUSED` | `ci` | `fail` | class `refused` | park, `ruled`, naming the refusals |
| `CI-INFRASTRUCTURE` | `ci` | `fail` | class `ci-infrastructure` | park, `infrastructure` `ci-infrastructure` |
| `CI-PASS` | `ci` | `pass` | class `green` | `classify` |
| `CI-PASS-UNBOUND` | `ci` | `pass` | | park, `ruled`, naming the word binding |
| `CI-FAIL` | `ci` | `fail` | | `implement`, `flagged` |

**The table is total.** HEAD's compile refuses a table with an unruled valuation:
`seated_and_ruled` calls `refuse_unruled` (`crates/brokkr-runtime/src/bundle/phases.rs:118-130`),
which sweeps every valuation of each `(from, result)` group's axes through the real evaluator
and refuses the first no rule rules (`crates/brokkr-core/src/policy/audit.rs:187-197`,
`:358-381`). A flag axis is both booleans (`:441-443`), whoever owns the flag. So with
`CI-PASS` the sole `(ci, pass)` row, the valuation `ci_green = false` is unruled and the
composed overlay would not compile. **Recommended: (a) `CI-PASS-UNBOUND`, a named, fail-closed
row:** the unconditional `(ci, pass)` rule after `CI-PASS`, parking `ruled` for a person and
naming the word binding. It is never reached while the binding holds: ruling 3 refuses `pass`
on any class but `green` before the table reads it. It is the table's answer if the binding
were ever lifted, and a park, never a route. The `(ci, fail)` group is already total:
`CI-FAIL` is unconditional beneath its two parks. **(b) Proving the valuation impossible** is
not taken: the audit sweeps each flag on its own, and HEAD gives it no way to read a
cross-input invariant such as the word binding, so the proof would need an engine change for
one row.

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
- any rule into `classify` but `CI-PASS`, and any explicit entry but `ci`: either would skip
  the gate;
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

**Recommended: (b) `infrastructure` with a closed `origin`, a field of its own, under
`park-cause/v2`.**

**`kind` is taken.** `park-cause/v1` already discriminates its four causes by a mandatory
`kind`: the infrastructure arm is `{"kind": "infrastructure", "retryable": …, "detail": …}`,
with `additionalProperties: false` (`contracts/park-cause.v1.schema.json:45-58`), and HEAD
serializes `ParkCause` with `#[serde(tag = "kind")]` (`crates/brokkr-core/src/park.rs:35-36`).
So the subtype cannot also be `kind`, and fix round 1093r5's "a v1 cause with no `kind` reads
`engine`" could never hold: every v1 cause has one. The subtype gets a distinct field.

- **(a) A new `ci-infrastructure` cause.** A fifth written word. Every match over `ParkCause`
  gains an arm (`crates/brokkr-core/src/park.rs:234`, `:264`, `:276`), 0106 ruling 1's four-word
  table becomes five, and 0106d's "only an `infrastructure` park spends" leaves it out by
  construction. Cost: a wider change to an accepted ruling's table. It refuses nothing (b) admits.
- **(b) `infrastructure` with `origin`.** Recommended. The infrastructure arm gains one
  optional field, `origin`, whose only written word is `ci`, carried with a required `faults`
  array, each fault naming its workflow, job, step, fault type (`setup-pattern` or
  `setup-via-needs`) and pattern id. **`engine` is the reader's word for an absent `origin`,
  and is never written**, as 0106 ruling 1 makes `unknown` the reader's word for an absent
  cause. Every infrastructure writer today writes no `origin`, so its bytes under v2 are its
  bytes under v1, byte for byte, and mean the same thing. The cause stays four words, the
  readouts already paint `infrastructure`, and 0106d's rule reads unchanged: it spends only where
  `retryable` is true (ruling 8). Cost: one closed enum of one written word inside one variant,
  matched without a wildcard.
- **(c) An explicit nested discriminator,** `"origin": {"kind": "engine"}` or
  `{"kind": "ci", "faults": […]}`, written on every infrastructure park. Unambiguous too, but
  every engine writer's bytes change, so a v1 reader refuses every infrastructure park written
  after v2, the engine's own included. Not recommended.

**The decode table, pinned.** Each object below is serialized and decoded in 0111d's tests, and
the result is exact. The v1 reader is HEAD's frozen one. It reads the infrastructure arm with
`retryable` and `detail` allowed, and refuses the **first** other key it meets as
`ParkCauseError::Forbidden { kind: "infrastructure", field }`
(`crates/brokkr-core/src/park/parse.rs:24-37`, `:137-142`), whose text is "the park cause does
not read: a infrastructure cause forbids `<field>`" (`crates/brokkr-core/src/park/error.rs:24-26`).
The keys come in the map's sorted order: the workspace's `serde_json` has no `preserve_order`
feature (`Cargo.lock:2401-2411` lists no `indexmap` among its dependencies), so its `Map` is a
`BTreeMap`, and `faults` sorts before `origin`. So an object holding both is refused for
`faults`, never for `origin`; the origin-only extension is the control that names `origin`.

| Object | v1 reader | v2 reader |
|---|---|---|
| `{"kind":"infrastructure","retryable":false,"detail":"d"}` | `Infrastructure { retryable: false }` | `Infrastructure { origin: Engine, retryable: false }` |
| `{"kind":"infrastructure","origin":"ci","retryable":false,"detail":"d","faults":[…]}` | `Forbidden { kind: "infrastructure", field: "faults" }` | `Infrastructure { origin: Ci { faults }, retryable: false }` |
| `{"kind":"infrastructure","origin":"ci","retryable":false,"detail":"d"}` (origin only) | `Forbidden { kind: "infrastructure", field: "origin" }` | refused by name: `origin: ci` requires `faults` |
| `{"kind":"infrastructure","origin":"ci","retryable":false,"detail":"d","faults":[]}` | `Forbidden { kind: "infrastructure", field: "faults" }` | refused by name: `faults` is empty |
| `{"kind":"infrastructure","origin":"engine","retryable":false,"detail":"d"}` | `Forbidden { kind: "infrastructure", field: "origin" }` | refused by name: `engine` is never written |
| `{"kind":"infrastructure","retryable":false,"detail":"d","faults":[…]}` (no `origin`) | `Forbidden { kind: "infrastructure", field: "faults" }` | refused by name: `faults` needs `origin: ci` |
| `{"kind":"infrastructure","origin":"ci","retryable":true,"detail":"d","faults":[…]}` (valid faults) | `Forbidden { kind: "infrastructure", field: "faults" }` | refused: `a ci infrastructure park cause is never retryable` |
| `{"kind":"infrastructure","retryable":true,"detail":"d"}` (legacy, no `origin`) | `Infrastructure { retryable: true }` | `Infrastructure { origin: Engine, retryable: true }` |

**A CI cause is never retryable, at the schema and at decode.** HEAD's parser accepts either
boolean for an engine infrastructure cause, and that stays. Under `origin: ci`,
`park-cause/v2`'s schema holds `retryable` to `{"const": false}`, and v2's decode refuses
`true` with the typed error whose text the row above pins, before any reader sees the cause.
The writer this decision specifies already emits `false` (ruling 8); the refusal keeps a
hand-written or tampered cause from spending 0106d's budget.

No object decodes under both readers to two different meanings: the only object both read is
the v1 shape, and both read it as the engine's.

**The typed park matches its own rule explicitly.** A park is not typed because its inputs
happen to carry the class. The compile marks, in a table whose recipe declares a `ci` seat,
exactly one rule as the CI-infrastructure park: the park rule `from` the `ci` phase whose `when`
is exactly `{"ci_infrastructure": true}`. It refuses a table with none, or with more than one.
The engine's park arm (`crates/brokkr-runtime/src/engine/cursor.rs:361-371`) gains one case. It
writes `infrastructure { origin: ci, retryable: false }`, with its faults, instead of `ruled` only
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
`origin`, under the errata rule for back-pointers.

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
`only_an_infrastructure_park_spends_the_budget` is unchanged: it reads `retryable`, not `origin`.

### 9. The readouts

**Recommended: (a) one derivation in `brokkr-view`, painted by `inspect` and the fleet.**

`inspect` names each observation's class, its refusal or its faults, and the evidence a person
needs to re-run: the workflow run's id and attempt and each failed job's id, from one view model
(0071 ruling 7). The fleet paints the park's cause through `ParkCause::line`
(`crates/brokkr-core/src/park.rs:275`), which the TUI and `inspect` already paint
(`crates/brokkr-cli/src/tui/columns.rs:375`, `crates/brokkr-cli/src/render/inspect.rs:36`; the
fleet's row field is `crates/brokkr-view/src/fleet.rs:339`). It reads
`infrastructure (ci, not retryable): …` with the faults' job and pattern ids.

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
- `CI-PASS-UNBOUND`: one result, `pass`; no `when`; `park: true`, as the two parks above. Legal,
  and not made unreachable by `CI-PASS`, which is conditional.
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
  plainly: no live producer exists before 0109c. 0111a to 0111d land and test offline, and live
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
- No refusal is weakened: refusals are evaluated first, and a fact one reads that is not
  established refuses ([No refusal weakens](#no-refusal-weakens)); `retryable`'s
  positive-evidence rule stands as 0106 wrote it.
- No Windows code (0063).
- #1113's own fix.
- The dispatcher's retry budget, beyond ruling 8.
- Flaky-test handling of any kind, quarantine included: a follow-up (ruling 4).
- `fast`'s `blocked_by_in` array outside v2's published schema: a follow-up (ruling 10).
- A CI gate on a head the run itself made, before review in `fast` or `verify`: a follow-up
  that needs a publish-and-resume handoff (ruling 6).
- Commit statuses as required evidence: a follow-up that needs a revision attestation (ruling 3).
- This house's own typed parking: deferred to ruling 3's one follow-up issue under #1094, which
  resolves the house's whole eligible closure, `scripts/test-shards.sh`'s computed dispatch and
  `scripts/mutants.sh`'s substituted `cd` among it, and names every closure still unresolved
  (ruling 3).
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
   `ci/tests.rs`; `crates/brokkr-core/src/lib.rs`; each closure tree's workflow parse into that typed
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
   `ci` declaration until 0111e (ruling 2):
   `a_bundle_declaring_a_ci_seat_refuses_to_compile_until_the_first_live_producer_lands`, a custom
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
   `a_failed_range_read_and_a_verified_empty_range_replay_with_distinct_reasons` (one all-setup
   observation, the same two commit ids: the failed read seals `unproven` and replays `change`,
   `withheld`, `range-unproven`; the verified empty range seals `verified` with no path and
   replays `ci-infrastructure`);
   `the_basis_carries_the_required_list_the_literals_the_closure_and_the_comparison_values`;
   `the_live_class_and_the_replayed_class_take_one_function_over_one_basis`;
   `a_closure_beyond_its_bound_withholds_as_basis_too_large`;
   `a_range_of_4097_short_paths_withholds_as_range_too_large`;
   `a_range_too_large_class_replays_from_the_journal_alone` and
   `a_basis_too_large_class_replays_from_the_journal_alone` (each: the journal exported, then
   the bundle, the worktree and the forge stub removed; the replay re-derives `change` with its
   withholding from the witness);
   `the_overflow_witness_carries_the_bound_the_limit_the_measured_size_and_a_digest_never_the_list`;
   `a_list_past_the_hard_read_cap_seals_overflowed_unmeasured_with_its_lower_bound_and_no_digest`;
   `a_closure_over_128_kib_withholds_as_basis_too_large`;
   every reason replays (ruling 3), each asserting the exact class and the exact reason list,
   live and from the journal alone:
   `a_4097_path_range_including_deny_toml_names_range_too_large_and_machinery_touched_deny_toml`
   (a red all-setup observation: `change`, `withheld`, exactly `range-too-large` with its
   `measured` witness and `machinery-touched` `holds` naming `deny.toml`);
   `a_closure_overflow_beside_a_revision_disagreement_names_both`
   (`change`, `withheld`, exactly `basis-too-large` with its `closure` witness and
   `revision-disagrees` `holds` naming both instances and both commits);
   `a_reason_table_beyond_its_bound_records_each_hidden_reason_unavailable`;
   `a_range_past_its_hard_cap_records_machinery_touched_unavailable_with_its_lower_bound`;
   `a_valid_all_setup_aggregate_replays_from_the_journal_alone` (the journal exported, then the
   bundle, the worktree, the workflow and the forge stub removed: `ci-infrastructure`, from the
   sealed `verdict-only` kind, nine expected keys and their nine established revisions, all one
   merge commit, with the checkout-less aggregate passing the revision check through them);
   `a_checkout_less_verdict_only_aggregate_is_not_judged_by_the_one_checkout_rule` (the
   aggregate is not `revision-unmeasured` for having no checkout; an aggregate with an
   independent step and no checkout is);
   `an_all_setup_aggregate_with_one_disagreeing_dependency_instance_replays_as_revision_disagrees`
   (the negative control, live and from the journal alone: one instance's established revision
   another commit: `change`, `withheld`, `revision-disagrees` naming both instances);
   `a_missing_sibling_replays_from_the_journal_alone_as_matrix_incomplete` (the same, one
   instance short: `change`, `withheld`, `matrix-incomplete` naming the missing key);
   `a_sealed_matrix_unexpandable_outcome_replays_as_refused_instance_duplicated_unestablished`;
   `a_witness_within_its_limit_refuses_the_replay`. The evidence (ruling 4):
   `a_port_bind_literal_in_a_declared_setup_step_draws_ci_infrastructure`;
   `a_declared_literal_in_an_undeclared_step_is_the_changes`;
   `a_literal_printed_by_the_change_where_the_table_is_wrong_draws_a_park_never_green` (a
   stubbed table that omits a program's input: the bounded cost, `ci-infrastructure`, never
   `green`);
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
   `an_unadmitted_failure_beside_a_setup_failure_is_exactly_change_never_refused_or_parked`
   (the class is `change` with reason `withheld`, the routed rule is `CI-FAIL`, and no park is
   written);
   `a_valid_all_setup_verdict_only_aggregate_draws_ci_infrastructure` (a port bind in one
   `engine-machines` instance, the other eight observed and successful);
   `an_independent_assertion_failing_beside_a_setup_fault_in_a_dependency_is_change`
   (an aggregate with a second step of its own that fails: `change`, `untyped-failure`);
   every failure typed, each asserting the exact class and reason:
   `a_later_always_step_failing_after_a_matched_setup_step_is_change` (one job: a declared setup
   step fails with its literal, then an independent `if: always()` step fails: `change`,
   `untyped-failure`, naming the second step);
   `two_failed_setup_steps_each_matching_their_literals_draw_ci_infrastructure` (the control:
   every failed step a matched setup step: `ci-infrastructure`, naming both pattern ids);
   `an_all_setup_failure_set_draws_ci_infrastructure` (the all-setup control across jobs);
   `a_failed_job_with_no_failed_step_beside_a_matched_fault_is_change` (one independent job
   concludes `failure` and reports zero failed steps, another holds a matched setup fault:
   `change`, `untyped-failure`, naming the stepless job);
   `a_duplicated_matrix_instance_key_in_one_attempt_refuses_as_instance_duplicated` (all nine
   expected keys, plus a second job for one key, contradicting it: `refused`,
   `instance-duplicated`, never `ci-infrastructure`);
   `a_red_required_status_beside_every_pinned_entry_green_is_change_not_green` (`change`,
   `inadmissible-failure`, naming the context and `status-required`);
   `a_sole_red_producer_mismatch_is_change_not_missing_required` (`change`,
   `inadmissible-failure`, naming the check and `producer-mismatch`);
   `a_green_producer_mismatch_beside_an_absent_entry_refuses_as_missing_required` (the absent
   control);
   `a_missing_matrix_sibling_draws_no_ci_infrastructure` (eight of nine instances observed:
   `change`, `matrix-incomplete`);
   `a_cancelled_matrix_sibling_draws_no_ci_infrastructure` (`change`, `cancelled`);
   `a_matrix_built_by_an_expression_refuses_as_instance_duplicated_unestablished`;
   `an_aggregate_with_one_untyped_dependency_is_the_changes`;
   `an_aggregate_whose_dependency_is_in_another_attempt_is_the_changes`;
   `the_aggregate_reads_its_needs_and_matrix_from_the_event_workflow_never_the_checkout`
   (the event workflow and the checked-out head's copy name different `needs`: the event's are
   followed);
   `an_event_workflow_dependency_key_absent_from_the_checkout_observed_twice_refuses_as_instance_duplicated`
   (the chief's case: established instance naming; the event workflow adds a dependency key
   under a required aggregate that the checked-out head's copy lacks, and the forge reports that
   key twice in one attempt beside a single failed required aggregate: `refused`,
   `instance-duplicated` naming the key, `CI-REFUSED`, never `change` with `matrix-incomplete`
   or `CI-FAIL`, live and from the journal alone);
   `a_setup_command_that_differs_between_the_event_workflow_and_the_checkout_is_judged_by_the_event_workflow`
   (the event workflow's declared setup step runs `test -f crates/ci/ready`, the checked-out
   head's copy of the same step runs `echo`, and the change removes only that file: `change`,
   `withheld`, `machinery-escapes` naming the event workflow's command, and its control with
   the two commands swapped reaches classification);
   `a_run_whose_event_workflow_revision_is_unreported_refuses_as_source_unestablished` (live
   and from the journal alone);
   `a_local_helper_is_resolved_in_the_executed_checkout_never_the_event_revision` (a `head`
   entry whose helper differs between the merge commit and the head: the head's is followed);
   `a_setup_step_in_a_job_no_required_aggregate_needs_withholds`;
   `a_stale_head_observation_does_not_read`; `a_pass_with_a_failed_job_is_refused`;
   `a_seat_cannot_write_the_classification`; `a_ci_list_on_a_model_seat_refuses_to_compile`.
   The producer, the association and the tested revision (ruling 3):
   `a_same_named_check_from_another_workflow_withholds_as_producer_mismatch`;
   `a_green_check_on_another_event_does_not_match_its_entry`;
   `a_red_setup_only_run_on_the_same_head_for_another_pull_request_is_change_as_association_unproven`;
   `a_run_with_an_empty_association_withholds_as_association_unproven`;
   `an_association_naming_a_and_a_closed_b_draws_no_ci_infrastructure`
   (`association-ambiguous`, with B closed and the open list naming only A);
   `an_all_success_observation_with_an_unproven_association_is_green`;
   `an_association_and_a_checkout_that_disagree_withhold_as_revision_disagrees`;
   `a_head_entry_whose_event_workflow_job_checks_out_another_ref_refuses_as_source_unestablished`;
   `the_full_two_entry_engine_matrix_with_a_distinct_valid_merge_commit_reaches_classification`
   (a synthetic workflow of the house's shape, whose closure resolves: both `test (…)` entries
   over all nine `engine-machines` instances, each with a merge commit distinct from the head
   whose parents are the base and the head, and a declared port-bind literal in one instance:
   `ci-infrastructure`, so no comparison demands merge and head be one. It is the classifier's
   positive case, not the house's);
   `changing_the_checkout_revision_of_a_dependency_draws_no_ci_infrastructure`
   (one `engine-machines` instance records another merge commit: `revision-disagrees` for the
   aggregate);
   `an_entry_whose_run_records_no_tested_commit_refuses_as_source_unestablished`;
   `a_setup_step_whose_working_directory_points_at_another_provisioned_checkout_draws_no_ci_infrastructure`
   (unchanged machinery, one checkout of the correct merge commit into `candidate/`, every
   comparison passing, and the declared setup step's `working-directory` another directory
   already on the runner, printing the declared literal: `change`, `withheld`, with
   `revision-unmeasured` naming the step);
   `a_step_naming_no_directory_beside_a_checkout_into_candidate_withholds_as_revision_unmeasured`
   (the job's one checkout of the correct merge commit has `path: candidate`, and the declared
   setup step names no `working-directory`, so it runs in `GITHUB_WORKSPACE`: `change`,
   `withheld`, `revision-unmeasured` naming the step and the workspace root);
   `a_step_whose_working_directory_is_the_candidate_checkout_reaches_classification` (the
   positive control: the same job, the step's `working-directory: candidate`, a declared
   port-bind literal: `ci-infrastructure`);
   `a_job_with_two_checkouts_refuses_as_source_unestablished`;
   `a_step_working_directory_built_by_an_expression_withholds_as_revision_unmeasured`;
   `a_status_entry_in_the_required_list_refuses_to_compile`;
   `a_status_context_in_the_forges_required_set_withholds_as_status_required`;
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
   `a_makefile_only_change_under_a_pinned_working_directory_draws_no_ci_infrastructure` (a
   required setup step runs a bare `make` under a base `defaults.run.working-directory` of
   `crates/ci`, and the change edits only `crates/ci/Makefile` so the step prints the declared
   port-bind literal: `change`, `withheld`, with `machinery-escapes` naming the directory and the
   `Makefile`);
   `a_read_operand_in_a_change_class_draws_no_ci_infrastructure` (a required setup step runs
   `test -f crates/ci/ready` from the workspace root, and the change only removes that file:
   `change`, `withheld`, with `machinery-escapes` naming `test`, the operand and the step);
   `a_change_to_an_unchanged_symlinks_target_only_draws_no_ci_infrastructure` (a tracked
   symlink `ops/ci/ready`, unchanged, targets `../../crates/ci/ready`; a required setup step
   runs `test -f ops/ci/ready` from the workspace root, and the change edits only
   `crates/ci/ready`: `change`, `withheld`, with `machinery-escapes` naming `test`, the spelled
   `ops/ci/ready`, the link and the resolved `crates/ci/ready`; and its siblings, a symlinked
   working directory into `crates/` the same, and a link with an absolute target, one that
   leaves the tree and a loop past 40 links, each `machinery-unresolved` naming where the walk
   stopped);
   `a_machinery_makefile_including_a_change_class_file_draws_no_ci_infrastructure` (a bare
   `make` under a machinery directory whose `Makefile` includes `crates/ci/setup.mk`, the
   change editing only `setup.mk`: `change`, `withheld`, with `machinery-escapes` naming the
   `Makefile` and the include);
   `a_non_literal_or_unsupported_include_or_read_withholds_as_machinery_unresolved` (a
   `Makefile` `include $(DIR)/setup.mk`, a `$(shell cat …)`, and a `test -f "$X"`: each `change`,
   `withheld`, `machinery-unresolved` naming the form);
   `a_makefiles_environment_key_naming_a_change_class_file_draws_no_ci_infrastructure` (a
   required setup step runs a bare `make` under a machinery `defaults.run.working-directory` of
   `ops/ci`, the step's `env` sets `MAKEFILES: ../../crates/ci/extra.mk`, and the change edits
   only `crates/ci/extra.mk` so the step prints the declared port-bind literal: `change`,
   `withheld`, with `machinery-escapes` naming `make`, `MAKEFILES`, the normalized
   `crates/ci/extra.mk` and the step; and its siblings, the same file named by an inline
   `MAKEFILES=../../crates/ci/extra.mk make` prefix and by a literal `$GITHUB_ENV` line an
   earlier step wrote, each the same);
   `a_command_local_makefiles_outranks_a_safe_earlier_value` (the collision: under the same
   `ops/ci` directory, the job's `env`, an earlier step's `$GITHUB_ENV` line and an earlier
   `export` in the run line each set `MAKEFILES` to the machinery `safe.mk`, and the command is
   `MAKEFILES=../../crates/ci/extra.mk make`, the change editing only `crates/ci/extra.mk`:
   `change`, `withheld`, `machinery-escapes` naming the normalized `crates/ci/extra.mk`, never
   `safe.mk`; and its control, the inline prefix removed, reads `safe.mk` and reaches
   classification);
   `a_file_selecting_environment_key_the_table_cannot_resolve_withholds_as_machinery_unresolved`
   (`MAKEFILES: ${{ vars.MK }}`, `COMPOSE_FILE: $X`, and a non-empty `MAKEFLAGS`: each
   `change`, `withheld`, `machinery-unresolved` naming the key);
   `a_changed_build_script_under_a_declared_setup_cargo_build_draws_no_ci_infrastructure` (a
   declared setup step runs `cargo build` from the workspace root, and the change edits only
   `crates/boot/build.rs` so the build prints the declared literal: `change`, `withheld`, with
   `machinery-escapes` naming `cargo`, `crates/boot/build.rs` and the step);
   `a_step_before_a_declared_setup_step_reading_a_product_input_withholds_as_machinery_escapes`
   (the same build in an earlier step of the setup step's job, and again in a job it `needs`:
   each the same);
   `a_cargo_test_after_the_last_setup_step_keeps_the_product_input_exception` (the control: the
   closure does not withhold, and a failing test step is `untyped`, the change's);
   `a_state_restoring_action_supplying_a_setup_step_withholds_as_machinery_unresolved`;
   `a_step_whose_effective_working_directory_lies_in_a_change_class_withholds_as_machinery_escapes`;
   `a_cd_to_a_non_literal_directory_withholds_as_machinery_unresolved`;
   `a_command_word_the_pinned_table_does_not_name_withholds_as_machinery_unresolved`;
   `a_remote_action_the_pinned_table_does_not_name_withholds_as_machinery_unresolved`;
   `a_docker_compose_step_whose_compose_file_lies_in_a_change_class_withholds_as_machinery_escapes`;
   `the_closure_follows_needs_uses_and_sourced_helpers_transitively`;
   `the_closure_reads_each_jobs_executed_tree_through_the_verified_reader_never_the_worktree`;
   `a_setup_helper_the_advanced_base_replaced_but_the_tested_head_still_runs_draws_no_ci_infrastructure`
   (a `head` entry, its workflow revision recorded; the pull request opened on A, the base
   advanced to B, where the setup helper no longer reads `crates/ci/ready`, and the change
   editing only that file so A's helper, still in the head, prints the declared literal:
   `change`, `withheld`, with `machinery-escapes` naming A's helper and the read, though a scan
   of B closes);
   `a_job_whose_closure_tree_is_not_established_refuses_as_source_unestablished`;
   `a_setup_step_listed_before_the_checkout_withholds_as_revision_unmeasured` (the declared
   setup step precedes the job's one `actions/checkout`, and its literal `working-directory` is
   inside the checkout's `path`: `change`, `withheld`, `revision-unmeasured` naming the step and
   the checkout step; and its sibling, the checkout listed first but not reported `success`,
   the same);
   `an_empty_change_class_list_refuses_to_compile`;
   `a_change_class_under_github_or_a_required_workflow_refuses_to_compile`;
   `the_houses_own_closure_withholds_as_machinery_unresolved_at_test_shards_sh` (HEAD's base,
   the house's required list and a red setup-only observation of `engine-machines`: the class is
   exactly `change`, `withheld`, and its withholdings hold `machinery-unresolved` naming
   `scripts/test-shards.sh` and its array command word; until the follow-up rewrite, when this
   test is replaced by the house's positive case);
   `the_houses_mutants_closure_withholds_as_machinery_unresolved_at_mutants_sh` (HEAD's base and
   a red setup-only observation of `mutants in the diff: brokkr-core`: exactly `change`,
   `withheld`, `machinery-unresolved` naming `scripts/mutants.sh:31` and its substituted `cd`;
   replaced with the other at the follow-up);
   `an_ambiguous_tested_revision_refuses_as_source_unestablished`;
   `a_merge_revision_whose_parents_are_not_the_base_and_the_head_withholds`;
   `two_unsettled_runs_for_one_entry_refuse_as_execution_ambiguous`;
   `missing_required_beside_a_classifiable_failure_refuses_and_is_never_typed`;
   `a_forge_required_check_the_list_lacks_withholds_as_required_set_disagrees`. The
   refusals, one test each, every one asserting the refusal's variant and that the class is not
   `green`: `an_observation_with_no_check_refuses_as_empty`;
   `a_required_audit_missing_from_the_observation_refuses_as_missing_required`;
   `an_observation_of_another_head_repository_or_pull_request_refuses_as_unrelated`;
   `a_skipped_or_neutral_required_check_refuses_as_non_success`;
   `a_required_check_still_in_progress_refuses_as_not_settled`;
   `two_matches_for_one_required_check_refuse_as_ambiguous`;
   `an_empty_required_list_refuses_to_compile`.
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
   the observer credential's `access: read` declaration); run start's refusal of a `ci` seat
   whose bundle layer is inside the worktree, over 1091a's descriptor-held walk, so 0111c lands
   after 1091a; a house observer script, with its polling bound inside the seat's deadline, and
   its shellcheck clean. 0111b's live-admission refusal stands through this slice; 0111e lifts
   it (ruling 2). About 400 production, 150 script and 650 test lines. No contract change.
   Tests, the launch driven through a test-only bundle the live compile does not admit:
   `one_admitted_launch_resolves_the_sealed_binding_into_the_cleared_environment_captures_stdout_and_parses_the_result`
   (the real credential arrangement: a sealed binding in the operator-side store, resolved by
   the runner into `GH_TOKEN`, the one allow-listed token variable of a cleared
   `SpawnEnv::Exactly`, no other key present, and `GH_CONFIG_DIR` an empty directory; a stub
   `gh` by absolute path asserts it read the token from `GH_TOKEN`, nothing from argv and no
   stored login from its configuration directory; its stdout is captured over the engine's pipe and parsed into
   `ci-observation/v1`; run once under `harness` and once beside a boxed seat under
   `namespace`, both admitted);
   `an_observer_credential_declared_write_scoped_refuses_to_compile` (`"access"` of any word but
   `read`, and a second secret binding on the `ci` seat, each refused by name);
   `an_observer_credential_bound_under_any_name_but_gh_token_refuses_to_compile`;
   `on_macos_the_interpreter_runs_by_path_only_under_a_root_owned_chain`. The boundary
   (Linux, under a real box, as 0043's boundary tests are):
   `inside_the_box_opening_the_observers_pipe_endpoint_through_proc_fails`
   (a boxed seat walks its `/proc` for the observer's and the engine's pids and their `fd`
   entries; its control, the same walk from an unboxed same-UID sibling, opens the endpoint,
   which the trust root accepts);
   `a_configuration_directory_replaced_by_a_boxed_seat_before_gh_opens_it_refuses`
   (a boxed seat whose binds reach the directory's parent swaps it by name between the walk
   and the spawn: the held descriptor is the one `gh` reads, or the launch refuses);
   `the_observers_working_directory_is_set_from_the_held_descriptor`;
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
   the decode table (ruling 7), each object serialized and decoded by both readers with the
   exact result: `a_v1_infrastructure_cause_decodes_identically_under_v1_and_v2_as_origin_engine`;
   `a_full_v2_ci_cause_decodes_under_v2_and_the_frozen_v1_reader_refuses_it_as_forbidden_faults`
   (v1: exactly `ParkCauseError::Forbidden { kind: "infrastructure", field: "faults" }`, and
   its pinned text);
   `an_origin_only_extension_is_refused_by_the_frozen_v1_reader_as_forbidden_origin` (the
   origin-specific v1 control: `Forbidden { kind: "infrastructure", field: "origin" }`; v2
   refuses it for its missing `faults`);
   `an_origin_engine_word_is_refused_by_name`;
   `an_origin_ci_cause_with_empty_faults_is_refused_by_name`;
   `faults_without_origin_ci_are_refused_by_name`;
   `an_origin_ci_cause_with_retryable_true_is_refused_at_schema_and_decode` (valid faults: the
   schema rejects it, v2's decode returns the exact never-retryable error, and the frozen v1
   reader returns `Forbidden { kind: "infrastructure", field: "faults" }`);
   `a_legacy_retryable_engine_cause_still_decodes_retryable_under_v1_and_v2` (the control beside
   it);
   `every_engine_infrastructure_writer_serializes_v1_bytes_under_v2`;
   `a_ci_infrastructure_park_is_never_retryable`;
   `a_runner_fault_park_writes_the_ci_infrastructure_cause_and_keeps_the_rule_in_the_decision`;
   `a_refusal_park_carrying_a_ci_class_stays_ruled`;
   the eligible current match (ruling 3's one order), each over a test-built table holding ruling
   6's rows and asserting the class, the reason and the routed rule, live and again from the
   journal alone (exported, then the bundle, the worktree and the forge stub removed):
   `a_stale_same_producer_success_beside_an_ordinary_failure_refuses_as_missing_required`
   (a required audit with no check in its current run, an older same-producer `success` on the
   same head, and a second, eligible required job with an ordinary failed test: `refused`,
   `missing-required` naming the audit, `CI-REFUSED`, a `ruled` park, never `CI-FAIL`);
   `a_stale_same_producer_success_beside_an_all_setup_failure_refuses_as_missing_required` (the
   sibling: the second job's failure all-setup: the same `refused`, `missing-required`,
   `CI-REFUSED`, never `ci-infrastructure`);
   `a_stale_same_producer_red_is_neither_a_match_nor_an_inadmissible_failure` (the stale check
   red instead: `refused`, `missing-required`, not `change` with `inadmissible-failure`);
   `an_over_128_kib_closure_with_a_duplicated_expected_instance_refuses_as_instance_duplicated`
   (a closure sealed as its overflow witness, and one expected instance's key observed twice in
   one attempt: `refused`, `instance-duplicated` naming the key, `CI-REFUSED`, never `change`
   with `basis-too-large`, live and from the journal alone, from the instance table sealed
   beside the witness);
   `an_instance_table_beyond_its_bound_refuses_as_instances_too_large` (live and from the
   journal alone);
   no refusal weakens (ruling 3's one home), one test per row of the refusal table, the row's
   fact unestablished and every other fact established so the row's own predicate decides, each
   asserting exactly `refused`, the row's word with `unestablished` naming the fact and its
   state, and `CI-REFUSED`, never `CI-FAIL`, `CI-INFRASTRUCTURE` or `CI-PASS`, live and from the
   journal alone:
   `an_unread_engine_head_refuses_as_unrelated_unestablished`;
   `a_forge_head_reported_unread_refuses_as_head_unpublished_unestablished`;
   `a_check_list_read_short_of_its_last_page_with_no_check_refuses_as_empty_unestablished`;
   `a_check_list_read_short_of_its_last_page_beside_an_ordinary_failure_refuses_as_missing_required_unestablished`
   (the required audit is not on the pages read; another entry's ordinary failure is);
   `a_check_list_read_short_of_its_last_page_beside_one_match_refuses_as_ambiguous_required_unestablished`;
   `a_run_list_read_short_of_its_last_page_refuses_as_execution_ambiguous_unestablished`;
   `an_unreported_event_workflow_revision_beside_an_all_setup_failure_refuses_as_source_unestablished`
   and its sibling, a judged job whose closure tree no run field records;
   `an_unexpandable_matrix_beside_an_all_setup_failure_refuses_as_instance_duplicated_unestablished`;
   `unmeasured_instance_naming_beside_an_all_success_observation_refuses_as_instance_duplicated_unestablished`;
   `an_instance_table_whose_source_is_not_the_established_one_refuses_as_instance_duplicated_wrong_source`;
   `a_required_check_with_an_unread_status_refuses_as_not_settled_unestablished`;
   `a_required_check_with_an_unread_conclusion_refuses_as_non_success_unestablished`;
   `an_observation_missing_a_required_field_refuses_at_the_edge_as_observation_unreadable`;
   and `every_row_of_the_refusal_table_has_an_unestablished_case`, which enumerates the refusal
   enum without a wildcard and fails when a row has no case above. Every 0111b test over a
   matrix dependency builds its instance naming from 0111a's evidence; where 0111a leaves it
   unmeasured, those tests assert this refusal instead, and the decision is revised before
   acceptance (ruling 3's cost);
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
   6's table and the house's `ci-checks.json` with `delivered by brokkr` as `"after": "ship"`;
   `crates/brokkr-runtime/src/bundle/entry.rs` (the `BeforeCi` arm);
   `crates/brokkr-runtime/src/bundle.rs` (the lift of 0111b's live-admission refusal, ruling 2:
   `a_bundle_declaring_a_ci_seat_compiles_once_the_launcher_the_park_and_the_table_exist`);
   `crates/brokkr-runtime/src/engine/input.rs` (ruling 6's projection of the `ci` result); and
   its compile tests.
   About 100 data, 80 production and 500 test lines. No contract of this
   decision's own. **Live read-only telemetry starts here**: from the first landing the operator
   drives under this overlay on a real pull request, each observation, class and park is
   journaled, and a sweep can count typed parks against diagnosis seat attempts. The park's
   lifecycle is this slice's too: a person re-runs, then the operator retries the parked `ci`
   phase, which observes again. Tests: the frame's two
   ([The gate grants nothing](#the-gate-grants-nothing)),
   `after_ci_pass_the_composed_table_is_landings_own_rule_for_rule` and
   `nothing_in_the_recipes_or_seat_scripts_reads_the_ci_phases_result`; the consumer input
   (ruling 6's projection), each built by the engine's own input builder over a journal holding
   the `ci` decision and asserting the exact keys:
   `after_ci_pass_no_later_seat_input_holds_the_ci_observation_or_class` (`classify`, `verify`,
   `review` and `ship`: no `results.ci`, and no `ci_*` key anywhere in the input);
   `after_ci_fail_implement_receives_the_class_and_never_the_observation` (exactly `ci_class`'s
   class, reason and ids under `last_decision`);
   `the_review_seat_input_holds_no_ci_key_on_any_road` (after `CLASSIFY-DOCS`, after
   `VERIFY-PASS`, and after an implement visit `CI-FAIL` bought);
   the entry (ruling 6), each over the overlay's materialized bundle, its composed table and
   phase order, through HEAD's `first_phase` and `admitted()`:
   `the_landing_ci_default_start_enters_ci`;
   `landing_ci_admits_ci_alone_as_an_explicit_entry` (`--enter ci` passes every existing arm,
   `ForwardWork` among them, and `admitted()` is exactly `["ci"]`);
   `landing_ci_enter_ci_with_continues_is_admitted`;
   `landing_ci_enter_review_refuses_as_before_ci`;
   `landing_ci_enter_classify_refuses_as_before_ci`;
   `ci_prepended_to_landings_order_refuses_enter_ci_as_forward_work` (the control: the r5
   order, `ForwardWork` naming `implement` and `CI-FAIL`);
   `the_landing_ci_order_is_landings_own_with_ci_after_implement`;
   `review_stays_unavoidable_before_ship_from_ci`;
   `a_required_list_whose_every_entry_is_after_ship_refuses_to_compile`;
   `a_fresh_published_pull_request_with_no_anchor_and_no_label_is_not_routed_to_implement_by_the_contribution_check`
   (`delivered by brokkr` red, every other entry green: `green`, never `change`);
   `an_after_ship_check_is_in_the_observed_inventory_and_excluded_from_entry_classification`
   (the sealed observation lists `delivered by brokkr`; no failure, closure root, revision or
   completeness test reads it; and the forge-set comparison counts it declared);
   `a_red_after_ship_check_beside_every_eligible_entry_green_is_green` (`green`, not `change`
   with `inadmissible-failure`, though the forge requires the context);
   `a_red_after_ship_check_beside_a_valid_setup_failure_draws_ci_infrastructure` (the class
   names only the eligible setup fault);
   `an_undeclared_forge_required_context_beside_an_after_ship_entry_withholds_as_required_set_disagrees`;
   `a_green_observation_passes_to_classify`;
   `a_change_failure_returns_to_implement_once`;
   `a_refusal_parks_ruled_and_never_passes`;
   `a_ci_infrastructure_class_never_routes_to_implement`;
   `an_unpushed_head_refuses_as_head_unpublished`;
   `a_pushed_head_after_a_head_unpublished_park_observes_again_on_retry`;
   `a_head_the_run_made_is_never_gated_by_ci`;
   `the_environment_park_survives_the_ci_overlay`;
   `an_operator_retry_after_a_person_re_runs_observes_again`;
   `the_composed_landing_ci_table_validates_whole_against_phase_machine_v3`;
   `the_whole_composed_landing_ci_overlay_compiles_against_heads_totality_gate` (the
   materialized bundle through `seated_and_ruled` and `refuse_unruled`: it compiles; and its
   control, the same overlay less `CI-PASS-UNBOUND`, refuses with
   `Refusal::Totality` naming `ci`, `pass` and `ci_green = false`);
   `a_pass_word_on_a_non_green_class_never_reaches_ci_pass_unbound` (the word binding refuses it
   first).

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
