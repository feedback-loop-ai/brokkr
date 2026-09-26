#!/usr/bin/env bash
# The binary size budget (#342). The release binary is built under the
# pinned toolchain and the locked tree, so its size is a fact of the source,
# and it must stay within one per cent of the committed size either way:
# growth past it fails until the pull request that grows it raises the
# budget and says why, and a shrink past it fails until the lower size is
# committed, so the budget follows the code down.
set -euo pipefail

binary="$1"
budget_file="$2"

budget="$(jq -er '.budgets.bytes | select(type == "number" and . == floor and . > 0)' "$budget_file")" || {
  printf 'binary size refusal: %s holds no whole positive budgets.bytes\n' "$budget_file" >&2
  exit 1
}
size="$(wc -c < "$binary")"
low=$((budget * 99 / 100))
high=$((budget * 101 / 100))
printf 'binary size: %s is %s bytes; the budget is %s (%s..%s)\n' "$binary" "$size" "$budget" "$low" "$high"
if ((size > high)); then
  printf 'binary size refusal: %s bytes is past %s, the budget plus one per cent; raise %s in this pull request and say why\n' "$size" "$high" "$budget_file" >&2
  exit 1
fi
if ((size < low)); then
  printf 'binary size refusal: %s bytes is below %s, the budget less one per cent; commit the lower size in %s\n' "$size" "$low" "$budget_file" >&2
  exit 1
fi
