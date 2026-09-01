#!/usr/bin/env bash
# Render the channel templates from a release's own SHA256SUMS.
#
#   bash packaging/bump-from-sums.sh --version 0.6.0 --sums <path> [--root <dir>]
#
# Every digest a downstream channel publishes comes from this one file —
# the aggregate manifest the release attests — so homebrew, scoop and nix
# point at the same bytes the tarball path does. No channel computes its
# own hash, and nothing here downloads anything.
#
# Files rewritten in place:
#   flake.nix                     (version + the four unix tarballs)
#   packaging/homebrew/brokkr.rb  (version + the four unix tarballs)
#   packaging/scoop/brokkr.json   (version + the windows zip)
#
# The rule for the tagged files is one line: a line ending in a comment
# that names an artifact has its 64-hex-digit string replaced by that
# artifact's digest; a line tagged `# brokkr-version` has its quoted
# version replaced. Running it twice changes nothing the second time.
set -euo pipefail

version=""
sums=""
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

die() {
  printf 'bump-from-sums: %s\n' "$1" >&2
  exit 1
}

while [ $# -gt 0 ]; do
  case "$1" in
    --version) version="${2:-}"; shift 2 ;;
    --sums) sums="${2:-}"; shift 2 ;;
    --root) root="${2:-}"; shift 2 ;;
    *) die "unknown argument: $1" ;;
  esac
done

[ -n "$version" ] || die "--version <x.y.z> is required"
[ -n "$sums" ] || die "--sums <path> is required"
[ -f "$sums" ] || die "no such file: $sums"
case "$version" in
  v*) die "--version takes the number without its leading v: ${version#v}" ;;
esac

ARTIFACTS="brokkr-linux-x86_64.tar.gz brokkr-linux-aarch64.tar.gz brokkr-macos-arm64.tar.gz brokkr-macos-x86_64.tar.gz brokkr-windows-x86_64.zip"

digest_of() {
  awk -v want="$1" '
    { name = $NF; sub(/^\*/, "", name) }
    name == want { print $1; found = 1; exit }
    END { if (!found) exit 1 }
  ' "$sums"
}

for artifact in $ARTIFACTS; do
  digest="$(digest_of "$artifact")" ||
    die "SHA256SUMS names no $artifact — this is not a complete release manifest"
  # Every character, not just the first: what goes into a channel's
  # template has to be a digest and nothing else.
  case "$digest" in
    *[!0-9a-f]*) die "$artifact: digest is not lowercase hex: $digest" ;;
  esac
  [ "${#digest}" -eq 64 ] || die "$artifact: digest is not 64 hex digits: $digest"
done

# One awk pass per tagged file: the artifact named in a line's trailing
# comment decides which digest that line gets.
render_tagged() {
  file="$1"
  [ -f "$file" ] || die "no such file: $file"
  digests=""
  for artifact in $ARTIFACTS; do
    digests="${digests}${artifact}=$(digest_of "$artifact") "
  done
  awk -v version="$version" -v digests="$digests" '
    BEGIN {
      split(digests, pairs, " ");
      for (i in pairs) {
        if (pairs[i] == "") continue;
        split(pairs[i], pair, "=");
        digest[pair[1]] = pair[2];
      }
    }
    {
      line = $0;
      if (match(line, /#[ \t]*brokkr-version[ \t]*$/)) {
        sub(/"[^"]*"/, "\"" version "\"", line);
      } else if (match(line, /#[ \t]*[A-Za-z0-9_.-]+$/)) {
        tag = substr(line, RSTART, RLENGTH);
        sub(/^#[ \t]*/, "", tag);
        # The quoted hex string, not a counted-repetition pattern: mawk
        # and gawk disagree about interval expressions and both must
        # render the same file.
        if (tag in digest) sub(/"[0-9a-f]*"/, "\"" digest[tag] "\"", line);
      }
      print line;
    }
  ' "$file" >"$file.bumped"
  mv "$file.bumped" "$file"
}

render_tagged "$root/flake.nix"
render_tagged "$root/packaging/homebrew/brokkr.rb"

# JSON carries no comments, so the scoop manifest is rewritten by key:
# one "version", one download URL, one "hash".
manifest="$root/packaging/scoop/brokkr.json"
[ -f "$manifest" ] || die "no such file: $manifest"
windows_digest="$(digest_of brokkr-windows-x86_64.zip)"
# The autoupdate block keeps scoop's own `$version` placeholder, so the
# URL rewrite skips any line that carries it.
sed -E \
  -e "s|\"version\": \"[^\"]*\"|\"version\": \"${version}\"|" \
  -e "/[\$]version/!s|/download/v[^/]*/brokkr-windows|/download/v${version}/brokkr-windows|" \
  -e "s|\"hash\": \"[0-9a-f]{64}\"|\"hash\": \"${windows_digest}\"|" \
  "$manifest" >"$manifest.bumped"
mv "$manifest.bumped" "$manifest"

printf 'bump-from-sums: rendered v%s from %s\n' "$version" "$sums"
