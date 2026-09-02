#!/usr/bin/env bash
# Build a dnf-consumable repository out of already-built .rpm files.
#
#   bash packaging/rpm/build-repo.sh --rpms <dir> --out <dir> \
#        --base-url https://feedback-loop-ai.github.io/brokkr/rpm [--layout-only]
#
# What it writes:
#
#   <out>/<basearch>/<name>.rpm            one directory per architecture
#   <out>/<basearch>/repodata/repomd.xml   createrepo_c's index
#   <out>/brokkr.repo                      the file a user drops in /etc/yum.repos.d
#
# `--layout-only` stops before createrepo_c. It exists so the tooling
# test can prove the layout and the .repo rendering on a machine without
# createrepo_c installed — it is never how the release publishes, and
# the workflow does not pass it.
#
# Like the apt side, this script does not sign: the workflow signs
# repomd.xml with the steward key from a repository secret.
set -euo pipefail

rpms=""
out=""
base_url=""
layout_only=0

die() {
  printf 'rpm/build-repo: %s\n' "$1" >&2
  exit 1
}

while [ $# -gt 0 ]; do
  case "$1" in
    --rpms) rpms="${2:-}"; shift 2 ;;
    --out) out="${2:-}"; shift 2 ;;
    --base-url) base_url="${2:-}"; shift 2 ;;
    --layout-only) layout_only=1; shift ;;
    *) die "unknown argument: $1" ;;
  esac
done

[ -n "$rpms" ] || die "--rpms <dir> is required"
[ -n "$out" ] || die "--out <dir> is required"
[ -n "$base_url" ] || die "--base-url <url> is required"
[ -d "$rpms" ] || die "no such directory: $rpms"

mkdir -p "$out"
out="$(cd "$out" && pwd)"
rpms="$(cd "$rpms" && pwd)"

# The artifact names the release matrix produces are `brokkr-linux-<arch>`,
# with `<arch>` in the tarballs' vocabulary. dnf substitutes $basearch,
# which speaks rpm's, so the directory is named in rpm's.
basearch_of() {
  case "$1" in
    *linux-x86_64*) printf 'x86_64\n' ;;
    *linux-aarch64*) printf 'aarch64\n' ;;
    *) printf '\n' ;;
  esac
}

architectures=""
found=0
for rpm in "$rpms"/*.rpm; do
  [ -e "$rpm" ] || continue
  arch="$(basearch_of "$(basename "$rpm")")"
  # A name the mapping does not recognise is a refusal, not a guess: an
  # rpm filed under the wrong $basearch is a repository that lies.
  [ -n "$arch" ] || die "cannot place $(basename "$rpm"): no architecture in its name"
  found=$((found + 1))
  # The stale-package clean happens once per architecture, the first
  # time one is seen. Inside the copy it would delete a package this
  # same run had already placed, the moment two .rpm files share a
  # $basearch — the apt builder cleans once for the same reason.
  case " $architectures " in
    *" $arch "*) ;;
    *)
      architectures="${architectures:+$architectures }$arch"
      mkdir -p "$out/$arch"
      rm -f "$out/$arch"/*.rpm
      ;;
  esac
  cp "$rpm" "$out/$arch/"
done
[ "$found" -gt 0 ] || die "no .rpm files in $rpms"

# gpgcheck=0 is deliberate and documented: this slice signs repository
# metadata (repo_gpgcheck=1) and not the packages themselves. Package
# signing is a named follow-up in packaging/README.md, not a silent gap.
cat >"$out/brokkr.repo" <<REPO
[brokkr]
name=Brokkr
baseurl=${base_url}/\$basearch
enabled=1
gpgcheck=0
repo_gpgcheck=1
gpgkey=${base_url%/rpm}/brokkr-archive-keyring.asc
metadata_expire=6h
REPO

if [ "$layout_only" -eq 1 ]; then
  printf 'rpm/build-repo: layout only, architectures: %s\n' "$architectures"
  exit 0
fi

command -v createrepo_c >/dev/null 2>&1 || die "missing required tool: createrepo_c"
for arch in $architectures; do
  createrepo_c --quiet "$out/$arch"
  [ -f "$out/$arch/repodata/repomd.xml" ] || die "createrepo_c wrote no repomd.xml for $arch"
done

printf 'rpm/build-repo: architectures: %s\n' "$architectures"
printf 'rpm/build-repo: unsigned — the workflow signs each repodata/repomd.xml next\n'
