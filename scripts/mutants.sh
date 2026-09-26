#!/usr/bin/env bash
# Mutation testing (#289), report only. The exact coverage gate proves
# every production line runs; a surviving mutant is a line no test checks.
# The scope and the committed misses have one home: this script and
# quality/mutants/.
#
#   scripts/mutants.sh baseline <crate>   measure <crate>'s scope; rewrite its allow-list
#   scripts/mutants.sh in-diff <base>     mutate what this branch changed since <base>
#   scripts/mutants.sh shard <k>          one of eight weekly shards of the whole scope
#   scripts/mutants.sh weekly <dir>       the eight shards' outputs, compared once
#
# MUTANTS_OUT names the output directory (target/mutants by default) and
# MUTANTS_JOBS the parallel jobs (cargo-mutants' own default of one).
#
# A miss is reported, never failed on: the operator rules a gate from the
# baseline. Anything that stops the report from being true fails instead:
# a tool error, a failing unmutated tree, an unreadable diff, a run that
# tested nothing, a missing allow-list, shard or output.
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
  local crate line lines
  for crate in "$@"; do
    lines="$(scope "$crate")" || return 1
    while IFS= read -r line; do args+=("$line"); done <<< "$lines"
  done
}

# Run cargo-mutants. Exit 0 (every mutant caught), 2 (misses) and 3
# (timeouts) are measurements; every other status is a failure to measure.
# cargo-mutants also exits 0 when it finds nothing to mutate and writes
# nothing, so the output must be this run's and must hold a mutant.
measure() {
  local status=0 written
  local jobs=()
  [ -z "${MUTANTS_JOBS:-}" ] || jobs=(--jobs "$MUTANTS_JOBS")
  rm -rf "$out/mutants.out"
  cargo mutants --no-shuffle --output "$out" ${jobs[@]+"${jobs[@]}"} "$@" || status=$?
  case "$status" in
    0 | 2 | 3) ;;
    *) printf 'mutants: cargo mutants exited %s\n' "$status" >&2 && return "$status" ;;
  esac
  for written in missed.txt mutants.json; do
    [ -f "$out/mutants.out/$written" ] || {
      printf 'mutants: %s/mutants.out/%s is missing\n' "$out" "$written" >&2
      return 1
    }
  done
  [ "$(tr -d '[:space:]' < "$out/mutants.out/mutants.json")" != '[]' ] || {
    printf 'mutants: the run found no mutant to test\n' >&2
    return 1
  }
}

# A miss's identity is its file and mutation, without line and column,
# so an unrelated edit above it does not make it new.
identity() { sed -E 's/^([^:]+):[0-9]+:[0-9]+: /\1: /' "$@" | sort; }

# Every committed miss's identity, sorted.
known() {
  local crate files=()
  for crate in "${crates[@]}"; do
    [ -f "$allow/$crate.missed.txt" ] || {
      printf 'mutants: %s/%s.missed.txt is missing\n' "$allow" "$crate" >&2
      return 1
    }
    files+=("$allow/$crate.missed.txt")
  done
  identity "${files[@]}"
}

summary() { tee -a "${GITHUB_STEP_SUMMARY:-/dev/null}"; }

# Print the misses in $1 that no committed allow-list names. Identities
# are counted, so a second miss of a committed file and mutation is new.
# Only a run over the whole scope may subtract: a run over part of it
# would let each committed miss it did not mutate absorb a new one.
report_scope() {
  local committed fresh
  committed="$(mktemp)" fresh="$(mktemp)"
  known > "$committed"
  identity "$1" | comm -23 - "$committed" > "$fresh"
  {
    printf '### Mutants no committed miss names: %s\n\n' "$(wc -l < "$fresh")"
    sed 's/^/- /' "$fresh"
  } | summary
  rm -f "$committed" "$fresh"
}

# Print every miss in $1, a run over part of the scope. A miss that
# shares a committed one's file and mutation is marked, not dropped.
report_diff() {
  local committed line id
  committed="$(mktemp)"
  known > "$committed"
  {
    printf '### Mutants missed in this diff: %s\n\n' "$(wc -l < "$1")"
    while IFS= read -r line; do
      id="$(printf '%s\n' "$line" | identity)"
      if grep -qxF -e "$id" "$committed"; then
        printf -- '- %s (a committed miss shares its file and mutation: check the line)\n' "$line"
      else
        printf -- '- %s\n' "$line"
      fi
    done < "$1"
  } | summary
  rm -f "$committed"
}

# A timeout counts as caught, but it can hide a miss (quality/mutants/README.md).
timeouts() {
  [ -s "$1" ] || return 0
  {
    printf '\n### Timeouts, each a possible hidden miss: %s\n\n' "$(wc -l < "$1")"
    sed 's/^/- /' "$1"
  } | summary
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
    scope_args "${crates[@]}"
    # A diff with no mutant in the scope makes cargo-mutants exit 0 and
    # write nothing, which measure() refuses as a run that measured
    # nothing. Listing first names that case: only an empty list, from a
    # clean exit, reports zero; any other list must be measured.
    cargo mutants --list --in-diff "$out/branch.diff" "${args[@]}" > "$out/listed.txt"
    if [ ! -s "$out/listed.txt" ]; then
      rm -rf "$out/mutants.out"
      printf '### Mutants in this diff: 0\n' | summary
      exit 0
    fi
    measure --in-diff "$out/branch.diff" "${args[@]}"
    report_diff "$out/mutants.out/missed.txt"
    timeouts "$out/mutants.out/timeout.txt"
    ;;
  shard)
    k="${2:?usage: scripts/mutants.sh shard <k>}"
    scope_args "${crates[@]}"
    measure --shard "$k/$shards" --baseline=skip "${args[@]}"
    ;;
  weekly)
    # Each shard's mutants.out, as the workflow downloads them into
    # <dir>/mutants-shard-<k>/. Only all eight make the whole scope.
    dir="${2:?usage: scripts/mutants.sh weekly <dir>}"
    mkdir -p "$out"
    : > "$out/week.missed.txt"
    : > "$out/week.timeout.txt"
    k=0
    while [ "$k" -lt "$shards" ]; do
      [ -f "$dir/mutants-shard-$k/missed.txt" ] || {
        printf 'mutants: %s/mutants-shard-%s/missed.txt is missing\n' "$dir" "$k" >&2
        exit 1
      }
      cat "$dir/mutants-shard-$k/missed.txt" >> "$out/week.missed.txt"
      [ ! -f "$dir/mutants-shard-$k/timeout.txt" ] ||
        cat "$dir/mutants-shard-$k/timeout.txt" >> "$out/week.timeout.txt"
      k=$((k + 1))
    done
    report_scope "$out/week.missed.txt"
    timeouts "$out/week.timeout.txt"
    ;;
  *) printf 'usage: scripts/mutants.sh baseline <crate> | in-diff <base> | shard <k> | weekly <dir>\n' >&2 && exit 1 ;;
esac
