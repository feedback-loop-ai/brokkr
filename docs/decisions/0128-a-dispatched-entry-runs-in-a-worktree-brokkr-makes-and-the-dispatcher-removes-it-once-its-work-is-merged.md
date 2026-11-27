# 0128 — A dispatched entry runs in a worktree Brokkr makes for it, and the dispatcher removes the tree once its work is merged

Status: proposed
Date: 2026-11-27

Built: unbuilt (#1445)
Amends: 0054, 0068, 0104, 0110, 0125

## Context

The queue exists to run as much work side by side as the machine and the
providers allow. The operator, 2026-11-27: "the whole idea of the queue
is to [parallelize] as much work as possible", and "I would like queued
items being dispatched to be handled by brokkr in worktrees. And the
dispatcher to handle the worktree cleanup after the work is merged."

Today the checkout is what limits that, and Brokkr does not make one:

- **A run works where it was queued.** The dispatcher starts each driver
  in the entry's `workspace`, and the engine works in the entry's `repo`,
  or the workspace when none was named. Both are recorded at `queue add`
  (`crates/brokkr-cli/src/verbs/dispatch.rs:307-321`,
  `crates/brokkr-runtime/src/launch/queued.rs:255-264` and `:320-330`,
  `crates/brokkr-runtime/src/engine.rs:1041-1043`). Decision 0110 names
  the driver's working directory as the entry's workspace. No production
  code makes a worktree or a branch.
- **Nothing in admission keeps two entries off one checkout.** Decision
  0068 ruling 3 admits on waits, provider and route capacity, and host
  room. The only guard is the broker's per-attempt writer lease, keyed by
  the workdir's path (`crates/brokkr-runtime/src/engine/dispatch.rs:201-209`,
  `engine/broker/exclusion.rs:237-255`). A conflict fails the effect
  rather than holding the entry, and two runs still interleave between
  attempts. Parallel entries therefore need parallel checkouts that
  someone else made. Lane scripts and downstream delivery drivers make
  them by hand, and #1035's third step asks for exactly this:
  "Provision one worktree, branch and journal per delivery, and clean
  them up when it ends".
- **Nothing removes a worktree.** Decision 0104's sweeps remove seat
  scratch trees and dead hands session trees, and leave worktrees to the
  controller.
- **A merge has one witness, the operator.** `brokkr queue merged` is an
  attributed operator command whose help says Brokkr reads no code host,
  and its one reader is the `ENTRY:merged` wait
  (`crates/brokkr-cli/src/cli_args/queue.rs:41-46`,
  `crates/brokkr-runtime/src/admission/awaited.rs:86-96`).
- **Two facts would break a tree at a new path:**
  - the realm lookup is exact path equality
    (`crates/brokkr-runtime/src/realms.rs:569-585`), so a tree Brokkr made
    would resolve to no realm, and its house, grants and dialects would
    be lost;
  - a run's local results, scratch and secrets live under
    `<workdir>/.forge` unless a folder state exists
    (`crates/brokkr-runtime/src/places.rs:687-691`), so removing the tree
    would delete the run's evidence.
