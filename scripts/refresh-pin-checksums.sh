#!/usr/bin/env bash
# Renovate's post-upgrade task (issue #339): after it bumps a tool pinned by
# version AND digest in .github/actions/<name>/action.yml, re-download the
# release the new version names and rewrite that tool's recorded sha256.
# Renovate cannot compute a release tarball's digest; this script can.
#
# A digest moves only with its own version. For every action whose
# `*_VERSION` changed against `git show HEAD:<file>`, the release is
# downloaded and its digest rewritten. For every action whose version did
# not change, the release is downloaded too and must still match the
# recorded digest: an asset swapped upstream under an unchanged version is
# refused, never re-pinned. The script fails closed, with nothing written,
# when a download fails, a placeholder cannot be filled, a recorded digest
# no longer matches, or no pinned version moved at all.
#
# The download URL has one home, the action's own `curl` line; its
# `${NAME}` placeholders are filled from the same file (the step's env and
# its `asset=` line).
#
#   scripts/refresh-pin-checksums.sh               refresh the moved digests
#   scripts/refresh-pin-checksums.sh --print-urls  print each URL, fetch nothing
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

fail() {
  printf 'refresh-pin-checksums: %s\n' "$*" >&2
  exit 1
}

# The value of `NAME: value` in an action's env block, read from stdin.
env_value() {
  sed -n "s/^ *$1: \(.*\)$/\1/p" | head -n 1
}

# The release URL an action downloads, its placeholders filled.
release_url() {
  local action="$1" url asset name value
  url="$(grep -A1 -F 'curl -sSfL --retry 3 -o "$tarball"' "$action" | sed -n '2s/^ *"\(.*\)"$/\1/p')"
  [ -n "$url" ] || fail "$action: no release URL after its curl line"
  asset="$(sed -n 's/^ *asset="\(.*\)"$/\1/p' "$action" | head -n 1)"
  while [[ "$url" =~ \$\{([A-Za-z_][A-Za-z0-9_]*)\} ]]; do
    name="${BASH_REMATCH[1]}"
    if [ "$name" = asset ]; then value="$asset"; else value="$(env_value "$name" < "$action")"; fi
    [ -n "$value" ] || fail "$action: \${$name} has no value"
    url="${url//"\${$name}"/$value}"
  done
  printf '%s\n' "$url"
}

sha256() {
  if command -v sha256sum > /dev/null; then sha256sum "$1"; else shasum -a 256 "$1"; fi | cut -d' ' -f1
}

print_only=false
[ "${1:-}" = --print-urls ] && print_only=true

scratch="$(mktemp -d "${TMPDIR:-/tmp}/refresh-pin-checksums.XXXXXX")"
trap 'rm -rf "$scratch"' EXIT
# First every release is measured; only when all are in hand, and at least
# one version moved, is any digest written.
updates=()
for action in .github/actions/*/action.yml; do
  key="$(grep -oE '^ +[A-Z0-9_]+_SHA256:' "$action" | tr -d ' :' | head -n 1)"
  [ -n "$key" ] || continue
  url="$(release_url "$action")"
  if "$print_only"; then
    printf '%s %s\n' "$action" "$url"
    continue
  fi
  version_key="${key%_SHA256}_VERSION"
  version="$(env_value "$version_key" < "$action")"
  [ -n "$version" ] || fail "$action: no $version_key"
  was="$(git show "HEAD:$action" 2> /dev/null | env_value "$version_key" || true)"
  curl -sSfL --retry 3 -o "$scratch/release" "$url" || fail "$action: could not download $url"
  digest="$(sha256 "$scratch/release")"
  [ "${#digest}" -eq 64 ] || fail "$action: no sha256 for $url"
  if [ "$version" = "$was" ]; then
    recorded="$(env_value "$key" < "$action")"
    [ "$digest" = "$recorded" ] ||
      fail "$action: $version_key $version did not move, yet its release now hashes to $digest, not the recorded $recorded; refusing to re-pin it"
    continue
  fi
  updates+=("$action $key $digest")
done
"$print_only" && exit 0
[ "${#updates[@]}" -gt 0 ] || fail "no pinned version moved against HEAD, so there is no digest to refresh"
for update in "${updates[@]}"; do
  read -r action key digest <<< "$update"
  sed "s/^\( *$key: \)[0-9a-f]\{64\}$/\1$digest/" "$action" > "$scratch/action"
  cat "$scratch/action" > "$action"
  [ "$(env_value "$key" < "$action")" = "$digest" ] || fail "$action: $key was not rewritten"
  printf 'refresh-pin-checksums: %s %s\n' "$action" "$digest"
done
