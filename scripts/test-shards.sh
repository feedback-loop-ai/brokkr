#!/usr/bin/env bash
# The workspace suite as shards (#633). This script runs the tests `cargo
# test --workspace` runs, each exactly once, as shards run one after
# another in plan order, each with its own output and its own exit status.
# With --parallel-heaps, only the three single-test heap binaries overlap;
# the other shards still run alone. Broader parallelism cost deadline-bound
# tests their margins on hosted runners (operator ruling 2026-10-11).
# The run fails when any shard fails, and only after every shard has run.
#
# The test jobs spread the shards over machines (operator ruling
# 2026-11-01): each shard names its machine in each system's layout, and a
# run whose BROKKR_TEST_MACHINE names one, `linux/<n>` or `macos/<n>`, runs
# that machine's shards alone, one after another as above, so a shard still
# never shares its runner. Unset, every shard runs, as a local run does. The
# variable is read once and unset, so no test a shard runs inherits it. Each
# system's machines are numbered from 1 with none left empty, which is
# checked before anything runs.
#
#   scripts/test-shards.sh test [--parallel-heaps]     the test jobs' suite, doctests included
#   scripts/test-shards.sh coverage <nightly> [--parallel-heaps]   the exact coverage gate's instrumented run
#   scripts/test-shards.sh coverage-build <nightly>   that run's build alone, for the gate's report
#   scripts/test-shards.sh plan test|coverage   the partition check and each shard's command
#   scripts/test-shards.sh machines test|coverage   each machine and the shards it runs
#
# Every shard selects workspace-wide (`--workspace --all-features`), never by
# package: Cargo unifies features over the selected packages, so a shard of
# one package would build, and test, another binary than the suite does.
# capability_broker's real-box tests are deadline-bound by design and must
# not share the runner with another heavy binary, so they are their own
# shards, the last; the shard before runs `it` with them skipped. A libtest
# `--skip F` and the filter `F` select complementary tests, so that split
# runs each of `it`'s tests exactly once. The broker's tests split once more
# the same way: `only:F skip:G` and `only:G`, where G extends F, so a test
# whose name holds G holds F too, and each runs in exactly one of the three.
#
# Before anything runs, the shards are held to `cargo metadata`: every test
# target `cargo test --workspace` runs (every target with `test = true`,
# and in the test mode every library's doctests) is selected exactly once,
# or exactly twice by a filter and its skip. A target no shard selects, one
# selected twice, a selector that names no target, a shard that runs a
# target the suite does not, and a selector the plan cannot read are each
# refused by name. Every shard's output is ruled on as scripts/run-it-tests.sh
# rules on a run's: a shard that prints a `test result:` summary the
# grammar cannot read, or none at all, is refused, and so is a filtered
# shard whose run shows no executed test, since its filter matches nothing
# (#543). A shard whose runner ends without recording its exit status is
# refused by name, and the logs are then kept, since its command may still
# be writing them; for the same reason no later shard is started, since it
# could not be shown to run alone. A shard that merely fails stops nothing:
# every later shard still runs, as `--no-fail-fast` would. The threat model is
# the operator's of 2026-09-26: realistic accidental misuse is caught, and
# what cannot be read is refused.
set -euo pipefail

refuse() {
  printf 'test shards: %s\n' "$1" >&2
  exit 1
}

usage() {
  printf 'test shards: usage: %s test [--parallel-heaps] | coverage <nightly> [--parallel-heaps] | coverage-build <nightly> | plan test|coverage | machines test|coverage\n' "$0" >&2
  exit 2
}

