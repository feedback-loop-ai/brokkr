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
boot parks with cause `ci_infrastructure` and no seat attempt.** The engine draws the class
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
until slice 0111b records a real payload.**

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
  CI, judge a pass, classify a failure, and park a runner fault with typed evidence and no seat
  attempt. They send no re-run request and hold no write credential. A typed
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

**Enforcement binding:** 0111c's overlay test refuses any seat in the CI overlay that declares a
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
- **(b) A built-in engine effect that calls the forge.** One reader for every recipe, but the
  engine gains an HTTP client, a credential path and a forge vocabulary of its own, a second
  effect kind for one forge. Refused for now; the observation contract in (a) is what (b) would
  write, so (b) stays open.
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
  unmeasured; slice 0111a's test settles it.
- **Its tools.** The interpreter and `gh` resolve through the seat's `PATH`, which 0049 ruling 3
  leaves unpinned under `harness` and `open` (quoted at
  `crates/brokkr-runtime/src/bundle/model_policy_tests.rs:2595-2596`). Pinning them is
  **unmeasured**, so the observer fails closed: it refuses, typed, when `bash`, `gh` or `jq`
  resolves under the gated worktree. It names the repository explicitly (`--repo`), so `gh`
  never takes the repository from the worktree's git remotes. Whether other change-controlled
  files can reach `gh`'s configuration is unmeasured until 0111b, and until then a run whose
  observer cannot show its tool paths outside the worktree refuses.
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
(`crates/brokkr-core/src/policy/inputs.rs:116-118`). Slice 0111a adds the first, parsed at
decide under its contract and refused whole on any unknown field.

The observation names its binding:

- the forge (`github`, the one word v1 admits);
- the repository, by owner, name and numeric id;
- the pull request's number, and its base commit's full id;
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
- every workflow run the forge lists for the head, with its path, event, number, attempt and
  status, so the engine can tell which run is current;
