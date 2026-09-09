# 0054 — A linked worktree's git metadata: the seat writes a private common directory and the driver promotes the one ref its worktree owns

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
its packages, not assumed (re-measured 2026-09-09 against
`/home/vyanakiev/.volta/tools/image/packages/@deepseek-ai/dsh/.../@deepseek-ai/dsh-sandbox-local/lib/index.js`,
sha256 `f3533a38fca8e95755c69ab2e09c78e60c094e26105a0feb323e91daecb4ad1a`,
package version 0.1.2-rc.1):

- **dsh's own sandbox is the boundary in question.** The adapter declares
  `hands` unsupported (decision 0043's consequences), so a dsh seat runs
  under decision 0046's `harness` boundary, confined by
  `@deepseek-ai/dsh-sandbox-local`.
- **That boundary cannot express an extra writable root.**
  `SandboxExecutionPolicy` is `{mode, workspaceRoot}`; `writableRoots`
  is the workspace plus platform temp areas; `bwrapProfileArgs` emits
  `--ro-bind / /`, `--dev /dev`, `--unshare-pid`, `--proc /proc`,
  `--die-with-parent` and, for `workspace-write`, `--tmpfs /tmp` plus
  `--bind <workspaceRoot> <workspaceRoot>` — seven options, and nothing
  else. A linked worktree's per-worktree directory and the shared
  `objects`/`refs` lie outside that root.
- **Escalation is not a bounded answer.** The only wider mode is
  `danger-full-access`, the approval request carries no tool arguments,
  and a headless deployment with no answerer resolves `unavailable`.
  Approving it would run the whole command unconfined.
- **The provider's supported extension point is `runnerCommand`.**
  `confine()` returns `[...runnerCommand, ...bwrapProfileArgs(policy),
  "--", ...argv]` and skips its own runner selection entirely when
  `runnerCommand` is set; `runnerFailureSignatures` travels beside it. It
  is settable through the `--patch` overlay the driver already writes.
- **Brokkr's namespace boundary already answers the same problem, and
  more loosely.** Decision 0043 ruling 6 binds a worktree's external git
  directory with its hooks hidden and its config read-only. This decision
  does the same job for the harness-owned sandbox and does it TIGHTER.

### Why this decision was revised

The first form of this decision gave the seat the shared `objects`,
`refs`, `logs` and `packed-refs` read-write, because that is what a
commit writes, and recorded the consequence honestly: a seat could run
`git update-ref refs/heads/<sibling>` and move a branch a sibling
worktree had checked out, and could destroy the shared object store.
A controller reproduction on a disposable repository measured exactly
that against the real mount set: `sibling_ref_moved: true`.

Documenting a broader write set is not the same as bounding it. The
commission requires the sibling protections, so this revision removes the
shared write set entirely rather than describing it. The mechanism that
replaces it is git's own: a private common directory, and a promotion
performed by the trusted driver.

Alternatives weighed:

- **Set the dsh workspace to the parent checkout.** Rejected: it makes
  the whole checkout writable and is exactly the "blindly mount the
  parent" the boundary forbids.
- **Escalate to `danger-full-access` for a commit.** Rejected: the
  approval request carries no command, so the grant cannot be scoped to
  one trusted committer; it would run anything the model asked.
- **Bind only the seat's own ref file read-write.** Rejected because it
  does not work: git's files backend takes its lock in the CONTAINING
  directory (`refs/heads/<name>.lock`) and finishes with a rename, so a
  writable `refs/heads/<own>` alone cannot be updated, and a writable
  `refs/heads` is every sibling's branch. A packed-refs rewrite is the
  whole file. There is no bind-shaped way to say "this one ref".
- **An overlayfs upper layer over `refs` and `objects`.** Workable in
  principle and rejected as the first slice: unprivileged overlayfs in a
  user namespace is a kernel-configuration question this boundary would
  then depend on, and it buys nothing the private common directory does
  not already give, which needs no kernel feature beyond the bind mounts
  bubblewrap already makes.
- **A commit broker outside the box, spoken to over a socket.**
  Rejected: it needs a protocol, a `git` shim and a second writer, and
  git already has the mechanism — a common directory is a path in a
  file, and a promotion is a fetch.
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
     a directory under it read-write.
   - **The `gitdir` back-pointer exists, and the worktree ROOT it names
     is this seat's workspace directory.** The topology check alone is
     not enough, because a workspace `.git` file may name a REAL
     administrative directory belonging to another worktree of another
     repository. The back-pointer lives inside the git directory,
     outside the workspace and outside anything a seat can write, so it
     is the trustworthy end of the pair. A directory with no
     back-pointer is refused, not excused: git writes one for every
     worktree it creates.

     **What is compared is the DIRECTORY, not `<workspace>/.git`.**
     `<workspace>/.git` is a path the seat owns, and every comparison
     resolves symlinks: a seat that replaces its own `.git` with a
     symlink to another worktree's `.git` file — or simply copies that
     file — makes Git resolve the victim's real administrative
     directory, and a comparison against `<workspace>/.git` then
     resolves the seat's own side to the victim's path and finds the two
     ends agreeing. That was measured against a real `git rev-parse` and
     a real `bwrap`. The worktree root the pointer names is the end no
     workspace write can move: the victim's root is not this seat's
     workspace however the seat spells its own `.git`. A symlink AT
     `<workspace>/.git` is refused outright as a second belt, because
     `git worktree` never writes one, so its presence is evidence rather
     than a spelling.

   A back-pointer that names a worktree CONTAINING this workspace is the
   subdirectory case, and it is refused by its own name — the seat was
   started below its worktree's root, nothing is pointing at another
   worktree, and the remedy is the root the pointer names.

   **Enforcement binding:** `scope_refusal` in
   `brokkr-protocol::dsh_sandbox`, called by `dsh_sandbox_row_for`
   before the seat starts and by `runner_argv` on every command; unit
   tests for each arm, including the workspace-local redirect and both
   spellings of the borrowed-back-pointer alias (symlink and copy)
   measured against a real `git rev-parse` in the behavioral proof.

4. **The shared repository is never writable. The seat writes a PRIVATE
   common directory, and git follows it because `commondir` says so.**
   The runner is `brokkr dsh-sandbox-runner`, named in the sandbox row's
   `runnerCommand`, and it adds exactly two read-write mounts and four
   read-only ones:

   | Mount | Why |
   |---|---|
   | `--bind <git_dir>` | the per-worktree directory: `index`, `HEAD`, its own reflog — the worktree's own state, and nothing any sibling reads |
   | `--bind <store>` | the private common directory the driver staged |
   | `--ro-bind <trusted>/config-mask` over `<git_dir>/config` and `<git_dir>/config.worktree` | the per-worktree configs the host would honour under `extensions.worktreeConfig` |
   | `--ro-bind <trusted>/commondir` over `<git_dir>/commondir` | the redirect itself |
   | `--ro-bind <git_dir>/gitdir` | the worktree's pointer back to its own `.git` file |
   | `--ro-bind <git_dir>/HEAD` | the branch this worktree OWNS, which is the ref ruling 5 promotes |
   | `--ro-bind <common>/config` over `<store>/config`, `<common>/HEAD` over `<store>/HEAD`, `<trusted>/alternates` over `<store>/objects/info/alternates` | the three things git READS from a common directory that the seat must not choose |
   | `--tmpfs <git_dir>/hooks`, `--tmpfs <store>/hooks` | both hook paths git could use, empty and ephemeral |

   `HEAD` is read-only for the same reason `gitdir` is, one level up: it
   is what says which branch this worktree owns, and ruling 5 promotes
   exactly that branch. Left writable, a seat could point it at a
   sibling's branch and have the NEXT seat's honest commits promoted onto
   it. A commit does not write `HEAD` — it moves the branch `HEAD` names —
   so the work path is untouched; switching branches or rebasing inside a
   seat is not served, and says so with a lock failure rather than by
   quietly widening the write set.

   The private common directory is a real one: a copy of the shared
   `refs` tree and `packed-refs`, so every branch, tag and
   remote-tracking ref reads back exactly as the host has it; `HEAD`,
   `config`, `info` and `shallow` copied so the host-side promotion can
   read the directory as a repository; an empty `logs`; and an `objects`
   whose `info/alternates` names the host's real object store. The seat
   therefore READS every object the repository has and WRITES none of
   them.

   `<common>/objects`, `<common>/refs`, `<common>/packed-refs`,
   `<common>/logs`, `<common>/hooks`, `<common>/config`,
   `<common>/objects/info/alternates`, every sibling worktree's
   administrative directory and the parent checkout are not in the write
   set at all. They are readable only through the profile's own
   `--ro-bind / /`, so a write to any of them is `EROFS`.

   `commondir` and `gitdir` are read-only and HARD binds rather than
   `-try`: a missing pointer fails the run instead of silently leaving
   the box free to create one, and unlink or rename over a mount point is
   `EBUSY`.

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
   exists for.

   **Enforcement binding:** `brokkr-protocol::dsh_sandbox`
   (`runner_argv`, `scope_refusal`, `scoped_git_binds`, `stage_files`,
   `sandbox_row`); the `--patch` row `dsh_seat_overlay_with` composes;
   `brokkr dsh-sandbox-runner` dispatched before clap in `brokkr-cli`;
   unit tests over the argv — including an assertion that the runner adds
   exactly two `--bind` mounts and none under the shared directory — and
   a behavioral proof that commits in a real linked worktree under dsh's
   real bwrap profile and reads the host's bytes back.

5. **Exactly one ref is promoted: the branch the worktree had checked out
   when the seat started.** Git refuses to check one branch out in two
   worktrees, so that ref is this seat's and no sibling's — and that is
   VERIFIED rather than assumed: the driver reads the main checkout's
   `HEAD` and every sibling worktree's, all outside any seat's write set,
   and refuses a branch two checkouts claim. `<git_dir>/HEAD` itself is
   mounted read-only (ruling 4), so the value the driver reads is one only
   the host ever wrote. A worktree with a detached HEAD owns no ref and is
   refused before the seat starts, naming `git switch` as the remedy.

   After `dsh` exits, the trusted driver — outside every box — reads the
   private store's value for that ref, fetches it into
   `refs/brokkr/dsh-promotion` through git's own local transport (a ref
   namespace no worktree can have checked out, so `git fetch` has no
   checked-out branch to refuse), moves the branch with
   `update-ref <ref> <new> <old>` against the value the host STILL holds,
   and deletes the temporary ref. Everything else the seat wrote — a
   sibling's branch, a tag, a remote-tracking ref, a new branch, an
   object nothing reaches — stays in the private store and is discarded
   with it.

   A promotion that cannot happen is a driver failure that NAMES the
   private store's path and keeps it, never a silent loss of the seat's
   commits. The compare-and-swap means a branch something else moved
   while the seat ran refuses rather than being overwritten.

   **Enforcement binding:** `checked_out_branch`, `head_branch`,
   `other_checkout_on`, `stage_seat_store`,
   `promote_seat_commits` and `SeatGitStore::kept` in
   `brokkr-protocol::dsh_sandbox`; the call in `invoke_dsh_with` after
   the child exits; tests over a real repository for the promotion, for
   an untouched store, for a store whose branch the seat deleted, for a
   `git` that cannot run and for each of the three steps failing; and the
   behavioral proof, which moves a sibling's branch and a tag inside the
   box and reads the host's bytes back unchanged before AND after the
   promotion.

