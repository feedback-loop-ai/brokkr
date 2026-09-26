#!/usr/bin/env bash
# Hold the tree to the committed baselines under quality/ (#338). Each check
# may only let a number shrink: a function, a file or a clone set may not grow
# past its baseline or, when new, past the ceiling decision 0071 ruling 4
# ruled (quality/ceilings.json). Every check fails closed: input it cannot
# read, or a measurement that produced nothing, is a refusal.
#
#   ratchet.sh crap [lcov]       cyclomatic complexity, after the exact gate
#   ratchet.sh crap-judge <json> the verdict on a cargo-crap --baseline report
#   ratchet.sh files             lines per Rust file
#   ratchet.sh clones            jscpd fingerprints, per scope
#   ratchet.sh api               the public-API snapshots, on the pinned nightly
#   ratchet.sh baselines <rev>   a baseline raised since <rev> needs a ruling
set -euo pipefail
export LC_ALL=C
cd "$(git rev-parse --show-toplevel)"
# shellcheck source=quality/lib.sh
. quality/lib.sh

out=quality
scratch="$(mktemp -d "${TMPDIR:-/tmp}/forge-ratchet.XXXXXX")"
trap 'rm -rf "$scratch"' EXIT

refuse() {
  printf 'ratchet refusal: %s\n' "$*" >&2
  exit 1
}

ceiling() {
  jq -e --arg key "$1" '.[$key] | numbers' "$out/ceilings.json" ||
    refuse "quality/ceilings.json has no number for $1"
}

# Print each offense line on stderr and refuse when there is any.
verdict() {
  local what="$1" offenses="$2"
  [ -s "$offenses" ] || { printf 'ratchet: %s holds\n' "$what"; return 0; }
  sed 's/^/  /' "$offenses" >&2
  refuse "$what: $(wc -l < "$offenses" | tr -d ' ') finding(s)"
}

crap_judge() {
  local report="$1" cc
  [ -f "$report" ] || refuse "no cargo-crap report at $report"
  cc="$(ceiling ccNewFunction)"
  # cargo-crap matches a function to its baseline by file and name, not by
  # line, so an edit above a function does not make it new. An existing
  # function may grow to the ceiling or its baseline, whichever is higher;
  # a new one only to the ceiling. At the gate's 100% coverage CRAP equals
  # cyclomatic complexity, so anything less than 100% is refused.
  jq -r --argjson cc "$cc" '
    def known: IN("new", "regressed", "unchanged", "improved");
    def allowance: if .status == "new" then $cc else ([.baseline_crap, $cc] | max) end;
    if (.entries | type) != "array" or (.entries | length) == 0
    then "no function was measured"
    elif (.diagnostics.source_only.count | type) != "number" or (.diagnostics.lcov_only.count | type) != "number"
    then "the report carries no source and LCOV match counts"
    elif .diagnostics.source_only.count > 0 or .diagnostics.lcov_only.count > 0
    then "the scan and the LCOV disagree: \(.diagnostics.source_only.count) source file(s) with no coverage, \(.diagnostics.lcov_only.count) covered file(s) not scored"
    else .entries[]
      | "\(.file):\(.line) \(.function)" as $at
      | if (.status | type) != "string" or (.status | known | not) then "\($at): status \(.status) is not one this check reads"
        elif .coverage != 100 then "\($at): \(.coverage)% covered, so CRAP is not cyclomatic complexity"
        elif (.crap | type) != "number" then "\($at): no CRAP score"
        elif .status != "new" and (.baseline_crap | type) != "number" then "\($at): matched a baseline with no CRAP score"
        elif .crap > allowance then "\($at): CC \(.crap) over \(allowance) (\(.status))"
        else empty end
    end' "$report" > "$scratch/crap.offenses" || refuse "the cargo-crap report is not readable JSON"
  verdict "cyclomatic complexity" "$scratch/crap.offenses"
}

crap() {
  local lcov="${1:-target/coverage/lcov.info}"
  [ -f "$lcov" ] || refuse "no LCOV at $lcov: run scripts/coverage-exact.sh first"
  [ -f "$out/crap-baseline.json" ] || refuse "no quality/crap-baseline.json"
  crap_report "$lcov" "$scratch/crap.json" --baseline "$out/crap-baseline.json"
  crap_judge "$scratch/crap.json"
}

