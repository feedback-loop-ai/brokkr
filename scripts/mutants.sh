#!/usr/bin/env bash
# Mutation testing (#289). The exact coverage gate proves every
# production line runs; a surviving mutant is a line no test checks.
# The scope and the committed misses have one home: this script and
# quality/mutants/.
#
#   scripts/mutants.sh baseline <crate>        measure <crate>'s scope; rewrite its allow-list
#   scripts/mutants.sh gate <base> brokkr-core fail on a miss this branch adds to brokkr-core
#   scripts/mutants.sh in-diff <base> [crate]  report what this branch changed since <base>
#   scripts/mutants.sh shard <k>               one of eight weekly shards of the whole scope
#   scripts/mutants.sh weekly <dir>            the eight shards' outputs, compared once
#
# MUTANTS_OUT names the output directory (target/mutants by default),
# MUTANTS_JOBS the parallel jobs (cargo-mutants' own default of one), and
# MUTANTS_ALLOW the directory of committed misses (quality/mutants by
# default; crates/brokkr-cli/tests/mutants_gate.rs plants its own).
#
# The operator ruled on #289 (2026-09-26): brokkr-core's diff is gated, and
# a pull request that adds a miss there fails. Every other mode reports a
# miss and never fails on it. Anything that stops a verdict or a report
# from being true fails in every mode: a tool error, a failing unmutated
# tree, an unreadable diff, a run that tested nothing, a missing
# allow-list, shard or output.
set -euo pipefail
export LC_ALL=C
cd "$(git rev-parse --show-toplevel)"

allow="${MUTANTS_ALLOW:-quality/mutants}"
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

# Write the diff since $1 and list the mutants it touches in the scope of
# the crates that follow. A diff with no mutant in the scope makes
# cargo-mutants exit 0 and write nothing, which measure() refuses as a run
# that measured nothing. Listing first names that case: only an empty list,
# from a clean exit, means zero; any other list must be measured. A diff
# that does not apply to the tree exits non-zero and ends the script.
diff_mutants() {
  local base="$1"
  shift
  mkdir -p "$out"
  git diff "$base...HEAD" > "$out/branch.diff"
  scope_args "$@"
  cargo mutants --list --in-diff "$out/branch.diff" "${args[@]}" > "$out/listed.txt"
}

# The gate's verdict: print, and fail on, each miss in $2 whose file and
# mutation occur more often than in the committed list $1. A committed
# miss at a moved line is accounted for; a second one of the same file
# and mutation is not. A diff mutates only part of the scope, so a new
# miss that shares a committed one's file and mutation, where the diff
# did not reach the committed one, passes here: the weekly report over
# the whole scope names it.
gate_misses() {
  local fresh
  fresh="$(mktemp)"
  # The committed list is picked out by name, not by NR == FNR: an empty
  # list has no records, so that idiom would read every fresh miss as
  # committed and pass the gate. id() is identity()'s anchored rule.
  sort "$2" | awk -v committed="$1" '
    function id(line,   file) {
      if (!match(line, /^[^:]+:[0-9]+:[0-9]+: /)) return line
      file = line
      sub(/:.*/, "", file)
      return file ": " substr(line, RLENGTH + 1)
    }
    FILENAME == committed { have[id($0)]++; next }
    { key = id($0); if (++seen[key] > have[key]) print }
  ' "$1" - > "$fresh"
  if [ -s "$fresh" ]; then
    {
      printf '### brokkr-core misses this diff adds: %s\n\n' "$(wc -l < "$fresh")"
      sed 's/^/- /' "$fresh"
    } | summary
    printf 'mutants: this diff adds %s brokkr-core miss(es); a test must catch each (the #289 gate)\n' \
      "$(wc -l < "$fresh")" >&2
    rm -f "$fresh"
    return 1
  fi
  printf '### brokkr-core misses this diff adds: 0 (%s missed, each a committed miss)\n' \
    "$(wc -l < "$2")" | summary
  rm -f "$fresh"
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
  gate)
    base="${2:?usage: scripts/mutants.sh gate <base commit> brokkr-core}"
    crate="${3:?usage: scripts/mutants.sh gate <base commit> brokkr-core}"
    [ "$crate" = brokkr-core ] || {
      printf 'mutants: only brokkr-core is gated (the operator ruling on #289); %s reports\n' "$crate" >&2
      exit 1
    }
    [ -f "$allow/$crate.missed.txt" ] || {
      printf 'mutants: %s/%s.missed.txt is missing\n' "$allow" "$crate" >&2
      exit 1
    }
    diff_mutants "$base" "$crate"
    if [ ! -s "$out/listed.txt" ]; then
      rm -rf "$out/mutants.out"
      printf '### brokkr-core mutants in this diff: 0\n' | summary
      exit 0
    fi
    measure --in-diff "$out/branch.diff" "${args[@]}"
    timeouts "$out/mutants.out/timeout.txt"
    gate_misses "$allow/$crate.missed.txt" "$out/mutants.out/missed.txt"
    ;;
  in-diff)
    base="${2:?usage: scripts/mutants.sh in-diff <base commit> [crate...]}"
    shift 2
    if [ "$#" -eq 0 ]; then set -- "${crates[@]}"; fi
    diff_mutants "$base" "$@"
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
  *) printf 'usage: scripts/mutants.sh baseline <crate> | gate <base> brokkr-core | in-diff <base> [crate...] | shard <k> | weekly <dir>\n' >&2 && exit 1 ;;
esac
