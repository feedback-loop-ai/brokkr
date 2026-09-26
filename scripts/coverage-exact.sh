#!/usr/bin/env bash
set -euo pipefail

refuse() {
  printf 'coverage refusal: %s\n' "$1" >&2
  exit 1
}

for tool in jq perl; do
  command -v "$tool" >/dev/null 2>&1 || refuse "$tool is required for exact integer verification"
done

cd "$(git rev-parse --show-toplevel)"
metadata="$(cargo metadata --format-version 1 --no-deps --locked)"
root="$(jq -r .workspace_root <<<"$metadata")"

# The one test-path vocabulary. A file under a `tests/`, `examples/` or
# `benches/` directory, or named `tests.rs` or `*_tests.rs` or `*-tests.rs`,
# is test harness: the report leaves it out, and no production target or
# module may live there. Everything else under `crates/` is counted. The
# gate hands cargo-llvm-cov exactly this vocabulary and switches the tool's
# own default off, so the two can never disagree about what is a test.
test_dirs='tests|examples|benches'
test_files='tests\.rs|[^/]*[_-]tests\.rs'
# The same vocabulary over a path relative to the workspace root.
test_path_regex="(^|/)(${test_dirs})/|(^|/)(${test_files})\$"

# Regex escaping, once, for jq and for the anchors handed to llvm-cov. It
# is checked on every run: an escape that doubles its backslash matches a
# literal backslash instead of the character it meant to protect.
jq_defs='def re_escape: gsub("(?<c>[.^$*+?()\\[\\]{}|\\\\])"; "\\\(.c)");'
[[ "$(jq -rn "$jq_defs"' "a.b(c)[d]{e}|f^g$h*i+j?k\\l" | re_escape')" == 'a\.b\(c\)\[d\]\{e\}\|f\^g\$h\*i\+j\?k\\l' ]] ||
  refuse 're_escape does not escape each regex metacharacter with exactly one backslash'
re_escape() { jq -rn --arg path "$1" "$jq_defs"' $path | re_escape'; }
root_re="$(re_escape "$root")"

