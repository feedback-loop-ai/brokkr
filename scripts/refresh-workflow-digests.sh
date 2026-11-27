#!/usr/bin/env bash
# Refresh or verify .github/workflow-digests.txt from the files under
# .github/actions/ and .github/workflows/ (issue #1417, operator ruling
# 2026-11-27).
#
# Every file under those two directories is hashed by SHA-256 over its checked-out
# bytes and written in `sha256sum` format (`<64 lowercase hex>  <path>`), sorted
# by path in byte order, with no blank lines, no comments, and a final newline.
# Symbolic links and any non-regular, non-directory entries under either
# directory (or at the target digest file) are refused, never followed.
#
# Renovate runs this script as a post-upgrade task on every branch that can
# modify a workflow or local action. If Renovate's wiring is ever wrong, its
# pull request fails the workflow_digests guard, and one run of this script by
# hand mends it.
#
#   scripts/refresh-workflow-digests.sh          rewrite .github/workflow-digests.txt
#   scripts/refresh-workflow-digests.sh --check  verify without writing (non-zero if stale)
set -euo pipefail
export LC_ALL=C

fail() {
  printf 'refresh-workflow-digests: %s\n' "$*" >&2
  exit 1
}

check_only=false
if [ "$#" -eq 1 ] && [ "$1" = --check ]; then
  check_only=true
elif [ "$#" -ne 0 ]; then
  printf 'usage: refresh-workflow-digests.sh [--check]\n' >&2
  exit 2
fi

cd "$(git rev-parse --show-toplevel)"

if command -v sha256sum > /dev/null 2>&1; then
  hasher=sha256sum
elif command -v shasum > /dev/null 2>&1; then
  hasher=shasum
else
  fail "neither sha256sum nor shasum is on PATH"
fi

sha256_file() {
  local raw digest
  if [ "$hasher" = sha256sum ]; then
    raw="$(sha256sum -- "$1")" || fail "sha256sum failed on $1"
  else
    raw="$(shasum -a 256 -- "$1")" || fail "shasum failed on $1"
  fi
  digest="${raw%% *}"
  case "$digest" in
    *[!0-9a-f]*) fail "unexpected digest for $1: $digest" ;;
  esac
  [ "${#digest}" -eq 64 ] || fail "unexpected digest length for $1: $digest"
  printf '%s\n' "$digest"
}

target=".github/workflow-digests.txt"
[ ! -L ".github" ] || fail "refusing symbolic link: .github"
[ ! -L "$target" ] || fail "refusing symbolic link: $target"

for dir in .github/actions .github/workflows; do
  [ ! -L "$dir" ] || fail "refusing symbolic link: $dir"
  [ -d "$dir" ] || fail "missing directory: $dir"
done

scratch="$(mktemp -d "${TMPDIR:-/tmp}/refresh-workflow-digests.XXXXXX")"
trap 'rm -rf "$scratch"' EXIT

bad="$(find -P .github/actions .github/workflows ! -type f ! -type d -print)"
if [ -n "$bad" ]; then
  first="$(printf '%s\n' "$bad" | head -n 1)"
  fail "refusing non-regular entry: $first"
fi

find -P .github/actions .github/workflows -type f -print | LC_ALL=C sort > "$scratch/paths"

: > "$scratch/digests"
count=0
while IFS= read -r path; do
  [ -n "$path" ] || continue
  [ ! -L "$path" ] && [ -f "$path" ] || fail "unreadable path: $path"
  digest="$(sha256_file "$path")"
  printf '%s  %s\n' "$digest" "$path" >> "$scratch/digests"
  count=$((count + 1))
done < "$scratch/paths"

[ "$count" -gt 0 ] || fail "no files found under .github/actions or .github/workflows"

if "$check_only"; then
  [ -f "$target" ] || fail "$target is missing; run bash scripts/refresh-workflow-digests.sh"
  if cmp -s "$scratch/digests" "$target"; then
    exit 0
  fi
  fail "$target is out of date; run bash scripts/refresh-workflow-digests.sh"
fi

if [ -f "$target" ] && cmp -s "$scratch/digests" "$target"; then
  exit 0
fi

cat "$scratch/digests" > "$target"
