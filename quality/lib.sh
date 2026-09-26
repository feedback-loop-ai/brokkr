# shellcheck shell=bash
# One home for every measuring command (decision 0071 ruling 5). Sourced by
# quality/measure.sh, which writes the baselines, and by quality/ratchet.sh,
# which holds the tree to them, so a ratchet measures exactly as its baseline
# was measured.

# The exact gate's test-path vocabulary, read from its one home
# (scripts/coverage-exact.sh's test_dirs and test_files) rather than copied.
# A test path lies under a tests/, examples/ or benches/ directory, or is
# named tests.rs, *_tests.rs or *-tests.rs.
gate_word() {
  sed -n "s/^$1='\\(.*\\)'\$/\\1/p" scripts/coverage-exact.sh
}
test_dirs="$(gate_word test_dirs)"
test_files="$(gate_word test_files)"
if [ -z "$test_dirs" ] || [ -z "$test_files" ]; then
  printf 'quality: scripts/coverage-exact.sh declares no test_dirs or test_files\n' >&2
  exit 1
fi
test_re="(^|/)(${test_dirs})/|(^|/)(${test_files})\$"
# jscpd and cargo-crap take globs, not a regex. The directory globs follow
# test_dirs; the file globs are written for the gate's file vocabulary as it
# reads today, so a change to it stops here until they are rewritten.
if [ "$test_files" != 'tests\.rs|[^/]*[_-]tests\.rs' ]; then
  printf 'quality: the gate'"'"'s test_files changed (%s); rewrite lib.sh'"'"'s globs\n' "$test_files" >&2
  exit 1
fi
test_file_globs=(tests.rs '*_tests.rs' '*-tests.rs')
IFS='|' read -r -a test_dir_names <<< "$test_dirs"
test_path_globs=()   # every test path, for --ignore and --exclude
test_source_globs=() # every test source, for jscpd's --pattern
for dir in "${test_dir_names[@]}"; do
  test_path_globs+=("**/$dir/**")
  test_source_globs+=("$dir/**/*.rs")
done
for name in "${test_file_globs[@]}"; do
  test_path_globs+=("**/$name")
  test_source_globs+=("$name")
done
test_ignore="$(IFS=,; printf '%s' "${test_path_globs[*]}")"
test_glob="**/{$(IFS=,; printf '%s' "${test_source_globs[*]}")}"

# Left out of the data scan, and why:
# - contracts/, reference/ and fixtures/ are deliberately frozen copies;
# - their in-crate twins are pinned byte for byte to them by tests: the
#   embedded seat-record schemas (seat_record.rs, against contracts/) and the
#   dialect library brokkr-cli scaffolds (tests/packaging.rs, against dialects/);
# - openspec/changes/archive/ and docs/evidence/ are append-only history whose
#   text the specs and features repeat by design (decision 0042's archive step);
# - .github/lint/ holds the lint tools' generated npm lockfile;
# - quality/ is this directory, and target/ and .forge/ are build and run
#   output. jscpd also honours .gitignore, so the scan reads the tree as git
#   sees it; measure.sh and ratchet.sh are run on a clean checkout.
data_ignore='contracts/**,reference/**,fixtures/**,quality/**,target/**,.forge/**'
data_ignore+=',crates/brokkr-store/src/seat-record.v*.schema.json,crates/brokkr-cli/dialects/**'
data_ignore+=',openspec/changes/archive/**,docs/evidence/**,.github/lint/**'

# jscpd 5.3.2 has no default --max-lines cap, but any explicit value caps
# whole files, not clone blocks: -x 1000 would skip every file longer than
# that. The high explicit cap keeps a future default from doing the same.
jscpd_flags=(--min-tokens 50 --min-lines 5 --max-lines 100000 --max-size 10mb
  --mode mild --silent --no-tips --reporters json)

# jscpd_scope <prod|tests|data> <report-dir> [jscpd flags...]
jscpd_scope() {
  local scope="$1" report="$2"
  shift 2
  case "$scope" in
    prod) jscpd "${jscpd_flags[@]}" --output "$report" "$@" --format rust --ignore "$test_ignore" crates ;;
    tests) jscpd "${jscpd_flags[@]}" --output "$report" "$@" --format rust --pattern "$test_glob" crates ;;
    data) jscpd "${jscpd_flags[@]}" --output "$report" "$@" --format json,markdown --ignore "$data_ignore" . ;;
    *)
      printf 'quality: unknown jscpd scope %s\n' "$scope" >&2
      return 1
      ;;
  esac
}

# The exact gate's named coverage exclusions, read from its own list
# (coverage_exclusions in scripts/coverage-exact.sh), so they have one home.
# Should that list's shape change and this read nothing, the excluded crate
# is scored as uncovered and the complexity ratchet refuses: it fails closed.
coverage_exclusions() {
  awk '/^coverage_exclusions=\(/ { on = 1; next }
       on && /^\)/ { exit }
       on && $1 !~ /^#/ && NF { print $1 }' scripts/coverage-exact.sh
}

# crap_report <lcov> <out.json> [cargo-crap flags...]
# At the gate's 100% coverage CRAP equals cyclomatic complexity. Test files
# are not in the LCOV and would score as 0% covered, so every test path is
# excluded, and so is each package the gate excludes by name. `--path .`
# records repository-relative paths.
crap_report() {
  local lcov="$1" out="$2" name glob
  local excluded=()
  shift 2
  for name in $(coverage_exclusions); do excluded+=(--exclude "crates/$name/**"); done
  for glob in "${test_path_globs[@]}"; do excluded+=(--exclude "$glob"); done
  cargo crap --path . --lcov "$lcov" --exclude 'target/**' \
    ${excluded[@]+"${excluded[@]}"} \
    --sort file --format json --output "$out" "$@"
}

# The library crates whose public API is snapshotted, read from cargo metadata.
api_crates() {
  cargo metadata --format-version 1 --no-deps --locked |
    jq -r '.packages[] | select(any(.targets[]; .kind | index("lib"))) | .name' | sort
}

# api_file <crate>: a snapshot's exact committed bytes. A provenance line,
# then the crate's public API on the pinned nightly, with blanket,
# auto-trait and auto-derived impls omitted.
api_file() {
  local nightly
  nightly="$(cat rust-nightly-version.txt)"
  printf '# produced by quality/measure.sh with %s on %s, -sss\n' "$(cargo public-api --version)" "$nightly"
  cargo "+$nightly" public-api -p "$1" -sss --color never
}

# Lines per Rust file, production and test apart, sorted by path.
rs_lines() { while IFS= read -r file; do printf '%6d %s\n' "$(wc -l < "$file")" "$file"; done; }
file_lines() {
  printf '# production\n'
  git ls-files '*.rs' | { grep -v -E "$test_re" || true; } | sort | rs_lines
  printf '# test\n'
  git ls-files '*.rs' | { grep -E "$test_re" || true; } | sort | rs_lines
}