- **Linked worktrees share one common directory:** objects, refs,
  config, hooks and each tree's administrative record. The box binds a
  linked worktree's common directory in place, read-write, with `hooks`
  hidden and `config` read-only
  (`crates/brokkr-protocol/src/hands/namespace/workspace.rs:154-212`).
  A harness seat can therefore move or delete a sibling's branch, or run
  `git worktree prune`, which deletes the records of every sibling its
  box does not mount (#851: one prune dropped about 65). Decision 0054
  already gives a dsh seat a private common directory and promotes back
  only the ref its worktree owns.

**The alternatives weighed,** each ruled by the operator on #1445 on
2026-11-27 (https://github.com/feedback-loop-ai/brokkr/issues/1445#issuecomment-7227340625
and https://github.com/feedback-loop-ai/brokkr/issues/1445#issuecomment-7227474161):

- one tree per entry, or one per work order, so a story, its fix rounds
  and its landing share one: per work order, else per entry ("approved");
- trees under a declared root, or wherever the operator queued: a
  declared root ("approved");
- the merge witnessed only by the operator, or by the dispatcher reading
  the code host: "queue merged I guess, and the dispatcher should make
  sure queue merged is journaled properly after PR merge", generalized
  because "we are building an engine for other repos";
- a tree whose work never merges kept until the operator removes it, or
  removed after an age: "removed after configurable grace period,
  default - 2 weeks";
- linked worktrees as they are, linked worktrees whose boxed seats get a
  private common directory, or a local clone per tree: "b", the second;
- the realm journaled with the tree, and run-local state kept outside
  it: "yes".

## Rulings

1. **A tree belongs to a work order, else to an entry.** An entry that
   serves a work order (decision 0116) runs in that work order's tree,
   and every later entry of the same work order runs there too: its fix
   rounds and its landing share one tree and one branch. An entry that
   serves none gets a tree of its own.

   *Binding:* a tree record in the journal, keyed by the work order's id
   or the entry's id, holding the tree's path, realm, branch, base and
   state. A second record for one key is refused.

2. **Trees live under a declared root, never in a temporary place.** The
   machine's host file (decision 0068's 2026-10-10 addendum) declares a
   root per realm, defaulting to
   `$XDG_STATE_HOME/brokkr/worktrees/<realm>/<key>`, where `<key>` is
   `WO-<n>` or the entry's id. A root on tmpfs, under `/tmp`, or inside
   any worktree is refused by name, at `doctor` and when a tree is made.

   *Binding:* the host file's schema; typed refusals for each refused
   root.

3. **A tree's branch and base are fixed at `queue add`.**
   - The branch is `brokkr/WO-<n>`, or `brokkr/entry-<id>`.
   - The base is a commit resolved and journaled at `queue add`: `--base
     <rev>`, defaulting to the head of the realm's default branch at that
     moment.
   - A work order's later entries keep its tree's branch.
   - The dispatcher never rebases, merges into or force-moves a tree's
     branch. Restacking stays a person's or a run's.

   *Binding:* the entry's journaled base and the tree record. A branch
   that already exists and is not the record's is refused by name.

4. **A tree carries its realm, and a run's own state lives outside it.**
   - A tree Brokkr made resolves to the realm it was made from. The
     realm is read from the tree record before any path match.
   - A dispatched run's local results, scratch and secrets live in the
     journal's folder state, keyed by run, never under the tree. Removing
     a tree therefore removes no evidence, and the sweep never closes a
     row because its tree is gone.

   *Binding:* the realm lookup reads tree records first
   (`realms.rs`); `places.rs` places a dispatched run's state in the
   folder state. A test removes a tree and finds every evidence file
   and row intact.

5. **A run made from the queue works in its tree, made when it is
   claimed.**
   - At the claim (decision 0110's pass, step 5), before the driver
     starts, the dispatcher makes the tree with `git worktree add -b
     <branch> <path> <base>` in the realm's repository, unless the work
     order's tree already exists.
   - The driver's working directory and the engine's repository are the
     tree.
   - A tree that cannot be made holds the entry with its typed cause,
     journaled. It is not a park, and no run is started.

   *Binding:* the claim path in `crates/brokkr-runtime/src/dispatch/`;
   a journaled tree fact for each made or refused tree.

6. **Admission holds an entry whose tree another live run holds, and
   counts a fresh tree's build.**
   - One condition joins decision 0068 ruling 3: no other live run holds
     the entry's tree. Entries of one work order still serialize through
     their edges (decision 0116 ruling 3). Entries of different work
     orders never wait on each other's checkout.
   - An entry whose tree does not exist yet adds a fresh tree's first
     build to its measured demand (decision 0125). That build is the
     recipe's own measured build footprint, which for this repository is
     9 to 28 GB of disk on its lane hosts.

   *Binding:* a typed admission refusal, `tree held by <run>`; the
   demand term in 0125's reading.

7. **A boxed seat in a tree Brokkr made writes a private common
   directory.**
   - Every boxed seat in such a tree gets a private common directory, as
     decision 0054's dsh runner already does. Its objects are shared
     read-only, and its refs, config and administrative records are its
     own.
   - At the attempt's end the engine promotes only the tree's own branch
     and the run's anchors back. A seat cannot move a sibling's branch,
     prune a sibling's record, or write the shared objects.
   - Until this ruling is built, the dispatcher holds a second tree of
     one repository from starting a boxed harness seat.

   *Binding:* the hands workspace binding (`workspace.rs`) for a tree
   record's path; a test where a seat's `git worktree prune` and
   `git branch -D <sibling>` leave every sibling intact.

8. **A merge is journaled by its witness, and the dispatcher makes sure
   one is.** One `merged` record serves every witness, and it names the
   witness that wrote it:
   1. **A code-host adapter, where the realm declares one.** It reads the
      realm's change requests under Brokkr's own credential (decision
      0118). GitHub is the first adapter, and other code hosts are
      adapters of the same shape.
      - The realm declares how a change request names its run: by its
        head branch (the tree's branch), or by a trailer in its body,
        `Brokkr-Run:` by default (decision 0033's form).
      - The record is written only after a fetch shows the merge commit
        on the realm's default branch. It holds the change request and
        the merge commit.
   2. **A Git-only reading, for any repository with no adapter.** The
      fetched default branch contains the tree's head (a merge or a
      fast-forward), or every file patch of the run's last anchor (a
      squash).
   3. **The operator's `brokkr queue merged`,** always.

   On each pass the dispatcher reads its witnesses for every tree whose
   work order or entry has a shipped run and no merged record.

   *Binding:* the `merged` record gains `witness` and its evidence (the
   next free queue storage version); the realm's code-host declaration in
   the realms map; `queue merged`'s help no longer says Brokkr reads no
   code host.

9. **A merged tree is removed when nothing still needs it.** On a pass
   after its merged record, the dispatcher removes a tree when all of
   these hold:
   - no run of its work order or entry is live, parked or awaiting the
     operator;
   - the tree has no uncommitted change;
   - every commit on its branch is reachable from the merge commit or
     from a run's anchor.

   It runs `git worktree remove <path>`, never with `--force`, and never
   `git worktree prune`. Then it deletes the local branch. A remote branch
   is never touched. Each removal, and each tree kept with its reason, is
   a journaled dispatcher fact, and `queue dispatch status` lists the
   kept trees.

   *Binding:* the dispatcher pass; one test per condition that keeps the
   tree.

10. **A tree whose work ends without a merge is removed after a grace
    period.**
    - The host file declares the period, two weeks by default. It is
      counted from the last ending of a run of the tree's work order or
      entry, with none of them live, parked or awaiting the operator.
    - When it ends, the tree and its build output go. The local branch
      stays unless every commit on it is merged or reachable from a run's
      anchor. A deadline never deletes a commit.
    - The operator may remove a tree earlier with a journaled command
      under `brokkr queue`, which keeps the same branch rule.

    *Binding:* the host file's `tree_grace`; the operator command's
    journaled reason.

11. **The dispatcher removes only the trees it made.** Decision 0104
    leaves worktrees to the controller. Here the trees Brokkr made are
    the dispatcher's, and every other worktree stays the controller's. A
    path with no tree record is never removed, whatever it holds.

    *Binding:* removal reads the tree record alone. A test places a
    foreign worktree under the root and finds it kept.

## Consequences

- **Parallel work no longer needs hand-made checkouts.** What limits the
  queue's parallel starts is admission alone: providers, the host's room
  and the queue's order.
- **Disk becomes the binding resource.** Each fresh tree builds its own
  target, so ruling 6's demand term and ruling 10's grace period decide
  how many trees a host can carry.
- **The merge witness reaches the code host.** That needs decision 0118's
  credential, which is unbuilt (#1093), so the code-host adapter comes
  after the Git-only reading and the operator's command.
- **An unboxed seat has no box to give a private common directory.** In a
  tree Brokkr made, it reaches the shared refs as it does today.
- **The slices, in order:**
  1. the tree record, the realm read from it, and run-local state
     outside the tree (rulings 1 to 4);
  2. the private common directory for boxed seats (ruling 7, #851);
  3. making the tree at the claim, and the admission condition and demand
     term (rulings 5 and 6);
  4. removal, the grace period, the operator command and the status
     listing, with the Git-only witness and the operator's command
     (rulings 8.2, 8.3, 9, 10 and 11);
  5. the code-host adapter, GitHub first (ruling 8.1), after #1093's
     credential.
