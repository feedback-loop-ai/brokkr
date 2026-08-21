---
name: forge
description: Fully autonomous multi-provider vertical delivery for the Alkemio polyrepo — a model-blind architect council synthesized by a Fable chief, provider-routed implementation waves, isolated live verification, adversarial security review, regression, and evidence-backed PRs. Supports single trusted providers, policy-confined DeepSeek, and public-only Qwen multimodal analysis without weakening human gates.
argument-hint: "<feature description> [--repos a,b] [--providers auto|claude|codex|codex,claude|codex,claude,deepseek plus optional ,qwen] [--resume NNN-slug] [--from implement|verify|review|ship] [--rounds N] [--no-ask] [--skip-verify] [--council|--solo]"
user-invocable: true
---

# /forge — autonomous vertical delivery

One command: idea → research → spec → plan → parallel implementation across
repos → live test-pyramid verification → adversarial multi-model review
(security included) → targeted regression re-verify → PRs + evidence ledger.
Model capability is routed to where it pays:

```
 /forge ─▶ 0  Intake        ≤1 batched ask, or none (--no-ask)
        ─▶ A  Architecture  model-blind council: 3–4 contrarian senior seats ∥ (independent
                            papers from different evidence) → clash (mandatory objections) →
                            fork adjudication → [the one intake ask, only for contested forks]
                            → CHIEF (FABLE→OPUS, max) synthesizes w/ recorded dissent · spec ·
                            self-clarify · plan · tasks (test-paired) · analyze→0 · story-sync
                            hook · waves/contracts/gates/verification in the feature repos.yaml
                            (solo chief, no council, for small features)
        ─▶ W  Worktrees     feature-worktree.sh per repo, in parallel
        ─▶ I  Implement     provider-routed implementers ∥ per wave · cross-repo contract
                            checks (gql-pipeline generalized) · independent HAIKU gate audit
        ─▶ V  Verify live   trusted routes: stack boots FROM WORKTREES → repo it-specs ∥ test-suites
                            ∥ gql-live ∥ headless-Playwright acceptance walks (pseudo-manual QA,
                            per story) → attributed routed fixes → re-run failed only → persist
                            passing walks as @forge-acceptance specs → teardown, always
        ─▶ R  Review        trusted senior panel (correctness · spec-compliance · quality ·
                            SECURITY SOC2/ISO27001) → two-lens adversarial verify → routed fixes
                            → scoped re-review · security-hold is a hard stop
        ─▶ V' Regression    forge-verify mode=regression: fixed repos' tracks + P1 smoke stories
        ─▶ S  Ship          push ∥ · PRs ∥ (CodeRabbit triage hook fires) · evidence ledger
```

**Design lineage**: this generalizes the server repo's gql-pipeline
(haiku runner → sonnet fixer → opus reviewer) into arbitrary cross-repo
contracts, and wraps the existing vertical-feature/SpecKit machinery — it
composes the vendored `speckit-*` skills, `feature-worktree.sh`, the
story-sync hook, and the CodeRabbit triage hook. It replaces none of them.

## Provider routing (the whole point — do not degrade silently)

Read `.agents/forge/providers.json` and `.agents/forge/handoff-protocol.md`,
then resolve the run once with:

```bash
python3 .scripts/forge-control.py resolve-profile --providers <selection>
```

`auto` selects the strongest eligible maximal, full, multimodal, dual, or
single-provider profile. DeepSeek and Model Studio keys are optional and never
required for a single-provider run. Explicit unavailable selections fail
preflight without silent downgrade.
Persist the resolution in `specs/<feature-id>/forge/run.json`.

The `full` public architecture council is GPT-5.6 Sol, Claude Opus 4.8, and
DeepSeek V4 Pro, each at max effort. The `maximal` profile adds Qwen 3.7 Plus
at its highest Model Studio API effort as a fourth experience-architecture
seat. Charters rotate deterministically. Claude Fable 5 at max is chief, with
a recorded Opus fallback. Council prompts and blind handoffs never reveal
provider provenance to seats or chief. Claude-only remains three Opus seats
plus Fable chief; Codex-only remains three Sol seats plus Sol chief.

The policy file, not this summary table, is authoritative for every route:

