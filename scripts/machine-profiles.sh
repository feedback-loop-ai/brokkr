#!/usr/bin/env bash
# The exact coverage gate's machines hand their profiles to its report
# (operator ruling 2026-11-01). A machine merges every profile of its run
# into one indexed profile and writes beside it a manifest naming the machine
# and the candidate it measured; the report takes each machine's profile into
# its own run, refusing a manifest it cannot read, a machine the layout does
# not name, one measured twice or never, and one that measured another
# candidate. A machine's list of unreadable profiles joins the report's
# evidence, each line led by the machine.
#
#   scripts/machine-profiles.sh write <llvm-profdata> <target dir> <run name> <machine> <candidate> <out dir>
#   scripts/machine-profiles.sh read <profiles dir> <candidate> <target dir> <run name> <list file> <machine>...
#
# The report gives a machine's profile this run's name, `<run name>-machine-
# linux-2.profraw`: llvm-profdata reads a profile by its contents, so an
# indexed one merges beside raw ones, and the gate merges only this run's
# names. The candidate is the gate's: the checked-out tree and the cache key.
set -euo pipefail

refuse() {
  printf 'machine profiles: %s\n' "$1" >&2
  exit 1
}

schema=brokkr.coverage-machine/v1
case "${1:-}" in
  write)
    [ "$#" -eq 7 ] ||
      refuse "usage: $0 write <llvm-profdata> <target dir> <run name> <machine> <candidate> <out dir>"
    tool="$2" target="$3" run="$4" machine="$5" candidate="$6" out="$7"
    [ -x "$tool" ] || refuse "$tool is not an executable llvm-profdata"
    shopt -s nullglob
    profiles=("$target/$run"-*.profraw)
    shopt -u nullglob
    [ "${#profiles[@]}" -gt 0 ] || refuse "machine $machine wrote no profile under $target"
    mkdir -p "$out"
    "$tool" merge -sparse -o "$out/machine.profdata" "${profiles[@]}" ||
      refuse "machine $machine's ${#profiles[@]} profiles do not merge"
    jq -n --arg schema "$schema" --arg machine "$machine" --arg candidate "$candidate" \
      '{schema: $schema, machine: $machine, candidate: $candidate}' >"$out/machine.json"
    printf 'machine profiles: %s merged %s profiles for the report\n' "$machine" "${#profiles[@]}"
    ;;
  read)
    [ "$#" -ge 7 ] ||
      refuse "usage: $0 read <profiles dir> <candidate> <target dir> <run name> <list file> <machine>..."
    dir="$2" candidate="$3" target="$4" run="$5" list="$6"
    shift 6
    laid_out=" $* "
    seen=' '
    : >"$list"
    shopt -s nullglob
    manifests=("$dir"/*/machine.json)
    shopt -u nullglob
    [ "${#manifests[@]}" -gt 0 ] || refuse "no machine's manifest is under $dir"
    for manifest in "${manifests[@]}"; do
      measured="$(jq -r --arg schema "$schema" '
        select(type == "object" and .schema == $schema
               and (.machine | type) == "string" and (.candidate | type) == "string")
        | "\(.machine) \(.candidate)"' "$manifest" 2>/dev/null)" || measured=''
      [[ "$measured" =~ ^((linux|macos)/[1-9][0-9]?)\ ([^[:space:]]+)$ ]] ||
        refuse "$manifest is not a machine's manifest"
      machine="${BASH_REMATCH[1]}" measured_candidate="${BASH_REMATCH[3]}"
      [[ "$laid_out" == *" $machine "* ]] || refuse "$manifest names $machine, which the layout does not"
      [[ "$seen" != *" $machine "* ]] || refuse "machine $machine was measured twice"
      [ "$measured_candidate" = "$candidate" ] ||
        refuse "machine $machine measured $measured_candidate, not this candidate, $candidate"
      [ -f "$(dirname "$manifest")/machine.profdata" ] || refuse "machine $machine left no profile"
      seen+="$machine "
      cp -- "$(dirname "$manifest")/machine.profdata" "$target/$run-machine-${machine//\//-}.profraw"
      if [ -f "$(dirname "$manifest")/unreadable-profiles.txt" ]; then
        while IFS= read -r line; do
          printf '%s\t%s\n' "$machine" "$line" >>"$list"
        done <"$(dirname "$manifest")/unreadable-profiles.txt"
      fi
    done
    for machine in "$@"; do
      [[ "$seen" == *" $machine "* ]] || refuse "machine $machine was never measured"
    done
    printf 'machine profiles: the report reads %s machines\n' "$#"
    ;;
  *) refuse "usage: $0 write ... | read ..." ;;
esac
