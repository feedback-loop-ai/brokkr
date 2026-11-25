#!/usr/bin/env python3
# Drive `brokkr hands serve` over MCP stdio with no model: one tools/call per command given.
import json, subprocess, sys
brokkr, workdir, artifacts, reach, spec = sys.argv[1:6]
cmds = sys.argv[6:]
p = subprocess.Popen([brokkr, "hands", "serve", "--workdir", workdir, "--artifacts", artifacts,
                      "--reach", open(reach).read().strip(), "--spec", open(spec).read().strip()],
                     stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
def rpc(i, method, params=None, note=False):
    m = {"jsonrpc": "2.0", "method": method}
    if params is not None: m["params"] = params
    if not note: m["id"] = i
    p.stdin.write(json.dumps(m) + "\n"); p.stdin.flush()
    if note: return None
    while True:
        line = p.stdout.readline()
        if not line: return {"eof": p.stderr.read()[-600:]}
        r = json.loads(line)
        if r.get("id") == i: return r
print(json.dumps(rpc(1, "initialize", {"protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": "drive", "version": "0"}}))[:300])
rpc(0, "notifications/initialized", {}, note=True)
tools = rpc(2, "tools/list", {})
print("tools:", [t["name"] for t in tools.get("result", {}).get("tools", [])])
for n, c in enumerate(cmds, 3):
    r = rpc(n, "tools/call", {"name": "workspace", "arguments": {"command": c}})
    print("$", c); print(json.dumps(r.get("result", r))[:700])
p.stdin.close(); p.wait(timeout=20)
