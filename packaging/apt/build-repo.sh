#!/usr/bin/env bash
# Build a flat, static APT repository out of already-built .deb files.
#
#   build-repo.sh --debs <dir> --out <dir> [--suite stable] [--component main]
#
# What it writes, and nothing else:
#
#   <out>/pool/<component>/b/brokkr/<name>.deb
#   <out>/dists/<suite>/<component>/binary-<arch>/Packages
#   <out>/dists/<suite>/<component>/binary-<arch>/Packages.gz
#   <out>/dists/<suite>/Release
#
# It does NOT sign. Signing is a workflow step that consumes the steward
# key from a repository secret (decision 0012: names here, values only in
# the runner) — see packaging/README.md. A `Release` without its
# `InRelease`/`Release.gpg` beside it is an unfinished repository, and
# apt will say so, which is the honest failure.
#
# Dependencies are the ones an ubuntu runner already has: dpkg-deb and
# dpkg-scanpackages (dpkg-dev), gzip, sha256sum, md5sum.
set -euo pipefail

debs=""
out=""
suite="stable"
component="main"
origin="Brokkr"
label="Brokkr"

die() {
  printf 'apt/build-repo: %s\n' "$1" >&2
  exit 1
}

while [ $# -gt 0 ]; do
  case "$1" in
    --debs) debs="${2:-}"; shift 2 ;;
    --out) out="${2:-}"; shift 2 ;;
    --suite) suite="${2:-}"; shift 2 ;;
    --component) component="${2:-}"; shift 2 ;;
    --origin) origin="${2:-}"; shift 2 ;;
    --label) label="${2:-}"; shift 2 ;;
    *) die "unknown argument: $1" ;;
  esac
done

[ -n "$debs" ] || die "--debs <dir> is required"
[ -n "$out" ] || die "--out <dir> is required"
[ -d "$debs" ] || die "no such directory: $debs"

for tool in dpkg-deb dpkg-scanpackages gzip sha256sum md5sum; do
  command -v "$tool" >/dev/null 2>&1 || die "missing required tool: $tool"
done

mkdir -p "$out"
out="$(cd "$out" && pwd)"
debs="$(cd "$debs" && pwd)"

pool="pool/${component}/b/brokkr"
mkdir -p "$out/$pool"

# The pool holds exactly what this release built. Anything a previous
# run left behind is removed rather than silently re-indexed: an index
# that lists a .deb nobody can attest to is worse than a smaller one.
rm -f "$out/$pool"/*.deb

architectures=""
found=0
for deb in "$debs"/*.deb; do
  [ -e "$deb" ] || continue
  found=$((found + 1))
  arch="$(dpkg-deb --field "$deb" Architecture)"
  [ -n "$arch" ] || die "no Architecture field in $deb"
  cp "$deb" "$out/$pool/"
  case " $architectures " in
    *" $arch "*) ;;
    *) architectures="${architectures:+$architectures }$arch" ;;
  esac
done
[ "$found" -gt 0 ] || die "no .deb files in $debs"

# Sorted, so the Release file reads the same whatever order the pool was
# walked in — a repository index is a thing people diff.
architectures="$(printf '%s\n' $architectures | LC_ALL=C sort | tr '\n' ' ')"
architectures="${architectures% }"

cd "$out"

# One scan of the pool, then split by the stanza's own Architecture
# field. dpkg-scanpackages' `--arch` selects by *file name* (`*_amd64.deb`),
# and these files are named for the release artifact they are —
# `brokkr-linux-x86_64.deb` — so the field is the only honest key.
scan="$(mktemp)"
trap 'rm -f "$scan"' EXIT
# `pool` relative to $out, so every `Filename:` is repository-relative —
# which is exactly what apt joins onto the sources.list URL.
dpkg-scanpackages --multiversion pool /dev/null >"$scan"

for arch in $architectures; do
  dir="dists/${suite}/${component}/binary-${arch}"
  mkdir -p "$dir"
  # An `Architecture: all` package belongs in every index; nothing here
  # produces one today, and an index that quietly dropped it would be a
  # surprise later.
  awk -v arch="$arch" '
    BEGIN { RS = ""; ORS = "\n\n" }
    $0 ~ ("(^|\n)Architecture: " arch "(\n|$)") { print; next }
    $0 ~ "(^|\n)Architecture: all(\n|$)" { print }
  ' "$scan" >"$dir/Packages"
  [ -s "$dir/Packages" ] || die "empty Packages index for $arch"
  gzip -9nc "$dir/Packages" >"$dir/Packages.gz"
done

# `date -u -R` is locale-sensitive; apt wants the C locale's English
# day and month names. SOURCE_DATE_EPOCH makes the file reproducible,
# which is what lets a test assert on it.
release_date="$(LC_ALL=C date -u -d "@${SOURCE_DATE_EPOCH:-$(date -u +%s)}" '+%a, %d %b %Y %H:%M:%S UTC' 2>/dev/null ||
  LC_ALL=C date -u '+%a, %d %b %Y %H:%M:%S UTC')"

release="dists/${suite}/Release"
{
  printf 'Origin: %s\n' "$origin"
  printf 'Label: %s\n' "$label"
  printf 'Suite: %s\n' "$suite"
  printf 'Codename: %s\n' "$suite"
  printf 'Architectures: %s\n' "$architectures"
  printf 'Components: %s\n' "$component"
  printf 'Date: %s\n' "$release_date"
  printf 'Acquire-By-Hash: no\n'
  printf 'Description: Brokkr releases — the same attested artifacts the GitHub release carries\n'
} >"$release"

# Both digest sections, each entry `<hash> <size> <path>` with the path
# relative to the dists/<suite> directory apt fetched the Release from.
for section in MD5Sum SHA256; do
  printf '%s:\n' "$section" >>"$release"
  (
    cd "dists/${suite}"
    find "${component}" -type f \( -name Packages -o -name Packages.gz \) | LC_ALL=C sort |
      while IFS= read -r index; do
        case "$section" in
          MD5Sum) hash="$(md5sum "$index" | cut -d' ' -f1)" ;;
          SHA256) hash="$(sha256sum "$index" | cut -d' ' -f1)" ;;
        esac
        size="$(wc -c <"$index" | tr -d ' ')"
        printf ' %s %s %s\n' "$hash" "$size" "$index"
      done
  ) >>"$release"
done

printf 'apt/build-repo: %s suite, architectures: %s\n' "$suite" "$architectures"
printf 'apt/build-repo: unsigned — the workflow signs %s next\n' "$release"
