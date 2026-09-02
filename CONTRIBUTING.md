# Contributing to Brokkr

Every pull request to `main` goes through Brokkr. The run implements,
verifies, reviews, and prepares the change; you review the result and
open the pull request that names the run.

## 1. Install Brokkr

Use a one-command [release channel](packaging/README.md#using-the-channels):

```console
cargo binstall brokkr-cli
```

The main README also lists the checksum-verified archives and source
bootstrap. Those release and bootstrap paths are the authority.

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
| `crucible` | Engine, store, protocol, or contract changes needing an Opus review sequence. | implement, review[positions>chief], ship, verify | high |
| `ember` | Docs, chores, and small fixes with a cheap intake and economical working seats. | implement, intake, review, ship, verify | low |
| `fast` | Default Rust delivery from implementation through verification, review, and ship. | implement, review, ship, verify | medium |
| `night-shift` | Unattended work that should park on the first unusual result instead of retrying. | implement, review, ship, verify | medium-high |
| `node` | Node and TypeScript repositories using JavaScript-specific seats and tools. | implement, review, ship, verify | medium |
| `panel-review` | General delivery needing independent correctness and security reviewers. | implement, intake, review[correctness+security], ship, verify | high |
| `preflight` | Verify and review an existing branch without implementing or shipping it. | review, verify | medium |
| `sdd` | Spec-driven work that needs a design panel, chief synthesis, and spec-kit check. | design[positions>chief>speckit-check], implement, intake, review[security+spec-compliance], ship, verify | high |
| `sdd-paranoid` | Spec-driven high-risk work needing adversarial and security review. | design[positions>chief>speckit-check], implement, intake, review[adversarial+security], ship, verify | very high |
| `wager-harness` | Driver evaluation that swaps only implementation to Codex for a fair wager. | implement, review, ship, verify | medium |
| `wager-harness-dsh` | Driver evaluation that swaps only implementation to DSH for a fair wager. | implement, review, ship, verify | medium |
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
requires a completed run, and requires its vouched commit to equal the
pull request head. Only the operator may apply the visible `by-hand`
escape-hatch label.

Seat commits are unsigned; `main` requires signatures; the operator squash-merges, and that merge is the signed commit.

Curious about the machinery? [The by-hand guide](docs/guides/contributing-by-hand.md) preserves the nine exact checks, coverage practicalities and refusals, signing walkthrough, decision culture, and frozen surfaces; the verify seat runs them, so contributors do not need to.

Contributions are dual licensed under Apache-2.0 OR MIT unless you say
otherwise.
