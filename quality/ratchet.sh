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

# parse_file_lines <file> <label> <noun> <dest>: the "<count> <path>" entries
# of a file-lines listing, as "<count> <path> <ceiling>" in <dest>. A file's
# ceiling follows its path, read with the gate's one test vocabulary
# (test_re), never the section it sits under. A line that is not an entry, an
# entry under the wrong section, or a listing with no entry at all is written
# to <dest>.offenses.
parse_file_lines() {
  local file="$1" label="$2" noun="$3" dest="$4" production test
  production="$(ceiling linesProductionFile)"
  test="$(ceiling linesTestFile)"
  : > "$dest.offenses"
  RE="$test_re" LABEL="$label" NOUN="$noun" BAD="$dest.offenses" \
    awk -v production="$production" -v test="$test" '
    BEGIN { re = ENVIRON["RE"]; bad = ENVIRON["BAD"] }
    /^# production$/ { section = "production"; next }
    /^# test$/ { section = "test"; next }
    /^#/ { next }
    !/^ *[0-9]+ [^ ]+$/ { printf "%s:%d is not \"<count> <path>\"\n", ENVIRON["LABEL"], FNR > bad; next }
    {
      class = ($2 ~ re) ? "test" : "production"
      if (section != class) {
        printf "%s:%d: %s is %s code, listed under %s\n", ENVIRON["LABEL"], FNR, $2, class,
          (section == "" ? "no section" : "# " section) > bad
        next
      }
      print $1, $2, (class == "test" ? test : production); n++
    }
    END { if (!n) printf "%s: no %s parsed\n", ENVIRON["LABEL"], ENVIRON["NOUN"] > bad }
  ' "$file" > "$dest"
}

