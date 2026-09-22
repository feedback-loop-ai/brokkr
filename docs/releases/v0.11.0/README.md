# v0.11.0 release handoff

The previous published release is `v0.10.0`, commit
`881df2e6`. The preparation starts from `314e8786`, forty merged changes
later. The release pull request records the final candidate commit and its
check results. [Release description](../v0.11.0.md).

## Documentation audit

- Versioning guide: current engine version, the `brokkr doctor` contracts
  line and the pre-1.0 promise now read 0.11.0; it links these notes and
  keeps the v0.10.0 notes for the boundary migration.
- Release recipe: its example command names v0.11.0.
- README, quickstart and packaging guide: reviewed. Decision 0063's
  enactment (#309) already removed the Windows archive, the Scoop row and the
  Windows badge and says "on Windows, use WSL2"; the "live from v0.9.x"
  channel rows are channel history and stay. The Nix version changes only
  with real release digests in the post-publication channel pull request.
- ARCHITECTURE and CONTRIBUTING: reviewed; no current-version reference.
- Frozen surfaces: against v0.10.0 the release adds `dialect.v3`,
  `realms.v5`, `run-manifest.v10` and `seat-record.v5` beside their earlier
  versions and updates `contracts/README.md`. No earlier schema, evaluator
  fixture, `policy/phase-machine.json` or reference file changed.
- Workflow toolchain agreement: CI, release admission and
  `scripts/coverage-exact.sh` still read `rust-nightly-version.txt`.

## First release without Windows

`.github/workflows/release.yml` lost its Windows leg, its PowerShell
packaging step and its Scoop step in #309 and has not run since. This tag is
its first real run. A release is four archives plus the deb and rpm packages.
The Scoop bucket's retirement note is feedback-loop-ai/scoop-bucket#5,
pending the operator's merge.

## Organization profile

Repository: `feedback-loop-ai/.github`.
Base: `1bd9f7f69b4d79370e4644cbb4539516cbacda4e`.
Path: `profile/README.md`.
Patch: [organization-profile.patch](organization-profile.patch).

The profile's one current-version label is v0.10.0; the patch sets v0.11.0
and links that release. Its Scoop channel row still read as a live channel;
the patch marks it retired at v0.10.0 and points Windows users to WSL2.
Application is pending until v0.11.0 has been published.

## Publication order

1. Require green CI on the release PR's final commit and squash-merge it.
2. Tag the merged commit `v0.11.0`; the release workflow must pass admission
   and exact coverage before it builds or publishes anything.
3. Publish the release description; verify the four platform archives, the
   deb and rpm packages, `SHA256SUMS` and the provenance attestations; then
   the seven crates on crates.io and the signed apt and rpm indexes.
4. Review and merge the generated Homebrew and Nix changes; verify their
   versions and digests.
5. Apply the organization-profile patch against its recorded base and verify
   the public profile.

Not done, by the operator's instruction: the new binary is **not** installed
on the operator's host.
