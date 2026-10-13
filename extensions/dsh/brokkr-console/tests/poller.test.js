// Live refresh (design D5): a run is re-read only when its head moved, only
// for the client that has it open; the overview's readouts go only to a
// client on the Realms view; a client that reconnects gets a full snapshot.
import assert from 'node:assert/strict'
import { test } from 'node:test'

import { brokkrOf } from '../lib/host/brokkr.js'
import { pollerOf } from '../lib/host/poller.js'
import { analyticsDoc, fleetDoc, json, row, runViewDoc, stubBrokkr } from './support/world.js'

const RUN = row().run_id
const OTHER = 'lane-detection-v2-77aa01bc'

const fleetAt = (seq, other = 3) => json(fleetDoc([['brokkr', [row({ seq }), row({ run_id: OTHER, seq: other, title: 'Lane detection v2' })]]]))

function polled(fleets) {
  const brokkr = stubBrokkr([
    { match: ['runs'], responses: fleets },
    { match: ['inspect'], responses: [json(runViewDoc())] },
    { match: ['queue'], responses: [json([])] },
    { match: ['muninn'], responses: [json([])] },
    { match: ['analytics'], responses: [json(analyticsDoc())] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const peer = () => {
    const sent = []
    return { sent, send: (message) => sent.push(message), onClose: () => {} }
  }
  /** Wait until `client` has been sent its snapshot. */
  const settle = async (client) => {
    for (let waited = 0; client.sent.length === 0; waited += 50) {
      assert.ok(waited < 10_000, 'no snapshot within 10s')
      await new Promise((wake) => setTimeout(wake, 50))
    }
  }
  return { poller, peer, settle, calls: () => brokkr.calls().map((argv) => argv[0]) }
}

test('a run is re-read only when its head moved, and only for the client that has it open', async () => {
  const { poller, peer, settle, calls } = polled([fleetAt(12), fleetAt(12), fleetAt(12, 4), fleetAt(13, 4)])
  const open = peer()
  const elsewhere = peer()
  poller.attach(open)({ type: 'watch', view: 'realm', run: { realm: 'brokkr', run_id: RUN } })
  await settle(open)
  poller.attach(elsewhere)({ type: 'watch', view: 'analytics', run: null })
  await settle(elsewhere)
  const before = calls().filter((verb) => verb === 'inspect').length
  open.sent.length = 0
  elsewhere.sent.length = 0

  await poller.tick() // only the other run moved
  assert.equal(calls().filter((verb) => verb === 'inspect').length, before)
  assert.deepEqual(open.sent.map((message) => message.type), ['fleet'])

  await poller.tick() // the open run moved
  assert.equal(calls().filter((verb) => verb === 'inspect').length, before + 1)
  assert.deepEqual(open.sent.map((message) => message.type), ['fleet', 'fleet', 'run'])
  assert.deepEqual(elsewhere.sent.map((message) => message.type), ['fleet', 'fleet'])
})

test('the overview’s readouts go only to a client on the Realms view', async () => {
  const { poller, peer, settle } = polled([fleetAt(12), fleetAt(13)])
  const realms = peer()
  const realm = peer()
  poller.attach(realms)({ type: 'watch', view: 'realms', run: null })
  await settle(realms)
  poller.attach(realm)({ type: 'watch', view: 'realm', run: null })
  await settle(realm)
  assert.deepEqual([realms.sent[0].type, realms.sent[0].overview === null], ['snapshot', false])
  assert.deepEqual([realm.sent[0].type, realm.sent[0].overview], ['snapshot', null])
})

test('a client that reconnects receives the current snapshot, its open run included', async () => {
  const { poller, peer, settle } = polled([fleetAt(12), fleetAt(14)])
  const first = peer()
  poller.attach(first)({ type: 'watch', view: 'realm', run: { realm: 'brokkr', run_id: RUN } })
  await settle(first)
  first.onClose()
  const again = peer()
  poller.attach(again)({ type: 'watch', view: 'realm', run: { realm: 'brokkr', run_id: RUN } })
  await settle(again)
  const [snapshot] = again.sent
  assert.deepEqual(
    [snapshot.type, snapshot.fleet.hearths[0].runs[0].seq, snapshot.run.run_id, snapshot.run.view.summary.seq],
    ['snapshot', 14, RUN, 12],
  )
})
