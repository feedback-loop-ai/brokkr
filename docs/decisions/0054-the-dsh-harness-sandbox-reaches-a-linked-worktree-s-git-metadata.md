# 0054 — A linked worktree's git metadata: the dsh driver supplies a scoped runner, and only bubblewrap can stand in

Status: proposed
Date: 2026-09-09

> **Number.** This decision was drafted as 0053 and renumbered to 0054 on
> 2026-09-09: issue #219's provider-refusal decision claims 0053 on
> `fire/219-limit-fallback`, and the constitution's rule is that the next
> free number is claimed in the PR. 0053 in this branch's history means
> this document.

## Context

On 2026-09-08 two dsh implementation runs,
`close-issue-218-doctor-probes-a--71197022` and
`close-issue-221-muninn-s-two-res-2f692419`, implemented their slices,
reported the runnable checks green, and then answered `blocked`: `git
add` / `git commit` could not create
`<parent>/.git/worktrees/<name>/index.lock`. The harness had lit each
fire in a linked `git worktree`, whose metadata lives under the shared
repository's `.git`, and the dsh harness confines writes to the seat's
own workspace. The operator committed both artifacts by hand as
`243b1f3` and `dbe6e4a`. The same defect is already described in
[docs/essays/one-job-three-hires.md](../essays/one-job-three-hires.md),
and the workaround — commit at the bench, run the rematch in a
standalone clone — is a referee's patch, not a delivery.

What is measured, from the installed `@deepseek-ai/dsh` 0.1.2-rc.1 and
its packages, not assumed:

