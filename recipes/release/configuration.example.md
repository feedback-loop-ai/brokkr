## Release configuration

Replace these values and include this section in the realm's house file.
The fields describe the work; they do not execute commands or grant tools.

```json
{
  "repository": "owner/project",
  "default_branch": "main",
  "tag_format": "v{version}",
  "version_files": ["path/to/version-source", "path/to/lockfile"],
  "version_update": "Describe the repository's version and derived-identity update procedure.",
  "release_notes": "docs/releases/v{version}.md",
  "handoff_directory": "docs/releases/v{version}",
  "scratch_directory": ".release-work",
  "documentation": ["README.md", "ARCHITECTURE.md", "docs/"],
  "validation": [
    {"name": "local checks", "command": "the repository's validation command"},
    {"name": "remote checks", "instructions": "Inspect the required checks on the candidate commit."}
  ],
  "organization_profiles": [
    {
      "repository": "owner/.github",
      "path": "profile/README.md",
      "project_url": "https://github.com/owner/project",
      "apply": "after verified publication"
    }
  ],
  "channels": [
    {"name": "release assets", "evidence": "Published assets, checksums and build provenance."}
  ],
  "extensions": [
    {"name": "project-specific work", "instructions": "Describe the additional required work.", "evidence": "Name the artifact or observation that proves it."}
  ]
}
```

Remove inapplicable entries, add as many targets or named extensions as needed,
and keep required commands consistent with the configured verifier and agent
tool grants. An empty `organization_profiles` or `extensions` array is valid;
an inaccessible target that remains listed is unfinished work.

The scratch directory must be inside the worktree and ignored by Git; add the
configured path to the adopting repository's ignore file before the run.
