// brokkr-console-probe, node (Host) half.
//
// A Cordis function plugin: `name`, `inject`, `apply(ctx)`. It registers on the
// composition's `webServer` service (@deepseek-ai/dsh-host-webserver):
//   GET  /brokkr-probe/ping     open route (no fence, no auth) -> JSON
//   GET  /brokkr-probe/fenced   same, but behind ctx.connection.requestRejection
//                               (the /api Host/Origin fence + browser login cookie)
//   GET  /brokkr-probe/report   beacon the browser half calls so client-side
//                               slot activity is observable in the Host log
//   UPGRADE /brokkr-probe/ws    hand-rolled RFC 6455 echo (no `ws` dependency)
import { createHash } from 'node:crypto'
import { appendFileSync } from 'node:fs'

export const name = 'brokkr-console-probe'
export const inject = ['webServer', 'connection']

const WS_GUID = '258EAFA5-E914-47DA-95CA-C5AB0DC85B11'

function log(line) {
  const text = `${new Date().toISOString()} [brokkr-console-probe] ${line}`
  console.log(text)
  if (process.env.BROKKR_PROBE_LOG) appendFileSync(process.env.BROKKR_PROBE_LOG, text + '\n')
}

function sendJson(res, status, body) {
  res.statusCode = status
  res.setHeader('content-type', 'application/json; charset=utf-8')
  res.setHeader('cache-control', 'no-store')
  res.end(JSON.stringify(body))
}

/** Encode one unmasked server frame. */
function frame(opcode, payload) {
  const len = payload.length
  let header
  if (len < 126) header = Buffer.from([0x80 | opcode, len])
  else if (len < 65536) {
    header = Buffer.alloc(4)
    header[0] = 0x80 | opcode
    header[1] = 126
    header.writeUInt16BE(len, 2)
  } else {
    header = Buffer.alloc(10)
    header[0] = 0x80 | opcode
    header[1] = 127
    header.writeBigUInt64BE(BigInt(len), 2)
  }
  return Buffer.concat([header, payload])
}

/** Parse as many complete client frames as `buf` holds; returns [frames, rest]. */
function parseFrames(buf) {
  const frames = []
  let off = 0
  while (buf.length - off >= 2) {
    const b0 = buf[off]
    const b1 = buf[off + 1]
    const opcode = b0 & 0x0f
    const masked = (b1 & 0x80) !== 0
    let len = b1 & 0x7f
    let p = off + 2
    if (len === 126) {
      if (buf.length < p + 2) break
      len = buf.readUInt16BE(p)
      p += 2
    } else if (len === 127) {
      if (buf.length < p + 8) break
      len = Number(buf.readBigUInt64BE(p))
      p += 8
    }
    const maskLen = masked ? 4 : 0
    if (buf.length < p + maskLen + len) break
    const mask = masked ? buf.subarray(p, p + 4) : null
    p += maskLen
    const payload = Buffer.from(buf.subarray(p, p + len))
    if (mask) for (let i = 0; i < payload.length; i++) payload[i] ^= mask[i % 4]
    frames.push({ opcode, payload })
    off = p + len
  }
  return [frames, buf.subarray(off)]
}

export function apply(ctx) {
  log(`apply: webServer listening on ${ctx.webServer.host}:${ctx.webServer.port}`)

  ctx.effect(() => ctx.webServer.register({
    kind: 'exact',
    path: '/brokkr-probe/ping',
    handler: (req, res) => {
      log(`GET /brokkr-probe/ping host=${req.headers.host} origin=${req.headers.origin ?? '-'}`)
      if (req.method !== 'GET') {
        res.statusCode = 405
        res.setHeader('allow', 'GET')
        res.end()
        return
      }
      sendJson(res, 200, { probe: 'brokkr-console-probe', route: '/brokkr-probe/ping', pid: process.pid, at: new Date().toISOString() })
    },
  }), 'brokkr-console-probe: GET /brokkr-probe/ping')

  ctx.effect(() => ctx.webServer.register({
    kind: 'exact',
    path: '/brokkr-probe/fenced',
    handler: (req, res) => {
      const rejection = ctx.connection.requestRejection(req)
      log(`GET /brokkr-probe/fenced host=${req.headers.host} origin=${req.headers.origin ?? '-'} rejection=${rejection ?? 'none'}`)
      if (rejection !== undefined) {
        res.statusCode = rejection
        res.end()
        return
      }
      sendJson(res, 200, { probe: 'brokkr-console-probe', route: '/brokkr-probe/fenced', authenticated: true })
    },
  }), 'brokkr-console-probe: GET /brokkr-probe/fenced')

  ctx.effect(() => ctx.webServer.register({
    kind: 'exact',
    path: '/brokkr-probe/report',
    handler: (req, res) => {
      const url = new URL(String(req.url), 'http://localhost')
      log(`client-report event=${url.searchParams.get('event')} detail=${url.searchParams.get('detail') ?? '-'}`)
      res.statusCode = 204
      res.end()
    },
  }), 'brokkr-console-probe: GET /brokkr-probe/report')

  ctx.effect(() => ctx.webServer.registerUpgrade({
    path: '/brokkr-probe/ws',
    handler: (req, socket, head) => {
      const key = req.headers['sec-websocket-key']
      log(`UPGRADE /brokkr-probe/ws host=${req.headers.host} origin=${req.headers.origin ?? '-'} upgrade=${req.headers.upgrade} key=${key ? 'present' : 'missing'}`)
      if (String(req.headers.upgrade).toLowerCase() !== 'websocket' || !key) {
        socket.end('HTTP/1.1 400 Bad Request\r\nconnection: close\r\n\r\n')
        return
      }
      const accept = createHash('sha1').update(key + WS_GUID).digest('base64')
      socket.write([
        'HTTP/1.1 101 Switching Protocols',
        'Upgrade: websocket',
        'Connection: Upgrade',
        `Sec-WebSocket-Accept: ${accept}`,
        '', '',
      ].join('\r\n'))
      socket.write(frame(0x1, Buffer.from('hello from brokkr-console-probe')))
      let pending = head && head.length ? Buffer.from(head) : Buffer.alloc(0)
      socket.on('data', (chunk) => {
        const [frames, rest] = parseFrames(Buffer.concat([pending, chunk]))
        pending = Buffer.from(rest)
        for (const f of frames) {
          if (f.opcode === 0x1) {
            log(`ws message: ${f.payload.toString('utf8')}`)
            socket.write(frame(0x1, Buffer.from('echo:' + f.payload.toString('utf8'))))
          } else if (f.opcode === 0x9) {
            socket.write(frame(0xA, f.payload))
          } else if (f.opcode === 0x8) {
            log('ws close')
            socket.end(frame(0x8, f.payload))
          }
        }
      })
      socket.on('error', () => {})
    },
  }), 'brokkr-console-probe: UPGRADE /brokkr-probe/ws')
}