- **dsh's own sandbox is the boundary in question.** The adapter declares
  `hands` unsupported (decision 0043's consequences), so a dsh seat runs
  under decision 0046's `harness` boundary, confined by
  `@deepseek-ai/dsh-sandbox-local`.
- **That boundary cannot express an extra writable root.**
  `SandboxExecutionPolicy` is `{mode, workspaceRoot}`; `writableRoots`
  is the workspace plus platform temp areas; `bwrapProfileArgs` emits
  `--ro-bind / /` and, for `workspace-write`, only `--bind
  <workspaceRoot> <workspaceRoot>`. A linked worktree's per-worktree
  directory and the shared `objects`/`refs` lie outside that root.
- **Escalation is not a bounded answer.** The only wider mode is
  `danger-full-access`, the approval request carries no tool arguments,
  and a headless deployment with no answerer resolves `unavailable`.
  Approving it would run the whole command unconfined.
- **The provider's supported extension point is `runnerCommand`.** It
  names a bwrap-compatible runner that receives the composed profile
  argv, and it is settable through the `--patch` overlay the driver
  already writes: `dsh --profile headless --patch <row> --dump-config`
  composes the row over `@deepseek-ai/dsh-sandbox-local`. Re-measured
  on 0.1.2-rc.1 (2026-09-09): a row with the trusted flags composes
  verbatim, so the runner sees every path the driver resolved.
- **Brokkr's namespace boundary already answers the same problem.**
  Decision 0043 ruling 6 binds a worktree's external git directory with
  its hooks hidden and its config read-only. This decision does the same
  thing for the harness-owned sandbox, through the harness's own runner,
  and scoped more tightly than a whole-common-dir bind.

Alternatives weighed:

- **Set the dsh workspace to the parent checkout.** Rejected: it makes
  the whole checkout writable and is exactly the "blindly mount the
  parent" the boundary forbids.
- **Escalate to `danger-full-access` for a commit.** Rejected: the
  approval request carries no command, so the grant cannot be scoped to
  one trusted committer; it would run anything the model asked.
- **Ask the seat to commit through a Brokkr broker outside the box.**
  Rejected as the first slice: it adds a socket protocol, a `git` shim
  and a second writer while a supported kernel boundary (bubblewrap
  through the harness's runner) is available and testable.
- **Do nothing and keep committing at the bench.** Rejected: it repeats
  the workaround the commission exists to remove, and it lets a run burn
  an implementation before discovering it cannot deliver.

## Rulings

1. **The driver resolves the two git directories before the seat
   starts.** `brokkr driver dsh` runs
   `git rev-parse --path-format=absolute --git-dir --git-common-dir` in
   the seat's workdir, outside any sandbox, and carries the result as
   trusted data. It never re-resolves them from a workspace file the
   model can edit.

   Resolving them early is necessary but NOT sufficient, and this ruling
   does not claim otherwise: Git answers that question by FOLLOWING the
   workspace's own `.git` file, which is a file the model can write, so
   what the driver resolves early is still a value a previous seat could
   have influenced. Ruling 3's layout proof is what makes it trustworthy.

   **Enforcement binding:** `GitFacts` in `brokkr-protocol::hands`
   (per-worktree `git_dir` and shared `common_dir`, each `None` for a
   non-repository or an ambiguous path); `dsh_git_runner_scope` in
   `adapters.rs`; unit tests over a primary checkout, a linked
   worktree, a subdirectory, a missing repository, and both
   non-confining modes.

2. **Only a confining workspace-write seat whose git metadata lies
   outside the session workspace gets the scoped runner.** A primary
   checkout whose workspace IS the repository root, a `read-only` seat
   and a `danger-full-access` seat are left to dsh's own provider; the
   driver changes nothing for them.

   **Enforcement binding:** `dsh_git_runner_scope` returns `None` for
   each of those cases; `dsh_sandbox_row_for` refuses an unserved layout
   and otherwise writes the row.

3. **Exactly one layout is served: a linked worktree Git itself created,
   proved against the host.** A scope whose common directory lies outside
   the workspace is served only when all three of these hold, and is
   refused — early at seat start and again in the runner, with the
   remedy named — when any of them does not:

   - **The git directory is not the shared repository.** `git_dir ==
     common_dir` means a commit needs `index.lock` and `HEAD.lock`
     inside the shared `.git`, so the only writable answer is the whole
     shared `.git`, every sibling worktree and every submodule git
     directory with it. A primary checkout reached through a
     subdirectory, a `--separate-git-dir` checkout and a worktree `.git`
     file redirected at the parent all resolve here.
   - **The git directory sits where Git would have put it**, at
     `<common_dir>/worktrees/<name>`, compared with symlinks resolved.
     Without this, a seat that writes `<workspace>/.git` ->
     `gitdir: <workspace>/fake` and fills `<workspace>/fake/commondir`
     with an unrelated repository's path makes Git report that
     repository as the common directory — and the runner would then bind
     its `objects`, `refs` and `logs` read-write for the next seat in
     the same worktree.
   - **The `gitdir` back-pointer exists and names this workspace's own
     `.git`.** The topology check alone is not enough, because a
     workspace `.git` file may name a REAL administrative directory
     belonging to another worktree of another repository. The
     back-pointer lives inside the git directory, outside the workspace
     and outside anything a seat can write, so it is the trustworthy end
     of the pair. A directory with no back-pointer is refused, not
     excused: git writes one for every worktree it creates.

   A back-pointer that names a worktree CONTAINING this workspace is the
   subdirectory case, and it is refused by its own name — the seat was
   started below its worktree's root, nothing is pointing at another
   worktree, and the remedy is the root the pointer names.

   **Enforcement binding:** `scope_refusal` in
   `brokkr-protocol::dsh_sandbox`, called by `dsh_sandbox_row_for`
   before the seat starts and by `runner_argv` on every command; unit
   tests for each arm, including the workspace-local redirect measured
   against a real `git rev-parse` in the behavioral proof.

4. **The runner is `brokkr dsh-sandbox-runner`, named in the sandbox
   row's `runnerCommand`, and it adds exactly the scoped write set.**
   The per-worktree git directory, `objects`, `refs`, `logs` and
   `packed-refs` are writable; `hooks` is an empty tmpfs, per-worktree
   and shared; the shared `config` is read-only; the per-worktree
   `config` and `config.worktree` are masked; `commondir` and `gitdir`
   are bound read-only. The whole shared `.git`, the parent checkout and
   every sibling worktree directory stay outside the write set.

   `commondir` and `gitdir` are read-only because each is a pointer git
   follows to a config: left writable, a boxed command could redirect the
   host's next `git` invocation at a `config` it wrote in the workspace.
   They are HARD binds rather than `-try`, so a missing pointer fails the
   run instead of silently leaving the box free to create one.

   The two per-worktree config paths are MASKED rather than bound
   `--ro-bind-try`, because that flag no-ops when the host file is absent
   — the normal state of a linked worktree — while the writable directory
   around it would let the box create one the host then honours under
   `extensions.worktreeConfig`. Bubblewrap creates the mount point for a
   missing destination, so the host gains an empty file and the box gains
   no way to fill it.

   **The mask source is an empty regular file the driver stages, never
   `/dev/null`.** Bubblewrap mounts every bind source with `MS_NODEV`, so
   a bound character device cannot be opened inside the box, and git
   answers an unreadable configuration file with `fatal: unknown error
   occurred while reading the configuration files` — which would break
   every git command in the seat, in exactly the repositories the mask
   exists for. The driver holds the staged file for the seat's whole
   life and names it in the runner's argv; the runner refuses a mask that
   is missing, is not a regular file, is not empty, or lies inside the
   seat's own writable workspace, because a bind is only as read-only as
   its source is unreachable.

   The runner's own flags (`--workspace`, `--git-dir`, `--common-dir`,
   `--bwrap`, `--mask`) are the driver's trusted paths, and a profile
   that does not root at `--ro-bind / /`, or has no `--` before the
   command, is refused rather than guessed at. A profile with no
   workspace bind (a `read-only` seat) gets no added bind.

   **Enforcement binding:** `brokkr-protocol::dsh_sandbox`
   (`runner_argv`, `scope_refusal`, `mask_refusal`, `scoped_git_binds`,
   `stage_mask_file`, `sandbox_row`); the `--patch` row
   `dsh_seat_overlay_with` composes; `brokkr dsh-sandbox-runner`
   dispatched before clap in `brokkr-cli`; unit tests over the argv, the
   row, every refusal arm, and a behavioral test that stages and commits
   in a real linked worktree under dsh's real bwrap profile, rooted
   outside the profile's `/tmp` tmpfs and asserting the host's bytes
   back.

5. **Unsupported hosts refuse at seat start, not at commit time, and the
   runner execs the bubblewrap the driver probed.** A host that is not
   Linux, has no `bwrap` on `PATH`, or has a `bwrap` that cannot build
   the empty-root namespace refuses before the model runs, naming the
   reason and the remedies (a standalone checkout, or a boundary Brokkr
   builds). There is no fallback to a wider mode and no simulated
   boundary. The absolute path the driver resolved and probed travels as
   the runner's `--bwrap`; the runner execs that path and never searches
   `PATH` again, so the boundary is the binary the driver measured rather
   than whatever the seat's environment holds.

   **Enforcement binding:** `dsh_bwrap` / `dsh_bwrap_on` and the
   non-Linux arm; `dsh_sandbox::require_usable_bwrap`; the `--bwrap`
   flag in `sandbox_row` / `runner_argv`; tests for a missing `bwrap`,
   an unusable one, the refusal text, a relative `--bwrap`, and a decoy
   `bwrap` on `PATH` that the runner must not exec.

6. **Seat commits are unsigned and attributed to the host.** The driver
   sets `GIT_CONFIG_COUNT=1`, `GIT_CONFIG_KEY_0=commit.gpgsign`,
   `GIT_CONFIG_VALUE_0=false`, and the host's `GIT_AUTHOR_*` /
   `GIT_COMMITTER_*` on the dsh child, the same environment discipline
   the namespace box applies (decision 0043 ruling 6; CONTRIBUTING).

   **Enforcement binding:** the child environment in `invoke_dsh_with`;
   a test whose fake dsh dumps its environment.

7. **A boundary proof that cannot run is a failure on the host whose job
   is to run it.** Every proof that needs a real namespace skips on a
   machine that cannot open one — a developer's laptop, a nested box —
   and a skipped Rust test prints `... ok`, which is why a red behavioral
   test once reached a commit. A host that sets
   `BROKKR_REQUIRE_BOUNDARY_EVIDENCE` fails the proof instead of
   skipping it, and CI's Linux legs set it, so the evidence is either
   produced or the job is red. It is never a reason to weaken a proof.

   **Enforcement binding:** `hands::BOUNDARY_EVIDENCE_ENV`,
   `boundary_evidence_required` and `skip_boundary_proof`; every
   namespace-dependent skip routed through it; the `engine` and
   `coverage` Linux jobs in `.github/workflows/ci.yml` setting it; unit
   tests over both arms of the guard.

8. **The boundary does not move.** The seat still stands under
   `harness`; the runner refines the harness's own sandbox and adds no
   new boundary value, no manifest key and no bundle identity. The
   record still reads `harness`, and the fact that the harness's runner
   is Brokkr's is stated in the guide.

   **Enforcement binding:** no contract or `policy/` change; the guide's
   dsh section; the frozen-contracts test unchanged.

## The threat model

The adversary is a previous seat in the same worktree — implement,
verify and review all reuse it — that can write anything inside its own
workspace and nothing outside it. What it may try, and what stops it:

| Attempt | Stopped by |
|---|---|
| Rewrite `<workspace>/.git` to point at a fake administrative directory in the workspace, with `commondir` naming another repository | Ruling 3's topology check: `git_dir` must be `<common>/worktrees/<name>` |
| Point `<workspace>/.git` at a REAL administrative directory of another worktree or another repository | Ruling 3's back-pointer check, read from a file outside the workspace |
| Reach a repository through a symlinked alias so the paths compare unequal | Every comparison resolves symlinks before comparing |
| Rewrite `commondir` or `gitdir` from inside the box, so the HOST's next `git` reads a config the box wrote | Both bound read-only and hard: write is `EROFS`, unlink and rename over the mount point are `EBUSY` |
| Create `config.worktree` so `extensions.worktreeConfig` makes the host honour a `core.hooksPath` the box chose | Masked with an empty read-only regular file, whether or not the host file exists |
| Fill the mask itself through its source path | The source is staged outside the workspace and refused if it is inside one; dsh's own profile replaces `/tmp` with a fresh tmpfs, so the box has no path to it |
| Plant a hook the host later runs | `hooks` is an empty tmpfs, per-worktree and shared |
| Write a sibling worktree's checkout, index or `HEAD` | Nothing under `<common>/worktrees/<other>` is bound, and the sibling's checkout directory is not in the write set |
| Write the parent checkout, or any credential path | Neither is bound; the profile's root is read-only |
| Move a sibling's BRANCH through the shared ref store | **Not stopped.** See the residual below |

## Consequences

- **Linux and bubblewrap only.** A linked-worktree dsh seat on macOS or
  Windows, or on a Linux host without a usable `bwrap`, refuses at
  start. dsh's `runnerCommand` is a bwrap-compatible contract, so no
  other platform's sandbox is claimed.
- **The scoped set shares `objects`, `refs`, `logs` and `packed-refs`
  with the repository**, because a commit writes there. **This is a real
  residual, and it is a ref-store residual, not a worktree one:** a seat
  can run `git update-ref refs/heads/<other>` and move a branch a sibling
  worktree has checked out. What the boundary promises for a sibling is
  its checkout directory and its per-worktree metadata — `HEAD`,
  `index`, `config.worktree` — and those stay unwritable and are proved
  so. Narrowing the shared ref store further is not possible under a
  kernel bind (git takes its lock in the containing directory); it would
  need the broker this decision rejected as a first slice, and that is a
  new number.
- **Only a linked worktree at its own root is served.** A primary
  checkout reached through a subdirectory, a `--separate-git-dir`
  checkout, a `.git` file redirected at the parent or at another
  worktree, an administrative directory outside `<common>/worktrees`, and
  a seat rooted in a SUBDIRECTORY of its linked worktree all refuse at
  start, each naming its own cause and remedy. The subdirectory case is
  refused rather than served because dsh's writable root would then be
  the subdirectory alone; serving it is a widening, not a bug fix.
- **A workspace whose `.git` file has been damaged refuses the NEXT
  seat, and says so at start.** That is the fail-closed direction — the
  alternative is binding whatever the damaged file points at — but it
  does mean a seat can leave a worktree unusable for its successors. The
  remedy is `git worktree repair`, and the refusal arrives before an
  implementation is spent rather than after it.
- **The masks create empty files on the host when they were absent.**
  Bubblewrap creates the mount point for a missing destination; the box
  sees an empty read-only file, and the host file stays empty. The
  alternative — `--ro-bind-try` — silently protects nothing when the file
  is absent. `<git_dir>/config` gains the same empty file; git reads a
  linked worktree's `config` from the shared directory, so an empty one
  beside it is inert.
- **The runner is an operator assertion to dsh.** dsh skips its own
  runner probe when `runnerCommand` is set, so the driver probes the
  same `bwrap` itself and refuses on a failure; the runner also prints
  the `brokkr-dsh-sandbox-runner:` signature dsh classifies as a runner
  failure, never as a denied command.
- **What this does not do.** It does not make the seat's git history
  private, does not sign commits, and does not bound egress (decision
  0036). It does not give dsh `hands`, and it does not touch the
  namespace boundary's own git handling. That handling is wider: 0043
  ruling 6 binds a linked worktree's whole shared `.git` read-write with
  `hooks` masked and `config` read-only, which leaves the per-worktree
  `commondir` and `gitdir` writable and carries the same redirect
  residual this decision closes for the harness runner. Tightening the
  namespace box is a change under 0043, not here.

## The question this decision asks

Ruling 8 is the semantic choice, and it is the operator's: **does a
realm's `harness` boundary authorize Brokkr's driver to refine the
harness's own sandbox through the harness's supported runner, for the
scoped git metadata a linked worktree needs?** This decision proposes
yes, and the code ships behind it. If the operator rules otherwise, the
alternatives are a new boundary value that Brokkr builds (0046 slice
(iii)'s `container`), or a first-class scoped-commit mechanism outside
the harness; either is a new number, not an edit here.
