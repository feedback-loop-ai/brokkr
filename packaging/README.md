# Packaging — every channel serves the same attested artifacts

One build per platform, in the release workflow, attested there. Every
channel below hands a user *those bytes*: the `.deb` and `.rpm` are made
by nfpm out of the binary the tarball leg already packaged, the apt and
dnf repositories serve those package files, and homebrew, scoop and nix
carry digests rendered from the release's own `SHA256SUMS`. Nothing in
this directory builds a second binary, and nothing computes a digest of
its own.

That is the whole design constraint. If a change here would make a
channel install something the release did not attest, it is the wrong
change.

## What is wired, and what is waiting on the bench

The accuracy law for this directory: a command is either exercised by
`crates/brokkr-cli/tests/packaging.rs` and CI, or it is marked **wired
at the bench** — meaning the code exists and is tested, and the last
step (a secret, a Pages site, a sibling repository) is the operator's.

| Channel | State | Proof |
|---|---|---|
| tarball + `SHA256SUMS` + attestation | working today | the release workflow, unchanged by this slice |
| `.deb` / `.rpm` build (nfpm) | working, first published in the next release | CI job `packaging` builds both from the real binary |
| apt repository (Pages) | **wired at the bench** — needs `BROKKR_APT_SIGNING_KEY` and Pages enabled | `packaging/apt/build-repo.sh` is run against real `.deb` files in CI and its `Release` digests are verified |
| dnf repository (Pages) | **wired at the bench** — same secret, same site | `packaging/rpm/build-repo.sh` is run in CI; `createrepo_c` produces real repodata |
| `cargo binstall` | **wired at the bench** — needs `brokkr-cli` published to crates.io | the binstall metadata is resolved against the release matrix in CI |
| nix flake | evaluates today; installs from the next release's rendered digests | `nix flake check` in CI |
| homebrew tap | **wired at the bench** — needs `BROKKR_TAP_TOKEN` and the tap repository | the render is tested; the pull request is not opened until the token exists |
| scoop bucket | **wired at the bench** — needs `BROKKR_BUCKET_TOKEN` and the bucket repository | same |

No `.deb`, `.rpm`, apt repository or dnf repository exists for v0.6.0:
this slice adds the machinery, and the first release to carry them is
the next tag. Any command below that names a package manager is
therefore describing that release, not v0.6.0 — the quickstart says so
in the same words.

## The operator's steps

### 1. The archive signing key (`BROKKR_APT_SIGNING_KEY`)

The apt `Release` file and each `repodata/repomd.xml` are signed by a
workflow step. Decision 0012: the repository holds the **name** of the
secret and never its value, and this slice generated no key material.

At the bench, once:

```
# On the operator's own machine, not in CI, not in a container that ships.
gpg --quick-generate-key "Brokkr archive signing key <you@example.com>" \
    ed25519 sign never
gpg --armor --export-secret-keys <fingerprint>   # paste into the secret
```

Then set, as **repository secrets**:

- `BROKKR_APT_SIGNING_KEY` — the ASCII-armoured *private* key block.
- `BROKKR_APT_SIGNING_KEY_PASSPHRASE` — the passphrase, or empty if the
  key has none.

The public half is never stored here: the workflow exports it from the
imported private key and publishes it at
`https://feedback-loop-ai.github.io/brokkr/brokkr-archive-keyring.asc`.

If `BROKKR_APT_SIGNING_KEY` is unset the `pages` job **fails**. It does
not publish an unsigned repository, because the install instructions
tell users the metadata is signed.

Expect that failure until this step is done, and read it for what it is:
`pages` runs *after* `publish`, so the release itself — tarballs,
packages, `SHA256SUMS`, attestation — is already complete and correct
when the run goes red. What is missing is the apt and dnf site, not the
artifacts every other channel serves.

The key's passphrase reaches gpg on stdin (`--passphrase-fd 0`) and the
key block the same way, and the tap and bucket tokens are passed to
`packaging/open-channel-pr.sh` by the *name* of their variable
(`--token-env`). None of the three is ever an argument: arguments are
readable from the runner's process table while the command runs.

### 2. GitHub Pages

Repository → Settings → Pages → Source: **GitHub Actions**. The `pages`
job then owns the whole site for this repository — before enabling it,
confirm nothing else publishes there.

The site holds only the current release: the pool is rebuilt each time,
so `apt-get install brokkr` and `apt-get upgrade` get the latest, and
older versions are not kept. A cumulative pool across releases is a
named follow-up below.

