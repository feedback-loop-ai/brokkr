# Contributing to Brokkr

Every pull request to `main` goes through Brokkr. The run implements,
verifies, reviews, and prepares the change; you review the result and
open the pull request that names the run.

## 1. Install Brokkr

Download the checksum-verified archive from the
[latest release](https://github.com/feedback-loop-ai/brokkr/releases/latest)
exactly as the [main README](README.md#60-second-bootstrap) shows, or build the
binary from your clone after step 2:

```console
cargo install --path crates/brokkr-cli
```

Those two README paths are the authority. `cargo binstall brokkr-cli`
reads the crate the release workflow publishes to crates.io at each tag
from v0.9.0 ([packaging](packaging/README.md)).

## 2. Fork and clone

```console
gh repo fork feedback-loop-ai/brokkr --clone
cd brokkr
git switch -c <your-branch>
```

## 3. Pick one recipe

Costs are relative, not quotes; provider rates and retries vary. Pick a
delivery recipe; `preflight` is an optional branch check, not a delivery.

<!-- recipe-table:start -->
| Recipe | When to use it | What it seats | Rough cost |
|---|---|---|---|
| `fast` | Default Rust delivery from implementation through verification, review, and ship. | implement, review, ship, verify | medium |
| `night-shift` | Unattended triage routing that parks on the first unusual result and uses the dsh implementation lane. | analyze[check>judge], clarify[check>judge], design[positions>chief>validate], implement, review{chore=reviewer;design=positions>chief;engine=positions>chief;feature=review-correctness+review-security}, ship, specify[author>validate], tasks[author>validate], triage, verify[checks>dialect-verify] | medium-high |
| `node` | Node and TypeScript repositories using JavaScript-specific seats and tools. | implement, review, ship, verify | medium |
| `panel-review` | General delivery needing independent correctness and security reviewers. | implement, intake, review[correctness+security], ship, verify | high |
| `preflight` | Verify and review an existing branch without implementing or shipping it. | review, verify | medium |
| `research` | Read articles and propose registry entries with cited classifications; the operator rules them. | research, verify | low |
| `research-dsh` | The research intake on the dsh lane: the same charter and gate, the researcher seated on Qwen3.8-Max with page fetch turned on. | research, verify | low |
| `standby` | Fast's shape with every model seat on the other vendor at judge-grade effort, so delivery survives one account's exhausted limit. | implement, review, ship, verify | medium |
| `triage` | Routing delivery: a chief-grade triage gate rules the class before Fast's crew, adding the current SDD design council when ruled. | analyze[check>judge], clarify[check>judge], design[positions>chief>validate], implement{chore=implementer;design=implementer-sdd;engine=implementer-engine;feature=implementer}, review{chore=reviewer;design=positions>chief;engine=positions>chief;feature=review-correctness+review-security}, ship, specify[author>validate], tasks[author>validate], triage, verify[checks>dialect-verify] | variable |
| `wager-harness` | Driver evaluation that swaps only implementation to Codex for a fair wager. | implement, review, ship, verify | medium |
| `wager-harness-dsh` | Driver evaluation that swaps only implementation to DSH for a fair wager. | implement, review, ship, verify | medium |
| `wager-harness-muse` | Driver evaluation that swaps only implementation to Muse Spark 1.3 on its contributor terms, through dsh, for a fair wager. | implement, review, ship, verify | low |
<!-- recipe-table:end -->

## 4. Light the run

```console
brokkr run --recipe <name> --feature "<what you want>"
```

Let the machine finish its verify, review, and ship seats. Fixes return
through the same policy; do not substitute hand-run checks for the run.

## 5. Review, publish the evidence, and propose

Review the branch and the run. Brokkr writes the final anchor locally;
publish its synthetic commit beside your branch without changing HEAD:

```console
git push origin refs/forge/<run-id>:refs/heads/brokkr-runs/<run-id>
```

Open the pull request using the repository template and replace its line
with `Brokkr-Run: <run-id>`. CI verifies the published journal offline,
requires a completed run, and cuts a tier by what changed since it
judged (decision 0038): a rebase that leaves your slice's patch unchanged
keeps the vouch; a delta confined to docs needs `brokkr run --recipe
preflight` on the new head, named as `Brokkr-Preflight: <run-id>`; a code
delta needs a new run. Only the operator may apply the visible `by-hand`
escape-hatch label.

Seat commits are unsigned; `main` requires signatures; the operator squash-merges, and that merge is the signed commit.

Curious about the machinery? [The by-hand guide](docs/guides/contributing-by-hand.md) preserves the nine exact checks, coverage practicalities and refusals, signing walkthrough, decision culture, and frozen surfaces; the verify seat runs them, so contributors do not need to.

Contributions are dual licensed under Apache-2.0 OR MIT unless you say
otherwise.
