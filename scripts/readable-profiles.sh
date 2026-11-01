#!/usr/bin/env bash
# The exact coverage gate's profiles, each read on its own before the merge
# (operator ruling 2026-11-01). A process killed while it writes its profile
# leaves a file no reader can parse, and one such file fails cargo-llvm-cov's
# whole merge: on 2026-11-01 a merge group whose every shard passed was
# refused "file header is corrupt" (#935). So every profile of this run that
# `llvm-profdata show` cannot read is moved aside, and each is named, with
# the reader's first line, on stderr and in the list file.
#
# Leaving a profile out can only lower what the report counts: the regions
# and branches the report must cover come from the instrumented binaries,
# never from the profiles, so a region only the dead process reached reads
# as missed and the gate refuses. A run none of whose profiles can be read,
# or that wrote none, is refused here by name.
#
#   scripts/readable-profiles.sh <llvm-profdata> <target dir> <run name> <aside dir> <list file>
#
# Only `<target dir>/<run name>-*.profraw` is read, the names the gate gives
# this run's profiles; a profile of another run is left where it is, for the
# gate's own check to refuse.
set -euo pipefail

refuse() {
  printf 'readable profiles: %s\n' "$1" >&2
  exit 1
}

[ "$#" -eq 5 ] ||
  refuse "usage: $0 <llvm-profdata> <target dir> <run name> <aside dir> <list file>"
tool="$1" target="$2" run="$3" aside="$4" list="$5"
[ -x "$tool" ] || refuse "$tool is not an executable llvm-profdata"
[ -d "$target" ] || refuse "the target directory $target is not there"
[[ "$run" =~ ^[A-Za-z0-9._-]+$ ]] || refuse "the run name '$run' is not one the gate gives"

shopt -s nullglob
profiles=("$target/$run"-*.profraw)
shopt -u nullglob
[ "${#profiles[@]}" -gt 0 ] || refuse "this run wrote no profile under $target"

mkdir -p "$aside"
: >"$list"
left=0
for profile in "${profiles[@]}"; do
  if "$tool" show "$profile" >/dev/null 2>"$aside/.reader"; then continue; fi
  name="$(basename "$profile")"
  reason="$(head -n 1 "$aside/.reader")"
  reason="${reason//"$profile"/$name}"
  mv -- "$profile" "$aside/"
  left=$((left + 1))
  printf '%s\t%s\n' "$name" "${reason:-no reason printed}" >>"$list"
  printf 'readable profiles: left out %s, which llvm-profdata cannot read: %s\n' \
    "$name" "${reason:-no reason printed}" >&2
done
rm -f "$aside/.reader"

[ "$left" -lt "${#profiles[@]}" ] ||
  refuse "none of this run's ${#profiles[@]} profiles can be read"
if [ "$left" -gt 0 ]; then
  printf 'readable profiles: %s of %s profiles left out; the report counts the rest\n' \
    "$left" "${#profiles[@]}" >&2
fi
