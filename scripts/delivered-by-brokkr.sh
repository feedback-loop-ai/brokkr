#!/usr/bin/env bash
# delivered by brokkr — the contribution gate.
#
# Decision 0033 rulings 1–5, as amended by decision 0038 rulings 2, 3 and
# 6: a pull request names the completed run that delivered it; the run's
# published anchor is verified offline; and the tier is cut by what
# changed since that judgment, not by the commit id the judgment named.
#
#   empty delta   the head's per-file patch map equals the vouched one
#                 (or the head IS the vouched head) → vouched
#   docs delta    every differing path is in the docs class → a completed
#                 `preflight` run over this head, named as Brokkr-Preflight
#   code delta    anything else → a new run vouches for the head, or the
#                 operator's by-hand label skips the gate
#
# The by-hand label skips the gate at every tier, and the tier it would
# have applied is logged so the label's use is countable (ruling 6).
#
# Inputs, all environment:
#   PR_BODY    the pull request body
#   PR_HEAD    the pull request head commit id
#   PR_BASE    a revision naming the base branch's tip, resolvable in REPO
#   REPO       a checkout holding PR_HEAD and PR_BASE with enough history
#              for their merge-base
#   EVIDENCE   a git URL or path serving refs/heads/brokkr-runs/<run id>
#   VERIFIER   the brokkr binary that verifies journals offline
#   LABELS     comma-separated pull request labels (optional)
#   CLASSES    the delivery-classes file (default: beside this script's tree)
set -euo pipefail

: "${PR_BODY?}" "${PR_HEAD:?}" "${PR_BASE:?}" "${REPO:?}" "${EVIDENCE:?}" "${VERIFIER:?}"
LABELS="${LABELS:-}"
CLASSES="${CLASSES:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/.github/delivery-classes.json}"

work="$(mktemp -d "${TMPDIR:-/tmp}/delivered-by-brokkr.XXXXXX")"
trap 'rm -rf "$work"' EXIT
git init -q --bare "$work/evidence"

say() { printf 'delivered by brokkr: %s\n' "$*"; }
fail() { printf 'delivered by brokkr: %s\n' "$*" >&2; exit 1; }

by_hand=0
case ",${LABELS}," in *,by-hand,*) by_hand=1 ;; esac

