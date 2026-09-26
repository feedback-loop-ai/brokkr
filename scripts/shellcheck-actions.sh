#!/usr/bin/env bash
# Shellcheck the `run: |` scripts of the local composite actions (issue
# #339). actionlint reads only workflows, and the `*.sh` step only script
# files, so these bash bodies are checked here, each as the file its step
# runs.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

scratch="$(mktemp -d "${TMPDIR:-/tmp}/brokkr-action-scripts.XXXXXX")"
trap 'rm -rf "$scratch"' EXIT
count=0
while IFS= read -r -d '' action; do
  # Each block scalar after `run: |`, dedented, into a file of its own.
  awk -v out="$scratch/$count" '
    body && (/^[[:space:]]*$/ || match($0, /^ */) && RLENGTH > indent) {
      print substr($0, indent + 3) > (out "-" n ".sh"); next
    }
    { body = 0 }
    /^ *run: \|$/ { match($0, /^ */); indent = RLENGTH; body = 1; n++
      print "#!/usr/bin/env bash" > (out "-" n ".sh") }
  ' "$action"
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
