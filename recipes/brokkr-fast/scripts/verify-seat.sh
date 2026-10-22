#!/usr/bin/env bash
# Deterministic verifier seat. Under a `namespace` boundary the box denies
# network; under `harness` it runs unboxed and no denial is reported. Cargo
# is told explicitly to stay offline so a cache miss is reported as such. It runs
# from the repository root, where scripts/lint-non-rust.sh lives.
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

# The decisive output is the failed tests, their panics and summaries, and
# the lines naming an error, else the output's tail; a suite's run also
# counts its summaries. A passing test whose name says "fail" is not
# decisive, so the error filter does not match it (#287). With "tail" as
# $2 it is the tail alone: a lint's finding (a SC code from shellcheck, an
# actionlint message) carries no such keyword, and the list's closing
# "lint-non-rust: <command> failed" line always would (#444).
failure_notes() {
    command_name="$1"
    printf '%s failed; decisive output follows verbatim:\n' "$command_name" > "$notes_file"
    if [ "${2-}" != tail ]; then
        grep -E '^test .+ \.\.\. FAILED$' "$output" | tail -n 20 >> "$notes_file" || true
        awk '/panicked at/ { print; if (getline > 0) print }' "$output" \
            | tail -n 20 >> "$notes_file" || true
        grep -E '^test result: FAILED|^error: test failed' "$output" \
            | tail -n 20 >> "$notes_file" || true
        grep -E '(^|[[:space:]])(error|Error|ERROR|Caused by|not found|offline)' "$output" \
            | tail -n 20 >> "$notes_file" || true
    fi
    [ "$(wc -l < "$notes_file")" -gt 1 ] || tail -n 20 "$output" >> "$notes_file"
    if grep -q '^test result: ' "$output"; then
        successful_summaries="$(grep -c '^test result: ok' "$output" || true)"
        failed_summaries="$(grep -c '^test result: FAILED' "$output" || true)"
        printf 'counts: %s successful test-suite summaries, %s failed\n' \
            "$successful_summaries" "$failed_summaries" >> "$notes_file"
    fi
    write_result fail
    exit 0
}

export CARGO_NET_OFFLINE=true
# Cheapest first, every required check that works offline, boxed or not
# (#427). The non-Rust lints are the list ci.yml's lint-non-rust job runs;
# a tool not on this seat's PATH, or not at the version CI pins, is named
# in the notes, never skipped silently.
if ! cargo fmt --all -- --check > "$output" 2>&1 </dev/null; then
    failure_notes "cargo fmt --all -- --check"
fi
if ! bash scripts/lint-non-rust.sh --seat > "$output" 2>&1 </dev/null; then
    failed_lint="$(sed -n 's/^lint-non-rust: \(.*\) failed$/\1/p' "$output")"
    failure_notes "${failed_lint:-bash scripts/lint-non-rust.sh --seat}" tail
fi
lint_notes="$(grep '^lint-non-rust: ' "$output")"
if ! cargo clippy --workspace --all-targets --all-features --locked -- -D warnings > "$output" 2>&1 </dev/null; then
    failure_notes "cargo clippy --workspace --all-targets --all-features --locked -- -D warnings"
fi
if ! cargo test --workspace > "$output" 2>&1 </dev/null; then
    failure_notes "cargo test --workspace"
fi
test_summaries="$(grep -c '^test result: ok' "$output" || true)"
if ! cargo run -p brokkr-cli -- compile --bundle recipes/self > "$output" 2>&1 </dev/null; then
    failure_notes "cargo run -p brokkr-cli -- compile --bundle recipes/self"
fi
printf 'cargo fmt --all -- --check: clean; cargo clippy --workspace --all-targets --all-features --locked -- -D warnings: clean\n%s\n' "$lint_notes" > "$notes_file"
printf 'cargo test --workspace: %s successful test-suite summaries, 0 failed; cargo run -p brokkr-cli -- compile --bundle recipes/self: 1 bundle compiled, 0 failed (offline from the bound Cargo registry cache)' "$test_summaries" >> "$notes_file"
write_result pass