files() {
  [ -s "$out/file-lines.txt" ] || refuse "quality/file-lines.txt is missing or empty"
  parse_file_lines "$out/file-lines.txt" quality/file-lines.txt "baseline entry" "$scratch/base-lines"
  file_lines > "$scratch/current.txt"
  parse_file_lines "$scratch/current.txt" "the measured tree" "Rust file" "$scratch/current-lines"
  cat "$scratch/base-lines.offenses" "$scratch/current-lines.offenses" > "$scratch/files.offenses"
  # A file may grow to its ceiling or its baseline, whichever is higher.
  awk 'FILENAME == ARGV[1] { base[$2] = $1; next }
    {
      allowed = ($2 in base && base[$2] > $3) ? base[$2] : $3
      if ($1 > allowed) printf "%s: %d lines, over %d\n", $2, $1, allowed
    }' "$scratch/base-lines" "$scratch/current-lines" >> "$scratch/files.offenses"
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

# pair <path>: <path> at <rev> into $scratch/base; true when both sides
# hold something. A baseline emptied on either side is malformed, and a
# removed one is recorded by raised_since itself.
pair() {
  base_file "$rev" "$1" "$scratch/base"
  [ -f "$scratch/base" ] && [ -e "$1" ] || return 1
  if [ ! -s "$scratch/base" ] || [ ! -s "$1" ]; then
    printf '%s: empty at %s or here\n' "$1" "$rev" >> "$scratch/malformed"
    return 1
  fi
}

# json_holds <path> <jq condition> <what>: refuse a JSON baseline, on either
# side, that does not have the shape its comparison reads.
json_holds() {
  local side
  for side in "$scratch/base" "$1"; do
    jq -e "$2" "$side" > /dev/null 2>&1 || {
      printf '%s: %s (%s)\n' "$1" "$3" "$([ "$side" = "$1" ] && echo here || echo "at $rev")" >> "$scratch/malformed"
      return 1
    }
  done
}

# too_many_lines_keyed <file>: "<path> <name> #<n> <lines>" for each entry of
# a too-many-lines listing, twins of one name in one file numbered in line
# order. A line that is not "<lines> <path>:<line> [<name>]" is malformed.
too_many_lines_keyed() {
  LABEL="$2" BAD="$scratch/malformed" awk '
    /^#/ || !NF { next }
    !/^ *[0-9]+ [^ ]+:[0-9]+( [A-Za-z0-9_]+)?$/ {
      printf "%s:%d is not \"<lines> <path>:<line> <name>\"\n", ENVIRON["LABEL"], FNR >> ENVIRON["BAD"]; next
    }
    {
      split($2, at, ":"); key = at[1] " " ($3 == "" ? "?" : $3)
      print key " #" (seen[key]++), $1
    }' "$1"
}

# Every raised baseline since <rev>, one line each, into $scratch/raised, and
# every baseline this check cannot read, into $scratch/malformed.
raised_since() {
  local cc scope path crate before after
  rev="$1"
  cc="$(ceiling ccNewFunction)"
  : > "$scratch/raised"
  : > "$scratch/malformed"
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
  if pair quality/crap-baseline.json &&
    json_holds quality/crap-baseline.json '(.entries | type) == "array" and (.entries | length) > 0' "no baseline entry parsed"; then
    jq -r -n --argjson cc "$cc" --slurpfile base "$scratch/base" --slurpfile head "$out/crap-baseline.json" '
      def keyed: [.entries[]] | group_by([.file, .function])
        | map(sort_by(.line) | to_entries[] | {key: "\(.value.file) \(.value.function) #\(.key)", value: .value.cyclomatic})
        | from_entries;
      ($base[0] | keyed) as $b | ($head[0] | keyed) as $h
      | $h | to_entries[] | select(.value > $cc and (($b[.key] // -1) < .value))
      | "crap-baseline.json: \(.key) at CC \(.value) (was \($b[.key] // "absent"))"' >> "$scratch/raised"
  fi
  # File size: a file over its ceiling that is new or grew, its ceiling
  # following its path.
  if pair quality/file-lines.txt; then
    parse_file_lines "$scratch/base" "file-lines.txt at $rev" "baseline entry" "$scratch/base-lines"
    parse_file_lines "$out/file-lines.txt" "file-lines.txt" "baseline entry" "$scratch/head-lines"
    cat "$scratch/base-lines.offenses" "$scratch/head-lines.offenses" >> "$scratch/malformed"
    awk 'FILENAME == ARGV[1] { base[$2] = $1; next }
      $1 > $3 && (!($2 in base) || $1 > base[$2]) {
        printf "file-lines.txt: %s at %d lines (was %s)\n", $2, $1, ($2 in base) ? base[$2] : "absent"
      }' "$scratch/base-lines" "$scratch/head-lines" >> "$scratch/raised"
  fi
  # Function length (#337's reference): an entry that is new or grew.
  if pair quality/too-many-lines.txt; then
    too_many_lines_keyed "$scratch/base" "too-many-lines.txt at $rev" > "$scratch/base-long"
    too_many_lines_keyed "$out/too-many-lines.txt" too-many-lines.txt > "$scratch/head-long"
    awk 'FILENAME == ARGV[1] { base[$1 " " $2 " " $3] = $4; next }
      {
        key = $1 " " $2 " " $3
        if (!(key in base) || $4 > base[key])
          printf "too-many-lines.txt: %s at %d lines (was %s)\n", key, $4, (key in base) ? base[key] : "absent"
      }' "$scratch/base-long" "$scratch/head-long" >> "$scratch/raised"
  fi
  # Duplication: any fingerprint not in the earlier baseline.
  for scope in prod tests data; do
    path="quality/jscpd-baseline-$scope.json"
    pair "$path" && json_holds "$path" '(.fingerprints | type) == "object"' "no fingerprints object" || continue
    jq -r -n --arg scope "$scope" --slurpfile base "$scratch/base" --slurpfile head "$path" '
      $head[0].fingerprints | to_entries[] | select(.value > ($base[0].fingerprints[.key] // 0))
      | "jscpd-baseline-\($scope).json: new clone \(.key)"' >> "$scratch/raised"
  done
  # Suppressions (#337): a lint's count that rose, per section and kind.
  if pair quality/suppressions.txt; then
    awk '
      /^# (production|test)$/ { section = substr($0, 3); next }
      /^#/ || !NF { next }
      FILENAME == ARGV[1] { base[section " " $2 " " $3] = $1; next }
      {
        key = section " " $2 " " $3; was = (key in base) ? base[key] : 0
        if ($1 > was) printf "suppressions.txt: %s at %d (was %d)\n", key, $1, was
      }' "$scratch/base" "$out/suppressions.txt" >> "$scratch/raised"
  fi
  # Duplicate crate versions (#337): a skip the earlier list did not hold.
  # An emptied list is a shrink, so this one list may be empty.
  base_file "$rev" quality/duplicate-skips.txt "$scratch/base"
  if [ -f "$scratch/base" ] && [ -f "$out/duplicate-skips.txt" ]; then
    { grep -v '^#' "$out/duplicate-skips.txt" || true; } | { grep -vxF -f "$scratch/base" || true; } |
      sed 's/^/duplicate-skips.txt: new skip /' >> "$scratch/raised"
  fi
  # Public API: each crate's public items, and serde_json::Value in
  # brokkr-core's signatures (decision 0071 ruling 3).
  for path in "$out"/public-api/*.txt; do
    [ -e "$path" ] || continue
    crate="$(basename "$path" .txt)"
    after="$(grep -vc '^#' "$path" || true)"
    base_file "$rev" "$path" "$scratch/base"
    if [ -f "$scratch/base" ]; then
      before="$(grep -vc '^#' "$scratch/base" || true)"
    else
      before=absent
    fi
    if [ "$before" = absent ] || [ "$after" -gt "$before" ]; then
      printf 'public-api/%s.txt: %s public items (was %s)\n' "$crate" "$after" "$before" >> "$scratch/raised"
    fi
  done
  if pair quality/public-api/brokkr-core.txt; then
    before="$(grep -c 'serde_json::value::Value' "$scratch/base" || true)"
    after="$(grep -c 'serde_json::value::Value' "$out/public-api/brokkr-core.txt" || true)"
    [ "$after" -le "$before" ] ||
      printf 'public-api/brokkr-core.txt: %s items name serde_json::Value (was %s)\n' "$after" "$before" >> "$scratch/raised"
  fi
}

baselines() {
  rev="${1:-}"
  [ -n "$rev" ] || refuse "baselines needs the revision to compare with"
  git rev-parse --verify --quiet "$rev^{commit}" > /dev/null || refuse "cannot resolve $rev"
  raised_since "$rev"
  if [ -s "$scratch/malformed" ]; then
    sed 's/^/  /' "$scratch/malformed" >&2
    refuse "a baseline this check cannot read; no ruling passes it"
  fi
  if [ ! -s "$scratch/raised" ]; then
    printf 'ratchet: no baseline raised since %s\n' "$rev"
    return 0
  fi
  sed 's/^/  /' "$scratch/raised" >&2
  # A raised baseline passes only when the pull request names the ruling
  # that allowed it, on a line of its own: "Ruling: <where it was ruled>".
  # A web form sends the body with CRLF line ends, so each \r is dropped
  # first, and the ruling must name something that is not white space.
  local ruling
  ruling="$(printf '%s\n' "${PR_BODY:-}" | tr -d '\r' | grep -E '^Ruling:[[:space:]]+[^[:space:]]' | head -n 1 || true)"
  if [ -n "$ruling" ]; then
    printf 'ratchet: raised baselines allowed by: %s\n' "$ruling"
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