# The shards, in the order they run, each with its machine on Linux and on
# macOS. A selector is `lib` (every library's unit tests), `bin:<name>`,
# `test:<name>` or `doc` (every library's doctests, alone in its shard);
# `skip:<filter>` and `only:<filter>` pass libtest a skip or a filter, and
# a shard may pass both, `only:F skip:G`, where G extends F.
# Hosted runners took about 11, 3, 1, 4, 13, 0 and 20 minutes (the
# broker's two together) for them on Linux, half as long again under
# coverage, and 9, 8, 3, 19 (the two heap shards together), 0 and 0 on
# macOS, where the broker's boundary proofs skip (2026-11-01). The broker's
# enabled suite is Linux's alone (`#![cfg(target_os = "linux")]`), so macOS
# never runs its shard, whose filter would match nothing there.
shard_name=()
shard_linux=()
shard_macos=()
shard_selectors=()
shard() {
  shard_name+=("$1")
  shard_linux+=("$2")
  shard_macos+=("$3")
  shift 3
  shard_selectors+=("$*")
}
shard 'every library' 1 1 lib
shard "the it binaries, capability_broker skipped" 2 1 test:it skip:capability_broker::
shard 'the heap measurements' 2 1 test:heap_claude test:heap_codex test:heap_dsh
shard 'the other own-process binaries' 2 2 bin:brokkr test:driver_conformance test:transcript_surfaces \
  test:hands_exits test:secret_drop test:capability_launch test:operated_repo test:queued_launch \
  test:seatbelt_lifetime_probe test:summary_heap test:heap_open test:heap_costs
shard 'the fleet, run and seat heap views' 3 2 test:heap_fleet test:heap_run_view test:heap_seat_view
shard 'the doctests' 2 1 doc
shard "capability_broker's enabled suite, alone" 4 - test:it only:capability_broker::enabled::
shard "capability_broker, its enabled suite skipped, alone" 5 1 test:it only:capability_broker:: skip:capability_broker::enabled::

# The machine this run is, read once and unset (above).
machine="${BROKKR_TEST_MACHINE:-}"
unset BROKKR_TEST_MACHINE

parallel_heaps=false
if [ "$#" -gt 0 ] && [ "${!#}" = --parallel-heaps ]; then
  parallel_heaps=true
  set -- "${@:1:$(($# - 1))}"
fi
mode="${1:-}"
case "$mode" in
  test)
    [ "$#" -eq 1 ] || usage
    prefix=(cargo test --workspace --all-features --locked --no-fail-fast)
    doctests=true
    ;;
  coverage | coverage-build)
    [ "$#" -eq 2 ] || usage
    [ "$mode" = coverage ] || ! "$parallel_heaps" || usage
    [[ "$2" =~ ^nightly-[0-9]{4}-[0-9]{2}-[0-9]{2}$ ]] || refuse "the coverage toolchain '$2' is not nightly-YYYY-MM-DD"
    # cargo-llvm-cov runs no doctests unless asked, and the gate never asks.
    # `--no-fail-fast` runs a shard's every binary past a failing one, as
    # the test mode does; the shard still exits non-zero.
    prefix=(cargo "+$2" llvm-cov --no-report --workspace --all-features --locked --branch --no-fail-fast)
    doctests=false
    ;;
  plan | machines)
    ! "$parallel_heaps" || usage
    [ "$#" -eq 2 ] || usage
    case "$2" in
      test) doctests=true ;;
      coverage) doctests=false ;;
      *) usage ;;
    esac
    prefix=()
    ;;
  *) usage ;;
esac

# The shards this mode runs: the coverage mode has no doctests to run.
planned=()
for index in "${!shard_name[@]}"; do
  if [ "${shard_selectors[$index]}" = doc ] && ! "$doctests"; then continue; fi
  planned+=("$index")
done

# The machine shard $2 runs on in system $1's layout.
machine_of() {
  case "$1" in
    linux) printf '%s' "${shard_linux[$2]}" ;;
    macos) printf '%s' "${shard_macos[$2]}" ;;
  esac
}
# Each system's layout over the shards this mode plans: every machine is a
# number from 1, or `-` for a shard whose tests that system never compiles,
# and the machines run from 1 with none left empty, so a machine the test
# job lists runs something and no shard waits on a machine no job is.
for system in linux macos; do
  highest=0
  used=' '
  for index in "${planned[@]}"; do
    number="$(machine_of "$system" "$index")"
    if [ "$number" = - ]; then continue; fi
    [[ "$number" =~ ^[1-9][0-9]?$ ]] ||
      refuse "${shard_name[$index]} names the $system machine '$number', which is not a number from 1"
    used+="$number "
    if [ "$number" -gt "$highest" ]; then highest="$number"; fi
  done
  for ((number = 1; number <= highest; number++)); do
    [[ "$used" == *" $number "* ]] || refuse "the $system layout leaves machine $number with no shard"
  done
