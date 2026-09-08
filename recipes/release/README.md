# release — prepare a release from the record

```console
brokkr run --recipe release --repo . --feature "Prepare v0.10.0 from the latest published release using the realm's Release configuration"
```

The library's `release-manager` reads the commits, prepares the version change
and release description, reconciles the configured documentation, and prepares
all matching organization-profile updates. A separate `reviewer` judges the
candidate and its external patches. The recipe inherits `fast`'s policy, boxed
exec verifier and shipper, including bounded returns on findings. Shipping here
means a reviewed local candidate with journal evidence; tagging, publishing and
applying external patches are subsequent operator actions.

## Configuration belongs to the realm

The portable [charter](../../agents/charters/release-manager.md) contains the
process. Project details are the **Release configuration** section of the
Markdown file named by the realm's `house`. This uses the existing house pin:
the configuration is included in the prompt and its content digest is recorded
in the run. No release-specific engine keys, hidden environment settings or
untracked configuration files are introduced.

Copy the [configuration template](configuration.example.md) into your house
file. Brokkr's own values live in [its house rules](../../docs/house-rules.md).
The structured fields are instructions to the agent, not a new parser or an
automatic command runner. Both the manager and reviewer receive them. The
target version belongs in the commission, so configuration survives a release.

Extend the arrays of version files, documentation targets, validation commands,
organization profiles and channels. Add project-specific work through
`extensions`, each naming instructions and required evidence. A required item
that cannot be performed blocks preparation; the charter does not silently
ignore it. Keep configuration inline in the house file so the existing pin
covers it; merely linking to another mutable file does not pin its contents.

The shipped hire is **opus → astra, both at medium effort**, as ruled by the
operator. Both use the workspace tool because Codex cannot express a native
tool allow-list. The agent definition enables workspace networking for release
and profile reads, binds the Rust toolchain and Linux resolver/CA files, and
masks registry credentials. These are this repository's configuration, separate
from the charter. Adopters configure their own model chain and binds. Native
fetch/search grants are unchanged. Networking is enabled, not restricted to a
URL allow-list; the charter limits its purpose to the commissioned preparation.

## Verification is still a machine job

This shipped recipe uses `fast`'s **Rust verifier**. The manager additionally
runs the configured checks, but their prose results do not change what the
deterministic verify gate executes. For another stack, seat `release-manager`
in that stack's delivery recipe, or extend `release` and explicitly override
`verify` with the stack's own boxed exec script and required toolchain binds.
The existing [recipe composition](../../docs/guides/recipe-authoring.md)
mechanism covers both cases; declaring a command in house text does not install
it, grant it or add it to the gate.

Checks that exercise the namespace boundary itself run outside the workspace
box: nesting is deliberately refused. Brokkr's configuration therefore assigns
full exact coverage to CI or host validation before tagging. The manager records
that external check as pending; an in-box test run does not certify it.

## Organization profiles and publication

The shipped networked workspace can read public release/profile repositories;
private targets need an explicitly configured access mechanism or preparation
reports the missing access. It does not expose the host's GitHub credentials.

A configured profile is a repository and path, with a project selector and an
application stage. The manager reads the remote file, prepares a local patch
against a recorded commit, and includes it in the release handoff for review.
All current-release references for the selected project are audited together;
historical references and other projects are preserved. Missing access blocks
that required preparation.

After publication, apply each reviewed patch against its recorded base, review
any intervening changes, and verify the live profile. A completed Brokkr run is
evidence about its own worktree, not a transaction across multiple repositories.
The handoff keeps publication, package channels and profile application pending
until their actual remote results have been checked.
