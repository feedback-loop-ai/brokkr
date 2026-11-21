# 0111 — A check the CI runner failed, not the change, is re-run once before the run parks

Status: proposed
Date: 2026-11-18

Built: unbuilt (#1093)

## Context

Issue #1093, under the adopter-findings epic #1094 (delivery cost and recovery), carries an
adopter's figures, and they are theirs: on brokkr 0.12.0 with GitHub-hosted runners, three parked
runs in one week, each on a runner fault. In one, a compose service could not bind its host port
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
This decision writes only the `ruled` cause v1 already has, and weakens neither.

**The heading names the issue's ask; this decision rules narrower.** The heading keeps the
issue's ask, a runner fault re-run once before the run is parked, because a decision's heading
is never edited. What this decision rules is that Brokkr re-runs nothing and types nothing: it
observes a pull request's CI at landing's entry, lets a `green` observation proceed as landing
does, and parks every other observation for a person, on a row of one
[park table](#the-park-table).

**No re-run.** A re-run is a forge write, and the forge cannot confine one. GitHub's two re-run
verbs, `POST /repos/{owner}/{repo}/actions/jobs/{job_id}/rerun` and
`POST /repos/{owner}/{repo}/actions/runs/{run_id}/rerun-failed-jobs`
(<https://docs.github.com/en/rest/actions/workflow-runs>), take only `enable_debug_logging` in
their body, and the job form also `enable_debugger`. Neither takes an attempt number, an
expected state or an `If-Match` precondition, and each re-runs the named jobs "and its dependent
jobs". So no request can be confined, at the moment it is sent, to the attempt and the closure
the engine observed. The controller read that page on 2026-11-18; this seat holds no web
capability and did not, and the forge's live behavior is **unmeasured**. Fix round 1093r2's
review held as a security risk a design that sent the request anyway, and on 2026-11-18 the
operator ruled ruling 1 (a): Brokkr observes, classifies and parks, and a person re-runs (the
park is now the [park table](#the-park-table)'s, with no classification).

**No classification.** Fix rounds 1093r2 to 1093r8 drafted a classifier that typed a red
observation as the runner's or the change's, and each security hold was one class: a fact a
refusal needs, lost or misread, let a case owed a person fall through to an implement visit. On
2026-11-20 the operator ruled "narrow 0111 as recommended", deferring the classifier and the
implement route ([Deferred to a later decision](#deferred-to-a-later-decision)). With no
outcome weaker than a person's look, the class has nowhere to fall.

What this decision holds fixed: 0071's rules (one home per rule, a closed vocabulary matched
without a wildcard, observe before decide, every guard bound by a test); 0082 (a contract version
is minted only under an accepted decision, so every version below is proposed and none is
minted); no model decides a routing, and a seat's "no code change applies" is prose, not a
judgment; 0106's cause and budget stand; 0051's landing and the landing recipe stand; and 0063:
Linux and macOS only.


### What #1093's acceptance becomes

Narrowed by ruling 1 (a) and the narrowing of 2026-11-20, the issue's acceptance is this: **in a
landing run with the gate, a non-green CI observation parks for a person before any seat attempt
beyond the `ci` gate's own exec attempt.** A non-green observation is one on which a row of the
[park table](#the-park-table) is true. The person reads the observation and re-runs or rules.
The observation is itself an exec attempt: the engine journals `effect/started` with an attempt
id before the `ci` gate's driver starts (`crates/brokkr-runtime/src/engine/dispatch.rs:147-165`).
What the park saves is the model seat the adopter counted, the one that diagnoses: a red CI no
longer reaches an implement visit, or any other seat, through this gate. The park is `ruled`,
names the gate's rule, and carries the observation's digest (ruling 7).

What #1093 does not get now, and why:

- **The re-run once.** No forge write is sent, because none can be confined to the authorized
  attempt and closure when it is sent (<https://docs.github.com/en/rest/actions/workflow-runs>).
  A person re-runs.
- **The typed port-bind park.** A port bind during service boot parks like every other red, on
  row `not-success`: for a person, as `ruled`, with no word that says "the runner's". Typing
  it as a runner fault is deferred with the classifier
  ([Deferred to a later decision](#deferred-to-a-later-decision)). The observation reads no job
  log, so it cannot tell a port bind from a red test either: what it gives the person is where
  to look, the run id, attempt and job id the sealed observation names (ruling 9), from which
  the person opens that job's log on the forge, tells the two apart there, and re-runs.
- **The wall clock.** The wait until a person looks remains. What is gone is the diagnosing
  seat attempt the issue counts.
- **The timing-sensitive test.** It parks for a person too, on row `not-success` as every red
  does; nothing tells it apart. Flaky-test handling is a follow-up of its own.
### The gate grants nothing

This is the frame every ruling below is read in, and this is its one home.

**Nothing the `ci` gate judges lets anything reach a later phase, or a merge, that landing
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

**The outcomes, stated once.** Nothing leaves `ci` but two roads:

- **`green`**, the gate's `pass`, which proceeds to `classify` exactly as landing's own entry
  does: the complement of the [park table](#the-park-table);
- **a `ruled` park for a person**, on every true row of the [park table](#the-park-table),
  which is the one home of what parks. The park names the gate's rule and carries the digest
  ruling 2's outcome table gives the attempt, or says no observation exists (ruling 7). The person reads the observation and re-runs or rules.

There is no outcome weaker than that park, so the rule that an unestablished fact must not
weaken a refusal is one sentence here: **whatever is not established `green` parks**, because a
fact a row reads that is not established makes that row true ([the park table](#the-park-table)). So
`green` is not a verdict: it is the one observation the gate does not hand to a person. Every
other part of this decision points here rather than restating the outcomes.

**Every outcome of a wrong or forged observation is bounded:**

| A wrong … | Costs |
|---|---|
| `green` | landing as it is today: review, ship, and the forge's required checks at merge |
| park | a person's look |

The gate grants nothing, then: a wrong `green` reaches only what landing without the gate
reaches, and a wrong park reaches only a person. No row reaches a seat's input as authority
either: later seats receive no projection of the observation (ruling 6's projection).

**What this is not.** A change that weakens CI's own verdict, such as `deny.toml` widened to
admit a licence, is green on the forge as well. That is landing's existing exposure, and review
and the forge hold it, unchanged. It is not a residual of this gate, and this decision does not
claim to close it.

**Enforcement binding (0111d):**
`after_ci_pass_the_composed_table_is_landings_own_rule_for_rule` (the composed `landing-ci`
table, less its `ci` rules, `initial` and phase, is `landing`'s rule for rule) and
`nothing_in_the_recipes_or_seat_scripts_reads_the_ci_phases_result` (a search of recipes and
`scripts/` for `ci_observation`, `ci_green` and a `from: ci` decision), both deriving the `ci`
rows from the composed table; and, since a seat reads its built input rather than the tree,
ruling 6's projection tests over the engine's own input builder.

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

No house recipe has a CI phase. `landing`'s phase list is `classify`, `implement`, `verify`,
`review`, `ship`, `done` and `stop` (`recipes/landing/policy.json:5-7`), its normal route
`classify` → `verify` → `review` → `ship` → `done`, and "the rest of the checks main requires
stay CI's" (`recipes/landing/README.md:5-11`). That is an absence of a CI reader, not an ordering. CI runs
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

**Measured by 0111a** (built at `9679ac541`): this repository's own CI, eight cases on four
heads, read with `gh api --method GET` by the controller on 2026-11-21 and pinned by digest under
`crates/brokkr-core/src/ci/evidence/`, whose `README.md` says how each was taken. Each fact below
names its payload; where that README and a payload disagree, the payload wins.

- **Check runs** (`<case>/check-runs.{latest,all}.page<n>.json`) carry `id`, `name`, `status`,
  `conclusion`, `head_sha`, `details_url`, `check_suite.id`, `app`, `pull_requests` and their
  times, and name no attempt; `filter` defaults to `latest`. The app is `github-actions`, id
  15368, on every check run read.
- **Runs** (`<case>/runs.page1.json`, `run.<id>.json`, `run.<id>.attempt<n>.json`) carry
  `run_number`, `run_attempt`, `created_at`, `run_started_at`, `event`, `path`, `status`,
  `conclusion`, `head_sha`, `pull_requests` and `previous_attempt_url`. No field names a tested
  commit beyond `head_sha` (ruling 3).
- **Jobs** (`run.<id>.jobs.{latest,all}.page1.json`, `run.<id>.attempt<n>.jobs.page1.json`)
  carry `id`, `run_id`, `run_attempt`, `name`, `status`, `conclusion`, `started_at`,
  `completed_at` and `steps`, and no workflow or matrix key.
- **The words seen:** `completed` with `success`, `failure`, `cancelled` or `skipped`, and
  `in_progress` with a null conclusion. The documentation's other words (`queued`, `waiting`,
  and `neutral`, `timed_out`, `action_required`, `startup_failure`,
  <https://docs.github.com/en/rest/checks/runs>) were not caught: **unmeasured**, and kept in
  the closed list. Whether the API says who cancelled a job was not looked for.
- **Pages:** every list read with `per_page=100`, its last page the short one, `total_count`
  on the first page (`failed-jobs-rerun-with-cancelled-twin/check-runs.all.page{1,2}.json`: 103,
  as 100 and 3). The largest response is 356,478 bytes
  (`older-run-full-rerun-complete/index.json`, its `check-runs.all.page1.json`), under the
  8 MiB bound; the README's 309,267 is `failed-jobs-rerun-with-cancelled-twin`'s, not the
  largest, and the payload wins.
- **Still unmeasured:** a fork's pull request; a last page of exactly 100 items; whether
  `gh api` streams a body or holds it whole.

The observation reads no step and no job log (ruling 3), so no step's role enters it. The re-run
verbs, which this decision never calls (ruling 1), are at
<https://docs.github.com/en/rest/actions/workflow-runs>.

## The rulings

Each ruling gives its options, what each costs, refuses and changes, and one recommended option.
Ruling 1 is the operator's, ruled on 2026-11-18 and narrowed on 2026-11-20. For every other
ruling the operator may answer "accept all" or pick. Rulings 4 (the classifier) and 8 (how its
park read in 0106d's budget), as fix rounds 1093r2 to 1093r8 drafted them, are not this
decision's ([Deferred to a later decision](#deferred-to-a-later-decision)); the rest keep their
numbers, so the review record's citations still resolve.

### 1. Brokkr makes no forge write

**Ruled by the operator on 2026-11-18: (a).** The operator answered this choice in chat with
"a for 1093".

**Narrowed by the operator on 2026-11-20.** The operator ruled in chat "narrow 0111 as
recommended": keep the CI reading, the trust-root model and the security bounds; park every
non-green CI result for a person; defer the automatic classification and the implement route
to a later decision. That amends (a) below by deferring "classify". No forge write stays, and a
person still re-runs. Accepting the rest of this decision is still the operator's.

- **(a) Ruled by the operator: observe, classify and park; narrowed to observe and park.** The
  engine and the recipes observe CI, and the gate parks on every true row of the
  [park table](#the-park-table), with no diagnosis seat attempt. They send no re-run request and hold no write credential. A
  person reads the observation and re-runs or rules.
  - Cost: the issue's "re-run once" is not built. The wall-clock wait for a person remains, but
    the diagnosing seat attempt the issue counts is gone.
  - It refuses every forge write. A park costs a person's look, never a pass or a re-run.
- **(b) An automated re-run on GitHub, as the forge offers it.** The engine would send one of
  the two verbs above, with no attempt precondition (another actor's re-run between read and
  write is re-run or widened again), dependents re-run with the target (an audit among them, its
  new result replacing the observed one), no mark of which request made a new attempt, and a lost
  receipt that cannot be told from a request never sent. The operator did not take it.
- **(c) An automated re-run only on a forge that gives an attempt-bound, closure-bound write.**
  That is a forge whose re-run takes the attempt, refuses when the attempt moved, and re-runs
  exactly the named jobs. None is measured. It is the follow-up decision this one names, and that
  decision would rule the durable intent, its reconciliation and the scope rules a write needs.

**What leaves with the write**, each drafted by fix round 1093r2 for a finding that held only
because a write was sent: the writer seat and its token, the durable intent and its
reconciliation, the re-run rounds and count, the re-run routes with the classes only they drew,
and the rule that let an audit job be re-run.

**Enforcement binding:** 0111c's compile test refuses any seat in the CI overlay that declares a
write binding, and 0111b's compile refuses an observer credential declared with any `access`
but `read` (ruling 2).

### 2. What reads the checks

**Recommended: (a) a recipe-declared exec gate observes, and the engine judges.**

- **(a) An exec gate, `ci`, observes; core judges; the table routes.** The gate runs
  `brokkr driver exec` over a script, as `fast`'s verifier does, with no model. It waits for the
  gated head's checks to settle and writes what the forge said as a typed observation (ruling 3).
  The engine judges that observation green or not with a pure core function (ruling 3) and the
  table routes on the engine-owned result. Observing and deciding stay apart (0071 ruling 10).
  Cost: one script, one core module, one engine input. It refuses forge-specific network code in
  the engine. It serves an adopter's exec gate and a future house one through the same contract.
- **(b) A built-in engine effect that calls the forge.** No child launch to prove, but HEAD's one
  HTTP client, `ureq` in `brokkr-bridge`, speaks to Looper (`crates/brokkr-bridge/src/lib.rs:1-7`,
  `:210-219`), so (b) builds a client, a credential path and a forge vocabulary into the runtime.
  Not recommended now; it would write (a)'s contract, so it stays open as a slice if ruled.
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
  unmeasured; slice 0111b's test settles it.
- **Its launch.** Checking where `gh` resolves from inside the script proves nothing about the
  process that runs the script. An inherited `BASH_ENV` naming a worktree file runs before the
  script's first line, and `bash` itself resolves through `PATH`, which 0049 ruling 3 leaves
  unpinned under `harness` and `open` (quoted at
  `crates/brokkr-runtime/src/bundle/model_policy_tests.rs:2595-2596`). So the launch is ruled
  below, and the engine proves it before it spawns anything.

**The observer's launch is trusted, or the observation refuses.** **Recommended: (a) an
engine-built launch, proven before the spawn.** The process that observes CI, read-only as it
is, is launched with all four of these, or the engine refuses before the spawn: the attempt
ends as the outcome `launch-refused` naming the condition that failed, which parks on
`launch-untrusted` (the outcome table below), and observes nothing:

- **A pinned executable identity outside the change, executed as the bytes it verified.** The
  `ci` seat's interpreter and each tool it calls (`gh`, `jq`, `head`) are declared in ruling 5's file by
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
  but no site executes a descriptor, so 0111b builds that.
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

  The alternatives: **(b)** copy the verified bytes into an engine-owned 0700 directory and
  execute from there. Not recommended: every process of the account can still reach it, so it
  shortens the window and does not close it. **(c)** the engine makes the read-only call itself
  (ruling 2 (b)): no executable to bind, at the cost of an HTTP client, a credential path and a
  GitHub vocabulary in the runtime. It stays open as that option's slice.

#### The account is the trust root

This is the one home of the rule; every other mention points here.

**The launch claims no isolation of the observer from any process of the account, whether
Brokkr launched it or not.** Fix round 1093r5's boxed-only admission could not see a second run
under the same UID, started with another map (`--realms`, `crates/brokkr-cli/src/cli_args.rs:220-225`;
`crates/brokkr-runtime/src/realms.rs:409-412`) or from a site with no hands, which HEAD spawns
unboxed (`crates/brokkr-runtime/src/engine/dispatch.rs:555-559`,
`crates/brokkr-runtime/src/engine.rs:2867-2869`); and run
`fix-round-1093r4-for-issue-1093--b1cf18e6`'s adversarial probe had such a sibling open another
process's pipe endpoint through `/proc/<pid>/fd/<n>`.

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

- **The frame.** A forged or tampered observation costs at most a person's look, or landing as
  it is today, never a merge ([The gate grants nothing](#the-gate-grants-nothing)); and no
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
exercised in this session: **unmeasured** until 0111b's test, and if that test can open the
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
(#1091; in fix round at the story's base, so 0111b lands after it), and are held by descriptor
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
hold, the launch refuses. The offline judgment (0111c) runs on both systems (0063).

- **(b) An account-wide fence: refuse while any unboxed site of any run shares the UID.** Not
  taken. It needs a host registry of runs and sites that Brokkr does not keep, and it still
  cannot see the operator's own processes, which hold the same store.
- **(c) Fix round 1093r5's `namespace`-only admission.** Removed, with its refusal word
  `launch-unboxed-peer`, for the blind spots above.
- **(d) Linux non-dumpable processes** (`PR_SET_DUMPABLE` 0,
  <https://man7.org/linux/man-pages/man2/prctl.2.html>; not read in this session). Under the
  trust root it buys nothing the store does not already give away, costs the operator's
  diagnostics of the engine, is expected to reset on `execve` (unmeasured), and has no macOS
  equivalent. Not recommended.

**The result comes back through the engine, never the worktree.** HEAD hands every seat a result
path under the run-local results directory (`crates/brokkr-runtime/src/engine/input.rs:69-82`),
which is the folder's state or, by default, `<workdir>/.forge`
(`crates/brokkr-runtime/src/engine.rs:1047-1050`, `crates/brokkr-runtime/src/places.rs:687-690`).
In the default form that is under the worktree, where the change and every other seat can write.
So a `ci` seat's observation is never read from a file. The engine captures the observer's
stdout through its own pipe, bounded at 1 MiB; an observer that writes more makes
`observation-too-large` true. HEAD's exec adapter already captures stdout into its invocation
(`crates/brokkr-protocol/src/adapters/exec.rs:42`, `:50`, `:71`), but decodes it lossily; the
`ci` path parses the raw bytes, and bytes that are not UTF-8 make `observation-unreadable` true.
A file under `.forge/results` for a `ci` attempt is never read, and its presence makes
`result-file-present` true: no writer of the engine's makes one, so it is evidence a person
looks at. The observer writes nothing to stderr but its own diagnostics, and the `ci` site
journals stderr's length and digest only.

**Every admitted attempt ends in one typed outcome, and none is an effect failure.** At HEAD a
determinate effect failure returns the effect to `ExecuteEffect` with one more failed attempt
(`crates/brokkr-core/src/fold.rs:669-693`), and the engine then retries it, or ends it at
the seat's attempt cap with an `attempt-cap` park (`crates/brokkr-runtime/src/engine/cursor.rs:105-128`);
the exec driver reports a non-zero exit as `Failed`
(`crates/brokkr-protocol/src/adapters.rs:6754-6762`). A `ci` attempt on that road would buy a
second observer attempt, or park with no row. **Recommended: (a) the `ci` site's driver reports
every attempt it ran to an end as `effect/succeeded`, carrying an engine-built outcome,** so the
fold takes it to `Decide` (`fold.rs:658-667`) and the table routes it, once. The outcome is a
closed enum, `CiAttempt`, the envelope `ci-observation/v1` defines around the observation; the
observer writes only stdout, and the driver builds the envelope, so no observer byte can name
its own outcome:

| Outcome | When | Spawned | Row it makes true | The park's digest covers |
|---|---|---|---|---|
| `observed` | the observer exited 0 before its deadline, and its stdout, inside its bound, is UTF-8 and parses | yes | none of its own; the observation's rows judge it | the captured stdout bytes, all of them |
| `unbound` | the run's `run/started` holds no pull request (ruling 3) | no | `pull-request-unbound` | nothing: no observation exists |
| `launch-refused` | a launch rule of ruling 2 failed before the spawn, or the driver failed to spawn (below); it names which | no | `launch-untrusted` | nothing: no observation exists |
| `deadline` | the observer had not exited at its own bound, which ends a reporting margin before the seat's deadline; the driver kills the observer's process group | yes | `observer-deadline` | the stdout bytes captured before the kill |
| `exited` | the observer exited non-zero, or on a signal; it names the status | yes | `observer-exited` | the captured stdout bytes |
| `absent` | the observer exited 0 with no stdout byte | yes | `observation-absent` | nothing: no byte was written |
| `too-large` | stdout passed 1 MiB; the driver stops reading at the bound plus one byte | yes | `observation-too-large` | the first 1 MiB, the bounded witness, with the count read as a lower bound; no byte past it is read |
| `unreadable` | stdout is not UTF-8, or does not parse under the contract; it names the reason | yes | `observation-unreadable` | the captured stdout bytes |

The digest is SHA-256 over exactly the bytes the seat record seals for that attempt, after
0012's masking, so a person checks it against the journal; where the table says nothing, the
park names no digest, says `no observation`, and names the attempt id. Only `observed` carries
an observation, so the observation's rows are evaluated only on it (the dependency rule in the
[park table](#the-park-table)).

**The record is HEAD's, and the word is the engine's.** HEAD's fence checks every
`effect/succeeded` before it is appended (`crates/brokkr-store/src/chain.rs:24-45`): seat-record
v6 requires a non-empty `result` string in a closed object
(`contracts/seat-record.v6.schema.json:557-571`), and a refused record becomes `effect/failed`
(`crates/brokkr-runtime/src/engine/sequence/success.rs:17-35`), the road the fold retries. So
the driver seals every `ci` attempt with the constant word `fail`, never a word the observer
wrote, and the `CiAttempt` envelope as its `ci_observation` input; decide, for a `ci` seat
alone, raises the decision's word to `pass` exactly when the judgment is `green`. No word on the
record can pass, and `word-unbound` stays the table's totality row (ruling 6). **Proved by
0111c:** `a_ci_attempt_record_passes_heads_fence_and_decides_from_the_judgment` (an observed
green attempt and a no-spawn `launch-refused` one, each through the real store append, fold and
decide: both append as `effect/succeeded` with the word `fail`; the first decides `CI-PASS`, the
second `CI-PARK` naming `launch-untrusted`).

**Pre-admission errors are not outcomes.** They end before a run exists, so they journal
nothing and need no row: every compile refusal of rulings 2, 5, 6 and 10; and run start's
refusals before the journal opens (`crates/brokkr-runtime/src/launch.rs:301-306`, HEAD's
`door` in `engine/start.rs`), which are the entry check's, `BeforeCi` among them (ruling 6), a
`ci` seat whose bundle layer is inside the worktree (above), and `--pull-request` on a bundle
with no `ci` seat (ruling 3). Each is a typed `LaunchError` or compile refusal the operator
reads at the prompt.

**Who settles each end.** The driver reports every outcome above, and the engine settles three
ends the driver cannot report:

- **A driver that fails to spawn.** HEAD settles it as `effect/failed` (`engine.rs:1467-1484`),
  which the fold retries. For a `ci` site the engine settles it once as `launch-refused`, naming
  the spawn error, on `launch-untrusted`; nothing ran, so no observation exists.
- **The seat's watchdog.** At the seat's deadline HEAD kills the whole process group, driver
  included, and reports the attempt `Indeterminate` (`crates/brokkr-protocol/src/process.rs:503-520`).
  So the driver enforces the observer's own bound, which ends a reporting margin before that
  deadline, and reports `deadline` itself; the compile refuses a `ci` seat whose bound leaves no
  margin under its `timeout_seconds`. The watchdog stays the backstop.
- **A moved head.** HEAD replaces any settlement of a gate attempt whose head moved with
  `effect/indeterminate` (`engine.rs:953-995`).

The watchdog's kill, a moved head and a driver that dies before it reports stay HEAD's
indeterminate, held for operator judgment and never retried (`fold.rs:694-699`): the one end that
is not `CI-PARK`, and never `CI-PASS`. **Proved by 0111c**, each through dispatch and the fold,
counting attempts: `a_ci_driver_that_fails_to_spawn_parks_once_as_launch_untrusted_with_no_retry`
(`CI-PARK` naming `launch-untrusted`, one attempt, no `effect/failed`);
`an_observer_past_its_own_bound_is_killed_by_the_driver_and_parks_once_as_observer_deadline`
(`CI-PARK` naming `observer-deadline`, the digest of the stdout captured before the kill);
`a_seat_watchdog_kill_of_the_ci_driver_is_indeterminate_never_ci_park` and
`a_head_moved_during_ci_is_indeterminate_never_ci_park` (one `effect/indeterminate`, no `ci`
decision, held for operator judgment). **Proved by 0111b:**
`a_ci_seat_whose_observer_bound_leaves_no_reporting_margin_refuses_to_compile`.

**Live admission is refused until the first live producer lands, and this is the one statement
of the order.** 0111b builds the observer, its trusted launch and the compile's `ci`
declaration, but its compile refuses every bundle that declares a `ci` seat, a `ci_observation`
input or a `ci-checks.json`, a custom exec recipe's included, with the reason that the gate is
unbuilt. 0111c lands the judgment and the park rule, tested through builders and test-only
bundles the live compile never admits. **0111d lifts the refusal**, for `landing-ci` and a
custom exec recipe alike, in the same change that lands the first live producer, so no live
compile admits a `ci` seat before the launcher, the judgment and a composed table that validates
whole all exist. Refused alternative: lifting it in 0111b, which would admit a live `ci` seat
whose observation no judgment reads and whose table validates under no published schema
(ruling 10).

What HEAD lacks for this form, each built by slice 0111b: a cleared environment for a site
without hands, which spawns in the engine's own (`SiteSpawn::inherit`,
`crates/brokkr-runtime/src/engine.rs:2040-2050`; `harness_command` adds bindings without
clearing, `crates/brokkr-protocol/src/adapters.rs:756-770`), though the cleared
`SpawnEnv::Exactly` exists (`crates/brokkr-protocol/src/process.rs:85-94`, `:303-305`) and an
unboxed exec dispatch with hands uses it (`engine.rs:2871-2880`); a working directory other than
the worktree (`crates/brokkr-protocol/src/adapters/exec.rs:33-42`); a pinned executable identity
(0049 ruling 3); execution of verified bytes; an engine-owned result channel; and 1091a's
descriptor-held walk, not yet on main. The other forms: **(b)** a forge client in the engine,
ruling 2 (b)'s cost; and **(c) a boxed observer under `namespace`.** The box clears the environment and starts from `/`
  (`crates/brokkr-protocol/src/hands.rs:764`), and supplies the filesystem and `PATH` (0049,
  `docs/decisions/0049-the-platform-classes.md:43-46`). But HEAD refuses any hands site with a
  secret binding (`crates/brokkr-runtime/src/bundle.rs:3940-3967`), so a boxed observer could
  read only with no credential: a public repository, under the forge's unauthenticated rate
  limit (unmeasured), and on Linux only. Not recommended.

**The observer's credential is read-only.** A forged observation reaches no further than the
bounded table in [The gate grants nothing](#the-gate-grants-nothing): a forged `green` is landing
as it is today, and a forged park a person's look. What the launch rules guard is that the
observation is the forge's, against the change's tree and boxed seats, so a person reads what
the forge said and `green` is the forge's word; what bounds the account is the trust root
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

### 3. The observation, the judgment, and where the journal keeps them

**Recommended: (a) the observation is the seat's, and the judgment is the engine's alone.**

- **The observation is the seat's.** It is a seat-owned typed record input,
  `ci_observation`, under `ci-observation/v1`, which the `ci` seat declares in its `inputs`.
- **The judgment is the engine's alone.** It is drawn by one pure core function over the
  sealed observation and the pinned list, and reaches the table as one engine-owned flag
  (ruling 10), written on the gate's `transition/decided`.

They must be owned apart. The first draft made one `ci` input engine-owned and had the seat
write it, which HEAD refuses twice over. A seat that declares an engine-owned input fails the
compile (`crates/brokkr-runtime/src/bundle/assemble.rs:327-332`). And a seat's payload loses
every engine-owned key, and every undeclared one, before the table sees it
(`crates/brokkr-runtime/src/engine/decide.rs:73-82`). So the observation must be seat-owned to
reach the engine at all, and the judgment engine-owned so no seat can reach it. A seat-owned
**record** input is new: every record input in the registry today is engine-owned
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

**The intended pull request is bound at launch, before the observer runs.** The observation
names a pull request, and the association rule below compares each run against one; but the
number it is compared against must not be the observation's own. Two open pull requests can
share a head, and the highest run on it can name the other one, uniquely: compared against
itself, that observation would agree. The compiled `ci-checks.json` pins the repository and
the observer, never a per-run pull request (ruling 5). **Recommended: (a) the operator selects
the pull request at launch, and the run journals it before any effect.**

- **The selection.** `brokkr run` gains `--pull-request <number>`, a positive integer. It is
  read only for a bundle that declares a `ci` seat; for any other bundle the launch refuses the
  flag before the journal opens, beside HEAD's entry checks (`crates/brokkr-runtime/src/launch.rs:301-306`),
  so it has no consumer it does not reach (0071 ruling 6).
- **The record.** The number is an optional `run/started` member, `pull_request: {number}`,
  added under the contracts README's additive rule for `run/started` members, as proposed
  decision 0094 added `entry` (`contracts/README.md:246-256`). Its object denies unknown
  members, and the core fold never reads it. HEAD writes `run/started` in the store's one start
  transaction, before the run's first effect (`crates/brokkr-runtime/src/engine/start.rs:88-117`),
  so the number is journaled before the observer is launched. Its extension schema waits for
  the operator's acceptance (0082), as 0094's does.
- **The use.** At the `ci` phase the engine reads the number from the run's own `run/started`,
  hands it to the observer in its argv beside `--repo`, and compares against it alone: it is
  the only source of the pull request P1 and P5 read. A continuation imports none: `--continues` is attribution only
  (`launch.rs:453`), and the new run names its own. The operator's retry of a park reads the
  same run's record.
- **Its rows.** A `run/started` that holds no number, whether the flag was not given, the run
  was started by a build before 0111b, or the run came from a dispatch that carries none, makes
  `pull-request-unbound` true; the engine reads that before the launch, so the observer is never
  spawned for an unbound run. An observation that names another number makes `unrelated` true,
  and a current run associated with another pull request, uniquely or not, makes
  `association-unproven` or `association-ambiguous` true; and an open-pull-request list on the
  head that lacks it or holds another makes `pulls-disagree` true.

Refused alternatives: **(b)** finding the pull request from the branch through the forge at
`ci`, which reads it from the same source the observation is compared against, and on a shared
head finds two; and **(c)** a pin in `ci-checks.json`, which is compiled once per recipe and
cannot name a per-run fact.

It then lists every check run the forge reports for that head, always with `filter=all`: the
forge's default is `latest`, which drops an earlier attempt's check runs within the same run
(0111a: `failed-jobs-rerun-with-cancelled-twin/check-runs.latest.page1.json` holds 70,
`check-runs.all.page1.json` and `page2.json` 103). Checks outside Actions are included:

- a check run carries its id, its app (slug and numeric id), its name, its `head_sha`, its
  status, its conclusion and its `details_url`, and names no run attempt; its run, attempt and
  state are read through its job ([the join](#the-park-table), below);
- every workflow run the forge lists for the head, with its path, event, number, attempt,
  `run_started_at`, status, conclusion, `head_sha` and its pull-request association (below), so
  the engine can tell which run is current and whose it is; and every job of each such run,
  under `filter=all`, each with its `id`, `run_id`, `run_attempt`, `name`, status, conclusion,
  `started_at` and `completed_at`;
- every open pull request on the head, read as 0111a read it,
  `pulls?state=open&head=<owner>:<branch>` (`dispatch-in-progress-beside-pr-green/index.json`),
  each with its number, state and `head.sha`, so a head two pull requests share is seen;
  `commits/<sha>/pulls` is not that list, since it still names a closed one
  (`cancelled-twin/commit.pulls.json`: #1222, `closed`);
- a commit status carries its context, its state and its creator's login and numeric id
  (<https://docs.github.com/en/rest/commits/statuses#get-the-combined-status-for-a-specific-reference>),
  so it can be named; it is never admitted as required evidence (below).

**The reads are bounded, and an overflow is a witness, never a truncation.** Each list is paged
by the forge, and the observation carries, for each, whether it was read to its last page. Each
has a bound the contract fixes: 1,024 checks and 1,024 statuses for the head, 256 runs per
workflow key, 1,024 jobs per run, 64 open pull requests on the head and 256 contexts in
the forge's required set. A list that reaches its
bound is not truncated into a complete-looking one: the observation seals, in its place, a
bounded **overflow witness**, the list, the limit, the count and pages read when the observer
stopped (a lower bound), and no entries past the bound. A witness, like an unread page, leaves
the list incomplete. **Every list the observer reads has its own completeness row**, because
P2 needs every one of them, not only those another predicate reads: the check runs for the head
(`checks-incomplete`), the commit statuses for the head (`statuses-incomplete`), each workflow
key's runs (`runs-incomplete`), each run's jobs (`jobs-incomplete`), the open
pull requests on the head (`pulls-incomplete`) and, where it is read, the forge's required set
(`required-set-incomplete`). A list whose overflow witness or unread page the observation
carries makes its row true, whether or not any other row reads that list. The whole observation
is also bounded at 1 MiB on the observer's stdout (ruling 2).

**How a list's last page is told, as measured.** Every list is read with `per_page=100`; the
last page is the one holding fewer than 100 items, and the first page's `total_count`, where the
list carries one, is the whole count (0111a: `two-runs-both-rerun/check-runs.all.page{1,2}.json`,
136 as 100 and 36). A list whose items read differ from its `total_count` is not read to its last
page. A last page of exactly 100 is **unmeasured**, so a full page is always followed by one more
read, and a list whose read ends on a full page, or on a page that does not parse, is not read to
its last page.

**One response is bounded in bytes while it is read, before it is buffered or parsed.** The
item counts above bound a list, and the stdout cap bounds what the observer writes; neither
bounds one forge response, and a single page can be large. So every response has a byte bound
the contract fixes, 8 MiB, and the observer enforces it on the stream: it fetches one page per
`gh api` call, never `--paginate`, and reads that call's body through the pinned `head -c`
with a limit of the bound plus one byte, into a buffer of that size. A response of the bound or
fewer bytes is parsed; when the bound plus one byte is read, it stops reading, ends the call,
parses no byte of that response, and seals a **response witness** in the list's place: the
list, the page, the bound, and the bytes read as a lower bound. A response witness makes
`response-too-large` true, and leaves its list incomplete, so that list's own row is true too.
The largest response 0111a read is 356,478 bytes (`older-run-full-rerun-complete/index.json`),
so the bound sits well above every measured page. Whether `gh api` itself streams a body or holds
it whole before writing it is still **unmeasured**: 0111a did not measure it. So the measurement
moves to 0111b, before it mints: `gh api` through `head` if the pinned `gh` streams, and
otherwise, or while unmeasured, a reader that bounds the transfer itself, named in 0111b's plan
by a revision of this decision before 0111b mints. **Proved by 0111b,** over the selected transport:
`a_response_of_exactly_the_byte_bound_is_parsed_and_one_byte_more_is_a_response_witness` (8 MiB
parses; 8 MiB and one byte seals the witness, and `jq` is never handed it); and
`the_selected_transport_takes_in_no_more_than_the_bound_plus_one_byte_of_a_streamed_page` (a
source streaming 64 MiB counts the bytes taken from it: no more than the bound plus one byte and
one pipe buffer, before any parse). 0111c's
`an_oversized_first_page_through_the_selected_transport_parks_as_response_too_large` binds the
park through that same production reader.

**The observer reads no job log.** No condition of `green` reads a step's log, so no log byte is
read, sealed or journaled, and the person reads the log itself on the forge, from the run,
attempt and job ids the park names. Whether the forge masks every secret is therefore not a
question this decision depends on.

It is parsed once at the edge, with unknown fields refused (0071 ruling 3). An unknown
conclusion word is `observation-unreadable`, naming it.

**No job key: an entry binds its job by display name.** A job carries its display `name`, such
as `test (ubuntu-latest) on linux/1`, but neither its workflow key nor a matrix key (0111a,
every `run.<id>.jobs.all.page1.json`). So each matrix instance is its own entry, bound by its
exact name, as `test (ubuntu-latest)` and `test (macos-latest)` are, and a name is never a
pattern: `test (ubuntu-latest) on linux/1` is not `test (ubuntu-latest)`. A required name the
declared list does not hold parks on `required-set-disagrees` where the forge's set is read.

**`green` needs this pull request's run.** HEAD's workflow keys its concurrency by ref
(`.github/workflows/ci.yml:19-27`) and reads pull-request-specific data, the body and the
labels, into a required job (`:78-102`, where the `by-hand` label changes its outcome). So two
pull requests that share a head can each produce a run on it, with different results, and a run
keyed only by workflow path, event and head may be the other one's. **Recommended: (a) an
Actions execution counts toward `green` only when the forge positively associates it with this
pull request, and with no other.** The field is the workflow run's `pull_requests` array, each
element naming a pull request's `number` and its `head` and `base` with `ref` and `sha`
(<https://docs.github.com/en/rest/actions/workflow-runs#get-a-workflow-run>); a check run
carries the same array. 0111a measured it: on an open pull request's head it names that pull
request on every run and check run, the `workflow_dispatch` run's included
(`dispatch-in-progress-beside-pr-green/runs.page1.json`, #1237); on the heads whose pull request
is closed it is empty (`cancelled-twin/runs.page1.json`, #1222; likewise #1216 and #1229), though
`commits/<sha>/pulls` still names the closed one. A fork's pull request is still **unmeasured**,
and an empty association is never read as this pull request's. An execution is admitted only when P5 holds, and otherwise the gate parks on
`association-unproven` or `association-ambiguous` ([the park table](#the-park-table)). An array
naming a since-closed pull request beside this one is ambiguous too: the array alone cannot tell
it from the one that triggered the run. 0111a saw a closed one leave the array rather than
stay in it, so that case was not caught; the row stays, the conservative reading. Refused alternatives: **(b)** matching by workflow path, event and head alone, which is
fix round 1093r3's key and admits the other pull request's run; and **(c)** matching by the
merge commit's parents, which cannot tell two pull requests with the same base and head apart.

**Commit statuses are not admitted as required evidence.** A status names a context, a state
and a creator, and attests no revision: nothing in it says what its producer tested
(<https://docs.github.com/en/rest/commits/statuses>). HEAD's required checks are all Actions
jobs: each of the twelve contexts `.github/branch-protection.json:7-20` records (observed
2026-09-26, its `:3`) is a job's display name in `.github/workflows/ci.yml` or, for
`mutants in the diff: brokkr-core`, in `.github/workflows/mutants.yml:42`. So `required_checks`
admits no `status` entry: the compile refuses one, and where the forge's own required set names a
status context, the observation parks as `status-required`: the gate cannot read that
requirement, so it cannot establish that every required check succeeded. A status on the head at
all is `status-present` (the currentness rule). Admitting statuses is a follow-up that needs a
revision attestation the forge does not give a status.

**The required set.** **Recommended: (a) a recipe-declared `required_checks` list, pinned by
the compiled bundle outside the change, each entry bound to its producer.**

- **(a) Recipe-declared.** `required_checks` lives in ruling 5's file, which the operator's
  compile pins. It must be non-empty, or the compile refuses. Each entry names its producer, and
  a check matches it only when every part of that producer matches:
  - **an Actions job** is bound to its app (slug and numeric id), its workflow file's path, its
    display name, and its triggering event; the forge reports no job key (above). For this house
    the audit entry is `RustSec dependency audit` in `.github/workflows/ci.yml` (`:673-674`),
    from `github-actions`, id 15368, on `pull_request`. The required test jobs are named by
    system: `test (ubuntu-latest)` and `test (macos-latest)`, the verdict job over the
    machines' matrix (`:315-322`), not its `… on linux/<n>` shards. Each is its own entry.
  - **No commit status** is an entry, as above.

  Where the observer can also read the forge's own required set for the base branch
  (branch protection, <https://docs.github.com/en/rest/branches/branch-protection>, or rulesets,
  <https://docs.github.com/en/rest/repos/rules>), every check the forge requires must be in the
  declared list, with the same app id, or the observation parks as `required-set-disagrees`,
  for the same reason. The forge's set can only widen what `green` needs, never narrow it.
  0111a read it: for this house it is branch protection's 12 checks, each with `app_id` 15368,
  `strict: false`, from `branches/main/protection/required_status_checks`, and main's ruleset
  holds only the merge queue (each case's `protection.required_status_checks.json` and
  `rules.main.json`); so the observer reads both. Whether a read-only token can read protection
  is still **unmeasured** (the capture's token was not read-only); where it is unread, the
  observation records `forge_required: unread`, and the pinned list alone governs. A read that began and stopped short is not `unread`: it is
  `required-set-incomplete`. Cost: one more list for the operator to keep.
- **(b) The forge's branch protection alone.** It is outside the change, but a repository
  administrator can change it at any time, the read needs an administration scope (unmeasured),
  and an unprotected branch yields an empty set, which must refuse. So an adopter without
  protection cannot use the gate. Not recommended.
- **(c) Whatever checks the forge reports for the head.** Refused: a missing check is invisible
  to it, so an absent audit would pass. That is the fault this ruling closes.

**`green` needs the revision the run tested, established.** HEAD's pull-request workflow tells
three revisions apart: `delivered by brokkr` checks out the trusted base and the pull request's
head by explicit ref (`.github/workflows/ci.yml:49-64`), and the ratchet job asserts that its
checkout is the pull request's merge commit (`:663-671`). A checkout with no `ref`, as the audit
job's (`:681-683`), tests the forge's generated merge revision on a `pull_request` event
(<https://docs.github.com/en/actions/writing-workflows/choosing-when-your-workflow-runs/events-that-trigger-workflows#pull_request>;
unmeasured here). And a run's association SHA is not proof of what it tested, nor is its
`head_sha`: both name the pull request's head whatever the job checked out. So each required
Actions entry declares `tested`, one of two words, and **in every checkout mode the current
execution's tested revision is proven on its own**, from the run's own record of the commit it
tested (its **tested-commit record**), never inferred from the head or the association:

- **In both modes, first:** the observation names the tested-commit record as the run records
  it. Where the forge records no such field, or the observation carries it as `unread`, the
  revision is not established, in `head` mode as in `merge` mode: row `revision-unproven`.
- `head`: the run tested the gated head itself. Three separate comparisons, never one: the
  tested-commit record against the gated head; the run's `head_sha` against the gated head; and
  the association's `head.sha` against the gated head. Matching heads with no tested-commit
  record never stand in for it.
- `merge`: the run tested the forge's merge revision, `refs/pull/<number>/merge`. The
  tested-commit record names the exact merge commit, and the observation names that commit's two
  parents. Its first parent must be the base the observation names, which must be the pull
  request's current base, and its second the gated head. The merge commit is a distinct commit
  from the head, as a valid merge always is, so the comparisons are separate, never one: the
  run's `head_sha` and the association's `head.sha` against the gated head; the merge commit's
  second parent against the gated head; its first parent against the association's `base.sha`
  and the observation's base; and the merge commit against the tested-commit record.

The `tested` word is the operator's statement about a pinned workflow, in the compiled list; a
wrong one costs at most a wrong `green`, which is landing as it is today.

Any one of those comparisons that fails is row `revision-disagrees`, naming the two values that
differ; it is never resolved by preferring one source. (Fix round 1093r4's row demanded that
the association, the run's head, the tested commit and the gated head all be one commit, which
no valid merge is; that row is replaced.)

**The tested commit, as measured: no source, so no entry is `green` yet.** 0111a found that a
run names only `head_sha`, the pull request's head, in both checkout modes, and that no field of
a run, a job or a check run names the merge revision a `merge`-checkout job tested (every
`run.<id>.json` and `run.<id>.jobs.all.page1.json`; the house's audit and ratchet jobs are both
`merge` checkouts). So the rule above applies as written: the observer seals every tested-commit
record as `unread`, with the reason that the forge names none, and every observation with a
matched required entry makes `revision-unproven` true, in `head` mode as in `merge` mode, for
this house and every adopter. **No live observation is `green` until a later measurement**: the
gate, once 0111d lifts its refusal, parks every observation for a person. That is the
conservative outcome, and it costs every landing under the overlay a person's look, never a
pass. What would change it is a source of the tested commit outside the change, measured by a
later capture and ruled by a revision of this decision: a forge field naming a job's checkout,
or a record the trusted base writes; a record a job writes under a workflow file the change can
edit is not such a source.

The revision is the run's, not a step's: a step moved off it is CI's own machinery, green on the
forge as well, landing's existing exposure; proving a step's revision was the deferred
classifier's need. `delivered by brokkr` (`:49-102`) is not in the entry set (ruling 6).

#### The park table

This, the [park table](#the-park-table), is the one home of every condition under which the
`ci` gate parks. **Every row is a
condition that parks; `green` is the complement of the table, never a list beside it.** Every
other passage of this decision that says the gate parks cites the row it means by its id, or,
when it speaks of every row, links this table, and restates no condition. `green` grants nothing
([The gate grants nothing](#the-gate-grants-nothing)); the table says only when the gate
proceeds instead of handing the observation to a person.

**`green` holds only when every row is false and every pass predicate holds.** The pass
predicates are `green`'s own conditions, stated once here and nowhere else:

- **P1, the binding.** The run's `run/started` holds a launch-bound pull request, and the
  observation's repository (owner, name and id), pull request and gated head equal the run's
  pinned repository, that launch-bound pull request and the engine's own git head; and the forge's `head.sha` for the pull request is that head, so the
  head is published.
- **P2, complete inventories.** Every list the observer reads was read to its last page, inside
  its bound, with no page unread: the check runs, the statuses, each key's runs, each run's
  jobs, the open pull requests on the head, and the forge's required set where it is
  read ([the bounded reads](#3-the-observation-the-judgment-and-where-the-journal-keeps-them)).
  Each list's completeness is its own sealed fact, scoped to the
  exact list it names: a job list's to its run id, so a complete check list never
  stands for a complete job list, nor one run's for another's.
- **P3, every required entry matched once, currently.** Every entry of `required_checks` in the
  entry set (ruling 6) has exactly one **eligible current match**: a check from its bound
  producer (its app, workflow path, event and name) in that producer's current execution, the
  one run attempt defined below, read through its job. A check in an earlier attempt of that
  run is **history**, whatever it concluded, and never counts; a check in any other run, or in
  a newer attempt, is P8's (the currentness rule). "Exactly
  one" is read over the complete check list and the complete job list of that run (P2), so
  a second job reporting the entry's name on a page the observer did not read can never leave
  the entry looking matched once.
- **P4, settled success.** Each match's status is `completed` and its conclusion `success`.
- **P5, this pull request's single association.** Each current execution's `pull_requests`
  names this pull request and no other, with `head.sha` the gated head and `base.sha` the base
  the observation names; and the open pull requests on the head are the launch-bound one alone.
- **P6, the revision proven tested.** Each current execution's tested revision is established
  from its tested-commit record, in every checkout mode, as its entry's `tested` word requires
  (above).
- **P7, a readable observation from a trusted launch.** The launch passed ruling 2's rules, and
  the observation parsed under its contract, inside its bound.
- **P8, one consistent snapshot.** For each entry, read through the join, every item names the
  current execution's attempt or its history, each check run agrees with its job, no match is
  carried, the current run is completed, every other run of the workflow on the head is
  completed `success`, and the head holds no commit status (the currentness rule, below).

Each pass predicate is the negation of the rows that name it in the `Negates` column, so no
predicate holds while one of its rows is true, and each predicate is named by at least one row.
A row whose `Negates` cell is empty, such as `producer-mismatch`, parks on a fact outside
`green`'s own conditions. **A fact
a row reads that is not established makes that row true:** a fact absent, unread, overflowed,
unparsed, ambiguous or read from another source than the row names. The observation's contract
carries each fact as a value or as the typed `unread` with the observer's reason, never a
default, and each paged list carries whether it was read to its last page.

| Id | Reads | Negates | True when | Bound by (0111c unless named) |
|---|---|---|---|---|
| `pull-request-unbound` | the run's `run/started` `pull_request` member | P1 | it is absent or unreadable; read before the launch, so no observer is spawned | `a_ci_run_started_without_a_pull_request_parks_as_pull_request_unbound_and_spawns_nothing` |
| `launch-untrusted` | the attempt's outcome, `launch-refused` | P7 | a rule of ruling 2 failed before the spawn, or the driver failed to spawn; it names which | `a_launch_refusal_parks_once_as_launch_untrusted_and_spawns_nothing` |
| `observer-deadline` | the attempt's outcome, `deadline` | P7 | the observer had not exited at its own bound | `an_observer_past_its_own_bound_is_killed_by_the_driver_and_parks_once_as_observer_deadline` |
| `observer-exited` | the attempt's outcome, `exited` | P7 | the observer exited non-zero or on a signal; it names the status | `an_observer_that_exits_non_zero_parks_once_as_observer_exited_with_no_second_attempt` |
| `observation-absent` | the attempt's outcome, `absent` | P7 | the observer exited 0 and wrote no stdout byte | `an_observer_with_no_stdout_parks_once_as_observation_absent` |
| `observation-too-large` | the attempt's outcome, `too-large` | P7 | stdout passed its 1 MiB bound | `an_observer_over_the_stdout_bound_parks_once_as_observation_too_large_naming_the_witness_digest` |
| `observation-unreadable` | the attempt's outcome, `unreadable` | P7 | stdout is not UTF-8, or holds an unknown field or word, or lacks a field | `an_unreadable_observation_parks_once_as_observation_unreadable_naming_its_digest` |
| `result-file-present` | the run-local results directory, for the `ci` attempt's id | | a result file exists for the attempt; it is never read | `a_result_file_for_the_ci_attempt_is_never_read_and_parks_as_result_file_present` |
| `unrelated` | the observation's repository, pull request and gated head; the run's pinned repository, the launch-bound pull request and the engine's git head | P1 | any of the three differs, or is unread | `an_observation_of_another_head_repository_or_pull_request_parks_as_unrelated` and `a_run_launched_for_one_pull_request_parks_when_the_observation_names_another` |
| `head-unpublished` | the forge's `head.sha` for the pull request; the engine's git head | P1 | they differ, or either is unread | `an_unpushed_head_parks_as_head_unpublished` |
| `checks-incomplete` | the check-run list's last-page fact and overflow witness | P2 | the list was not read to its last page, or overflowed its bound; it names the pages read and the witness | `the_check_run_list_unread_or_overflowed_parks_as_checks_incomplete_live_and_on_replay` |
| `statuses-incomplete` | the status list's last-page fact and overflow witness | P2 | as `checks-incomplete`, for the commit statuses | `the_status_list_unread_or_overflowed_parks_as_statuses_incomplete_live_and_on_replay` |
| `runs-incomplete` | each workflow key's run list, its last-page fact and overflow witness | P2 | as `checks-incomplete`, for one key's runs; it names the key | `a_run_list_unread_or_overflowed_parks_as_runs_incomplete_live_and_on_replay` |
| `jobs-incomplete` | each run's job list under `filter=all`, its last-page fact and overflow witness | P2 | as `checks-incomplete`, for one run's jobs; it names the run id and the attempts read | `a_job_list_unread_or_overflowed_parks_as_jobs_incomplete_naming_the_run_and_attempt_live_and_on_replay` |
| `pulls-incomplete` | the open-pull-request list for the head, its last-page fact and overflow witness | P2 | as `checks-incomplete`, for the open pull requests on the head | `the_open_pull_request_list_unread_or_overflowed_parks_as_pulls_incomplete_live_and_on_replay` |
| `required-set-incomplete` | the forge's required set, where read, its last-page fact and overflow witness | P2 | the read began and did not reach its last page, or overflowed; a set never read is `unread`, and the pinned list alone governs | `a_partly_read_forge_required_set_parks_as_required_set_incomplete_live_and_on_replay` |
| `response-too-large` | each response witness | P2 | one forge response passed its 8 MiB byte bound while being read: the bound plus one byte was read; it names the list, the page and the bytes read | `an_oversized_first_page_through_the_selected_transport_parks_as_response_too_large` |
| `status-present` | the complete commit-status list for the head | P8 | it holds any status; a status names no attempt (the currentness rule) | `a_status_on_the_head_parks_as_status_present_live_and_on_replay` |
| `pulls-disagree` | the complete open-pull-request list for the head; the launch-bound pull request | P5 | the list lacks the launch-bound pull request, or holds another; it names every one | `an_open_pull_request_list_without_the_bound_one_or_with_another_parks_as_pulls_disagree` |
| `empty` | the complete check list for the head | P3 | the forge reports no check for the head | `an_observation_with_no_check_names_empty_then_missing_required_for_every_entry` |
| `missing-required` | each entry's matches | P3 | an entry has no eligible current match; a check in history or in another run never makes it present | `a_required_audit_missing_from_the_observation_parks_as_missing_required` |
| `ambiguous-required` | each entry's matches over the check and job lists | P3 | an entry matches more than one check, or more than one job of its current run attempt; or it is seen exactly once over a list whose `-incomplete` row is true, where a second may sit on the unread page; it names every match | `two_matches_for_one_required_check_park_as_ambiguous_required` |
| `execution-ambiguous` | each key's complete run list, each run's `run_started_at` | P3 | an entry's current execution cannot be told: two runs share the latest `run_started_at`, or one's does not read (the currentness rule) | `two_runs_of_one_key_started_at_one_time_or_unreadable_park_as_execution_ambiguous` |
| `execution-unsettled` | the current execution's run status | P8 | it is any word but `completed` | `a_current_run_still_in_progress_beside_green_required_checks_parks_as_execution_unsettled` |
| `other-run-not-success` | every other run of the entry's workflow path on the head, of any event, its latest attempt and its check runs | P8 | its latest attempt is not completed `success`, or a check run of it under a name the declared list holds, after-ship entries included, is not completed `success`; `skipped` is not `success`; it names the run | `a_dispatch_run_whose_delivered_by_brokkr_skipped_beside_a_green_run_parks_as_other_run_not_success` and `a_cancelled_twin_beside_a_green_run_parks_as_other_run_not_success` |
| `snapshot-inconsistent` | each item bearing on the entry, by the join, and the attempt it names; the run list's latest attempt for its run | P8 | an item names an attempt above the run list's latest, in any status; or an earlier attempt's item started after the current attempt's `run_started_at`; it names the list, the item and both attempts | `a_complete_green_attempt_beside_a_queued_newer_attempt_on_the_check_list_parks_live_and_on_replay` |
| `carried-job` | each match's job: its `started_at` and its attempt's `run_started_at` | P8 | the job started before its attempt did, or either time does not read: a failed-jobs-only re-run carried it | `a_failed_jobs_only_rerun_carrying_a_passed_entry_parks_as_carried_job_live_and_on_replay` |
| `attempt-unnamed` | each check run bearing on the entry | P8 | it joins no job read, or its job's `run_attempt` is `unread` | `a_check_run_that_joins_no_job_parks_as_attempt_unnamed_live_and_on_replay` |
| `check-job-disagrees` | each check run bearing on the entry and its joined job | P8 | their status or conclusion differ; it names both | `a_check_run_and_its_job_disagreeing_on_state_park_as_check_job_disagrees_live_and_on_replay` |
| `not-settled` | each match's status | P4 | a match's status is any word but `completed`, `in_progress` (measured) and `queued`, `waiting`, `pending` and `requested` (unmeasured, kept in the closed list) among them, when the observer's polling bound ends | `a_required_check_not_completed_with_a_null_conclusion_parks_as_not_settled_alone` |
| `not-success` | each match's conclusion | P4 | a `completed` match concluded anything but `success`; it names the check and the word. It reads only a completed match, so a null conclusion beside another status is `not-settled` alone | `a_failed_cancelled_timed_out_skipped_or_neutral_required_check_parks_as_not_success` |
| `association-unproven` | each current execution's `pull_requests` | P5 | it does not name this pull request, its head and its base, or is empty | `a_run_with_an_empty_association_parks_as_association_unproven` |
| `association-ambiguous` | each current execution's `pull_requests` | P5 | it names more than one pull request, open or closed; it names every one | `an_association_naming_a_and_a_closed_b_parks_as_association_ambiguous` |
| `revision-unproven` | each current execution's tested-commit record, in either checkout mode | P6 | the record is absent or unread, in `head` mode as in `merge` mode | `a_head_entry_whose_run_records_no_tested_commit_parks_as_revision_unproven` and `a_merge_entry_whose_run_records_no_tested_commit_parks_as_revision_unproven` |
| `revision-disagrees` | each current execution's tested-commit record and its mode's separate comparisons | P6 | one comparison fails; it names the two values | `a_head_entry_whose_tested_commit_differs_from_the_gated_head_parks_as_revision_disagrees` and `an_association_and_a_merge_commit_that_disagree_park_as_revision_disagrees` |
| `producer-mismatch` | every check under an entry-set entry's name from outside the entry's workflow on the head | | a check under that name from another app or workflow path is anything but completed `success`, in progress among them | `a_sole_red_from_another_producer_names_missing_required_for_its_entry_then_producer_mismatch` |
| `status-required` | the forge's required set, where read | | it names a commit status context | `a_status_context_in_the_forges_required_set_parks_as_status_required` |
| `required-set-disagrees` | the forge's required set, where read; the declared list | | the forge requires a context the declared list lacks | `a_forge_required_check_the_list_lacks_parks_as_required_set_disagrees` |
| `word-unbound` | the decision's word and `ci_green`, read only by the composed table's `CI-PASS-UNBOUND`, never by the judge | | the word is `pass` while `ci_green` is false; decide raises the word only on `green` (ruling 2's outcomes), so it is reached only if that were lifted | 0111d: `a_pass_without_green_through_the_real_policy_and_park_writer_parks_at_ci_pass_unbound` |

**Every true row parks, and the park records every one, in the table's order.** There is no
weaker outcome to fall to, so no precedence among the rows is needed and no row suppresses
another: the judge returns every true row of the [park table](#the-park-table) but
`word-unbound`, which only the composed table reads, in its order; a per-entry row once per
entry, in `required_checks` order, and a per-list row once per list. That ordered list is the
judge's output, re-derived from the sealed record and the compiled list, live and on replay; the
park's reason holds its prefix inside 256 bytes with a count of the rest (ruling 7).

**`inspect` paints the whole list only while it can derive it.** With the compiled bundle held,
it re-derives and paints every true row of the [park table](#the-park-table) (ruling 9). With the bundle gone, the journal holds the
observation, the digest and the reason, and not the list the judge read, so `inspect` paints the
digest, the rows the reason names and the count of the rest, and says the rest needs the
compiled bundle; it never paints a prefix as the whole. **Proved by 0111c:**
`inspect_on_a_truncated_ci_park_after_its_bundle_is_gone_paints_the_named_rows_and_the_count`
(a `CI-PARK` whose true rows pass 256 bytes, its compiled bundle deleted: exactly the reason's
rows, the count of the rest and the bundle-gone line, and no other row).

**The dependency rule: what a row does when its subject does not exist.** A row reads one
subject: the attempt (the rows from `pull-request-unbound` to `result-file-present`), the
observation (every row from `unrelated` to `required-set-disagrees`), and, within it, one entry's matches
(the rows from `missing-required` to `revision-disagrees`, evaluated per entry). Where a
subject does not exist, a row that says so is itself true, and a row that reads the absent
subject is not evaluated, so it is never read as false in place of a park:

- **No observation.** On any outcome but `observed`, that outcome's row is true, and no row
  that reads the observation is evaluated. `result-file-present` reads the attempt, and is
  evaluated on every outcome.
- **No match for an entry.** `missing-required` is true for that entry, and the rows that read a
  match (`carried-job`, `not-settled`, `not-success`, `association-unproven`,
  `association-ambiguous`, `revision-unproven`, `revision-disagrees`) have nothing to read for
  it and are false for it. `snapshot-inconsistent`, `attempt-unnamed` and `check-job-disagrees`
  read the items bearing on the entry, and `other-run-not-success` the other runs, not its
  match, so they are evaluated whether or not it has one.
- **No current execution for an entry.** `execution-ambiguous` is true, and the entry has no
  eligible current match, so `missing-required` is true for it as well, before it in the
  table's order. `execution-unsettled`, `other-run-not-success` and `snapshot-inconsistent`
  have no execution to compare with, and are not evaluated for it.
- **More than one match.** `ambiguous-required` is true, and the match rows read every match.
- **An incomplete list.** Its row is true, and the rows that read it are evaluated over what was
  read: a fact the unread pages could change is not established, so an entry seen with no match
  is `missing-required` and one seen once is `ambiguous-required`. Every other row reads only
  what was read; what the unread pages could hold is the list's own row. An overflow witness
  holds no entry, so over it every entry is seen with no match.

So every entry that is not exactly one settled, successful, singly associated, revision-proven
match makes at least one row true, and no subject's absence lets green through. Worked, in the
table's order:

- a complete, empty check list with a non-empty required set: `empty`, then `missing-required`
  for every entry;
- a sole red under an entry's name from another producer, the entry otherwise unmatched:
  `missing-required` for that entry, then `producer-mismatch`;
- an unread engine head: `unrelated` and `head-unpublished`;
- a check list short of its last page beside an absent audit: `checks-incomplete`, then
  `missing-required` for the audit and every other entry seen with no match, and
  `ambiguous-required` for each entry seen once.

The set is exact and deterministic. Live and on replay the judgment takes the one function over the one sealed
observation and the one pinned list ([What a replay reads](#3-the-observation-the-judgment-and-where-the-journal-keeps-them)),
so the rows a replay names are the rows named live.

**The table binds the text.** 0111c's doc test,
`every_park_statement_in_0111_cites_a_row_of_its_park_table`, reads this decision: every
paragraph, list item or table row that says the gate parks, spelled `parks`, `parked` or
`parking` in prose, the bare verb after a modal or `to` (`would park`, `to park`), the noun
`a person's park` (a test name in a code span is not a statement) or `CI-PARK`, outside the
heading, [What main holds at HEAD](#what-main-holds-at-head),
[Deferred to a later decision](#deferred-to-a-later-decision) and [Evidence](#evidence), names
a row id in backticks, such as `unrelated`, and every id it names is a row of this table. Only a
statement about every row, spelled with `every`, may link this table in place of an id; HEAD's
own hold is spelled `held`. Every row has an id, a `Reads` fact and a `Bound by` test. Its
control, `the_park_citation_checker_refuses_a_statement_with_no_row_or_an_unknown_row`, feeds it
a statement naming no row, one naming an id the table lacks, a specific one that only links
the table, and one in each further form (`a person's park is reached on every red run`, `the run
would park on a red`) naming no row, and expects each refused by its line. Its companions:
`every_park_row_is_bound_by_a_test_the_slices_name` (every `Bound by` cell names a test one of
[The slices](#the-slices) lists); `every_test_proved_by_the_slices_is_named_in_its_slices_plan`,
both ways: every name [Proved by the slices](#proved-by-the-slices) lists under a slice appears
in that slice's plan, the plan's `Acceptance rows:` line states how many the table lists for
it and the count agrees, and every slice it lists is one of [The slices](#the-slices); its
removal controls delete one table row, and one plan entry, from a copy of this text and expect
each refused naming the test; and
`the_judges_park_rows_are_the_decisions_park_table_in_order`, which holds the judge's closed row
enum to this table's ids in this order, less `word-unbound`, the composed table's alone.

**The currentness rule: `green` needs one consistent snapshot.** This is its one home; P3, P5
and P8 and their rows cite it. The lists are read one after another, and a run can gain an
attempt between two reads: a person's re-run, or a new run on an unchanged head, which HEAD's
`labeled` and `unlabeled` events start (`.github/workflows/ci.yml:6-10`). The snapshot is the
lists as sealed, and the rule reads nothing else. Every forge fact below is 0111a's, by the path
of its payload under `crates/brokkr-core/src/ci/evidence/`:

- **The join, the one way a check run's attempt is read.** A check run names no attempt: its
  `id` is its job's `id`, its `details_url` names `/actions/runs/<run>/job/<job>`, and only the
  job carries `run_id` and `run_attempt` (every case; for one,
  `failed-jobs-rerun-with-cancelled-twin/check-runs.all.page1.json` beside
  `run.44837139463.jobs.all.page1.json`). So each check run is joined by id to the job of that id
  on its run's job list, and its run, attempt and state are read through that job. One that
  joins no job read makes `attempt-unnamed` true; one whose status or conclusion differs from
  its job's makes `check-job-disagrees` true. Neither is resolved by preferring a side.
- **The current execution, by start.** An entry's **current execution** is the run attempt with
  the latest `run_started_at` among every run the forge lists for its workflow path, its event
  and the head, whatever its `run_number`. `created_at` and `run_number` order runs by creation
  only, and an older run's re-run is newest by `run_started_at` (`older-run-full-rerun-complete/runs.page1.json`:
  run 3568's attempt 2 at 17:32:56Z, after run 3569's 16:50:47Z). A tie at the latest, or a
  `run_started_at` that does not read, makes `execution-ambiguous` true; a current run that is
  not `completed` makes `execution-unsettled` true. An item **bears on** an entry when it is a
  check run or job of that run under the entry's name.
- **Run versus attempt.** An earlier attempt of the current execution's run is history, visible
  only through `filter=all`'s view: its check runs and jobs are read and joined, never count as
  a match, and none may be newer than the current attempt. An earlier *run* is never history: it
  is one of the workflow's runs, ruled by the next item.
- **Every other run of the workflow on the head.** Any other run of the entry's workflow path on
  the head, of any event, whose latest attempt is queued, in progress or waiting, or completed
  with anything but `success`, or which holds a check run under a name of the declared list that
  is not completed `success`, makes `other-run-not-success` true. `skipped` is not `success`. A
  `workflow_dispatch` run writes check runs under the same names (`dispatch-in-progress-beside-pr-green/`),
  and its `delivered by brokkr` concludes `skipped`, since that job runs only for a pull request
  (`dispatch-and-pr-both-green/check-runs.all.page1.json`); HEAD's workflow notes that "a skipped
  job reports success to a required check" (`.github/workflows/ci.yml:312-314`). So the
  after-ship exclusion (ruling 6) is the current execution's alone, a head with a dispatch run of
  the workflow parks, and a cancelled twin (`cancelled-twin/`, run 3560) parks until a person
  re-runs it.
- **One attempt.** By the join, an item of the current execution's run that names an attempt
  above the run list's latest for that run, queued, in progress or concluded, whether it
  succeeded or failed, or an earlier attempt's item that started after the current attempt's
  `run_started_at`, makes `snapshot-inconsistent` true for the entry.
- **Carried jobs.** A failed-jobs-only re-run lists every job in the new attempt, each with
  `run_attempt` 2 and a new id, and the jobs it did not re-run keep attempt 1's `started_at`,
  `completed_at` and conclusion (`failed-jobs-rerun-with-cancelled-twin/run.44837139463.attempt2.jobs.page1.json`:
  27 of 33, the required `RustSec dependency audit` and `test (ubuntu-latest)` among them). They
  are told apart only by a `started_at` before the attempt's `run_started_at`. Such a job is
  **carried**, and a carried match makes `carried-job` true, so the person re-runs all jobs. A
  full re-run re-creates every job, each started after its attempt began
  (`older-run-full-rerun-complete/run.44924886779.attempt2.jobs.page1.json`).
- **The lists that name no attempt are ruled by what they are.** Commit statuses: every head
  0111a read holds none (each case's `statuses.page1.json` is `[]`), so any status at all makes
  `status-present` true. The open pull requests on the head are an association list, not an
  attempt list: it must hold the launch-bound pull request and no other, or `pulls-disagree` is
  true. It carries no attempt, and that is safe: it says which pull requests hold the head,
  never a CI verdict, so no stale or newer attempt can hide in it, and a pull request opened
  after its read is outside the snapshot, as below.
- **No winner is picked.** The rule says only when the snapshot is consistent; it never chooses
  between two attempts. An attempt that starts after the last read is outside the snapshot: its
  reach is a wrong `green`, landing as it is today, and the forge's checks at merge
  ([The gate grants nothing](#the-gate-grants-nothing)).

The sealed observation holds each key's whole run list and each run's jobs, so which match is
current, which is history and which is carried is re-derived from the journal alone.

**The person's re-run is of every job.** A failed-jobs-only re-run carries the jobs it did not
re-run into the new attempt, as above, so the recovery `inspect` names is re-running all of the
run's jobs, which re-creates every one. **Proved by 0111c:**
`a_failed_jobs_only_rerun_carrying_a_passed_entry_parks_as_carried_job_live_and_on_replay`
(R/1: A success, B failure; R/2 re-runs B to success and lists A carried, attempt 2, a new id,
R/1's times: `carried-job` for A); **by 0111d:**
`an_operator_retry_after_a_person_re_runs_all_jobs_observes_again_and_passes` (`CI-PASS`).

The judgment is engine-written: an `Owner::Engine` flag, as `reviewed_heads` is an engine-owned
input (`crates/brokkr-core/src/policy/inputs.rs:116`), so a seat may neither declare nor
overwrite it, as the engine-owned strategy is held (`:87-95`). Both inputs ride in the decision's
`inputs` (`crates/brokkr-core/src/decision.rs:86`). So the journal holds three things, in the
order they happened, with no new event: the observation the seat sealed, the judgment the engine
drew from it, and the rule that routed on it; and, on a park, the park's reason naming the
observation's digest and its true rows (ruling 7).

**What a replay reads.** `judge(observation, pinned)` lives in `brokkr-core`, reads no clock,
environment or network (0071 ruling 1), and is total; live and replay take the one function. Its
inputs are the sealed observation and the pinned `ci-checks.json` with the run's comparison
values (the pinned repository, the launch-bound pull request from `run/started`, and the
engine's git head). HEAD's manifest
records each bundle file as a digest, never its bytes
(`crates/brokkr-runtime/src/bundle.rs:4214-4239`), so a replay reads the list from the compiled
bundle and refuses when its digest is not the manifest's, and reads the comparison values from
the run's own journal: the pull request from its `run/started`, and which record holds the
gated head is settled by 0111c's replay test, and is unmeasured here. Where the bundle is gone, the park's
reason still names the observation's digest, its first true rows and the count of the rest
(`inspect`, above). Fix rounds 1093r4 to 1093r8's `ci-classification/v1` basis leaves with the
classifier.

An attempt with no observation, `launch-refused` or `unbound` among them, is still a sealed
record: its `CiAttempt` outcome is the seat record of its `effect/succeeded`, naming the
condition that failed, so a replay reads the same row from the journal (ruling 2's outcome
table).

- **(b) A new `ci/judged` event.** Moves the event envelope, every manifest and every fold arm
  for a fact an input can carry. Refused, as 0106 ruling 1 refused `run/park-caused`.

### 5. The list: its home and who declares it

**Recommended: (a) recipe data, `ci-checks.json` beside the policy, under `ci-checks/v1`,
compiled into the bundle and pinned with it.**

**`green` still needs a recipe-declared set, re-derived.** P3 matches every required entry by
its producer, so the set must come from where the change cannot edit, and ruling 3 refuses the
forge's protection alone and "whatever the forge reports" ((b) and (c)). So the declared list
stays, and with it its file. What the file loses is every key that existed
only to type a failure: the change classes, the setup steps and the patterns leave with the
classifier ([Deferred to a later decision](#deferred-to-a-later-decision)). What it keeps beside
`required_checks` are the pins the kept bounds read: the repository the binding checks
(ruling 3), and the observer's executables and credential (ruling 2).

```json
{
  "schema": "ci-checks/v1",
  "repository": {"owner": "example", "name": "app", "id": 1},
  "observer": {
    "interpreter": {"path": "/usr/bin/bash", "sha256": "<64 hex>"},
    "tools": {"gh": {"path": "/usr/bin/gh", "sha256": "<64 hex>"},
              "jq": {"path": "/usr/bin/jq", "sha256": "<64 hex>"},
              "head": {"path": "/usr/bin/head", "sha256": "<64 hex>"}},
    "credential": {"binding": "GH_TOKEN", "access": "read"}
  },
  "required_checks": [
    {"actions_job": {
      "app": "github-actions", "app_id": 15368, "workflow": ".github/workflows/ci.yml",
      "name": "test (ubuntu-latest)", "event": "pull_request", "tested": "merge"}},
    {"actions_job": {
      "app": "github-actions", "app_id": 15368, "workflow": ".github/workflows/ci.yml",
      "name": "RustSec dependency audit", "event": "pull_request", "tested": "merge"}}
  ]
}
```

The list is cut short: the house's full `required_checks` is the twelve required contexts
ruling 3 names. The app's numeric id, 15368, is pinned beside its slug, as 0111a read it on
every check run and in branch protection's `checks`. There is no `job` key: the forge reports
none, so a pin nothing could compare would have no reader (0071 ruling 6). The compile refuses a
`status` entry.

`required_checks` must be non-empty and hold at least one entry with no `"after"` (ruling 6's
entry set is never empty); every Actions entry must name all of its producer's parts and its
`tested` word, and may carry `"after": "ship"` (ruling 6's entry set; the only word v1 admits,
so the field is a closed enum of one). The compile refuses a list that leaves any of them to a
default, and refuses any key v1 does not name, `change_classes`, `setup_steps` and `patterns`
among them. `repository` pins the binding ruling 3 checks.

The operator declares it, as any recipe or `extends` overlay (0017). No seat writes it: the
run reads the list its compiled bundle pinned at start, never the change's tree, so a branch
that edits `ci-checks.json` changes the next run's list, by the operator's compile, not its own.

- **(b) Host data under `forge.host`.** One list for every recipe; but which checks a pull
  request needs is a property of a repository's workflows, not a machine. Refused.
- **(c) Engine defaults.** A second home for a fact the recipe owns (0071 ruling 5). Refused.

### 6. Each outcome, and every route

**Recommended: (a) a pass proceeds; everything else parks for a person, on the
[park table](#the-park-table).**

The judgment a table sees is closed: `green`, or not. A seat cannot write it.

- **`green`**: the gate's `pass`; the run proceeds to `classify`, exactly as landing's own entry
  does, and nothing later reads it ([The gate grants nothing](#the-gate-grants-nothing)).
- **Everything else**: the run parks `ruled` for a person (ruling 7), naming every true row of
  the [park table](#the-park-table), with no seat attempt beyond the `ci` gate's own exec attempt. The person
  reads the observation and re-runs or rules.

These are the frame's two roads ([stated once](#the-gate-grants-nothing)).

**The `ci` result's projection.** A rule is not the only reader of a phase's result: a seat
reads its built input. HEAD hands every seat but triage its run context: `last_decision`, the
decision that brought it here; `results`, the result and inputs of every phase earlier in the
table's order; and, on a return, `returned_from` (`crates/brokkr-runtime/src/engine/input.rs:110-168`).
A phase result carries its decision's inputs (`crates/brokkr-core/src/decision.rs:113-126`).
Unchanged, that would hand `ci_observation` and `ci_green` to `classify` (its `last_decision`
after `CI-PASS`) and to `verify`, `review` and `ship` (`results.ci`, because `ci` precedes them
in the order), where a model reviewer could weigh a `pass` as CI's verdict. **Recommended: (a)
the projection is fixed, in the one input builder:**

- `results` never holds `ci`, for any seat;
- a `last_decision` or `returned_from` whose phase is `ci` carries its rule id and result word,
  and no `ci_*` input;
- no seat ever receives `ci_observation` or `ci_green`. The review seat receives nothing of `ci`
  on any road.

**What `classify` sees, exactly.** On landing's one road to a seat, `classify`'s
`last_decision` names the rule `CI-PASS` and the result word `pass`, and nothing else of `ci`:
the observation and the `ci_green` flag are omitted from it, and `results` holds no `ci`. That
is the fact `classify` was entered by `CI-PASS`, which landing's own entry already implies, and
a park's evidence is for a person, through `inspect` (ruling 9). **Proved by 0111d:**
`classify_after_ci_pass_receives_last_decision_naming_ci_pass_and_pass_and_nothing_else_of_ci`
(the engine's own input builder, `crates/brokkr-runtime/src/engine/input.rs:110-168`, over a
journal holding the `ci` decision: `last_decision` is present, its rule id `CI-PASS` and its
result `pass`, each asserted by value; it holds no `ci_*` key, and `results` holds no `ci`; so
dropping `last_decision`, or a wrong rule or word, fails it as an extra key does). **(b)** projecting it
unchanged: a model seat would read the observation and a `green` it could take as authority.
Not recommended.

**Where the gate sits: where the head is already published.** `fast`'s implement completes with
a local commit (`recipes/fast/policy.json:69-75`), pushed by a person after review
(`recipes/fast/scripts/ship-seat.sh:73`, `:91`), so a gate before review there has no remote
checks to read. **Recommended: (a) the gate is the `landing` recipe's check-watching step, its
entry phase** ("a land run that watches pull request checks", as the commission quotes the
issue). Landing runs over a branch that already exists (`recipes/landing/README.md:5-8`), and at
its entry no seat has committed; an unpushed head is row `head-unpublished`'s case, and after a
push the operator's retry observes again.

The overlay, `landing-ci`, extends `landing` and overrides its table's `initial` to `ci` and
its `phases` to add `ci`; `landing` overrides the same keys over `brokkr-fast`
(`recipes/landing/bundle.json:7-10`). Its rules precede landing's
(`crates/brokkr-runtime/src/bundle/compose.rs:871-921`), and no rule of landing's or `fast`'s
changes.

**Where `ci` sits in the phase order, re-derived.** The order is not cosmetic. HEAD orders every
transition by phase position (`crates/brokkr-core/src/policy/direction.rs:10-19`), and the entry
check refuses an entry from which a forward rule reaches a seat with no gate, a work seat
(`crates/brokkr-runtime/src/bundle/entry.rs:186-209`). Landing's order is `classify`,
`implement`, `verify`, `review`, `ship`, `done`, `stop` (`recipes/landing/policy.json`). Fix
round 1093r6 placed `ci` after `implement` only because `CI-FAIL`, `ci` to `implement`, was then
a forward path to a work seat if `ci` came first, so `--enter ci` would have refused as
`ForwardWork`. With no road from `ci` to `implement`, that reason is gone. **Recommended: (a)
the overlay's materialized order is `ci`, `classify`, `implement`, `verify`, `review`, `ship`,
`done`, `stop`**: `ci` prepended, landing's own phases in their own order. Then, by HEAD's
entry check as read (`entry.rs:152-171`), and not yet run:

- **the forward-work check.** The only road from `ci` to another phase is `CI-PASS`, forward to
  `classify`, which is a gate (`recipes/landing/bundle.json`, its `classify` seat's
  `"class": "gate"`); every other walk from `ci` is the walk from `classify`, which HEAD admits
  as landing's own `initial` today. So `ForwardWork` admits `ci` exactly as it admits `classify`;
- **the protected check.** Every road from `ci` runs through `classify`, so `ci` reaches no
  terminal around `review` that `classify` does not, and `classify` reaches none
  (`crates/brokkr-runtime/src/bundle/phases.rs:146-157` skips `stop`). Review stays unavoidable
  before `ship`;
- **the visit and change checks** (`entry.rs:239-266`, `:268`) see from `ci` the groups and
  sources they see from `classify`, since `ci` reads no visit and needs no change, and no
  landing phase moved relative to another;
- no landing rule changes direction: the only pairs whose order moved involve `ci` itself, and
  `CI-PASS` is forward.

The rail, which reads the recorded order (`crates/brokkr-view/src/rail.rs:9-12`), paints
`CI-PASS` as a road forward, and `classify` receives no `returned_from`, as on landing's own
default start. **(b)** fix round 1093r6's order, `ci` after `implement`: it still admits every
start, but it paints `CI-PASS` as a road back and hands `classify` a `returned_from` naming `ci`,
for a reason that no longer holds. Not recommended. **(c)** an engine rule exempting `ci`'s edges
from `ForwardWork`: no order needs it, so no engine change is proposed.

**CI is the only way in.** A table's `initial` is not the only entry: an explicit `--enter`
phase is admitted on its own, through the entry check, and `initial` is used only when no entry
is named (`crates/brokkr-runtime/src/bundle/entry.rs:139-150`). The check refuses an absent,
terminal or work phase, and one from which a path reaches a terminal around the protected phase
(`:152-184`); `fast`'s protected phase is `review` (`recipes/fast/bundle.json:6`). Landing
documents `--enter review` and `--enter classify --continues <run>`
(`recipes/landing/README.md:91-99`), and both pass that check. So, as fix round 1093r4 left it,
`landing-ci --enter review` would skip `ci`. Under the frame that is landing as it is today, not
a pass the gate grants; it is still designed out, so every run that names the overlay enters at
`ci`, where every true row of the [park table](#the-park-table) holds it for a person. **Recommended: (a) `landing-ci`'s entry admission admits `ci` alone,
through the existing entry check.** The check's chain of refusals gains one arm, after HEAD's
own first four. HEAD's `refusal` returns, in order, `Absent` for a phase the table lacks,
`Terminal`, `Unprintable` for a name outside the entry grammar, and `Work` for a seat with
work (`crates/brokkr-runtime/src/bundle/entry.rs:152-171`); those outcomes are kept, unchanged,
under the overlay. Then, in a bundle whose table declares a `ci` seat, every phase that passed
them but the `ci` phase refuses as a new `EntryRefusal` variant, `BeforeCi`, naming the `ci`
phase; so `BeforeCi` is the refusal of an otherwise eligible later gate, such as `classify` or
`review`, and `admitted()` lists `ci` alone. The arm sits before the protected arm, so `ci`
itself still meets every existing check: protected, forward work, strategy, change and visits,
which the materialized order above admits. Every start the overlay admits enters at `ci`:

- **the default start** uses `initial`, `ci`, as HEAD's `first_phase` does (`entry.rs:139-150`);
- **`--enter ci`** passes every existing arm, as above, and `BeforeCi` does not judge `ci`
  itself;
- **continuation**, `--enter ci --continues <run>`, is the same admission over a continued run,
  after HEAD's lineage checks: the launch admits the entry, then requires each continued run to
  exist, carry no dispatch and be concluded (`crates/brokkr-runtime/src/launch.rs:301-306`,
  `:453-489`). Landing's documented `--enter classify --continues <run>` refuses as `BeforeCi`
  under the overlay, and the person continues at `ci` instead;
- **`--enter review`, `--enter classify`**, `--enter verify` and every other gate that passes
  HEAD's four first arms refuse as `BeforeCi`; `--enter implement` keeps HEAD's `Work`, and
  `--enter done` its `Terminal`.

The protected-phase arm is unchanged, so `review` stays unavoidable before `ship`. Refused alternatives: **(b)**
a table rule alone, which the explicit entry never reads; **(c)** removing `--enter` from the
CLI for every recipe, which takes landing's documented continuation away from recipes that have
no `ci` phase.

**The entry set holds only checks that can be green before landing.** HEAD's required
`delivered by brokkr` (`.github/branch-protection.json:8`) fails until landing publishes its
shipped evidence: with no `by-hand` label, a body with no `Brokkr-Run:` line fails, and a named
run must have ruled `shipped` (`scripts/delivered-by-brokkr.sh:189-204`), which only this landing's
ship produces (decision 0051 ruling 2). At entry it fails on every first landing without the
label, and as an entry it would make `not-success` true on every such landing. **Recommended: (a) the entry set excludes
every required check whose evidence is this landing's own output.** The recipe data says which:
a `required_checks` entry may carry `"after": "ship"`, a closed one-word field. **An after-ship
check is in the observed inventory and excluded from the judgment**, stated here once: the
observer lists it with every other check, unfiltered by name, and the sealed observation holds it; no
condition of `green` reads it in the current execution; and the forge's required-set comparison
counts it as declared. In any other run of the workflow on the head it is read as every
declared name is (`other-run-not-success`).
The rule for the recipe data: an entry whose producer reads anything this run's ship seat writes
(its anchor, its journal, its ledger) carries `"after": "ship"`. For the house that is
`delivered by brokkr`, and only it. The forge's requirement stays at merge, unchanged. The
declaration is the operator's: the compile cannot see what a check reads, and an entry left
undeclared costs a person's look on row `not-success`, never a pass. Refused alternative:
**(b)** observing it as any other entry, which parks every first landing on `not-success`.

**The entry set is derived once, and every reader but the inventory reads it.** One function
over the pinned list gives the entry set: the entries with no `"after"`. Every condition of
ruling 3's `green` rule reads that set and nothing wider. The full declared list is read only to
reconcile the inventory, as the after-ship invariant above states once. A red
`delivered by brokkr` beside every eligible entry green is therefore `green`.

**The entry set is never empty.** A declared list whose every entry carries `"after": "ship"`
filters to nothing, and an observation with no entry to match would draw `green` on whatever
unrelated checks it saw. So the compile requires at least one entry-eligible check, one entry
without `"after"`, and refuses the list otherwise (ruling 5).

**Every route, from the incoming edge.** First match wins, in the order listed. The `When`
column names the judgment; ruling 10 gives its schema-legal spelling.

| Rule | From | Result | When | Next |
|---|---|---|---|---|
| (entry) | | | `initial: ci`; an explicit entry admits `ci` alone | `ci` |
| `CI-PASS` | `ci` | `pass` | `green` | `classify` |
| `CI-PASS-UNBOUND` | `ci` | `pass` | | park, `ruled`, on row `word-unbound` |
| `CI-PARK` | `ci` | `fail` | | park, `ruled`, naming the observation's digest and every true row of the [park table](#the-park-table) (ruling 7) |

`CI-PARK` is the one park rule for an observation: every true row of the
[park table](#the-park-table) but `word-unbound` reaches it, `launch-untrusted` among them,
whose attempt ends as the outcome `launch-refused`, reported as `effect/succeeded` with the
driver's `fail`, which decide raises only on `green` (ruling 2). It needs no `when`.

**The table is total.** HEAD's compile refuses a table with an unruled valuation:
`seated_and_ruled` calls `refuse_unruled` (`crates/brokkr-runtime/src/bundle/phases.rs:118-130`),
which sweeps every valuation of each `(from, result)` group's axes through the real evaluator
and refuses the first no rule rules (`crates/brokkr-core/src/policy/audit.rs:187-197`,
`:358-381`). A flag axis is both booleans (`:441-443`), whoever owns the flag. So with
`CI-PASS` the sole `(ci, pass)` row, the valuation `ci_green = false` is unruled and the
composed overlay would not compile. So **`CI-PASS-UNBOUND`**, recommended as (a), is the
unconditional `(ci, pass)` rule after `CI-PASS`, parking `ruled` on row `word-unbound`: never
reached while decide raises the word only on `green`, and a park, never a route. `CI-PARK` makes
`(ci, fail)` total. **(b) Proving the valuation impossible** would need an engine change for one
row, since the audit sweeps each flag on its own. Not taken.

Then landing's own two rules, `CLASSIFY-DOCS` to `review` and `CLASSIFY-CODE` to `verify`
(`recipes/landing/policy.json:9-21`), and every `fast` rule, unchanged.

**No loop runs through `ci`.** `ci` is entered only as the run's `initial`, and again only by
the operator's retry of its own park; the compile refuses any rule whose `next` is `ci`. Nothing
leaves `ci` but `CI-PASS` and the two parks on the [park table](#the-park-table)'s rows, so the
gate buys no seat attempt beyond its own
exec attempt, and `fast`'s implement bound, `visits_implement_gte: 3`
(`recipes/fast/policy.json:76-94`), counts only the visits landing's own rules buy, as today.

**What this does not gate.** A head the run itself makes, by an implement after a return, is
local: verified locally, judged and shipped for a person to push, and its CI runs as today.
Gating it needs a publish-and-resume handoff, a follow-up on which fix round 1093r3's pre-review
interception waits.

**Refused routes**, each because it would hide a failure, skip the gate, read an unpublished
head or write to the forge:

- any route to a re-run (ruling 1);
- any rule from `ci` whose `next` is not `classify`, `implement` among them: the implement
  route waits on the deferred classifier
  ([Deferred to a later decision](#deferred-to-a-later-decision));
- any rule whose `next` is `ci`: it would gate a head the run made, which is unpublished;
- any rule into `classify` but `CI-PASS`, and any explicit entry but `ci`: either would skip
  the gate;
- `CI-PASS` off `green`; decide never raises the word to `pass` off `green` (ruling 2's outcomes).

Each is refused by the overlay test, over the complete composed table (ruling 10). **(b)** fix
round 1093r3's pre-review gate in `fast` and `verify` is refused for now: its head is unpublished
by construction, and it returns with the follow-up handoff above.

### 7. The park's cause

**Recommended: (a) 0106's existing `ruled`, naming the gate's rule and carrying the
observation's digest.**

- **(a) `ruled`.** HEAD's park arm already writes a table park as `ruled` with the matched
  rule's id (`crates/brokkr-runtime/src/engine/cursor.rs:369`; the variant is
  `crates/brokkr-core/src/park.rs:40-45`, a `rule_id` and a `detail`). The gate's park is a table
  park, so its cause is `ruled` with the id of the gate's park rule (ruling 6), and its `detail`,
  the park's reason, names the digest ruling 2's outcome table gives the attempt (the SHA-256
  of the stdout bytes the seat record seals, the first 1 MiB of an oversized stdout, or
  `no observation` where no byte exists) and the true rows of the
  [park table](#the-park-table), the first row ids that fit inside the reason's bound of 256 bytes
  (`crates/brokkr-core/src/summary.rs:53`), with a count of the rest. The cause's shape is
  `park-cause/v1`'s, unchanged, so no contract moves and every reader of a cause reads it as it
  reads every rule park. It is what the park is: a person's ruling is owed, and the rule that
  owed it is named. The full list of true rows is `inspect`'s, from the sealed observation
  (ruling 9).
- **(b) `infrastructure` with `origin: ci`, under a `park-cause/v2`.** Not taken. It claims a
  class, a runner fault, that the gate no longer draws: the gate parks a port bind and a red test
  alike on `not-success`, and an unread page on its list's row, such as `jobs-incomplete`, and tells no runner fault
  apart. Typing a park as the runner's waits on
  the deferred classifier ([Deferred to a later decision](#deferred-to-a-later-decision)).

A `ruled` park is never retryable under 0106, and spends nothing of 0106d's budget, which is
spent only on an `infrastructure` park with `retryable` true
(`docs/decisions/0106-a-park-names-its-cause-a-held-wait-and-a-budget.md:250-252`, `:667-683`). So
no reading of this park in that budget needs a ruling of its own.

### 9. The readouts

**Recommended: (a) one derivation in `brokkr-view`, painted by `inspect` and the fleet.**

`inspect` names each observation's judgment, every true row of the
[park table](#the-park-table) while the bundle is held (ruling 3), and the evidence a
person needs to look and re-run: the workflow run's id and attempt and each required job's id
and conclusion, from one view model (0071 ruling 7). The fleet paints the park's cause through
`ParkCause::line` (`crates/brokkr-core/src/park.rs:275-280`), which the TUI and `inspect`
already paint (`crates/brokkr-cli/src/tui/columns.rs:375`,
`crates/brokkr-cli/src/render/inspect.rs:36`; the fleet's row field is
`crates/brokkr-view/src/fleet.rs:339`). It reads `ruled <rule id>: <detail>`, the detail naming
the observation's digest and its first true rows (ruling 7).

### 10. How a table spells the judgment

**Recommended: (a) one engine-owned flag, `ci_green`, legal under phase-machine v2 with no
version.**

v2's `when` admits a boolean, a number or a string, never an array
(`contracts/phase-machine.v2.schema.json:71-77`), though HEAD's runtime reads choice arrays
(`crates/brokkr-core/src/policy/conditions.rs:43-75`). With the judgment `green` or not, one
flag spells it whole; fix rounds 1093r5 to 1093r8's other two flags left with their classes.

- **(a) `ci_green` alone.** An `Owner::Engine` `Flag`, true exactly when the judgment is
  `green`. A boolean input by name is what v2's `when` admits (`:73`). Ruling 6's `CI-PASS`
  reads `ci_green: true`, and nothing else reads it. It is the table's reading of the engine's
  judgment rather than of the result word: decide raises the word from the same judgment
  (ruling 2's outcomes), so `pass` and `ci_green` agree, and otherwise `CI-PASS-UNBOUND` parks
  on row `word-unbound`. No second flag is needed: the park rule is
  unconditional on `fail`, and a flag for "not green" would be the same fact twice (0071 ruling
  5). It mints no contract.
- **(b) No flag: route on the result word alone.** `(ci, pass)` to `classify` and `(ci, fail)`
  to the park, with no `when`. It is one input fewer, but the table would then admit a pass on
  the word alone, and the engine's judgment would guard it only through decide's derivation of
  the word. Not recommended.
- **(c) A table contract of this decision's own, admitting a `<choice>_in` array.** Refused.
  Phase-machine v3 is decision 0109's, which is accepted, and its v3 is proposed in its slice
  0109c (`docs/decisions/0109-a-gate-s-re-visit-reads-what-changed.md:1500-1520`, `:1830`). This
  decision claims no table version of its own. What it needs from v3 is the ask below.

**One result per row.** Each rule's `result` is one string under v2
(`contracts/phase-machine.v2.schema.json:31`, `:50`), and the engine matches it by exact
equality (`crates/brokkr-core/src/policy.rs:264-273`). Against v2, each row 0111 adds is legal
on its own: `CI-PASS` is `pass` with `when` `ci_green: true` (`:73`) and `next: classify`;
`CI-PASS-UNBOUND` and `CI-PARK` are `pass` and `fail` with no `when` and `park: true`, as v2's
park admits (`:53-57`), the first after a conditional rule and the second alone in its group
(`:28`); and `initial: ci` and the `phases` list are a string and a list (`:16-20`).

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

**The 0109 v3 dependency stands for the narrowed gate, and so does its ask.** The narrowing
changes only the gate's own rows, and those were already v2-legal; it never touched the array
`landing-ci` inherits. `landing` extends `brokkr-fast`, which extends `fast`
(`recipes/landing/bundle.json:5`, `recipes/brokkr-fast/bundle.json:4`), so `landing-ci`'s
composed table carries `fast`'s `IMPL-BLOCKED-ENVIRONMENT` and its array. That is true of
`landing` today, before 0111 adds a row; 0111 adds no array and inherits the one there. No
composition that holds the landing gate and keeps the environment park validates under v2.

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
`landing-ci`. The narrowed gate needs nothing else of v3: its own rows use only a boolean flag.
0111 mints nothing for it; the ask is 0109's to rule.

**Enforcement binding:** 0111d validates the complete materialized table `landing-ci` composes,
not the overlay's file, against v3. A test asserts that `ci_green` is true exactly when the
judgment is `green`.

## Deferred to a later decision

The automatic classification of a red observation, and every route it would take, are not this
decision's. On 2026-11-20 the operator narrowed it ("narrow 0111 as recommended"): keep the CI
reading, the trust-root model and the security bounds; park every non-green CI result for a
person; defer the automatic classification and the implement route to a later decision. This
section is the one place that classifier is named.

**What that decision would rule.** Whether, and from which typed facts, a red observation may be
typed as the runner's (a park that names a runner fault, so a person re-runs without looking
further) or as the change's (a return to `implement`, so a model diagnoses it); the evidence
that would type each; its home in recipe data; how such a park reads in 0106d's budget; and the
cause it would write.

**The facts it waits on, which are 0111a's measurements and beyond.** Each of these was an input
the classifier drafted in fix rounds 1093r2 to 1093r8 needed, and each is unmeasured but the one
0111a settled:

- which revision a step ran on, beyond the run's tested commit: whether any forge field records
  a job's checkout;
- the revision of the workflow file GitHub ran for a run, and whether the forge reports it;
- how the forge names a matrix instance: settled by 0111a, by display name alone, with no key;
- GitHub's precedence among workflow, job and step `env:` and `GITHUB_ENV` writes, from a
  recorded run or the runner's source;
- how a job log is cut into steps, and what a step's bounded tail holds.

**What was learned here that it must meet.** Name the refusal-weakening class first: **a fact a
refusal needs is lost or misread, so a case owed a person's ruling falls through to a weaker
outcome, an implement visit.** For three rounds each security hold was that class, and each
round's fix added surface in which the next round found a new case:

- fix round 1093r6's landing review: a stale same-producer `success` satisfied the absence test,
  so a missing required audit beside an ordinary test failure reached `implement`;
- fix round 1093r7: the expected matrix keys were parsed from the checked-out head rather than
  the workflow GitHub ran, so a duplicated instance read as an unexpected one and reached
  `implement`;
- fix round 1093r8: two passages still routed an unestablished tested revision to `change`
  after the refusal-first rule moved it to a refusal; and a duplicated instance on an unread page
  of the job list could not be seen, so the guard owed a person's park could be established
  false over a partial inventory.

A later classifier must have no outcome weaker than a person's park for any case whose facts it
cannot establish, by construction and in one home, before it adds a class beneath it. It must
also meet the precision guards the drafts found it needed: a change kept out of every input of a
step that types the runner's evidence (its working directory, its implicit and
environment-selected inputs, the files it includes, symlink targets and the steps that supply
it), read on the tree the job executed; and the house's own closure, which does not resolve at
HEAD (`scripts/test-shards.sh:558-575` builds its argv at run time, and `scripts/mutants.sh:31`
sets its directory by a command substitution).

**Its follow-up issue.** The controller opens it under #1094; this decision names it as that
issue and designs nothing for it. The issue's own first slice, a classifier over 0111a's
recorded payloads with no route, would let a sweep count, offline, what a typed park would have
saved, so the later decision is ruled on measured cases.

## What stays out

- Any forge write, and an automated re-run: ruling 1, and the follow-up its (c) names.
- No model decides: the gate is an exec seat, and the judgment is a core function.
- No required check is skipped, marked passed or written.
- No refusal is weakened: the gate parks on every true row of the
  [park table](#the-park-table), nothing is weaker, `retryable`'s positive-evidence rule stands
  as 0106 wrote it, and no `infrastructure` cause is written.
- No Windows code (0063).
- #1113's own fix. Its "Not Found" is a failed step of `RustSec dependency audit`
  (`.github/workflows/ci.yml:673-695`), which under the gate parks on `not-success` for a person.
- The dispatcher's retry budget: a `ruled` park spends none of it (ruling 7).
- The automatic classification and its routes, the typed port-bind park among them
  ([Deferred to a later decision](#deferred-to-a-later-decision)); flaky-test handling.
- Follow-ups: `fast`'s `blocked_by_in` under v2 (ruling 10); a gate on a head the run made
  (ruling 6); commit statuses as evidence (ruling 3); a forge other than GitHub.

## Consequences

A red CI no longer costs a model diagnosis attempt at landing's entry: the gate parks on every
true row of the [park table](#the-park-table), with a cause a script can read and a digest a
person can act on. Brokkr sends the forge nothing and types no failure, and the forge's required
checks at merge stay the only CI authority. A recipe with no list behaves exactly as now. The
cost is an exec observer per recipe that wants it, one read-only token, a list an operator keeps,
and a person's look at every red until the deferred classifier is ruled. Until a tested-commit
source outside the change is measured, that look is at every observation, green on the forge
or not: 0111a found none (ruling 3). Two contract versions
are proposed, `ci-observation/v1` and `ci-checks/v1`, each minted only when 0111b lands under
this decision's acceptance (0082) and after 0111a's evidence, and one additive `run/started`
member, `pull_request` (ruling 3). No table contract is proposed: the gate waits on 0109's v3 and
ruling 10's ask.

## The slices

Smallest first; each lands on its own. Evidence comes before any contract, because 0082 forbids
fixing a minted version in place: a shape the evidence would change is changed here, while
proposed. Each assertion is in the slice that builds its writer, and each acceptance test of the
tenth hold in the slice [Proved by the slices](#proved-by-the-slices) names.

1. **0111a — the forge's evidence, before any contract.** The operator captures, by hand, outside
   any run, and with a read-only token, representative payloads for this repository's own pull
   requests: the check runs and the commit statuses for a head, every page of each; the workflow
   runs that produced them, with path, event, number, attempt, head, association and whatever
   the forge says of the revision each tested; the jobs of each current run attempt, every page;
   the open pull requests on the head, every page; and the base branch's required set, every
   page. No job log is captured: no condition of `green` reads one.
   Files: the captured payloads, redacted, as test data under `crates/brokkr-core/src/ci/evidence/`
   with a note of how each was taken; one test that pins their digests, so no later slice
   regenerates them. About 40 test lines and the data. No contract and no production code. It
   settles, or marks still unmeasured:
   - the tested-run identity in both checkout modes: whether a run names the commit it tested,
     as a field distinct from its `head_sha`, for a job that checks out the pull request's head
     by explicit ref (`.github/workflows/ci.yml:49-64`) and for one that checks out the forge's
     merge revision (`:663-683`); this decides whether any `head` or `merge` entry can be
     `green` (ruling 3);
   - what a run's and a check run's `pull_requests` association holds, and in which cases it is
     empty, a fork's pull request among them (ruling 3);
   - whether the forge reports a job's key, and the app's numeric id (ruling 3);
   - which field of each list names the run and attempt an item belongs to, and which attempt
     a job a failed-jobs re-run did not re-run reports (the currentness rule);
   - how each list pages, and how the observer tells the last page (ruling 3's bounded reads);
   - the largest page each list returned, against the 8 MiB response bound, and whether
     `gh api` streams a response body or holds it whole before writing it (ruling 3);
   - the closed list of conclusion and status words.

   Where a fact differs from what this decision assumes, the decision is revised before it is
   accepted, or before 0111b mints. Test: `the_ci_evidence_files_are_pinned_by_digest`.
2. **0111b — the observer and its trusted launch.** Files: a new `crates/brokkr-core/src/ci.rs`
   with `ci/observation.rs` and `ci/checks.rs` (the two contracts' types, parsed once with
   unknown fields refused; `ci/observation.rs` holds the closed `CiAttempt` outcome around the
   observation, ruling 2) and `ci/tests.rs`; `crates/brokkr-core/src/lib.rs`;
   `crates/brokkr-core/src/policy/inputs.rs` (the seat-owned `ci_observation` record);
   `crates/brokkr-runtime/src/bundle.rs` (load and pin `ci-checks.json`, refuse it on a non-exec
   seat, the pinned interpreter and tools, the allow-list, and the observer credential's
   `access: read` declaration); the launch in `crates/brokkr-runtime/src/engine.rs` (a `ci` site's
   cleared `SpawnEnv::Exactly` table, its own working directory, created before the spawn and
   removed after, its digest-checked executables, the script fed over the engine's pipe, and the
   observation captured from stdout, bounded) and `crates/brokkr-protocol/src/adapters/exec.rs`
   (a working directory the site names, and the raw stdout channel);
   `crates/brokkr-protocol/src/process.rs` (execution of an opened descriptor on Linux, and the
   root-owned chain check on both systems); run start's refusal of a `ci` seat whose bundle layer
   is inside the worktree, over 1091a's descriptor-held walk, so 0111b lands after 1091a; the
   launch-bound pull request (ruling 3): `crates/brokkr-cli/src/cli_args.rs` (`--pull-request`),
   `crates/brokkr-runtime/src/launch.rs` (carried into the start, and refused on a bundle with no
   `ci` seat) and `crates/brokkr-runtime/src/engine/start.rs` (the `run/started` member, and its
   reading at `ci`, which hands the number to the observer's argv); and a
   house observer script, with its polling bound inside the seat's deadline, every list read to
   its last page or sealed with its overflow witness, and its shellcheck clean. Its compile
   refuses every live `ci` declaration until 0111d (ruling 2):
   `a_bundle_declaring_a_ci_seat_refuses_to_compile_until_the_first_live_producer_lands`, a
   custom exec recipe's declaration included. About 450 production, 150 script and 700 test
   lines; it may split at triage into the contracts and the launch. Contracts, minted here and
   only after 0111a: `ci-observation/v1` and `ci-checks/v1`. `Acceptance rows:` 4
   ([Proved by the slices](#proved-by-the-slices)). Tests, the launch driven through a
   test-only bundle the live compile does not admit:
   - the contracts: `an_observation_with_an_unknown_field_refuses_whole`;
     `an_unknown_conclusion_word_is_observation_unreadable`;
     `a_ci_checks_file_with_change_classes_setup_steps_or_patterns_refuses_to_compile`;
     `a_status_entry_in_the_required_list_refuses_to_compile`;
     `an_empty_required_list_refuses_to_compile`;
     `a_ci_list_on_a_model_seat_refuses_to_compile`;
     `a_seat_declaring_ci_green_fails_the_compile`;
     `a_seat_payload_carrying_ci_green_is_stripped_before_the_table`;
   - the bounded reads: `a_list_past_its_bound_seals_an_overflow_witness_never_a_truncated_list`;
     `each_paged_list_carries_whether_it_was_read_to_its_last_page`;
     `the_observer_reads_every_page_of_each_runs_job_list`;
     `the_observer_reads_check_runs_and_jobs_with_filter_all` (a stub `gh` by absolute path
     records each call's query, and fails the test on any check-run or job call without
     `filter=all`);
     `a_response_of_exactly_the_byte_bound_is_parsed_and_one_byte_more_is_a_response_witness`;
     `the_selected_transport_takes_in_no_more_than_the_bound_plus_one_byte_of_a_streamed_page`;
     `no_job_log_is_read_or_sealed`;
   - the launch:
     `one_admitted_launch_resolves_the_sealed_binding_into_the_cleared_environment_captures_stdout_and_parses_the_result`
     (the real credential arrangement: a sealed binding in the operator-side store, resolved by
     the runner into `GH_TOKEN`, the one allow-listed token variable of a cleared
     `SpawnEnv::Exactly`, no other key present, and `GH_CONFIG_DIR` an empty directory; a stub
     `gh` by absolute path asserts it read the token from `GH_TOKEN`, nothing from argv and no
     stored login from its configuration directory; its stdout is captured over the engine's
     pipe and parsed into `ci-observation/v1`; run once under `harness` and once beside a boxed
     seat under `namespace`, both admitted);
     `an_observer_credential_declared_write_scoped_refuses_to_compile` (`"access"` of any word
     but `read`, and a second secret binding on the `ci` seat, each refused by name);
     `an_observer_credential_bound_under_any_name_but_gh_token_refuses_to_compile`;
     `on_macos_the_interpreter_runs_by_path_only_under_a_root_owned_chain`;
   - the boundary (Linux, under a real box, as 0043's boundary tests are):
     `inside_the_box_opening_the_observers_pipe_endpoint_through_proc_fails` (a boxed seat walks
     its `/proc` for the observer's and the engine's pids and their `fd` entries; its control,
     the same walk from an unboxed same-UID sibling, opens the endpoint, which the trust root
     accepts);
     `a_configuration_directory_replaced_by_a_boxed_seat_before_gh_opens_it_refuses` (a boxed
     seat whose binds reach the directory's parent swaps it by name between the walk and the
     spawn: the held descriptor is the one `gh` reads, or the launch refuses);
     `the_observers_working_directory_is_set_from_the_held_descriptor`;
   - the verified bytes: `an_interpreter_replaced_after_verification_never_runs` (Linux: the
     hashed descriptor runs; both systems: a path whose chain the run's account can write
     refuses before the spawn);
     `a_tool_whose_directory_the_runs_account_can_write_refuses_before_the_spawn`;
     `a_pinned_path_that_is_a_link_into_the_worktree_refuses_before_the_spawn`;
     `a_pinned_path_whose_ancestor_is_a_link_refuses_before_the_spawn`;
     `the_script_reaches_the_interpreter_over_the_engines_pipe_as_the_bytes_the_manifest_digests`;
     `an_observer_interpreter_whose_digest_moved_refuses_before_the_spawn`;
     `an_observer_tool_under_the_worktree_refuses_before_the_spawn`;
     `a_bare_interpreter_name_refuses_to_compile`;
     `a_ci_seat_whose_observer_bound_leaves_no_reporting_margin_refuses_to_compile`;
   - the result channel and environment:
     `a_replaced_result_file_under_forge_results_is_never_read_and_the_observation_comes_from_stdout`;
     `the_driver_stops_reading_stdout_at_the_bound_plus_one_byte`;
     `every_ci_attempt_outcome_is_reported_as_effect_succeeded_never_failed` (each row of ruling
     2's outcome table, through the driver: the `CiAttempt` it reports, and no
     `ResultStatus::Failed` on any);
     `a_non_utf8_observation_refuses_and_is_never_decoded_lossily`;
     `the_ci_site_journals_no_stderr_bytes`;
     `a_ci_seat_whose_layer_is_inside_the_worktree_refuses_at_start`;
     `an_inherited_bash_env_never_reaches_the_observer`;
     `the_observer_runs_from_a_directory_outside_the_worktree`;
     `the_launch_directory_is_removed_after_the_observer_ends`;
     `an_environment_key_outside_the_allow_list_refuses_to_compile`;
     `the_observer_reads_no_gh_configuration_from_the_worktree_or_home`;
     `the_observer_names_the_repository_and_never_reads_the_worktrees_remotes`;
     `the_observer_lists_every_check_for_the_head_unfiltered` (no name filter; a stub `gh`
     given by absolute path, its output held to `ci-observation/v1` and to 0111a's evidence);
     `the_observer_holds_no_write_binding`;
   - the launch-bound pull request, through `launch.rs`'s start over a test-only bundle:
     `the_launch_bound_pull_request_is_journaled_in_run_started_before_the_first_effect`;
     `the_observer_receives_the_journaled_pull_request_in_its_argv`;
     `a_pull_request_flag_on_a_bundle_with_no_ci_seat_refuses_before_the_journal_opens`;
     `a_continued_run_imports_no_pull_request_from_its_predecessor`;
   - the seal: `a_recorded_observation_over_stdout_is_sealed_as_the_seat_record`.
3. **0111c — the judgment, the park rule and its readout.** Files: `crates/brokkr-core/src/ci/judge.rs`
   (`judge(observation, pinned)`, total, matching every conclusion and status word without a
   wildcard, 0071 ruling 2); `crates/brokkr-core/src/policy/inputs.rs` (the engine-owned
   `ci_green` flag); `crates/brokkr-runtime/src/engine/decide.rs` (judge at decide over the
   `CiAttempt` outcome, raise the word, and write `ci_green`); the park's reason for a `ci` park, naming the gate's rule, the
   observation's digest and its true rows inside 256 bytes, in the engine's park path
   (`crates/brokkr-runtime/src/engine/cursor.rs:369` keeps writing `ruled`);
   `crates/brokkr-runtime/src/bundle/` (the refusals of ruling 6's routes); and the readout it
   produces, `crates/brokkr-view/src/` (the CI view model: the judgment, every true row, and the
   run, attempt and job ids a person needs) with `crates/brokkr-cli/src/render/inspect.rs`. About
   300 production and 700 test lines, over test-built tables and observations built by a shared
   builder from 0111a's evidence; no recipe yet. No contract change. **Every judgment test
   asserts the exact ordered list of true rows** the [park table](#the-park-table)'s order and
   its dependency rule give its fixture, its own row and every row that rule makes true with it,
   and nothing else; so deleting any row from the expected list, or adding one, fails it. **Each
   row's test binds its predicate in production:** a compiling mutation of `ci/judge.rs` that
   makes that row's predicate always false leaves the row out of the fixture's list and fails
   that row's `Bound by` test, and 0111c records each such mutation with the test it failed
   (0071 ruling 9); `scripts/mutants.sh` over `ci/judge.rs` leaves no surviving mutant of a row
   predicate. **The tested commit in the lists below:** a builder fixture carries a tested-commit
   record, which no live observer writes (ruling 3); a case run live through 0111b's observer
   seals it `unread`, so its exact list also holds `revision-unproven` for every entry with a
   match, in the table's place, and so does its replay. `Acceptance rows:` 35
   ([Proved by the slices](#proved-by-the-slices)). Tests:
   - 0111a's cases: `every_0111a_evidence_case_parks_with_its_recorded_rows` (each of the eight
     directories under `crates/brokkr-core/src/ci/evidence/`, its pages served by a stub `gh`, the
     binding its own pull request and head: `CI-PARK`, never `CI-PASS`, with the exact list the
     slice records from the payloads, `revision-unproven` for every matched entry in each);
   - the park table ([its one home](#the-park-table)):
     `every_park_statement_in_0111_cites_a_row_of_its_park_table`,
     `the_park_citation_checker_refuses_a_statement_with_no_row_or_an_unknown_row`,
     `every_park_row_is_bound_by_a_test_the_slices_name`,
     `every_test_proved_by_the_slices_is_named_in_its_slices_plan` and
     `the_0111_index_row_names_the_last_round_the_decision_records`, doc tests in a new
     `crates/brokkr-cli/tests/ci_park_table.rs`, a module of `tests/it.rs` beside
     `decisions_index` (`it.rs:48`), reading this decision's text; and
     `the_judges_park_rows_are_the_decisions_park_table_in_order`, which parses the table's `Id`
     column and asserts it equals the judge's closed row enum, less `word-unbound`, in order;
   - `green`: `a_complete_settled_successful_current_observation_is_green`;
     `the_full_two_entry_engine_matrix_with_a_distinct_valid_merge_commit_is_green` (both
     `test (…)` entries, each run's merge commit distinct from the head, its parents the base and
     the head);
     `an_inadmissible_check_that_succeeded_changes_nothing`;
   - one test per row, its fixture making that row true and every fact no dependent row reads
     established, its expected list exact:
     `an_observation_of_another_head_repository_or_pull_request_parks_as_unrelated`;
     `an_unpushed_head_parks_as_head_unpublished`;
     `an_observation_with_no_check_names_empty_then_missing_required_for_every_entry` (a complete,
     empty check list and a non-empty required set: `empty`, then `missing-required` once per
     entry, in `required_checks` order);
     `a_required_audit_missing_from_the_observation_parks_as_missing_required`;
     `two_matches_for_one_required_check_park_as_ambiguous_required`;
     `a_duplicate_job_for_a_required_entry_on_the_last_page_parks_as_ambiguous_required` (the r8
     case: every expected job observed once on the first pages, a second job with the entry's
     name on the last; and its control, the last page unread: `jobs-incomplete`, then
     `ambiguous-required` for each entry, never `green`);
     `two_runs_of_one_key_started_at_one_time_or_unreadable_park_as_execution_ambiguous` (a tie
     at the latest `run_started_at`, and a time that does not read: `missing-required`, then
     `execution-ambiguous`, for each entry);
     `a_required_check_not_completed_with_a_null_conclusion_parks_as_not_settled_alone`
     (each status word but `completed` in the closed list, `in_progress` as 0111a caught it and
     `queued` and `waiting`, which it did not:
     `not-settled` alone, never `not-success`);
     `a_failed_cancelled_timed_out_skipped_or_neutral_required_check_parks_as_not_success`;
     `a_run_with_an_empty_association_parks_as_association_unproven`;
     `a_red_run_on_the_same_head_for_another_pull_request_parks_as_association_unproven`;
     `an_association_naming_a_and_a_closed_b_parks_as_association_ambiguous`;
     `an_open_pull_request_list_without_the_bound_one_or_with_another_parks_as_pulls_disagree`
     (the list empty, as on a closed pull request's head in `cancelled-twin/pulls.open.on-head.page1.json`,
     and the list holding the bound one and a second: `pulls-disagree` alone in each);
     `a_head_entry_whose_run_records_no_tested_commit_parks_as_revision_unproven` (the chief's
     r9 case: the run's `head_sha` and the association's `head.sha` both the gated head, every
     required check a settled success, and no tested-commit record);
     `a_head_entry_whose_tested_commit_differs_from_the_gated_head_parks_as_revision_disagrees`
     (both heads match; the record names another commit);
     `a_merge_entry_whose_run_records_no_tested_commit_parks_as_revision_unproven`;
     `an_association_and_a_merge_commit_that_disagree_park_as_revision_disagrees`;
     `a_merge_revision_whose_parents_are_not_the_base_and_the_head_parks_as_revision_disagrees`;
     `a_sole_red_from_another_producer_names_missing_required_for_its_entry_then_producer_mismatch`;
     `a_status_context_in_the_forges_required_set_parks_as_status_required`;
     `a_forge_required_check_the_list_lacks_parks_as_required_set_disagrees`;
   - the inventories, one test per list the observer reads, each with two cases, the last page
     unread and the list sealed as its overflow witness, over two entries A and B of one key,
     each seen once on what was read as a settled, consistent success, every other list
     complete, and each run twice: live, a stub `gh` serving the pages to 0111b's observer
     through its test-only launch and decide judging its stdout; and on replay, a fold rebuilt
     from that journal re-deriving the same rows. Each expects `CI-PARK` with exactly the list
     its name gives, in the table's order, an entry row per entry in `required_checks` order
     (the dependency rule):
     `the_check_run_list_unread_or_overflowed_parks_as_checks_incomplete_live_and_on_replay`
     (unread: `checks-incomplete`, `ambiguous-required` A, B; overflowed, whose witness holds no
     entry: `checks-incomplete`, `missing-required` A, B);
     `the_status_list_unread_or_overflowed_parks_as_statuses_incomplete_live_and_on_replay`
     (`statuses-incomplete` alone);
     `a_run_list_unread_or_overflowed_parks_as_runs_incomplete_live_and_on_replay`
     (unread, R seen on what was read: `runs-incomplete`, `missing-required` A, B,
     `execution-ambiguous` A, B; overflowed, whose witness holds no run, so no job list is read
     and no check run joins: the same, then `attempt-unnamed` A, B);
     `a_job_list_unread_or_overflowed_parks_as_jobs_incomplete_naming_the_run_and_attempt_live_and_on_replay`
     (unread, A's and B's jobs seen once: `jobs-incomplete`, `ambiguous-required` A, B;
     overflowed, whose witness holds no job, so A's and B's checks join none:
     `jobs-incomplete`, `missing-required` A, B, `attempt-unnamed` A, B);
     `the_open_pull_request_list_unread_or_overflowed_parks_as_pulls_incomplete_live_and_on_replay`
     (the chief's r9 case, only this list incomplete: `pulls-incomplete` alone);
     `a_partly_read_forge_required_set_parks_as_required_set_incomplete_live_and_on_replay`
     (`required-set-incomplete` alone);
     `an_oversized_first_page_through_the_selected_transport_parks_as_response_too_large` (live only:
     the first page of the check-run list is past 8 MiB, served at the seam of whichever
     transport 0111b selects (a stub `gh` to the real observer script and its pinned `head`, or
     a stub forge to the bounded reader) and read by that production reader;
     `checks-incomplete`, `response-too-large`, `missing-required` A, B, and no byte of that page
     parsed; its name stays if the selection changes);
   - the currentness rule, each live and on replay as the inventories are, over two entries A
     (`test (ubuntu-latest)`) and B (the audit) of one key, every list complete, each
     expecting `CI-PARK` with exactly these rows:
     `a_complete_green_attempt_beside_a_queued_newer_attempt_on_the_check_list_parks_live_and_on_replay`
     (the run list names run R at attempt 1, completed, its jobs complete and successful; the
     check list holds those successes and A's check queued, joined to a job of R/2:
     `snapshot-inconsistent` for A);
     `a_newer_attempt_in_progress_on_the_run_list_only_parks_live_and_on_replay` (the run list
     names R/2 in progress, no job of R/2 listed yet, the check list R/1's successes, which are
     history: `missing-required` for A and B, then `execution-unsettled` for A and B);
     `two_lists_naming_different_attempts_park_live_and_on_replay` (the run list names R/1 as
     the latest, completed; the job list, read after a re-run, holds R/1's jobs and R/2's, and
     the check list joins to both: `snapshot-inconsistent` for A and B);
     `a_newer_attempt_that_already_failed_parks_live_and_on_replay` (the run list names R/1
     completed; the check list holds R/1's successes and A's `failure` joined to R/2:
     `snapshot-inconsistent` for A, and no `not-success`, since R/1's is the current match);
     `a_re_run_of_an_older_run_that_fails_after_the_current_succeeded_parks_live_and_on_replay`
     (the adversarial F1 case: run 10/1 cancelled, run 11/1 green, then 10/2 started after 11/1
     with B `failure`: 10/2 is current by `run_started_at`, so `not-success` for B, and no row
     for 11/1, which is completed `success`);
     `a_status_on_the_head_parks_as_status_present_live_and_on_replay` (one status under A's
     name: `status-present` alone);
     `a_check_run_that_joins_no_job_parks_as_attempt_unnamed_live_and_on_replay` (A's check id on
     no job list: `missing-required` for A, then `attempt-unnamed` for A);
     `a_check_run_and_its_job_disagreeing_on_state_park_as_check_job_disagrees_live_and_on_replay`
     (A's check `completed` `success`, its job `in_progress` with the same id and attempt:
     `check-job-disagrees` for A, then `not-settled` for A);
     `a_current_run_still_in_progress_beside_green_required_checks_parks_as_execution_unsettled`
     (`execution-unsettled` for A and B);
     `a_failed_jobs_only_rerun_carrying_a_passed_entry_parks_as_carried_job_live_and_on_replay`
     (its fixture shaped as `failed-jobs-rerun-with-cancelled-twin/run.44837139463.attempt2.jobs.page1.json`);
     the other runs, each over 0111a's payloads, bound to the case's own pull request and head:
     `a_dispatch_run_whose_delivered_by_brokkr_skipped_beside_a_green_run_parks_as_other_run_not_success`
     (`dispatch-and-pr-both-green/`, #1237 open: `other-run-not-success` for each `ci.yml`
     entry, naming run 44926784147; the same over `dispatch-in-progress-beside-pr-green/`) and
     `a_cancelled_twin_beside_a_green_run_parks_as_other_run_not_success` (`cancelled-twin/`:
     `other-run-not-success` naming run 44913617230 among the exact list the slice records from
     the payloads, #1222 being closed); and the control,
     `a_consistent_single_attempt_snapshot_is_green` (judged over a builder observation, every
     item naming R/1, no other run, a tested-commit record: `CI-PASS`);
   - the shared facts and the dependency rule, each its worked case in the
     [park table](#the-park-table):
     `an_unread_engine_head_names_unrelated_and_head_unpublished`;
     `a_short_check_list_beside_an_absent_audit_names_checks_incomplete_and_missing_required`
     (and `ambiguous-required` for each entry seen once);
     `an_entry_seen_once_on_an_incomplete_job_list_names_ambiguous_required`;
     `an_entry_with_no_match_is_false_on_every_match_row_and_true_on_missing_required`;
     `an_entry_with_no_current_execution_names_missing_required_then_execution_ambiguous`;
     `no_observation_row_is_evaluated_on_an_attempt_with_no_observation` (each outcome but
     `observed`: its own row alone, and `result-file-present` where a file exists);
     `the_true_rows_are_recorded_in_the_tables_order_entries_in_required_checks_order`;
   - history and the other runs:
     `a_stale_same_producer_success_beside_an_ordinary_failure_names_missing_required_and_not_success`
     (a required audit with no check in its current run, a `success` for it in an earlier
     attempt of that run, and a second required job with a failed test: `missing-required` for
     the audit, `not-success` for the test, and nothing for the history);
     `an_earlier_red_run_of_the_key_parks_as_other_run_not_success` (an earlier run's red never
     counts as a match, and parks on its own row, naming the run);
   - the launch-bound pull request, each driven through `launch.rs`'s start, 0111b's stub `gh`
     and decide over a test-only bundle (ruling 3):
     `a_ci_run_started_without_a_pull_request_parks_as_pull_request_unbound_and_spawns_nothing`
     (no `--pull-request`: the run starts, `CI-PARK` names `pull-request-unbound`, and the stub
     `gh` records no call);
     `a_run_launched_for_one_pull_request_parks_when_the_observation_names_another`;
     `a_run_launched_for_pull_request_a_parks_when_the_only_current_run_is_associated_with_b`
     (the chief's r9 case: two open pull requests on one head, and the current run uniquely
     associated with B, which a comparison against the observation's own number would admit:
     `pulls-disagree`, then `association-unproven` for each entry);
   - every attempt's outcome (ruling 2's outcome table), each driven through dispatch and the
     fold over a test-only bundle, each asserting one `effect/started`, one `effect/succeeded`,
     one `ci` decision, one `ruled` park at `CI-PARK` naming its row (`launch-untrusted` to
     `result-file-present`) and the digest the outcome
     table names, no `effect/failed`, no second attempt and no attempt-cap park:
     `a_launch_refusal_parks_once_as_launch_untrusted_and_spawns_nothing` (the stub interpreter
     records no exec);
     `a_ci_driver_that_fails_to_spawn_parks_once_as_launch_untrusted_with_no_retry`;
     `an_observer_past_its_own_bound_is_killed_by_the_driver_and_parks_once_as_observer_deadline`;
     `an_observer_that_exits_non_zero_parks_once_as_observer_exited_with_no_second_attempt`;
     `an_observer_with_no_stdout_parks_once_as_observation_absent`;
     `an_observer_over_the_stdout_bound_parks_once_as_observation_too_large_naming_the_witness_digest`
     (the digest is of the first 1 MiB);
     `an_unreadable_observation_parks_once_as_observation_unreadable_naming_its_digest`;
     `a_result_file_for_the_ci_attempt_is_never_read_and_parks_as_result_file_present`;
     `the_park_digest_is_the_sha256_of_the_bytes_the_seat_record_seals`; and the two ends that
     are HEAD's indeterminate, never `CI-PARK`:
     `a_seat_watchdog_kill_of_the_ci_driver_is_indeterminate_never_ci_park`;
     `a_head_moved_during_ci_is_indeterminate_never_ci_park`;
   - the word and the table: `a_ci_attempt_record_passes_heads_fence_and_decides_from_the_judgment`;
     `decide_raises_the_drivers_fail_to_pass_exactly_on_green`;
     `a_ci_attempt_carries_no_result_word_of_the_observers`; `ci_green_is_true_exactly_on_green`;
     `every_non_green_observation_parks_at_ci_park_with_no_seat_attempt_after_the_gates`
     (the gate's one exec attempt is journaled, and no `implement` or other model seat attempt
     follows it);
     `a_ci_park_is_ruled_naming_ci_park_with_the_observations_digest_in_its_reason`;
     `a_ci_park_reason_longer_than_its_bound_names_the_first_words_and_a_count`;
     `a_rule_whose_next_is_ci_refuses_to_compile`;
     `a_rule_from_ci_whose_next_is_not_classify_refuses_to_compile`;
     `a_rule_into_classify_but_ci_pass_refuses_to_compile`;
     `no_rule_routes_to_a_rerun`; `a_ci_overlay_seat_declaring_a_write_binding_refuses_to_compile`;
   - the replay: `the_live_and_the_replayed_judgment_take_one_function_over_one_observation_and_one_list`;
     `a_replay_whose_bundle_digest_is_not_the_manifests_refuses`;
     `a_recorded_observation_is_sealed_judged_replayed_and_inspected` (decide is handed a
     recorded observation through a test-built bundle; the sealed seat record holds it, the
     decision's inputs hold `ci_green`, a fold rebuilt from the journal re-derives the same
     judgment, and `inspect` paints it);
   - the readout: `inspect_names_the_judgment_and_every_true_row`;
     `inspect_on_a_truncated_ci_park_after_its_bundle_is_gone_paints_the_named_rows_and_the_count`;
     `inspect_names_the_run_attempt_and_job_ids_a_person_re_runs`;
     `the_fleet_paints_the_ci_park_as_ruled_ci_park_with_its_digest`.
4. **0111d — the gate on `landing`, over a published head.** The first slice with a live
   producer. Lands only after 0109c moves `fast` to v3, and only if the operator grants ruling
   10's ask (or rules its option (b)). Files: `recipes/landing-ci/`, the overlay declaring ruling
   6's order and table and the house's `ci-checks.json` with `delivered by brokkr` as
   `"after": "ship"`; `crates/brokkr-runtime/src/bundle/entry.rs` (the `BeforeCi` arm);
   `crates/brokkr-runtime/src/bundle.rs` (the lift of 0111b's live-admission refusal, ruling 2:
   `a_bundle_declaring_a_ci_seat_compiles_once_the_launcher_the_judgment_and_the_table_exist`);
   `crates/brokkr-runtime/src/engine/input.rs` (ruling 6's projection of the `ci` result); and its
   compile tests. About 100 data, 80 production and 450 test lines. No contract of this
   decision's own. **Live read-only telemetry starts here**: from the first landing the operator
   drives under this overlay on a real pull request, each observation, judgment and park is
   journaled, and a sweep can count parked runs against the diagnosis seat attempts they saved.
   Until a tested-commit source is measured (ruling 3), every live observation parks on
   `revision-unproven`, so the telemetry counts parks only; this slice's `CI-PASS` tests feed
   decide a stub observer's stdout that carries a tested-commit record. The
   park's lifecycle is this slice's too: a person looks and re-runs, then the operator retries
   the parked `ci` phase, which observes again. `Acceptance rows:` 3
   ([Proved by the slices](#proved-by-the-slices)). Tests:
   - the frame's two ([The gate grants nothing](#the-gate-grants-nothing)):
     `after_ci_pass_the_composed_table_is_landings_own_rule_for_rule` and
     `nothing_in_the_recipes_or_seat_scripts_reads_the_ci_phases_result`;
   - the consumer input (ruling 6's projection), each built by the engine's own input builder
     over a journal holding the `ci` decision and asserting the exact keys:
     `after_ci_pass_no_later_seat_input_holds_the_ci_observation_or_judgment` (`classify`,
     `verify`, `review` and `ship`: no `results.ci`, and no `ci_*` key anywhere in the input);
     `the_review_seat_input_holds_no_ci_key_on_any_road` (after `CLASSIFY-DOCS`, and after
     `VERIFY-PASS`);
     `classify_after_ci_pass_receives_last_decision_naming_ci_pass_and_pass_and_nothing_else_of_ci`;
   - the entry (ruling 6), each over the overlay's materialized bundle, its composed table and
     phase order, through HEAD's `first_phase` and `admitted()`:
     `the_landing_ci_default_start_enters_ci`;
     `landing_ci_admits_ci_alone_as_an_explicit_entry` (`--enter ci` passes every existing arm,
     `ForwardWork` among them, and `admitted()` is exactly `["ci"]`);
     `landing_ci_enter_ci_continuing_a_concluded_landing_run_is_admitted_through_launch` (through
     `launch.rs`'s start, over a store holding a concluded, eligible predecessor run with no
     dispatch: the lineage checks pass and the run enters `ci`);
     `landing_ci_enter_review_refuses_as_before_ci`;
     `landing_ci_enter_classify_refuses_as_before_ci`, with and without `--continues`;
     `landing_ci_enter_verify_refuses_as_before_ci` (every otherwise eligible later gate);
     `landing_ci_keeps_heads_absent_terminal_unprintable_and_work_refusals` (`--enter` of a
     phase the table lacks, `done`, a name outside the grammar, and `implement`: `Absent`,
     `Terminal`, `Unprintable` and `Work`, never `BeforeCi`);
     `the_landing_ci_order_is_ci_then_landings_own`;
     `ci_after_implement_admits_every_start_but_paints_ci_pass_backward` (the control: fix round
     1093r6's order, option (b));
     `review_stays_unavoidable_before_ship_from_ci`;
   - the entry set: `a_required_list_whose_every_entry_is_after_ship_refuses_to_compile`;
     `a_fresh_published_pull_request_with_no_anchor_and_no_label_is_green_not_parked_by_the_contribution_check`
     (`delivered by brokkr` red, every other entry green);
     `an_after_ship_check_is_in_the_observed_inventory_and_excluded_from_the_judgment`;
     `an_undeclared_forge_required_context_beside_an_after_ship_entry_parks_as_required_set_disagrees`;
   - the roads: `a_green_observation_passes_to_classify`;
     `a_red_observation_parks_for_a_person_and_never_reaches_implement`;
     `a_pushed_head_after_a_head_unpublished_park_observes_again_on_retry`;
     `an_operator_retry_after_a_person_re_runs_all_jobs_observes_again_and_passes`;
     `a_head_the_run_made_is_never_gated_by_ci`;
     `the_environment_park_survives_the_ci_overlay`;
   - the table: `the_composed_landing_ci_table_validates_whole_against_phase_machine_v3`;
     `the_whole_composed_landing_ci_overlay_compiles_against_heads_totality_gate` (the
     materialized bundle through `seated_and_ruled` and `refuse_unruled`: it compiles; and its
     control, the same overlay less `CI-PASS-UNBOUND`, refuses with `Refusal::Totality` naming
     `ci`, `pass` and `ci_green = false`);
     `a_pass_without_green_through_the_real_policy_and_park_writer_parks_at_ci_pass_unbound`
     (`(ci, pass)` with `ci_green` false, through the composed table's evaluator and the
     engine's park writer: one `ruled` park at `CI-PASS-UNBOUND`, its reason naming
     `word-unbound`, and no `phase/entered` after it; deleting `word-unbound` from the reason
     fails it, which the totality control alone would not).

## Proved by the slices

The operator ruled on 2026-11-21 that each build-proof finding of the tenth hold (landing
`landing-over-an-existing-branch--7833a069`) is an acceptance test of the slice that builds its
piece, which that slice's own review proves; one spanning two slices is owned by the slice that
builds the reader it binds. The eleventh hold's findings are held the same way (round
1093r12). Each rule is stated where it lives, and each test is named in its slice's plan, whose
`Acceptance rows:` line counts them, as `every_test_proved_by_the_slices_is_named_in_its_slices_plan` holds.

| Slice | Test | Answers | Expects |
|---|---|---|---|
| 0111b | `a_ci_seat_whose_observer_bound_leaves_no_reporting_margin_refuses_to_compile` | who settles a watchdog kill | a compile refusal |
| 0111b | `a_response_of_exactly_the_byte_bound_is_parsed_and_one_byte_more_is_a_response_witness` | the transport's byte bound | parsed; a response witness |
| 0111b | `the_selected_transport_takes_in_no_more_than_the_bound_plus_one_byte_of_a_streamed_page` | the transport's byte bound | bounded intake before any parse |
| 0111b | `the_observer_reads_check_runs_and_jobs_with_filter_all` | the default filter (r11 F4) | every check-run and job call carries `filter=all` |
| 0111c | `a_complete_green_attempt_beside_a_queued_newer_attempt_on_the_check_list_parks_live_and_on_replay` | mixed attempts (the HIGH) | `CI-PARK`: `snapshot-inconsistent` |
| 0111c | `a_newer_attempt_in_progress_on_the_run_list_only_parks_live_and_on_replay` | mixed attempts | `CI-PARK`: `missing-required`, `execution-unsettled` |
| 0111c | `two_lists_naming_different_attempts_park_live_and_on_replay` | mixed attempts | `CI-PARK`: `snapshot-inconsistent` |
| 0111c | `a_newer_attempt_that_already_failed_parks_live_and_on_replay` | mixed attempts | `CI-PARK`: `snapshot-inconsistent` |
| 0111c | `a_re_run_of_an_older_run_that_fails_after_the_current_succeeded_parks_live_and_on_replay` | ordering by start (r11 F1, HIGH) | `CI-PARK`: `not-success` |
| 0111c | `a_status_on_the_head_parks_as_status_present_live_and_on_replay` | attempt-less lists (r11 HIGH) | `CI-PARK`: `status-present` |
| 0111c | `an_open_pull_request_list_without_the_bound_one_or_with_another_parks_as_pulls_disagree` | attempt-less lists (r11 HIGH) | `CI-PARK`: `pulls-disagree` |
| 0111c | `a_check_run_that_joins_no_job_parks_as_attempt_unnamed_live_and_on_replay` | the join | `CI-PARK`: `missing-required`, `attempt-unnamed` |
| 0111c | `a_check_run_and_its_job_disagreeing_on_state_park_as_check_job_disagrees_live_and_on_replay` | equal ids, conflicting states (r11 HIGH) | `CI-PARK`: `check-job-disagrees`, `not-settled` |
| 0111c | `a_current_run_still_in_progress_beside_green_required_checks_parks_as_execution_unsettled` | equal ids, conflicting states (r11 HIGH) | `CI-PARK`: `execution-unsettled` |
| 0111c | `a_dispatch_run_whose_delivered_by_brokkr_skipped_beside_a_green_run_parks_as_other_run_not_success` | another run under the names (r11 F3) | `CI-PARK`: `other-run-not-success` |
| 0111c | `a_cancelled_twin_beside_a_green_run_parks_as_other_run_not_success` | another run under the names | `CI-PARK`: `other-run-not-success` among the recorded list |
| 0111c | `a_stale_same_producer_success_beside_an_ordinary_failure_names_missing_required_and_not_success` | run versus attempt (r11 F2) | `CI-PARK`: `missing-required`, `not-success` |
| 0111c | `a_consistent_single_attempt_snapshot_is_green` | mixed attempts, the control | `CI-PASS` |
| 0111c | `every_0111a_evidence_case_parks_with_its_recorded_rows` | the measured shapes (round 1093r12) | `CI-PARK`, `revision-unproven` in each |
| 0111c | `a_failed_jobs_only_rerun_carrying_a_passed_entry_parks_as_carried_job_live_and_on_replay` | the person's re-run (LOW, corrected) | `CI-PARK`: `carried-job` |
| 0111c | `a_ci_attempt_record_passes_heads_fence_and_decides_from_the_judgment` | a wordless `CiAttempt` (text corrected) | `CI-PASS`; `CI-PARK`: `launch-untrusted` |
| 0111c | `a_ci_driver_that_fails_to_spawn_parks_once_as_launch_untrusted_with_no_retry` | who settles a spawn failure (text corrected) | `CI-PARK`: `launch-untrusted` |
| 0111c | `an_observer_past_its_own_bound_is_killed_by_the_driver_and_parks_once_as_observer_deadline` | who settles a deadline (text corrected) | `CI-PARK`: `observer-deadline` |
| 0111c | `a_seat_watchdog_kill_of_the_ci_driver_is_indeterminate_never_ci_park` | who settles a watchdog kill | HEAD's indeterminate, never `CI-PASS` |
| 0111c | `a_head_moved_during_ci_is_indeterminate_never_ci_park` | who settles a moved head | HEAD's indeterminate, never `CI-PASS` |
| 0111c | `the_check_run_list_unread_or_overflowed_parks_as_checks_incomplete_live_and_on_replay` | ordered reasons on an incomplete list (text corrected) | `CI-PARK`: `checks-incomplete`, then `ambiguous-required` or `missing-required` |
| 0111c | `the_status_list_unread_or_overflowed_parks_as_statuses_incomplete_live_and_on_replay` | ordered reasons | `CI-PARK`: `statuses-incomplete` |
| 0111c | `a_run_list_unread_or_overflowed_parks_as_runs_incomplete_live_and_on_replay` | ordered reasons (r12: overflow split) | `CI-PARK`: `runs-incomplete`, `missing-required`, `execution-ambiguous`; overflowed, then `attempt-unnamed` |
| 0111c | `a_job_list_unread_or_overflowed_parks_as_jobs_incomplete_naming_the_run_and_attempt_live_and_on_replay` | ordered reasons (r11 MEDIUM: overflow split) | `CI-PARK`: unread, `jobs-incomplete`, `ambiguous-required`; overflowed, `jobs-incomplete`, `missing-required`, `attempt-unnamed` |
| 0111c | `the_open_pull_request_list_unread_or_overflowed_parks_as_pulls_incomplete_live_and_on_replay` | ordered reasons | `CI-PARK`: `pulls-incomplete` |
| 0111c | `a_partly_read_forge_required_set_parks_as_required_set_incomplete_live_and_on_replay` | ordered reasons | `CI-PARK`: `required-set-incomplete` |
| 0111c | `an_oversized_first_page_through_the_selected_transport_parks_as_response_too_large` | ordered reasons; the byte bound through the selected transport (r11 MEDIUM) | `CI-PARK`: `checks-incomplete`, `response-too-large`, `missing-required` |
| 0111c | `a_duplicate_job_for_a_required_entry_on_the_last_page_parks_as_ambiguous_required` | ordered reasons, the control | `CI-PARK`: `ambiguous-required`; unread, `jobs-incomplete` first |
| 0111c | `an_entry_with_no_current_execution_names_missing_required_then_execution_ambiguous` | ordered reasons (name corrected) | `CI-PARK`: `missing-required`, `execution-ambiguous` |
| 0111c | `inspect_on_a_truncated_ci_park_after_its_bundle_is_gone_paints_the_named_rows_and_the_count` | `inspect` after the bundle is gone (text corrected) | the reason's rows, the count of the rest |
| 0111c | `a_required_check_not_completed_with_a_null_conclusion_parks_as_not_settled_alone` | P4's rows partition (LOW) | `CI-PARK`: `not-settled` |
| 0111c | `the_park_citation_checker_refuses_a_statement_with_no_row_or_an_unknown_row` | the checker's reach (LOW; r11 LOW: the noun and bare verb) | each bad line refused |
| 0111c | `every_test_proved_by_the_slices_is_named_in_its_slices_plan` | this list, both ways (r11 MEDIUM) | every name in its plan, each plan's count agreeing; a deleted row or plan entry refused |
| 0111c | `the_0111_index_row_names_the_last_round_the_decision_records` | the index row's history (INFO) | the row names round 1093r11 |
| 0111d | `a_pass_without_green_through_the_real_policy_and_park_writer_parks_at_ci_pass_unbound` | one owner for `word-unbound` (text corrected) | a `ruled` park at `CI-PASS-UNBOUND` |
| 0111d | `classify_after_ci_pass_receives_last_decision_naming_ci_pass_and_pass_and_nothing_else_of_ci` | `classify`'s input assertions | `CI-PASS` and `pass`, nothing else of `ci` |
| 0111d | `an_operator_retry_after_a_person_re_runs_all_jobs_observes_again_and_passes` | the person's re-run (LOW) | `CI-PASS` |

## Evidence

Every claim about today's behavior is read at the story's base, `e667a04c2`, or named
unmeasured. The forge's shape was not read in any round: this seat holds no web capability, the
re-run verbs' parameters are the controller's reading of
<https://docs.github.com/en/rest/actions/workflow-runs> on 2026-11-18, and every forge fact cited
by URL is unmeasured until 0111a.

The first draft ran the two `grep` inventories in
[What main holds at HEAD](#what-main-holds-at-head) and the count of `infrastructure(true`
writers. Each later round read, at the same base, every file:line it newly cites, and each
citation stands where it is used. The round-by-round lists of what fix rounds 1093r2 to 1093r10
read, and of the review records each answered, are this section as it stood at `b5ffc5944`;
this round cut them so the decision does not grow (X3 of round 1093r11).

Round 1093r11, read at the story's base `e667a04c2`:

- `.github/workflows/ci.yml:1-30` (`labeled` and `unlabeled` re-run the gate on the same head);
- `crates/brokkr-store/src/chain.rs:15-45` and `crates/brokkr-store/src/seat_record/validation.rs:55-75`
  (the fence on `effect/succeeded`'s `result`), `contracts/seat-record.v6.schema.json:550-575`,
  `crates/brokkr-runtime/src/engine/sequence/success.rs:10-40` and
  `crates/brokkr-runtime/src/engine/decide.rs:20-100`, `:239-261`;
- `crates/brokkr-runtime/src/engine.rs:950-996` (the moved-head guard) and `:1460-1490` (a driver
  spawn failure is `effect/failed`); `crates/brokkr-protocol/src/process.rs:500-522` (the
  watchdog's `Indeterminate`); `crates/brokkr-runtime/src/engine/dispatch.rs:155-175`;
  `crates/brokkr-runtime/src/engine/input.rs:108-130`;
- the chief's notes and both positions of landing `landing-over-an-existing-branch--7833a069`, in
  `.forge/results/43ce433d-ed05-443e-8509-071c2ffb16ea-{chief,positions-adversarial,positions-correctness}.json`.

This round held each name [Proved by the slices](#proved-by-the-slices) lists to its slice's plan
by a scratch `grep -n -o -F` over this file (32 of 32, each inside its own slice's item), and read
its own new park statements against the citation rule by hand. 0111c's doc tests make both a
gate.

The entry check's verdicts over the narrowed order (ruling 6) are read from HEAD's code, not run:
**unmeasured** until 0111d's entry tests.