files() {
  local production test
  production="$(ceiling linesProductionFile)"
  test="$(ceiling linesTestFile)"
  [ -f "$out/file-lines.txt" ] || refuse "no quality/file-lines.txt"
  file_lines > "$scratch/current.txt"
  # A file may grow to its ceiling or its baseline, whichever is higher. A
  # baseline line that is neither a comment nor "<count> <path>" is refused.
  awk -v production="$production" -v test="$test" '
    FNR == NR {
      if ($0 ~ /^#/) next
      if ($0 !~ /^ *[0-9]+ [^ ]+$/) { printf "quality/file-lines.txt:%d is not \"<count> <path>\"\n", FNR; bad = 1; next }
      base[$2] = $1; next
    }
    /^# production$/ { limit = production; next }
    /^# test$/ { limit = test; next }
    {
      seen++
      allowed = ($2 in base && base[$2] > limit) ? base[$2] : limit
      if ($1 > allowed) printf "%s: %d lines, over %d\n", $2, $1, allowed
    }
    END { if (!seen) print "no Rust file was measured" }
  ' "$out/file-lines.txt" "$scratch/current.txt" > "$scratch/files.offenses"
  verdict "file size" "$scratch/files.offenses"
}

clones() {
  local scope failed=0
  for scope in prod tests data; do
    [ -s "$out/jscpd-baseline-$scope.json" ] || refuse "no quality/jscpd-baseline-$scope.json"
    if ! jscpd_scope "$scope" "$scratch/$scope" --baseline "$out/jscpd-baseline-$scope.json" \
      --fail-on-new-clones --fail-on-empty > /dev/null; then
      failed=1
    fi
    [ -f "$scratch/$scope/jscpd-report.json" ] || refuse "jscpd wrote no report for $scope"
    jq -r --arg scope "$scope" '.duplicates[] | select(.isNew)
      | "\($scope): \(.lines) lines, \(.firstFile.name):\(.firstFile.start) and \(.secondFile.name):\(.secondFile.start)"' \
      "$scratch/$scope/jscpd-report.json" >> "$scratch/clones.offenses"
  done
  [ "$failed" = 0 ] || [ -s "$scratch/clones.offenses" ] || refuse "jscpd failed without naming a new clone (an empty scan?)"
  verdict "duplication" "$scratch/clones.offenses"
}

api() {
  local crate crates
  crates="$(api_crates)" || refuse "cargo metadata failed"
  [ -n "$crates" ] || refuse "no library crate to snapshot"
  : > "$scratch/api.offenses"
  for crate in $crates; do
    api_file "$crate" > "$scratch/$crate.txt" || refuse "cargo public-api failed for $crate"
    [ "$(wc -l < "$scratch/$crate.txt")" -gt 1 ] || refuse "cargo public-api printed nothing for $crate"
    if ! diff -u "$out/public-api/$crate.txt" "$scratch/$crate.txt" >> "$scratch/api.diff" 2>&1; then
      printf '%s: the public API differs from quality/public-api/%s.txt\n' "$crate" "$crate" >> "$scratch/api.offenses"
    fi
  done
  for snapshot in "$out"/public-api/*.txt; do
    crate="$(basename "$snapshot" .txt)"
    [ -f "$scratch/$crate.txt" ] || printf '%s: a snapshot for a crate that has no library\n' "$crate" >> "$scratch/api.offenses"
  done
  [ ! -s "$scratch/api.offenses" ] || cat "$scratch/api.diff" >&2
  verdict "public API" "$scratch/api.offenses"
}

# base_file <rev> <path> <dest>: the file at <rev>, or nothing when it is absent.
base_file() {
  if git cat-file -e "$1:$2" 2> /dev/null; then git show "$1:$2" > "$3"; else rm -f "$3"; fi
}

# Every raised baseline since <rev>, one line each, into $scratch/raised.
raised_since() {
  local rev="$1" cc production test scope path
  cc="$(ceiling ccNewFunction)"
  production="$(ceiling linesProductionFile)"
  test="$(ceiling linesTestFile)"
  : > "$scratch/raised"
  # The measuring rules themselves.
  for path in quality/ceilings.json quality/lib.sh quality/ratchet.sh quality/measure.sh; do
    base_file "$rev" "$path" "$scratch/base"
    if [ -f "$scratch/base" ] && ! cmp -s "$scratch/base" "$path"; then
      printf '%s: the measuring rules changed\n' "$path" >> "$scratch/raised"
    fi
  done
  # Every baseline present at <rev> must still be here. A removed one is
  # recorded here, and skipped by the comparisons below.
  git ls-tree -r --name-only "$rev" -- quality/ | while IFS= read -r path; do
    [ -e "$path" ] || printf '%s: removed\n' "$path"
  done >> "$scratch/raised"
  # Complexity: a function over the ceiling that is new or grew. Twins of one
  # name in one file pair in line order.
  base_file "$rev" quality/crap-baseline.json "$scratch/base"
  if [ -f "$scratch/base" ] && [ -f "$out/crap-baseline.json" ]; then
    jq -r -n --argjson cc "$cc" --slurpfile base "$scratch/base" --slurpfile head "$out/crap-baseline.json" '
      def keyed: [.entries[]] | group_by([.file, .function])
        | map(sort_by(.line) | to_entries[] | {key: "\(.value.file) \(.value.function) #\(.key)", value: .value.cyclomatic})
        | from_entries;
      ($base[0] | keyed) as $b | ($head[0] | keyed) as $h
      | $h | to_entries[] | select(.value > $cc and (($b[.key] // -1) < .value))
      | "crap-baseline.json: \(.key) at CC \(.value) (was \($b[.key] // "absent"))"' >> "$scratch/raised"
  fi
  # File size: a file over its ceiling that is new or grew.
  base_file "$rev" quality/file-lines.txt "$scratch/base"
  if [ -f "$scratch/base" ] && [ -f "$out/file-lines.txt" ]; then
    awk -v production="$production" -v test="$test" '
      FNR == NR { if ($0 !~ /^#/) base[$2] = $1; next }
      /^# production$/ { limit = production; next }
      /^# test$/ { limit = test; next }
      /^#/ { next }
      $1 > limit && (!($2 in base) || $1 > base[$2]) { printf "file-lines.txt: %s at %d lines (was %s)\n", $2, $1, ($2 in base) ? base[$2] : "absent" }
    ' "$scratch/base" "$out/file-lines.txt" >> "$scratch/raised"
  fi
  # Duplication: any fingerprint not in the earlier baseline.
  for scope in prod tests data; do
    base_file "$rev" "quality/jscpd-baseline-$scope.json" "$scratch/base"
    [ -f "$scratch/base" ] && [ -f "$out/jscpd-baseline-$scope.json" ] || continue
    jq -r -n --arg scope "$scope" --slurpfile base "$scratch/base" --slurpfile head "$out/jscpd-baseline-$scope.json" '
      $head[0].fingerprints | to_entries[] | select(.value > ($base[0].fingerprints[.key] // 0))
      | "jscpd-baseline-\($scope).json: new clone \(.key)"' >> "$scratch/raised"
  done
  # Suppressions (#337): a lint's count that rose, per section and kind.
  base_file "$rev" quality/suppressions.txt "$scratch/base"
  if [ -f "$scratch/base" ] && [ -f "$out/suppressions.txt" ]; then
    awk '
      /^# (production|test)$/ { section = substr($0, 3); next }
      /^#/ || !NF { next }
      FNR == NR { base[section " " $2 " " $3] = $1; next }
      {
        key = section " " $2 " " $3; was = (key in base) ? base[key] : 0
        if ($1 > was) printf "suppressions.txt: %s at %d (was %d)\n", key, $1, was
      }' "$scratch/base" "$out/suppressions.txt" >> "$scratch/raised"
  fi
  # Duplicate crate versions (#337): a skip the earlier list did not hold.
  base_file "$rev" quality/duplicate-skips.txt "$scratch/base"
  if [ -f "$scratch/base" ] && [ -f "$out/duplicate-skips.txt" ]; then
    grep -v '^#' "$out/duplicate-skips.txt" | grep -vxF -f "$scratch/base" |
      sed 's/^/duplicate-skips.txt: new skip /' >> "$scratch/raised" || true
  fi
  # Public API: serde_json::Value in brokkr-core's signatures (decision 0071 ruling 3).
  base_file "$rev" quality/public-api/brokkr-core.txt "$scratch/base"
  if [ -f "$scratch/base" ] && [ -f "$out/public-api/brokkr-core.txt" ]; then
    local before after
    before="$(grep -c 'serde_json::value::Value' "$scratch/base" || true)"
    after="$(grep -c 'serde_json::value::Value' "$out/public-api/brokkr-core.txt" || true)"
    [ "$after" -le "$before" ] ||
      printf 'public-api/brokkr-core.txt: %s items name serde_json::Value (was %s)\n' "$after" "$before" >> "$scratch/raised"
  fi
}

baselines() {
  local rev="${1:-}"
  [ -n "$rev" ] || refuse "baselines needs the revision to compare with"
  git rev-parse --verify --quiet "$rev^{commit}" > /dev/null || refuse "cannot resolve $rev"
  raised_since "$rev"
  if [ ! -s "$scratch/raised" ]; then
    printf 'ratchet: no baseline raised since %s\n' "$rev"
    return 0
  fi
  sed 's/^/  /' "$scratch/raised" >&2
  # A raised baseline passes only when the pull request names the ruling
  # that allowed it, on a line of its own: "Ruling: <where it was ruled>".
  if printf '%s\n' "${PR_BODY:-}" | grep -Eq '^Ruling: +[^ ]'; then
    printf 'ratchet: raised baselines allowed by: %s\n' "$(printf '%s\n' "${PR_BODY:-}" | grep -E '^Ruling: ' | head -n 1)"
    return 0
  fi
  refuse "a baseline was raised or a measuring rule changed; name the ruling in the pull request as \"Ruling: <reference>\""
}

case "${1:-}" in
  crap) shift; crap "$@" ;;
  crap-judge) shift; crap_judge "${1:?crap-judge needs a report}" ;;
  files) files ;;
  clones) clones ;;
  api) api ;;
  baselines) shift; baselines "$@" ;;
  *) refuse "usage: ratchet.sh crap [lcov] | crap-judge <report> | files | clones | api | baselines <rev>" ;;
esac
