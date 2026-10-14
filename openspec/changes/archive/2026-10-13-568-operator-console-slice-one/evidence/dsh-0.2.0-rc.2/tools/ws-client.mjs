// Node 22 built-in WebSocket client (undici). Usage: node ws-client.mjs <url>
const url = process.argv[2]
const ws = new WebSocket(url)
const t0 = Date.now()
const got = []
ws.onopen = () => console.log(`open (+${Date.now() - t0}ms) readyState=${ws.readyState}`)
ws.onmessage = (m) => {
  console.log(`message: ${m.data}`)
  got.push(m.data)
  if (got.length === 1) { ws.send('ping-from-node-ws-client'); console.log('sent: ping-from-node-ws-client') }
  else ws.close(1000, 'done')
}
ws.onerror = (e) => { console.log('error', e.message ?? e.type); process.exitCode = 1 }
ws.onclose = (e) => console.log(`close code=${e.code} reason=${e.reason}`)
setTimeout(() => { console.log('timeout'); process.exit(2) }, 5000).unref()
