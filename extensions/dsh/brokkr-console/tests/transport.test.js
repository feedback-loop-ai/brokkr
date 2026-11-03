// The transport judges the bytes it sends: a real socket.accept peer over a
// recording EventEmitter, attached to the real poller, refuses a message whose
// serialized form carries world data even when its live object shows another
// type. The recording peer in poller.test.js never serializes, so these cases
// bind the boundary it cannot.
import assert from 'node:assert/strict'
import { EventEmitter } from 'node:events'
import { readFileSync, readdirSync } from 'node:fs'
import { join } from 'node:path'
import { test } from 'node:test'
import { fileURLToPath } from 'node:url'

import { brokkrOf } from '../lib/host/brokkr.js'
import { pollerOf } from '../lib/host/poller.js'
import { fleetOf } from '../lib/host/readouts.js'
import { accept } from '../lib/host/socket.js'
import * as socket from '../lib/host/socket.js'
import { guardTransport, PeerRegistered, WORLD_DATA_TYPES, WorldDataRefused } from '../lib/host/transport.js'
import { fleetAt } from './support/poller.js'
import { stubBrokkr, WS_KEY } from './support/world.js'

const LIB = fileURLToPath(new URL('../lib', import.meta.url))

/** Every source file under `dir`, recursively. */
function sources(dir) {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) =>
    entry.isDirectory() ? sources(join(dir, entry.name)) : [join(dir, entry.name)])
}

/** The text of one server frame `accept` wrote. */
function frameText(frame) {
  const short = frame[1] & 0x7f
  const at = short < 126 ? 2 : short === 126 ? 4 : 10
  return frame.subarray(at).toString('utf8')
}

/**
 * A real accepted peer on a recording socket, registered with the real poller
 * and no watch requested; its Brokkr proxy counts every world read.
 */
function registered() {
  const reads = []
  const brokkr = new Proxy(brokkrOf(stubBrokkr([]).config), {
    get(target, name) {
      const value = target[name]
      return typeof value === 'function' ? (...args) => { reads.push(name); return value(...args) } : value
    },
  })
  const poller = pollerOf({ brokkr, pollMs: 60_000 })
  const socket = new EventEmitter()
  const written = []
  socket.write = (bytes) => written.push(bytes)
  socket.end = () => {}
  let receive = () => {}
  const accepted = accept({ headers: { upgrade: 'websocket', 'sec-websocket-key': WS_KEY } }, socket, Buffer.alloc(0), (message) => receive(message))
  receive = poller.attach(accepted.peer, accepted.takeWriter)
  const frames = () => written.filter((bytes) => Buffer.isBuffer(bytes)).map(frameText)
  return { peer: accepted.peer, poller, frames, reads }
}

/** Assert `run` refuses with the named variant and `type`, and that nothing was read. */
function refused(run, type, reads) {
  assert.throws(run, (error) => {
    assert.equal(error.name, 'WorldDataRefused')
    assert.equal(error.type, type)
    return error instanceof WorldDataRefused
  })
  assert.deepEqual(reads, [])
}

test('M1: a toJSON fleet send outside publish is refused at the transport', () => {
  const { peer, frames, reads } = registered()
  const fleet = fleetOf(JSON.parse(fleetAt(12).stdout))
  refused(() => peer.send({ toJSON: () => ({ type: 'fleet', fleet }) }), 'fleet', reads)
  assert.deepEqual(frames(), [])
})

test('M1 residual: toJSON publishes an unminted overview on a registered real socket', () => {
  const { peer, frames, reads } = registered()
  refused(() => peer.send({ toJSON: () => ({ type: 'overview', overview: { stale: true } }) }), 'overview', reads)
  assert.deepEqual(frames(), [])
})

test('M1: a changing type getter is refused at the real transport', () => {
  const { peer, frames, reads } = registered()
  let seen = 0
  const changing = { get type() { return seen++ === 0 ? 'error' : 'overview' } }
  refused(() => peer.send(changing), 'overview', reads)
  assert.deepEqual(frames(), [])
})