6. **The profile is parsed by a complete option table, and an option the
   runner does not know refuses the command.** Whether the staged
   read-only files are reachable from the box is answered by reading
   every read-write bind in the profile the runner was actually handed —
   `--bind`, `--bind-try`, `--dev-bind`, `--dev-bind-try` and
   `--overlay`'s upper layer — plus the read-write set the runner adds
   itself. Stepping over the rest requires knowing each option's arity,
   because a wrong guess reads an ARGUMENT as a flag; so the table covers
   bubblewrap 0.11 exhaustively and an option outside it is a refusal
   rather than a skip. dsh 0.1.2-rc.1 emits seven of them.

   **Enforcement binding:** `profile_flag`, `writable_binds`,
   `trusted_refusal` and `staged_file_refusal`; a unit test that walks
   every entry of the table and proves the scan lands on the bind that
   follows it, plus the unknown-option and truncated-arguments refusals.

7. **Unsupported hosts refuse at seat start, not at commit time, and the
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

8. **Seat commits are unsigned and attributed to the host.** The driver
   sets `GIT_CONFIG_COUNT=1`, `GIT_CONFIG_KEY_0=commit.gpgsign`,
   `GIT_CONFIG_VALUE_0=false`, and the host's `GIT_AUTHOR_*` /
   `GIT_COMMITTER_*` on the dsh child, the same environment discipline
   the namespace box applies (decision 0043 ruling 6; CONTRIBUTING).

   **Enforcement binding:** the child environment in `invoke_dsh_with`;
   a test whose fake dsh dumps its environment.

