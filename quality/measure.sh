#!/usr/bin/env bash
# Regenerate the code-health baselines (#335) from the tree and from the
# LCOV the exact coverage gate writes. Run scripts/coverage-exact.sh first.
# The tools and versions are in quality/README.md.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

out=quality
lcov=target/coverage/lcov.info
# The exact gate's test-file convention: `tests/` directories, `tests.rs`
# and `*_tests.rs`, plus `benches/`.
test_re='(^|/)tests/|(^|/)tests\.rs$|_tests\.rs$|(^|/)benches/'
test_glob='**/{tests/**/*.rs,tests.rs,*_tests.rs,benches/**/*.rs}'
test_ignore='**/tests/**,**/tests.rs,**/*_tests.rs,**/benches/**'

[ -f "$lcov" ] || {
  printf 'measure: %s is missing; run scripts/coverage-exact.sh first\n' "$lcov" >&2
  exit 1
}

# 1. CRAP. At the gate's 100% coverage CRAP equals cyclomatic complexity.
# Test files are not in the LCOV and would score as 0% covered, so every
# test path is excluded. `--path .` rather than `--workspace` records
# repository-relative paths, which a ratchet on another checkout can match.
cargo crap --path . --lcov "$lcov" \
  --exclude 'target/**' --exclude '**/tests/**' --exclude '**/benches/**' \
  --exclude '**/tests.rs' --exclude '**/*_tests.rs' \
  --sort file --format json --output "$out/crap-baseline.json"

# 2. Lines per Rust file, production and test apart, sorted by path.
lines() { while IFS= read -r file; do printf '%6d %s\n' "$(wc -l < "$file")" "$file"; done; }
{
  printf '# production\n'
  git ls-files '*.rs' | grep -v -E "$test_re" | sort | lines
  printf '# test\n'
  git ls-files '*.rs' | grep -E "$test_re" | sort | lines
} > "$out/file-lines.txt"

# 3. Duplication. `--max-lines` caps whole files, not clone blocks: at its
# default of 1,000 jscpd silently skips every file longer than that, which
# is every module this baseline exists to watch.
scratch="$(mktemp -d "${TMPDIR:-/tmp}/forge-jscpd.XXXXXX")"
trap 'rm -rf "$scratch"' EXIT
jscpd_run() {
  local scope="$1"
  shift
  rm -f "$out/jscpd-baseline-$scope.json"
  jscpd --min-tokens 50 --min-lines 5 --max-lines 100000 --max-size 10mb \
    --mode mild --silent --no-tips --reporters json --output "$scratch/$scope" \
    --baseline "$out/jscpd-baseline-$scope.json" --update-baseline "$@" > /dev/null
}
jscpd_run prod --format rust --ignore "$test_ignore" crates
jscpd_run tests --format rust --pattern "$test_glob" crates
# Contracts, reference and fixtures are deliberately frozen copies.
jscpd_run data --format json,markdown \
  --ignore 'contracts/**,reference/**,fixtures/**,quality/**' .

# 4. Functions over clippy's default 100 lines. `--force-warn` reaches the
# ones an `#[allow]` silences, so the list is complete.
cargo clippy --workspace --all-targets --all-features --locked --message-format=json \
  -- -A clippy::all --force-warn clippy::too_many_lines 2> /dev/null |
  jq -r 'select(.reason == "compiler-message"
           and .message.code.code == "clippy::too_many_lines")
         | (.message.message | capture("\\((?<n>[0-9]+)/").n) as $n
         | .message.spans[] | select(.is_primary)
         | "\($n) \(.file_name):\(.line_start)"' |
  sort -u -k2,2V |
  while read -r n at; do
    name="$(sed -n "${at##*:}p" "${at%:*}" | grep -oE 'fn [A-Za-z0-9_]+' | head -n 1)"
    printf '%4d %s %s\n' "$n" "$at" "${name#fn }"
  done |
  awk -v re="$test_re" '
    { split($2, at, ":"); if (at[1] ~ re) test[++t] = $0; else prod[++p] = $0 }
    END {
      print "# production"; for (i = 1; i <= p; i++) print prod[i]
      print "# test"; for (i = 1; i <= t; i++) print test[i]
    }' > "$out/too-many-lines.txt"
