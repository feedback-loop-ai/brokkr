#!/usr/bin/env bash
# Research recipe's deterministic gate (decision 0044 rulings 4 and 6):
# the registry parses and every citation resolves, and the researcher
# wrote at most ten entries. Nothing here is a model's word.
set -u
prompt_file="${1:-}"
[ -f "$prompt_file" ] || { echo "research verify-seat: prompt file missing" >&2; exit 2; }
result_path=""
while IFS= read -r line; do
    line="${line#"${line%%[![:space:]]*}"}"; line="${line%"${line##*[![:space:]]}"}"
    case "$line" in /*.json) result_path="$line" ;; esac
done < "$prompt_file"
[ -n "$result_path" ] || { echo "research verify-seat: result path missing" >&2; exit 2; }
mkdir -p "$(dirname "$result_path")"
output="$(dirname "$result_path")/research-output.$$"
notes="$(dirname "$result_path")/research-notes.$$"
trap 'rm -f "$output" "$notes"' EXIT
write_result() { awk -v result="$1" '
  function json(text,out,i,byte,c){for(i=1;i<=length(text);i++){c=substr(text,i,1);if(c=="\\")out=out "\\\\";else if(c=="\"")out=out "\\\"";else{for(byte=1;byte<32&&c!=sprintf("%c",byte);byte++){}out=out (byte<32?sprintf("\\u%04x",byte):c)}}return out}
  BEGIN{printf "{\"result\": \"%s\", \"notes\": \"",result}{if(NR>1)printf "\\n";printf "%s",json($0)}END{print "\"}"}' "$notes" > "$result_path"; }
run() {
    label="$1"; shift
    if ! "$@" > "$output" 2>&1 </dev/null; then
        printf '%s failed; decisive output follows verbatim:\n' "$label" > "$notes"
        grep -E '(error|Error|ERROR|fail|FAIL|panicked|not found|ENOSPC|No space)' "$output" | tail -n 24 >> "$notes" || true
        [ "$(wc -l < "$notes")" -gt 1 ] || tail -n 24 "$output" >> "$notes"
        write_result fail; exit 0
    fi
}
dirty="$(git status --porcelain 2>&1 || true)"
[ -z "$dirty" ] || { printf 'git status --porcelain is not empty; the researcher left uncommitted work:\n%s' "$dirty" > "$notes"; write_result fail; exit 0; }
base="main"
git rev-parse --verify --quiet "$base" >/dev/null || base="origin/main"
run "git diff $base...HEAD --name-only" git diff "$base...HEAD" --name-only
if [ -s "$output" ]; then
    outside="$(grep -v '^docs/research/' "$output" || true)"
    [ -z "$outside" ] || { printf 'the researcher changed files outside docs/research/:\n%s' "$outside" > "$notes"; write_result fail; exit 0; }
    added="$(git diff "$base...HEAD" --name-only --diff-filter=A | grep -c '^docs/research/[0-9][0-9][0-9][0-9]-' || true)"
    [ "$added" -le 10 ] || { printf 'the researcher wrote %s entries; the operator ruled at most ten per run (decision 0044 ruling 4)' "$added" > "$notes"; write_result fail; exit 0; }
else
    added=0
fi
export CARGO_NET_OFFLINE=true
run "cargo test --locked -p brokkr-cli --test research_registry" cargo test --locked -p brokkr-cli --test research_registry
printf 'the registry parses, every citation resolves, and %s new entries were written on top of %s (cap ten); the classifications are proposals until the operator rules them' "$added" "$base" > "$notes"
write_result pass
