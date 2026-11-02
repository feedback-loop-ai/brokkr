// Live refresh (design D5): every open run is re-read on every poll, only
// for the clients that have it open; the overview's readouts go only to a
// client on the Realms view; a refused read reaches the page as Brokkr's own
// words. The fixtures these tests share live in tests/support/poller.js.
import assert from 'node:assert/strict'
import { existsSync, readdirSync, readFileSync } from 'node:fs'
import { test } from 'node:test'

import { brokkrOf } from '../lib/host/brokkr.js'
import { configFrom } from '../lib/host/config.js'
import { pollerOf } from '../lib/host/poller.js'
import { fleetOf, ONE_HEARTH } from '../lib/host/readouts.js'
import { findAll, textOf } from './support/client.js'
import { RUN, OTHER, attached, fleetAt, openedPage, pasted, polled, heard, realmsDoc, refusedHint, runOf, windowOf } from './support/poller.js'
import { analyticsDoc, fleetDoc, json, row, runViewDoc, stubBrokkr } from './support/world.js'

test('an open run is re-read on every poll, and only for the client that has it open', async () => {
  const { poller, calls } = polled(
    [fleetAt(12), fleetAt(12), fleetAt(12, 4), fleetAt(13, 4)],
    undefined,
    [12, 12, 13].map((seq) => json(runViewDoc({ seq }))),
  )
  const open = await attached(poller, { view: 'realm', run: { realm: 'brokkr', run_id: RUN } })
  const elsewhere = await attached(poller, { view: 'analytics', run: null })
  const before = calls().filter((verb) => verb === 'inspect').length
  open.sent.length = 0
  elsewhere.sent.length = 0

  await poller.tick() // only the other run moved; the open run is still read
  assert.equal(calls().filter((verb) => verb === 'inspect').length, before + 1)
  assert.deepEqual(open.sent.map((message) => message.type), ['fleet'])

  await poller.tick() // the open run moved
  assert.equal(calls().filter((verb) => verb === 'inspect').length, before + 2)
  assert.deepEqual(open.sent.map((message) => message.type), ['fleet', 'fleet', 'run'])
  assert.deepEqual(elsewhere.sent.map((message) => message.type), ['fleet', 'fleet'])
})

test('the overview’s readouts go only to a client on the Realms view', async () => {
  const { poller } = polled([fleetAt(12), fleetAt(13)])
  const realms = await attached(poller, { view: 'realms', run: null })
  const realm = await attached(poller, { view: 'realm', run: null })
  assert.deepEqual([realms.sent[0].type, realms.sent[0].overview === null], ['snapshot', false])
  assert.deepEqual([realm.sent[0].type, realm.sent[0].overview], ['snapshot', null])
})

test('a client that reconnects receives the current snapshot, its open run included', async () => {
  const { poller } = polled([fleetAt(12), fleetAt(14)])
  const first = await attached(poller, { view: 'realm', run: { realm: 'brokkr', run_id: RUN } })
  first.onClose()
  const again = await attached(poller, { view: 'realm', run: { realm: 'brokkr', run_id: RUN } })
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
    const { poller } = polled([fleetAt(12)], ask)
    const client = await attached(poller, { view: 'realms', run: null })
    assert.equal(client.sent[0].ask, carried)
  }
})

test('a refused read reaches the open client as Brokkr’s words and the command the console ran, in the snapshot and after the head moves', async () => {
  const words = 'brokkr inspect was refused: the journal moved beneath the read'
  const hint = `brokkr inspect --run=${RUN} --db=/hearths/brokkr/forge.db`
  const brokkr = stubBrokkr([
    { match: ['realms'], responses: [json(realmsDoc(['brokkr']))] },
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
    { match: ['realms'], responses: [json(realmsDoc(['brokkr']))] },
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
    { match: ['realms'], responses: [json(realmsDoc(['brokkr']))] },
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

test('unchanged Realms snapshots reuse cached day analytics', async () => {
  const brokkr = stubBrokkr([
    { match: ['runs'], responses: [json(windowOf([runOf('alpha', 0)], null))] },
    { match: ['queue'], responses: [json([])] },
    { match: ['muninn'], responses: [json([])] },
    { match: ['analytics'], responses: [json(analyticsDoc())] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const client = await attached(poller, { view: 'realms', run: null })
  const analytics = () => brokkr.calls().filter((argv) => argv[0] === 'analytics').length
  assert.equal(analytics(), 1)

  client.sent.length = 0
  client.receive({ type: 'watch', view: 'realms', run: null })
  assert.equal((await heard(client)).type, 'snapshot')
  assert.equal(analytics(), 1)
})

test('a source pin: the one fleet message under lib is built only inside sendFleet', () => {
  const root = new URL('../lib/', import.meta.url)
  const files = readdirSync(root, { recursive: true, encoding: 'utf8' }).filter((file) => file.endsWith('.js'))
  const built = []
  for (const file of files) {
    const text = readFileSync(new URL(file, root), 'utf8')
    for (let at = text.indexOf("type: 'fleet'"); at !== -1; at = text.indexOf("type: 'fleet'", at + 1)) {
      const before = text.slice(0, at)
      built.push({ file, sendFleet: before.lastIndexOf('function sendFleet(') > before.lastIndexOf('\n  function ') })
    }
  }
  assert.deepEqual(built, [{ file: 'host/poller.js', sendFleet: true }])
})
