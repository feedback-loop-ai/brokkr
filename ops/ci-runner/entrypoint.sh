#!/usr/bin/env bash
set -euo pipefail
if [[ ! -f /runner/.runner && $# -eq 0 ]]; then
  echo 'Register this runner with ops/ci-runner/manage.sh register first.' >&2
  exit 1
fi
# The broker deliberately refuses overlay filesystem identities, including
# ancestors of a file on a volume. Put the entire CI root on native storage.
# Only image files are copied; the host filesystem is never mounted here.
rsync -a --delete --one-file-system \
  --exclude=/ci-root --exclude=/runner --exclude=/home \
  --exclude=/proc --exclude=/sys --exclude=/dev --exclude=/tmp --exclude=/run \
  / /ci-root/
mkdir -p /ci-root/{runner,home/runner,proc,sys,dev,tmp,run}
chmod 1777 /ci-root/tmp
for path in runner home/runner proc sys dev; do
  mount --rbind "/$path" "/ci-root/$path"
done
for path in resolv.conf hosts hostname; do
  mount --bind "/etc/$path" "/ci-root/etc/$path"
done
if [[ ! -s /runner/.machine-id ]]; then
  tr -d '-' < /proc/sys/kernel/random/uuid > /runner/.machine-id
fi
install -m 0444 /runner/.machine-id /ci-root/etc/machine-id
export HOME=/home/runner USER=runner LOGNAME=runner RUNNER_MANUALLY_TRAP_SIG=1
mount --make-rprivate /
cd /ci-root
mkdir -p .old-root
pivot_root . .old-root
umount -l /.old-root
rmdir /.old-root
cd /runner
if [[ $# -eq 0 ]]; then set -- /runner/run.sh; fi
# SYS_ADMIN is used only for the mounts above. Drop it from the bounding
# set before the runner starts; even sudo inside a job cannot regain it.
exec setpriv --bounding-set=-sys_admin --reuid=1001 --regid=1001 --init-groups "$@"
