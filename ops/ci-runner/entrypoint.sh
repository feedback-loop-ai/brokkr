#!/usr/bin/env bash
set -euo pipefail
cd /runner
if [[ ! -f .runner ]]; then
  echo 'Register this runner with ops/ci-runner/manage.sh register first.' >&2
  exit 1
fi
exec ./run.sh
