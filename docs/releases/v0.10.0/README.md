# v0.10.0 release handoff

The previous published release is `v0.9.1`, commit
`ff9e5d20e99d9fbc370d3fb2793ab613f5c35d1d`. The preparation starts from
`f2737075418fc1e086f12d742f6faba2245d9d72`, sixteen merged changes later.
The release pull request records the final candidate commit and its check
results. [Release description](../v0.10.0.md).

## Documentation audit

- README: release highlights and links to release notes and the manager.
- ARCHITECTURE: named boundaries, explicit finding closure, and the split
  between the portable office, pinned configuration and deterministic gates.
- CONTRIBUTING and agent-library guide: the new recipe and office.
- Quickstart: the release recipe's current harness refusal and the full
  shipped-bundle counts.
- Versioning: current engine version, manifest lineage through v9, and boundary
  migration. Contributor coverage instructions now use the compiler pin.
- Packaging guide: reviewed; the "live from v0.9.x" dates describe channel
  history and remain accurate. The Nix version changes only with real release
  digests in the post-publication channel PR.
- Frozen surfaces: the comparison to v0.9.1 adds numbered contracts and updates
  their README; it changes no earlier schema, evaluator fixture, heritage table
  or reference file.

## Organization profile

Repository: `feedback-loop-ai/.github`.
Base: `f4c220890307c8d7e8dcc0c4a3f15bc683937e24`.
Path: `profile/README.md`.
Patch: [organization-profile.patch](organization-profile.patch).

The entire profile was searched for Brokkr version labels, release links and
download references. Its one stale current-version label is v0.8.0. The patch
sets v0.10.0 and links directly to that release; generic project, guide and
installation links remain valid. Other projects are unchanged. Application is
pending until v0.10.0 has been published; verify the remote profile after merge.

## Publication order

1. Require green CI on the release PR's final commit and squash-merge it.
2. Tag the merged commit `v0.10.0`; the release workflow must pass admission and
   exact coverage before building or publishing artifacts.
3. Publish the committed release description, verify all platform assets and
   checksum/provenance evidence, then verify all seven registry crates and the
   signed apt/rpm indexes.
4. Review and merge the generated Homebrew, Scoop and Nix changes using the
   release's checksums. Verify their remote versions and digests.
5. Apply the organization-profile patch against its recorded base, accounting
   for intervening edits, and verify the public profile.

This handoff describes pending publication work at preparation time. Remote
release, workflow and PR records are the evidence that those steps completed.
