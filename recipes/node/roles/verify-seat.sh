#!/usr/bin/env bash
# Node recipe's deterministic verifier; same result-file contract as the Rust seat.
set -u
prompt_file="${1:-}"
[ -f "$prompt_file" ] || { echo "node verify-seat: prompt file missing" >&2; exit 2; }
result_path=""
while IFS= read -r line; do
    line="${line#"${line%%[![:space:]]*}"}"; line="${line%"${line##*[![:space:]]}"}"
    case "$line" in /*.json) result_path="$line" ;; esac
done < "$prompt_file"
[ -n "$result_path" ] || { echo "node verify-seat: result path missing" >&2; exit 2; }
mkdir -p "$(dirname "$result_path")"
output="$(dirname "$result_path")/node-verify-output.$$"
notes="$(dirname "$result_path")/node-verify-notes.$$"
trap 'rm -f "$output" "$notes"' EXIT
write_result() { awk -v result="$1" 'BEGIN{printf "{\"result\": \"%s\", \"notes\": \"",result}{gsub(/\\/,"\\\\");gsub(/\"/,"\\\"");if(NR>1)printf "\\n";printf "%s",$0}END{print "\"}"}' "$notes" > "$result_path"; }
run() {
    label="$1"; shift
    if ! "$@" > "$output" 2>&1 </dev/null; then
        printf '%s failed; decisive output follows verbatim:\n' "$label" > "$notes"
        grep -E '(error|Error|ERROR|fail|FAIL|not found)' "$output" | tail -n 20 >> "$notes" || true
        [ "$(wc -l < "$notes")" -gt 1 ] || tail -n 20 "$output" >> "$notes"
        write_result fail; exit 0
    fi
}
export npm_config_offline=true
run "npm ci --offline" npm ci --offline
run "npx tsc --noEmit" npx tsc --noEmit
run "npm test" npm test
lint="not declared"
if node -e 'let p=require("./package.json");process.exit(p.scripts&&p.scripts.lint?0:1)' </dev/null; then
    run "npm run lint" npm run lint
    lint="passed"
fi
dirty="$(git status --porcelain 2>&1 || true)"
if [ -n "$dirty" ]; then
    printf 'suite left the worktree dirty; git status --porcelain:\n%s' "$dirty" > "$notes"
    write_result fail
else
    printf 'npm ci --offline, npx tsc --noEmit, and npm test passed; npm run lint: %s' "$lint" > "$notes"
    write_result pass
fi
