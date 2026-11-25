#!/usr/bin/env python3
# PreToolUse on every tool: allow only the listed names, deny the rest, log each name. Any error denies.
import json,sys
ALLOW={"view_file","write_to_file","replace_file_content","multi_replace_file_content","list_dir","find_by_name","grep_search","run_command","command_status","call_mcp_tool"}
try:
    d=json.load(sys.stdin); name=d["toolCall"]["name"]
    open("/var/tmp/probe-agy/hooks.log","a").write("allowlist saw tool: %s\n"%name)
    print(json.dumps({"decision":"allow"} if name in ALLOW else {"decision":"deny","reason":"engine hook: tool %s is switched off"%name}))
except Exception as e:
    print(json.dumps({"decision":"deny","reason":"engine hook failed closed"}))
