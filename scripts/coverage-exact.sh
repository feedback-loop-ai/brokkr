#!/usr/bin/env bash
set -euo pipefail

command -v jq >/dev/null 2>&1 || {
  printf '%s\n' 'coverage refusal: jq is required for exact integer verification' >&2
  exit 1
}

# Test harnesses live in cargo-llvm-cov's conventional `tests.rs`,
# `*_tests.rs`, and `tests/` paths. Nothing else leaves the denominator:
# attribute exclusions and `cfg(coverage)` switches are refused wherever
# they are written, whitespace, nesting and line breaks included, and a
# production target may not sit at a test path. The one way out is a
# package named in `coverage_exclusions` below, with its reason.
test_path_regex='(^|/)(tests\.rs|[^/]+_tests\.rs|tests/)'

# Untracked files are read too: a module compiles whether or not it is
# committed yet.
git ls-files -z --cached --others --exclude-standard -- 'crates/*.rs' 'crates/**/*.rs' |
  xargs -0 perl -0777 -ne '
  my $source = $_;
  my $line = sub { 1 + (substr($source, 0, $_[0]) =~ tr/\n//) };
  # Every cfg predicate, as a balanced parenthesis group, in all three
  # spellings: #[cfg(..)], #[cfg_attr(..)] and cfg!(..).
  while ($source =~ /\bcfg(?:_attr)?\s*(?:!\s*)?(\((?:[^()]++|(?1))*\))/g) {
    my ($at, $predicate) = ($-[0], $1);
    next unless $predicate =~ /\bcoverage(?:_nightly)?\b/;
    printf "%s:%d: a cfg predicate names coverage\n", $ARGV, $line->($at);
    $refused = 1;
  }
  while ($source =~ /#\s*!?\s*\[\s*coverage\s*\(|\bcoverage\s*\(\s*off\s*\)/g) {
    printf "%s:%d: a coverage attribute\n", $ARGV, $line->($-[0]);
    $refused = 1;
  }
  END { exit($refused ? 1 : 0) }
' || {
  printf '%s\n' 'coverage refusal: attribute and cfg(coverage) source exclusions are forbidden' >&2
  exit 1
}

# The single named exclusion. Its sources stay out of the report, and its
# tests still run, so the production code they reach is still measured.
# An entry must name a workspace package that is never published.
coverage_exclusions=(
  # The seatbelt probe's helper. Its Gate B roles (payload, detaching
  # descendants, guard, supervisor) run only inside a macOS `sandbox-exec`
  # cell under launchd, so no Linux run can reach most of it: 12 of its 69
  # functions ran here on 2026-09-26 (decision 0046 slice II; issue #341).
  brokkr-seatbelt-probe
)

metadata="$(cargo metadata --format-version 1 --no-deps --locked)"
exclusions_json="$(printf '%s\n' "${coverage_exclusions[@]}" | jq -R . | jq -s .)"
escapes="$(jq -r --argjson excluded "$exclusions_json" --arg tests "$test_path_regex" '
  (.workspace_root + "/") as $root
  | [.packages[] | {name, publish}] as $packages
  | ($excluded[] as $name
      | ($packages | map(select(.name == $name)) | first) as $package
      | if $package == null then "\($name): named in coverage_exclusions but not a workspace package"
        elif $package.publish != [] then "\($name): excluded from coverage but publishable"
        else empty end),
    (.packages[] | .name as $name | select($excluded | any(. == $name) | not)
      | .targets[]
      | select(.kind | any(. == "test" or . == "bench" or . == "example") | not)
      | (.src_path | ltrimstr($root)) as $path
      | select($path | test($tests))
      | "\($name): \(.kind | join(",")) target \(.name) sits at test path \($path)")
' <<<"$metadata")"
if [[ -n "$escapes" ]]; then
  printf '%s\n' "$escapes" >&2
  printf '%s\n' 'coverage refusal: a production target escapes the denominator, or an exclusion is not what it claims' >&2
  exit 1
fi

forge_coverage_dir="$(mktemp -d "${TMPDIR:-/tmp}/forge-coverage.XXXXXX")"
trap 'rm -rf "$forge_coverage_dir"' EXIT
mkdir -p target/coverage

# The toolchain is the pin's, not "whatever nightly is current": a nightly
# release changes the instrumented set, so an unpinned compiler makes this
# gate and a developer's local run disagree on identical bytes. One file,
# read here and by the workflow (issue #235).
nightly="$(tr -d '[:space:]' < "$(dirname "$0")/../rust-nightly-version.txt")"

# A warm instrumented target (issue #341). The dependency build survives
# between runs under a key naming everything that shapes it: the pinned
# nightly, the lockfile and the workspace's target set. `llvm-cov clean
# --workspace` below still drops every workspace artifact and profile, so
# the report stays bound to this candidate. One run at a time holds the
# cache, because two runs sharing a target would merge each other's
# profiles; the lock is the kernel's, so a killed run leaves none behind.
sha256() {
  if command -v sha256sum >/dev/null 2>&1; then sha256sum; else shasum -a 256; fi | cut -c1-16
}
cache_root="${BROKKR_COVERAGE_CACHE:-${XDG_CACHE_HOME:-$HOME/.cache}/brokkr-coverage}"
targets="$(jq -c '(.workspace_root + "/") as $root
  | [.packages[] | {name, targets: [.targets[] | {name, kind, path: (.src_path | ltrimstr($root))}]}]' <<<"$metadata")"
key="$nightly-$(sha256 < Cargo.lock)-$(printf '%s' "$targets" | sha256)"
mkdir -p "$cache_root"
exec 9>"$cache_root/.lock"
if command -v flock >/dev/null 2>&1; then
  flock 9
else
  perl -MFcntl=:flock -e 'open(my $lock, ">&=", 9) or die "coverage cache lock: $!\n"; flock($lock, LOCK_EX) or die "coverage cache lock: $!\n"'
fi
for stale in "$cache_root"/*; do
  if [[ -e "$stale" && "$(basename "$stale")" != "$key" ]]; then rm -rf "$stale"; fi
done
export CARGO_LLVM_COV_TARGET_DIR="$cache_root/$key/target"

# A report is candidate-bound only when no instrumented executable or profile
# from an earlier source graph can participate in the merge.
cargo "+$nightly" llvm-cov clean --workspace

# The named packages leave the report by their directories, which cargo
# metadata resolves, so the test run and the LCOV report drop exactly the
# same files. cargo-llvm-cov keeps its own test-file ignores beside this.
excluded_dirs="$(jq -r --argjson excluded "$exclusions_json" '
  [.packages[] | .name as $name | select($excluded | any(. == $name))
   | .manifest_path | sub("Cargo\\.toml$"; "")
   | "^" + gsub("(?<c>[.^$*+?()\\[\\]{}|\\\\])"; "\\\\\(.c)")]
  | join("|")' <<<"$metadata")"
exclusion_flags=()
if [[ -n "$excluded_dirs" ]]; then
  exclusion_flags=(--ignore-filename-regex "$excluded_dirs")
fi

cargo "+$nightly" llvm-cov \
  --workspace \
  "${exclusion_flags[@]}" \
  --all-features \
  --locked \
  --branch \
  --json \
  --output-path "$forge_coverage_dir/coverage.json"

# Preserve the complete report before evaluating the threshold. A red exact
# gate must still leave operators enough evidence to see and burn down every
# missing region instead of returning only an opaque non-zero exit.
cp "$forge_coverage_dir/coverage.json" target/coverage/coverage-exact.json
cargo "+$nightly" llvm-cov report "${exclusion_flags[@]}" --branch --lcov --output-path target/coverage/lcov.info

jq -e --arg tests "$test_path_regex" '
  [.data[0].files[].filename | select(test($tests))] | length == 0
' target/coverage/coverage-exact.json >/dev/null || {
  printf '%s\n' 'coverage refusal: test harness source leaked into the production report' >&2
  exit 1
}

# LLVM's JSON summary treats distinct compiler instantiations of the same
# source line as separate lines. The ratified contract is source coverage, so
# evaluate the canonical LCOV records: every DA and BRDA record must be hit,
# and every logical function counted by LLVM must be hit. This remains literal
# integer equality, not a rounded percentage threshold. `#[derive]` output
# never reaches these records: rustc does not instrument impls marked
# `#[automatically_derived]`, so a derive costs no line, branch or function.
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
