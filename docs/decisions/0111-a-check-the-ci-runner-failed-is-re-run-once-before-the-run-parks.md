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

**No classification.** The first draft and fix rounds 1093r2 to 1093r8 drafted a classifier
that typed a red
observation as the runner's (a typed park) or the change's (a return to `implement`). For three
rounds each security hold was the same class: a fact a refusal needs was lost or misread, so a
case owed a person's ruling fell through to an implement visit. Each round's fix added surface,
and the next round found a new case in it; the decision grew from 2,366 to 2,795 lines. Many of
the classifier's inputs were unmeasured until 0111a anyway: GitHub's environment-file
precedence, the step logs, matrix identity and the revision a step ran on. On 2026-11-20 the
operator ruled "narrow 0111 as recommended": keep the CI reading, the trust-root model and the
security bounds; park every non-green CI result for a person; defer the classifier and the
implement route ([Deferred to a later decision](#deferred-to-a-later-decision)). With no
outcome weaker than a person's park, the class has nowhere to fall.

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
  ([Deferred to a later decision](#deferred-to-a-later-decision)). The person tells a port bind
  from a red test by reading the observation, and re-runs.
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
  which is the one home of what parks. The park names the gate's rule and carries the
  observation's digest (ruling 7). The person reads the observation and re-runs or rules.

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
table, less every rule whose `from` is `ci`, its `initial` and its `ci` phase, equals
`landing`'s composed table rule for rule); and
`nothing_in_the_recipes_or_seat_scripts_reads_the_ci_phases_result` (a search of every recipe's
bundle, policy and scripts, and of `scripts/`, for `ci_observation`, `ci_green` and a
`from: ci` decision finds only the rules whose `from` is `ci`). Both derive the `ci` rows from
the composed table itself, never from a count, so they agree with ruling 6's table, whatever it
holds. A text search
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
  engine and the recipes observe CI, and park every observation that is not established green,
  with no diagnosis seat attempt. They send no re-run request and hold no write credential. A
  person reads the observation and re-runs or rules.
  - Cost: the issue's "re-run once" is not built. The wall-clock wait for a person remains, but
    the diagnosing seat attempt the issue counts is gone.
  - It refuses every forge write. A park costs a person's look, never a pass or a re-run.
- **(b) An automated re-run on GitHub, as the forge offers it.** The engine would send one of
  the two verbs above. The residual, stated plainly:
  - **No attempt precondition.** A request names a job or a run, never the attempt the engine
    observed. A re-run another actor starts between the engine's read and its write is re-run
    or widened again, and the forge cannot be asked to refuse it.
  - **Dependents re-run, an audit among them.** Each verb re-runs "and its dependent jobs". A
    dependent that passed, or an audit job, re-runs with the target, and its new result replaces
    the one the engine observed.
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
  unmeasured; slice 0111b's test settles it.
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

- **The frame.** A forged or tampered observation costs at most a person's park, or landing as
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

What HEAD lacks for this form, each built by slice 0111b:

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

It then lists every check the forge reports for that head, with no filtering. Checks outside
Actions are included:

- a check run carries its id, its app (slug and numeric id), its name, its `head_sha`, its
  status and its conclusion
  (<https://docs.github.com/en/rest/checks/runs#list-check-runs-for-a-git-reference>);
- a check from Actions also carries its producer: its workflow run's id, number and attempt, its
  workflow file's path, its triggering event, the revision the run tested, and its job's id and
  name (<https://docs.github.com/en/rest/actions/workflow-runs>,
  <https://docs.github.com/en/rest/actions/workflow-jobs>);
- every workflow run the forge lists for the head, with its path, event, number, attempt,
  status, `head_sha` and its pull-request association (below), so the engine can tell which
  run is current and whose it is, and every job of each current run attempt;
- every open pull request the forge lists for the head commit
  (<https://docs.github.com/en/rest/commits/commits#list-pull-requests-associated-with-a-commit>),
  so a head two pull requests share is seen;
- a commit status carries its context, its state and its creator's login and numeric id
  (<https://docs.github.com/en/rest/commits/statuses#get-the-combined-status-for-a-specific-reference>),
  so it can be named; it is never admitted as required evidence (below).

**The reads are bounded, and an overflow is a witness, never a truncation.** Each list is paged
by the forge, and the observation carries, for each, whether it was read to its last page. Each
has a bound the contract fixes: 1,024 checks and 1,024 statuses for the head, 256 runs per
workflow key, 1,024 jobs per run attempt, 64 open pull requests on the head and 256 contexts in
the forge's required set. A list that reaches its
bound is not truncated into a complete-looking one: the observation seals, in its place, a
bounded **overflow witness**, the list, the limit, the count and pages read when the observer
stopped (a lower bound), and no entries past the bound. A witness, like an unread page, leaves
the list incomplete. **Every list the observer reads has its own completeness row**, because
P2 needs every one of them, not only those another predicate reads: the check runs for the head
(`checks-incomplete`), the commit statuses for the head (`statuses-incomplete`), each workflow
key's runs (`runs-incomplete`), each current run attempt's jobs (`jobs-incomplete`), the open
pull requests on the head (`pulls-incomplete`) and, where it is read, the forge's required set
(`required-set-incomplete`). A list whose overflow witness or unread page the observation
carries makes its row true, whether or not any other row reads that list. The whole observation
is also bounded at 1 MiB on the observer's stdout (ruling 2).

**The observer reads no job log.** No condition of `green` reads a step's log, so no log byte is
read, sealed or journaled, and the person reads the log itself on the forge, from the run,
attempt and job ids the park names. Whether the forge masks every secret is therefore not a
question this decision depends on.

It is parsed once at the edge, with unknown fields refused (0071 ruling 3). An unknown
conclusion word is `observation-unreadable`, naming it. Whether the forge reports a workflow
job's key, as the workflow file spells it, is unmeasured until 0111a's evidence; if it does, the
observation carries it.

**`green` needs this pull request's run.** HEAD's workflow keys its concurrency by ref
(`.github/workflows/ci.yml:19-27`) and reads pull-request-specific data, the body and the
labels, into a required job (`:78-102`, where the `by-hand` label changes its outcome). So two
pull requests that share a head can each produce a run on it, with different results, and a run
keyed only by workflow path, event and head may be the other one's. **Recommended: (a) an
Actions execution counts toward `green` only when the forge positively associates it with this
pull request, and with no other.** The field is the workflow run's `pull_requests` array, each
element naming a pull request's `number` and its `head` and `base` with `ref` and `sha`
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

Otherwise the execution is not admitted, and the observation parks, naming
`association-unproven` and the run, or `association-ambiguous` and every pull request the array
names. Refused alternatives: **(b)** matching by workflow path, event and head alone, which is
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
requirement, so it cannot establish that every required check succeeded. Admitting statuses is a
follow-up that needs a revision attestation the forge does not give a status.

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
  declared list, or the observation parks as `required-set-disagrees`, for the same reason. The
  forge's set can only widen what `green` needs, never narrow it. The token scope that read needs
  is unmeasured; where it is unread, the observation records `forge_required: unread`, and the
  pinned list alone governs. A read that began and stopped short is not `unread`: it is
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
no valid merge is; that row is replaced.) Whether any field of a run records the commit it
tested, in either mode, is **unmeasured** until 0111a, which measures the tested-run identity
for a `head` checkout and a `merge` checkout alike. Until 0111a's evidence shows such a field,
every observation over any required entry makes `revision-unproven` true, for this house or an
adopter. That fails closed: it is a person's look, never a pass. If 0111a finds no such field
for a mode, the decision is revised before acceptance.

The revision is the run's, not a step's. Which commit a particular step of a job ran on, after a
checkout the workflow or a script moves, is CI's own machinery: a change that moves it is green
on the forge as well, which is landing's existing exposure ([The gate grants
nothing](#the-gate-grants-nothing)), and a wrong `green` from it costs landing as it is today.
Proving a step's revision was the deferred classifier's need, so that a park could be typed as
the runner's; it leaves with it ([Deferred to a later decision](#deferred-to-a-later-decision)).
`delivered by brokkr`, with its trusted-base verifier and its explicit pull-request head
(`:49-102`), is not in the entry set at all (ruling 6): its evidence is this landing's own
output.

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

- **P1, the binding.** The observation's repository (owner, name and id), pull request and
  gated head equal the run's pinned repository, the pull request the gate names and the
  engine's own git head; and the forge's `head.sha` for the pull request is that head, so the
  head is published.
- **P2, complete inventories.** Every list the observer reads was read to its last page, inside
  its bound, with no page unread: the check runs, the statuses, each key's runs, each current
  attempt's jobs, the open pull requests on the head, and the forge's required set where it is
  read ([the bounded reads](#3-the-observation-the-judgment-and-where-the-journal-keeps-them)).
  Each list's completeness is its own sealed fact, scoped to the
  exact list it names: a job list's to its run id and attempt, so a complete check list never
  stands for a complete job list, nor one attempt's for another's.
- **P3, every required entry matched once, currently.** Every entry of `required_checks` in the
  entry set (ruling 6) has exactly one **eligible current match**: a check from its bound
  producer (its app, workflow path, job and event) in that producer's current execution, the
  one run attempt defined below. A check from the same producer in another run, or in another
  attempt of the current run, is **stale**, whatever it concluded, and never counts. "Exactly
  one" is read over the complete check list and the complete job list of that attempt (P2), so
  a second job reporting the entry's name on a page the observer did not read can never leave
  the entry looking matched once.
- **P4, settled success.** Each match's status is `completed` and its conclusion `success`.
- **P5, this pull request's single association.** Each current execution's `pull_requests`
  names this pull request and no other, with `head.sha` the gated head and `base.sha` the base
  the observation names.
- **P6, the revision proven tested.** Each current execution's tested revision is established
  from its tested-commit record, in every checkout mode, as its entry's `tested` word requires
  (above).
- **P7, a readable observation from a trusted launch.** The launch passed ruling 2's rules, and
  the observation parsed under its contract, inside its bound.

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
| `unrelated` | the observation's repository, pull request and gated head; the run's pinned repository, the gate's pull request and the engine's git head | P1 | any of the three differs, or is unread | `an_observation_of_another_head_repository_or_pull_request_parks_as_unrelated` |
| `head-unpublished` | the forge's `head.sha` for the pull request; the engine's git head | P1 | they differ, or either is unread | `an_unpushed_head_parks_as_head_unpublished` |
| `checks-incomplete` | the check-run list's last-page fact and overflow witness | P2 | the list was not read to its last page, or overflowed its bound; it names the pages read and the witness | `the_check_run_list_unread_or_overflowed_parks_as_checks_incomplete_live_and_on_replay` |
| `statuses-incomplete` | the status list's last-page fact and overflow witness | P2 | as `checks-incomplete`, for the commit statuses | `the_status_list_unread_or_overflowed_parks_as_statuses_incomplete_live_and_on_replay` |
| `runs-incomplete` | each workflow key's run list, its last-page fact and overflow witness | P2 | as `checks-incomplete`, for one key's runs; it names the key | `a_run_list_unread_or_overflowed_parks_as_runs_incomplete_live_and_on_replay` |
| `jobs-incomplete` | each current run attempt's job list, its last-page fact and overflow witness | P2 | as `checks-incomplete`, for one attempt's jobs; it names the run id and attempt | `a_job_list_unread_or_overflowed_parks_as_jobs_incomplete_naming_the_run_and_attempt_live_and_on_replay` |
| `pulls-incomplete` | the open-pull-request list for the head, its last-page fact and overflow witness | P2 | as `checks-incomplete`, for the open pull requests on the head | `the_open_pull_request_list_unread_or_overflowed_parks_as_pulls_incomplete_live_and_on_replay` |
| `required-set-incomplete` | the forge's required set, where read, its last-page fact and overflow witness | P2 | the read began and did not reach its last page, or overflowed; a set never read is `unread`, and the pinned list alone governs | `a_partly_read_forge_required_set_parks_as_required_set_incomplete_live_and_on_replay` |
| `empty` | the complete check list for the head | P3 | the forge reports no check for the head | `an_observation_with_no_check_parks_as_empty` |
| `missing-required` | each entry's matches | P3 | an entry has no eligible current match; a stale check never makes it present | `a_required_audit_missing_from_the_observation_parks_as_missing_required` |
| `ambiguous-required` | each entry's matches over the complete check and job lists | P3 | an entry matches more than one check, or more than one job of its current run attempt; it names every match | `two_matches_for_one_required_check_park_as_ambiguous_required` |
| `execution-ambiguous` | each key's complete run list | P3 | an entry's current execution cannot be told, as below | `two_unsettled_runs_for_one_entry_park_as_execution_ambiguous` |
| `not-settled` | each match's status | P4 | a match is still queued or in progress when the observer's polling bound ends | `a_required_check_still_in_progress_parks_as_not_settled` |
| `not-success` | each match's conclusion | P4 | a match concluded anything but `success`; it names the check and the word | `a_failed_cancelled_timed_out_skipped_or_neutral_required_check_parks_as_not_success` |
| `association-unproven` | each current execution's `pull_requests` | P5 | it does not name this pull request, its head and its base, or is empty | `a_run_with_an_empty_association_parks_as_association_unproven` |
| `association-ambiguous` | each current execution's `pull_requests` | P5 | it names more than one pull request, open or closed; it names every one | `an_association_naming_a_and_a_closed_b_parks_as_association_ambiguous` |
| `revision-unproven` | each current execution's tested-commit record, in either checkout mode | P6 | the record is absent or unread, in `head` mode as in `merge` mode | `a_head_entry_whose_run_records_no_tested_commit_parks_as_revision_unproven` and `a_merge_entry_whose_run_records_no_tested_commit_parks_as_revision_unproven` |
| `revision-disagrees` | each current execution's tested-commit record and its mode's separate comparisons | P6 | one comparison fails; it names the two values | `a_head_entry_whose_tested_commit_differs_from_the_gated_head_parks_as_revision_disagrees` and `an_association_and_a_merge_commit_that_disagree_park_as_revision_disagrees` |
| `producer-mismatch` | every check under an entry-set entry's name | | a check under that name from another producer concluded red | `a_sole_red_producer_mismatch_parks_as_producer_mismatch` |
| `status-required` | the forge's required set, where read | | it names a commit status context | `a_status_context_in_the_forges_required_set_parks_as_status_required` |
| `required-set-disagrees` | the forge's required set, where read; the declared list | | the forge requires a context the declared list lacks | `a_forge_required_check_the_list_lacks_parks_as_required_set_disagrees` |
| `observation-too-large` | the observer's stdout length | P7 | it is over its bound | 0111b: `an_observation_over_the_stdout_bound_refuses_as_observation_too_large` |
| `observation-unreadable` | the observer's stdout bytes | P7 | they are not UTF-8, or hold an unknown field or word, or lack a field | 0111b: `an_observation_with_an_unknown_field_refuses_whole` |
| `launch-untrusted` | the launcher's pre-spawn checks | P7 | a rule of ruling 2 failed before the spawn; it names the rule | `an_untrusted_launch_parks_ruled_as_launch_untrusted_and_observes_nothing` |
| `word-unbound` | the gate's result word and `ci_green`, read by the table, not the judge | | the word is `pass` while `ci_green` is false, which the word binding refuses (below), so it is reached only at `CI-PASS-UNBOUND` if that binding were lifted | 0111d: `the_whole_composed_landing_ci_overlay_compiles_against_heads_totality_gate` |

**Every true row parks, and none is weaker than another.** There is no weaker outcome to fall
to, so no precedence among the rows is needed: the engine names every row that is true, in the
table's order, and parks. A fact several rows read leaves each of them true, and each is named:
an unread engine head names both `unrelated` and `head-unpublished`; a check list short of its
last page names `checks-incomplete` and every `missing-required` it leaves. The set is exact
and deterministic. Live and on replay the judgment takes the one function over the one sealed
observation and the one pinned list ([What a replay reads](#3-the-observation-the-judgment-and-where-the-journal-keeps-them)),
so the rows a replay names are the rows named live.

**The table binds the text.** 0111c's doc test,
`every_park_statement_in_0111_cites_a_row_of_its_park_table`, reads this decision: every
paragraph, list item or table row that says the gate parks, spelled `parks` in prose (a test
name in a code span is not a statement) or `CI-PARK`, outside the heading,
[What main holds at HEAD](#what-main-holds-at-head),
[Deferred to a later decision](#deferred-to-a-later-decision) and [Evidence](#evidence), names
a row id of this table in backticks, such as `unrelated`, or links this table; and every row
has an id, a `Reads` fact and a `Bound by` test.
Its companion, `every_park_row_is_bound_by_a_test_the_slices_name`, checks that every row's
`Bound by` cell names a test that one of [The slices](#the-slices) lists, and 0111c's
`the_judges_park_rows_are_the_decisions_park_table_in_order` holds the judge's closed row enum
to this table's ids in this order, so the code and the decision cannot drift apart.

The **current execution** of an Actions entry is the workflow run with the highest run number
among those the forge lists for its workflow path, its event and the head, read at its highest
attempt. Any other run for that key still queued or in progress, or a run list the observer
could not read whole, is `execution-ambiguous`. The sealed observation holds each check's run id
and attempt and each key's whole run list, so which match is current, and which stale, is
re-derived from the journal alone.

So a required audit with no check in its current run, an older same-producer `success` on the
same head, and a second required job with an ordinary failed test parks, naming
`missing-required` for the audit and `not-success` for the test: the stale success never makes
the audit present. Fix rounds 1093r6 to 1093r8 each found a case where such a fact fell through
to a weaker class; with none below the park, there is nowhere for it to fall.

The gate's word is bound to the engine's judgment: `pass` is admitted only on `green`, and
`fail` on `green` is refused. A refused word is never rewritten (the same binding 0109 ruling 2
gives a verdict).

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
values (the pinned repository, the gate's pull request and the engine's git head). HEAD's manifest
records each bundle file as a digest, never its bytes
(`crates/brokkr-runtime/src/bundle.rs:4214-4239`), so a replay reads the list from the compiled
bundle and refuses when its digest is not the manifest's, and reads the comparison values from
the run's own journal; which row holds the gated head is settled by 0111c's replay test, and is
unmeasured here. Where the bundle is gone, the park's
reason still names the observation's digest and its true rows from the journal alone, which is
what a person needs. Fix rounds 1093r4 to 1093r8 sealed a basis under `ci-classification/v1` so
a typed class could be re-derived from the journal alone; with no class to re-derive, that
record leaves with the classifier.

Ruling 2's launch refusal, `launch-untrusted`, holds before any observation exists. It is the
engine's pre-spawn fact, journaled as the park's reason with the condition that failed, and a
replay reads it there.

- **(b) A new `ci/judged` event.** Moves the event envelope, every manifest and every fold arm
  for a fact an input can carry. Refused, as 0106 ruling 1 refused `run/park-caused`.

### 5. The list: its home and who declares it

**Recommended: (a) recipe data, `ci-checks.json` beside the policy, under `ci-checks/v1`,
compiled into the bundle and pinned with it.**

**`green` still needs a recipe-declared set, re-derived.** Ruling 3's condition 3 matches every
required entry by its producer, so the set must come from somewhere the change cannot edit.
The forge's branch protection alone can be changed by an administrator at any time, needs an
administration scope to read (unmeasured), and yields an empty set on an unprotected branch;
and "whatever checks the forge reports" cannot see an absent audit (ruling 3's (b) and (c)). So
the declared list stays, and with it its file. What the file loses is every key that existed
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
              "jq": {"path": "/usr/bin/jq", "sha256": "<64 hex>"}},
    "credential": {"binding": "GH_TOKEN", "access": "read"}
  },
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

The list is cut short: the house's full `required_checks` is the twelve required contexts
ruling 3 names. The app's numeric id is pinned beside its slug once 0111a's evidence shows it.
The compile refuses a `status` entry.

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

So the `ci` result has no projection, and nothing a later seat reads can stand for CI's
verdict. No seat needs one: the only road out of `ci` to a seat is `CI-PASS`, and a park's
evidence is for a person, who reads it through `inspect` (ruling 9). **(b)** projecting it
unchanged: a model seat would read the observation and a `green` it could take as authority.
Not recommended.

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
a pass the gate grants; it is still designed out, so a person's park is reached on every run
that names the overlay over a non-green observation. **Recommended: (a) `landing-ci`'s entry admission admits `ci` alone,
through the existing entry check.** The check's chain of refusals gains one arm: in a bundle
whose table declares a `ci` seat, every phase but that one refuses as a new `EntryRefusal`
variant, `BeforeCi`, naming the `ci` phase, and `admitted()` lists `ci` alone. The arm sits
after the work-seat arm and before the protected arm, so `ci` itself still meets every existing
check: protected, forward work, strategy, change and visits, which the materialized order above
admits. Every start the overlay admits enters at `ci`:

- **the default start** uses `initial`, `ci`, as HEAD's `first_phase` does (`entry.rs:139-150`);
- **`--enter ci`** passes every existing arm, as above, and `BeforeCi` does not judge `ci`
  itself;
- **continuation**, `--enter ci --continues <run>`, is the same admission over a continued run;
  landing's documented `--enter classify --continues <run>` refuses as `BeforeCi` under the
  overlay, and the person continues at `ci` instead;
- **`--enter review`, `--enter classify`** and every other phase refuse as `BeforeCi`.

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
observer lists it with every other check, unfiltered, and the sealed observation holds it; no
condition of `green` reads it; and the forge's required-set comparison counts it as declared.
The rule for the recipe data: an entry whose producer reads anything this run's ship seat writes
(its anchor, its journal, its ledger) carries `"after": "ship"`. For the house that is
`delivered by brokkr`, and only it. The forge's requirement stays at merge, unchanged. The
declaration is the operator's: the compile cannot see what a check reads, and an entry left
undeclared costs a person's park (the frame's table), never a pass. Refused alternative:
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
whose attempt ends in the gate's `fail` before any observation exists (ruling 2). It needs no
`when`: the word binding admits `fail` only off `green` (ruling 3).

**The table is total.** HEAD's compile refuses a table with an unruled valuation:
`seated_and_ruled` calls `refuse_unruled` (`crates/brokkr-runtime/src/bundle/phases.rs:118-130`),
which sweeps every valuation of each `(from, result)` group's axes through the real evaluator
and refuses the first no rule rules (`crates/brokkr-core/src/policy/audit.rs:187-197`,
`:358-381`). A flag axis is both booleans (`:441-443`), whoever owns the flag. So with
`CI-PASS` the sole `(ci, pass)` row, the valuation `ci_green = false` is unruled and the
composed overlay would not compile. HEAD's boolean sweep therefore still needs
**`CI-PASS-UNBOUND`**, recommended as (a): the unconditional `(ci, pass)` rule after `CI-PASS`,
parking `ruled` for a person on row `word-unbound`. It is never reached while the binding
holds: ruling 3 refuses `pass` off `green` before the table reads it. It is the table's answer if
the binding were ever lifted, and a park, never a route. The `(ci, fail)` group is total:
`CI-PARK` is unconditional ([park table](#the-park-table)). **(b) Proving the valuation impossible** is not taken: the audit
sweeps each flag on its own, and HEAD gives it no way to read a cross-input invariant such as the
word binding, so the proof would need an engine change for one row.

Then landing's own two rules, `CLASSIFY-DOCS` to `review` and `CLASSIFY-CODE` to `verify`
(`recipes/landing/policy.json:9-21`), and every `fast` rule, unchanged.

**No loop runs through `ci`.** `ci` is entered only as the run's `initial`, and again only by
the operator's retry of its own park; the compile refuses any rule whose `next` is `ci`. Nothing
leaves `ci` but `CI-PASS` and the two parks on the [park table](#the-park-table)'s rows, so the
gate buys no seat attempt beyond its own
exec attempt, and `fast`'s implement bound, `visits_implement_gte: 3`
(`recipes/fast/policy.json:76-94`), counts only the visits landing's own rules buy, as today.

**What this does not gate.** A head the run itself makes, by an implement after a verify or a
review return, is local, and no gate in this run reads its CI: it is verified locally, judged,
and shipped for a person to push, and the pull request's CI then runs on it as it does today.
Gating a head the run made needs a publish-and-resume handoff: a park that names the head to
push, and a resume that refuses any head but that one. That is a follow-up, and the pre-review
interception in `fast` and `verify` that fix round 1093r3 drafted, removed here, waits on it.

**Refused routes**, each because it would hide a failure, skip the gate, read an unpublished
head or write to the forge:

- any route to a re-run (ruling 1);
- any rule from `ci` whose `next` is not `classify`, `implement` among them: the implement
  route waits on the deferred classifier
  ([Deferred to a later decision](#deferred-to-a-later-decision));
- any rule whose `next` is `ci`: it would gate a head the run made, which is unpublished;
- any rule into `classify` but `CI-PASS`, and any explicit entry but `ci`: either would skip
  the gate;
- `CI-PASS` off `green`; the word binding refuses it (ruling 3).

Each is refused by the overlay test, over the complete composed table (ruling 10). The
scope-widened reconciliation park fix round 1093r2 promised went with the write (ruling 1); no
route or cause remains for it.

- **(b) The pre-review gate in `fast` and `verify`, as fix round 1093r3 drafted.** Refused for
  now: its head is unpublished by construction. It returns with the follow-up handoff above.

### 7. The park's cause

**Recommended: (a) 0106's existing `ruled`, naming the gate's rule and carrying the
observation's digest.**

- **(a) `ruled`.** HEAD's park arm already writes a table park as `ruled` with the matched
  rule's id (`crates/brokkr-runtime/src/engine/cursor.rs:369`; the variant is
  `crates/brokkr-core/src/park.rs:40-45`, a `rule_id` and a `detail`). The gate's park is a table
  park, so its cause is `ruled` with the id of the gate's park rule (ruling 6), and its `detail`,
  the park's reason, names the sealed observation's SHA-256 and the true rows of the
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
[park table](#the-park-table), and the evidence a
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

The first draft wrote `ci_fault_in: [runner-fault]`. That fails the frozen v2 schema: `when`'s
values are a boolean, a number or a string, never an array
(`contracts/phase-machine.v2.schema.json:71-77`). HEAD's runtime does read choice arrays
(`crates/brokkr-core/src/policy/conditions.rs:43-75`), but the published contract does not admit
them, and 0082 mints no version outside an accepted decision.

**Re-derived for one judgment.** Fix rounds 1093r5 to 1093r8 spelled four class words as three
flags, `ci_green`, `ci_infrastructure` and `ci_refused`, with `change` all three false. With the
judgment `green` or not, one flag spells it whole, and the rest leave with their classes.

- **(a) `ci_green` alone.** An `Owner::Engine` `Flag`, true exactly when the judgment is
  `green`. A boolean input by name is what v2's `when` admits (`:73`). Ruling 6's `CI-PASS`
  reads `ci_green: true`, and nothing else reads it. It is the table's reading of the engine's
  judgment rather than of the seat's word: the word binding (ruling 3) makes `pass` and
  `ci_green` agree, and if that binding were ever lifted, `CI-PASS-UNBOUND` parks a `pass`
  without `green` on row `word-unbound` rather than proceeding. No second flag is needed: the park rule is
  unconditional on `fail`, and a flag for "not green" would be the same fact twice (0071 ruling
  5). It mints no contract.
- **(b) No flag: route on the result word alone.** `(ci, pass)` to `classify` and `(ci, fail)`
  to the park, with no `when`. It is one input fewer, but the table would then admit a pass on
  the seat's word, and the engine's judgment would guard it only through the binding. Not
  recommended.
- **(c) A table contract of this decision's own, admitting a `<choice>_in` array.** Refused.
  Phase-machine v3 is decision 0109's, which is accepted, and its v3 is proposed in its slice
  0109c (`docs/decisions/0109-a-gate-s-re-visit-reads-what-changed.md:1500-1520`, `:1830`). This
  decision claims no table version of its own. What it needs from v3 is the ask below.

**One result per row.** Each rule's `result` is one string under v2
(`contracts/phase-machine.v2.schema.json:31`, `:50`), and the engine matches it by exact
equality (`crates/brokkr-core/src/policy.rs:264-273`). Ruling 1 removed the two re-run words
fix round 1093r2 put on one row. **Each row of ruling 6's landing table, checked against v2:**

- `CI-PASS`: one result, `pass`; `when` `ci_green: true` (`:73`); `next: classify`, a declared
  phase. Legal.
- `CI-PASS-UNBOUND`: one result, `pass`; no `when`; `park: true`, with no `next`, no severity and
  no `requires_artifacts`, as v2's park admits (`:53-57`). Legal, and not made unreachable by
  `CI-PASS`, which is conditional.
- `CI-PARK`: one result, `fail`; no `when`; `park: true`, as above. Legal, and the only rule of
  its `(from, result)` group, so no rule above it makes it unreachable (`:28`). The rows it parks
  on are the [park table](#the-park-table)'s, which the table's `when` never reads.
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
the classifier drafted in fix rounds 1093r2 to 1093r8 needed, and each is unmeasured:

- which revision a step ran on, beyond the run's tested commit: whether any forge field records
  a job's checkout;
- the revision of the workflow file GitHub ran for a run, and whether the forge reports it;
- how the forge names a matrix instance, so an observed job binds to its expected instance;
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

- Any forge write: ruling 1. No re-run is requested, and no seat holds a write credential.
- No model decides: the gate is an exec seat, and the judgment is a core function.
- No required check is skipped, marked passed or written.
- No refusal is weakened: nothing is weaker than the gate's park, and whatever is not
  established `green` parks ([the park table](#the-park-table)); `retryable`'s
  positive-evidence rule stands as 0106 wrote it, and this decision writes no `infrastructure`
  cause.
- No Windows code (0063).
- #1113's own fix. Its "Not Found" is a failed step of `RustSec dependency audit`
  (`.github/workflows/ci.yml:673-695`), so under the gate it parks for a person on `not-success`, who re-runs, as
  today outside any run.
- The dispatcher's retry budget: a `ruled` park spends none of it (ruling 7).
- The automatic classification of a red observation, and every route it would take, the typed
  port-bind park among them: the follow-up decision
  [Deferred to a later decision](#deferred-to-a-later-decision) names.
- Flaky-test handling of any kind, quarantine included: a follow-up of its own.
- `fast`'s `blocked_by_in` array outside v2's published schema: a follow-up (ruling 10).
- A CI gate on a head the run itself made, before review in `fast` or `verify`: a follow-up
  that needs a publish-and-resume handoff (ruling 6).
- Commit statuses as required evidence: a follow-up that needs a revision attestation (ruling 3).
- A forge other than GitHub: `forge` is a closed enum of one word in v1.
- An automated re-run: the follow-up decision ruling 1 (c) names.

## Consequences

A red CI no longer costs a model diagnosis attempt at landing's entry: every observation not
established `green` parks for a person, with a cause a script can read and the observation's
digest and every true row of the [park table](#the-park-table) a person can act on, and the person re-runs or rules. Brokkr sends the
forge nothing and types no failure. Nothing leaves `ci` but `green`, which proceeds as landing
does, and the park, so the gate is not a CI authority: the forge's required checks at merge stay
the only one. A wrong `green` costs landing as it is today, and a wrong park a person's look. A
recipe with no list behaves exactly as now. The cost is an exec observer per recipe that wants
it, one read-only token, a list an operator keeps, and a person's look at every red, a runner's
fault and the change's alike, until the deferred classifier is ruled. Two contract versions are
proposed: `ci-observation/v1` and `ci-checks/v1`. Each is minted only when 0111b lands under
this decision's acceptance (0082), and only after 0111a's evidence. This decision proposes no
table contract: the gate is `landing`'s entry, over a published head, and it waits on 0109's v3
and ruling 10's ask.

## The slices

Smallest first; each lands on its own. Evidence comes before any contract, because 0082 forbids
fixing a minted version in place: a shape the evidence would change is changed in this decision,
while it is proposed, never in a minted file. Each piece of lifecycle and readout work is in the
slice that produces it, and each assertion is in the slice that builds its writer. Two contract
versions are proposed: `ci-observation/v1` and `ci-checks/v1`.

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
   - how each list pages, and how the observer tells the last page (ruling 3's bounded reads);
   - the closed list of conclusion and status words.

   Where a fact differs from what this decision assumes, the decision is revised before it is
   accepted, or before 0111b mints. Test: `the_ci_evidence_files_are_pinned_by_digest`.
2. **0111b — the observer and its trusted launch.** Files: a new `crates/brokkr-core/src/ci.rs`
   with `ci/observation.rs` and `ci/checks.rs` (the two contracts' types, parsed once with
   unknown fields refused) and `ci/tests.rs`; `crates/brokkr-core/src/lib.rs`;
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
   is inside the worktree, over 1091a's descriptor-held walk, so 0111b lands after 1091a; and a
   house observer script, with its polling bound inside the seat's deadline, every list read to
   its last page or sealed with its overflow witness, and its shellcheck clean. Its compile
   refuses every live `ci` declaration until 0111d (ruling 2):
   `a_bundle_declaring_a_ci_seat_refuses_to_compile_until_the_first_live_producer_lands`, a
   custom exec recipe's declaration included. About 450 production, 150 script and 700 test
   lines; it may split at triage into the contracts and the launch. Contracts, minted here and
   only after 0111a: `ci-observation/v1` and `ci-checks/v1`. Tests, the launch driven through a
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
     `the_observer_reads_every_page_of_each_current_attempts_job_list`;
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
   - the result channel and environment:
     `a_replaced_result_file_under_forge_results_is_ignored_and_the_stdout_observation_is_read`;
     `an_observation_over_the_stdout_bound_refuses_as_observation_too_large`;
     `a_non_utf8_observation_refuses_and_is_never_decoded_lossily`;
     `the_ci_site_journals_no_stderr_bytes`;
     `a_ci_seat_whose_layer_is_inside_the_worktree_refuses_at_start`;
     `an_inherited_bash_env_never_reaches_the_observer`;
     `the_observer_runs_from_a_directory_outside_the_worktree`;
     `the_launch_directory_is_removed_after_the_observer_ends`;
     `an_environment_key_outside_the_allow_list_refuses_to_compile`;
     `the_observer_reads_no_gh_configuration_from_the_worktree_or_home`;
     `the_observer_names_the_repository_and_never_reads_the_worktrees_remotes`;
     `the_observer_lists_every_check_for_the_head_unfiltered` (a stub `gh` given by absolute
     path, its output held to `ci-observation/v1` and to 0111a's evidence);
     `the_observer_holds_no_write_binding`;
   - the seal: `a_recorded_observation_over_stdout_is_sealed_as_the_seat_record`.
3. **0111c — the judgment, the park rule and its readout.** Files: `crates/brokkr-core/src/ci/judge.rs`
   (`judge(observation, pinned)`, total, matching every conclusion and status word without a
   wildcard, 0071 ruling 2); `crates/brokkr-core/src/policy/inputs.rs` (the engine-owned
   `ci_green` flag); `crates/brokkr-runtime/src/engine/decide.rs` (judge at decide, bind the
   word, and write `ci_green`); the park's reason for a `ci` park, naming the gate's rule, the
   observation's digest and its true rows inside 256 bytes, in the engine's park path
   (`crates/brokkr-runtime/src/engine/cursor.rs:369` keeps writing `ruled`);
   `crates/brokkr-runtime/src/bundle/` (the refusals of ruling 6's routes); and the readout it
   produces, `crates/brokkr-view/src/` (the CI view model: the judgment, every true row, and the
   run, attempt and job ids a person needs) with `crates/brokkr-cli/src/render/inspect.rs`. About
   300 production and 700 test lines, over test-built tables and observations built by a shared
   builder from 0111a's evidence; no recipe yet. No contract change. Tests, each asserting the
   exact set of unmet words, so deleting any one word from the expected set fails it:
   - the park table ([its one home](#the-park-table)):
     `every_park_statement_in_0111_cites_a_row_of_its_park_table` and
     `every_park_row_is_bound_by_a_test_the_slices_name`, two doc tests in a new
     `crates/brokkr-cli/tests/ci_park_table.rs`, a module of `tests/it.rs` beside
     `decisions_index` (`it.rs:48`), reading this decision's text; and
     `the_judges_park_rows_are_the_decisions_park_table_in_order`, which parses the table's `Id`
     column and asserts it equals the judge's closed row enum, in order;
   - `green`: `a_complete_settled_successful_current_observation_is_green`;
     `the_full_two_entry_engine_matrix_with_a_distinct_valid_merge_commit_is_green` (both
     `test (…)` entries, each run's merge commit distinct from the head, its parents the base and
     the head);
     `an_inadmissible_check_that_succeeded_changes_nothing`;
   - one test per word, the word's condition unmet and every other established:
     `an_observation_of_another_head_repository_or_pull_request_parks_as_unrelated`;
     `an_unpushed_head_parks_as_head_unpublished`;
     `an_observation_with_no_check_parks_as_empty`;
     `a_required_audit_missing_from_the_observation_parks_as_missing_required`;
     `two_matches_for_one_required_check_park_as_ambiguous_required`;
     `a_duplicate_job_for_a_required_entry_on_the_last_page_parks_as_ambiguous_required` (the r8
     case: every expected job observed once on the first pages, a second job with the entry's
     name on the last; and its control, the last page unread: `jobs-incomplete`, never
     `green`);
     `two_unsettled_runs_for_one_entry_park_as_execution_ambiguous`;
     `a_required_check_still_in_progress_parks_as_not_settled`;
     `a_failed_cancelled_timed_out_skipped_or_neutral_required_check_parks_as_not_success`;
     `a_run_with_an_empty_association_parks_as_association_unproven`;
     `a_red_setup_only_run_on_the_same_head_for_another_pull_request_parks_as_association_unproven`;
     `an_association_naming_a_and_a_closed_b_parks_as_association_ambiguous`;
     `a_head_entry_whose_run_records_no_tested_commit_parks_as_revision_unproven` (the chief's
     r9 case: the run's `head_sha` and the association's `head.sha` both the gated head, every
     required check a settled success, and no tested-commit record);
     `a_head_entry_whose_tested_commit_differs_from_the_gated_head_parks_as_revision_disagrees`
     (both heads match; the record names another commit);
     `a_merge_entry_whose_run_records_no_tested_commit_parks_as_revision_unproven`;
     `an_association_and_a_merge_commit_that_disagree_park_as_revision_disagrees`;
     `a_merge_revision_whose_parents_are_not_the_base_and_the_head_parks_as_revision_disagrees`;
     `a_sole_red_producer_mismatch_parks_as_producer_mismatch`;
     `a_status_context_in_the_forges_required_set_parks_as_status_required`;
     `a_forge_required_check_the_list_lacks_parks_as_required_set_disagrees`;
     `an_untrusted_launch_parks_ruled_as_launch_untrusted_and_observes_nothing`;
   - the inventories, one test per list the observer reads, each with two cases, the last page
     unread and the list sealed as its overflow witness, every other list complete and every
     other row false, each expecting `CI-PARK` naming that list's row (`checks-incomplete` to
     `required-set-incomplete`), and each run twice:
     live, a stub `gh` serving the pages to 0111b's observer through its test-only launch and
     decide judging its stdout; and on replay, a fold rebuilt from that journal re-deriving the
     same rows:
     `the_check_run_list_unread_or_overflowed_parks_as_checks_incomplete_live_and_on_replay`;
     `the_status_list_unread_or_overflowed_parks_as_statuses_incomplete_live_and_on_replay`;
     `a_run_list_unread_or_overflowed_parks_as_runs_incomplete_live_and_on_replay`;
     `a_job_list_unread_or_overflowed_parks_as_jobs_incomplete_naming_the_run_and_attempt_live_and_on_replay`;
     `the_open_pull_request_list_unread_or_overflowed_parks_as_pulls_incomplete_live_and_on_replay`
     (the chief's r9 case: every required job complete and successful, its run uniquely
     associated, and only this list overflowed);
     `a_partly_read_forge_required_set_parks_as_required_set_incomplete_live_and_on_replay`;
   - the shared-fact cases, each naming every word the one fact leaves unmet:
     `an_unread_engine_head_names_unrelated_and_head_unpublished`;
     `a_short_check_list_beside_an_absent_audit_names_checks_incomplete_and_missing_required`;
   - the stale checks:
     `a_stale_same_producer_success_beside_an_ordinary_failure_names_missing_required_and_not_success`
     (a required audit with no check in its current run, an older same-producer `success` on
     the same head, and a second required job with a failed test);
     `a_stale_same_producer_red_never_counts`;
   - the word binding and the table: `a_pass_word_off_green_is_refused_and_never_reaches_ci_pass_unbound`;
     `a_fail_word_on_green_is_refused`; `ci_green_is_true_exactly_on_green`;
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
   journaled, and a sweep can count parked runs against the diagnosis seat attempts they saved. The
   park's lifecycle is this slice's too: a person looks and re-runs, then the operator retries
   the parked `ci` phase, which observes again. Tests:
   - the frame's two ([The gate grants nothing](#the-gate-grants-nothing)):
     `after_ci_pass_the_composed_table_is_landings_own_rule_for_rule` and
     `nothing_in_the_recipes_or_seat_scripts_reads_the_ci_phases_result`;
   - the consumer input (ruling 6's projection), each built by the engine's own input builder
     over a journal holding the `ci` decision and asserting the exact keys:
     `after_ci_pass_no_later_seat_input_holds_the_ci_observation_or_judgment` (`classify`,
     `verify`, `review` and `ship`: no `results.ci`, and no `ci_*` key anywhere in the input);
     `the_review_seat_input_holds_no_ci_key_on_any_road` (after `CLASSIFY-DOCS`, and after
     `VERIFY-PASS`);
   - the entry (ruling 6), each over the overlay's materialized bundle, its composed table and
     phase order, through HEAD's `first_phase` and `admitted()`:
     `the_landing_ci_default_start_enters_ci`;
     `landing_ci_admits_ci_alone_as_an_explicit_entry` (`--enter ci` passes every existing arm,
     `ForwardWork` among them, and `admitted()` is exactly `["ci"]`);
     `landing_ci_enter_ci_with_continues_is_admitted`;
     `landing_ci_enter_review_refuses_as_before_ci`;
     `landing_ci_enter_classify_refuses_as_before_ci`, with and without `--continues`;
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
     `an_operator_retry_after_a_person_re_runs_observes_again`;
     `a_head_the_run_made_is_never_gated_by_ci`;
     `the_environment_park_survives_the_ci_overlay`;
   - the table: `the_composed_landing_ci_table_validates_whole_against_phase_machine_v3`;
     `the_whole_composed_landing_ci_overlay_compiles_against_heads_totality_gate` (the
     materialized bundle through `seated_and_ruled` and `refuse_unruled`: it compiles; and its
     control, the same overlay less `CI-PASS-UNBOUND`, refuses with `Refusal::Totality` naming
     `ci`, `pass` and `ci_green = false`).

## Evidence

Every claim about today's behavior is read at the story's base, `e667a04c2`, or named
unmeasured. The forge's shape was not read in any round: this seat holds no web capability, the
re-run verbs' parameters are the controller's reading of
<https://docs.github.com/en/rest/actions/workflow-runs> on 2026-11-18, and every forge fact cited
by URL is unmeasured until 0111a.

The first draft ran the two `grep` inventories in
[What main holds at HEAD](#what-main-holds-at-head) and the count of `infrastructure(true`
writers, and read `crates/brokkr-core/src/park.rs`, `crates/brokkr-core/src/policy/inputs.rs`,
`crates/brokkr-runtime/src/engine/cursor.rs`, `recipes/fast/policy.json`,
`recipes/fast/bundle.json`, `recipes/landing/README.md`, `adapters/exec.json` and
`contracts/phase-machine.v2.schema.json`.

Fix rounds 1093r2 to 1093r8 each read, at the same base, every file:line they newly cited, and
each round's commit messages name the review positions it answered. The citations this decision
still carries, by the round that first read them:

- 1093r2: `.github/workflows/ci.yml:3-26` and `:673-695`; `recipes/fast/bundle.json:25-100`,
  `recipes/fast/policy.json:30-100` and `recipes/verify/bundle.json:40-46`;
  `crates/brokkr-runtime/src/bundle/expand.rs:1-60` and `bundle/model_policy_tests.rs:2595-2625`;
  `bundle/assemble.rs:320-342` and `engine/decide.rs:68-86`; `engine/cursor.rs:355-390`;
  `crates/brokkr-core/src/policy/conditions.rs:35-80` and `policy/inputs.rs:75-190`;
  `contracts/phase-machine.v2.schema.json:55-95`; decision 0106 (`:1205-1235`).
- 1093r3: `.github/workflows/ci.yml:1-70`, `:300-325` and `:655-700`;
  `recipes/brokkr-fast/bundle.json:4` and a `jq` scan of every `recipes/*/policy.json` for an
  array-valued condition; `crates/brokkr-protocol/src/adapters/exec.rs:25-50`,
  `adapters.rs:756-853` and `process.rs:85-100`, `:295-310`;
  `crates/brokkr-runtime/src/engine.rs:2040-2056` and `:2860-2885`; `bundle.rs:3938-3968` and
  `:4147-4157`; `bundle/compose.rs:815-840` and `:868-922`; `crates/brokkr-core/src/policy.rs:258-278`;
  `crates/brokkr-bridge/src/lib.rs:1-30`, `:210-229`; decisions 0049 (`:30-60`) and 0109
  (`:1500-1530`, `:1830`).
- 1093r4: `.github/workflows/ci.yml:25-27`, `:40-105` and `:225-330`;
  `.github/branch-protection.json` whole; `recipes/landing/README.md`, `bundle.json` and
  `policy.json` (by `jq`), and `recipes/fast/scripts/ship-seat.sh`'s `push` lines;
  `crates/brokkr-runtime/src/engine/input.rs:66-84`, `engine.rs:1044-1052`, `places.rs:684-692`,
  `engine/dispatch.rs:145-166` and `bundle.rs:4212-4240`.
- 1093r5 to 1093r8: decisions 0012 (`:33-37`) and 0046 (`:179-191`);
  `crates/brokkr-cli/src/cli_args.rs:220-225`; `crates/brokkr-runtime/src/realms.rs:409-412`;
  `crates/brokkr-protocol/src/hands/namespace/isolation.rs:4-23`, `hands/namespace.rs:154-161`
  and `hands.rs:764`; `crates/brokkr-protocol/src/secret.rs:123-130`;
  `scripts/delivered-by-brokkr.sh:189-204`, `:197-200` and `:265-267`;
  `crates/brokkr-runtime/src/bundle/entry.rs:139-209`, `bundle/phases.rs:118-157` and
  `crates/brokkr-core/src/policy/audit.rs:187-197`, `:358-381`, `:441-443`;
  `crates/brokkr-core/src/policy/direction.rs:10-19`; `crates/brokkr-view/src/rail.rs:9-12`.

This round, 1093r9, read at the story's base or at its head `27fa1c0c4` (the tracked tree
outside `docs/decisions/` is the base's):

- `crates/brokkr-core/src/park.rs:20-80` and `:272-295` (the `Ruled` variant's `rule_id` and
  `detail`, and `ParkCause::line`'s `ruled <rule>: <detail>`);
- `crates/brokkr-runtime/src/engine/cursor.rs:350-395` (a table park is written `ruled` with the
  matched rule's id) and `crates/brokkr-core/src/fold/run.rs:66-85` (`ParkReason`);
- `crates/brokkr-core/src/summary.rs:53` (`PARK_REASON_BYTES`, 256);
- `crates/brokkr-runtime/src/bundle/entry.rs:130-300` (`first_phase`, `refusal`, `protected`,
  `forward_work`, `visits` and `change`) and `crates/brokkr-core/src/policy/direction.rs:1-30`;
- `recipes/landing/policy.json` by `jq` (its phases, `initial` and two rules) and
  `recipes/landing/bundle.json:1-30` (its `classify` seat is `"class": "gate"`);
- decision 0106 (`:248-257`, the cause table: `ruled` is never retried);
- `git show e667a04c2:scripts/mutants.sh` line 31 and `scripts/test-shards.sh:570-575`, for the
  deferred section's two house-closure facts;
- the review positions of run `fix-round-1093r8-for-issue-1093--daf9cd23`, in
  `.forge/results/c476742a-4e0f-4609-9046-2d8909c2b5f9-positions-{adversarial,correctness}.json`.
  The chief's own result for that run was not on disk in this worktree, and `brokkr inspect`
  needed a grant this seat did not hold, so the chief's notes were read only as the positions and
  the commission quote them.

The entry check's verdicts over the narrowed order (ruling 6) are read from HEAD's code, not run:
**unmeasured** until 0111d's entry tests.
