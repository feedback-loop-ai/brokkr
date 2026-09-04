#!/usr/bin/env bash
# Deterministic verifier seat. The box denies network; Cargo is also told
# explicitly to stay offline so a cache miss is reported as such.
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
output="$(dirname "$result_path")/verify-seat-output.$$"
notes_file="$(dirname "$result_path")/verify-seat-notes.$$"
trap 'rm -f "$output" "$notes_file"' EXIT

write_result() {
    seat_result="$1"
    awk -v result="$seat_result" 'BEGIN { printf "{\"result\": \"%s\", \"notes\": \"", result }
      { gsub(/\\/, "\\\\"); gsub(/\"/, "\\\""); if (NR > 1) printf "\\n"; printf "%s", $0 }
      END { print "\"}" }' "$notes_file" > "$result_path"
}

failure_notes() {
    command_name="$1"
    printf '%s failed; decisive output follows verbatim:\n' "$command_name" > "$notes_file"
    grep -E '(^|[[:space:]])(error|Error|ERROR|fail|FAILED|Caused by|not found|offline)' "$output" \
        | tail -n 20 >> "$notes_file" || true
    [ "$(wc -l < "$notes_file")" -gt 1 ] || tail -n 20 "$output" >> "$notes_file"
    write_result fail
    exit 0
}

export CARGO_NET_OFFLINE=true
if ! cargo test --workspace > "$output" 2>&1 </dev/null; then
    failure_notes "cargo test --workspace"
fi
test_summaries="$(grep -c '^test result: ok' "$output" || true)"
if ! cargo run -p brokkr-cli -- compile --bundle bundles/self > "$output" 2>&1 </dev/null; then
    failure_notes "cargo run -p brokkr-cli -- compile --bundle bundles/self"
fi
printf 'cargo test --workspace: %s successful test-suite summaries, 0 failed; cargo run -p brokkr-cli -- compile --bundle bundles/self: 1 bundle compiled, 0 failed (offline from the bound Cargo registry cache)' "$test_summaries" > "$notes_file"
write_result pass