done
# The shards this run runs: the named machine's, or every planned one this
# host's system compiles. The report's build is every machine's, so it
# names none.
if [ "$mode" = coverage-build ] && [ -n "$machine" ]; then
  refuse "coverage-build builds for the gate's report, and BROKKR_TEST_MACHINE names $machine"
fi
case "$(uname -s)" in
  Linux) host=linux ;;
  Darwin) host=macos ;;
  *) host='' ;;
esac
running=()
for index in "${planned[@]}"; do
  if [ -z "$host" ] || [ "$(machine_of "$host" "$index")" != - ]; then running+=("$index"); fi
done
if [ -n "$machine" ]; then
  [[ "$machine" =~ ^(linux|macos)/([1-9][0-9]?)$ ]] ||
    refuse "BROKKR_TEST_MACHINE '$machine' is not linux/<n> or macos/<n>"
  system="${BASH_REMATCH[1]}" number="${BASH_REMATCH[2]}"
  running=()
  for index in "${planned[@]}"; do
    if [ "$(machine_of "$system" "$index")" = "$number" ]; then running+=("$index"); fi
  done
  [ "${#running[@]}" -gt 0 ] || refuse "BROKKR_TEST_MACHINE names $machine, which runs no shard"
fi

for tool in jq cargo; do
  command -v "$tool" >/dev/null 2>&1 || refuse "$tool is required to hold the shards to the workspace"
done
if "$parallel_heaps"; then
  command -v python3 >/dev/null 2>&1 || refuse 'python3 is required for parallel heaps'
fi
cd "$(git rev-parse --show-toplevel)"

# The partition check, read from Cargo's own typed account of the targets.
plan_lines=()
for index in "${planned[@]}"; do
  plan_lines+=("${shard_name[$index]}	${shard_selectors[$index]}")
done
# jq evaluates its filter once for each JSON value it reads, so output that
# holds no value would pass every check below without reading a target:
# the read is refused unless it is exactly one object with a packages array.
metadata="$(cargo metadata --format-version 1 --no-deps --locked)" \
  || refuse 'cargo metadata could not be read'
jq -s -e 'length == 1 and (.[0] | type == "object" and (.packages | type) == "array")' \
  >/dev/null 2>&1 <<<"$metadata" || refuse 'cargo metadata could not be read'
