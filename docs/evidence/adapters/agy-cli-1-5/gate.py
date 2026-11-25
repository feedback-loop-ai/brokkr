#!/usr/bin/env python3
# The engine's gate as one PreToolUse hook on every tool: a tool outside the list is denied by name;
# a listed tool is left to the permission rules, and what no rule grants is denied instead of asked.
import json,sys
LISTED={"read_file","write_file","edit_file","run_command","manage_task"}
try:
    d=json.load(sys.stdin); name=d["toolCall"]["name"]
    open("/var/tmp/probe-agy/hooks.log","a").write("gate saw tool: %s\n"%name)
    print(json.dumps({"decision":"deny_unless_prior_grant"} if name in LISTED else {"decision":"deny","reason":"engine gate: tool %s is switched off"%name}))
except Exception:
    print(json.dumps({"decision":"deny","reason":"engine gate failed closed"}))
