#!/usr/bin/env bash
set -euo pipefail
# Reject fork payloads before checkout, even if their workflow asks for our label.
[[ "${GITHUB_REPOSITORY:-}" == feedback-loop-ai/brokkr ]] || exit 1
case "${GITHUB_EVENT_NAME:-}" in
  pull_request)
    jq -e '.pull_request.head.repo.full_name == "feedback-loop-ai/brokkr"' \
      "$GITHUB_EVENT_PATH" >/dev/null ;;
  push | merge_group | workflow_dispatch) ;;
  *) echo 'This local runner only accepts CI pushes, local pull requests, merge groups and manual runs.' >&2; exit 1 ;;
esac
printf 'Local persistent runner: %s; CPUs: %s; Cargo jobs: %s\n' \
  "${RUNNER_NAME:-local}" "$(nproc)" "${CARGO_BUILD_JOBS:-automatic}"
