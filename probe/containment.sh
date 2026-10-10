#!/usr/bin/env bash
# What this machine offers for containing a process tree (#472). Prints facts
# only; every probe runs whether or not an earlier one failed. Not for merge.
set -u
self=$(readlink -f "$0")
say() { printf '\n== %s\n' "$*"; }
try() { printf '$ %s\n' "$*"; ("$@") 2>&1 | head -20; printf '[rc=%s]\n' "${PIPESTATUS[0]}"; }
alive() { pgrep -fx "sleep $1" | wc -l; }

# A tree in a fresh child of cgroup $1 escapes by setsid and a double fork;
# one write to cgroup.kill must end all of it.
contain() {
  local base=$1 a=$1/probe-attempt-$$ n
  echo "base: $base"
  ls -ld "$base"
  echo "controllers: $(cat "$base/cgroup.controllers" 2>&1)"
  echo "subtree_control: $(cat "$base/cgroup.subtree_control" 2>&1)"
  if ! mkdir "$a" 2>&1; then echo "RESULT cgroup: cannot create a child cgroup"; return 0; fi
  bash -c 'echo $$ > "$1/cgroup.procs" || exit 7; exec setsid bash -c "(setsid sleep 3011 &); sleep 3011"' _ "$a" &
  sleep 1
  n=$(wc -l < "$a/cgroup.procs")
  echo "processes in the attempt: $n; live sleeps: $(alive 3011)"
  if [ "$n" -eq 0 ]; then echo "RESULT cgroup: cannot move a process into the child"; rmdir "$a"; wait; return 0; fi
  if [ -e "$a/cgroup.kill" ]; then echo 1 > "$a/cgroup.kill"; else echo "no cgroup.kill (kernel < 5.14)"; fi
  for _ in $(seq 50); do grep -q '^populated 0' "$a/cgroup.events" && break; sleep 0.1; done
  echo "events: $(tr '\n' ' ' < "$a/cgroup.events")"
  echo "RESULT cgroup: live sleeps after cgroup.kill: $(alive 3011)"
  wait 2>/dev/null; rmdir "$a" 2>&1
}

case "${1:-all}" in
contain-here)
  contain "/sys/fs/cgroup$(sed -n 's/^0:://p' /proc/self/cgroup)"
  exit 0 ;;
esac

say identity
try uname -srm
try id
if [ "$(uname -s)" = Darwin ]; then
  try sw_vers
  try sysctl -n kern.version
  say macOS mechanisms
  try command -v sandbox-exec
  try launchctl print "gui/$(id -u)"
  exit 0
fi

say cgroup v2
try stat -fc %T /sys/fs/cgroup
try cat /proc/self/cgroup
own="/sys/fs/cgroup$(sed -n 's/^0:://p' /proc/self/cgroup)"
try ls -ld "$own"
try cat /sys/fs/cgroup/cgroup.controllers
echo "XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR:-unset}"
try systemctl --user is-system-running
if [ "${PROBE_OWN:-0}" = 1 ]; then
  say "contain in the job's own cgroup"
  contain "$own"
fi
say contain in a user scope with Delegate=yes
try systemd-run --user --scope -p Delegate=yes bash "$self" contain-here
say contain in a system scope with Delegate=yes, through sudo
if sudo -n true 2>/dev/null; then
  try sudo -n systemd-run --scope -p Delegate=yes --uid="$(id -u)" --gid="$(id -g)" bash "$self" contain-here
else
  echo "no passwordless sudo"
fi

say PID namespaces
for k in kernel.apparmor_restrict_unprivileged_userns kernel.unprivileged_userns_clone user.max_user_namespaces; do
  try sysctl -n "$k"
done
try unshare --user --map-root-user --pid --fork --mount-proc bash -c '(setsid sleep 3012 &); sleep 0.3; echo "inside: pid $$"'
sleep 0.5
echo "RESULT unshare: live sleeps after the namespace init exits: $(alive 3012)"
try command -v bwrap
if command -v bwrap >/dev/null; then
  try bwrap --unshare-pid --die-with-parent --dev-bind / / bash -c '(setsid sleep 3013 &); sleep 0.3; echo "inside: pid $$"'
  sleep 0.5
  echo "RESULT bwrap: live sleeps after its init exits: $(alive 3013)"
fi
