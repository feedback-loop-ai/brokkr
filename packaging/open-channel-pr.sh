#!/usr/bin/env bash
# Open a pull request in a sibling channel repository (the homebrew tap,
# the scoop bucket) carrying one rendered file.
#
#   bash packaging/open-channel-pr.sh --token <token> --repo <owner/name> \
#        --source packaging/homebrew/brokkr.rb --destination Formula/brokkr.rb \
#        --version 0.6.0
#
# The token is a repository secret the bench provisions, and the only
# thing this script does with it is hand it to git and gh for that one
# repository (decision 0012). It is never printed, never written to a
# file that survives the run, and never used against this repository.
set -euo pipefail

token=""
repo=""
source_file=""
destination=""
version=""

die() {
  printf 'open-channel-pr: %s\n' "$1" >&2
  exit 1
}

while [ $# -gt 0 ]; do
  case "$1" in
    --token) token="${2:-}"; shift 2 ;;
    --repo) repo="${2:-}"; shift 2 ;;
    --source) source_file="${2:-}"; shift 2 ;;
    --destination) destination="${2:-}"; shift 2 ;;
    --version) version="${2:-}"; shift 2 ;;
    *) die "unknown argument: $1" ;;
  esac
done

[ -n "$token" ] || die "--token is required"
[ -n "$repo" ] || die "--repo <owner/name> is required"
[ -n "$source_file" ] || die "--source <path> is required"
[ -n "$destination" ] || die "--destination <path> is required"
[ -n "$version" ] || die "--version <x.y.z> is required"
[ -f "$source_file" ] || die "no such file: $source_file"

# A rendered file still carrying the placeholder digest would publish a
# formula nobody can install. Refuse rather than open that pull request.
if grep -q '0000000000000000000000000000000000000000000000000000000000000000' "$source_file"; then
  die "$source_file still carries a placeholder digest — run packaging/bump-from-sums.sh first"
fi

command -v gh >/dev/null 2>&1 || die "missing required tool: gh"

rendered="$(mktemp)"
cp "$source_file" "$rendered"

work="$(mktemp -d)"
cleanup() {
  rm -rf "$work" "$rendered"
}
trap cleanup EXIT

export GH_TOKEN="$token"
gh repo clone "$repo" "$work/checkout" -- --depth 1 >/dev/null 2>&1 ||
  die "cannot clone $repo — is the token scoped to it?"

cd "$work/checkout"
branch="brokkr-${version}"
git checkout -b "$branch" >/dev/null
mkdir -p "$(dirname "$destination")"
cp "$rendered" "$destination"

# Staged first, then compared against the index: `git diff` alone is
# silent about a destination the tap does not carry yet, so the very
# first bump into a fresh tap would report "nothing to open" and open
# nothing — a channel that never gets its formula, and a green release.
git config user.name "brokkr release"
git config user.email "noreply@github.com"
git add "$destination"

if git diff --cached --quiet -- "$destination"; then
  printf 'open-channel-pr: %s already carries v%s, nothing to open\n' "$repo" "$version"
  exit 0
fi

git commit --quiet --message "brokkr ${version}"
git push --quiet origin "$branch"

gh pr create \
  --repo "$repo" \
  --head "$branch" \
  --title "brokkr ${version}" \
  --body "Digests rendered from the attested \`SHA256SUMS\` of brokkr v${version}. Same artifacts as the GitHub release; nothing rebuilt."

printf 'open-channel-pr: opened a pull request against %s for v%s\n' "$repo" "$version"
