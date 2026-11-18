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
wall clock until a person re-ran the jobs.

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

What this decision holds fixed: 0071's rules (one home per rule, a closed vocabulary matched
without a wildcard, observe before decide, every guard bound by a test); 0082 (a contract version
is minted only under an accepted decision, so every version below is proposed and none is
minted); no model decides a routing, and a seat's "no code change applies" is prose, not a
classifier; 0106's cause and budget stand; 0051's landing and the landing recipe stand; and 0063:
Linux and macOS only.

## What main holds at HEAD

Read at `e667a04c2`.

### Nothing in the engine or the house recipes reads a forge check

Two searches were run over the tree. The first,
`grep -rnE "gh (pr checks|run (rerun|view|watch|list))|statusCheckRollup|check-runs|check_runs|checks --watch|mergeStateStatus|--required" recipes crates scripts`,
printed only `recipes/preflight/README.md:48` (a table cell saying preflight has no `gh`) and
tool-grant strings in tests (`crates/brokkr-runtime/src/agents/tests.rs:4219`,
`crates/brokkr-runtime/tests/library_data.rs:745`, `crates/brokkr-runtime/tests/capability_launch.rs:5747`,
`crates/brokkr-protocol/src/native_controls/tests.rs:6434`). The second, for any `gh` subcommand in
recipe scripts, recipe data, agents and adapters, printed only the two tool grants
`gh-pr-view` and `gh-run-view` (`adapters/claude.json:69-70`, `adapters/lanetally.json:67-68`)
and the agent that hires them (`agents/verify-reviewer.json:7`). A search of non-test Rust under
`crates/` for `"gh"` as a command, `api.github.com`, `/check-runs` or `/actions/runs` printed no
reader.

So Brokkr reads no check. What a model seat may do is call `gh run view` under its grant and
write prose about it; nothing types what it read.

The house's own runs end before CI. `landing` is `classify` → `verify` → `review` → `ship` →
`done`, and "the rest of the checks main requires stay CI's" (`recipes/landing/README.md:5-11`).
`fast`'s two exec seats, the verifier and the shipper, run with `"network": false`
(`recipes/fast/bundle.json:46`, `:91`). A red check on a house pull request is read by a person or
the merge queue, outside any run; #1113 is one.

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
  choice (`crates/brokkr-core/src/policy/inputs.rs:106-115`): the seat's prose chose the park.

A rule park is written `ruled` with the matched rule id
(`crates/brokkr-runtime/src/engine/cursor.rs:362-369`, `:386`). No writer at HEAD passes
`retryable: true`: `grep -rn "infrastructure(true" crates --include=*.rs`, filtered to non-test
lines, counted 0.

### What the journal records of it

The gate's result and notes, sealed as its seat record, and the decision's seven fields
(`crates/brokkr-core/src/decision.rs:86`), whose `inputs` carry engine- and seat-owned inputs; the
park's reason and its `ruled` cause. Nothing typed names a job, a step, a forge run or a re-run.

### The forge's shape

This shell has no forge client and this seat holds no web capability, so the forge's shape below
is from GitHub's public documentation by URL and was **not read in this session: unmeasured
until slice 0111b records a real payload.**

- A workflow run's jobs, each with `id`, `name`, `status`, `conclusion` and `steps[]` of
  `{number, name, status, conclusion}`:
  <https://docs.github.com/en/rest/actions/workflow-jobs>.