# The run id a `<Key>: <id>` line declares; nothing when the line is
# absent; a refusal when it is malformed or repeated.
declaration() {
  local key="$1"
  mapfile -t lines < <(printf '%s\n' "$PR_BODY" | sed -n "/^${key}:/p")
  (( ${#lines[@]} <= 1 )) || fail "expected at most one ${key}: line in the pull request body"
  (( ${#lines[@]} == 1 )) || return 0
  local line="${lines[0]%$'\r'}"
  [[ "$line" =~ ^${key}:\ ([a-z0-9-]{0,32}-[0-9a-f]{8})$ ]] \
    || fail "${key} must carry a native run id: [a-z0-9-]{0,32}-[0-9a-f]{8}"
  printf '%s' "${BASH_REMATCH[1]}"
}

# The published evidence of one run: its canonical journal and the anchor
# message, fetched into a scratch repository so the checkout under
# judgment is never made shallow.
fetch_evidence() {
  local run="$1" ref="refs/brokkr-evidence/$1"
  git -C "$work/evidence" fetch -q --no-tags "$EVIDENCE" "refs/heads/brokkr-runs/${run}:${ref}" \
    || fail "no published evidence for run ${run} at ${EVIDENCE} (push refs/forge/${run} to refs/heads/brokkr-runs/${run})"
  git -C "$work/evidence" show "${ref}:${run}.ndjson" > "$work/${run}.ndjson"
  git -C "$work/evidence" show -s --format=%B "$ref" | sed '/^$/d' > "$work/${run}.anchor.json"
}

# The journal verifies offline as this run, completed; the anchor names
# its head; the anchor's version is one this gate reads.
verify_evidence() {
  local run="$1" verified journal_head journal_seq
  verified="$("$VERIFIER" verify-run "$work/${run}.ndjson")"
  printf '%s\n' "$verified" | jq -e --arg run "$run" \
    '.chain == "verified" and .state.run_id == $run and .state.status == "completed"' >/dev/null \
    || fail "run ${run}: the journal does not verify as this run, completed"
  journal_head="$(tail -n 1 "$work/${run}.ndjson" | jq -er '.event_hash')"
  journal_seq="$(tail -n 1 "$work/${run}.ndjson" | jq -er '.seq')"
  jq -e --arg run "$run" --arg journal "$journal_head" --argjson seq "$journal_seq" \
    '(.anchor | IN("forge.journal-anchor/v2", "forge.journal-anchor/v3")) and
     .run_id == $run and .seq == $seq and .journal_head_hash == $journal' \
    "$work/${run}.anchor.json" >/dev/null \
    || fail "run ${run}: the anchor does not match its journal, or names a version this gate does not read"
}

# Decision 0038 ruling 1's map, computed for the head under judgment
# exactly as the engine computes it at ship: per path, the stable patch
# id of that file's diff from the merge-base. Renames are never paired,
# so a moved file is a deletion and an addition and no path leaves the
# map by being moved.
patch_map() {
  local base="$1" head="$2" path id
  {
    git -C "$REPO" diff --no-renames --name-only "$base" "$head" | while IFS= read -r path; do
      id="$(git -C "$REPO" diff --no-renames "$base" "$head" -- "$path" | git patch-id --stable | cut -d' ' -f1)"
      jq -n --arg path "$path" --arg id "$id" '{($path): $id}'
    done
  } | jq -s 'add // {}'
}

run_id="$(declaration Brokkr-Run)"
tier=unknown
delta='[]'
if [[ -z "$run_id" ]]; then
  (( by_hand )) || fail "expected exactly one Brokkr-Run: line in the pull request body"
else
  fetch_evidence "$run_id"
  verify_evidence "$run_id"
  jq -s -e 'any(.[]; .type == "transition/decided" and
    .payload.from == "ship" and .payload.result == "shipped")' "$work/${run_id}.ndjson" >/dev/null \
    || fail "run ${run_id} never ruled shipped"
  anchor="$work/${run_id}.anchor.json"
  vouched_head="$(jq -r '.repo_head // ""' "$anchor")"
  if [[ "$vouched_head" == "$PR_HEAD" ]]; then
    tier=vouched
  elif jq -e '.anchor == "forge.journal-anchor/v3" and (.patch | type) == "object"' "$anchor" >/dev/null; then
    base="$(git -C "$REPO" merge-base "$PR_BASE" "$PR_HEAD")"
    patch_map "$base" "$PR_HEAD" > "$work/head.patch.json"
    jq '.patch' "$anchor" > "$work/vouched.patch.json"
    delta="$(jq -c -n --slurpfile vouched "$work/vouched.patch.json" --slurpfile head "$work/head.patch.json" \
      '[ ((($vouched[0] | keys) + ($head[0] | keys)) | unique[]) as $path
         | select($vouched[0][$path] != $head[0][$path]) | $path ]')"
    if [[ "$delta" == "[]" ]]; then
      tier=vouched
    else
      # One jq ruling, not a pipeline: a path that matches no docs
      # pattern makes the whole delta code (ruling 3).
      docs_pattern="$(jq -r '.classes.docs.paths | join("|")' "$CLASSES")"
      if jq -e --arg docs "$docs_pattern" 'all(.[]; test($docs))' <<<"$delta" >/dev/null; then
        tier=docs
      else
        tier=code
      fi
    fi
  else
    # A v2 anchor carries no patch identity: the head must be the one it names.
    tier=code
  fi
fi

say "tier ${tier} · delta since the judgment: ${delta}"
if (( by_hand )); then
  say "skipped — operator applied the by-hand label (the tier would have been ${tier})"
  exit 0
fi

case "$tier" in
  vouched)
    say "run ${run_id} vouches for ${PR_HEAD}"
    ;;
  docs)
    preflight="$(declaration Brokkr-Preflight)"
    [[ -n "$preflight" ]] || fail "docs delta since run ${run_id}: ${delta}. Light \`preflight\` on ${PR_HEAD} and name it as Brokkr-Preflight: <run id>"
    fetch_evidence "$preflight"
    verify_evidence "$preflight"
    jq -s -e '[.[] | select(.type == "transition/decided")] | last
      | .payload.from == "review" and .payload.next == "done"' "$work/${preflight}.ndjson" >/dev/null \
      || fail "preflight ${preflight}: its last ruling is not a review ruling into done"
    jq -e --arg head "$PR_HEAD" '.repo_head == $head' "$work/${preflight}.anchor.json" >/dev/null \
      || fail "preflight ${preflight} judged $(jq -r '.repo_head' "$work/${preflight}.anchor.json"), not ${PR_HEAD}"
    say "run ${run_id} delivered the slice; preflight ${preflight} judged ${PR_HEAD}"
    ;;
  code)
    fail "code delta since run ${run_id}: ${delta}. A new run must vouch for ${PR_HEAD}, or the operator applies the by-hand label"
    ;;
esac