test('the plain-type refusals are kept on the real transport', () => {
  const { peer, frames, reads } = registered()
  for (const type of WORLD_DATA_TYPES) refused(() => peer.send({ type }), type, reads)
  assert.deepEqual(frames(), [])
})

test('a message the transport cannot judge is refused, fail closed', () => {
  const { peer, frames, reads } = registered()
  const circular = {}
  circular.self = circular
  for (const message of ['a string', 42, null, undefined, circular, 1n]) {
    assert.throws(() => peer.send(message), (error) => error instanceof WorldDataRefused && error.type === null)
  }
  assert.deepEqual(frames(), [])
  assert.deepEqual(reads, [])
})

test('the plain-error control reaches the wire unchanged', () => {
  const { peer, frames, reads } = registered()
  const message = { type: 'error', message: 'Brokkr refused the read' }
  peer.send(message)
  assert.deepEqual(frames(), [JSON.stringify(message)])
  assert.deepEqual(reads, [])
})

test('the wire is the one serialization the transport judged, not a second read', () => {
  const { peer, frames, reads } = registered()
  let bodies = 0
  peer.send({ type: 'error', get message() { return bodies++ === 0 ? 'first' : 'second' } })
  assert.deepEqual(frames(), ['{"type":"error","message":"first"}'])
  assert.equal(bodies, 1)
  assert.deepEqual(reads, [])
})

test('H1-A: exported text writer publishes an unminted overview on a registered real socket', () => {
  const { peer, frames, reads } = registered()
  const overview = JSON.stringify({ type: 'overview', overview: { stale: true } })
  const exported = socket.sendText
  if (exported !== undefined) assert.throws(() => exported(peer, overview), (error) => error instanceof WorldDataRefused)
  assert.equal(socket.sendText, undefined, 'socket.js exports no raw writer')
  assert.deepEqual(frames(), [])
  assert.deepEqual(reads, [])
})

test('H1-B: re-guarding a registered real socket grants an unminted world-data sender', () => {
  const { peer, frames, reads } = registered()
  const again = guardTransport(peer)
  assert.equal(again, peer.send, 'a second guard returns the same facade')
  refused(() => again({ type: 'overview', overview: { stale: true } }), 'overview', reads)
  assert.deepEqual(frames(), [])
})

test('registering a peer twice refuses and grants nothing', () => {
  const { peer, poller, frames, reads } = registered()
  assert.throws(() => poller.attach(peer), (error) => {
    assert.ok(error instanceof PeerRegistered)
    assert.equal(error.name, 'PeerRegistered')
    return true
  })
  assert.deepEqual(frames(), [])
  assert.deepEqual(reads, [])
})

test('the raw writer is named only by socket.js, which exports no writer', () => {
  const named = sources(LIB).filter((file) => /sendText|textSends|writeText/.test(readFileSync(file, 'utf8')))
  assert.deepEqual(named.map((file) => file.slice(LIB.length + 1)), ['host/socket.js'])
  assert.deepEqual(Object.keys(socket).filter((name) => /writ|send/i.test(name)), [])
})

test('L2: WorldDataRefused’s complete operator text is pinned exactly', () => {
  const { peer } = registered()
  assert.throws(() => peer.send({ type: 'overview' }), (error) => {
    assert.equal(error.name, 'WorldDataRefused')
    assert.equal(error.type, 'overview')
    assert.equal(error.message, 'a “overview” message carries world data and may only be sent by publish')
    return true
  })
  assert.throws(() => peer.send('not a message'), (error) => {
    assert.equal(error.name, 'WorldDataRefused')
    assert.equal(error.type, null)
    assert.equal(error.message, 'a message that is not one stable typed JSON object may only be sent by publish')
    return true
  })
})

test('L3: a getter that answers error twice then overview paints the judged bytes, not a re-serialization', () => {
  const { peer, frames, reads } = registered()
  let seen = 0
  const changing = { get type() { return seen++ < 2 ? 'error' : 'overview' } }
  peer.send(changing)
  assert.deepEqual(frames(), ['{"type":"error"}'])
  assert.equal(seen, 2, 'the object is read once to serialize and once to live-check it')
  assert.deepEqual(reads, [])
})