# Attribute exclusions and `cfg(coverage)` switches are refused in source,
# read as tokens: comments, strings, raw strings, byte strings and char
# literals are blanked first, so no comment or literal can hide or fake one.
# A lint level on `unexpected_cfgs`, or an allow of every warning, is refused
# too: it would silence the compiler's own refusal of an undeclared cfg,
# which the production check below relies on.
git ls-files -z --cached --others --exclude-standard -- 'crates/*.rs' 'crates/**/*.rs' |
  xargs -0 perl -CSD -0777 -ne '
  sub blank { my $text = shift; $text =~ s/[^\n]/ /g; $text }
  sub code_of {
    my ($source) = @_;
    my $code = "";
    pos($source) = 0;
    while (pos($source) < length $source) {
      if ($source =~ /\G(\/\/[^\n]*)/gc) { $code .= blank($1); next }
      if ($source =~ /\G\/\*/gc) {
        my ($start, $depth) = (pos($source) - 2, 1);
        while ($depth > 0) {
          if    ($source =~ /\G\/\*/gc) { $depth++ }
          elsif ($source =~ /\G\*\//gc) { $depth-- }
          elsif ($source =~ /\G(?:[^\/*]+|[\/*])/gc) { }
          else  { last }
        }
        $code .= blank(substr($source, $start, pos($source) - $start));
        next;
      }
      if ($source =~ /\G((?<!\w)[bc]?r(#*)".*?"\2)/gcs) { $code .= blank($1); next }
      if ($source =~ /\G((?<!\w)[bc]?"(?:[^"\\]|\\.)*")/gcs) { $code .= blank($1); next }
      if ($source =~ /\G((?<!\w)b?\x27(?:[^\x27\\\n]|\\(?:[nrt\\0\x27"]|x[0-9A-Fa-f]{2}|u\{[0-9A-Fa-f]{1,6}\}))\x27)/gc) {
        $code .= blank($1);
        next;
      }
      $source =~ /\G(\w+|.)/gcs;
      $code .= $1;
    }
    $code;
  }
  my $code = code_of($_);
  my $line = sub { 1 + (substr($code, 0, $_[0]) =~ tr/\n//) };
  my $refuse = sub { printf "%s:%d: %s\n", $ARGV, $line->($_[0]), $_[1]; $refused = 1 };
  # Every cfg predicate, as a balanced parenthesis group, in all three
  # spellings: #[cfg(..)], #[cfg_attr(..)] and cfg!(..).
  while ($code =~ /\bcfg(?:_attr)?\s*(?:!\s*)?(\((?:[^()]++|(?1))*\))/g) {
    my ($at, $predicate) = ($-[0], $1);
    $refuse->($at, "a cfg predicate names coverage") if $predicate =~ /\bcoverage(?:_nightly)?\b/;
  }
  while ($code =~ /#\s*!?\s*\[\s*coverage\s*\(/g) { $refuse->($-[0], "a coverage attribute") }
  while ($code =~ /\bcoverage_attribute\b/g) { $refuse->($-[0], "the coverage_attribute feature") }
  while ($code =~ /\bunexpected_cfgs\b/g) { $refuse->($-[0], "a lint level on unexpected_cfgs") }
  while ($code =~ /\b(?:allow|expect)\s*(\((?:[^()]++|(?1))*\))/g) {
    my ($at, $lints) = ($-[0], $1);
    $refuse->($at, "an allow of every warning") if $lints =~ /\bwarnings\b/;
  }
  END { exit($refused ? 1 : 0) }
' || refuse 'attribute and cfg(coverage) source exclusions are forbidden'

# A cfg is declared, and so escapes the compiler check below, only through
# a manifest's check-cfg list or a build script. The one lint entry the
# workspace may carry is the root's plain `unexpected_cfgs = "warn"`, and
# no manifest may spell a key through a TOML escape.
manifests=(Cargo.toml crates/*/Cargo.toml)
perl -ne '
  if (/check-cfg|\\[uU]/) { print "$ARGV:$.: a check-cfg list or a TOML escape\n"; $refused = 1 }
  if (/unexpected_cfgs/ && !($ARGV eq "Cargo.toml" && /^unexpected_cfgs = "warn"$/)) {
    print "$ARGV:$.: a lint level on unexpected_cfgs\n"; $refused = 1;
  }
  close ARGV if eof;
  END { exit($refused ? 1 : 0) }
' "${manifests[@]}" >&2 || refuse 'a manifest declares a cfg or changes the unexpected_cfgs lint'

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
# `${a[@]+"${a[@]}"}` expands an empty list to nothing under `set -u`, on
# bash 3.2 as well.
exclusions_json="$(jq -n '$ARGS.positional' --args ${coverage_exclusions[@]+"${coverage_exclusions[@]}"})"
excluded_manifests="$(jq -c --argjson excluded "$exclusions_json" '
  [.packages[] | select(.name as $name | $excluded | any(. == $name)) | .manifest_path]' <<<"$metadata")"
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
      | if (.kind | any(. == "custom-build"))
        then "\($name): a build script, which could declare a cfg the compiler check then accepts"
        else select(.kind | any(. == "test" or . == "bench" or . == "example") | not)
          | (.src_path | ltrimstr($root)) as $path
          | select($path | test($tests))
          | "\($name): \(.kind | join(",")) target \(.name) sits at test path \($path)"
        end)
' <<<"$metadata")"
if [[ -n "$escapes" ]]; then
  printf '%s\n' "$escapes" >&2
  refuse 'a production target escapes the denominator, or an exclusion is not what it claims'
fi

forge_coverage_dir="$(mktemp -d "${TMPDIR:-/tmp}/forge-coverage.XXXXXX")"
trap 'rm -rf "$forge_coverage_dir"' EXIT
mkdir -p target/coverage

# The toolchain is the pin's, not "whatever nightly is current": a nightly
# release changes the instrumented set, so an unpinned compiler makes this
# gate and a developer's local run disagree on identical bytes. One file,
# read here and by the workflow (issue #235).
nightly="$(tr -d '[:space:]' < "$(dirname "$0")/../rust-nightly-version.txt")"
[[ "$nightly" =~ ^nightly-[0-9]{4}-[0-9]{2}-[0-9]{2}$ ]] ||
  refuse "the coverage pin '$nightly' is not nightly-YYYY-MM-DD"

# A warm instrumented target (issue #341). The dependency build survives
# between runs in a directory the script owns, under a key naming everything
# that shapes it: the pinned nightly, the lockfile and the workspace's target
# set. `llvm-cov clean --workspace` below still drops every workspace
# artifact and profile, so the report stays bound to this candidate. Only
# directories with the key's exact shape are ever pruned, and only inside
# that directory, wherever BROKKR_COVERAGE_CACHE points.
sha256() {
  if command -v sha256sum >/dev/null 2>&1; then sha256sum; else shasum -a 256; fi | cut -c1-16
}
cache_root="${BROKKR_COVERAGE_CACHE:-${XDG_CACHE_HOME:-$HOME/.cache}}"
cache_dir="$cache_root/brokkr-coverage-cache"
key_shape='^nightly-[0-9]{4}-[0-9]{2}-[0-9]{2}-[0-9a-f]{16}-[0-9a-f]{16}$'
targets="$(jq -c '(.workspace_root + "/") as $root
  | [.packages[] | {name, targets: [.targets[] | {name, kind, path: (.src_path | ltrimstr($root))}]}]' <<<"$metadata")"
key="$nightly-$(sha256 < Cargo.lock)-$(printf '%s' "$targets" | sha256)"
[[ "$key" =~ $key_shape ]] || refuse "the cache key '$key' does not have the key's shape"
mkdir -p "$cache_dir"

# One run holds the cache at a time, because two runs sharing a target would
# merge each other's profiles. The lock is the kernel's, so a killed run
# leaves none behind; no child inherits it (9>&-), so a lingering test
# process cannot hold it past this run.
exec 9>"$cache_dir/.lock"
lock_wait=3600
if command -v flock >/dev/null 2>&1; then
  if ! flock -n 9; then
    printf 'coverage: waiting up to %ss for another run to release %s\n' "$lock_wait" "$cache_dir/.lock" >&2
    flock -w "$lock_wait" 9 || refuse "another run held $cache_dir/.lock for ${lock_wait}s"
  fi
else
  perl -MFcntl=:flock -e '
    open(my $lock, ">&=", 9) or die "coverage cache lock: $!\n";
    exit 0 if flock($lock, LOCK_EX | LOCK_NB);
    printf STDERR "coverage: waiting up to %ss for another run to release %s\n", @ARGV;
    local $SIG{ALRM} = sub { exit 1 };
    alarm $ARGV[0];
    flock($lock, LOCK_EX) or exit 1;
  ' "$lock_wait" "$cache_dir/.lock" || refuse "another run held $cache_dir/.lock for ${lock_wait}s"
fi
for stale in "$cache_dir"/*; do
  name="$(basename "$stale")"
  if [[ -d "$stale" && "$name" =~ $key_shape && "$name" != "$key" ]]; then rm -rf "$stale"; fi
done
export CARGO_LLVM_COV_TARGET_DIR="$cache_dir/$key/target"

# Every production target, checked by the compiler with every undeclared cfg
# forbidden: `coverage` is declared nowhere, so a cfg that names it fails
# here however it was written, macro expansion included. The same build's
# dep-info lists every source file each production target compiles, and none
# may sit at a test path, where the report would drop it.
check_messages="$forge_coverage_dir/check.json"
if ! CARGO_ENCODED_RUSTFLAGS='-Funexpected_cfgs' CARGO_TARGET_DIR="$cache_dir/$key/check" \
  cargo "+$nightly" check --workspace --all-features --locked --lib --bins \
  --message-format=json 9>&- >"$check_messages"; then
  jq -r 'select(.reason == "compiler-message" and .message.level == "error") | .message.rendered' \
    "$check_messages" >&2
  refuse 'a production target does not compile with every undeclared cfg forbidden'
fi
depinfo="$(jq -r --arg root "$root/" --argjson excluded "$excluded_manifests" '
  select(.reason == "compiler-artifact" and (.manifest_path | startswith($root))
         and (.profile.test | not))
  | select(.manifest_path as $manifest | $excluded | any(. == $manifest) | not)
  | .filenames[] | select(endswith(".rmeta"))
  | sub("/lib(?<stem>[^/]*)\\.rmeta$"; "/\(.stem).d")' "$check_messages")"
[[ -n "$depinfo" ]] || refuse 'the production check reported no dep-info to read'
while IFS= read -r file; do
  [[ -f "$file" ]] || refuse "dep-info $file is missing"
done <<<"$depinfo"
dropped="$(printf '%s\n' "$depinfo" | tr '\n' '\0' | xargs -0 perl -ne '
  next unless /^((?:[^:\\\s]|\\.)+):\s*$/;
  (my $path = $1) =~ s/\\(.)/$1/g;
  print "$path\n";
' | sed "s#^$root/##" | sort -u | grep -E "$test_path_regex" || true)"
if [[ -n "$dropped" ]]; then
  printf 'a production target compiles %s, which the report drops as a test path\n' $dropped >&2
  refuse 'a production module sits at a test path'
fi

# A report is candidate-bound only when no instrumented executable or profile
# from an earlier source graph can participate in the merge.
cargo "+$nightly" llvm-cov clean --workspace 9>&-

# What the report leaves out, stated whole: the test vocabulary under the
# workspace root, the named exclusions, and sources that are not this
# workspace's (the standard library, the registry and git checkouts, the
# toolchains and the build output). The tool's own default is off.
ignore=(
  "^$root_re/(.*/)?($test_dirs)/"
  "^$root_re/(.*/)?($test_files)\$"
  '/rustc/([0-9a-f]+|[0-9]+\.[0-9]+\.[0-9]+)/'
  "^$(re_escape "${CARGO_HOME:-$HOME/.cargo}")/(registry|git)/"
  "^$(re_escape "${RUSTUP_HOME:-$HOME/.rustup}")/toolchains/"
  "^$(re_escape "$CARGO_LLVM_COV_TARGET_DIR")/"
)
while IFS= read -r directory; do
  ignore+=("^$(re_escape "$directory")/")
done < <(jq -r '.[] | sub("/Cargo\\.toml$"; "")' <<<"$excluded_manifests")
ignore_regex="$(IFS='|'; printf '%s' "${ignore[*]}")"
report_flags=(--no-default-ignore-filename-regex --ignore-filename-regex "$ignore_regex")

cargo "+$nightly" llvm-cov \
  --workspace \
  "${report_flags[@]}" \
  --all-features \
  --locked \
  --branch \
  --json \
  --output-path "$forge_coverage_dir/coverage.json" 9>&-

# Preserve the complete report before evaluating the threshold. A red exact
# gate must still leave operators enough evidence to see and burn down every
# missing region instead of returning only an opaque non-zero exit.
cp "$forge_coverage_dir/coverage.json" target/coverage/coverage-exact.json
cargo "+$nightly" llvm-cov report "${report_flags[@]}" --branch --lcov \
  --output-path target/coverage/lcov.info 9>&-

# Every file the report counts is a production source of this workspace: a
# test file, or a source from anywhere else, is refused by name.
foreign="$(jq -r --arg crates "$root/crates/" --arg tests "$test_path_regex" --arg root "$root/" '
  .data[0].files[].filename
  | select((startswith($crates) | not) or (ltrimstr($root) | test($tests)))' \
  target/coverage/coverage-exact.json)"
if [[ -n "$foreign" ]]; then
  printf '%s\n' "$foreign" >&2
  refuse 'the report counts a file that is not a production source of this workspace'
fi

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
