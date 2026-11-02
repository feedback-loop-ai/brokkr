# v0.13.0 release handoff

The previous published release is `v0.12.0`, commit `daba0dce`. The
preparation starts from `4f3dd878`, 330 merged changes later. The
release pull request records the final candidate commit and its check
results. [Release description](../v0.13.0.md).

## Name, story and cut-off

The operator ruled all three on 2026-11-02, and #957 records them:
- **The name:** “Járngreipr”, Thor's iron gloves.
- **The story:** [The Lay of the Gloves](../../lore/sagas/the-lay-of-the-gloves.md),
  which ships here and gives the notes their epigraph.
- **The cut-off:** the release is cut from main once these have merged:
  #429's sequence endings, #753 slice 3, U9b2, the records of decisions
  0098 and 0100, the guides that match U1d7, and the two endings below.

## Two endings the code announced for 0.13.0

- **#415's lockless transition ends.** A hands tree with no lock file is
  kept and reported, never reaped. The operator ruled on 2026-09-28 that
  such a tree could be reaped "from 0.12.0 until 0.13.0 ends it".
- **`hands::box_argv` is removed.** It was deprecated in 0.12.0 as "removed
  in 0.13.0" and retired by U8k (decision 0071 ruling 6).

Both landed in [#969](https://github.com/feedback-loop-ai/brokkr/pull/969).

## Documentation audit

- **Versioning guide:** these now read 0.13.0, and the guide links these
  notes:
  - the current engine version;
  - the `brokkr doctor` contracts line;
  - the pre-1.0 promise.
- **Quickstart:** the `brokkr --version` and `brokkr doctor` samples read
  0.13.0.
- **Front page:** its release line names v0.13.0.
- **Release recipe:** its example command names v0.13.0.
- **`docs/releases/unreleased.md`:** said "Nothing yet." for the whole
  cycle. These notes are written instead from:
  - the merged pull requests;
  - the decision record;
  - the facts the U9b2 and U1d7 owners confirmed.

  The file is reset for the next cycle.
- **Lifecycle table:** the seat-record v1 to v4 rows and the dialect v1
  row read `Reader until` 0.12. Decision 0082 ruling 3's windows are
  unbuilt (#361). Under that ruling, a row's deadline is the later of two
  lines: its `Last written` line plus three, and the line the ruling's
  build lands in. So 0.13.0 removes no reader, and the build restates
  those cells.
- **Frozen surfaces:** against v0.12.0, the release adds these beside
  their earlier versions, and updates `contracts/README.md`:
  - `host.v1`;
  - `panel-carry.v1`;
  - `realms.v7` and `realms.v8`;
  - `run-manifest.v12` to `run-manifest.v14`;
  - `seat-record.v6`.

  No earlier schema, evaluator fixture, `policy/phase-machine.json` or
  reference file changed.
- **Witness digests:** re-blessed by `BROKKR_BLESS=1 cargo test -p
  brokkr-runtime --test it witness_digests::` (#358's table). The engine
  version moved them all, and the release recipe's example, now naming
  v0.13.0, moved `recipes/release`'s a second time.
- **Nix flake:** its version changes only with real release digests, in
  the post-publication channel pull request.

## The operator's host

The house engine, `brokkr-main-ac54b857`, and `~/.cargo/bin/brokkr` stay
where they are until v0.13.0 is published. The engine then moves for new
runs only: replacing the binary under a live run ends it. After the move:
- dsh's implementer seat is boxed through hands, with the cargo, git and
  gpg binds it needs;
- Context7 is granted keyless through the realm;
- dsh's native web tools are measured.

## Organization profile

Repository: `feedback-loop-ai/.github`.
Base: `c1921df939ecbf9178805b450f07fa6468f31156`.
Path: `profile/README.md`.
Patch: [organization-profile.patch](organization-profile.patch).

The profile's one current-version label is v0.12.0. The patch sets v0.13.0
and links that release. Apply it only once v0.13.0 is published.

## Publication order

The order is v0.12.0's ([its handoff](../v0.12.0/README.md#publication-order)),
with `v0.13.0` as the tag. Land the release PR through the merge queue on
green CI. Tag its merged commit; the release workflow publishes nothing
until admission and exact coverage pass. Verify:
- the published artifacts, their provenance and the seven crates;
- the Homebrew and Nix channel changes;
- and, last, the organization profile, patched against the base above.