- a commit status carries its context, its state and its creator's login and numeric id
  (<https://docs.github.com/en/rest/commits/statuses#get-the-combined-status-for-a-specific-reference>).

It is parsed once at the edge, with unknown fields refused (0071 ruling 3). An unknown
conclusion word refuses by name. Whether the forge reports a workflow job's key, as the workflow
file spells it, is unmeasured until 0111a's evidence; if it does, the observation carries it.

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
    `test (macos-latest)`, job `engine` over its matrix (`:315-322`). Each is its own entry;
  - **a commit status** is bound to its context and a pinned creator, by numeric id.

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

**A changed check definition gets no automatic pass.** A change to CI can keep a required
check's name while substituting what produces its success. So each required entry declares its
`definition`: the paths that define what the check runs. These are its workflow file, and every
local action and script its job runs, with those of every job it `needs:`, transitively. HEAD's
audit job checks out the change and runs the checkout-local action
`./.github/actions/setup-cargo-audit` (`.github/workflows/ci.yml:681-689`), so that action's
directory is in the audit's definition. HEAD's `test (…)` jobs need `engine-machines`
(`:317`), which runs `./.github/actions/local-runner`, `./.github/actions/setup-bubblewrap` and
`scripts/test-shards.sh` (`:232`, `:247`, `:255`), so all three are in theirs. The compile
refuses an entry whose definition lacks its own workflow file. When any changed path falls
inside any required entry's definition, the observation refuses with `definition-changed`,
naming the paths, and the run parks for a person. Cost, said plainly: every change to CI's own
definition is a person's to pass. The residual is the operator's: a script a required job runs
that the declared definition omits is not guarded, and the review that follows reads the change.

**The tested revision is established, or the pass refuses.** HEAD's pull-request workflow tells
three revisions apart: `delivered by brokkr` checks out the trusted base and the pull request's
head by explicit ref (`.github/workflows/ci.yml:49-64`), and the ratchet job asserts that its
checkout is the pull request's merge commit (`:663-671`). A checkout with no `ref`, as the audit
job's (`:681-683`), tests the forge's generated merge revision on a `pull_request` event
(<https://docs.github.com/en/actions/writing-workflows/choosing-when-your-workflow-runs/events-that-trigger-workflows#pull_request>;
unmeasured here). So each required Actions entry declares `tested`, one of two words:

- `head`: the run tested the gated head itself. The run's `head_sha` must equal it;
- `merge`: the run tested the forge's merge revision of the pull request. The observation must
  name that merge commit's id and its two parents. Its second parent must be the gated head, and
  its first the base commit the observation names, which must be the pull request's current base.

The declaration is the operator's statement about a pinned workflow, and `definition-changed`
holds that workflow to the base's bytes. Whether the forge's run payload names the merge commit
it tested is **unmeasured**. Until 0111a's evidence shows it, every `merge` entry refuses as
`revision-unestablished`, and that fails closed.

**What a pass is.** The engine draws `green` only when all of these hold:

- the observation's repository, pull request and head equal the run's pinned repository, the
  pull request the gate names and the engine's own git head;
- no changed path falls inside a required entry's definition;
- every entry of `required_checks` is matched by exactly one settled check from its bound
  producer, in that producer's current execution;
- each matched Actions entry's tested revision is established, as above;
- each matched check's conclusion is `success`, or each matched status's state is `success`.

The **current execution** of an Actions entry is the workflow run with the highest run number
among those the forge lists for its workflow path, its event and the head, read at its highest
attempt. Any other run for that key still queued or in progress, or a run list the observer could
not read whole, refuses as `execution-ambiguous`.

Nothing short of that is a pass. **Every binding and completeness refusal comes before any
classification.** The engine checks each row below in order, records every one that holds, and
classifies a failure only when none does. So a required check that is missing beside an
otherwise classifiable failure refuses as `missing-required`; it is never typed.

| Refusal | When |
|---|---|
| `unrelated` | the repository, the pull request or the head differs from the binding |
| `range-unavailable` | git cannot take ruling 4's pinned base-to-head range |
| `definition-changed` | a changed path falls inside a required entry's definition; the refusal names the paths |
| `empty` | the forge reports no check for the head |
| `producer-mismatch` | a check carries a required entry's name, or a status its context, from another app, workflow path or creator; the refusal names it, so a substitute is never silently passed over |
| `missing-required` | a required entry matches no check from its bound producer; the refusal names the entry |
| `ambiguous-required` | a required entry matches more than one check in its current execution |
| `execution-ambiguous` | a required entry's current execution cannot be told, as above |
| `revision-unestablished` | a required entry's tested revision is not established, as above |
| `not-settled` | a required check is still queued or in progress when the observer's own polling bound ends, short of the seat's deadline |
| `required-set-disagrees` | the forge requires a check the pinned list lacks |

One more refusal follows the pass and precedes classification: `non-success`, when no required
check failed, and one concluded `neutral`, `skipped`, `stale` or `action_required`. It names the
check and the word.

A refusal never passes. The run parks `ruled`, with the rule's id and the refusals' words.

The gate's word is bound to the engine's class: `pass` is admitted only on `green`, and `fail`
with a `green` class is refused. A refused word is never rewritten (the same binding 0109
ruling 2 gives a verdict).

The classification is engine-written: an `Owner::Engine` record input, as `reviewed_heads` is
(`crates/brokkr-core/src/policy/inputs.rs:116`), so a seat may neither declare nor overwrite it,
as the engine-owned strategy is held (`:87-95`). Both inputs ride in the decision's `inputs`
(`crates/brokkr-core/src/decision.rs:86`). So the journal holds three things, in the order they
happened, with no new event: the observation the seat sealed, the class the engine drew from it,
and the rule that routed on it. A replay re-derives the class from the sealed observation and
the pinned list, and must equal the recorded one.

- **(b) A new `ci/classified` event.** Moves the event envelope, every manifest and every fold
  arm for a fact an input can carry. Refused, as 0106 ruling 1 refused `run/park-caused`.
- **(c) The excerpt kept outside the journal by digest.** Smaller rows, but the classification
  could not be replayed from the journal alone. Refused; the bound below is the cost control.

**Log excerpts, bounded in total and handled before they are durable.** Under ruling 4 (a) the
observer reads no log, and the observation carries no excerpt; v1's parser refuses the field.
Under (b) only, an excerpt is carried only for a failed step that is runner-owned, and the
observer applies four bounds before the seat's result is written, so they hold before sealing:

- each excerpt is at most 8 KiB, from the step log's tail, with a `truncated` flag;
- an observation carries at most four excerpts, so 32 KiB in total, and a fifth candidate
  refuses the observation rather than dropping one silently;
- each byte outside printable ASCII, the newline aside, is escaped;
- the excerpt is data the classifier matches bytes against, never text any seat or model reads
  as instruction.

Whether the forge masks every secret in a step's log is **unmeasured**
(<https://docs.github.com/en/actions/security-for-github-actions/security-guides/using-secrets-in-github-actions>).
So (b) does not build until 0111b measures it. If masking cannot be shown, the observer journals
only the matched pattern's id and the excerpt's digest, and the replay property of (c) is lost
for (b).

### 4. What evidence types a failure as the runner's, and the classifier

**This is the operator's first choice. Recommended: (a) forge-proven non-execution only.**

The first draft typed a failure as the runner's when an allow-listed setup step's excerpt held
a declared literal, and the step's workflow was untouched. That is not enough. An unchanged
setup step can still run the pull request's code: a service bootstrap that starts the
repository's own compose file or script, or HEAD's own checkout-local action,
`uses: ./.github/actions/setup-cargo-audit` (`.github/workflows/ci.yml:681-689`), whose code comes
from the checkout. A source-only change can then make that step print the declared literal. So
the evidence is the operator's choice, stated fairly:

- **(a) Recommended: forge-proven non-execution only.** A failure is the runner's only when the
  forge's own job-level conclusion proves the job's steps never ran. No log pattern types
  anything, and the list carries no patterns.
  - **Cost, said plainly:** the adopter's port-bind case and their timing-sensitive test are the
    change's, as they are today.
- **(b) Declared runner-owned evidence.** A literal types a failure only from a step whose
  provenance is runner-owned and positively known. An example is the runner's own
  service-container start for the workflow's `services:` block
  (<https://docs.github.com/en/actions/using-containerized-services/about-service-containers>).
  - **Cost:** what each forge exposes as runner-owned is partly unmeasured.
- **(c) Refused: any pattern over a step that runs repository code.**

**Timing-sensitive tests are not runner faults under any option.** A flaky test fails in a step
that runs the change's code, and nothing the forge states types it. Flaky-test handling is out of
scope; it is a follow-up for a decision of its own.

**The classifier.** One pure function over the observation, the declared list and the change's
paths, with a closed result, failing closed.

`classify(observation, faults, changed_paths) -> Classification` lives in `brokkr-core`, reads
no clock, environment or network (0071 ruling 1), and is total.

`changed_paths` is the engine's, through the one git observation HEAD already has:
`crate::anchor::changed_paths(repo, from, to)` (`crates/brokkr-runtime/src/anchor.rs:171`),
which 0039's docs-only flag reads (`crates/brokkr-runtime/src/engine.rs:3158-3173`). It is not
read over the same range. The docs flag diffs the latest phase entry's head against the phase's
end, so it sees only that phase's own commits. The CI gate needs the whole change, so its range
is pinned: from the merge base of the pull request's base commit, as the observation names it,
to the gated head. Both ends are checked as commit ids before git sees them. A range git cannot
take (a base it does not have, or no merge base) refuses as `range-unavailable`, with ruling 3's
refusals. It is never read as an empty change. In order:

1. Any binding or completeness refusal of ruling 3 holds: **`refused`**, naming every one that
   holds. Nothing below runs, so no refusal is ever outranked by a classifiable failure.
2. The observation passes ruling 3's pass rule: **`green`**. One that cannot pass and holds no
   failed, timed-out, unstarted or cancelled required check is **`refused`** as `non-success`.
3. Each failed required job is typed on its own:
   - a job-level conclusion that proves non-execution: runner, kind **`not-started`**. That set
     is empty until 0111b measures `startup_failure` on a job;
   - under (b) only: conclusion `failure`, its first failed step runner-owned as (b) defines,
     and that step's excerpt holding one of the list's literals for that step: runner, kind
     **`runner-owned-pattern`** with the pattern's id. Under (a) this arm does not exist;
   - anything else, `timed_out` included (a hang may be the change's): **untyped**.
4. Every failed job runner, and at least one failed: **`ci-infrastructure`**, naming each job,
   its step, its kind and pattern id.
5. Otherwise **`change`**, with a closed reason: `untyped-failure`, or `cancelled` (any required
   job cancelled: who cancelled is unmeasured, though on a pull request a newer push cancels the
   older run, `.github/workflows/ci.yml:24-26`).

A change to a required check's definition never reaches step 3: it is refused at step 1 as
`definition-changed`, for a person, and no pattern over its steps is read.

One untyped job makes the whole observation `change`: the change has a failure to answer. A
failure the classifier cannot type is the change's, as today.

**Audit jobs need no rule of their own.** Fix round 1093r2 kept an audit job out of any re-run
unless the forge proved its steps never ran, so a re-run could not launder an advisory. With no
write, no class reaches a pass but `green`, and `green` needs the audit's own `success` (ruling
3). An audit failure is typed like any other: the change's, or a person's park. #1113's "Not
Found" is a failed step inside the `RustSec dependency audit` job
(`.github/workflows/ci.yml:673-695`). Under ruling 4 (a) it is the change's, as today; this
decision declares nothing for it and designs no fix for it.

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
  "required_checks": [
    {"actions_job": {
      "app": "github-actions", "workflow": ".github/workflows/ci.yml",
      "job": "engine", "name": "test (ubuntu-latest)", "event": "pull_request",
      "tested": "merge",
      "definition": [".github/workflows/ci.yml", ".github/actions/local-runner/",
                     ".github/actions/setup-bubblewrap/", "scripts/test-shards.sh"]}},
    {"actions_job": {
      "app": "github-actions", "workflow": ".github/workflows/ci.yml",
      "job": "dependency-audit", "name": "RustSec dependency audit", "event": "pull_request",
      "tested": "merge",
      "definition": [".github/workflows/ci.yml", ".github/actions/setup-cargo-audit/"]}},
    {"status": {"context": "example/deploy-preview", "creator_id": 1}}
  ]
}
```

The `tested` words, the status entry and its creator id are illustrative; the app's numeric id
is pinned beside its slug once 0111a's evidence shows it.

That is the whole list under ruling 4 (a). Under (b) only, two more keys are admitted:

```json
{
  "runner_owned_steps": [{"workflow": ".github/workflows/ci.yml", "job": "test", "step": "Initialize containers"}],
  "patterns": [
    {"id": "port-in-use", "steps": ["Initialize containers"], "literal": "address already in use"}
  ]
}
```

The step name above is illustrative and unmeasured. Under (a) the compile refuses either key. A
pattern is a literal: 1 to 200 printable ASCII bytes, matched as an exact, case-sensitive byte
substring. No regex, glob or wildcard is admitted, so matching is linear and the same on every
host. `id` is kebab-case and unique; every step a pattern names must be in
`runner_owned_steps`.

The list has two syntaxes, and they never mix. A **path** in a `definition` is a
repository-relative path. It matches a changed path that equals it, or, when it ends in `/`,
any changed path under that directory. It is not a glob, and not the docs class's regular
expressions (`crates/brokkr-runtime/src/engine.rs:3217`). A **literal** in `patterns` is matched
only against log bytes, never against a path. Each field admits only its own syntax: the
compile refuses a `*`, `?` or `[` in a path, so a glob cannot pass for one, while a literal's
bytes are all literal.

`required_checks` must be non-empty, every Actions entry must name all of its producer's parts
and its `tested` word, and every `definition` must hold its own workflow file. The compile
refuses a list that leaves any of them to a default. `repository` pins the binding ruling 3
checks.

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

- **`green`**: the gate's `pass`; the run proceeds.
- **`ci-infrastructure`**: the run parks, typed (ruling 7), with the faults as its evidence, and
  no seat attempt is spent. A person reads the evidence and re-runs.
- **`change`**: routed exactly as a red gate is today, to `implement`.
- **`refused`** (ruling 3): parks `ruled`.

**A deterministic match.** Under ruling 4 (b) only, where several literals occur in one excerpt,
the classifier takes the first pattern in the list's declaration order. The fault carries that
pattern's id, so one excerpt yields one fault.

**Every route and cap.** A table in the recipe declares the arms, first match winning. The
`When` column names the class; ruling 10 gives its schema-legal spelling.

| Rule | From | Result | When | Next |
|---|---|---|---|---|
| `CI-REFUSED` | `ci` | `fail` | class `refused` | park, `ruled`, naming the refusal |
| `CI-PASS` | `ci` | `pass` | class `green` | the recipe's next phase |
| `CI-INFRASTRUCTURE` | `ci` | `fail` | class `ci-infrastructure` | park, `infrastructure` `ci-infrastructure` |
| `CI-FAIL-EXHAUSTED` | `ci` | `fail` | `visits_implement_gte: 3` | stop, `hard` |
| `CI-FAIL` | `ci` | `fail` | | `implement`, `flagged` |

**The CI loop keeps a bound.** HEAD caps only `VERIFY-FAIL`: `VERIFY-FAIL-EXHAUSTED` stops at
`visits_implement_gte: 3` (`recipes/fast/policy.json:76-94`). `CI-FAIL-EXHAUSTED` reads the same
counter. `visits_<phase>_gte` counts the run's entries of that phase, whatever routed into it
(the `when` description at `contracts/phase-machine.v2.schema.json:71-73`). So a `VERIFY-FAIL`
return and a `CI-FAIL` return spend the one implement bound of three between them. A green local
verify and a red CI therefore stop after the third implement visit, not never. No other loop
exists: every other class parks.

**Refused routes**, each because it would unbound a loop, hide a failure or write to the forge:

- any route to a re-run, under every class (ruling 1);
- `ci-infrastructure` routed to `implement`: a seat attempt is what the class exists to save;
- a refusal routed to `implement`; a refusal parks;
- `CI-PASS` on any class but `green`; the word binding refuses it (ruling 3);
- a `CI-FAIL` with no exhaustion arm before it; the compile check for this belongs to 0111c's
  overlay test.

- **(b) A runner fault routed to `implement`, with the class in the commission.** It spends the
  seat attempt the issue counts, to tell a model what the engine already typed. Not
  recommended.

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

The writer: the engine's park arm (`crates/brokkr-runtime/src/engine/cursor.rs:361-371`) gains
one case. It writes `infrastructure { kind: ci-infrastructure, retryable: false }` instead of
`ruled` only when both hold:

- the park is a table park, `ParkReason::Stated` with a matched rule id (`:368`);
- the deciding inputs' `ci_class` is `ci-infrastructure`.

A closed indeterminate end keeps HEAD's `infrastructure` arm (`:364-366`). Every other CI park,
every refusal included, stays `ruled`. The rule id stays in the decision beside the cause, where
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
  could also write two of them in one rule, which would never match; 0111c's overlay test refuses
  that at compile. It mints no contract.
- **(b) `phase-machine/v3`, admitting a non-empty string array for a `<choice>_in` key.** It
  is the honest shape for a closed choice, and it would also bring `fast`'s existing
  `blocked_by_in: ["environment"]` (`recipes/fast/policy.json:44-46`) inside a published schema.
  Cost: a new frozen contract file under 0082, its pin beside v2's in
  `crates/brokkr-runtime/tests/frozen_contracts.rs:303`, and every table's `schema` word moving
  or a reader for both. A wider change than this decision needs.
- **(c) `ci_fault_in` under v2, as drafted.** Refused: it is not legal under the schema it
  claims.

`fast`'s `blocked_by_in` array already sits outside v2's published schema. That is a finding
for a follow-up, and this decision does not fix it.

**Enforcement binding:** 0111c validates the overlay's `policy.json` against
`contracts/phase-machine.v2.schema.json`, and a test asserts that exactly one flag is true for
each non-`change` class.

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
- A forge other than GitHub: `forge` is a closed enum of one word in v1.
- An automated re-run: the follow-up decision ruling 1 (c) names.

## Consequences

A runner fault the classifier can type costs no model diagnosis attempt: the run parks with a
cause a script can read and evidence a person can act on, and the person re-runs. Every other
failure is the change's, as today. Brokkr sends the forge nothing, so no mistyped class can
reach a pass or a re-run; its worst cost is a person's look. A recipe with no list behaves
exactly as now. The cost is an exec observer per recipe that wants it, one read-only token, and
a list an operator keeps. Four contract versions are proposed: `ci-observation/v1`,
`ci-classification/v1`, `ci-checks/v1` and `park-cause/v2`. Each is minted only when its slice
lands under this decision's acceptance (0082). Under ruling 10 (a) the table contract does not
move; under (b) a fifth, `phase-machine/v3`, is proposed.

## The slices

Smallest first; each lands on its own.

1. **0111a — the classifier and its record over a recorded payload.** Files: a new
   `crates/brokkr-core/src/ci.rs` with `ci/observation.rs`, `ci/faults.rs`, `ci/classify.rs` and
   `ci/tests.rs`; `crates/brokkr-core/src/lib.rs`; `crates/brokkr-core/src/policy/inputs.rs` (the
   seat-owned `ci_observation` record; the engine-owned `ci_class` record and ruling 10's three
   flags); `crates/brokkr-runtime/src/bundle.rs` (load and pin `ci-checks.json`, refuse it on a
   non-exec seat); `crates/brokkr-runtime/src/engine/decide.rs` (classify at decide and record
   the input); `crates/brokkr-view/src/` (the CI view model) and
   `crates/brokkr-cli/src/render/inspect.rs`. About 550 production and 850 test lines; it may
   split at triage into the observation and its pass (with the wire test) and the classifier.
   Contracts: `ci-observation/v1`, `ci-checks/v1`, `ci-classification/v1`. Tests: one per arm
   and reason of ruling 4, over recorded observations built by a shared builder. The wire, end
   to end, over one recorded observation fixture, in `ci-observation/v1`'s shape and checked in
   under the slice's tests:
   `a_recorded_observation_is_sealed_classified_replayed_and_inspected`. That test drives an
   exec `ci` seat whose result carries the fixture. It asserts that the sealed seat record holds
   the observation; that the decision's inputs hold `ci_class` and its one true flag; that a
   fold rebuilt from the journal re-derives the same class; and that `inspect` paints it. The
   ownership tests: `a_seat_declaring_ci_class_or_a_class_flag_fails_the_compile`;
   `a_seat_payload_carrying_ci_class_is_stripped_before_the_table`;
   `an_observation_with_an_unknown_field_refuses_whole`. The range and the list's syntax:
   `the_change_range_is_the_merge_base_to_the_gated_head_not_the_phase_entry`;
   `a_range_git_cannot_take_refuses_as_range_unavailable`;
   `a_glob_character_in_a_definition_path_refuses_to_compile`;
   `a_directory_path_matches_only_under_its_slash`. The evidence:
   `a_timing_sensitive_test_failure_is_the_changes`; `a_cancelled_job_is_the_changes`;
   `one_untyped_job_makes_the_whole_change`;
   `an_unknown_conclusion_refuses_by_name`; `a_stale_head_observation_does_not_read`;
   `a_pass_with_a_failed_job_is_refused`; `a_seat_cannot_write_the_classification`;
   `a_ci_list_on_a_model_seat_refuses_to_compile`; `inspect_names_the_class`. The producer, the
   definition and the tested revision (ruling 3):
   `a_same_named_green_check_from_another_workflow_refuses_as_producer_mismatch`;
   `a_status_from_an_unpinned_creator_refuses_as_producer_mismatch`;
   `a_green_check_on_another_event_does_not_match_its_entry`;
   `a_change_to_ci_yml_refuses_as_definition_changed`;
   `a_change_to_the_local_action_a_required_job_runs_refuses_as_definition_changed`;
   `a_change_to_a_script_a_needed_job_runs_refuses_as_definition_changed`;
   `a_definition_without_its_own_workflow_refuses_to_compile`;
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
   `a_forge_required_check_the_list_lacks_refuses`; `an_empty_required_list_refuses_to_compile`;
   `a_ci_seat_whose_layer_is_inside_the_worktree_refuses_at_start`.
2. **0111b — a GitHub observer script.** Files: a house observer script and its shellcheck
   clean; a test that runs it against a stub `gh` on the path and holds its output to
   `ci-observation/v1`; one recorded real payload, measured by the operator, which settles this
   decision's unmeasured forge facts. Tests:
   `the_observer_refuses_when_gh_resolves_under_the_worktree`;
   `the_observer_names_the_repository_and_never_reads_the_worktrees_remotes`;
   `the_observer_lists_every_check_for_the_head_unfiltered`;
   `the_observer_holds_no_write_binding`. About 150 script and 250 test lines. No contract
   change.
3. **0111c — the routing overlay.** Files: a recipe overlay declaring ruling 6's table, and its
   compile tests. About 100 data and 350 test lines. No contract change. Tests:
   `a_green_observation_passes`; `a_change_failure_returns_to_implement`;
   `a_runner_fault_parks_with_no_seat_attempt`;
   `a_green_verify_and_a_red_ci_stop_at_the_third_implement_visit`;
   `verify_and_ci_returns_share_one_implement_bound`;
   `a_refusal_never_routes_to_implement`; `no_rule_routes_to_a_rerun`;
   `a_ci_infrastructure_class_never_routes_to_implement`;
   `a_ci_overlay_seat_declaring_a_write_binding_refuses_to_compile`;
   `the_ci_overlay_policy_validates_against_phase_machine_v2`;
   `exactly_one_class_flag_is_true_for_each_class_but_change`;
   `a_rule_naming_two_class_flags_refuses_to_compile`.
4. **0111d — the typed park.** Files: `crates/brokkr-core/src/park.rs` and its `parse` and
   `forms` modules, `crates/brokkr-runtime/src/engine/cursor.rs`, and the view and CLI tests that
   paint the cause. About 100 production and 180 test lines. Contract: `park-cause/v2`. Tests:
   `a_v1_infrastructure_cause_reads_kind_engine`; `a_ci_infrastructure_park_is_never_retryable`;
   `a_runner_fault_park_writes_the_ci_infrastructure_cause_and_keeps_the_rule_in_the_decision`;
   `a_refusal_park_carrying_a_ci_class_stays_ruled`; `the_fleet_paints_the_ci_infrastructure_cause`;
   and, when 0106d lands, its `a_non_retryable_infrastructure_park_spends_nothing` (0106:1224)
   extended with a `ci-infrastructure` park (ruling 8).

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
page in any round. Every forge fact cited by URL is unmeasured until 0111b.
