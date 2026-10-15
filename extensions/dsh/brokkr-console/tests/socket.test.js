// The console's WebSocket reads only masked client frames (RFC 6455 5.1),
// and once it has closed, it reads and holds nothing more.
import assert from 'node:assert/strict'
import { EventEmitter } from 'node:events'
import { test } from 'node:test'

import { accept } from '../lib/host/socket.js'
import { clientFrame, WS_KEY as KEY } from './support/world.js'

/** An accepted peer on a fake socket, with what it wrote and what it heard. */
function opened() {
  const socket = new EventEmitter()
  const written = []
  socket.write = (bytes) => written.push(['write', Buffer.from(bytes)])
  socket.end = (bytes) => written.push(['end', Buffer.from(bytes ?? '')])
  const heard = []
  const peer = accept({ headers: { upgrade: 'websocket', 'sec-websocket-key': KEY } }, socket, Buffer.alloc(0), (message) => heard.push(message))
  return { socket, peer, written, heard }
}

const closedWith = (written) => written.some(([kind, bytes]) => kind === 'end' && (bytes[0] & 0x0f) === 0x8)

test('a masked text frame is heard', () => {
  const { socket, heard, peer } = opened()
  socket.emit('data', clientFrame('{"watch":"realms"}'))
  assert.deepEqual(heard, [{ watch: 'realms' }])
  assert.equal(peer.open, true)
})

test('an unmasked client frame closes the connection and is not heard', () => {
  const { socket, heard, peer, written } = opened()
  socket.emit('data', clientFrame('{"watch":"realms"}', false))
  assert.deepEqual(heard, [])
  assert.equal(peer.open, false)
  assert.ok(closedWith(written))
})

test('after the close, later bytes are neither heard nor held', () => {
  const { socket, heard, peer } = opened()
  socket.emit('data', clientFrame('{}', false))
  assert.equal(peer.open, false)
  socket.emit('data', clientFrame('{"late":true}'))
  socket.emit('data', Buffer.alloc(2 * 1024 * 1024, 0x81))
  assert.deepEqual(heard, [])
})

test('a frame after one that closed the connection in the same chunk is not heard', () => {
  const { socket, heard } = opened()
  const close = Buffer.from([0x88, 0x80, 1, 2, 3, 4])
  socket.emit('data', Buffer.concat([close, clientFrame('{"after":true}')]))
  assert.deepEqual(heard, [])
})