9. **A boundary proof that cannot run is a failure on the host whose job
   is to run it.** Every proof that needs a real namespace skips on a
   machine that cannot open one — a developer's laptop, a nested box —
   and a skipped Rust test prints `... ok`, which is why a red behavioral
   test once reached a commit. A host that sets
   `BROKKR_REQUIRE_BOUNDARY_EVIDENCE` fails the proof instead of
   skipping it, and CI's Linux legs set it, so the evidence is either
   produced or the job is red. It is never a reason to weaken a proof.

   **Enforcement binding:** `hands::BOUNDARY_EVIDENCE_ENV`,
   `boundary_evidence_required` and `skip_boundary_proof`; every
   namespace-dependent skip routed through it — including the three
   `engine/boundary_tests.rs` proofs and `doctor/tests.rs`'s that this
   revision routed; the `engine` and `coverage` Linux jobs in
   `.github/workflows/ci.yml` setting it; unit tests over both arms of
   the guard.

10. **The boundary does not move.** The seat still stands under
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
| Point `<workspace>/.git` at a REAL administrative directory of another worktree or another repository | Ruling 3's back-pointer check: the worktree ROOT the pointer names must be this seat's workspace directory |
| Spell the same borrowed metadata as a SYMLINK at `<workspace>/.git`, so that a resolving comparison against `<workspace>/.git` agrees with the victim | The comparison is against the back-pointer's parent DIRECTORY, which no workspace write can move; and a symlink at `<workspace>/.git` is refused outright, because `git worktree` writes a plain file there |
| Spell it as a plain COPY of another worktree's `.git` file, so no symlink is involved at all | The same directory comparison: Git resolves the victim's administrative directory, whose back-pointer names the victim's root |
| Reach a repository through a symlinked alias so the paths compare unequal | Every comparison resolves symlinks before comparing — but never a path the seat owns against a path the seat owns (see the two rows above) |
| Move a sibling's branch with `git update-ref` / `git branch -f` | It succeeds — inside the PRIVATE store. The host's `<common>/refs` is read-only, and the promotion moves one ref: the branch this worktree had checked out |
| Move it by writing `<common>/refs/heads/<sibling>` directly, or by taking `<sibling>.lock` first | `EROFS`: the shared `refs` tree is not in the write set |
| Move it through PACKED refs, by rewriting `<common>/packed-refs` or running `git pack-refs --all` | `EROFS` for the direct write; `pack-refs` rewrites the private store's copy |
| Create a new branch, tag or remote-tracking ref in the shared store | Written in the private store; the promotion moves one ref and discards the rest |
| Destroy or corrupt the shared object store, or repoint `<common>/objects/info/alternates` at another store | `EROFS`: `<common>/objects` is read-only. The seat's own objects go to `<store>/objects` and reach the shared store only through the promotion's fetch, which is git's own connectivity-checked transfer |
| Repoint the PRIVATE store's `objects/info/alternates`, so the host-side promotion reads objects from elsewhere | It is mounted read-only from the trusted staging directory |
| Rewrite `commondir` or `gitdir` from inside the box, so the HOST's next `git` reads a config the box wrote | Both bound read-only and hard: write is `EROFS`, unlink and rename over the mount point are `EBUSY` |
| Point `<git_dir>/HEAD` at a sibling's branch, so the NEXT seat's honest commits are promoted onto it | `HEAD` is bound read-only too, by write, by `git symbolic-ref`, by `git checkout` and by unlink; and the driver refuses a branch another checkout already claims |
| Create `config.worktree` so `extensions.worktreeConfig` makes the host honour a `core.hooksPath` the box chose | Masked with an empty read-only regular file, whether or not the host file exists |
| Fill the mask, the `commondir` pointer or the `alternates` pointer through their SOURCE paths | The runner refuses when the staging directory lies under the source of ANY read-write bind in the profile it was handed, or under its own write set, so the answer is measured per command rather than resting on dsh's `/tmp` being a fresh tmpfs — which it is on 0.1.2-rc.1, proved by the box writing to the source path while the host's file reads back empty |
| Plant a hook the host later runs | The host's `<common>/hooks` is read-only, and the hook path the seat's own git uses is an empty tmpfs |
| Run the host's existing hooks | The seat's hook path is `<store>/hooks`, a fresh tmpfs; the behavioral proof plants a failing `pre-commit` on the host and the seat's commit still succeeds |
| Write a sibling worktree's checkout, index, `HEAD` or `config.worktree` | Nothing under `<common>/worktrees/<other>` is bound, and the sibling's checkout directory is not in the write set |
| Write the parent checkout, or any credential path | Neither is bound; the profile's root is read-only |
| Come back as a LATER session and find a repository a promotion has already written to | The store is staged fresh for each seat and the checks run again; the behavioral proof runs two boxed sessions in the same worktree and reads the host's bytes back after each |

