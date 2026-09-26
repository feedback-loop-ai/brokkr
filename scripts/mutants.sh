#!/usr/bin/env bash
# Mutation testing (#289), report only. The exact coverage gate proves
# every production line runs; a surviving mutant is a line no test checks.
# The scope and the committed misses have one home: this script and
# quality/mutants/.
#
#   scripts/mutants.sh baseline <crate>   measure <crate>'s scope; rewrite its allow-list
#   scripts/mutants.sh in-diff <base>     mutate what this branch changed since <base>
#   scripts/mutants.sh shard <k>          one of eight weekly shards of the whole scope
#
# MUTANTS_OUT names the output directory (target/mutants by default) and
# MUTANTS_JOBS the parallel jobs (cargo-mutants' own default of one).
#
# A miss is reported, never failed on: the operator rules a gate from the
# baseline. Anything that stops the report from being true fails instead:
# a tool error, a failing unmutated tree, an unreadable diff, a missing
# allow-list or output.
set -euo pipefail
export LC_ALL=C
cd "$(git rev-parse --show-toplevel)"

allow=quality/mutants
out="${MUTANTS_OUT:-target/mutants}"
shards=8

# The scope: all of brokkr-core, and two of brokkr-protocol's refusal
# paths, the sealed secrets and the hands box. The rest of brokkr-protocol
# is unmeasured: quality/mutants/README.md says what and why (#289).
scope() {
  case "$1" in
    brokkr-core) printf '%s\n' --package brokkr-core --file 'crates/brokkr-core/**' ;;
    brokkr-protocol)
      printf '%s\n' --package brokkr-protocol \
        --file crates/brokkr-protocol/src/secret.rs \
        --file crates/brokkr-protocol/src/hands.rs
      ;;
    *) printf 'mutants: %s is outside the measured scope\n' "$1" >&2 && return 1 ;;
  esac
}
crates=(brokkr-core brokkr-protocol)

# The scope's arguments for the crates named, one per array element. A
# read loop, not mapfile: macOS still ships bash 3.2.
scope_args() {
  args=()
  local crate line
  for crate in "$@"; do
    while IFS= read -r line; do args+=("$line"); done < <(scope "$crate")
  done
}

# Run cargo-mutants. Exit 0 (every mutant caught), 2 (misses) and 3
# (timeouts) are measurements; every other status is a failure to measure.
measure() {
  local status=0
  local jobs=()
  [ -z "${MUTANTS_JOBS:-}" ] || jobs=(--jobs "$MUTANTS_JOBS")
  cargo mutants --no-shuffle --output "$out" ${jobs[@]+"${jobs[@]}"} "$@" || status=$?
  case "$status" in
    0 | 2 | 3) [ -f "$out/mutants.out/missed.txt" ] || {
      printf 'mutants: %s/mutants.out/missed.txt is missing\n' "$out" >&2
      return 1
    } ;;
    *) printf 'mutants: cargo mutants exited %s\n' "$status" >&2 && return "$status" ;;
  esac
}

# A miss's identity is its file and mutation, without line and column,
# so an unrelated edit above it does not make it new.
identity() { sed -E 's/^([^:]+):[0-9]+:[0-9]+: /\1: /' "$@" | sort; }

# Print the misses in $out that no committed allow-list names.
report() {
  local known fresh
  known="$(mktemp)" fresh="$(mktemp)"
  for crate in "${crates[@]}"; do
    [ -f "$allow/$crate.missed.txt" ] || {
      printf 'mutants: %s/%s.missed.txt is missing\n' "$allow" "$crate" >&2
      return 1
    }
    identity "$allow/$crate.missed.txt" >> "$known"
  done
  sort -o "$known" "$known"
  identity "$out/mutants.out/missed.txt" | comm -23 - "$known" > "$fresh"
  {
    printf '### Mutants no committed miss names: %s\n\n' "$(wc -l < "$fresh")"
    sed 's/^/- /' "$fresh"
  } | tee -a "${GITHUB_STEP_SUMMARY:-/dev/null}"
  rm -f "$known" "$fresh"
}

case "${1:-}" in
  baseline)
    crate="${2:?usage: scripts/mutants.sh baseline <crate>}"
    scope_args "$crate"
    measure "${args[@]}"
    # Sorted, so a refresh's diff shows only what changed, not the order
    # the jobs happened to finish in.
    sort "$out/mutants.out/missed.txt" > "$allow/$crate.missed.txt"
    ;;
  in-diff)
    base="${2:?usage: scripts/mutants.sh in-diff <base commit>}"
    mkdir -p "$out"
    git diff "$base...HEAD" > "$out/branch.diff"
    measure --in-diff "$out/branch.diff" --package brokkr-core --package brokkr-protocol
    report
    ;;
  shard)
    k="${2:?usage: scripts/mutants.sh shard <k>}"
    scope_args "${crates[@]}"
    measure --shard "$k/$shards" --baseline=skip "${args[@]}"
    report
    ;;
  *) printf 'usage: scripts/mutants.sh baseline <crate> | in-diff <base> | shard <k>\n' >&2 && exit 1 ;;
esac
