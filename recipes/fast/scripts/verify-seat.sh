#!/usr/bin/env bash
# Fast's verify is the folder's own (decision 0091 ruling 3). Where a folder
# stands, the compile takes the verify seat `brokkr init` wrote in
# .brokkr/bundle.json and never spawns this script. It stands where no folder
# does, and there it fails closed having run no check: it never passes. Brokkr's
# own verifier is brokkr-fast's.
set -u

prompt_file="${1:-}"
[ -f "$prompt_file" ] || { echo "verify-seat: prompt file missing" >&2; exit 2; }
result_path=""
while IFS= read -r line; do
    trimmed="${line#"${line%%[![:space:]]*}"}"
    trimmed="${trimmed%"${trimmed##*[![:space:]]}"}"
    case "$trimmed" in /*.json) result_path="$trimmed" ;; esac
done < "$prompt_file"
[ -n "$result_path" ] || { echo "verify-seat: result path missing from prompt" >&2; exit 2; }
mkdir -p "$(dirname "$result_path")"
notes="fast's verify is the folder's own (decision 0091), and no folder stands here, so no check ran: run brokkr init in this repository, or run a recipe that declares its own verify seat"
printf '{"result": "fail", "notes": "%s"}\n' "$notes" > "$result_path"
