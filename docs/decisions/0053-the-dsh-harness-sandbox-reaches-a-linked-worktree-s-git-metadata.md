# 0053 — A linked worktree's git metadata: the dsh driver supplies a scoped runner, and only bubblewrap can stand in

Status: proposed
Date: 2026-09-09

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
  on 0.1.2-rc.1 (2026-09-09): a row with the four trusted flags
  (`--workspace`, `--git-dir`, `--common-dir`, `--bwrap`) composes
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
   model can edit, so a rewritten `.git` file cannot redirect the
   runner's binds.

   **Enforcement binding:** `GitFacts` in `brokkr-protocol::hands`
   (per-worktree `git_dir` and shared `common_dir`, each `None` for a
   non-repository or an ambiguous path); `dsh_git_runner_scope` in
   `adapters.rs`; unit tests over a primary checkout, a linked
   worktree, a subdirectory, a missing repository, and both
   non-confining modes.

2. **Only a confining workspace-write seat whose git metadata lies
   outside the session workspace gets the scoped runner.** A primary
   checkout, a `read-only` seat and a `danger-full-access` seat are left
   to dsh's own provider; the driver changes nothing for them.

   **Enforcement binding:** `dsh_git_runner_scope` returns `None` for
   each of those cases, and `dsh_sandbox_row_for` then writes no row.

3. **The runner is `brokkr dsh-sandbox-runner`, named in the sandbox
   row's `runnerCommand`, and it adds exactly the scoped write set.**
   The per-worktree git directory, `objects`, `refs`, `logs` and
   `packed-refs` are writable; `hooks` is an empty tmpfs; `config`,
   `config.worktree`, `commondir` and `gitdir` are read-only. The whole
   shared `.git`, the parent checkout and every sibling worktree stay
   outside the write set. The worktree's `commondir` and `gitdir` are
   read-only with the config files because each is a pointer git follows
   to a config: left writable, a boxed command could redirect the host's
   next `git` invocation at a `config` it wrote in the workspace. A
   read-only bind refuses the write (EROFS) and refuses unlinking or
   renaming over the mount point (EBUSY). The runner's own flags
   (`--workspace`, `--git-dir`, `--common-dir`, `--bwrap`) are the
   driver's trusted paths, and a profile that does not root at
   `--ro-bind / /`, or has no `--` before the command, is refused rather
   than guessed at. A profile with no workspace bind (a `read-only` seat)
   gets no added bind.

   **Enforcement binding:** `brokkr-protocol::dsh_sandbox`
   (`runner_argv`, `scoped_git_binds`, `sandbox_row`); the `--patch`
   row `dsh_seat_overlay_with` composes; `brokkr dsh-sandbox-runner`
   dispatched before clap in `brokkr-cli`; unit tests over the argv and
   row, and a behavioral test that stages and commits in a real linked
   worktree under dsh's real bwrap profile, rooted outside the profile's
   `/tmp` tmpfs and asserting the host's bytes back.

4. **Unsupported hosts refuse at seat start, not at commit time, and the
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

5. **Seat commits are unsigned and attributed to the host.** The driver
   sets `GIT_CONFIG_COUNT=1`, `GIT_CONFIG_KEY_0=commit.gpgsign`,
   `GIT_CONFIG_VALUE_0=false`, and the host's `GIT_AUTHOR_*` /
   `GIT_COMMITTER_*` on the dsh child, the same environment discipline
   the namespace box applies (decision 0043 ruling 6; CONTRIBUTING).

   **Enforcement binding:** the child environment in `invoke_dsh_with`;
   a test whose fake dsh dumps its environment.

6. **The boundary does not move.** The seat still stands under
   `harness`; the runner refines the harness's own sandbox and adds no
   new boundary value, no manifest key and no bundle identity. The
   record still reads `harness`, and the fact that the harness's runner
   is Brokkr's is stated in the guide.

   **Enforcement binding:** no contract or `policy/` change; the guide's
   dsh section; the frozen-contracts test unchanged.

## The question this decision asks

Ruling 6 is the semantic choice, and it is the operator's: **does a
realm's `harness` boundary authorize Brokkr's driver to refine the
harness's own sandbox through the harness's supported runner, for the
scoped git metadata a linked worktree needs?** This decision proposes
yes, and the code ships behind it. If the operator rules otherwise, the
alternatives are a new boundary value that Brokkr builds (0046 slice
(iii)'s `container`), or a first-class scoped-commit mechanism outside
the harness; either is a new number, not an edit here.

## Consequences

- **Linux and bubblewrap only.** A linked-worktree dsh seat on macOS or
  Windows, or on a Linux host without a usable `bwrap`, refuses at
  start. dsh's `runnerCommand` is a bwrap-compatible contract, so no
  other platform's sandbox is claimed.
- **The scoped set shares `objects` and `refs` with the repository**,
  because a commit writes there; the per-worktree directory is the
  worktree's own, and sibling worktree directories, the parent checkout
  and credentials stay outside the write set.
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
