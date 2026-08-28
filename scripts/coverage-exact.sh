#!/usr/bin/env bash
set -euo pipefail

command -v jq >/dev/null 2>&1 || {
  printf '%s\n' 'coverage refusal: jq is required for exact integer verification' >&2
  exit 1
}

# Test harnesses live in cargo-llvm-cov's conventional `tests.rs`,
# `*_tests.rs`, and `tests/` paths. Attribute-based exclusions are forbidden
# everywhere so production code cannot silently shrink the denominator.
if git grep -n 'coverage(off)' -- 'crates/*.rs' 'crates/**/*.rs'; then
  printf '%s\n' 'coverage refusal: attribute-based source exclusions are forbidden' >&2
  exit 1
fi

forge_coverage_dir="$(mktemp -d "${TMPDIR:-/tmp}/forge-coverage.XXXXXX")"
trap 'rm -rf "$forge_coverage_dir"' EXIT
mkdir -p target/coverage

# A unique target directory is stronger than cleaning shared coverage state:
# no stale instrumented executable can participate in this candidate's merge.
export CARGO_LLVM_COV_TARGET_DIR="$forge_coverage_dir/target"

# A report is candidate-bound only when no instrumented executable or profile
# from an earlier source graph can participate in the merge.
cargo +nightly llvm-cov clean --workspace

cargo +nightly llvm-cov \
  --workspace \
  --all-features \
  --locked \
  --branch \
  --json \
  --output-path "$forge_coverage_dir/coverage.json"

# Preserve the complete report before evaluating the threshold. A red exact
# gate must still leave operators enough evidence to see and burn down every
# missing region instead of returning only an opaque non-zero exit.
cp "$forge_coverage_dir/coverage.json" target/coverage/coverage-exact.json
cargo +nightly llvm-cov report --branch --lcov --output-path target/coverage/lcov.info

jq -e '
  [.data[0].files[].filename |
    select(test("(^|/)(tests\\.rs|[^/]+_tests\\.rs|tests/)") )
  ] | length == 0
' target/coverage/coverage-exact.json >/dev/null || {
  printf '%s\n' 'coverage refusal: test harness source leaked into the production report' >&2
  exit 1
}

# LLVM's JSON summary treats distinct compiler instantiations of the same
# source line as separate lines. The ratified contract is source coverage, so
# evaluate the canonical LCOV records: every DA and BRDA record must be hit,
# and every logical function counted by LLVM must be hit. This remains literal
# integer equality, not a rounded percentage threshold.
read -r line_count line_covered branch_count branch_covered function_count function_covered < <(
  awk '
    /^DA:/ {
      record = substr($0, 4);
      split(record, line_fields, ",");
      line_count += 1;
      if (line_fields[2] + 0 > 0) line_covered += 1;
    }
    /^BRDA:/ {
      record = substr($0, 6);
      split(record, branch_fields, ",");
      branch_count += 1;
      if (branch_fields[4] != "-" && branch_fields[4] + 0 > 0) branch_covered += 1;
    }
    /^SF:/ { source_file = substr($0, 4); }
    /^FN:/ {
      record = substr($0, 4);
      split(record, function_fields, ",");
      name = record;
      sub(/^[^,]*,/, "", name);
      function_start[source_file SUBSEP name] = function_fields[1];
    }
    /^FNDA:/ {
      record = substr($0, 6);
      split(record, function_fields, ",");
      name = record;
      sub(/^[^,]*,/, "", name);
      function_hits[source_file SUBSEP name] += function_fields[1] + 0;
    }
    END {
      # Rust crate hashes and generic call-site types create multiple symbols
      # for one source-defined function. Its stable identity is file + start
      # line; any positive compiled instance covers that source function.
      for (symbol in function_start) {
        split(symbol, parts, SUBSEP);
        source_function = parts[1] SUBSEP function_start[symbol];
        functions[source_function] = 1;
        if (function_hits[symbol] > 0) function_is_covered[source_function] = 1;
      }
      for (source_function in functions) {
        function_count += 1;
        if (function_is_covered[source_function]) function_covered += 1;
      }
      print line_count + 0, line_covered + 0,
            branch_count + 0, branch_covered + 0,
            function_count + 0, function_covered + 0;
    }
  ' target/coverage/lcov.info
)

jq -n \
  --argjson line_count "$line_count" \
  --argjson line_covered "$line_covered" \
  --argjson branch_count "$branch_count" \
  --argjson branch_covered "$branch_covered" \
  --argjson function_count "$function_count" \
  --argjson function_covered "$function_covered" \
  '{
    lines: {count: $line_count, covered: $line_covered},
    branches: {count: $branch_count, covered: $branch_covered},
    functions: {count: $function_count, covered: $function_covered}
  }' >target/coverage/coverage-summary.json

if (( line_count == 0 || line_covered != line_count ||
      branch_count == 0 || branch_covered != branch_count ||
      function_count == 0 || function_covered != function_count )); then
  jq . target/coverage/coverage-summary.json >&2
  printf '%s\n' 'coverage refusal: literal nonzero 100% source-line/branch/function equality not met' >&2
  exit 1
fi

jq . target/coverage/coverage-summary.json
