#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
case "${1:-status}" in
  build) docker compose build engine ;;
  profile)
    # Only this named profile is installed. MAC_ADMIN loads it through the
    # host security filesystem; no privileged container or host sysctl.
    docker run --rm -i --user root --cap-add MAC_ADMIN \
      --security-opt apparmor=unconfined \
      --mount type=bind,src=/sys/kernel/security,dst=/sys/kernel/security \
      --mount type=bind,src=/etc/apparmor.d,dst=/host-profiles \
      --entrypoint bash brokkr-ci:local -c '
        set -euo pipefail
        apt-get update -qq
        apt-get install -y -qq --no-install-recommends apparmor
        cat > /host-profiles/brokkr-ci
        apparmor_parser --replace /host-profiles/brokkr-ci
      ' < brokkr-ci.apparmor ;;
  register)
    for service in engine coverage build; do
      # A one-hour registration token goes over stdin, never into a file,
      # Docker environment, shell trace, or the long-lived container.
      gh api --method POST repos/feedback-loop-ai/brokkr/actions/runners/registration-token --jq .token |
        docker compose run --rm --no-deps -T --user runner --entrypoint bash "$service" -c '
          set -euo pipefail
          read -r token
          cd /runner
          if [[ -f .runner ]]; then echo "$RUNNER_NAME is already registered"; exit 0; fi
          ./config.sh --unattended --url https://github.com/feedback-loop-ai/brokkr \
            --token "$token" --name "$RUNNER_NAME" --labels "$RUNNER_LABELS" --work _work
        '
    done ;;
  start) docker compose up -d --no-build ;;
  stop) docker compose stop ;;
  status) docker compose ps; gh api repos/feedback-loop-ai/brokkr/actions/runners --jq '.runners[] | {name,status,busy}' ;;
  logs) docker compose logs --tail 80 "${2:-engine}" ;;
  *) echo 'Usage: manage.sh build|profile|register|start|stop|status|logs [service]' >&2; exit 2 ;;
esac