problems="$(jq -r --argjson doctests "$doctests" '
  def kind: .kind as $kinds
    | if ($kinds | any(. == "lib" or . == "rlib" or . == "dylib" or . == "cdylib"
                         or . == "staticlib" or . == "proc-macro")) then "lib"
      elif ($kinds | any(. == "bin" or . == "test" or . == "example" or . == "bench"))
      then ($kinds | map(select(. == "bin" or . == "test" or . == "example" or . == "bench")) | first)
      else ($kinds | join(",")) end;
  [.packages[] | .name as $package | .targets[]
    | {package: $package, kind: kind, name, test, doctest}] as $targets
  | ([$targets[] | select(.test) | "\(.package) \(.kind) \(.name)"]
     + if $doctests then [$targets[] | select(.kind == "lib" and .doctest) | "\(.package) doc \(.name)"]
       else [] end) as $suite
  | [$ARGS.positional[] | split("\t") | {shard: .[0], words: (.[1] | split(" "))}] as $shards
  | [$shards[] | .shard as $shard
      | ([.words[] | select(startswith("skip:") or startswith("only:"))]) as $filters
      | (if ($filters | length) < 2 then empty
         elif ($filters | length) == 2 and ($filters[0] | startswith("only:"))
              and ($filters[1] | startswith("skip:"))
              and (($filters[1] | ltrimstr("skip:")) as $narrower
                   | ($filters[0] | ltrimstr("only:")) as $wider
                   | ($narrower | startswith($wider)) and $narrower != $wider)
         then empty
         else "\($shard): more than one filter: \($filters | join(" "))" end),
        (.words[] | select((startswith("skip:") or startswith("only:")) | not) as $word
          | ($word | capture("^(?<kind>bin|test):(?<name>[A-Za-z0-9_-]+)$") // null) as $named
          | if $word == "lib" or $word == "doc" or $named != null then empty
            else "\($shard): a selector the plan cannot read: \($word)" end)]
    as $unread
  | [$shards[] | .shard as $shard
      | ([.words[] | select(startswith("skip:") or startswith("only:"))] | join(" ")
         | if . == "" then "none" else . end) as $filter
      | .words[] | select((startswith("skip:") or startswith("only:")) | not) as $word
      | ($word | capture("^(?<kind>bin|test):(?<name>.*)$") // null) as $named
      | [if $word == "lib" then $targets[] | select(.kind == "lib") | "\(.package) lib \(.name)"
         elif $word == "doc" then $targets[] | select(.kind == "lib" and .doctest) | "\(.package) doc \(.name)"
         elif $named != null then $targets[] | select(.kind == $named.kind and .name == $named.name)
           | "\(.package) \(.kind) \(.name)"
         else empty end] as $units
      | if ($units | length) == 0 then {shard: $shard, word: $word, unit: null, filter: $filter}
        else $units[] | {shard: $shard, word: $word, unit: ., filter: $filter} end]
    as $selected
  | $unread[],
    ($selected[] | select(.unit == null and (.word | test("^(lib|doc|(bin|test):[A-Za-z0-9_-]+)$")))
      | "\(.shard): \(.word) selects no target"),
    ($selected[] | select(.unit != null) | .unit as $unit | select($suite | index($unit) | not)
      | "\(.shard) runs \($unit), which cargo test --workspace does not"),
    ($suite[] as $unit
      | [$selected[] | select(.unit == $unit)] as $runs
      | ($runs | map(.filter) | sort) as $filters
      | if ($runs | length) == 0 then "no shard runs \($unit)"
        elif $filters == ["none"] then empty
        elif ($filters | length) == 2 and ($filters[0] | startswith("only:"))
             and $filters[1] == "skip:\($filters[0] | ltrimstr("only:"))" then empty
        # `only:F skip:G`, `only:G` and `skip:F`, sorted: the space sorts first.
        elif ($filters | length) == 3 and ($filters[2] | startswith("skip:"))
             and ($filters[1] | startswith("only:"))
             and (($filters[2] | ltrimstr("skip:")) as $wider
                  | ($filters[1] | ltrimstr("only:")) as $narrower
                  | ($narrower | startswith($wider)) and $narrower != $wider
                    and $filters[0] == "only:\($wider) skip:\($narrower)") then empty
        else "\($unit) does not run exactly once: \($runs | map("\(.shard) (\(.filter))") | join(", "))"
        end)
' --args "${plan_lines[@]}" <<<"$metadata")" || refuse 'cargo metadata could not be read'
if [ -n "$problems" ]; then
  printf '%s\n' "$problems" >&2
  refuse 'the shards are not the whole suite, each test target exactly once'
fi

# One shard's whole command, into `argv`.
argv_of() {
  local word libtest=()
  argv=(${prefix[@]+"${prefix[@]}"})
  for word in ${shard_selectors[$1]}; do
    case "$word" in
      lib) argv+=(--lib) ;;
      doc) argv+=(--doc) ;;
      # `bin:<name>` passes `--bin <name>`, `test:<name>` the test target.
      bin:* | test:*) argv+=("--${word%%:*}" "${word#*:}") ;;
      skip:*) libtest+=(--skip "${word#skip:}") ;;
      only:*) libtest+=("${word#only:}") ;;
    esac
  done
  if [ "${#libtest[@]}" -gt 0 ]; then argv+=(-- "${libtest[@]}"); fi
}

if [ "$mode" = plan ]; then
  for index in "${planned[@]}"; do
    argv_of "$index"
    printf '%s:%s\n' "${shard_name[$index]}" "$(printf ' %s' "${argv[@]}")"
  done
  exit 0
fi
if [ "$mode" = machines ]; then
  for system in linux macos; do
    number=1
    while :; do
      names=''
      for index in "${planned[@]}"; do
        if [ "$(machine_of "$system" "$index")" = "$number" ]; then
          names+="${names:+ | }${shard_name[$index]}"
        fi
      done
      [ -n "$names" ] || break
      printf '%s/%s: %s\n' "$system" "$number" "$names"
      number=$((number + 1))
    done
  done
  exit 0
fi
if [ -n "$machine" ]; then
  printf 'test shards: machine %s runs %s of the %s shards\n' "$machine" "${#running[@]}" "${#planned[@]}"
