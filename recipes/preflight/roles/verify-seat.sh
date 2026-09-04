#!/usr/bin/env bash
# Preflight recipe's deterministic local-CI verifier.
set -u
prompt_file="${1:-}"
[ -f "$prompt_file" ] || { echo "preflight verify-seat: prompt file missing" >&2; exit 2; }
result_path=""
while IFS= read -r line; do
    line="${line#"${line%%[![:space:]]*}"}"; line="${line%"${line##*[![:space:]]}"}"
    case "$line" in /*.json) result_path="$line" ;; esac
done < "$prompt_file"
[ -n "$result_path" ] || { echo "preflight verify-seat: result path missing" >&2; exit 2; }
mkdir -p "$(dirname "$result_path")"
output="$(dirname "$result_path")/preflight-output.$$"
notes="$(dirname "$result_path")/preflight-notes.$$"
trap 'rm -f "$output" "$notes"' EXIT
write_result() { awk -v result="$1" 'BEGIN{printf "{\"result\": \"%s\", \"notes\": \"",result}{gsub(/\\/,"\\\\");gsub(/\"/,"\\\"");if(NR>1)printf "\\n";printf "%s",$0}END{print "\"}"}' "$notes" > "$result_path"; }
run() {
    label="$1"; shift
    if ! "$@" > "$output" 2>&1 </dev/null; then
        printf '%s failed; decisive output follows verbatim:\n' "$label" > "$notes"
        grep -E '(error|Error|ERROR|fail|FAIL|not found|ENOSPC|No space)' "$output" | tail -n 24 >> "$notes" || true
        [ "$(wc -l < "$notes")" -gt 1 ] || tail -n 24 "$output" >> "$notes"
        write_result fail; exit 0
    fi
}
dirty="$(git status --porcelain 2>&1 || true)"
[ -z "$dirty" ] || { printf 'git status --porcelain is not empty:\n%s' "$dirty" > "$notes"; write_result fail; exit 0; }
run "git log --oneline main..HEAD" git log --oneline main..HEAD
[ -s "$output" ] || { printf 'git log --oneline main..HEAD is empty; the branch adds no commit' > "$notes"; write_result fail; exit 0; }
run "git diff main...HEAD --stat" git diff main...HEAD --stat
export CARGO_NET_OFFLINE=true
run "cargo fmt --all -- --check" cargo fmt --all -- --check
run "cargo clippy --workspace --all-targets --all-features --locked -- -D warnings" cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
run "cargo test --workspace --all-features --locked" cargo test --workspace --all-features --locked
run "cargo +1.88.0 check --workspace --locked" cargo +1.88.0 check --workspace --locked
run "compile bundles/self" cargo run --locked -p brokkr-cli -- compile --bundle bundles/self
run "compile bundles/verify" cargo run --locked -p brokkr-cli -- compile --bundle bundles/verify
run "bash scripts/coverage-exact.sh" bash scripts/coverage-exact.sh
run "cargo deny check licenses" cargo deny check licenses
run "cargo build --release --locked -p brokkr-cli" cargo build --release --locked -p brokkr-cli
printf 'all reproducible preflight gates passed offline; RustSec advisory data and the other operating-system matrix remain CI-only and are not claimed here' > "$notes"
write_result pass
