// Live refresh (design D5): a run is re-read only when its head moved, only
// for the client that has it open; the overview's readouts go only to a
// client on the Realms view; a client that reconnects gets a full snapshot.
import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { existsSync, readFileSync } from 'node:fs'
import { test } from 'node:test'

import { brokkrOf } from '../lib/host/brokkr.js'
import { configFrom } from '../lib/host/config.js'
import { pollerOf } from '../lib/host/poller.js'
import { fleetOf, ONE_HEARTH } from '../lib/host/readouts.js'
import { findAll, openedOn, textOf } from './support/client.js'
import { analyticsDoc, fleetDoc, json, row, runViewDoc, stubBrokkr } from './support/world.js'

const RUN = row().run_id
const OTHER = 'lane-detection-v2-77aa01bc'

const fleetAt = (seq, other = 3) => json(fleetDoc([['brokkr', [row({ seq }), row({ run_id: OTHER, seq: other, title: 'Lane detection v2' })]]]))

/** Wait until `client` has been sent its snapshot, then hand it back. */
async function settle(client) {
  for (let waited = 0; client.sent.length === 0; waited += 50) {
    assert.ok(waited < 10_000, 'no snapshot within 10s')
    await new Promise((wake) => setTimeout(wake, 50))
  }
  return client
}

