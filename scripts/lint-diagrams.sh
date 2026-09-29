#!/usr/bin/env bash
# Render every mermaid diagram in the tracked Markdown with the pinned
# mermaid-cli, so a diagram that does not parse fails (issue #339). Needs
# `npm ci --prefix .github/lint --ignore-scripts` first, and the Chrome
# that .github/lint/puppeteer.json names.
#
# A fence is found however Markdown allows it to open: indented up to three
# spaces, with backticks or tildes. mermaid-cli renders only some of those
# forms, so each file must yield one rendered diagram per fence found, or
# the file is refused rather than passed half-read.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

mmdc=.github/lint/node_modules/.bin/mmdc
[ -x "$mmdc" ] || {
  printf '%s\n' 'lint-diagrams: run npm ci --prefix .github/lint --ignore-scripts first' >&2
  exit 1
}

fence='^ {0,3}(```|~~~)[[:space:]]*mermaid'
files=()
while IFS= read -r -d '' file; do
  files+=("$file")
done < <(git grep -l -z -E -e "$fence" -- '*.md')
# The tree has diagrams; finding none means this search broke, not that
# every diagram is fine.
[ "${#files[@]}" -gt 0 ] || {
  printf '%s\n' 'lint-diagrams: no mermaid diagram found in the tracked Markdown' >&2
  exit 1
}

out="$(mktemp -d "${TMPDIR:-/tmp}/brokkr-diagrams.XXXXXX")"
trap 'rm -rf "$out"' EXIT
for file in "${files[@]}"; do
  flat="${file//\//_}"
  "$mmdc" -q -p .github/lint/puppeteer.json -i "$file" -o "$out/$flat"
  found="$(grep -c -E "$fence" "$file")"
  rendered="$(find "$out" -maxdepth 1 -name "${flat%.md}-*.svg" | wc -l | tr -d ' ')"
  [ "$rendered" -eq "$found" ] || {
    printf 'lint-diagrams: %s renders %s of its %s mermaid diagrams; open each with ```mermaid\n' "$file" "$rendered" "$found" >&2
    exit 1
  }
  printf 'lint-diagrams: %s renders\n' "$file"
done
