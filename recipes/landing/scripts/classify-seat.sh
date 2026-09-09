#!/usr/bin/env bash
# Landing's first gate (decision 0051): what class is the branch?
#
# Reads the repository's own docs class — `.github/delivery-classes.json`,
# the file the contribution gate cuts its tiers by (decision 0038 ruling
# 3) — and answers `docs` when every path the branch changes against its
# base matches the class, `code` otherwise. Seconds, no model, boxed like
# the verifier. Anything it cannot establish is answered `code`: a
# missing base, a missing class file, an empty diff. Fail closed toward
# the build, never toward the skip.
set -u

prompt_file="${1:-}"
[ -f "$prompt_file" ] || { echo "classify-seat: prompt file missing" >&2; exit 2; }
result_path=""
while IFS= read -r line; do
    trimmed="${line#"${line%%[![:space:]]*}"}"
    trimmed="${trimmed%"${trimmed##*[![:space:]]}"}"
    case "$trimmed" in /*.json) result_path="$trimmed" ;; esac
done < "$prompt_file"
[ -n "$result_path" ] || { echo "classify-seat: result path missing from prompt" >&2; exit 2; }
mkdir -p "$(dirname "$result_path")"

# The result contract (decision 0034): result, inputs, notes and nothing
# else. Notes are one line; the two JSON metacharacters are escaped.
write_result() {
    notes="$(printf '%s' "$2" | sed 's/\\/\\\\/g; s/"/\\"/g')"
    printf '{"result": "%s", "notes": "%s"}\n' "$1" "$notes" > "$result_path"
}

# The base is the default branch as this checkout knows it. A branch
# whose base is elsewhere says so in the feature text and is landed as
# code: this seat does not parse prose.
base=""
for candidate in origin/main main; do
    if git rev-parse --verify --quiet "$candidate" >/dev/null 2>&1; then
        base="$(git merge-base "$candidate" HEAD 2>/dev/null)" && [ -n "$base" ] && break
    fi
done
[ -n "$base" ] || { write_result code "no base branch (origin/main or main) to diff against; the branch is landed as code"; exit 0; }

classes=".github/delivery-classes.json"
[ -f "$classes" ] || { write_result code "$classes is absent; a repository without a docs class lands everything as code (decision 0038 ruling 3)"; exit 0; }
# The docs patterns as the file writes them, on one line, JSON escapes
# undone; jq is not assumed inside the box. The class file is the
# authority — this seat carries no pattern of its own.
regex="$(sed -n 's/.*"paths": *\[\(.*\)\].*/\1/p' "$classes" \
    | tr ',' '\n' | sed 's/^[[:space:]]*"//; s/"[[:space:]]*$//; s/\\\\/\\/g' | sed '/^$/d' | paste -sd'|' -)"
[ -n "$regex" ] || { write_result code "no docs paths in $classes; everything is code"; exit 0; }

paths="$(git diff --name-only "$base" HEAD)"
[ -n "$paths" ] || { write_result code "the branch changes nothing against ${base:0:7}; landed as code"; exit 0; }

count=0
outside=""
while IFS= read -r path; do
    [ -n "$path" ] || continue
    count=$((count + 1))
    printf '%s\n' "$path" | grep -E -q "$regex" || outside="$outside $path"
done <<< "$paths"

if [ -z "$outside" ]; then
    write_result docs "$count path(s) since ${base:0:7}, every one in the docs class; the landing enters at review"
else
    write_result code "$count path(s) since ${base:0:7}; outside the docs class:$outside"
fi
exit 0
