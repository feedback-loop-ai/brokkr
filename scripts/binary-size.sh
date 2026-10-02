#!/usr/bin/env bash
# The binary size budget (#342). The release binary is built under the
# pinned toolchain and the locked tree, so its size is a fact of the source,
# and it must not pass the committed ceiling: growth past it fails until the
# pull request that grows it raises the ceiling and says why. The ceiling is
# a bound, not a measurement (operator ruling, 2026-10-01: 30 MB), so a
# binary under it passes however far under it is; each run prints the size.
set -euo pipefail

binary="$1"
budget_file="$2"

# The file must declare one budget and nothing ambiguous: an ambiguous
# budget that reaches the comparison is a pass, so every check fails
# closed. A parse keeps only the last of a repeated key, so a file that
# declares the budget twice (a merge that kept both lines) would be read
# as the later one; --stream sees every declaration, and budgets.bytes
# must be written exactly once in the whole file.
declared="$(jq -n --stream '[inputs | select(length == 2 and .[0] == ["budgets", "bytes"])] | length' "$budget_file")" || declared=0
[[ $declared == 1 ]] || {
  printf 'binary size refusal: %s holds no whole positive budgets.bytes\n' "$budget_file" >&2
  exit 1
}
# Read whole (-s), it must be exactly one document whose budgets.bytes is
# one whole positive number: two documents would print two lines, and a
# comparison against them is a bash arithmetic error an `if` reads as false.
budget="$(jq --slurp --exit-status --raw-output 'if length == 1 then .[0].budgets.bytes | select(type == "number" and . == floor and . > 0) else empty end' "$budget_file")" || {
  printf 'binary size refusal: %s holds no whole positive budgets.bytes\n' "$budget_file" >&2
  exit 1
}
[[ $budget =~ ^[1-9][0-9]{0,17}$ ]] || {
  printf 'binary size refusal: %s holds no whole positive budgets.bytes\n' "$budget_file" >&2
  exit 1
}
size="$(wc -c < "$binary")"
size="${size//[[:space:]]/}"
[[ $size =~ ^[0-9]+$ ]] || {
  printf 'binary size refusal: the size of %s does not read as a number: %s\n' "$binary" "$size" >&2
  exit 1
}
printf 'binary size: %s is %s bytes; the ceiling is %s\n' "$binary" "$size" "$budget"
if ((size > budget)); then
  printf 'binary size refusal: %s bytes is past the ceiling of %s; raise %s in this pull request and say why\n' "$size" "$budget" "$budget_file" >&2
  exit 1
fi
