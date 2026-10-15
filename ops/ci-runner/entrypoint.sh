#!/usr/bin/env bash
set -euo pipefail
if [[ ! -f /runner/.runner && $# -eq 0 ]]; then
  echo 'Register this runner with ops/ci-runner/manage.sh register first.' >&2
  exit 1
fi
# The broker deliberately refuses overlay filesystem identities, including
# ancestors of a file on a volume. Put the entire CI root on native storage.
# Only image files are copied; the host filesystem is never mounted here.
rsync -aH --delete --one-file-system \
  --exclude=/ci-root --exclude=/runner --exclude=/home \
  --exclude=/proc --exclude=/sys --exclude=/dev --exclude=/tmp --exclude=/run \
  / /ci-root/
mkdir -p /ci-root/{runner,home/runner,proc,sys,dev,tmp,run}
chmod 1777 /ci-root/tmp
for path in runner home/runner proc sys dev; do
  mount --rbind "/$path" "/ci-root/$path"
done
for path in hosts hostname; do
  mount --bind "/etc/$path" "/ci-root/etc/$path"
done
# Keep a real local resolver, as on the hosted Ubuntu runner. Its upstream
# is Docker's resolver inside this container's private network namespace.
upstream=$(awk '$1 == "nameserver" { print $2 }' /etc/resolv.conf | paste -sd ' ')
mkdir -p /ci-root/etc/systemd/resolved.conf.d
printf '[Resolve]\nDNS=%s\nFallbackDNS=\nLLMNR=no\nMulticastDNS=no\n' "$upstream" \
  > /ci-root/etc/systemd/resolved.conf.d/ci.conf
rm -f /ci-root/etc/resolv.conf
printf 'nameserver 127.0.0.53\noptions edns0 trust-ad\n' > /ci-root/etc/resolv.conf
# Test journals and disposable repositories need real filesystem semantics
# but no durability across jobs. Avoid making their fsyncs wait for NVMe's
# journal; compiler outputs and coverage caches stay on persistent volumes.
for path in tmp var/tmp; do
  mount -t tmpfs -o size=2g,mode=1777,nosuid,nodev tmpfs "/ci-root/$path"
done
mount -t tmpfs -o size=16m,mode=755,nosuid,nodev tmpfs /ci-root/run
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
mkdir -p /run/dbus /run/systemd
dbus-daemon --system --fork --nopidfile
setpriv --bounding-set=-sys_admin /usr/lib/systemd/systemd-resolved &
for _attempt in {1..100}; do
  if getent ahostsv4 _localdnsstub >/dev/null; then break; fi
  sleep 0.1
done
getent ahostsv4 _localdnsstub >/dev/null
if [[ $# -eq 0 ]]; then set -- /runner/run.sh; fi
# SYS_ADMIN is used only for the mounts above. Drop it from the bounding
# set before the runner starts; even sudo inside a job cannot regain it.
exec setpriv --bounding-set=-sys_admin --reuid=1001 --regid=1001 --init-groups "$@"