### 3. The tap and the bucket

Two sibling repositories, created by the operator on 2026-09-02:

- `feedback-loop-ai/homebrew-tap`, file `Formula/brokkr.rb` — installs
  as `brew install feedback-loop-ai/tap/brokkr`
- `feedback-loop-ai/scoop-bucket`, file `bucket/brokkr.json` — installs
  after `scoop bucket add brokkr https://github.com/feedback-loop-ai/scoop-bucket`

and two repository secrets holding a fine-grained token with
`contents: write` and `pull requests: write` **on that repository only**:

- `BROKKR_TAP_TOKEN`
- `BROKKR_BUCKET_TOKEN`

Until each is set the corresponding release step logs a warning and does
nothing — it never fails the release, and it never pretends it bumped a
channel it did not.

The flake-digest pull request against this repository is opened by the
workflow token, which only works where the organization (or the
enterprise above it) allows *Actions may create pull requests*. Where
that policy is off, a third fine-grained token with `contents: write`
and `pull requests: write` on this repository, held as
`BROKKR_FLAKE_PR_TOKEN`, is used instead; without it the step falls
back to the workflow token and fails with the policy's own message.

## Using the channels

Every command in this section that is not marked otherwise requires the
bench steps above.

**apt** (Debian, Ubuntu) — *wired at the bench*:

```
curl -fsSL https://feedback-loop-ai.github.io/brokkr/brokkr-archive-keyring.asc \
  | sudo tee /usr/share/keyrings/brokkr-archive-keyring.asc >/dev/null
echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/brokkr-archive-keyring.asc] https://feedback-loop-ai.github.io/brokkr/apt stable main" \
  | sudo tee /etc/apt/sources.list.d/brokkr.list
sudo apt-get update && sudo apt-get install brokkr
```

`apt-get upgrade` picks up later releases from the same line.

The `Release` file carries a `Valid-Until` 90 days past the release that
built it. That is what lets apt notice a frozen or replayed repository —
an old signature is still a valid one, so without an end date a stale
copy of the site is indistinguishable from the current one. The cost is
the other edge: 90 days after the last release, `apt-get update` refuses
the repository until a new tag rebuilds and re-signs the site. Cutting a
release resets it; follow-up 6 is the scheduled re-sign that removes the
dependence on cadence, and `--valid-days` on
`packaging/apt/build-repo.sh` is the knob.

**dnf** (Fedora, RHEL, openSUSE) — *wired at the bench*:

```
sudo curl -fsSL -o /etc/yum.repos.d/brokkr.repo \
  https://feedback-loop-ai.github.io/brokkr/rpm/brokkr.repo
sudo dnf install brokkr
```

The `.repo` file sets `repo_gpgcheck=1` (the metadata is signed) and
`gpgcheck=0` (the packages themselves are not signed yet — see the
follow-ups). Both facts are in the file, not just here.

Without the repository, a single release asset installs directly — this
form needs no Pages site and no secret, only a release that carries the
`.rpm`:

```
sudo dnf install https://github.com/feedback-loop-ai/brokkr/releases/download/vX.Y.Z/brokkr-linux-x86_64.rpm
```

**cargo binstall** — *wired at the bench*:

```
cargo binstall brokkr-cli
```

`cargo binstall` resolves a crate through crates.io first and only then
reads the `[package.metadata.binstall]` block that points it at our
release asset. Nothing in this repository publishes `brokkr-cli` to
crates.io, so this line waits on that — see the follow-ups. The metadata
itself is tested against the release matrix in CI today.

**nix**:

```
nix profile install github:feedback-loop-ai/brokkr
```

Installs from a release tarball rather than compiling. The flake's
digests are placeholders until a release renders them (see below), so
this works from the first tag after this slice lands.

## How a release renders the channels

`bash packaging/bump-from-sums.sh --version X.Y.Z --sums SHA256SUMS`
reads the release's attested manifest and rewrites, in place:

- `flake.nix` — version and the four unix tarball digests
- `packaging/homebrew/brokkr.rb` — the same four
- `packaging/scoop/brokkr.json` — version and the windows zip digest

The rule is one line: a line whose trailing comment names an artifact
gets that artifact's digest; a line tagged `# brokkr-version` gets the
version.

