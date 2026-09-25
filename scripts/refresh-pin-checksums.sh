#!/usr/bin/env bash
# Renovate's post-upgrade task (issue #339): after it bumps a tool pinned by
# version AND digest in .github/actions/<name>/action.yml, re-download the
# release the new version names and rewrite the recorded sha256. Renovate
# cannot compute a release tarball's digest; this script can.
#
# The download URL has one home, the action's own `curl` line; its
# `${NAME}` placeholders are filled from the same file (the step's env and
# its `asset=` line). The script fails closed: a placeholder it cannot
# fill or a download that fails stops it before any file is written.
#
#   scripts/refresh-pin-checksums.sh               refresh every digest
#   scripts/refresh-pin-checksums.sh --print-urls  print each URL, fetch nothing
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

fail() {
  printf 'refresh-pin-checksums: %s\n' "$*" >&2
  exit 1
}

# The value of `NAME: value` in an action's env block.
env_value() {
  sed -n "s/^ *$2: \(.*\)$/\1/p" "$1" | head -n 1
}

# The release URL an action downloads, its placeholders filled.
release_url() {
  local action="$1" url asset name value
  url="$(grep -A1 -F 'curl -sSfL --retry 3 -o "$tarball"' "$action" | sed -n '2s/^ *"\(.*\)"$/\1/p')"
  [ -n "$url" ] || fail "$action: no release URL after its curl line"
  asset="$(sed -n 's/^ *asset="\(.*\)"$/\1/p' "$action" | head -n 1)"
  while [[ "$url" =~ \$\{([A-Za-z_][A-Za-z0-9_]*)\} ]]; do
    name="${BASH_REMATCH[1]}"
    if [ "$name" = asset ]; then value="$asset"; else value="$(env_value "$action" "$name")"; fi
    [ -n "$value" ] || fail "$action: \${$name} has no value"
    url="${url//"\${$name}"/$value}"
  done
  printf '%s\n' "$url"
}

print_only=false
[ "${1:-}" = --print-urls ] && print_only=true

scratch="$(mktemp -d "${TMPDIR:-/tmp}/refresh-pin-checksums.XXXXXX")"
trap 'rm -rf "$scratch"' EXIT
# First every digest is measured; only when all are in hand is any written.
updates=()
for action in .github/actions/*/action.yml; do
  key="$(grep -oE '^ +[A-Z0-9_]+_SHA256:' "$action" | tr -d ' :' | head -n 1)"
  [ -n "$key" ] || continue
  url="$(release_url "$action")"
  if "$print_only"; then
    printf '%s %s\n' "$action" "$url"
    continue
  fi
  curl -sSfL --retry 3 -o "$scratch/release" "$url" || fail "$action: could not download $url"
  digest="$(sha256sum "$scratch/release" | cut -d' ' -f1)"
  [ "${#digest}" -eq 64 ] || fail "$action: no sha256 for $url"
  updates+=("$action $key $digest")
done
for update in "${updates[@]}"; do
  read -r action key digest <<<"$update"
  sed -i "s/^\( *$key: \)[0-9a-f]\{64\}$/\1$digest/" "$action"
  [ "$(env_value "$action" "$key")" = "$digest" ] || fail "$action: $key was not rewritten"
  printf 'refresh-pin-checksums: %s %s\n' "$action" "$digest"
done