- A job's log: <https://docs.github.com/en/rest/actions/workflow-jobs#download-job-logs-for-a-workflow-run>.
- Re-running failed jobs: `gh run rerun <run-id> --failed`
  (<https://cli.github.com/manual/gh_run_rerun>) and the REST verb
  <https://docs.github.com/en/rest/actions/workflow-runs#re-run-failed-jobs-from-a-workflow-run>.
- Check runs and their conclusions: <https://docs.github.com/en/rest/checks/runs>.

Unmeasured in particular: the closed list of `conclusion` words (the documentation lists
`success`, `failure`, `neutral`, `cancelled`, `skipped`, `timed_out`, `action_required`, and for a
run `startup_failure`); whether `--failed` includes cancelled jobs; how a job log is cut into steps;
and whether the API says who cancelled a job. **A step has no `kind` field**: whether a step is
setup, a service or a test is not a forge fact, so any step role below is recipe data.

## The rulings

Each ruling gives its options, what each costs, refuses and changes, and one recommended option.
The operator may answer "accept all" or pick.

### 1. What reads the checks

**Recommended: (a) a recipe-declared exec gate observes, and the engine classifies.**

- **(a) An exec gate, `ci`, observes; core classifies; the table routes.** The gate runs
  `brokkr driver exec` over a script, as `fast`'s verifier does, with no model. It waits for the
  gated head's checks to settle and writes what the forge said as a typed observation (ruling 2).
  The engine classifies that observation with a pure core function (ruling 3) and the table
  routes on the engine-owned result. Observing and deciding stay apart (0071 ruling 10). Cost: one
  script, one core module, one engine input. It refuses forge-specific network code in the
  engine. It serves an adopter's exec gate and a future house one through the same contract.
- **(b) A built-in engine effect that calls the forge.** One reader for every recipe, but the
  engine gains an HTTP client, a credential path and a forge vocabulary of its own, a second
  effect kind for one forge. Refused for now; the observation contract in (a) is what (b) would
  write, so (b) stays open.
- **(c) A model seat reads `gh run view` and says.** Refused: a model's reading is prose, and
  no model decides a routing.

**Enforcement binding:** the bundle compile refuses a `ci` declaration (ruling 4) on a seat
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
  unmeasured; slice 0111a's test settles it.
- **Its tools.** The interpreter and `gh` resolve through the seat's `PATH`, which 0049 ruling 3
  leaves unpinned under `harness` and `open` (quoted at
  `crates/brokkr-runtime/src/bundle/model_policy_tests.rs:2595-2596`). Pinning them is
  **unmeasured**, so the observer fails closed: it refuses, typed, when `bash`, `gh` or `jq`
  resolves under the gated worktree. It names the repository explicitly (`--repo`), so `gh`
  never takes the repository from the worktree's git remotes. Whether other change-controlled
  files can reach `gh`'s configuration is unmeasured until 0111b, and until then a run whose
  observer cannot show its tool paths outside the worktree refuses.
- **Its word is data.** The engine re-checks the observation's binding itself (ruling 2): it
  compares the head to its own git head and the repository to the pinned one. It never takes
  them from the observer.

### 2. The observation, the classification, and where the journal keeps them

**Recommended: (a) the observation in the gate's `inputs.ci` under `ci-observation/v1`; the
classification as an engine-owned record input on the gate's `transition/decided`, under
`ci-classification/v1`.**

The observation names its binding:

- the forge (`github`, the one word v1 admits);
- the repository, by owner, name and numeric id;
- the pull request's number;
- the gated head's full commit id.

It then lists every check the forge reports for that head, with no filtering. Checks outside
Actions are included:

- a check run carries its id, its app, its name, its `head_sha`, its status and its conclusion
  (<https://docs.github.com/en/rest/checks/runs#list-check-runs-for-a-git-reference>);
- a check from Actions also carries its workflow run's id and attempt, its workflow file's path,
  and its job's id, name and steps;
- a commit status carries its context and state
  (<https://docs.github.com/en/rest/commits/statuses#get-the-combined-status-for-a-specific-reference>).

It is parsed once at the edge, with unknown fields refused (0071 ruling 3). An unknown
conclusion word refuses by name.

**The required set.** **Recommended: (a) a recipe-declared `required_checks` list, pinned by
the compiled bundle outside the change.**

- **(a) Recipe-declared.** `required_checks` lives in ruling 4's file, which the operator's
  compile pins. It must be non-empty, or the compile refuses. Each entry is a check run by app
  and name, or a commit status by context. For this house the audit entry is the check run
  `RustSec dependency audit` from `github-actions` (`.github/workflows/ci.yml:673-674`), not
  "RustSec". Where the observer can also read the forge's own required set for the base branch
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

**What a pass is.** The engine draws `green` only when all of these hold:

- the observation's repository, pull request and head equal the run's pinned repository, the
  pull request the gate names and the engine's own git head;
- each workflow run's attempt equals the current attempt, which is the latest the forge lists
  on a first observation and exactly the attempt ruling 6's accepted re-run made on a second;
- every entry of `required_checks` is matched by exactly one settled check;
- each matched check's conclusion is `success`, or each matched status's state is `success`.

Nothing short of that is a pass. Where the observation cannot pass and the classifier has no
failure to type, it refuses with its own closed, typed reason, before and after any re-run:

| Refusal | When |
|---|---|
| `empty` | the forge reports no check for the head |
| `missing-required` | a required entry matches no check; the refusal names the entry |
| `ambiguous-required` | a required entry matches more than one check in the current attempt |
| `unrelated` | the repository, the pull request, the head or the attempt differs from the binding |
| `not-settled` | a required check is still queued or in progress when the observer's own polling bound ends, short of the seat's deadline |
| `non-success` | a required check concluded `neutral`, `skipped`, `stale` or `action_required`, words the classifier types as no failure; the refusal names the check and the word |
| `required-set-disagrees` | the forge requires a check the pinned list lacks |

A refusal never re-runs and never passes. The run parks `ruled`, with the rule's id and the
refusal's word. On a second observation two more bindings hold. Every required check outside
the re-run's scope keeps its first observation's identity (ruling 7) and its `success`; a
higher attempt alone is not accepted. And the second observation must be bound to the exact
accepted re-run. How the forge presents an unaffected job in a new attempt, with the same check
run or a copy, is unmeasured until 0111b.

The gate's word is bound to the engine's class: `pass` is admitted only on `green`, and `fail`
with a `green` class is refused. A refused word is never rewritten (the same binding 0109
ruling 2 gives a verdict).

The classification is engine-written: an `Owner::Engine` record input, as `reviewed_heads` is
(`crates/brokkr-core/src/policy/inputs.rs:116`), so a seat may neither declare nor overwrite it,
as the engine-owned strategy is held (`:87-95`). It rides in the decision's `inputs` (`crates/brokkr-core/src/decision.rs:86`), so
the journal holds the observation, the class the engine drew from it, and the rule that routed
on it, in the order they happened, with no new event.

- **(b) A new `ci/classified` event.** Moves the event envelope, every manifest and every fold
  arm for a fact an input can carry. Refused, as 0106 ruling 1 refused `run/park-caused`.
- **(c) The excerpt kept outside the journal by digest.** Smaller rows, but the classification
  could not be replayed from the journal alone. Refused; the 8 KiB bound is the cost control.

### 3. The classifier

**Recommended: (a) one pure function over the observation, the declared list and the change's
paths, with a closed result, failing closed.**

`classify(observation, faults, changed_paths) -> Classification` lives in `brokkr-core`, reads
no clock, environment or network (0071 ruling 1), and is total. `changed_paths` is the
engine's, read from git as 0039's docs-only flag is
(`crates/brokkr-core/src/policy/inputs.rs:81-84`). In order:

1. The observation passes ruling 2's pass rule: **`green`**. One that cannot pass and holds no
   failed, timed-out, unstarted or cancelled required check takes ruling 2's refusal.
2. Any changed path matches the list's `infrastructure_paths`: **`change`**, reason
   `touches-infrastructure`. Job and step names are written in the workflow files; a change that
   edits them could rename a failing step into a setup step. A change that touches none is run
   by the base's names.
3. Each failed job is typed on its own:
   - conclusion `startup_failure`: runner, kind **`startup-failure`** — the forge says the job
     never started a step;
   - conclusion `failure`, the job not a security job (ruling 5), its first failed step named in
     the list's `setup_steps`, and that step's excerpt holding one of the list's patterns for
     that step byte for byte: runner, kind **`declared-pattern`** with the pattern's id;
   - anything else, `timed_out` included (a hang may be the change's): **untyped**.
4. Every failed job runner, and at least one failed: **`runner-fault`**, naming each job, its
   step, its kind and pattern id, and the re-run scope: the failed jobs and any cancelled job of
   the same workflow runs.
5. Otherwise **`change`**, with a closed reason: `untyped-failure`, `security-job`,
   `cancelled-only` (cancelled jobs and no failed one: who cancelled is unmeasured), or
   `touches-infrastructure`.

One untyped job makes the whole observation `change`: a re-run would hide nothing then, and
would not help. A failure the classifier cannot type is the change's, as today.

The adopter's two cases read differently, and the decision says so. The port bind fails in a
setup step and matches a declared `address already in use` pattern: `runner-fault`. The
timing-sensitive test fails in a test step: **`change`**, as today. A flaky test is not typed as
the runner's by anything the forge states, and no pattern may type a test step's failure, so this
decision does not recover it. Quarantining a named flaky test is a different decision.

- **(b) Step role alone, no pattern.** Cheaper to declare, but a setup step that installs the
  change's dependencies fails for the change's reasons too. Refused.
- **(c) Pattern alone, any step.** Lets a test's own output type its failure as the runner's.
  Refused.
- **(d) Fail open on an unread conclusion or a missing excerpt.** Refused by 0004.

**Enforcement binding:** a table test over recorded observations, one per arm and reason; and
`classify` matches the conclusion enum without a wildcard (0071 ruling 2).

### 4. The runner-fault list: its home and who declares it

**Recommended: (a) recipe data, `ci-checks.json` beside the policy, under `ci-checks/v1`,
compiled into the bundle and pinned with it.**

```json
{
  "schema": "ci-checks/v1",
  "repository": {"owner": "example", "name": "app", "id": 1},
  "required_checks": [
    {"check_run": {"app": "github-actions", "name": "test"}},
    {"check_run": {"app": "github-actions", "name": "RustSec dependency audit"}}
  ],
  "infrastructure_paths": [".github/workflows/**", "compose.yaml"],
  "setup_steps": ["Start services", "Set up job"],
  "audit_checks": [{"check_run": {"app": "github-actions", "name": "RustSec dependency audit"}}],
  "patterns": [
    {"id": "port-in-use", "steps": ["Start services"], "literal": "address already in use"}
  ]
}
```

A pattern is a literal: 1 to 200 printable ASCII bytes, matched as an exact, case-sensitive byte
substring. No regex, glob or wildcard is admitted, so matching is linear and the same on every
host. `id` is kebab-case and unique; every step a pattern names must be in `setup_steps`.
`required_checks` and `infrastructure_paths` must be non-empty, and `audit_checks` must be
present, even if empty, with every entry also in `required_checks`. The compile refuses a list
that leaves any of them to a default. `repository` pins the binding ruling 2 checks.

The operator declares it, as any recipe or `extends` overlay (0017). No seat writes it: the
run reads the list its compiled bundle pinned at start, never the change's tree, so a branch
that edits `ci-checks.json` changes the next run's list, by the operator's compile, not its own.

- **(b) Host data under `forge.host`.** One list for every recipe; but runner faults are a
  property of a workflow, not a machine. Refused.
- **(c) Engine defaults.** A second home for a fact the recipe owns (0071 ruling 5). Refused.

### 5. Security and audit jobs

**Recommended: (a) a job named in `audit_checks` is re-run only when the forge's own conclusion
is `startup_failure`; a pattern never types it.**

This cannot launder a real advisory. A `startup_failure` means the forge ran none of the job's
steps, so no advisory was evaluated; the re-run is the first evaluation, and what it reports
stands. A job that ran and failed, an advisory among its causes, is untyped, and the run is
`change` with reason `security-job`. A rename to escape the list is a workflow edit, which is
`touches-infrastructure`. Nothing here skips a check or marks one passed: the only forge write
in this decision is the re-run request.

#1113 reads under this rule as a failed step inside the RustSec job: not re-run by this
decision. It stays the operator's, as today.

- **(b) A security job's reporting step may be typed by pattern when its audit step concluded
  `success`.** It would cover #1113's shape, but the step split is the workflow's to declare,
  and it gives the list a way to type a failure in a security job at all. Refused here; a later
  decision may weigh it with #1113's measured payload.
- **(c) Security jobs treated as any other job.** Refused.

### 6. The re-run

**Recommended: (a) one request, scoped to the failed and cancelled jobs, by an exec seat holding a
write token the observer never holds.**

- **Scope.** Once per gated head. The seat asks the forge to re-run the failed jobs of each run
  the classification names, as `gh run rerun <run-id> --failed` does, and re-runs a cancelled job
  in that scope by its own id if `--failed` proves not to include it (unmeasured). It re-runs no
  passed job and no other run.
- **The effect.** An exec seat, `ci-rerun`, with no model, whose commission is the run ids the
  engine's classification names. It reads no log and classifies nothing. Its result is
  `requested`, with the forge's acceptance as `inputs`; a refusal by the forge is `refused`, which
  the table parks `ruled`.
- **Its credential.** A sealed secret binding (0012) declared in the seat's `secrets` and
  resolved by the runner, never in argv: `gh` reads a token from its environment, an env-only
  consumer 0012's compile lint already admits (`crates/brokkr-runtime/src/bundle.rs:4149-4156`).
  The observer binds a read token and `ci-rerun` a separate write token, so the seat that reads
  logs never holds the right to change the forge. The token scopes a forge needs are unmeasured.
- **Its egress.** Both seats run on the exec adapter, declared `egress: contracted`
  (`adapters/exec.json:4`); 0036 ruling 4 keys a binding off that class against the bundle's
  minimum. Both need `"network": true`, unlike `fast`'s exec seats.
- **Its bounded wait.** The `ci` gate, not the re-run seat, waits: it polls at a fixed interval
  until every job of the head's runs is settled, within its own `limits.timeout_seconds`. A
  timeout is 0006's, never a classification.
- **What the journal records.** The first observation and its classification (ruling 2); the
  re-run seat's commission, naming the run and job ids, and its result; the second observation,
  whose run attempt must exceed the first's, and its classification.

- **(b) The observer re-runs as it observes.** One seat, but the seat that reads logs would hold
  a write token and decide when to use it. Refused.
- **(c) Re-run the whole workflow.** Re-runs passed jobs and spends runner time for nothing.
  Refused.

### 7. Each outcome, and what "identical" means

**Recommended: (a) a pass proceeds; the same fault parks typed; a different runner fault also
parks typed; a change failure is the change's.**

A fault's identity is the set of `(workflow, job, failed step, kind, pattern id)` tuples. After
the one re-run:

- **`green`**: the gate's `pass`; the run proceeds as if it had passed the first time. The
  journal still names the re-run.
- **`runner-fault`, the same identity**: parks, typed (ruling 8).
- **`runner-fault`, another identity**: parks, typed. "Once" is once per gated head.
- **`change`**: the change's, routed by the table exactly as a first-time red gate is.

"Once per gated head" is an engine-owned count, `ci_reruns`, of `ci-rerun` results recorded
since the gated head was entered. It is not `visits_ci-rerun`, which counts per run (0022): a
head the implementer moves earns its own re-run. The table reads it as `ci_reruns_gte`; a seat
cannot write it. A table in the recipe declares the arms, first match winning:

| Rule | From | Result | When | Next |
|---|---|---|---|---|
| `CI-PASS` | `ci` | `pass` | | the recipe's next phase |
| `CI-RUNNER-AGAIN` | `ci` | `fail` | `ci_fault_in: [runner-fault]`, `ci_reruns_gte: 1` | park |
| `CI-RUNNER` | `ci` | `fail` | `ci_fault_in: [runner-fault]` | `ci-rerun` |
| `CI-RERUN-REQUESTED` | `ci-rerun` | `requested` | | `ci` |
| `CI-FAIL` | `ci` | `fail` | | `implement`, as `VERIFY-FAIL` |

`ci_fault` is an engine-owned choice over `green`, `runner-fault` and `change`, beside the
seat-owned `blocked_by` (`crates/brokkr-core/src/policy/inputs.rs:106-115`). Phase-machine v2's
`when` already admits any named condition the engine defines
(`contracts/phase-machine.v2.schema.json:71-77`), so no table contract version moves.

- **(b) A different runner fault earns a second re-run.** Two different faults in a row are
  more likely one sick runner pool than luck; a bound of one is the point. Refused.

### 8. The park's cause

**Recommended: (b) `infrastructure` with a closed `kind`, under `park-cause/v2`.**

- **(a) A new `ci-infrastructure` cause.** A fifth written word. Every match over `ParkCause`
  gains an arm (`crates/brokkr-core/src/park.rs:234`, `:264`, `:276`), 0106 ruling 1's four-word
  table becomes five, and 0106d's "only an `infrastructure` park spends" leaves it out by
  construction. Cost: a wider change to an accepted ruling's table. It refuses nothing (b) admits.
- **(b) `infrastructure` with `kind`.** Recommended. `kind` is `engine` (every writer today) or
  `ci-runner`, the latter with `faults` (the identity tuples) and `reruns: 1`. A v1 cause with no
  `kind` reads `engine`. The cause stays four words, the readouts already paint `infrastructure`,
  and 0106d's rule reads unchanged: it spends only where `retryable` is true (ruling 9). Cost:
  one closed enum inside one variant, matched without a wildcard.

The writer: the engine's table-park arm (`crates/brokkr-runtime/src/engine/cursor.rs:362-369`)
gains one case. A park whose decision's inputs carry a `runner-fault` classification writes
`infrastructure { kind: ci-runner, retryable: false }` instead of `ruled`; the rule id stays in
the decision beside it, where 0106 says a rule park's rule already lives.

On acceptance, 0106's header and this one's gain the pointer pair for ruling 1's cause gaining
`kind`, under the errata rule for back-pointers.

### 9. How a CI park reads in 0106d's budget

**Recommended: (a) a `ci-runner` park is `retryable: false` under this decision, and spends
nothing.**

0106 ruling 4 asks for positive evidence that the last attempt never executed. Here the last
attempt is the `ci` gate's, and it executed: it read the forge and returned a typed result. The
forge's statement that the change's checks never ran is a fact about CI, not about the attempt,
and a pattern match is evidence that a step ran. So the rule, read literally, gives `false`.

What positive non-execution evidence would be for CI: the forge's own `startup_failure` on every
failed job of both observations, so that no step of the change's checks ran either time. Even
then a budget retry, as 0106d builds it, re-enters the parked phase as a new effect; it would
observe the same settled runs and park again, spending the budget for nothing. Admitting that
evidence needs the retry to reach `ci-rerun`, and that is the budget's own decision, not this
one's.

- **(b) `retryable: true` on a twice-`startup-failure` park.** Spends a retry that cannot help,
  as above. Refused here.

**Enforcement binding:** `a_ci_runner_park_is_never_retryable` and 0106d's
`only_an_infrastructure_park_spends_the_budget`, extended with a `ci-runner` park that spends
nothing.

### 10. The readouts

**Recommended: (a) one derivation in `brokkr-view`, painted by `inspect` and the fleet.**

`inspect` names each observation's class, the re-run's request with its run and job ids, and the
outcome, from one view model (0071 ruling 7). The fleet paints the park's cause through
`ParkCause::line` (`crates/brokkr-core/src/park.rs:275`), which the TUI and `inspect` already
paint (`crates/brokkr-cli/src/tui/columns.rs:375`, `crates/brokkr-cli/src/render/inspect.rs:36`;
the fleet's row field is `crates/brokkr-view/src/fleet.rs:339`). It reads
`infrastructure (ci-runner, not retryable): …` with the faults' job and pattern ids.

## What stays out

- No model decides: the gate and the re-run seat are exec seats, the classifier is core.
- No required check is skipped, marked passed or written, other than by a re-run request.
- No refusal is weakened; `retryable`'s positive-evidence rule stands as 0106 wrote it.
- No Windows code (0063).
- #1113's own fix.
- The dispatcher's retry budget, beyond ruling 9.
- Flaky-test quarantine (ruling 3).
- A forge other than GitHub: `forge` is a closed enum of one word in v1.

## Consequences

A runner fault the operator declared costs one re-run and no seat attempt, and a second one
parks with a cause a script can read. Everything the list does not name is the change's, as
today, so a recipe with no list behaves exactly as now. The cost is an exec observer per recipe
that wants it, two tokens, and a list an operator keeps; a pattern that matches too much is the
operator's to narrow. Four contract versions are proposed: `ci-observation/v1`,
`ci-classification/v1`, `ci-checks/v1` and `park-cause/v2`; each is minted only when its slice
lands under this decision's acceptance (0082).

## The slices

Smallest first; each lands on its own.

1. **0111a — the classifier and its record over a recorded payload, with no re-run.** Files: a new
   `crates/brokkr-core/src/ci.rs` with `ci/observation.rs`, `ci/faults.rs`, `ci/classify.rs` and
   `ci/tests.rs`; `crates/brokkr-core/src/lib.rs`; `crates/brokkr-core/src/policy/inputs.rs` (the
   `ci` record and `ci_fault` choice, engine-owned); `crates/brokkr-runtime/src/bundle.rs` (load
   and pin `ci-checks.json`, refuse it on a non-exec seat);
   `crates/brokkr-runtime/src/engine/decide.rs` (classify at decide and record the input);
   `crates/brokkr-view/src/` (the CI view model) and `crates/brokkr-cli/src/render/inspect.rs`.
   About 450 production and 450 test lines. Contracts: `ci-observation/v1`, `ci-checks/v1`,
   `ci-classification/v1`. Tests: one per arm and reason of ruling 3, over recorded observations
   built by a shared builder; `a_pattern_in_a_test_step_is_the_changes`;
   `a_security_job_failure_is_never_typed_by_pattern`;
   `a_startup_failure_in_a_security_job_is_the_runners`;
   `a_change_to_a_workflow_file_is_the_changes`; `one_untyped_job_makes_the_whole_change`;
   `a_pattern_beyond_the_excerpt_is_not_found`; `an_unknown_conclusion_refuses_by_name`;
   `a_stale_head_observation_does_not_read`; `a_pass_with_a_failed_job_is_refused`;
   `a_seat_cannot_write_the_classification`; `a_ci_list_on_a_model_seat_refuses_to_compile`;
   `an_empty_infrastructure_list_refuses_to_compile`; `inspect_names_the_class`. The pass's
   refusals, one test each, every one asserting the refusal's variant and that the class is not
   `green`: `an_observation_with_no_check_refuses_as_empty`;
   `a_required_audit_missing_from_the_observation_refuses_as_missing_required`;
   `an_observation_of_another_head_repository_pull_request_or_attempt_refuses_as_unrelated`;
   `a_skipped_or_neutral_required_check_refuses_as_non_success`;
   `a_required_check_still_in_progress_refuses_as_not_settled`;
   `two_matches_for_one_required_check_refuse_as_ambiguous`;
   `a_forge_required_check_the_list_lacks_refuses`; `an_empty_required_list_refuses_to_compile`;
   `a_ci_seat_whose_layer_is_inside_the_worktree_refuses_at_start`. No table routes
   on it yet, so the next sweep can count what a re-run would have saved.
2. **0111b — a GitHub observer script.** Files: a house observer script and its shellcheck
   clean; a test that runs it against a stub `gh` on the path and holds its output to
   `ci-observation/v1`; one recorded real payload, measured by the operator, which settles this
   decision's unmeasured forge facts. About 150 script and 200 test lines. No contract change.
3. **0111c — the re-run and its routing.** Files: `crates/brokkr-core/src/policy/inputs.rs`
   (`ci_reruns`), `crates/brokkr-runtime/src/engine/decide.rs`, a re-run script, and a recipe
   overlay declaring ruling 7's table. About 200 production and 300 test lines. No contract
   change. Tests: `a_runner_fault_routes_to_one_rerun`; `a_green_after_the_rerun_passes`;
   `the_same_fault_after_the_rerun_parks`; `a_different_runner_fault_after_the_rerun_parks`;
   `a_change_failure_after_the_rerun_returns_to_implement`;
   `a_moved_head_earns_its_own_rerun`; `the_observer_never_holds_the_write_token`;
   `a_forge_refusal_parks_ruled`. The second observation's refusals:
   `a_second_observation_on_an_attempt_the_rerun_did_not_make_refuses_as_unrelated`;
   `an_unaffected_required_check_that_changed_after_the_rerun_refuses`;
   `a_required_check_missing_after_the_rerun_refuses_as_missing_required`.
4. **0111d — the typed park.** Files: `crates/brokkr-core/src/park.rs` and its `parse` and
   `forms` modules, `crates/brokkr-runtime/src/engine/cursor.rs`, and the view and CLI tests that
   paint the cause. About 120 production and 200 test lines. Contract: `park-cause/v2`. Tests:
   `a_v1_infrastructure_cause_reads_kind_engine`; `a_ci_runner_park_is_never_retryable`;
   `a_rerun_park_writes_the_ci_runner_cause_and_keeps_the_rule_in_the_decision`;
   `the_fleet_paints_the_ci_runner_cause`; and, when 0106d lands, its
   `only_an_infrastructure_park_spends_the_budget` extended with a `ci-runner` park.

## Evidence

Commands run in this seat at `e667a04c2`, each named above where it grounds a claim: the two
`grep` inventories in [What main holds at HEAD](#what-main-holds-at-head); the count of
`infrastructure(true` writers; and reads of `crates/brokkr-core/src/park.rs`,
`crates/brokkr-core/src/policy/inputs.rs`, `crates/brokkr-runtime/src/engine/cursor.rs`,
`recipes/fast/policy.json`, `recipes/fast/bundle.json`, `recipes/landing/README.md`,
`adapters/exec.json` and `contracts/phase-machine.v2.schema.json`. The forge's shape was not read.
