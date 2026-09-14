#!/usr/bin/env bash
# Deterministic two-entry ship seat: write, then confirm, never commit or push.
set -u

prompt_file="${1:-}"
brokkr="${2:-brokkr}"
[ -f "$prompt_file" ] || { echo "ship-seat: prompt file missing" >&2; exit 2; }
result_path=""
run_id=""
rule_id=""
journal=""
in_context=false
while IFS= read -r line; do
    trimmed="${line#"${line%%[![:space:]]*}"}"
    trimmed="${trimmed%"${trimmed##*[![:space:]]}"}"
    if [ "$line" = '```json' ]; then
        in_context=true
        continue
    elif [ "$in_context" = true ] && [ "$line" = '```' ]; then
        in_context=false
        continue
    fi
    case "$trimmed" in
        /*.json) result_path="$trimmed" ;;
    esac
    # compose_prompt renders direct context keys at two spaces and fields of
    # last_decision at four. Inputs nested below it cannot impersonate either.
    if [ "$in_context" = true ]; then
        case "$line" in
            '  "journal": '*) journal="$(printf '%s' "$line" | sed -n 's/^  "journal": "\([^"]*\)".*/\1/p')" ;;
            '  "run_id": '*) run_id="$(printf '%s' "$line" | sed -n 's/^  "run_id": "\([^"]*\)".*/\1/p')" ;;
            '    "rule_id": '*) rule_id="$(printf '%s' "$line" | sed -n 's/^    "rule_id": "\([^"]*\)".*/\1/p')" ;;
        esac
    fi
done < "$prompt_file"
[ -n "$result_path" ] && [ -n "$run_id" ] || { echo "ship-seat: prompt lacks result path or run id" >&2; exit 2; }
mkdir -p "$(dirname "$result_path")"
notes_file="$(dirname "$result_path")/ship-seat-notes.$$"
trap 'rm -f "$notes_file"' EXIT

write_result() {
    seat_result="$1"
    awk -v result="$seat_result" '
      function json(text, out, i, byte, character) {
        for (i = 1; i <= length(text); i++) {
          character = substr(text, i, 1)
          if (character == "\\") out = out "\\\\"
          else if (character == "\"") out = out "\\\""
          else {
            for (byte = 1; byte < 32 && character != sprintf("%c", byte); byte++) {}
            out = out (byte < 32 ? sprintf("\\u%04x", byte) : character)
          }
        }
        return out
      }
      BEGIN { printf "{\"result\": \"%s\", \"notes\": \"", result }
      { if (NR > 1) printf "\\n"; printf "%s", json($0) }
      END { print "\"}" }' "$notes_file" > "$result_path"
}

ledger=".forge/ledger/$run_id.md"
if [ "$rule_id" != "SHIP-READY" ]; then
    [ -n "$journal" ] || { echo "ship-seat: prompt lacks journal path" >&2; exit 2; }
    dirty="$(git status --porcelain 2>&1 || true)"
    ledger_output="$("$brokkr" ledger --run "$run_id" --db "$journal" --repo . 2>&1)"
    ledger_status=$?
    if [ "$ledger_status" -ne 0 ]; then
        printf 'ship-seat: ledger generation failed: %s\n' "$ledger_output" >&2
        exit "$ledger_status"
    elif [ -n "$dirty" ]; then
        printf 'ledger written to %s; worktree discrepancy before close-out: %s' "$ledger" "$dirty" > "$notes_file"
    else
        printf 'ledger written to %s; review the recorded commits and evidence, then push and merge' "$ledger" > "$notes_file"
    fi
    write_result ready
    exit 0
fi

dirty="$(git status --porcelain 2>&1 || true)"
head="$(git rev-parse HEAD 2>&1 || true)"
recorded="$(sed -n 's/^Repository head: `\([^`]*\)`.*/\1/p' "$ledger" 2>/dev/null | head -n 1)"
if [ ! -f "$ledger" ]; then
    printf 'close-out discrepancy: ledger %s is missing' "$ledger" > "$notes_file"
elif [ -n "$dirty" ] && [ "$head" != "$recorded" ]; then
    printf 'close-out discrepancies: worktree is dirty (%s); HEAD is %s but ledger records %s' "$dirty" "$head" "$recorded" > "$notes_file"
elif [ -n "$dirty" ]; then
    printf 'close-out discrepancy: worktree is dirty (%s); HEAD still matches ledger at %s' "$dirty" "$head" > "$notes_file"
elif [ "$head" != "$recorded" ]; then
    printf 'close-out discrepancy: HEAD is %s but ledger records %s; worktree is clean' "$head" "$recorded" > "$notes_file"
else
    printf 'close-out confirmed at %s with a clean worktree; ledger %s records the delivery; review, push, and merge next' "$head" "$ledger" > "$notes_file"
fi
write_result shipped
