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

### 3. What evidence types a failure as the runner's, and the classifier

**This is the operator's first choice. Recommended: (a) forge-proven non-execution only.**

The first draft typed a failure as the runner's when an allow-listed setup step's excerpt held
a declared literal, and the step's workflow was untouched. That is not enough. An unchanged
setup step can still run the pull request's code: a service bootstrap that starts the
repository's own compose file or script, or HEAD's own checkout-local action,
`uses: ./.github/actions/setup-cargo-audit` (`.github/workflows/ci.yml:681-689`), whose code comes
from the checkout. A source-only change can then make that step print the declared literal, and
a real code failure is laundered as a runner fault. Untouched declared paths cannot bound what a
step executes. So the evidence is the operator's choice, stated fairly:

- **(a) Recommended: forge-proven non-execution only.** A failure is the runner's only when the
  forge's own job-level conclusion proves the job's steps never ran (ruling 5's proven set). No
  log pattern types anything, and the list carries no patterns. A change cannot produce a
  conclusion that says its code never ran without its code never running.
  - **Cost, said plainly:** the adopter's port-bind case and their timing-sensitive test are the
    change's, as they are today. Both still cost a seat attempt and the wall clock until a person
    re-runs. Their figures, three parks in one week, stay theirs. This decision recovers neither
    under (a). It recovers only jobs that never started, and until 0111b measures the proven set,
    none.
- **(b) Declared runner-owned evidence.** A literal types a failure only from a step whose
  provenance is runner-owned and positively known. An example is the runner's own
  service-container start for the workflow's `services:` block
  (<https://docs.github.com/en/actions/using-containerized-services/about-service-containers>).
  It qualifies only when all of these hold:
  - the block is unchanged by the diff over ruling 2's pinned range;
  - its images are named by digest;
  - it mounts nothing from the workspace;
  - no `steps:` entry in the workflow at the gated head carries that step's name, so the step can
    only be the runner's.

  A step that runs repository code is never runner-owned, whatever its name.
  - **Cost:** what each forge exposes as runner-owned is partly unmeasured. Unmeasured: whether
    the jobs API names the runner's container start as its own step, and by what name; whether a
    user step can take the same name in the API's listing; how a job log is cut into steps; and
    whether the forge masks every secret in that step's log. The adopter's port bind was a compose
    service, which a step starts from the repository's compose file. That step runs repository
    code, so even (b) does not recover their case unless the service moves to `services:`.
- **(c) Refused: any pattern over a step that runs repository code.** That is the first draft's
  rule. It lets a change type its own failure as the runner's.

**Timing-sensitive tests are not runner faults under any option.** A flaky test fails in a step
that runs the change's code, and nothing the forge states types it. Flaky-test handling is out of
scope; it is a follow-up for a decision of its own.

**The classifier.** One pure function over the observation, the declared list and the change's
paths, with a closed result, failing closed.

`classify(observation, faults, changed_paths) -> Classification` lives in `brokkr-core`, reads
no clock, environment or network (0071 ruling 1), and is total. `changed_paths` is the
engine's, read from git as 0039's docs-only flag is
(`crates/brokkr-core/src/policy/inputs.rs:81-84`). In order:

1. The observation passes ruling 2's pass rule: **`green`**. One that cannot pass and holds no
   failed, timed-out, unstarted or cancelled required check takes ruling 2's refusal.
2. Any changed path matches the list's `infrastructure_paths`: **`change`**, reason
   `touches-infrastructure`. This guards structure only: job names, the `needs:` graph ruling 6
   reads, and under (b) the `services:` block. It bounds nothing a step executes.
3. Each failed job is typed on its own:
   - a job-level conclusion in ruling 5's proven non-execution set: runner, kind
     **`not-started`**. That set is empty until 0111b measures `startup_failure` on a job;
   - under (b) only: conclusion `failure`, the job not an audit job (ruling 5), its first failed
     step runner-owned as (b) defines, and that step's excerpt holding one of the list's literals
     for that step: runner, kind **`runner-owned-pattern`** with the pattern's id. Under (a) this
     arm does not exist;
   - anything else, `timed_out` included (a hang may be the change's): **untyped**.
4. Every failed job runner, and at least one failed: **`runner-fault`**, naming each job, its
   step, its kind and pattern id, and the re-run scope: exactly those jobs by id, each with its
   attempt (ruling 6). A cancelled job is never in the scope.
5. Otherwise **`change`**, with a closed reason: `untyped-failure`, `security-job`,
   `cancelled` (any required job cancelled: who cancelled is unmeasured, though on a pull request
   a newer push cancels the older run, `.github/workflows/ci.yml:24-26`),
   `touches-infrastructure`, or `scope-not-enforceable` (ruling 6).

One untyped job makes the whole observation `change`: a re-run would hide nothing then, and
would not help. A failure the classifier cannot type is the change's, as today.

Also refused, under every option: a step's role alone, with no pattern, since a setup step that
installs the change's dependencies fails for the change's reasons too; and failing open on an
unread conclusion or a missing excerpt (0004).

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
  "infrastructure_paths": [".github/workflows/**", ".github/actions/**"],
  "audit_checks": [{"check_run": {"app": "github-actions", "name": "RustSec dependency audit"}}]
}
```

That is the whole list under ruling 3 (a). Under (b) only, two more keys are admitted:

```json
{
  "runner_owned_steps": [{"workflow": ".github/workflows/ci.yml", "job": "test", "step": "Initialize containers"}],
  "patterns": [
    {"id": "port-in-use", "steps": ["Initialize containers"], "literal": "address already in use"}
  ]
}
```

The step name above is illustrative and unmeasured (ruling 3 (b)). Under (a) the compile refuses
either key. A pattern is a literal: 1 to 200 printable ASCII bytes, matched as an exact,
case-sensitive byte substring. No regex, glob or wildcard is admitted, so matching is linear and
the same on every host. `id` is kebab-case and unique; every step a pattern names must be in
`runner_owned_steps`.
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

**Recommended: (a) a job named in `audit_checks` is re-run only on a job-level forge conclusion
that proves its steps never ran; a pattern never types it, and nothing else brings it into a
scope.**

**Which conclusions prove non-execution.** The forge documents a job's `conclusion`
(<https://docs.github.com/en/rest/actions/workflow-jobs#get-a-job-for-a-workflow-run>), a check
run's (<https://docs.github.com/en/rest/checks/runs#get-a-check-run>) and a run's
(<https://docs.github.com/en/rest/actions/workflow-runs#get-a-workflow-run>). This seat could not
read them, so each candidate below is **unmeasured**:

| Conclusion | Proves the job's steps never ran? |
|---|---|
| `startup_failure` | the candidate. Whether the forge reports it on a **job**, and not only on a run or check suite, is unmeasured |
| `skipped` | the job's `if:` or a failed `needs:` kept it from starting; unmeasured, and it is no failure, so ruling 2 refuses it rather than re-running it |
| `cancelled` | no. A job can be cancelled mid-step |
| `failure`, `timed_out` | no. A step ran |
| `neutral`, `action_required`, `stale` | no. Ruling 2 refuses them |

Until slice 0111b records a payload showing a job-level `startup_failure` with every step
unstarted, the proven set is **empty**, and no audit job enters any scope. That fails closed. A
run-level status or conclusion proves nothing about one job: a run's `startup_failure` says the
run did not start, and it is never read as a fact about any job inside it.

This cannot launder a real advisory:

- an advisory is evaluated only by a step that runs;
- a job none of whose steps ran evaluated none, so its re-run is the first evaluation, and what
  that reports stands;
- a job that ran any step and failed, an advisory among its causes, is untyped, and the run is
  `change` with reason `security-job`;
- an audit job never enters a scope as another job's dependent (ruling 6) unless it has that
  same proof itself;
- and a pass needs the audit's own `success` in the re-run's attempt (ruling 2), so a re-run
  cannot drop it either.

A rename to escape the list is a workflow edit, which is `touches-infrastructure`. Nothing here
skips a check or marks one passed: the only forge write in this decision is the re-run request.

**#1113 is never re-run under this rule.** RustSec's `update-a-check-run` "Not Found" is a failed
step inside the `RustSec dependency audit` job (`.github/workflows/ci.yml:673-695`): the job ran.
It is not a non-execution, so it stays a person's re-run, as today. This decision designs no fix
for it.

- **(b) A security job's reporting step may be typed by pattern when its audit step concluded
  `success`.** It would cover #1113's shape, but the step split is the workflow's to declare,
  and it gives the list a way to type a failure in a security job at all. Refused here; a later
  decision may weigh it with #1113's measured payload.
- **(c) Security jobs treated as any other job.** Refused.

### 6. The re-run

**Recommended: (a) once per gated head, by named job ids the classification proved the runner's,
each validated against the exact classified attempt before the write, by an exec seat holding
a write token the observer never holds.**

- **Scope: named job ids, never a run's `--failed` set.** The write is the forge's per-job verb,
  `POST /repos/{owner}/{repo}/actions/jobs/{job_id}/rerun`
  (<https://docs.github.com/en/rest/actions/workflow-runs#re-run-a-job-from-a-workflow-run>), one
  request per target. `gh run rerun --failed` is refused, for two reasons. It also re-runs
  cancelled jobs, a cancelled audit job among them, with no `startup_failure`
  (<https://cli.github.com/manual/gh_run_rerun>; as a statement of what `--failed` selects, this
  is unmeasured here). And it lets the forge choose the set at write time, keyed by run id, so a
  concurrent attempt can change what it selects after the engine classified.
- **Dependents are in the scope.** The forge's per-job verb is documented to re-run the job
  "and its dependent jobs"; that wording is unmeasured until 0111b. So a target's scope is the
  job plus every job that `needs:` it, directly or transitively. The engine reads that graph from
  the workflow file at the gated head, which ruling 3's `touches-infrastructure` holds to the
  base's bytes. Every job in that closure must be one of:
  - a classified target;
  - a non-audit job whose conclusion is `skipped` because a target failed.

  A closure holding a passed job, a cancelled job, an unclassified failure, or an audit job
  without ruling 5's proof is **wider than classified**, and the classification is `change` with
  reason `scope-not-enforceable`. It never re-runs.
- **Validated before the write.** Immediately before each request, the seat re-reads the target
  job and its run. The run's attempt must equal the classified attempt, the job's conclusion the
  classified one, and the run must not be in progress. Otherwise the seat sends nothing and
  returns `refused` with reason `stale-scope`. A target the forge reports unknown returns
  `refused` with `unknown-target`.
- **Checked after the write.** The second observation must show a new attempt for exactly the
  authorized closure. A job outside it with a new run is `scope-widened`, and the run parks for
  reconciliation (ruling 7). It never passes. A person's concurrent re-run in the window between
  the re-read and the write is therefore detected, never passed over.
- **The effect.** An exec seat, `ci-rerun`, with no model, whose commission is the engine's
  typed list of `(repository, run id, run attempt, job id)` targets with their closure. It reads
  no log and classifies nothing. Its result is
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
- **One re-run is one remote request, durably.** The forge may accept a request while the exec
  dies before recording it, so a result count cannot prove how many requests were sent. Three
  rules close this:
  - *Intent before the write.* The engine dispatches `ci-rerun` once per target, so one effect
    is one remote request. Its `effect/started`, naming the target, is durable before the driver
    spawns (`crates/brokkr-runtime/src/engine/dispatch.rs:159-165`). That row is the per-target
    intent, and it counts as spent whatever follows.
  - *No re-send.* An attempt that ends with no result is indeterminate. So is one a restarted
    engine finds in flight (0006's 2026-10-11 addendum, ruling 2,
    `docs/decisions/0006-bounded-attempts-and-deadlines.md:267-276`;
    `crates/brokkr-runtime/src/engine/broker/fold.rs:337-361`). The engine never retries it. An
    operator's retry re-requests the effect (`crates/brokkr-core/src/fold/operator.rs:38-42`): a
    new effect, not a replay. So every `ci-rerun` attempt reads before it writes. A target whose
    journal already holds an intent is **never sent again**. The seat only reconciles:
    - if the forge shows that job in a new attempt, it returns `already-accepted` with that
      attempt;
    - otherwise it returns `uncertain`.
  - *Park on uncertainty.* An indeterminate `ci-rerun` end parks as HEAD parks a closed
    indeterminate attempt today (`infrastructure`, not retryable,
    `crates/brokkr-runtime/src/engine/cursor.rs:364-366`). An `uncertain` result parks by rule,
    with the cause kind `ci-rerun-uncertain` (ruling 8). So does a second observation where some
    targets show a new attempt and others do not, which is partial acceptance across runs. The
    operator reconciles by hand. Brokkr does not guess which request the forge kept.
- **What the journal records.** The first observation and its classification (ruling 2); each
  target's intent (`effect/started`) and its result, `requested`, `already-accepted`,
  `refused` or `uncertain`, with the forge's answer; the second observation, bound to exactly
  the attempts those results name, and its classification.

- **(b) The observer re-runs as it observes.** One seat, but the seat that reads logs would hold
  a write token and decide when to use it. Refused.
- **(c) Re-run the whole workflow.** Re-runs passed jobs and spends runner time for nothing.
  Refused.
- **(d) `gh run rerun <run-id> --failed`, as the first draft had it.** One call per run, but
  the forge picks the set at write time, cancelled jobs included, so the scope can widen after
  classification. Refused.

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

"Once per gated head" is an engine-owned count, `ci_reruns`, of `ci-rerun` intents
(`effect/started`, whatever their end) recorded anywhere in the run's journal for the gated
head's commit id. It is not counted since the phase was entered, and it is not
`visits_ci-rerun`, which counts per run (0022). A head the implementer moves to earns its own
re-run. A return to an already-seen commit id, by a revert or a reset, earns none: that head
had its one. The table reads it as `ci_reruns_gte`; a seat
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
- **(b) `infrastructure` with `kind`.** Recommended. `kind` is one of:
  - `engine`, every writer today;
  - `ci-runner`, with `faults` (the identity tuples) and `reruns: 1`;
  - `ci-rerun-uncertain`, with the targets whose acceptance is unknown (ruling 6). A v1 cause with no
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

Under ruling 3 (a), a job the forge proves never started costs one re-run and no model
diagnosis attempt, and a second one parks with a cause a script can read. Every other failure
is the change's, as today. That includes the adopter's port bind and timing-sensitive test, so
under (a) this decision does not recover the figures #1093 reports, and says so. Under (b) a
runner-owned service start is recovered as well, at the cost of measuring what the forge
exposes. A recipe with no list behaves exactly as now. The cost is an exec observer per recipe
that wants it, two tokens, and a list an operator keeps. Four contract versions are proposed: `ci-observation/v1`,
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
   built by a shared builder. Ruling 3's evidence, under (a):
   `no_log_pattern_types_a_failure_as_the_runners`;
   `a_patterns_key_under_option_a_refuses_to_compile`;
   `a_declared_literal_printed_by_a_checkout_local_action_is_the_changes`;
   `a_declared_literal_printed_by_a_service_bootstrap_script_is_the_changes`;
   `a_timing_sensitive_test_failure_is_the_changes`. Under (b), if ruled:
   `a_pattern_in_a_step_named_in_the_workflows_steps_is_the_changes`;
   `a_pattern_in_a_services_block_the_diff_changed_is_the_changes`;
   `a_service_mounting_the_workspace_is_not_runner_owned`;
   `a_pattern_in_a_test_step_is_the_changes`. The rest:
   `a_security_job_failure_is_never_typed_by_pattern`;
   `an_audit_job_is_the_runners_only_on_a_proven_non_execution_conclusion`;
   `with_the_proven_set_empty_a_startup_failure_is_the_changes`;
   `a_cancelled_job_is_the_changes`;
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
   `a_forge_refusal_parks_ruled`. The scope:
   `the_rerun_names_job_ids_and_never_a_runs_failed_set`;
   `a_cancelled_audit_job_beside_a_runner_fault_is_never_in_the_scope`;
   `a_dependent_audit_job_without_proof_makes_the_scope_not_enforceable`;
   `a_dependent_that_passed_makes_the_scope_not_enforceable`;
   `a_target_whose_run_attempt_moved_since_classification_sends_nothing_as_stale_scope`;
   `an_unknown_target_sends_nothing`;
   `a_job_outside_the_closure_with_a_new_attempt_parks_as_scope_widened`;
   `an_audit_job_with_a_failed_step_is_never_rerun` (#1113's shape);
   `a_run_level_startup_failure_proves_nothing_about_an_audit_job`. Durability:
   `a_lost_receipt_after_the_forge_accepted_is_reconciled_and_never_resent` (the forge stub
   accepts, the driver exits with no result; the operator's retry reads `already-accepted` and the
   stub counts one request);
   `an_indeterminate_rerun_with_no_new_attempt_parks_uncertain_and_sends_nothing`;
   `partial_acceptance_across_two_runs_parks_ci_rerun_uncertain_naming_the_unknown_target`;
   `a_return_to_an_already_rerun_head_earns_no_second_rerun`;
   `the_intent_is_durable_before_the_driver_spawns`. The second observation's
   refusals:
   `a_second_observation_on_an_attempt_the_rerun_did_not_make_refuses_as_unrelated`;
   `an_unaffected_required_check_that_changed_after_the_rerun_refuses`;
   `a_required_check_missing_after_the_rerun_refuses_as_missing_required`.
4. **0111d — the typed park.** Files: `crates/brokkr-core/src/park.rs` and its `parse` and
   `forms` modules, `crates/brokkr-runtime/src/engine/cursor.rs`, and the view and CLI tests that
   paint the cause. About 120 production and 200 test lines. Contract: `park-cause/v2`. Tests:
   `a_v1_infrastructure_cause_reads_kind_engine`; `a_ci_runner_park_is_never_retryable`;
   `a_ci_rerun_uncertain_park_is_never_retryable`;
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
