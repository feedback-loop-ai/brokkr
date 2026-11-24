#!/usr/bin/env bash
# The docs lane (#1295): a pull request that changes only documentation
# runs the tests that read documentation in place of the whole matrix.
#
#   scripts/docs-lane.sh classify <base> <head>
#     prints `docs` or `full` on stdout, and on `full` its reason on stderr.
#   scripts/docs-lane.sh run
#     runs the tests scripts/docs-lane-tests.txt pins, each command through
#     scripts/run-it-tests.sh, so a filter that matches nothing fails.
#
# A change is `docs` only when every path `git diff <base>...<head>` lists,
# a rename read as a deletion and an addition, is a plain text file added,
# modified or deleted, and
#   (a) ends in `.md`;
#   (b) lies under docs/ or is a top-level `*.md`;
#   (c) has a file name that no file of the head's tree names: no file but
#       a `.md` one anywhere, and no file at all under crates/, scripts/,
#       recipes/, agents/, .github/ or quality/, the files the docs lane
#       runs whole aside. A file read by name, `include_str!`'d or pinned
#       by a digest is code's input, not documentation. A path holds its
#       file name, so this finds the path too.
# Anything else is `full`: an empty change, a commit git cannot find, a
# binary, a symlink or a mode change, a path (a) to (c) refuse, or a step
# that fails. When in doubt, it is full. The classification is a function
# of the two commits alone; it reads no working tree.
set -uo pipefail

tests_list=scripts/docs-lane-tests.txt

full() {
  printf 'docs lane: full: %s\n' "$1" >&2
  printf 'full\n'
  exit 0
}

# The files the docs lane runs whole, as the head's list names them: each
# entry's fourth field on, as exclude pathspecs.
lane_excludes() {
  local list line fields path
  list="$(git show "$1:$tests_list" 2>/dev/null)" || return 1
  while IFS= read -r line; do
    case "$line" in '' | '#'*) continue ;; esac
    read -r -a fields <<< "$line"
    [ "${#fields[@]}" -ge 4 ] || return 1
    for path in "${fields[@]:3}"; do
      printf ':(exclude,literal)%s\n' "$path"
    done
  done <<< "$list"
}

# Whether the head's tree names the file name of $2 outside the
# documentation and the docs lane's files: 0 when it does, 1 when it does
# not, anything else when it could not be searched.
named() {
  local head="$1" path="$2" excludes
  excludes="$(lane_excludes "$head")" || return 2
  [ -n "$excludes" ] || return 2
  local pathspecs=() status=0
  mapfile -t pathspecs <<< "$excludes"
  # Every file but a `.md` one, anywhere; then everything under the trees
  # code is built from, their `.md` files included (an agent's charter, a
  # recipe's role, a dialect `include_str!`'d).
  git grep -q --no-color -F -e "${path##*/}" "$head" -- . \
    ':(exclude,glob)**/*.md' "${pathspecs[@]}" 2> /dev/null || status=$?
  [ "$status" -eq 1 ] || return "$status"
  git grep -q --no-color -F -e "${path##*/}" "$head" -- \
    crates scripts recipes agents .github quality "${pathspecs[@]}" 2> /dev/null
}

# Each change, its `--raw` line and its path, one pair a line pair: a path
# with a newline in it is refused rather than split.
classify() {
  local base="$1" head="$2" meta path fields count=0
  # Not local: the EXIT trap reads it after this function returns.
  scratch=""
  git rev-parse -q --verify "$base^{commit}" > /dev/null || full "the base $base is not a commit here"
  git rev-parse -q --verify "$head^{commit}" > /dev/null || full "the head $head is not a commit here"
  scratch="$(mktemp -d "${TMPDIR:-/tmp}/brokkr-docs-lane.XXXXXX")" || full "no scratch directory"
  trap 'rm -rf "$scratch"' EXIT
  local diff=(diff --no-renames --no-ext-diff --no-textconv -z)
  git "${diff[@]}" --raw "$base...$head" -- > "$scratch/raw" || full "git diff $base...$head failed"
  git "${diff[@]}" --numstat "$base...$head" -- > "$scratch/numstat" ||
    full "git diff --numstat $base...$head failed"
  while IFS= read -r -d '' meta; do
    case "$meta" in -$'\t'-$'\t'*) full "${meta#-$'\t'-$'\t'} is a binary file" ;; esac
  done < "$scratch/numstat"
  while IFS= read -r -d '' meta; do
    IFS= read -r -d '' path || full "git diff's raw listing ends in $meta"
    count=$((count + 1))
    case "$path" in *$'\n'*) full "a changed path holds a newline" ;; esac
    # ":<old mode> <new mode> <old blob> <new blob> <status>"
    read -r -a fields <<< "${meta#:}"
    case "${fields[4]-}:${fields[0]-}:${fields[1]-}" in
      M:100644:100644 | A:000000:100644 | D:100644:000000) ;;
      *) full "$path changed as ${meta#:}, not as a text file added, modified or deleted" ;;
    esac
    case "$path" in *.md) ;; *) full "$path is not a .md file" ;; esac
    case "$path" in docs/*) ;; */*) full "$path is neither under docs/ nor a top-level .md" ;; esac
    named "$head" "$path"
    case $? in
      0) full "the file name of $path is named in code's input, outside the docs lane's tests" ;;
      1) ;;
      *) full "the head could not be searched for $path" ;;
    esac
  done < "$scratch/raw"
  [ "$count" -gt 0 ] || full "the change is empty"
  printf 'docs\n'
}

# The pinned tests, the filters of one package and target run as one
# command, in the order the list first names each.
run_lane() {
  local package target filter key status=0 kind selected
  local -A filters=()
  local order=()
  [ -f "$tests_list" ] || { printf 'docs lane: %s is missing\n' "$tests_list" >&2; exit 1; }
  while read -r package target filter _; do
    case "$package" in '' | '#'*) continue ;; esac
    key="$package $target"
    [ -n "${filters[$key]+set}" ] || order+=("$key")
    filters[$key]+=" $filter"
  done < "$tests_list"
  [ "${#order[@]}" -gt 0 ] || { printf 'docs lane: %s pins no test\n' "$tests_list" >&2; exit 1; }
  for key in "${order[@]}"; do
    read -r package target <<< "$key"
    read -r -a selected <<< "${filters[$key]}"
    # As in scripts/test-shards.sh: `lib` passes `--lib`, `test:<name>` the
    # test target.
    case "$target" in
      lib) kind=(--lib) ;;
      test:?*) kind=("--${target%%:*}" "${target#*:}") ;;
      *) printf 'docs lane: %s names %s, neither lib nor test:<name>\n' "$tests_list" "$target" >&2; exit 1 ;;
    esac
    scripts/run-it-tests.sh cargo test --locked -p "$package" "${kind[@]}" -- "${selected[@]}" || status=1
  done
  exit "$status"
}

case "${1-}" in
  classify)
    [ "$#" -eq 3 ] || { printf 'usage: docs-lane.sh classify <base> <head>\n' >&2; exit 2; }
    classify "$2" "$3"
    ;;
  run)
    [ "$#" -eq 1 ] || { printf 'usage: docs-lane.sh run\n' >&2; exit 2; }
    cd "$(git rev-parse --show-toplevel)" || exit 1
    run_lane
    ;;
  *)
    printf 'usage: docs-lane.sh classify <base> <head> | run\n' >&2
    exit 2
    ;;
esac