## Consequences

- **Linux and bubblewrap only.** A linked-worktree dsh seat on macOS or
  Windows, or on a Linux host without a usable `bwrap`, refuses at
  start. dsh's `runnerCommand` is a bwrap-compatible contract, so no
  other platform's sandbox is claimed.
- **A seat's commits reach the shared repository when the seat ENDS, not
  when it commits.** Between the two, the worktree's index and `HEAD`
  are the seat's while its branch is still the host's, so a host `git
  status` run in that worktree during a seat shows the seat's tree as
  staged. It resolves at the promotion. Nothing in the phase machine
  reads the branch mid-seat; the next seat's staging reads the promoted
  state.
- **A seat cannot switch branches or rebase.** `HEAD` is read-only, so
  `git checkout <other>`, `git switch` and `git rebase` fail on the lock
  rather than moving the ref this decision promotes. Committing, amending
  and resetting the OWN branch all work, because those move the branch
  `HEAD` names and not `HEAD`. A seat that wants a different branch
  delivered asks for a worktree on it. This is the price of "one ref,
  provably this worktree's", and it is paid deliberately: the alternative
  measured here is a previous seat aiming a later seat's commits at a
  branch it chose.
- **Only a linked worktree at its own root, on a branch, is served.** A
  primary checkout reached through a subdirectory, a `--separate-git-dir`
  checkout, a `.git` file redirected at the parent or at another
  worktree, an administrative directory outside `<common>/worktrees`, a
  seat rooted in a SUBDIRECTORY of its linked worktree, and a detached
  HEAD all refuse at start, each naming its own cause and remedy.
