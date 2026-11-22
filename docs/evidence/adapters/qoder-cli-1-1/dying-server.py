# A stdio MCP server that offers one `workspace` tool, answers its first tools/call, then exits: a server that dies mid-turn.
import json,sys
calls=0
for line in sys.stdin:
    line=line.strip()
    if not line: continue
    try: m=json.loads(line)
    except Exception: continue
    i=m.get("id"); meth=m.get("method")
    if i is None: continue
    if meth=="initialize":
        r={"protocolVersion":m.get("params",{}).get("protocolVersion","2024-11-05"),"capabilities":{"tools":{}},"serverInfo":{"name":"brokkr-hands","version":"0"}}
    elif meth=="tools/list":
        r={"tools":[{"name":"workspace","description":"Run a command in the workspace.","inputSchema":{"type":"object","properties":{"command":{"type":"string"}},"required":["command"]}}]}
    elif meth=="tools/call":
        calls+=1; r={"content":[{"type":"text","text":"first call answered\n\n[exit code: 0]"}]}
    else:
        sys.stdout.write(json.dumps({"jsonrpc":"2.0","id":i,"error":{"code":-32601,"message":"no such method"}})+"\n"); sys.stdout.flush(); continue
    sys.stdout.write(json.dumps({"jsonrpc":"2.0","id":i,"result":r})+"\n"); sys.stdout.flush()
    if calls==1: sys.exit(0)
