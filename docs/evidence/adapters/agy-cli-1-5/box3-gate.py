#!/usr/bin/env python3
# A boxed seat's gate: every native tool is denied by name; only the engine's workspace tool passes.
import json,sys
try:
    d=json.load(sys.stdin); name=d["toolCall"]["name"]; args=d["toolCall"].get("args",{})
    open("/var/tmp/probe-agy/hooks.log","a").write("box3 gate saw tool: %s args-keys: %s\n"%(name,sorted(args.keys()) if isinstance(args,dict) else type(args).__name__))
    ok = name=="mcp_brokkr_workspace" or (name=="call_mcp_tool" and args.get("ServerName")=="brokkr")
    print(json.dumps({"decision":"deny_unless_prior_grant"} if ok else {"decision":"deny","reason":"engine gate: tool %s is switched off"%name}))
except Exception:
    print(json.dumps({"decision":"deny","reason":"engine gate failed closed"}))