function polled(fleets, ask = undefined) {
  const brokkr = stubBrokkr([
    { match: ['runs'], responses: fleets },
    { match: ['inspect'], responses: [json(runViewDoc())] },
    { match: ['queue'], responses: [json([])] },
    { match: ['muninn'], responses: [json([])] },
    { match: ['analytics'], responses: [json(analyticsDoc())] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000, ...(ask === undefined ? {} : { ask }) })
  const peer = () => {
    const sent = []
    return { sent, send: (message) => sent.push(message), onClose: () => {} }
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

test('the question boxes are off unless BROKKR_CONSOLE_ASK is 1, and the snapshot carries the setting', async () => {
  assert.deepEqual(['', '0', 'yes', '1'].map((value) => configFrom({ BROKKR_CONSOLE_ASK: value }, '/w').ask), [false, false, false, true])
  assert.equal(configFrom({}, '/w').ask, false)
  for (const [ask, carried] of [[undefined, false], [true, true]]) {
    const { poller, peer, settle } = polled([fleetAt(12)], ask)
    const client = peer()
    poller.attach(client)({ type: 'watch', view: 'realms', run: null })
    await settle(client)
    assert.equal(client.sent[0].ask, carried)
  }
})

/** Attach a client watching `watch` to `poller`, and wait for its first message. */
async function attached(poller, watch) {
  const sent = []
  const client = { sent, send: (message) => sent.push(message), onClose: () => {} }
  poller.attach(client)({ type: 'watch', ...watch })
  return settle(client)
}

/** The argv a POSIX shell hands `brokkr` for a pasted hint: the function only prints them. */
function pasted(hint) {
  return execFileSync('bash', ['-c', `brokkr() { printf '%s\\n' "$@"; }; ${hint}`], { encoding: 'utf8' }).trimEnd().split('\n')
}

/** The browser half opened on the parked run, painting the host's `run` message. */
function openedPage(fleetView, run, realm = 'brokkr') {
  const page = openedOn(fleetView, realm)
  page.push({ type: 'run', run })
  return page
}

/** A refused inspect for the parked run in the many-hearth world whose journal is `journal`, read and painted. */
async function refusedHint(journal) {
  const words = 'brokkr inspect was refused: the journal could not be read'
  const many = fleetDoc([['Project Alpha', [row({ seq: 12 })]], ['looper', [row({ run_id: OTHER, seq: 3, title: 'Lane detection v2' })]]])
  many.realms[0].journal = journal
  const brokkr = stubBrokkr([
    { match: ['runs'], responses: [json(many)] },
    { match: ['inspect'], responses: [{ stderr: words, code: 1 }] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const client = await attached(poller, { view: 'realm', run: { realm: 'Project Alpha', run_id: RUN } })
  const run = client.sent[0].run
  return { run, page: openedPage(fleetOf(many), run, 'Project Alpha'), brokkr }
}

test('a refused read reaches the open client as Brokkr’s words and the command the console ran, in the snapshot and after the head moves', async () => {
  const words = 'brokkr inspect was refused: the journal moved beneath the read'
  const hint = `brokkr inspect --run=${RUN} --db=/hearths/brokkr/forge.db`
  const brokkr = stubBrokkr([
    { match: ['runs'], responses: [fleetAt(12), fleetAt(13)] },
    { match: ['inspect'], responses: [{ stderr: words, code: 1 }] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const client = await attached(poller, { view: 'realm', run: { realm: 'brokkr', run_id: RUN } })
  const refused = { realm: 'brokkr', run_id: RUN, view: null, refused: words, hint }
  assert.deepEqual(client.sent[0].run, refused)
  assert.deepEqual(brokkr.calls().find((argv) => argv[0] === 'inspect'), ['inspect', `--run=${RUN}`, '--db=/hearths/brokkr/forge.db', '--json'])

  client.sent.length = 0
  await poller.tick()
  const [moved] = client.sent.filter((message) => message.type === 'run')
  assert.deepEqual(moved.run, refused)
})

test('a run absent from the fleet reaches the open client as not listed, in the snapshot and after it drops out', async () => {
  const other = row({ run_id: OTHER, seq: 3, title: 'Lane detection v2' })
  const without = json(fleetDoc([['brokkr', [other]]]))
  const brokkr = stubBrokkr([
    { match: ['runs'], responses: [fleetAt(12), without] },
    { match: ['inspect'], responses: [json(runViewDoc())] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const client = await attached(poller, { view: 'realm', run: { realm: 'brokkr', run_id: RUN } })
  assert.equal(client.sent[0].run.view.summary.seq, 12)

  client.sent.length = 0
  await poller.tick()
  const [moved] = client.sent.filter((message) => message.type === 'run')
  assert.deepEqual(moved.run, { realm: 'brokkr', run_id: RUN, view: null, listed: false })

  const again = await attached(poller, { view: 'realm', run: { realm: 'brokkr', run_id: RUN } })
  assert.deepEqual(again.sent[0].run, { realm: 'brokkr', run_id: RUN, view: null, listed: false })
})

test('the runner’s own output bound reaches the run page as Brokkr’s refusal and the command that reads that journal', async () => {
  const words = 'brokkr inspect printed more than the console reads'
  const hint = `brokkr inspect --run=${RUN} --db=/hearths/brokkr/forge.db`
  const brokkr = stubBrokkr([
    { match: ['runs'], responses: [fleetAt(12)] },
    { match: ['inspect'], responses: [{ bytes: 8193 }] },
  ], { maxBytes: 8192 })
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const client = await attached(poller, { view: 'realm', run: { realm: 'brokkr', run_id: RUN } })
  const run = client.sent[0].run
  assert.deepEqual([run.view, run.refused, run.hint], [null, words, hint])

  const page = openedPage(fleetOf(fleetDoc([['brokkr', [row({ seq: 12 })]]])), run)
  assert.deepEqual(findAll(page.tree(), (node) => node.props.className === 'bk-error').map(textOf), [words])
  assert.deepEqual(findAll(page.tree(), (node) => String(node.props.className).includes('bk-hint') && textOf(node) === hint).map(textOf), [hint])
})

test('a one-hearth world’s refusal paints the command without a journal the console did not read', async () => {
  const words = 'brokkr inspect was refused: the journal could not be read'
  const hint = `brokkr inspect --run=${RUN}`
  const oneHearth = { view_version: 28, runs: [row({ seq: 12 })], count: 1 }
  const brokkr = stubBrokkr([
    { match: ['runs'], responses: [json(oneHearth)] },
    { match: ['inspect'], responses: [{ stderr: words, code: 1 }] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const client = await attached(poller, { view: 'realm', run: { realm: ONE_HEARTH, run_id: RUN } })
  const run = client.sent[0].run
  assert.deepEqual(run, { realm: ONE_HEARTH, run_id: RUN, view: null, refused: words, hint })
  assert.deepEqual(brokkr.calls().find((argv) => argv[0] === 'inspect'), ['inspect', `--run=${RUN}`, '--json'])

  const page = openedPage(fleetOf(oneHearth), run, ONE_HEARTH)
  assert.deepEqual(findAll(page.tree(), (node) => String(node.props.className).includes('bk-hint') && textOf(node) === hint).map(textOf), [hint])
})

test('a refused-run hint keeps a journal path with a space as one pasteable argument', async () => {
  const journal = '/hearths/Project Alpha/forge.db'
  const { run, page, brokkr } = await refusedHint(journal)
  assert.deepEqual(brokkr.calls().find((argv) => argv[0] === 'inspect'), ['inspect', `--run=${RUN}`, `--db=${journal}`, '--json'])
  const painted = findAll(page.tree(), (node) => String(node.props.className).includes('bk-hint') && textOf(node) === run.hint).map(textOf)
  assert.deepEqual(painted, [run.hint])
  assert.deepEqual(pasted(run.hint), ['inspect', `--run=${RUN}`, `--db=${journal}`])
})

test('a refused-run hint keeps repeated spaces and a tab in its journal path, and the paint shows them', async () => {
  const journal = '/hearths/Project  \tAlpha/forge.db'
  const { run, page, brokkr } = await refusedHint(journal)
  assert.deepEqual(brokkr.calls().find((argv) => argv[0] === 'inspect'), ['inspect', `--run=${RUN}`, `--db=${journal}`, '--json'])
  assert.deepEqual(pasted(run.hint), ['inspect', `--run=${RUN}`, `--db=${journal}`])
  const painted = findAll(page.tree(), (node) => String(node.props.className).includes('bk-hint') && textOf(node) === run.hint).map(textOf)
  assert.deepEqual(painted, [run.hint])
  assert.match(readFileSync(new URL('../lib/console.css', import.meta.url), 'utf8'), /\.bk-hint\{[^}]*white-space:pre-wrap/)
})

test('a refused-run hint keeps an apostrophe in its journal path as one pasteable argument', async () => {
  const journal = "/hearths/val's/forge.db"
  const { run, page, brokkr } = await refusedHint(journal)
  assert.deepEqual(brokkr.calls().find((argv) => argv[0] === 'inspect'), ['inspect', `--run=${RUN}`, `--db=${journal}`, '--json'])
  assert.deepEqual(pasted(run.hint), ['inspect', `--run=${RUN}`, `--db=${journal}`])
  const painted = findAll(page.tree(), (node) => String(node.props.className).includes('bk-hint') && textOf(node) === run.hint).map(textOf)
  assert.deepEqual(painted, [run.hint])
})

test('a refused-run hint keeps a Unicode journal path as one pasteable argument', async () => {
  const journal = '/hearths/Ærø ☃/forge.db'
  const { run, page, brokkr } = await refusedHint(journal)
  assert.deepEqual(brokkr.calls().find((argv) => argv[0] === 'inspect'), ['inspect', `--run=${RUN}`, `--db=${journal}`, '--json'])
  assert.deepEqual(pasted(run.hint), ['inspect', `--run=${RUN}`, `--db=${journal}`])
  const painted = findAll(page.tree(), (node) => String(node.props.className).includes('bk-hint') && textOf(node) === run.hint).map(textOf)
  assert.deepEqual(painted, [run.hint])
})

test('a refused-run hint keeps shell syntax in its journal path inert', async () => {
  const journal = '/hearths/$(touch pwned)`; touch pwned`/forge.db'
  const { run, page, brokkr } = await refusedHint(journal)
  assert.deepEqual(brokkr.calls().find((argv) => argv[0] === 'inspect'), ['inspect', `--run=${RUN}`, `--db=${journal}`, '--json'])
  assert.deepEqual(pasted(run.hint), ['inspect', `--run=${RUN}`, `--db=${journal}`])
  assert.equal(existsSync('pwned'), false)
  const painted = findAll(page.tree(), (node) => String(node.props.className).includes('bk-hint') && textOf(node) === run.hint).map(textOf)
  assert.deepEqual(painted, [run.hint])
})