| Role | Agent / call | Model | Effort |
|---|---|---|---|
| Chief architect: synthesis, spec, self-clarify, plan, tasks, analyze | native or provider adapter | **fable** (Opus fallback), or Sol in Codex-only | max |
| Council seats: position papers + clash (simplifier · systems · operator · optional experience) | native workflow or shared provider adapter | profile-routed | max |
| Fork adjudicator (agreed vs contested questions) | native or trusted adapter | trusted balanced route | low |
| Research scouts (spawned by a solo chief) | Task inside architect | chief-runtime route | medium |
| Implementers & fixers | native agent or adapter | profile route; DeepSeek only when public-eligible | high |
| Contract checkers | native agent or adapter | profile route; DeepSeek only when public-eligible | medium/high |
| Panel reviewers (correctness, spec-compliance, quality) | trusted native agent or adapter | Opus/Sol | high |
| Security reviewer | trusted native agent or adapter | Opus/Sol | xhigh |
| Finding verifiers (two lenses per finding) | trusted native agent or adapter | Opus/Sol | high |
| Stack runner and live tracks | trusted native agent or adapter | Claude/Codex balanced route | medium |
| Pseudo-manual acceptance walker | trusted native agent or adapter | Claude/Codex senior route | high |
| Public screenshot/image understanding | shared `visual-verifier` route; trusted fallback | Qwen 3.7 Plus first | high |
| Gate auditors, mechanical chores | native agent or adapter | profile route; DeepSeek read-only when eligible | policy |

**Fable is an opportunistic upgrade, not a dependency** in profiles that route
it: only the *chief* seat tries `fable`; `forge-architect.js` falls back to an
`opus` chief automatically and reports `architect_model` for the ledger. In
multi-provider mode the same fallback is explicit in `providers.json` and must
be recorded. Codex-only routes the Sol chief directly.

Claude-only uses the checked-in `.claude/workflows/*.js`. For any other
profile, the main loop follows the same phase schemas, budgets, and exit gates
but dispatches each role through `.scripts/forge-control.py route` and
`.scripts/forge-control.py invoke`; the JavaScript workflow remains reference
logic and must not be used as though it could host external providers.

## The phase machine is mandatory, not advisory

Phase transitions are **not** a judgement call. `.agents/forge/phase-machine.json`
is the deterministic transition table; `.scripts/forge-control.py` is its only
evaluator, shared by Claude Code, Codex, and opencode.

At the **start** of every phase, re-read authoritative state from disk rather
than trusting conversation context (which is summarized on long runs):

```bash
python3 .scripts/forge-control.py phase-state --spec-dir specs/<feature-id>
```

At the **end** of every phase, submit the result and let the table rule:

```bash
python3 .scripts/forge-control.py phase-advance --spec-dir specs/<feature-id> \
  --feature-id <feature-id> --phase <phase> --result <result> \
  [--repo <r> ...] [--skip-verify] [--fixes-applied] [--security-residual] \
  [--high-risk-uncovered] [--max-residual-severity low|medium|high|critical]
```

It returns `next` (the phase you may enter), the `rule` id that fired, and any
open `blockers`. **Go where it says.** Do not re-derive a ruling in prose, do
not "interpret" a stop, and do not proceed on an unrecognised result — an
unknown result is rejected by design rather than guessed. Phases cannot be
skipped: a transition is legal only from the phase the journal records.

Four facts are computed by the script and are **not** accepted from you: retry
budgets (replayed from `events.jsonl`), post-review drift (real `git rev-parse`
against the SHAs recorded at review time), dirty worktrees, and unsigned
commits. Asserting otherwise changes nothing.

Every transition also appends a commit to `refs/forge/<feature-id>` recording
the journal's hash and every feature repo's HEAD at that instant. It is built
with git plumbing on its own ref, so it never touches the index, the working
tree, or the checked-out branch — and because commits are content-addressed,
editing the journal afterwards is detectable even if its JSON anchor is
rewritten to agree. It is **tamper-evidence, not tamper-proofing**: the ref is
unsigned, so a party who can run git here can rebuild the chain. Treat it as
forensic evidence for honest failures, not as a barrier to a hostile agent. `git log refs/forge/<feature-id>` is the run's audit trail;
push it when the evidence needs to leave the machine.

`git push` and `gh pr create` on a `feat/<id>` branch are blocked by a
PreToolUse hook in all three runtimes when state is not shippable, **and** by a
git `pre-push` hook installed in every feature worktree — the latter sees the
real refs regardless of how the push was spelled or from which directory.

