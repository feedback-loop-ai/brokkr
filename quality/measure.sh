#!/usr/bin/env bash
# Regenerate the code-health baselines (#335) from the tree and from the
# LCOV the exact coverage gate writes. Run scripts/coverage-exact.sh first,
# on a clean checkout. quality/ratchet.sh holds the tree to what this writes
# (#338); the tools and versions are in quality/README.md.
set -euo pipefail
# Byte order, not the host locale's collation, so every host sorts alike.
export LC_ALL=C
cd "$(git rev-parse --show-toplevel)"
# shellcheck source=quality/lib.sh
. quality/lib.sh

out=quality
lcov=target/coverage/lcov.info
[ -f "$lcov" ] || {
  printf 'measure: %s is missing; run scripts/coverage-exact.sh first\n' "$lcov" >&2
  exit 1
}

# producedBy <file> <tool and version>: stamp a JSON baseline with what made it.
producedBy() {
  jq --arg tool "$2" '. + {producedBy: {tool: $tool, command: "quality/measure.sh", host: "linux"}}' "$1" > "$1.new"
  mv "$1.new" "$1"
}

# 1. CRAP, which is cyclomatic complexity at the gate's 100% coverage.
crap_report "$lcov" "$out/crap-baseline.json"
producedBy "$out/crap-baseline.json" "$(cargo crap --version)"

# 2. Lines per Rust file.
{
  printf '# produced by quality/measure.sh with git ls-files and wc -l\n'
  file_lines
} > "$out/file-lines.txt"

# 3. Duplication, one fingerprint baseline per scope.
scratch="$(mktemp -d "${TMPDIR:-/tmp}/forge-jscpd.XXXXXX")"
trap 'rm -rf "$scratch"' EXIT
for scope in prod tests data; do
  rm -f "$out/jscpd-baseline-$scope.json"
  jscpd_scope "$scope" "$scratch/$scope" --baseline "$out/jscpd-baseline-$scope.json" --update-baseline > /dev/null
  producedBy "$out/jscpd-baseline-$scope.json" "$(jscpd --version)"
done

# 4. The public API of every library crate, on the pinned nightly.
crates="$(api_crates)"
[ -n "$crates" ] || { printf 'measure: no library crate to snapshot\n' >&2; exit 1; }
mkdir -p "$out/public-api"
rm -f "$out"/public-api/*.txt
for crate in $crates; do
  api_file "$crate" > "$out/public-api/$crate.txt"
done

# 5. Functions over clippy's default 100 lines, for #337's ratchet. The lint
# is pedantic, and an attribute could silence it, so it is force-warned.
{
  printf '# produced by quality/measure.sh with %s\n' "$(cargo clippy --version)"
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
      }'
} > "$out/too-many-lines.txt"

# 6. Suppressions by lint (#337). The suppressions test is their one reader:
# it holds the tree to this file on every run, and rewrites it here.
BROKKR_REGENERATE_SUPPRESSIONS=1 cargo test --locked -q -p brokkr-cli --test suppressions \
  every_suppression_in_the_tree_is_counted_in_the_baseline > /dev/null
