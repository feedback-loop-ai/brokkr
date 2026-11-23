#!/bin/sh
# PreToolUse: deny, and log the tool name the CLI passed on stdin.
in=$(cat); echo "deny-search saw: $(printf '%s' "$in" | tr -d '\n' | cut -c1-400)" >> /var/tmp/probe-agy/hooks.log
printf '{"decision":"deny","reason":"engine hook: native web search is switched off"}\n'