**These are defence in depth, not an unbypassable control.** Anyone who can run
git locally can pass `--no-verify` or `-c core.hooksPath=`, and no client-side
hook can prevent that. What they buy is that the accidental and casual paths —
the ones a drifted agent actually takes — are closed, and a deliberate override
is a visible, attributable act. **The authoritative gate is server-side branch
protection with a required status check**, which is an open human gate, not
something this feature configures. Do not treat a passing local gate as proof
that a hard stop cannot be reached.

**Hard blockers are retired only by the operator — but you never stall waiting
for one.** A blocker is reconstructed from the append-only journal, so a later
`clean` result does not clear it, and deleting `state.json` does not either.
The flow is propose → attest:

```bash
# You may run this freely. It records context; the blocker STAYS OPEN.
python3 .scripts/forge-control.py phase-clear --spec-dir specs/<feature-id> \
  --rule <RULE-ID> --operator "<who should decide>" --reason "<what you did>" [--evidence <link>]
```

Keep going after proposing — an unattended run is never blocked on a human. At
the end, report the open blockers and the one command that retires them:

```bash
# OPERATOR ONLY, batched at run close. Never run this yourself.
python3 .scripts/forge-control.py phase-attest --spec-dir specs/<feature-id> \
  --assume-operator --operator "<you>"
```

Only a run that actually hit a hard blocker needs attestation; clean runs never
touch it. Both the proposal and the attestation are journalled, so the ledger
shows what was proposed, by whom it was accepted, and in which mode.

## Autonomy doctrine

- **Never interrupt the flow.** After Phase A starts, no user questions, ever.
  Ambiguity is resolved by the architect and recorded in
  Clarifications/Assumptions.
- **At most ONE intake gate — and it must be earned.** The single batched
  AskUserQuestion round (≤4 questions) happens between council and chief
  synthesis, and only for questions the council still *contests after the
  clash round* — returned seats answering differently is what "genuine scope
  fork" means, measured rather than vibed. No contested questions → no ask.
  Solo (no council) → ask only if the request itself is unintelligible.
  With `--no-ask`, never ask — the chief decides and records.
- **Human gates stay human**: PR merge, residual-security-risk acceptance,
  story sign-off, deploys. /forge stops at open PRs + evidence.
- **Parallel by default**: independent Bash calls (worktrees, pushes, PRs) go
  in one message; repo × dimension work runs inside the workflows.

## Arguments

`$ARGUMENTS`: free-text feature description, plus optional flags —
`--repos a,b,c` (constrain candidate repos) · `--resume NNN-slug` (existing
spec dir; skip completed phases) · `--from implement|verify|review|ship` (entry
phase for a resume) · `--rounds N` (review rounds, default 3) ·
`--dimensions d1,d2` (default all four; **security cannot be removed** — if
the list omits it, add it back and say so) · `--no-ask` (skip the intake gate) ·
`--council` / `--solo` (force or skip the architect council; default is the
sizing rule in Phase A) ·
`--providers auto|claude|codex|codex,claude|codex,claude,deepseek` plus an
optional trailing `,qwen` (Qwen may also augment one trusted provider; default
`auto`; `anthropic`, `chatgpt`, and `modelstudio` are aliases).

## Phase 0 — Intake (main loop)

1. Confirm cwd is the workspace root (`repos.yaml` present); abort with
   guidance otherwise. Warn that the workspace checkout will switch to the
   feature branch (speckit git hook behavior).
2. Read `repos.yaml` (workspace manifest). Candidate repos = `--repos` if
   given. Otherwise a trusted coordinator may derive an explicit candidate
   list for council routing (never archived); uncertainty or mixed visibility
   keeps DeepSeek and Qwen out. The chief may refine the final affected set.
3. Resolve and persist the provider profile. Classify every proposed DeepSeek
   or Qwen payload as public or non-public before serialization. Private issues/specs
   and mixed public/private contracts are non-public even for a public repo.
4. Ask nothing here (unless the request itself is unintelligible) — the
   intake gate is earned in Phase A, from the council's contested forks.

## Phase A — Architecture (council → gate → chief)

**Sizing rule**: convene the council when the feature plausibly touches ≥ 2
repos OR carries elevated risk (auth, migrations, new services, infra/GitOps,
data deletion). Below that, run the solo chief — a council on a button-color
change is waste. `--council` / `--solo` override the rule; record which path
ran and why in the ledger.

