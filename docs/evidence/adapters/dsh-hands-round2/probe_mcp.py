# A minimal MCP stdio server: one tool, `ping`, answering "pong". Appends a line to argv[1] when it starts.
import json, os, sys
try:
    with open(sys.argv[1], 'a') as f:
        f.write(f"started pid={os.getpid()}\n")
except Exception:
    pass
for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    msg = json.loads(line)
    if 'id' not in msg:
        continue
    method, rid = msg.get('method'), msg['id']
    if method == 'initialize':
        result = {'protocolVersion': msg.get('params', {}).get('protocolVersion', '2025-06-18'),
                  'capabilities': {'tools': {}}, 'serverInfo': {'name': 'probe', 'version': '0.0.1'}}
    elif method == 'tools/list':
        result = {'tools': [{'name': 'ping', 'description': 'Answers pong.',
                             'inputSchema': {'type': 'object', 'properties': {}}}]}
    elif method == 'tools/call':
        result = {'content': [{'type': 'text', 'text': 'pong'}]}
    elif method == 'ping':
        result = {}
    else:
        print(json.dumps({'jsonrpc': '2.0', 'id': rid, 'error': {'code': -32601, 'message': 'no such method'}}), flush=True)
        continue
    print(json.dumps({'jsonrpc': '2.0', 'id': rid, 'result': result}), flush=True)
