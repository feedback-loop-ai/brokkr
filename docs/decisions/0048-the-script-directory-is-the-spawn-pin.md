# 0048 — The script directory is the spawn pin

Status: proposed
Date: 2026-09-06

## Context

The returned review of decision 0046 slice (i), commit 78e33d9, names
two defects in the unboxed exec check. Replacing every backslash in a
manifest path merges the Unix file `scripts\gate.sh` with
`scripts/gate.sh`; the retained digest can describe the wrong file.
Re-walking the entire declaring layer also rejects a realm created by
`brokkr init .`: its journal, result files and implementation sources
live beside its bundle and change during the run.

The task commissions both repairs. This proposal records the narrower
reading of the archived design's DD9; it does not claim operator
acceptance. Checking only the executable would lose the check on helpers
it sources. Excluding particular journal filenames would miss configured
journal paths and would still freeze the implementation's sources.

## Rulings

1. **A manifest key preserves the actual filename components.** Only
   separators between filesystem components become `/`. A literal Unix
   backslash remains a backslash in its own key. A filename that cannot
   be represented as UTF-8 is refused instead of being decoded lossily
   into another file's key. Ordinary portable bundle keys keep their
   existing bytes and digests.

   **Enforcement binding:** `walk_files` in
   `crates/brokkr-runtime/src/bundle.rs`; the two filename regressions
   in `bundle/model_policy_tests.rs`, including both names in one layer.

2. **Spawn checks the script directory against its compiled file map.**
   Before every unboxed exec spawn, re-walk the directory containing the
   script and its descendants. Refuse changed, missing or added files,
   naming the declaring layer and the actual file key. Keep ancestor
   file maps from the exact manifests hashed into their compose digests;
   never take a new baseline at spawn. The script token is the first
   expanded bundle path in the dispatch, before its unjudged arguments.
   Compare paths by components under compile's canonical layer roots.

   This narrows DD9's whole-layer check: `scripts/` remains protected in
   an `init .` realm while sibling source trees, `.forge/results/` and
   the journal may change. A script at the layer root still selects that
   entire directory. Bundle identity and resume's manifest comparison
   keep their existing scope. No new manifest field or contract version
   is introduced.

   **Enforcement binding:** `script_directory` and `spawn_site` in
   `crates/brokkr-runtime/src/engine.rs`, `layer_drift` in `bundle.rs`,
   and the retained ancestor maps in `bundle/compose.rs`. Runtime tests
   cover leaf and inherited helpers and panel spawns; the CLI's
   `boundary_verbs.rs` runs `init .`, then an implementation and two
   unboxed gates with source edits, an existing journal and result
   files inside the layer, under both `harness` and `open`.

## Consequences

The check covers the script directory, not every host file a script may
read. Helpers outside that directory and the interval between the walk
and exec are outside its guarantee; the run remains visibly unboxed.
The original manifest still records the complete layer. Canonical paths
are not compared with independently canonicalised script targets, so a
symlink continues to pin the bytes read through its real entry name.

The review's third finding is a deletion: unboxed agent resolution
already omits the workspace fragment, so engine composition appends the
harness fragment directly and never removes matching tokens again.

D34's delivery-summary absence case and missing explicit open-work
delta scenario remain accepted residuals for Muninn. Claude's harness
fragments and codex's capture measurement remain operator-owned and
deferred as recorded in the slice's completion note.