The two templates bound for sibling repositories are rendered on the
runner and never committed here: `packaging/open-channel-pr.sh` refuses
to open a pull request carrying the placeholder, and the test suite
asserts the sixty-four zeros are still in the tap formula and the scoop
manifest, so neither can be published out of band.

`flake.nix` is the exception, and deliberately: `nix profile install
github:feedback-loop-ai/brokkr` reads the *default branch*, so the flake
has to end up carrying live digests. The `channels` job opens that pull
request against the default branch after each release, and merging it is
what makes the nix row work. The test therefore asserts the flake's
*shape* — four artifact-tagged 64-hex digests, all placeholder or all
rendered, never a mix — rather than the zeros, which would go red the
moment the release's own pull request merged.

## Files

```
packaging/nfpm.yaml           .deb and .rpm metadata; the binary only
packaging/nfpm-version.txt    the pinned nfpm both workflows install
packaging/apt/build-repo.sh   pool + dists + Packages(.gz) + Release
packaging/rpm/build-repo.sh   per-$basearch tree + repodata + brokkr.repo
packaging/bump-from-sums.sh   renders the channel templates from SHA256SUMS
packaging/open-channel-pr.sh  opens the tap/bucket pull request
packaging/homebrew/brokkr.rb  formula template
packaging/scoop/brokkr.json   manifest template
packaging/pages/index.html    the Pages site's front door
packaging/aur/PKGBUILD        sketch only — see out of scope
flake.nix                     repo root; fetches the release tarball
```

None of the scripts carries an executable bit; each is invoked as
`bash packaging/…` — in the workflows, in the tests, and above.

## Out of scope, deliberately

- **AUR.** `packaging/aur/PKGBUILD` is a sketch of `brokkr-bin` and
  nothing publishes it: `aur.archlinux.org` accepts pushes from a
  registered maintainer's SSH key, which is the operator's account and
  not a workflow secret.
- **winget.** Per-release manifests submitted to `microsoft/winget-pkgs`
  run on that project's review cadence, not ours.
- **snap, flatpak.** Deferred. Both want their own build and runtime
  story, which is the opposite of this directory's one rule.

## Named follow-ups

1. **Package-level signatures.** nfpm can sign the `.rpm` (and the
   `.deb`) with the same steward key. Doing it would let the `.repo`
   file say `gpgcheck=1`. It is not done here because it would put the
   signing key into the build matrix, where the binaries are made.
2. **A cumulative apt pool.** Today's site carries the current release
   only; keeping older versions means fetching the previous site before
   rebuilding it.
3. **Verifying the nfpm pin, and keeping it current.**
   `packaging/nfpm-version.txt` pins the version both workflows install,
   and Go's checksum database makes that version's content immutable —
   so a publication made after this commit cannot change what fills an
   attested `.deb`. What this run could not do is reach the network to
   confirm the pinned tag is the *current* release: the first CI run
   proves it is installable, and bumping it to the newest v2 is a
   one-line change the bench makes with a network it can see.
   Pinning the module *digest* (a `tools/go.mod` and its `go.sum`) is
   the stricter form, and needs the same network to generate.
4. **Publishing `brokkr-cli` to crates.io.** `cargo binstall brokkr-cli`
   cannot resolve a crate the registry has never seen, so the binstall
   metadata is correct and inert until a release publishes the crate.
   Whether Brokkr's crates belong on crates.io at all is the operator's
   call, not this directory's. If ruled yes: the workspace's sibling
   dependencies already carry the version `cargo publish` requires, and
   the crates go up in dependency order, each waiting for the registry
   to index the one before —

   ```sh
   for crate in brokkr-core brokkr-store brokkr-protocol brokkr-view \
                brokkr-runtime brokkr-bridge brokkr-cli; do
     cargo publish -p "$crate" --locked
   done
   ```
5. **The bootstrap spine.** The manager matrix landed as rows inside the
   quickstart's existing install section. When `slice-bootstrap` merges,
   its `| Step | Budget |` spine becomes the right home for them, and
   the 60-second tarball budget stays that path's gate: the manager rows
   are additional roads in, never a reason to loosen it.
6. **Re-signing the apt metadata on a schedule.** `Valid-Until` bounds a
   replay at 90 days, which also means a quarter without a release turns
   into `apt-get update` refusing the repository. A scheduled workflow
   that rebuilt and re-signed the site from the current release would let
   the window shrink to Debian's week without ever lapsing. Not done here
   because it wants the signing key on a timer rather than on a tag, and
   where that key is allowed to run is the operator's call.