fi

# Every test binary is built once, before any shard runs, so a compile
# error is reported once and no shard's tests, some of them bound to a
# wall-clock deadline, run beside a build. cargo-llvm-cov cannot build
# without running, so the coverage mode runs every binary once with
# libtest's `--list`, which executes no test and runs no workspace code
# before `main`, so it adds no count to the report.
if [ "$mode" = test ]; then
  "${prefix[@]}" --no-run
else
  "${prefix[@]}" -- --list >/dev/null
fi
# The gate's report builds what every machine measured, and runs no shard.
if [ "$mode" = coverage-build ]; then exit 0; fi

# The protected system set the broker's boundary proofs read, prepared in
# the background while the build ran (#647): where setup-bubblewrap's
# `protect-system-tree: background` named its directory in
# BROKKR_SYSTEM_SET, no shard starts until the preparation's `status` there
# records exit 0. A directory that is not there, a status that does not
# arrive within BROKKR_SYSTEM_SET_WAIT_SECONDS (600 unless set), one that
# cannot be read and a non-zero exit are each refused with its log printed:
# the set may be half protected. Where nothing names a set, nothing waits.
# An exit status and its seconds as a runner records them: the status is
# what `$?` can be, 0 to 255 written plainly, so a record outside that is
# refused as unreadable, never compared as a number a shell cannot hold.
exit_status_re='^(0|[1-9][0-9]?|1[0-9]{2}|2[0-4][0-9]|25[0-5]) [0-9]+$'
# The record in the file $1, into `record`, or '' where the file is not
# exactly one record and its newline, byte for byte. Bash drops a NUL from
# what it reads, with a warning, and strips every trailing newline, so the
# file is compared whole with the record read from it; a file that cannot
# be read is never read, since a failed redirection would end the script
# before any refusal.
read_record() {
  record=''
  if [ -r "$1" ] && { record="$(<"$1")"; } 2>/dev/null && [[ "$record" =~ $exit_status_re ]] &&
    printf '%s\n' "$record" | cmp -s - "$1"; then
    return
  fi
  record=''
}
await_system_set() {
  local dir="$1" limit="${BROKKR_SYSTEM_SET_WAIT_SECONDS:-600}" started="$SECONDS"
  local record='' status='' seconds='' refusal=''
  if ! [[ "$limit" =~ ^[0-9]+$ ]]; then
    printf '\n==> the protected system set: not awaited\n'
    if [ -f "$dir/log" ]; then cat "$dir/log"; fi
    refuse "BROKKR_SYSTEM_SET_WAIT_SECONDS '$limit' is not a number of seconds"
  fi
  [ -d "$dir" ] || refuse "the protected system set's directory $dir is not there, so its preparation was never started"
  while [ ! -e "$dir/status" ] && [ "$((SECONDS - started))" -lt "$limit" ]; do sleep 1; done
  # The whole record is read, a line it does not end with included: a
  # second record or any other byte refuses as unreadable.
  read_record "$dir/status"
  if [ ! -e "$dir/status" ]; then
    refusal="recorded no exit status within ${limit}s"
  elif [ -z "$record" ]; then
    refusal='recorded an exit status the guard cannot read'
  else
    status="${record%% *}" seconds="${record#* }"
    [ "$status" = 0 ] || refusal="failed with exit $status after ${seconds}s"
  fi
  printf '\n==> the protected system set: %s\n' "${refusal:-exit 0 after ${seconds}s, awaited for $((SECONDS - started))s}"
  if [ -f "$dir/log" ]; then cat "$dir/log"; fi
  [ -z "$refusal" ] || refuse "the protected system set's preparation $refusal, so the boundary proofs would read a set it may have left half protected"
}
if [ -n "${BROKKR_SYSTEM_SET:-}" ]; then await_system_set "$BROKKR_SYSTEM_SET"; fi

# shellcheck source=scripts/test-summary.sh
source scripts/test-summary.sh