1. **Council** (skipped when solo): for Claude-only, use the native workflow:
   ```
   Workflow({ scriptPath: "<workspace>/.claude/workflows/forge-architect.js",
     args: { mode: "council", featureDescription: "<verbatim>", candidateRepos: <--repos or null> }})
   ```
   Three contrarian Opus/max seats (charters:
   `.claude/skills/forge/references/architect-council.md`) write independent
   position papers from *different evidence*, clash with mandatory
   objections/concessions, and a fork adjudicator splits the question union
   into `agreed` vs `contested`. `status: "degraded-no-clash"` (budget floor)
   is usable — synthesis proceeds on positions alone, noted in the ledger.
   `council-failed` (<2 seats) → fall back to solo chief, noted.

   For every non-Claude-only profile, instead run `forge-control.py council-route` and
   dispatch the returned assignments through the native Claude agent or
   `forge-control.py invoke`. Write and verify each position handoff, render
   other-seat inputs with `blind-handoff`, then resume each seat for clash.
   Do not include provider/model names in prompts or filenames. DeepSeek is
   omitted from a non-public council by policy; that is a recorded
   trusted-council routing decision, not a silent profile downgrade. A maximal
   public route returns four seats and adds the experience-architect charter.

2. **The intake gate, earned**: if `contested` is non-empty and `--no-ask`
   is not set, run the flow's single batched AskUserQuestion round (≤4 —
   pick the highest-stakes forks; present each seat's option verbatim-
   condensed). Answers become `USER_ANSWERS`. Contested questions beyond the
   4 asked (or under `--no-ask`) are decided by the chief and recorded.

3. **Chief synthesis**:
   ```
   Workflow({ scriptPath: same, args: { mode: "synthesize",
     featureDescription, candidateRepos, council: <council result or null>,
     userAnswers: <USER_ANSWERS or null>, resume: <RESUME or null> }})
   ```
   One `vf-architect` (fable; automatic opus fallback surfaces in
   `architect_model`) synthesizes with recorded dissent, runs speckit
   specify → self-clarify → plan → tasks → analyze-to-zero, and returns the
   structured contract. Its `after_specify` hook syncs the projects/50 story
   automatically.

   The synthesize workflow accepts the provider-neutral council assembled by
   the shared router. Codex-only instead dispatches the routed Sol chief through
   `forge-control.py invoke`; never execute the Claude workflow in Codex.

On return, sanity-check: `specs/<id>/spec.md`, `plan.md`, `repos.yaml` (with
`forge.waves`, per-repo `gates`, and `forge.verification` — empty `tracks` is
valid only as an explicit decision), one `tasks/<repo>.md` per repo, and —
when the council ran — a non-absent `dissents` log (empty is valid only if
the chief adopted every live objection). Missing pieces → re-run
`mode: "synthesize"` once with the specific gap named in `resume` (workflow
`resumeFromRunId` replays cached agents); if still incomplete, stop and report.

## Phase W — Worktrees (main loop)

For every affected repo, in parallel:
`./.scripts/feature-worktree.sh <feature-id> <repo>`
(idempotent; copies local env files). Verify each landed on `feat/<id>`.

## Phase I — Implement (provider-routed waves)

Build args from `specs/<id>/repos.yaml` + the architect result:

```
Workflow({ scriptPath: "<workspace>/.claude/workflows/forge-implement.js", args: {
  featureId, specDir: "<abs specs/<id>>",
  waves: [[{repo, worktree: "<abs worktrees/<id>/<repo>>", base: "<repo default_branch>",
            tasksFile: "<abs specs/<id>/tasks/<repo>.md>", gates: [...]}], ...],
  contracts: [...from forge.contracts...],
  architectNotes: "<assumptions + risks from the architect>",
  maxFixRounds: 2 }})
```

Runs in the background — monitor via /workflows, act on the completion
notification. `status: "broken"` → inspect the per-repo reports; re-run the
workflow with `resumeFromRunId` after addressing the blocker (cached agents
replay free). Two consecutive broken runs → stop and report; don't thrash.

