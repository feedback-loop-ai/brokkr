#!/usr/bin/env bash
# The non-Rust lints that work offline, cheapest first (issues #339, #427):
# the one list both ci.yml's lint-non-rust job and recipes/fast's verify
# seat run, so the two cannot drift. Each row names the tools its command
# needs, and each tool must report the version CI installs: its entry on
# ci.yml's install-action `tool:` line, or its setup action's
# <TOOL>_VERSION.
#
# With no argument, as in CI, a tool that is not on PATH at its pin is
# refused. With --seat, as in a boxed verify seat that reaches only what
# its binds carry, that row is named on stdout as not run and the others
# still run. Either way the first lint that fails stops the list, named on
# stderr.
set -uo pipefail
cd "$(git rev-parse --show-toplevel)" || exit 1

lints=(
  "typos: typos --hidden"
  "shellcheck: git ls-files -z '*.sh' | xargs -0 shellcheck -S warning"
  "shellcheck: bash scripts/shellcheck-actions.sh"
  "actionlint shellcheck: SHELLCHECK_OPTS='-S warning' actionlint"
  "zizmor: git ls-files -z .github/workflows .github/actions | xargs -0 zizmor --offline"
  "lychee: git ls-files -z '*.md' | xargs -0 lychee --offline --include-fragments --no-progress"
)

case "${1-}" in
  "") seat=false ;;
  --seat) seat=true ;;
  *) printf 'lint-non-rust: unknown argument %s; the only one is --seat\n' "$1" >&2; exit 2 ;;
esac

# Every version CI installs $1 at; exactly one is a pin.
pins() {
  case "$1" in
    actionlint | lychee)
      sed -n "s/^ *$(tr '[:lower:]' '[:upper:]' <<< "$1")_VERSION: *//p" ".github/actions/setup-$1/action.yml" ;;
    *) grep -oE "[ ,]$1@[^,[:space:]]+" .github/workflows/ci.yml | sed 's/^.*@//' ;;
  esac | sort -u
}

ran=0
for row in "${lints[@]}"; do
  tools="${row%%: *}" command="${row#*: }" why=""
  for tool in $tools; do
    pin="$(pins "$tool")"
    if [ -z "$pin" ] || [ "$(wc -l <<< "$pin")" -ne 1 ]; then
      printf 'lint-non-rust: CI does not pin exactly one version of %s\n' "$tool" >&2
      exit 1
    fi
    if ! command -v "$tool" > /dev/null; then
      why="$tool is not on PATH"
    elif ! reported="$("$tool" --version 2>&1 < /dev/null)" || ! grep -qwF -- "$pin" <<< "$reported"; then
      why="$tool on PATH reports $(head -n 1 <<< "$reported"), not the pinned $pin"
    fi
    [ -z "$why" ] || break
  done
  if [ -n "$why" ]; then
    $seat || { printf 'lint-non-rust: %s cannot run: %s\n' "$command" "$why" >&2; exit 1; }
    printf 'lint-non-rust: not run: %s (%s)\n' "$command" "$why"
    continue
  fi
  bash -o pipefail -c "$command" < /dev/null || {
    printf 'lint-non-rust: %s failed\n' "$command" >&2
    exit 1
  }
  ran=$((ran + 1))
done
printf 'lint-non-rust: %d of %d lints ran clean\n' "$ran" "${#lints[@]}"
