#!/usr/bin/env bash
# The workspace suite with its slow test binaries run at once (#633). `cargo
# test` runs the test binaries one after another, and on CI's four cores
# brokkr-protocol's unit tests, brokkr-cli's `it` and the three heap
# measurements are most of the wall clock. This script runs the same tests,
# each exactly once, as shards: the shards of a phase run concurrently, each
# with its own output and its own exit status, and the phases run in order.
# The run fails when any shard fails, and only after every shard has ended.
#
#   scripts/test-shards.sh test                 the test jobs' suite, doctests included
#   scripts/test-shards.sh coverage <nightly>   the exact coverage gate's instrumented run
#   scripts/test-shards.sh plan test|coverage   the partition check and each shard's command
#
# Every shard selects workspace-wide (`--workspace --all-features`), never by
# package: Cargo unifies features over the selected packages, so a shard of
# one package would build, and test, another binary than the suite does.
# capability_broker's real-box tests are deadline-bound by design and must
# not share the runner with another heavy binary, so they run alone, in the
# second phase; the first phase runs `it` with them skipped. A libtest
# `--skip F` and the filter `F` select complementary tests, so that split
# runs each of `it`'s tests exactly once.
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
# be writing them. The threat model is the operator's of 2026-09-26:
# realistic accidental misuse is caught, and what cannot be read is refused.
set -euo pipefail

refuse() {
  printf 'test shards: %s\n' "$1" >&2
  exit 1
}

usage() {
  printf 'test shards: usage: %s test | coverage <nightly> | plan test|coverage\n' "$0" >&2
  exit 2
}

# The shards, in phase order. A selector is `lib` (every library's unit
# tests), `bin:<name>`, `test:<name>` or `doc` (every library's doctests,
# alone in its shard); `skip:<filter>` and `only:<filter>` pass libtest a
# skip or a filter.
shard_phase=()
shard_name=()
shard_selectors=()
shard() {
  shard_phase+=("$1")
  shard_name+=("$2")
  shift 2
  shard_selectors+=("$*")
}
shard 1 'every library' lib
shard 1 "the it binaries, capability_broker skipped" test:it skip:capability_broker::
shard 1 'the heap measurements' test:heap_claude test:heap_codex test:heap_dsh
shard 1 'the other own-process binaries' bin:brokkr test:driver_conformance test:transcript_surfaces \
  test:hands_exits test:secret_drop test:capability_launch test:operated_repo test:queued_launch \
  test:seatbelt_lifetime_probe
shard 1 'the doctests' doc
shard 2 'capability_broker, alone' test:it only:capability_broker::

mode="${1:-}"
case "$mode" in
  test)
    [ "$#" -eq 1 ] || usage
    prefix=(cargo test --workspace --all-features --locked --no-fail-fast)
    doctests=true
    ;;
  coverage)
    [ "$#" -eq 2 ] || usage
    [[ "$2" =~ ^nightly-[0-9]{4}-[0-9]{2}-[0-9]{2}$ ]] || refuse "the coverage toolchain '$2' is not nightly-YYYY-MM-DD"
    # cargo-llvm-cov runs no doctests unless asked, and the gate never asks.
    # `--no-fail-fast` runs a shard's every binary past a failing one, as
    # the test mode does; the shard still exits non-zero.
    prefix=(cargo "+$2" llvm-cov --no-report --workspace --all-features --locked --branch --no-fail-fast)
    doctests=false
    ;;
  plan)
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

for tool in jq cargo; do
  command -v "$tool" >/dev/null 2>&1 || refuse "$tool is required to hold the shards to the workspace"
done
cd "$(git rev-parse --show-toplevel)"

# The partition check, read from Cargo's own typed account of the targets.
plan_lines=()
for index in "${planned[@]}"; do
  plan_lines+=("${shard_name[$index]}	${shard_selectors[$index]}")
done
problems="$(cargo metadata --format-version 1 --no-deps --locked | jq -r --argjson doctests "$doctests" '
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
      | (if ($filters | length) > 1 then "\($shard): more than one filter: \($filters | join(" "))"
         else empty end),
        (.words[] | select((startswith("skip:") or startswith("only:")) | not) as $word
          | ($word | capture("^(?<kind>bin|test):(?<name>[A-Za-z0-9_-]+)$") // null) as $named
          | if $word == "lib" or $word == "doc" or $named != null then empty
            else "\($shard): a selector the plan cannot read: \($word)" end)]
    as $unread
  | [$shards[] | .shard as $shard
      | ([.words[] | select(startswith("skip:") or startswith("only:"))] | first // "none") as $filter
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
        else "\($unit) does not run exactly once: \($runs | map("\(.shard) (\(.filter))") | join(", "))"
        end)
' --args "${plan_lines[@]}")" || refuse 'cargo metadata could not be read'
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
      skip:*) libtest=(--skip "${word#skip:}") ;;
      only:*) libtest=("${word#only:}") ;;
    esac
  done
  if [ "${#libtest[@]}" -gt 0 ]; then argv+=(-- "${libtest[@]}"); fi
}