That workflow invocation is the Claude-only path. In every other profile,
coordinate the same ordered waves in the main loop: resolve `implementer`,
`contract-checker`, `fixer`, and `gate-auditor` per task; dispatch native Claude
when selected and use `forge-control.py invoke` for external providers. Require
the same result schemas, fix bounds, independent gate reruns, and stop states.

DeepSeek is eligible only when the complete prompt, all parent handoffs and
contracts, and the mounted repository worktree are public. The adapter proves
the cwd belongs to the declared public Git repo, exposes only allowlisted
system tools and approved repos through bubblewrap, and strips ambient
credentials. `infrastructure-provisioning` and `infrastructure-operations` are
permanently denied regardless of manifest visibility. Never call DeepSeek
directly or weaken its sandbox. Stack operations, live/browser control, review,
security, and finding adjudication remain on trusted Claude/Codex routes.

Qwen is read-only and receives neither a repository mount nor agent tools. Its
architecture seat receives only a coordinator-built public excerpt bundle.
Stage the exact public prompt and optional output schema under
`specs/<feature-id>/forge/public-evidence/`, record their SHA-256 values and
`prompt`/`schema` kinds in the `forge.public-evidence/v1` `manifest.json`, and
only then invoke Qwen.
It cannot implement, fix, operate a stack, drive Playwright, review security,
or adjudicate findings.

## Phase V — Verify live (trusted routes, test pyramid against a running stack)

The Workflow call below is Claude-only. Other profiles dispatch the resolved
trusted stack, live-test, acceptance, and fixer routes in the main loop while
preserving the same isolation manifest, result schemas, and fix bounds.

The unit level already ran twice (implementer inner loops + gate audit). This
phase runs the levels that need a **live stack built from the feature
worktrees** — API/it-specs, test-suites scenarios, gql-live validation, and a
**pseudo-manual QA pass**: one headless `playwright-verifier` per user story
walking every spec.md acceptance scenario like a tester would, screenshotting
evidence, escalating to console/network on failure.

For an entirely public feature, the trusted walker may stage only disclosure-
safe screenshots under `specs/<feature-id>/forge/public-evidence/` and resolve
the `visual-verifier` route. Qwen 3.7 Plus is first when available and is called
through `forge-control.py invoke --image ...`; the adapter accepts only bounded
images and a hash-attested public prompt from that staging directory. Its
hashed, model-blind visual handoff is
returned to the trusted Playwright verifier, which alone owns browser access,
credentials, diagnostics, and the pass/fail decision. Private or mixed runs use
the trusted visual fallback and send no images to Qwen.

```
Workflow({ scriptPath: "<workspace>/.claude/workflows/forge-verify.js", args: {
  featureId, specDir, repos: [{repo, worktree, base, gates}, ...],
  verification: <forge.verification from the feature repos.yaml>,
  leadRepo: <architect's lead_repo>, mode: "full", maxFixRounds: 2 }})
```

