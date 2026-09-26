#!/usr/bin/env bash
# Shellcheck the `run: |` scripts of the local composite actions (issue
# #339). actionlint reads only workflows, and the `*.sh` step only script
# files, so these bash bodies are checked here, each as the file its step
# runs. Every `run:` is read: one that is not a `|` block (a folded `>`, a
# chomped `|-`, an indented `|2`, a one-line command) is refused by file and
# line rather than skipped, and so is an action tree with no script at all.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

scratch="$(mktemp -d "${TMPDIR:-/tmp}/brokkr-action-scripts.XXXXXX")"
trap 'rm -rf "$scratch"' EXIT
count=0
while IFS= read -r -d '' action; do
  # Each block scalar after `run: |`, dedented, into a file of its own.
  awk -v out="$scratch/$count" -v action="$action" '
    body && (/^[[:space:]]*$/ || match($0, /^ */) && RLENGTH > indent) {
      print substr($0, indent + 3) > (out "-" n ".sh"); next
    }
    { body = 0 }
    /^ *(- +)?run:/ {
      if ($0 !~ /^ *(- +)?run: \|$/) {
        line = $0; sub(/^ *(- +)?/, "", line)
        printf "shellcheck-actions: %s:%d: not a `run: |` block, so it is not read: %s\n", action, NR, line > "/dev/stderr"
        exit 3
      }
      match($0, /^ *(- +)?/); indent = RLENGTH; body = 1; n++
      print "#!/usr/bin/env bash" > (out "-" n ".sh")
    }
  ' "$action" || exit 1
  count=$((count + 1))
done < <(git ls-files -z '.github/actions/*/action.yml' '.github/actions/*/action.yaml')

scripts=("$scratch"/*.sh)
# The actions have scripts; finding none means this extraction broke, not
# that every script is fine.
[ -e "${scripts[0]}" ] || {
  printf '%s\n' 'shellcheck-actions: no run: | script found in .github/actions' >&2
  exit 1
}
shellcheck -S warning -s bash "${scripts[@]}"
printf 'shellcheck-actions: %d action scripts clean\n' "${#scripts[@]}"