# Why the output $1 of the shard $2 refuses it, or nothing: as
# scripts/run-it-tests.sh refuses a run, a summary the grammar cannot read
# anywhere, or no summary at all; and a filtered shard whose summaries show
# no test executed. A shard runs many binaries, and one a filter leaves
# empty prints a summary of none, so one executed test is enough.
summary_refusal() {
  summaries_read "$1"
  if [ "$unread" -eq 1 ]; then
    printf 'printed a test result: summary the guard cannot read, so no test is shown to have run'
  elif [ "$summaries" -eq 0 ]; then
    printf 'printed no test result: summary, so no test is shown to have run'
  elif [[ " ${shard_selectors[$2]}" == *" skip:"* || " ${shard_selectors[$2]}" == *" only:"* ]] &&
    [ "$executed" -eq 0 ]; then
    printf 'executed no test, so its filter matches none'
  fi
}

work="$(mktemp -d "${TMPDIR:-/tmp}/brokkr-test-shards.XXXXXX")"
# The logs are removed on exit only once every shard's runner has been
# reaped and each recorded its status: a runner that died first may leave
# its command still writing, and a run cut short leaves a runner going.
settled=false
trap 'if "$settled"; then rm -rf "$work"; fi' EXIT

failed=()
shard_status=()
shard_seconds=()
lost=false
# Runs the shard $1 and waits for its runner, which records the command's
# exit status and seconds, before anything else runs.
run_shard() {
  local index="$1" waited=0 status='' seconds='' refusal
  argv_of "$index"
  if "$parallel_heaps" && [ "${shard_name[$index]}" = 'the heap measurements' ]; then
    argv=(python3 scripts/parallel-heaps.py)
    if [ "$mode" = coverage ]; then argv+=(--nightly "${prefix[1]#+}"); fi
  fi
  printf 'test shards: runs %s:%s\n' "${shard_name[$index]}" "$(printf ' %s' "${argv[@]}")"
  # The runner runs in the foreground: a non-interactive shell starts a
  # background command with SIGINT and SIGQUIT ignored, and a test that
  # raises SIGINT on a child would see it ignored. Bash's own notice of
  # a runner killed by a signal is replaced by the refusal below, which
  # names the runner's exit.
  {
    (
      set +e
      start="$SECONDS"
      "${argv[@]}" >"$work/$index.log" 2>&1
      status=$?
      printf '%s %s\n' "$status" "$((SECONDS - start))" >"$work/$index.status"
    )
  } 2>/dev/null || waited=$?
  local record=''
  read_record "$work/$index.status"
  if [ -z "$record" ]; then
    lost=true
    shard_status[$index]=none
    printf '\n==> %s: no exit status recorded, its runner ended with exit %s; the logs are kept in %s\n' \
      "${shard_name[$index]}" "$waited" "$work"
    if [ -f "$work/$index.log" ]; then cat "$work/$index.log"; fi
    failed+=("${shard_name[$index]} recorded no exit status (its runner ended with exit $waited), so it is not shown to have passed")
    return
  fi
  status="${record%% *}" seconds="${record#* }"
  shard_status[$index]="$status"
  shard_seconds[$index]="$seconds"
  printf '\n==> %s: exit %s after %ss\n' "${shard_name[$index]}" "$status" "$seconds"
  cat "$work/$index.log"
  if [ "$status" != 0 ]; then
    failed+=("${shard_name[$index]} failed with exit $status")
  fi
  refusal="$(summary_refusal "$work/$index.log" "$index")"
  if [ -n "$refusal" ]; then failed+=("${shard_name[$index]} $refusal"); fi
}

for index in "${running[@]}"; do
  if ! "$lost"; then
    run_shard "$index"
    continue
  fi
  shard_status[$index]=unstarted
  failed+=("${shard_name[$index]} was not started: an earlier runner recorded no exit status, so its command may still be running and the shard cannot be shown to run alone")
done
if ! "$lost"; then settled=true; fi

printf '\n'
for index in "${running[@]}"; do
  case "${shard_status[$index]}" in
    none) printf 'test shards: %s: no exit status recorded\n' "${shard_name[$index]}" ;;
    unstarted) printf 'test shards: %s: not started\n' "${shard_name[$index]}" ;;
    *)
      printf 'test shards: %s: exit %s after %ss\n' "${shard_name[$index]}" \
        "${shard_status[$index]}" "${shard_seconds[$index]}"
      ;;
  esac
done
if [ "${#failed[@]}" -gt 0 ]; then
  printf 'test shards: %s\n' "${failed[@]}" >&2
  exit 1
fi
