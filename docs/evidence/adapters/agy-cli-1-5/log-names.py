#!/usr/bin/env python3
# PreToolUse on every tool: allow, and log the name the CLI passes.
import json,sys
try:
    d=json.load(sys.stdin); open("/var/tmp/probe-agy/hooks.log","a").write("hook saw tool: %s\n"%d["toolCall"]["name"])
except Exception: pass
print(json.dumps({"decision":"allow"}))
