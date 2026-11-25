# A stdio MCP sentinel: logs that it was started (and by which scope label), serves one tool.
import json,sys,os
LOG=os.environ.get("SENTINEL_LOG","/dev/null"); LABEL=os.environ.get("SENTINEL_LABEL","?")
def log(m):
    with open(LOG,"a") as f: f.write(LABEL+" "+m+"\n")
log("started")
for line in sys.stdin:
    line=line.strip()
    if not line: continue
    try: m=json.loads(line)
    except Exception: continue
    i=m.get("id"); meth=m.get("method")
    if i is None: continue
    if meth=="initialize":
        r={"protocolVersion":m.get("params",{}).get("protocolVersion","2024-11-05"),"capabilities":{"tools":{}},"serverInfo":{"name":"sentinel-"+LABEL,"version":"0.0.1"}}
    elif meth=="tools/list":
        r={"tools":[{"name":"sentinel_nonce","description":"Returns the sentinel nonce. Call this when asked for the sentinel nonce.","inputSchema":{"type":"object","properties":{}}}]}
    elif meth=="tools/call":
        log("called"); r={"content":[{"type":"text","text":"SENTINEL-NONCE-"+LABEL.upper()}]}
    else:
        sys.stdout.write(json.dumps({"jsonrpc":"2.0","id":i,"error":{"code":-32601,"message":"no such method"}})+"\n"); sys.stdout.flush(); continue
    sys.stdout.write(json.dumps({"jsonrpc":"2.0","id":i,"result":r})+"\n"); sys.stdout.flush()
