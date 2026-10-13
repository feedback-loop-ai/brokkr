// The one WebSocket the console pushes on. DSH hands the upgrade over whole
// (task 0.2): the handshake and RFC 6455 framing are the plugin's, written
// against Node's standard library with no `ws` dependency (decision 0084
// ruling 4). The console's messages are small JSON texts; a client frame
// larger than the bound closes the socket.
import { createHash } from 'node:crypto'

const GUID = '258EAFA5-E914-47DA-95CA-C5AB0DC85B11'
const MESSAGE_BOUND_BYTES = 64 * 1024

const OP = { text: 0x1, close: 0x8, ping: 0x9, pong: 0xa }

/** One unmasked server frame. */
function frame(opcode, payload) {
  const length = payload.length
  let header
  if (length < 126) {
    header = Buffer.from([0x80 | opcode, length])
  } else if (length < 65536) {
    header = Buffer.alloc(4)
    header.writeUInt16BE(length, 2)
    header[1] = 126
  } else {
    header = Buffer.alloc(10)
    header.writeBigUInt64BE(BigInt(length), 2)
    header[1] = 127
  }
  header[0] = 0x80 | opcode
  return Buffer.concat([header, payload])
}

/** Where one frame's payload starts and how long it is, or `null` until it is whole. */
function bounds(buffer, at) {
  const masked = (buffer[at + 1] & 0x80) !== 0
  let length = buffer[at + 1] & 0x7f
  let start = at + 2
  if (length === 126) {
    if (buffer.length < start + 2) return null
    length = buffer.readUInt16BE(start)
    start += 2
  } else if (length === 127) {
    if (buffer.length < start + 8) return null
    length = Number(buffer.readBigUInt64BE(start))
    start += 8
  }
  const mask = masked ? buffer.subarray(start, start + 4) : null
  start += masked ? 4 : 0
  return { mask, start, length }
}

/** Every whole client frame `buffer` holds, and the bytes after them. */
function frames(buffer) {
  const found = []
  let at = 0
  while (buffer.length - at >= 2) {
    const frame = bounds(buffer, at)
    if (frame === null || buffer.length < frame.start + frame.length) break
    const payload = Buffer.from(buffer.subarray(frame.start, frame.start + frame.length))
    if (frame.mask !== null) payload.forEach((byte, index) => (payload[index] = byte ^ frame.mask[index % 4]))
    found.push({ opcode: buffer[at] & 0x0f, payload })
    at = frame.start + frame.length
  }
  return { found, rest: buffer.subarray(at) }
}

/**
 * Complete the handshake on `socket` and return a peer that sends JSON and
 * hands each JSON text it receives to `onMessage`; `null` for a request that
 * is not a WebSocket upgrade, which is answered 400.
 */
export function accept(req, socket, head, onMessage) {
  const key = req.headers['sec-websocket-key']
  if (String(req.headers.upgrade).toLowerCase() !== 'websocket' || !key) {
    socket.end('HTTP/1.1 400 Bad Request\r\nconnection: close\r\n\r\n')
    return null
  }
  const acceptKey = createHash('sha1').update(key + GUID).digest('base64')
  socket.write(
    ['HTTP/1.1 101 Switching Protocols', 'Upgrade: websocket', 'Connection: Upgrade', `Sec-WebSocket-Accept: ${acceptKey}`, '', ''].join('\r\n'),
  )
  const peer = {
    open: true,
    send(message) {
      if (peer.open) socket.write(frame(OP.text, Buffer.from(JSON.stringify(message))))
    },
    close() {
      if (peer.open) socket.end(frame(OP.close, Buffer.alloc(0)))
      peer.open = false
    },
    onClose: () => {},
  }
  let pending = Buffer.from(head ?? Buffer.alloc(0))
  socket.on('data', (chunk) => {
    pending = Buffer.concat([pending, chunk])
    if (pending.length > MESSAGE_BOUND_BYTES) return peer.close()
    const { found, rest } = frames(pending)
    pending = Buffer.from(rest)
    for (const { opcode, payload } of found) receive(peer, socket, opcode, payload, onMessage)
  })
  const closed = () => {
    peer.open = false
    peer.onClose()
  }
  socket.on('close', closed)
  socket.on('error', closed)
  return peer
}

function receive(peer, socket, opcode, payload, onMessage) {
  switch (opcode) {
    case OP.text:
      try {
        onMessage(JSON.parse(payload.toString('utf8')))
      } catch {
        peer.close()
      }
      return
    case OP.ping:
      socket.write(frame(OP.pong, payload))
      return
    case OP.close:
      peer.close()
      return
    default:
      return
  }
}
