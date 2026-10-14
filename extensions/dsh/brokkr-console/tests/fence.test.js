// Every route and the one upgrade ask DSH's fence first, and a refused
// request reaches nothing: no handler body and no `brokkr` call.
import assert from 'node:assert/strict'
import { test } from 'node:test'

import { apply } from '../lib/index.js'
import { fleetDoc, json, request, row, stubBrokkr, stubHost, withEnv } from './support/world.js'

const ROUTES = ['/brokkr-console/api/command', '/brokkr-console/api/analytics', '/brokkr-console/api/here', '/brokkr-console/api/transcript', '/brokkr-console/console.css']

function applied(rejection) {
  const brokkr = stubBrokkr([{ match: ['runs'], responses: [json(fleetDoc([['brokkr', [row()]]]))] }])
  const host = stubHost(rejection)
  return withEnv({ BROKKR_CONSOLE_BROKKR: brokkr.program, BROKKR_STUB_DIR: brokkr.dir }, () => {
    apply(host.ctx)
    return { brokkr, host }
  })
}

for (const status of [401, 403]) {
  test(`every route answers DSH's ${status} and runs nothing`, async () => {
    const { brokkr, host } = await applied(status)
    assert.deepEqual([...host.routes.keys()].sort(), [...ROUTES].sort())
    for (const [path, { handler }] of host.routes) {
      const body = { command_id: 'a1b2c3d4-0000', realm: 'brokkr', run_id: row().run_id, command: 'stop', reason: 'why', expected_seq: 12 }
      const answer = await request(handler, { method: path.endsWith('command') ? 'POST' : 'GET', url: path, body })
      assert.deepEqual([path, answer.status, answer.body], [path, status, ''])
    }
    assert.deepEqual(brokkr.calls(), [])
  })

  test(`the upgrade answers DSH's ${status} before any handshake`, async () => {
    const { brokkr, host } = await applied(status)
    assert.deepEqual([...host.upgrades.keys()], ['/brokkr-console/ws'])
    const written = []
    const socket = { end: (text) => written.push(['end', text]), write: (text) => written.push(['write', text]), on: () => {} }
    const req = { headers: { upgrade: 'websocket', 'sec-websocket-key': 'dGhlIHNhbXBsZSBub25jZQ==' } }
    host.upgrades.get('/brokkr-console/ws')(req, socket, Buffer.alloc(0))
    const reason = status === 401 ? 'Unauthorized' : 'Forbidden'
    assert.deepEqual(written, [['end', `HTTP/1.1 ${status} ${reason}\r\nconnection: close\r\n\r\n`]])
    assert.deepEqual(brokkr.calls(), [])
  })
}

test('past the fence, a command with an empty reason is answered 400 and sends nothing', async () => {
  const { brokkr, host } = await applied(undefined)
  const body = { command_id: 'a1b2c3d4-0000', realm: 'brokkr', run_id: row().run_id, command: 'stop', reason: '', expected_seq: 12 }
  const answer = await request(host.routes.get('/brokkr-console/api/command').handler, { method: 'POST', body })
  assert.deepEqual([answer.status, JSON.parse(answer.body)], [400, { error: 'a reason is required' }])
  assert.deepEqual(brokkr.calls(), [])
})

test('an admitted upgrade completes the RFC 6455 handshake', async () => {
  const { host } = await applied(undefined)
  const written = []
  const socket = { end: (text) => written.push(['end', text]), write: (text) => written.push(['write', String(text)]), on: () => {} }
  const req = { headers: { upgrade: 'websocket', 'sec-websocket-key': 'dGhlIHNhbXBsZSBub25jZQ==' } }
  host.upgrades.get('/brokkr-console/ws')(req, socket, Buffer.alloc(0))
  assert.equal(written[0][1].split('\r\n')[3], 'Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=')
})