if [ "$mode" = plan ]; then
  for index in "${planned[@]}"; do
    argv_of "$index"
    printf 'phase %s, %s:%s\n' "${shard_phase[$index]}" "${shard_name[$index]}" "$(printf ' %s' "${argv[@]}")"
  done
  exit 0
fi

# Every test binary is built once, before any shard runs, so a compile
# error is reported once, the shards start together, and no shard's tests,
# some of them bound to a wall-clock deadline, run beside another shard's
# build. cargo-llvm-cov cannot build without running, so the coverage mode
# runs every binary once with libtest's `--list`, which executes no test and
# runs no workspace code before `main`, so it adds no count to the report.
if [ "$mode" = test ]; then
  "${prefix[@]}" --no-run
else
  "${prefix[@]}" -- --list >/dev/null
fi

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
# its command still writing, and a run cut short leaves runners going.
settled=false
trap 'if "$settled"; then rm -rf "$work"; fi' EXIT

failed=()
shard_status=()
shard_seconds=()
lost=false
run_phase() {
  local phase="$1" index pids=() indices=() at waited status seconds refusal
  local status_re='^[0-9]+ [0-9]+$'
  for index in "${planned[@]}"; do
    [ "${shard_phase[$index]}" = "$phase" ] || continue
    argv_of "$index"
    printf 'test shards: phase %s starts %s:%s\n' "$phase" "${shard_name[$index]}" "$(printf ' %s' "${argv[@]}")"
    (
      set +e
      start="$SECONDS"
      "${argv[@]}" >"$work/$index.log" 2>&1
      status=$?
      printf '%s %s\n' "$status" "$((SECONDS - start))" >"$work/$index.status"
    ) &
    pids+=("$!")
    indices+=("$index")
  done
  for at in "${!pids[@]}"; do
    index="${indices[$at]}"
    waited=0
    # Bash's own notice of a runner killed by a signal is replaced by the
    # refusal below, which names the runner's exit.
    wait "${pids[$at]}" 2>/dev/null || waited=$?
    status='' seconds=''
    if [ -s "$work/$index.status" ]; then read -r status seconds <"$work/$index.status" || true; fi
    if ! [[ "$status $seconds" =~ $status_re ]]; then
      lost=true
      shard_status[$index]=none
      printf '\n==> %s (phase %s): no exit status recorded, its runner ended with exit %s; the logs are kept in %s\n' \
        "${shard_name[$index]}" "$phase" "$waited" "$work"
      if [ -f "$work/$index.log" ]; then cat "$work/$index.log"; fi
      failed+=("${shard_name[$index]} recorded no exit status (its runner ended with exit $waited), so it is not shown to have passed")
      continue
    fi
    shard_status[$index]="$status"
    shard_seconds[$index]="$seconds"
    printf '\n==> %s (phase %s): exit %s after %ss\n' "${shard_name[$index]}" "$phase" "$status" "$seconds"
    cat "$work/$index.log"
    if [ "$status" -ne 0 ]; then
      failed+=("${shard_name[$index]} failed with exit $status")
    fi
    refusal="$(summary_refusal "$work/$index.log" "$index")"
    if [ -n "$refusal" ]; then failed+=("${shard_name[$index]} $refusal"); fi
  done
}

phases="$(printf '%s\n' "${shard_phase[@]}" | sort -un)"
for phase in $phases; do run_phase "$phase"; done
if ! "$lost"; then settled=true; fi

printf '\n'
for index in "${planned[@]}"; do
  if [ "${shard_status[$index]}" = none ]; then
    printf 'test shards: phase %s, %s: no exit status recorded\n' "${shard_phase[$index]}" "${shard_name[$index]}"
  else
    printf 'test shards: phase %s, %s: exit %s after %ss\n' "${shard_phase[$index]}" "${shard_name[$index]}" \
      "${shard_status[$index]}" "${shard_seconds[$index]}"
  fi
done
if [ "${#failed[@]}" -gt 0 ]; then
  printf 'test shards: %s\n' "${failed[@]}" >&2
  exit 1
fi
