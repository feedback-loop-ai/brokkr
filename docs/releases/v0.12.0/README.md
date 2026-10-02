# v0.12.0 release handoff

The previous published release is `v0.11.0`, commit `e5921db6`. The
preparation starts from `d0aaeca8`, seventy-nine merged changes later. The
release pull request records the final candidate commit and its check
results. [Release description](../v0.12.0.md).

## Documentation audit

- **Versioning guide:** the current engine version, the `brokkr doctor`
  contracts line and the pre-1.0 promise now read 0.12.0, and the guide links
  these notes. "Releases after v0.11.0 read only the `BROKKR_*` names" is
  history and stays.
- **Release recipe:** its example command names v0.12.0.
- **`docs/releases/unreleased.md`:** carried the #355 retirements, which these
  notes now hold. It is reset for the next cycle.
- **Status page and security model:** they predate decision 0065's typed
  tools. #366's corrections must be re-derived against 0065 before they
  merge, so these notes say so under Known issues rather than claim them.
- **Frozen surfaces:** against v0.11.0, the release adds `realms.v6`,
  `run-manifest.v11`, `tool-dialect.v1` and `effect-cleanup.v1` beside their
  earlier versions, and updates `contracts/README.md`. No earlier schema,
  evaluator fixture, `policy/phase-machine.json` or reference file changed.
- **Witness digests:** re-pinned by `BROKKR_BLESS=1 cargo test -p
  brokkr-runtime --test witness_digests` (#358's table). Only the engine
  version moved them.
- **Nix flake:** its version changes only with real release digests, in the
  post-publication channel pull request.

## Not installed on the operator's host

`~/.cargo/bin/brokkr` stays 0.11.0. Two libraries on this host still write
capability flags inline, which this release refuses at compile:
- LaneTally's crew library and its commission bundles: `--sandbox read-only
  -c mcp_servers…` on every codex seat, and `--allowedTools` on every claude
  seat;
- the forge pipeline's story crew, which is pinned to a pre-0065 binary.

Both move to typed `tools` before either upgrades. LaneTally also has runs in
flight that must finish on 0.11.

## Organization profile

Repository: `feedback-loop-ai/.github`.
Base: `3770ee4a51e225b60928c3b5666c75b971e135a2`.
Path: `profile/README.md`.
Patch: [organization-profile.patch](organization-profile.patch).

The profile's one current-version label is v0.11.0. The patch sets v0.12.0
and links that release. Apply it only once v0.12.0 is published.

## Publication order

1. Require green CI on the release PR's final commit, and land it through
   the merge queue.
2. Tag the merged commit `v0.12.0`. The release workflow must pass admission
   and exact coverage before it builds or publishes anything.
3. Publish the release description. Verify:
   - the four platform archives, the deb and rpm packages, `SHA256SUMS` and
     the provenance attestations;
   - then the seven crates on crates.io, and the signed apt and rpm indexes.
4. Review and merge the generated Homebrew and Nix changes, and verify their
   versions and digests.
5. Apply the organization-profile patch against its recorded base, and verify
   the public profile.
