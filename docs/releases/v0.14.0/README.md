# v0.14.0 release handoff

The previous published release is `v0.13.1`. The release is cut from
main at `e9605f35d`, the merge of #1100, which completed #440. The
release pull request records the final candidate commit and its check
results. [Release description](../v0.14.0.md).

## Name and cut-off

- **The name:** “Megingjörð”, Thor's belt of strength. The operator ruled
  it on 2026-11-16, in chat.
- **The cut-off:** the operator ordered on 2026-11-13 that #440 be
  completed and then a new release tagged. #440's last unit, settle and
  the reseal (decision 0108 ruling 3), merged as #1100.
- **No story.** No lore saga ships with this release, so the notes carry
  no epigraph.

## After the tag

- **The org profile.** Apply `organization-profile.patch` to
  `feedback-loop-ai/.github`. It moves the profile's Brokkr row from
  v0.13.1 to v0.14.0.
- **The flake.** The flake's digests catch up with v0.14.0 in their own
  pull request, once the release artifacts exist, as #1010 did for
  v0.13.1.
