#!/usr/bin/env bash
# Refresh or verify .github/workflow-digests.txt from every file under
# .github/actions/ and .github/workflows/, plus .github/renovate.json5 and
# .github/renovate-global.json5 (issue #1417, operator ruling 2026-11-27).
#
# Every pinned file is hashed by SHA-256 over its checked-out
# bytes and written in `sha256sum` format (`<64 lowercase hex>  <path>`), sorted
# by path in byte order, with no blank lines, no comments, and a final newline.
# Every path component under either directory must be one or more ASCII letters,
# digits, `.`, `_`, or `-`, and never `.` or `..`; any other name is refused with
# its bytes escaped before anything is hashed or written. Symbolic links and any
# non-regular, non-directory entries under either directory (or any non-regular
# entry at the target digest file) are refused, never followed. Writes go to a
# temporary file in `.github/` and are renamed atomically into place.
#
# Renovate runs this script as a post-upgrade task on every branch; when no
# pinned file changed it rewrites nothing and exits 0. If Renovate's wiring is
# ever wrong, its pull request fails the workflow_digests guard, and one run of
# this script by hand mends it.
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

escape_path() {
  printf '%s' "$1" | od -v -An -tx1 | awk '
    BEGIN {
      split("2d 2e 2f 30 31 32 33 34 35 36 37 38 39 41 42 43 44 45 46 47 48 49 4a 4b 4c 4d 4e 4f 50 51 52 53 54 55 56 57 58 59 5a 5f 61 62 63 64 65 66 67 68 69 6a 6b 6c 6d 6e 6f 70 71 72 73 74 75 76 77 78 79 7a", hx, " ")
      chars = "-./0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ_abcdefghijklmnopqrstuvwxyz"
      for (i = 1; i <= length(chars); i++) {
        ok[hx[i]] = substr(chars, i, 1)
      }
    }
    {
      for (i = 1; i <= NF; i++) {
        b = tolower($i)
        if (b in ok) {
          printf "%s", ok[b]
        } else {
          printf "\\x%s", b
        }
      }
    }
    END { printf "\n" }
  '
}

target=".github/workflow-digests.txt"
[ ! -L ".github" ] || fail "refusing symbolic link: .github"
[ -d ".github" ] || fail "missing directory: .github"
if [ -L "$target" ]; then
  fail "refusing symbolic link: $target"
fi
if [ -e "$target" ] && [ ! -f "$target" ]; then
  fail "refusing non-regular target: $target"
fi

for dir in .github/actions .github/workflows; do
  [ ! -L "$dir" ] || fail "refusing symbolic link: $dir"
  [ -d "$dir" ] || fail "missing directory: $dir"
done

pinned_files=(.github/renovate-global.json5 .github/renovate.json5)
for pinned in "${pinned_files[@]}"; do
  [ ! -L "$pinned" ] || fail "refusing symbolic link: $pinned"
  [ -e "$pinned" ] || fail "missing pinned file: $pinned"
  [ -f "$pinned" ] || fail "refusing non-regular entry: $pinned"
done

scratch="$(mktemp -d "${TMPDIR:-/tmp}/refresh-workflow-digests.XXXXXX")"
tmp_target=""
cleanup() {
  rm -rf "$scratch"
  if [ -n "$tmp_target" ]; then
    rm -f "$tmp_target"
  fi
}
trap cleanup EXIT

find -P .github/actions .github/workflows -print0 > "$scratch/entries"
: > "$scratch/raw_paths"
: > "$scratch/bad_names"
: > "$scratch/non_regular"

while IFS= read -r -d '' entry; do
  case "$entry" in
    .github/actions|.github/workflows)
      continue
      ;;
    .github/actions/*)
      rel="${entry#.github/actions/}"
      ;;
    .github/workflows/*)
      rel="${entry#.github/workflows/}"
      ;;
    *)
      printf '%s\n' "$(escape_path "$entry")" >> "$scratch/bad_names"
      continue
      ;;
  esac

  invalid=false
  case "$rel" in
    ''|*[!A-Za-z0-9._/-]*) invalid=true ;;
  esac
  case "/$rel/" in
    *//*|*/./*|*/../*) invalid=true ;;
  esac
  if "$invalid"; then
    printf '%s\n' "$(escape_path "$entry")" >> "$scratch/bad_names"
    continue
  fi

  if [ -L "$entry" ] || { [ ! -f "$entry" ] && [ ! -d "$entry" ]; }; then
    printf '%s\n' "$entry" >> "$scratch/non_regular"
    continue
  fi

  if [ -f "$entry" ]; then
    printf '%s\n' "$entry" >> "$scratch/raw_paths"
  fi
done < "$scratch/entries"

if [ -s "$scratch/bad_names" ]; then
  LC_ALL=C sort "$scratch/bad_names" | while IFS= read -r escaped; do
    printf 'refresh-workflow-digests: refusing invalid path name: %s\n' "$escaped" >&2
  done
  exit 1
fi

if [ -s "$scratch/non_regular" ]; then
  LC_ALL=C sort "$scratch/non_regular" > "$scratch/non_regular.sorted"
  IFS= read -r first < "$scratch/non_regular.sorted"
  fail "refusing non-regular entry: $first"
fi

for pinned in "${pinned_files[@]}"; do
  printf '%s\n' "$pinned" >> "$scratch/raw_paths"
done

LC_ALL=C sort "$scratch/raw_paths" > "$scratch/paths"

: > "$scratch/digests"
while IFS= read -r path; do
  [ -n "$path" ] || continue
  [ ! -L "$path" ] && [ -f "$path" ] || fail "unreadable path: $path"
  digest="$(sha256_file "$path")"
  printf '%s  %s\n' "$digest" "$path" >> "$scratch/digests"
done < "$scratch/paths"

if [ -f "$target" ]; then
  cmp_status=0
  cmp -s "$scratch/digests" "$target" || cmp_status=$?
  case "$cmp_status" in
    0) exit 0 ;;
    1)
      if "$check_only"; then
        fail "$target is out of date; run bash scripts/refresh-workflow-digests.sh"
      fi
      ;;
    *) fail "failed to compare $target (cmp exited $cmp_status)" ;;
  esac
elif "$check_only"; then
  fail "$target is missing; run bash scripts/refresh-workflow-digests.sh"
fi

tmp_target="$(mktemp "${target}.XXXXXX")"
cat "$scratch/digests" > "$tmp_target"
chmod 0644 "$tmp_target"
mv -f "$tmp_target" "$target"
tmp_target=""
