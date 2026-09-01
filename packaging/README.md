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

### 2. GitHub Pages

Repository → Settings → Pages → Source: **GitHub Actions**. The `pages`
job then owns the whole site for this repository — before enabling it,
confirm nothing else publishes there.

The site holds only the current release: the pool is rebuilt each time,
so `apt-get install brokkr` and `apt-get upgrade` get the latest, and
older versions are not kept. A cumulative pool across releases is a
named follow-up below.

### 3. The tap and the bucket

Two sibling repositories the landing bench creates:

- `feedback-loop-ai/homebrew-brokkr`, file `Formula/brokkr.rb`
- `feedback-loop-ai/scoop-brokkr`, file `bucket/brokkr.json`

and two repository secrets holding a fine-grained token with
`contents: write` and `pull requests: write` **on that repository only**:

- `BROKKR_TAP_TOKEN`
- `BROKKR_BUCKET_TOKEN`

Until each is set the corresponding release step logs a warning and does
nothing — it never fails the release, and it never pretends it bumped a
channel it did not.

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
version. The committed templates carry sixty-four zeros, so an
unrendered file cannot be mistaken for a publishable one —
`packaging/open-channel-pr.sh` refuses to open a pull request carrying
the placeholder, and the test suite asserts the placeholder is still
there in this repository.

## Files

```
packaging/nfpm.yaml           .deb and .rpm metadata; the binary only
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
3. **Pinning nfpm.** The release installs it with `go install …@latest`,
   which is the only form that works unchanged on both runner
   architectures. A pin — a version or a digest — is a bench call.
4. **Publishing `brokkr-cli` to crates.io.** `cargo binstall brokkr-cli`
   cannot resolve a crate the registry has never seen, so the binstall
   metadata is correct and inert until a release publishes the crate.
   Whether Brokkr's crates belong on crates.io at all is the operator's
   call, not this directory's.
5. **The bootstrap spine.** The manager matrix landed as rows inside the
   quickstart's existing install section. When `slice-bootstrap` merges,
   its `| Step | Budget |` spine becomes the right home for them, and
   the 60-second tarball budget stays that path's gate: the manager rows
   are additional roads in, never a reason to loosen it.
