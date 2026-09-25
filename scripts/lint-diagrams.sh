#!/usr/bin/env bash
# Render every mermaid diagram in the tracked Markdown with the pinned
# mermaid-cli, so a diagram that does not parse fails (issue #339). Needs
# `npm ci --prefix .github/lint --ignore-scripts` first, and the Chrome
# that .github/lint/puppeteer.json names.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

mmdc=.github/lint/node_modules/.bin/mmdc
[ -x "$mmdc" ] || {
  printf '%s\n' 'lint-diagrams: run npm ci --prefix .github/lint --ignore-scripts first' >&2
  exit 1
}

files=()
while IFS= read -r -d '' file; do
  files+=("$file")
done < <(git grep -l -z -e '^```mermaid' -- '*.md')
# The tree has diagrams; finding none means this search broke, not that
# every diagram is fine.
[ "${#files[@]}" -gt 0 ] || {
  printf '%s\n' 'lint-diagrams: no mermaid diagram found in the tracked Markdown' >&2
  exit 1
}

out="$(mktemp -d "${TMPDIR:-/tmp}/brokkr-diagrams.XXXXXX")"
trap 'rm -rf "$out"' EXIT
for file in "${files[@]}"; do
  "$mmdc" -q -p .github/lint/puppeteer.json -i "$file" -o "$out/${file//\//_}"
  printf 'lint-diagrams: %s renders\n' "$file"
done