**Isolation guarantee** (the developer's environment is sacred): the stack
runner boots everything inside an ephemeral compose project
(`forge-<feature-id>`) with its own fresh, prefixed volumes — your dev
database and queues are never mounted, never migrated, never restored-over,
and teardown is a project-scoped `down -v`. There is no backup/restore dance
because your data is never in the blast radius. Port modes: `shifted` runs
beside your live dev stack on offset ports; `standard` borrows the well-known
ports (they must be free) but keeps full data isolation. Seeding is
`bootstrap` (empty DB → migrations → platform bootstrap) or `clone-local`
(pg_dump **read-only** from your dev DB into the forge instance). A DSN
sanity gate aborts before any migration whose target isn't provably inside
the forge project.

**Risk-based execution** (ISTQB-aligned): the architect's
`forge.verification.risk_register` (likelihood × impact per product risk,
each `high` risk traceably mitigated by named scenarios/tracks/tasks) drives
depth and order — high-risk tracks and stories run first so the loop fails
fastest, and the result maps every register entry to
`mitigated | partially-verified | at-risk | unmitigated` for the ledger.

Interpret strictly:
- `pass` → proceed to review. Emitted `@forge-acceptance` specs are already
  committed in their repos — durable regression tests grown from the spec.
  Carry `risk_coverage` forward: `at-risk`/`unmitigated` entries of level
  `high` are treated like residual findings — report, don't bury.
- `fail` → failures survived the fix rounds; stop and report (a `--resume
  --from verify` re-enters after human input).
- `stack-conflict` → only possible in `ports: standard` (exclusive) mode:
  the well-known ports are occupied (often your own dev stack). Nothing was
  killed and no data was touched — free the ports (or have the architect
  re-plan to `shifted`) and `--resume --from verify`. Never ship around it.
- `not-applicable` (architect declared no tracks — docs/infra-only) → proceed;
  the ledger records the explicit decision.
- `env-degraded` → some tracks produced no signal; treat as `fail` unless the
  user said `--skip-verify`.
- `--skip-verify` skips this phase and V′ entirely; the ledger and every PR
  body must say so in bold. Emergencies only.

## Phase R — Review (trusted senior panel + security)

The Workflow call below is Claude-only. Other profiles dispatch resolved
trusted reviewer, security-reviewer, finding-verifier, and fixer routes; when
more than one trusted provider is available, exclude the implementation
provider from review where an equally capable route exists.

```
Workflow({ scriptPath: "<workspace>/.claude/workflows/forge-review.js", args: {
  featureId, specDir, catalogPath: ".claude/skills/forge/references/security-review-catalog.md",
  repos: [{repo, worktree, base, tasksFile, gates}, ...],
  dimensions: <parsed --dimensions, or all four; security re-added if omitted>,
  maxRounds: <--rounds or 3> }})
```

Interpret the result strictly:
- `clean` → proceed.
- `clean-unverified` or `residual` → proceeding is allowed **only** for
  non-security residuals of severity ≤ medium; list them in the PR bodies and
  the ledger as tracked debt. Anything higher → do not ship; report.
- **`security-hold` → never ship.** Present the open security findings,
  verdicts, and remediation options to the user. Risk acceptance is theirs.

Ordering note: V runs before R so reviewers read *live-verified* code and
live ground truth is fixed cheaply first; R's fixes then get their own live
regression pass below. The worktrees stay stable during each phase — never
run R's fixers while V's stack is serving from the same trees.

## Phase V′ — Targeted regression re-verify (only if R applied fixes)

The Workflow call below is Claude-only. Other profiles re-dispatch the same
trusted verification routes against the scoped regression contract.

If any review round changed code, re-run the live layer scoped to what moved:

```
Workflow({ scriptPath: ".../forge-verify.js", args: { featureId, specDir, repos,
  verification, leadRepo, mode: "regression", maxFixRounds: 1,
  regressionScope: { repos: <repos forge-review fixed>,
                     scenarioIds: <scenario ids that failed in Phase V> } }})
```

Regression mode reruns: tracks bound to the fixed repos (plus repo-unbound
cross-service tracks), P1 stories as smoke, and any scenario that ever failed.
`pass` → ship. Anything else → same strict handling as Phase V. If R applied
no fixes, skip V′ — Phase V's evidence stands.

## Phase S — Ship (main loop, parallel)

Only when review status is shippable, the latest verification status is
`pass` or `not-applicable` (or `--skip-verify` was explicitly given), **and**
every worktree HEAD matches the SHA the review panel examined — otherwise the
post-review drift gate (Failure & stop conditions) runs first:

1. Per repo, verify the worktree is clean (`git status --porcelain` empty —
   implementers commit their work; a dirty tree here is a defect: stop for
   that repo and report).
2. In parallel per repo: `git push -u origin feat/<id>` then `gh pr create
   -R alkem-io/<repo> --base <default_branch>` with title `feat(<id>): <short
   title>` and body containing: what/why summary, `workspace#<id>` reference,
   spec/plan/tasks links, evidence digest (review rounds, findings
   fixed/declined, security verdict + control notes, gate results), tracked
   residuals, and the footer `🤖 Generated with [Claude Code](https://claude.com/claude-code)`.
   Each `gh pr create` automatically triggers the workspace CodeRabbit-triage
   hook — do not triage yourself.
3. Update `specs/<id>/repos.yaml` `pr:` fields. Write the evidence ledger
   (below). Commit the workspace feature branch as **one** signed commit
   (`git commit -S` — may need a hardware-key touch; batch, don't spam).
4. Report the final table.

## Evidence ledger — `specs/<id>/forge-run.md`

Audit-grade record (ISO 27001 A.8.32 / SOC 2 CC8.1 change evidence). Written
at ship time (or at stop time on any abort), containing: run date + operator;
models actually used per role (including any fable→opus fallback); the
standing accepted residual that trusted hands-free implementers run with full
tools inside worktrees under their charters, while DeepSeek writes run inside
the enforced public-repository sandbox; intake
answers or `none`; the council record when convened — per-seat position
summaries, objections and concessions, contested forks with their rulings,
and the dissent log (seat → objection → ruling → risk-register entry);
architect loop counts + assumptions + risks; implement
waves, contract check outcomes, gate audit results; **verification evidence**
— stack isolation record (compose project, port mode, seed mode), per-track
results, the acceptance table (scenario ID → pass/fail → evidence screenshot
path under `worktrees/<id>/.forge/evidence/`), the **risk-coverage table**
(register entry → mitigated/partially-verified/at-risk/unmitigated — the
ISTQB-style residual-risk exit report, and a direct input to the Release NN
risk profile), fix-round history, emitted `@forge-acceptance` specs, external
test debt, and any stack-conflict or `--skip-verify` in bold (this is the
machine-run evidence behind the release checklist's QA acceptance line — the
sign-off itself stays human);
review round history (found/killed/confirmed/fixed per round), every confirmed
finding with disposition and — for security — SOC 2/ISO control mappings and
the final verdict + `notes_for_release_risk_profile` (feeds the Release NN
risk table); V′ regression results; post-review drift-gate runs (triggering
commits → verdicts); residual items and who must decide them; PR URLs; human
gates left open.

The ledger is an index, not the only recovery artifact. Before every layer
hands off, write an append-only asset under `specs/<id>/forge/` using
`.scripts/forge-control.py write-handoff`, then require `verify-handoff` to
pass. This applies to every council position/clash/chief, repo implementation,
contract, gate, verification track/story, review dimension, finding lens,
regression, and ship result. Each handoff records provider, requested/actual
model and effort, runner, billing mode, token/cache usage, reported or marginal
USD cost, repo SHAs, parent/body hashes, and only a session fingerprint. Raw
session IDs remain in ignored mode-0600 `.forge/state`; never commit them or
hidden chain of thought. Use the protocol's stable-prefix prompt order for
cache reuse. Resume a session only when provider/model, prompt schema, parent
hashes, and repo SHAs still match; otherwise resume from the written asset.
At each phase checkpoint, run `forge-control.py summarize-handoffs
specs/<id>/forge` and carry its per-provider token/cache/cost totals into the
ledger without inventing API-equivalent prices for subscription calls.

## Failure & stop conditions

- Architect fails twice → stop, report artifacts so far.
- A repo's implementer reports `blocked` → finish the others, report the
  blocker; never silently drop a repo from the feature.
- Security-hold, or residual severity > medium → no ship, full report.
- Verification `fail`/`stack-conflict`/`env-degraded` → no ship (unless
  `--skip-verify`); the stack runner never kills processes it didn't start,
  so a conflict costs nothing but a `--resume --from verify` after you free
  the ports. `external_test_debt` (failures in repos without a feature
  worktree, e.g. stale test-suites specs) is reported for follow-up, never
  silently fixed in a sibling clone.
- **Post-review drift gate.** Review verdicts bind to the exact SHAs the
  panel examined (recorded in the ledger). ANY later commit on a feature
  branch — scope amendments, CodeRabbit-triage fixes, operator-feedback
  changes, even spec-dir-only edits (the spec is the contract being drifted
  from) — re-arms a scoped re-review before the work is again treated as
  review-clean: run `forge-review.js` for the changed repos with
  `dimensions: ["spec-compliance"]` (security rides along automatically —
  it is non-removable by construction), remediate confirmed findings through
  the normal fix flow, and record the gate run (triggering commits →
  verdict) in the ledger. This applies before ship and equally after PRs
  are open.
- Budget floors inside the workflows stop loops gracefully — their partial
  results are still valid input for a `--resume`.
- Everything is resumable: artifacts are on disk, workflows support
  `resumeFromRunId`, and `--resume <id> --from <phase>` re-enters anywhere.

## Hard rules

- Never merge PRs, never deploy, never flip the story to Done — humans do.
- Never edit the vendored `speckit-*` skills or `.specify/` engine.
- Never weaken the security dimension: it runs on every /forge, every round
  that touches code, and its `fail` verdict is a hard stop.
- The model routing table is contract, not suggestion — if a pinned model is
  unavailable, use the documented fallback and record it; never quietly drop
  to a cheaper tier.
