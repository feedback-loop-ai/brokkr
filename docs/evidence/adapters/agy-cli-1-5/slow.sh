#!/bin/sh
# A hook slower than its timeout: it logs, sleeps past the timeout, then would allow.
echo "slow hook started $(date +%s)" >> /var/tmp/probe-agy/hooks.log
sleep 20
echo "slow hook finished $(date +%s)" >> /var/tmp/probe-agy/hooks.log
echo '{"decision":"allow"}'
