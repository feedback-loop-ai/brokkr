# 0033 — Contributing goes through Brokkr

Status: accepted (operator ruled in chat, 2026-09-02)
Date: 2026-09-02

## Context

The contributing guide had become a 610-line second implementation of
the verify seat: nine commands, coverage refusal shapes, signing, frozen
surfaces, and decision culture. A contributor had to operate that
handbook before receiving the machine's own verification and review.
The operator ruled instead: "I want contributing to be via brokkr,
mandatory, and that to be in the contributing guidelines. So if we have
which strategy / recipe to run, it will essentially be a 60s read - that
should be our goal."

"Mandatory" needs a platform fact, not an agent's opinion. The fact must
also travel from a fork without publishing the operator's SQLite
journal. Committing a post-run export to the pull request cannot bind the
final head: that evidence commit changes the very head it is meant to
vouch for. The existing synthetic anchor ref does not change the working
branch, so it is the smaller carrier.

## Rulings

1. Every pull request to `main` is delivered by a Brokkr run and names
   it in the body as exactly `Brokkr-Run: <run-id>`. Native run ids have
   the grammar `[a-z0-9-]{0,32}-[0-9a-f]{8}`: the engine's at-most
   32-character feature slug, a separator, and eight lowercase hex
   characters.
2. The evidence is the run's anchor, pushed from local
   `refs/forge/<run-id>` to the contributor fork's
   `refs/heads/brokkr-runs/<run-id>`. Publishing evidence never changes
   the pull request head.
3. A new anchor is `forge.journal-anchor/v2`. Its synthetic commit tree
   contains the canonical `<run-id>.ndjson` export and its JSON commit
   message records `run_id`, journal sequence and head hash, and
   `repo_head`, the reviewed commit the run then drift-checked at ship.
   Anchors remain unsigned tamper-evidence, not identity proof; this
   ruling does not claim otherwise.
4. The CI job named `delivered by brokkr` is deterministic. It fetches
   the named evidence ref from the pull request's head repository, runs
   the base branch's `brokkr verify-run` over the embedded journal, and
   requires all of: a valid hash chain and fold; the declared run id;
   completed status and a `shipped` decision from the ship phase; the
   anchor's sequence and journal head matching the export's last event;
   and the anchor's `repo_head` matching the platform's pull request head
   SHA. No model or agent rules on any part of it.
5. The operator-only `by-hand` label is the escape hatch. When present,
   the same job succeeds without evidence and prints that the operator's
   label caused the skip; the exception is visible in the required
   check's own output. The job reads labels from the `pull_request`
   event that started it, and a re-run replays that event, so after
   applying the label the operator closes and reopens the pull request
   (or pushes) to make the skip take effect.
6. After this change lands, the operator must add `delivered by brokkr`
   to `main`'s required checks. Branch protection belongs to the
   platform operator, so repository code can name and exercise that step
   but cannot install the protection itself.

## Enforcement bindings

- [`.github/pull_request_template.md`](../../.github/pull_request_template.md)
  carries the declaration line.
- [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml) owns the
  platform job, grammar, offline verification, head comparison, and
  visible escape hatch.
- [`crates/brokkr-runtime/src/anchor.rs`](../../crates/brokkr-runtime/src/anchor.rs)
  writes the v2 carrier without touching the working branch.
- [`crates/brokkr-cli/tests/contributing.rs`](../../crates/brokkr-cli/tests/contributing.rs)
  keeps the short guide's one recipe table equal to the compiled recipe
  library and pins the contribution surfaces named above.

## Consequences

The contributor path is one screen: install, fork, choose a recipe, run
it, review, publish its anchor, and open the pull request naming the run.
The old handbook remains intact as the by-hand reference, but it is no
longer the ordinary workflow; verify owns those checks.

The pull request that introduces this ruling is anchored at completion
by an engine that predates v2, so its published evidence is written
afterwards with the branch's own binary,
`cargo run -p brokkr-cli -- anchor --run <run-id>`, which chains a v2
anchor onto the v1 tip before the ref is pushed. Every later run writes
v2 at conclusion by itself.

Evidence refs add one synthetic branch per proposed run to contributor
forks. They may be deleted after the pull request lands. A rebase or any
post-run commit deliberately makes the check fail because the vouched
head is no longer the proposed head; run Brokkr again, or ask the
operator to make the visible `by-hand` exception.

Seat commits are unsigned, while `main` requires signatures. The
operator squash-merges, and the platform-created squash commit is the
signed commit that reaches `main`.