- **Staging copies the shared ref tree.** For a repository with hundreds
  of thousands of loose refs that is a real cost; for one whose refs are
  packed it is a single file. The copy is what makes every ref read back
  exactly as the host has it, which a synthesized `packed-refs` would
  only approximate.
- **A workspace whose `.git` file has been damaged refuses the NEXT
  seat, and says so at start.** That is the fail-closed direction — the
  alternative is binding whatever the damaged file points at — but it
  does mean a seat can leave a worktree unusable for its successors. The
  remedy is `git worktree repair`, and the refusal arrives before an
  implementation is spent rather than after it.

  **What that refusal protects is the next SEAT, not a host `git` run in
  the same worktree.** Every work boundary gives a seat its own worktree
  to write, `.git` file included, so a host command run there afterwards
  follows whatever that file names — a residual of handing a model a
  worktree at all, not of this runner's binds, and the same under the
  namespace boundary.
- **The masks create empty files on the host when they were absent.**
  Bubblewrap creates the mount point for a missing destination; the box
  sees an empty read-only file, and the host file stays empty.
  `<git_dir>/config` gains the same empty file; git reads a linked
  worktree's `config` from the common directory, so an empty one beside
  it is inert.
- **`git worktree list` inside the seat reads the private store**, which
  has no `worktrees` directory, so it reports the store as the only
  checkout. Nothing a seat does depends on it, and the alternative —
  copying sibling worktree metadata into the store — would put back
  exactly what this decision removed.
- **The runner is an operator assertion to dsh.** dsh skips its own
  runner probe when `runnerCommand` is set, so the driver probes the
  same `bwrap` itself and refuses on a failure; the runner also prints
  the `brokkr-dsh-sandbox-runner:` signature dsh classifies as a runner
  failure, never as a denied command.
- **What this does not do.** It does not make the seat's git history
  private, does not sign commits, and does not bound egress (decision
  0036). It does not give dsh `hands`, and it does not touch the
  namespace boundary's own git handling. That handling is much wider:
  0043 ruling 6 binds a linked worktree's whole shared `.git` read-write
  with `hooks` masked and `config` read-only, which leaves the shared
  ref store, the shared object store, and the per-worktree `commondir`,
  `gitdir` and `config.worktree` writable — every residual this decision
  closes for the harness runner. That is not a regression here (the write
  set predates this work) and the commission scopes this run to the
  harness-owned sandbox, so it is recorded as a known-open path in the
  sibling module rather than silently changed: `box_argv`'s ruling-6
  block carries a `KNOWN OPEN` note naming the paths, so a reader of the
  write set meets it there. Tightening the namespace box the same way is
  a change under 0043, with its own issue and its own number. Filing that
  issue is the operator's.

## The question this decision asks

Ruling 10 is the semantic choice, and it is the operator's: **does a
realm's `harness` boundary authorize Brokkr's driver to refine the
harness's own sandbox through the harness's supported runner — and to
perform, outside the box, the one trusted ref update that turns the
seat's private commits into the worktree's branch?** This decision
proposes yes, and the code ships behind it. If the operator rules
otherwise, the alternatives are a new boundary value that Brokkr builds
(0046 slice (iii)'s `container`), or leaving dsh seats unable to commit
in a linked worktree; either is a new number, not an edit here.
