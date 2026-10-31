// Live refresh (design D5): every open run is re-read on every poll, only
// for the clients that have it open, wherever the fleet holds it; the
// overview's readouts go only to a client on the Realms view; a client that
// reconnects gets a full snapshot.
import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { existsSync, mkdirSync, mkdtempSync, readFileSync, realpathSync, symlinkSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { test } from 'node:test'

import { brokkrOf } from '../lib/host/brokkr.js'
import { configFrom, FLEET_WINDOW } from '../lib/host/config.js'
import { pollerOf } from '../lib/host/poller.js'
import { fleetOf, ONE_HEARTH } from '../lib/host/readouts.js'
import { buttons, findAll, mountClient, openedOn, textOf } from './support/client.js'
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

/** A connected peer whose every host message is recorded. */
function peer() {
  const sent = []
  return { sent, send: (message) => sent.push(message), onClose: () => {} }
}

/** Attach a client watching `watch` to `poller`, wait for its first message, and hand it back with its inbound channel. */
async function attached(poller, watch) {
  const client = peer()
  client.receive = poller.attach(client)
  client.receive({ type: 'watch', ...watch })
  return settle(client)
}

function polled(fleets, ask = undefined, inspections = [json(runViewDoc())]) {
  const brokkr = stubBrokkr([
    { match: ['runs'], responses: fleets },
    { match: ['inspect'], responses: inspections },
    { match: ['queue'], responses: [json([])] },
    { match: ['muninn'], responses: [json([])] },
    { match: ['analytics'], responses: [json(analyticsDoc())] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000, ...(ask === undefined ? {} : { ask }) })
  return { poller, calls: () => brokkr.calls().map((argv) => argv[0]) }
}

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

/** A refused inspect for the parked run in the one-hearth world whose journal is `journal`, read and painted. */
async function refusedHint(journal) {
  const words = 'brokkr inspect was refused: the journal could not be read'
  const world = fleetDoc([['Project Alpha', [row({ seq: 12 })]]])
  world.realms[0].journal = journal
  const brokkr = stubBrokkr([
    { match: ['runs'], responses: [json(world)] },
    { match: ['inspect'], responses: [{ stderr: words, code: 1 }] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const client = await attached(poller, { view: 'realm', run: { realm: 'Project Alpha', run_id: RUN } })
  const run = client.sent[0].run
  return { run, page: openedPage(fleetOf(world), run, 'Project Alpha'), brokkr }
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

/** A paged `brokkr runs --json`: one window and where the next page starts, as Brokkr states both. */
const windowOf = (runs, next = null) => ({ view_version: 28, runs, count: runs.length, next, behind: 0 })

/** `brokkr runs --json --run <id>`: the same page wire, holding that run's one row. */
const oneRow = (row) => ({ view_version: 28, runs: [row], count: 1 })

/** `brokkr realms --json` for `names`, each in its own journal. */
const realmsDoc = (names) => ({
  journal: './.forge/forge.db',
  map: './realms.json',
  realms: names.map((name) => ({ default_branch: 'main', head: '0000000', journal: `/hearths/${name}/forge.db`, name, path: `../${name}` })),
})

/** A run row of `name`'s hearth, its title and feature naming it. */
const runOf = (name, index) => row({
  run_id: `${name}-run-${String(index).padStart(4, '0')}`,
  title: `${name} run ${index}`,
  feature: `${name} run ${index}`,
  seq: index,
})

/** Wait for `client`'s next message, then clear what it has. */
async function heard(client) {
  for (let waited = 0; client.sent.length === 0; waited += 50) {
    assert.ok(waited < 10_000, 'no message within 10s')
    await new Promise((wake) => setTimeout(wake, 50))
  }
  const message = client.sent.at(-1)
  client.sent.length = 0
  return message
}

test('a world of four hearths of two thousand runs each is read a window a hearth, well under the bound, and the page says more', async () => {
  const names = ['alpha', 'beta', 'gamma', 'delta']
  const page = windowOf(Array.from({ length: FLEET_WINDOW }, (_, index) => runOf('alpha', index)), `needs-you:${FLEET_WINDOW}:alpha-run-0099`)
  // The page is what a windowed read hands back; a read without --limit would be the whole 2,000-run world, past the bound.
  const brokkr = stubBrokkr([
    { match: ['realms'], responses: [json(realmsDoc(names))] },
    { match: ['runs', `--limit=${FLEET_WINDOW}`], responses: [json(page)] },
    { match: ['runs'], responses: [{ bytes: 16 * 1024 * 1024 + 1 }] },
  ], { maxBytes: 16 * 1024 * 1024 })
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const client = await attached(poller, { view: 'realm', realm: 'alpha', run: null })
  const snapshot = client.sent[0]
  assert.equal(snapshot.error, undefined)
  assert.deepEqual(
    snapshot.fleet.hearths.map((hearth) => [hearth.realm, hearth.runs.length, hearth.next]),
    names.map((name) => [name, FLEET_WINDOW, page.next]),
  )

  const reads = brokkr.calls().filter((argv) => argv[0] === 'runs')
  assert.equal(reads.length, names.length)
  assert.deepEqual(reads.flatMap((argv) => argv.filter((word) => word.startsWith('--db='))).sort(), names.map((name) => `--db=/hearths/${name}/forge.db`).sort())
  assert.ok(reads.every((argv) => argv.includes(`--limit=${FLEET_WINDOW}`)))
  assert.ok(reads.every((argv) => !argv.some((word) => word.startsWith('--after='))))
  assert.ok(Buffer.byteLength(JSON.stringify(page)) < 16 * 1024 * 1024)

  const painted = mountClient('main')
  painted.push(snapshot)
  buttons(painted.tree()).find((button) => textOf(button).startsWith('alpha')).props.onClick()
  const runs = findAll(painted.tree(), (node) => node.props['aria-label'] === 'Runs')[0]
  assert.match(textOf(runs), /^runs · 100\+/)
  assert.deepEqual(findAll(runs, (node) => node.props.className === 'bk-grp').map(textOf), ['Needs you · 100+'])
  assert.equal(findAll(runs, (node) => node.props.className === 'bk-q').length, FLEET_WINDOW)
  buttons(runs).find((button) => textOf(button) === 'Show more runs').props.onClick()
  assert.deepEqual(painted.sockets[0].sent.at(-1), { type: 'more', realm: 'alpha' })
})

test('a hearth’s next page is read only when asked, appended in order, and a refresh keeps it only while that hearth is on screen', async () => {
  const names = ['alpha', 'beta']
  const alphaFirst = windowOf([runOf('alpha', 0), runOf('alpha', 1)], `needs-you:2:alpha-run-0001`)
  const alphaChanged = windowOf([runOf('alpha', 0), runOf('alpha', 1)], `needs-you:2:alpha-run-0001`)
  alphaChanged.runs[0] = { ...alphaChanged.runs[0], seq: 20 }
  const alphaNext = windowOf([runOf('alpha', 2), runOf('alpha', 3)], null)
  const betaPage = windowOf([runOf('beta', 0)], null)
  const brokkr = stubBrokkr([
    { match: ['realms'], responses: [json(realmsDoc(names))] },
    { match: ['runs', '--db=/hearths/alpha/forge.db'], responses: [json(alphaFirst), json(alphaNext), json(alphaChanged)] },
    { match: ['runs', '--db=/hearths/beta/forge.db'], responses: [json(betaPage)] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const client = await attached(poller, { view: 'realm', realm: 'alpha', run: null })
  const { receive, sent } = client
  const [snapshot] = sent.splice(0)
  assert.deepEqual(snapshot.fleet.hearths.find((hearth) => hearth.realm === 'alpha').runs.map((row) => row.run_id), alphaFirst.runs.map((row) => row.run_id))

  receive({ type: 'more', realm: 'alpha' })
  const more = await heard(client)
  const alphaReads = () => brokkr.calls().filter((argv) => argv[0] === 'runs' && argv.includes('--db=/hearths/alpha/forge.db'))
  assert.deepEqual(alphaReads().at(-1), ['runs', '--json', '--db=/hearths/alpha/forge.db', `--limit=${FLEET_WINDOW}`, `--after=${alphaFirst.next}`])
  const appended = more.fleet.hearths.find((hearth) => hearth.realm === 'alpha')
  assert.deepEqual(appended.runs.map((row) => row.run_id), [...alphaFirst.runs, ...alphaNext.runs].map((row) => row.run_id))
  assert.equal(appended.next, null)

  // A refresh reads each hearth's first page again and keeps the page alpha is showing.
  await poller.tick()
  const refreshed = await heard(client)
  const kept = refreshed.fleet.hearths.find((hearth) => hearth.realm === 'alpha')
  assert.deepEqual(kept.runs.map((row) => row.run_id), [...alphaChanged.runs, ...alphaNext.runs].map((row) => row.run_id))
  assert.deepEqual(alphaReads().map((argv) => argv.length), [4, 5, 4])
  assert.ok(!alphaReads().at(-1).some((word) => word.startsWith('--after=')))

  // Off alpha's screen, a refresh lets the paged-in rows go.
  receive({ type: 'watch', view: 'realm', realm: 'beta', run: null })
  const elsewhere = await heard(client)
  const dropped = elsewhere.fleet.hearths.find((hearth) => hearth.realm === 'alpha')
  assert.deepEqual(dropped.runs.map((row) => row.run_id), alphaFirst.runs.map((row) => row.run_id))
  assert.equal(dropped.next, alphaFirst.next)
})

test('a one-hearth world is read without --db, as the run hint names no journal', async () => {
  const page = windowOf([runOf('alpha', 0)], null)
  const brokkr = stubBrokkr([{ match: ['runs'], responses: [json(page)] }])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const client = await attached(poller, { view: 'realm', realm: ONE_HEARTH, run: null })
  assert.deepEqual(brokkr.calls().filter((argv) => argv[0] === 'runs'), [['runs', '--json', `--limit=${FLEET_WINDOW}`]])
  assert.deepEqual(client.sent[0].fleet.hearths.map((hearth) => [hearth.realm, hearth.journal, hearth.next]), [[ONE_HEARTH, null, null]])
})

/** A run row standing running, so a hearth's second page can hold one. */
const runningOf = (name, index, seq) => ({
  ...runOf(name, index),
  seq,
  verdict: { ...row().verdict, standing: 'running', rule: null, reason: null, text: 'running' },
})

test('one-hearth Show more appends a second page', async () => {
  const first = windowOf([runOf('alpha', 0)], 'needs-you:1:alpha-run-0000')
  const second = windowOf([runOf('alpha', 1)], null)
  const brokkr = stubBrokkr([
    { match: ['runs'], responses: [json(first), json(second)] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const client = await attached(poller, { view: 'realm', realm: ONE_HEARTH, run: null })
  assert.deepEqual(client.sent[0].fleet.hearths[0].runs.map((row) => row.run_id), first.runs.map((row) => row.run_id))

  client.sent.length = 0
  client.receive({ type: 'more', realm: ONE_HEARTH })
  const more = await heard(client)
  assert.deepEqual(more.fleet.hearths[0].runs.map((row) => row.run_id), [...first.runs, ...second.runs].map((row) => row.run_id))
  assert.equal(more.fleet.hearths[0].next, null)
  assert.deepEqual(brokkr.calls().filter((argv) => argv[0] === 'runs').at(-1), ['runs', '--json', `--limit=${FLEET_WINDOW}`, `--after=${first.next}`])
})

test('a refresh keeps a previously visible page-boundary run reachable', async () => {
  const first = windowOf([runOf('alpha', 0), runOf('alpha', 1)], 'needs-you:2:alpha-run-0001')
  const second = windowOf([runOf('alpha', 2), runOf('alpha', 3)], null)
  const shifted = windowOf([runOf('alpha', 9), runOf('alpha', 0)], 'needs-you:2:alpha-run-0000')
  const bridge = windowOf([runOf('alpha', 1)], null)
  const brokkr = stubBrokkr([
    { match: ['runs'], responses: [json(first), json(second), json(shifted), json(bridge)] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const client = await attached(poller, { view: 'realm', realm: ONE_HEARTH, run: null })
  client.sent.length = 0
  client.receive({ type: 'more', realm: ONE_HEARTH })
  await heard(client)
  client.sent.length = 0

  // A newer run pushes the page boundary: the loaded tail is let go, and the
  // fresh cursor still reaches the run the old boundary held.
  await poller.tick()
  const refreshed = await heard(client)
  const alpha = refreshed.fleet.hearths[0]
  assert.deepEqual(alpha.runs.map((row) => row.run_id), shifted.runs.map((row) => row.run_id))
  assert.equal(alpha.next, shifted.next)

  client.receive({ type: 'more', realm: ONE_HEARTH })
  const bridged = await heard(client)
  assert.deepEqual(bridged.fleet.hearths[0].runs.map((row) => row.run_id).slice(-1), [first.runs[1].run_id])
})

test('behind makes the painted count incomplete even with no next cursor', async () => {
  const page = { ...windowOf(Array.from({ length: FLEET_WINDOW }, (_, index) => runOf('alpha', index)), null), behind: 130 }
  const brokkr = stubBrokkr([
    { match: ['runs'], responses: [json(page)] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const client = await attached(poller, { view: 'realm', realm: ONE_HEARTH, run: null })
  const [snapshot] = client.sent
  assert.equal(snapshot.fleet.hearths[0].behind, 130)

  const painted = mountClient('main')
  painted.push(snapshot)
  buttons(painted.tree()).find((button) => textOf(button).startsWith(ONE_HEARTH)).props.onClick()
  const runs = findAll(painted.tree(), (node) => node.props['aria-label'] === 'Runs')[0]
  assert.match(textOf(runs), /^runs · 100\+/)
  assert.deepEqual(findAll(runs, (node) => node.props.className === 'bk-grp').map(textOf), ['Needs you · 100+'])
  assert.deepEqual(
    findAll(runs, (node) => node.props.className === 'bk-note').map(textOf).filter((text) => text.includes('behind their summary')),
    ['130 more runs behind their summary, unread until a writer catches them up'],
  )
})

test('a running second-page run refreshes behind 100 parked runs', async () => {
  const parked = Array.from({ length: FLEET_WINDOW }, (_, index) => runOf('alpha', index))
  const first = windowOf(parked, `needs-you:${FLEET_WINDOW}:alpha-run-0099`)
  const running = runningOf('alpha', 100, 12)
  const second = windowOf([running], null)
  const brokkr = stubBrokkr([
    { match: ['runs'], responses: [json(first), json(second), json(first)] },
    { match: ['inspect'], responses: [json(runViewDoc({ seq: 12 })), json(runViewDoc({ seq: 13 }))] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const client = await attached(poller, { view: 'realm', realm: ONE_HEARTH, run: null })
  client.sent.length = 0
  client.receive({ type: 'more', realm: ONE_HEARTH })
  await heard(client)

  client.receive({ type: 'watch', view: 'realm', realm: ONE_HEARTH, run: { realm: ONE_HEARTH, run_id: running.run_id } })
  const opened = await heard(client)
  assert.equal(opened.run.view.summary.seq, 12)

  client.sent.length = 0
  await poller.tick()
  const update = client.sent.find((message) => message.type === 'run')
  assert.equal(update.run.view.summary.seq, 13)
})

test('equivalent journal paths name one hearth and read one first page', async () => {
  const aliased = {
    journal: './.forge/forge.db',
    map: './realms.json',
    realms: [
      { default_branch: 'main', head: '0000000', journal: '/hearths/a/forge.db', name: 'alpha', path: '../alpha' },
      { default_branch: 'main', head: '0000000', journal: '/hearths/a/../a/forge.db', name: 'beta', path: '../beta' },
    ],
  }
  const page = windowOf([runOf('alpha', 0)], null)
  const brokkr = stubBrokkr([
    { match: ['realms'], responses: [json(aliased)] },
    { match: ['runs'], responses: [json(page)] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const client = await attached(poller, { view: 'realm', realm: ONE_HEARTH, run: null })
  assert.deepEqual(client.sent[0].fleet.hearths.map((hearth) => [hearth.realm, hearth.runs.length, hearth.next]), [['alpha+beta', 1, null]])
  assert.deepEqual(brokkr.calls().filter((argv) => argv[0] === 'runs'), [['runs', '--json', `--limit=${FLEET_WINDOW}`]])
})

test('simultaneous Show more requests cannot duplicate rows after refresh', async () => {
  const first = windowOf([runOf('alpha', 0)], 'needs-you:1:alpha-run-0000')
  const second = windowOf([runOf('alpha', 1)], null)
  const brokkr = stubBrokkr([
    { match: ['runs', `--after=${first.next}`], responses: [json(second)] },
    { match: ['runs'], responses: [json(first)] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const client = await attached(poller, { view: 'realm', realm: ONE_HEARTH, run: null })
  const { receive, sent } = client
  sent.length = 0
  receive({ type: 'more', realm: ONE_HEARTH })
  receive({ type: 'more', realm: ONE_HEARTH })
  const more = await heard(client)
  assert.deepEqual(more.fleet.hearths[0].runs.map((row) => row.run_id), [...first.runs, ...second.runs].map((row) => row.run_id))

  // Both requests read the page before either settles, so the loser is in
  // flight past the fleet the winner pushed.
  const after = () => brokkr.calls().filter((argv) => argv.some((word) => word.startsWith('--after='))).length
  for (let waited = 0; after() < 2; waited += 10) {
    assert.ok(waited < 10_000, 'the second request did not read within 10s')
    await new Promise((wake) => setTimeout(wake, 10))
  }
  await new Promise((wake) => setTimeout(wake, 50))
  sent.length = 0
  await poller.tick()
  sent.length = 0
  receive({ type: 'watch', view: 'realm', realm: ONE_HEARTH, run: null })
  const refreshed = await heard(client)
  assert.deepEqual(refreshed.fleet.hearths[0].runs.map((row) => row.run_id), [...first.runs, ...second.runs].map((row) => row.run_id))
})

test('a refused hearth keeps its detail while another hearth still lists', async () => {
  const names = ['alpha', 'beta']
  const words = 'brokkr runs was refused: the journal could not be read'
  const brokkr = stubBrokkr([
    { match: ['realms'], responses: [json(realmsDoc(names))] },
    { match: ['runs', '--db=/hearths/alpha/forge.db'], responses: [json(windowOf([runOf('alpha', 0)], null))] },
    { match: ['runs', '--db=/hearths/beta/forge.db'], responses: [{ stderr: words, code: 1 }] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const client = await attached(poller, { view: 'realm', realm: 'alpha', run: null })
  const byRealm = Object.fromEntries(client.sent[0].fleet.hearths.map((hearth) => [hearth.realm, hearth]))
  assert.deepEqual([byRealm.alpha.detail, byRealm.alpha.runs.length], [null, 1])
  assert.deepEqual([byRealm.beta.detail, byRealm.beta.runs.length, byRealm.beta.next, byRealm.beta.behind], [words, 0, null, 0])
})

test('a --db read that names more than one hearth is refused as a shape, not painted', async () => {
  const names = ['alpha', 'beta']
  const many = fleetDoc([['alpha', [row()]], ['beta', [row({ run_id: OTHER, title: 'Lane detection v2' })]]])
  const brokkr = stubBrokkr([
    { match: ['realms'], responses: [json(realmsDoc(names))] },
    { match: ['runs'], responses: [json(many)] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const client = await attached(poller, { view: 'realm', realm: 'alpha', run: null })
  assert.deepEqual(
    client.sent[0].fleet.hearths.map((hearth) => [hearth.realm, hearth.detail, hearth.runs.length, hearth.next]),
    names.map((name) => [name, 'brokkr runs named more than one hearth for one journal', 0, null]),
  )
})

test('a refused next page reaches the client as Brokkr’s words, and appends nothing', async () => {
  const first = windowOf([runOf('alpha', 0)], 'needs-you:1:alpha-run-0000')
  const next = windowOf([runOf('alpha', 1)], null)
  const words = 'brokkr runs was refused: the page could not be read'
  const brokkr = stubBrokkr([
    { match: ['runs'], responses: [json(first), { stderr: words, code: 1 }, json(next)] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const client = await attached(poller, { view: 'realm', realm: ONE_HEARTH, run: null })
  const { receive, sent } = client
  sent.length = 0
  receive({ type: 'more', realm: ONE_HEARTH })
  const refused = await heard(client)
  assert.deepEqual([refused.type, refused.message], ['error', words])
  assert.deepEqual(brokkr.calls().filter((argv) => argv[0] === 'runs').at(-1), ['runs', '--json', `--limit=${FLEET_WINDOW}`, `--after=${first.next}`])

  sent.length = 0
  receive({ type: 'more', realm: ONE_HEARTH })
  const more = await heard(client)
  assert.deepEqual(more.fleet.hearths[0].runs.map((row) => row.run_id), [...first.runs, ...next.runs].map((row) => row.run_id))
  assert.equal(more.fleet.hearths[0].next, null)
})

test('an open run stays readable when a new arrival moves the page boundary', async () => {
  const first = windowOf([runOf('alpha', 0)], 'needs-you:1:alpha-run-0000')
  const second = windowOf([runOf('alpha', 1)], null)
  const shifted = windowOf([runOf('alpha', 9), runOf('alpha', 0)], 'needs-you:2:alpha-run-0000')
  const brokkr = stubBrokkr([
    { match: ['runs'], responses: [json(first), json(second), json(first), json(shifted)] },
    { match: ['inspect'], responses: [json(runViewDoc({ seq: 12 }))] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const client = await attached(poller, { view: 'realm', realm: ONE_HEARTH, run: null })
  client.sent.length = 0
  client.receive({ type: 'more', realm: ONE_HEARTH })
  await heard(client)

  const paged = second.runs[0]
  client.receive({ type: 'watch', view: 'realm', realm: ONE_HEARTH, run: { realm: ONE_HEARTH, run_id: paged.run_id } })
  assert.equal((await heard(client)).run.view.summary.seq, 12)

  // The new arrival moves the first page's cursor, so the page holding this run
  // is let go; the page's absence cannot establish that the run was dropped.
  client.sent.length = 0
  await poller.tick()
  const [update] = client.sent.filter((message) => message.type === 'run')
  assert.equal(update.run.view.summary.seq, 12)
  assert.equal(update.run.listed, undefined)
  assert.equal(brokkr.calls().filter((argv) => argv[0] === 'inspect').length, 2)
})

test('Show more publishes its appended page when a poll finishes during the read', async () => {
  const first = windowOf([runOf('alpha', 0)], 'needs-you:1:alpha-run-0000')
  const second = windowOf([runOf('alpha', 1)], null)
  const brokkr = stubBrokkr([
    { match: ['runs', `--after=${first.next}`], responses: [{ ...json(second), sleep_ms: 400 }] },
    { match: ['runs'], responses: [json(first)] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const client = await attached(poller, { view: 'realm', realm: ONE_HEARTH, run: null })
  client.sent.length = 0
  client.receive({ type: 'more', realm: ONE_HEARTH })

  // Finish a whole poll while the next page is still in flight, then let it settle.
  const afterReads = () => brokkr.calls().filter((argv) => argv.some((word) => word.startsWith('--after='))).length
  for (let waited = 0; afterReads() === 0; waited += 10) {
    assert.ok(waited < 10_000, 'the next page did not start within 10s')
    await new Promise((wake) => setTimeout(wake, 10))
  }
  await poller.tick()
  const more = await heard(client)
  const alpha = more.fleet.hearths[0]
  assert.deepEqual(alpha.runs.map((row) => row.run_id), [...first.runs, ...second.runs].map((row) => row.run_id))
  assert.equal(alpha.next, null)
})

test('journal identity preserves OS resolution of a symlink followed by dot-dot', async () => {
  const workspace = mkdtempSync(join(tmpdir(), 'brokkr-console-links-'))
  mkdirSync(join(workspace, 'real/child'), { recursive: true })
  mkdirSync(join(workspace, 'work'), { recursive: true })
  mkdirSync(join(workspace, 'other'), { recursive: true })
  writeFileSync(join(workspace, 'real/forge.db'), '')
  writeFileSync(join(workspace, 'work/forge.db'), '')
  symlinkSync('../real/child', join(workspace, 'work/link'), 'dir')
  const real = realpathSync.native(join(workspace, 'real/forge.db'))
  const lexical = join(workspace, 'work/forge.db')
  const realms = {
    journal: './forge.db',
    map: './realms.json',
    realms: [
      { default_branch: 'main', head: '0000000', journal: 'work/link/../forge.db', name: 'alpha', path: '../alpha' },
      { default_branch: 'main', head: '0000000', journal: 'other/forge.db', name: 'beta', path: '../beta' },
    ],
  }
  const brokkr = stubBrokkr([
    { match: ['realms'], responses: [json(realms)] },
    { match: ['runs', `--db=${real}`], responses: [json(windowOf([runOf('alpha', 0)], null))] },
    { match: ['runs', `--db=${lexical}`], responses: [json(windowOf([{ ...runOf('alpha', 0), title: 'wrong-journal' }], null))] },
    { match: ['runs'], responses: [json(windowOf([runOf('beta', 0)], null))] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000, workspace })
  const client = await attached(poller, { view: 'realm', realm: 'alpha', run: null })
  const alpha = client.sent[0].fleet.hearths.find((hearth) => hearth.realm === 'alpha')
  assert.deepEqual([alpha.journal, alpha.runs[0].title], [real, 'alpha run 0'])
  assert.ok(brokkr.calls().some((argv) => argv.includes(`--db=${real}`)))
})

test('the realm header marks counts incomplete when behind is nonzero without next', async () => {
  const page = { ...windowOf([runOf('alpha', 0)], null), behind: 130 }
  const brokkr = stubBrokkr([
    { match: ['runs'], responses: [json(page)] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const client = await attached(poller, { view: 'realm', realm: ONE_HEARTH, run: null })
  const painted = mountClient('main')
  painted.push(client.sent[0])
  buttons(painted.tree()).find((button) => textOf(button).startsWith(ONE_HEARTH)).props.onClick()
  const bar = findAll(painted.tree(), (node) => node.props.className === 'bk-bar')[0]
  assert.match(textOf(bar), /0\+ running/)
  assert.match(textOf(bar), /1\+ need you/)
})

test('Show more preserves the new behind count and paints a lower bound', async () => {
  const first = windowOf([runOf('alpha', 0)], 'needs-you:1:alpha-run-0000')
  const second = { ...windowOf([runOf('alpha', 1)], null), behind: 130 }
  const brokkr = stubBrokkr([
    { match: ['runs'], responses: [json(first), json(second)] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const client = await attached(poller, { view: 'realm', realm: ONE_HEARTH, run: null })
  client.sent.length = 0
  client.receive({ type: 'more', realm: ONE_HEARTH })
  const more = await heard(client)
  const alpha = more.fleet.hearths[0]
  assert.deepEqual([alpha.runs.length, alpha.next, alpha.behind], [2, null, 130])

  const painted = mountClient('main')
  painted.push(more)
  buttons(painted.tree()).find((button) => textOf(button).startsWith(ONE_HEARTH)).props.onClick()
  const runs = findAll(painted.tree(), (node) => node.props['aria-label'] === 'Runs')[0]
  assert.match(textOf(runs), /^runs · 2\+/)
  assert.deepEqual(findAll(runs, (node) => node.props.className === 'bk-grp').map(textOf), ['Needs you · 2+'])
  assert.deepEqual(
    findAll(runs, (node) => node.props.className === 'bk-note').map(textOf).filter((text) => text.includes('behind their summary')),
    ['130 more runs behind their summary, unread until a writer catches them up'],
  )
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

test('a paged open run pushes one update for one head change across four polls', async () => {
  const first = windowOf([runOf('alpha', 0)], 'needs-you:1:alpha-run-0000')
  const running = runningOf('alpha', 1, 12)
  const second = windowOf([running], null)
  const brokkr = stubBrokkr([
    { match: ['runs'], responses: [json(first), json(second), json(first)] },
    { match: ['inspect'], responses: [json(runViewDoc({ seq: 12 })), json(runViewDoc({ seq: 13 }))] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const client = await attached(poller, { view: 'realm', realm: ONE_HEARTH, run: null })
  client.sent.length = 0
  client.receive({ type: 'more', realm: ONE_HEARTH })
  await heard(client)
  client.receive({ type: 'watch', view: 'realm', realm: ONE_HEARTH, run: { realm: ONE_HEARTH, run_id: running.run_id } })
  assert.equal((await heard(client)).run.view.summary.seq, 12)

  const pushes = []
  for (let poll = 0; poll < 4; poll += 1) {
    client.sent.length = 0
    await poller.tick()
    pushes.push(client.sent.filter((message) => message.type === 'run').length)
  }
  assert.deepEqual(pushes, [1, 0, 0, 0])
})

/** The fleet an open run was displaced from: a hundred parked first-page runs and a running run on the page after. */
function displaced() {
  const parked = Array.from({ length: FLEET_WINDOW }, (_, index) => runOf('alpha', index))
  const first = windowOf(parked, `needs-you:${FLEET_WINDOW}:alpha-run-0099`)
  const running = runningOf('alpha', 100, 12)
  const second = windowOf([running], null)
  // A new arrival takes the last first-page place and moves its cursor.
  const arrival = windowOf([runOf('alpha', 200), ...parked.slice(0, FLEET_WINDOW - 1)], `needs-you:${FLEET_WINDOW}:alpha-run-0098`)
  return { first, running, second, arrival }
}

test('chief F1: an open run remains painted after its first-page cursor changes', async () => {
  const { first, running, second, arrival } = displaced()
  const brokkr = stubBrokkr([
    { match: ['runs', `--run=${running.run_id}`], responses: [json(oneRow(runningOf('alpha', 100, 13)))] },
    { match: ['runs'], responses: [json(first), json(second), json(first), json(arrival)] },
    { match: ['inspect'], responses: [json(runViewDoc({ seq: 12 })), json(runViewDoc({ seq: 13 }))] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const host = await attached(poller, { view: 'realm', realm: ONE_HEARTH, run: null })
  host.sent.length = 0
  host.receive({ type: 'more', realm: ONE_HEARTH })
  await heard(host)
  host.receive({ type: 'watch', view: 'realm', realm: ONE_HEARTH, run: { realm: ONE_HEARTH, run_id: running.run_id } })
  const opened = await heard(host)
  assert.equal(opened.run.view.summary.seq, 12)

  // The arrival moves the first page's cursor, so the page holding this run is let go.
  host.sent.length = 0
  await poller.tick()
  const fleet = host.sent.find((message) => message.type === 'fleet')
  assert.ok(fleet, 'the moved first page is pushed')
  assert.ok(fleet.fleet.hearths[0].runs.some((row) => row.run_id === running.run_id), 'the open run stays in the fleet')

  const painted = mountClient('main')
  painted.push({ type: 'snapshot', ask: false, fleet: fleet.fleet, overview: null, run: null })
  buttons(painted.tree()).find((button) => textOf(button).startsWith(ONE_HEARTH)).props.onClick()
  buttons(painted.tree()).find((button) => textOf(button).startsWith('alpha run 100')).props.onClick()
  painted.push({ type: 'run', run: opened.run })
  assert.equal(findAll(painted.tree(), (node) => node.props['aria-label'] === 'Graph').length, 1)
  assert.equal(findAll(painted.tree(), (node) => node.props['aria-label'] === 'Runs').length, 0)
})

test('chief F2: a displaced open run is inspected on each subsequent poll', async () => {
  const { first, running, second, arrival } = displaced()
  const brokkr = stubBrokkr([
    { match: ['runs'], responses: [json(first), json(second), json(first), json(arrival)] },
    { match: ['inspect'], responses: [12, 13, 14, 15].map((seq) => json(runViewDoc({ seq }))) },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const host = await attached(poller, { view: 'realm', realm: ONE_HEARTH, run: null })
  host.sent.length = 0
  host.receive({ type: 'more', realm: ONE_HEARTH })
  await heard(host)
  host.receive({ type: 'watch', view: 'realm', realm: ONE_HEARTH, run: { realm: ONE_HEARTH, run_id: running.run_id } })
  await heard(host)
  const inspections = () => brokkr.calls().filter((argv) => argv[0] === 'inspect').length
  assert.equal(inspections(), 1)

  const counts = [inspections()]
  const updates = []
  for (let poll = 0; poll < 3; poll += 1) {
    host.sent.length = 0
    await poller.tick()
    counts.push(inspections())
    updates.push(host.sent.find((message) => message.type === 'run')?.run.view.summary.seq ?? null)
  }
  assert.deepEqual(counts, [1, 2, 3, 4])
  assert.deepEqual(updates, [13, 14, 15])
})

test('chief F3: a freshly inspected completed run is not still counted as running', async () => {
  const { first, running, second } = displaced()
  const completed = runViewDoc({ seq: 13 })
  completed.summary = { ...completed.summary, status: 'completed', phase: 'done', park_reason: null }
  const done = { ...running, seq: 13, status: 'completed', phase: 'done', verdict: { standing: 'shipped', rule: 'REVIEW-CLEAN', residual: null, reason: null, text: 'CLEAN' } }
  const brokkr = stubBrokkr([
    { match: ['runs', `--run=${running.run_id}`], responses: [json(oneRow(done))] },
    { match: ['runs'], responses: [json(first), json(second), json(first)] },
    { match: ['inspect'], responses: [json(runningRead(12)), json(completed)] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const host = await attached(poller, { view: 'realm', realm: ONE_HEARTH, run: null })
  host.sent.length = 0
  host.receive({ type: 'more', realm: ONE_HEARTH })
  await heard(host)
  host.receive({ type: 'watch', view: 'realm', realm: ONE_HEARTH, run: { realm: ONE_HEARTH, run_id: running.run_id } })
  assert.equal((await heard(host)).run.view.summary.seq, 12)

  host.sent.length = 0
  await poller.tick()
  const fleet = host.sent.filter((message) => message.type === 'fleet').at(-1)
  assert.ok(fleet, 'the fresh summary is pushed as a fleet')
  const alpha = fleet.fleet.hearths[0]
  const row = alpha.runs.find((each) => each.run_id === running.run_id)
  assert.deepEqual([row.seq, row.verdict.standing], [13, 'shipped'])
  assert.equal(alpha.retained, true)

  const painted = mountClient('main')
  painted.push({ type: 'snapshot', ask: false, fleet: fleet.fleet, overview: null, run: null })
  const status = findAll(painted.tree(), (node) => node.props.className === 'bk-status')[0]
  assert.equal(textOf(status), '0+ running · 100+ waiting on you')
  buttons(painted.tree()).find((button) => textOf(button).startsWith(ONE_HEARTH)).props.onClick()
  const runs = findAll(painted.tree(), (node) => node.props['aria-label'] === 'Runs')[0]
  assert.match(textOf(buttons(runs).find((button) => textOf(button).startsWith('alpha run 100'))), /shipped/)
})

test('chief F4: changing a realm journal discards pages read from its old journal', async () => {
  const names = ['alpha', 'beta']
  const realmsWith = (journal) => ({
    journal: './.forge/forge.db',
    map: './realms.json',
    realms: names.map((name) => ({ default_branch: 'main', head: '0000000', journal: name === 'alpha' ? journal : `/hearths/${name}/forge.db`, name, path: `../${name}` })),
  })
  const cursor = 'needs-you:1:alpha-run-0000'
  const oldFirst = windowOf([runOf('alpha', 0)], cursor)
  const oldOnly = windowOf([runOf('alpha', 1)], null)
  const newFirst = windowOf([runOf('alpha', 2)], cursor)
  const beta = windowOf([runOf('beta', 0)], null)
  const brokkr = stubBrokkr([
    { match: ['realms'], responses: [json(realmsWith('/hearths/alpha/forge.db')), json(realmsWith('/hearths/alpha-new/forge.db')), json(realmsWith('/hearths/alpha-new/forge.db'))] },
    { match: ['runs', '--db=/hearths/alpha/forge.db'], responses: [json(oldFirst), json(oldOnly)] },
    { match: ['runs', '--db=/hearths/alpha-new/forge.db'], responses: [json(newFirst)] },
    { match: ['runs', '--db=/hearths/beta/forge.db'], responses: [json(beta)] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const host = await attached(poller, { view: 'realm', realm: 'alpha', run: null })
  host.sent.length = 0
  host.receive({ type: 'more', realm: 'alpha' })
  const more = await heard(host)
  assert.ok(more.fleet.hearths.find((hearth) => hearth.realm === 'alpha').runs.some((row) => row.run_id === 'alpha-run-0001'))

  const readings = []
  host.sent.length = 0
  await poller.tick()
  readings.push((await heard(host)).fleet)
  // The second poll need not push an unchanged fleet; a watch reads the state it settled on.
  host.sent.length = 0
  await poller.tick()
  host.receive({ type: 'watch', view: 'realm', realm: 'alpha', run: null })
  readings.push((await heard(host)).fleet)
  for (const fleet of readings) {
    const alpha = fleet.hearths.find((hearth) => hearth.realm === 'alpha')
    assert.deepEqual(alpha.runs.map((row) => row.run_id), ['alpha-run-0002'])
    assert.equal(alpha.next, cursor)
  }

  const painted = mountClient('main')
  painted.push({ type: 'snapshot', ask: false, fleet: readings.at(-1), overview: null, run: null })
  buttons(painted.tree()).find((button) => textOf(button).startsWith('alpha')).props.onClick()
  const runs = findAll(painted.tree(), (node) => node.props['aria-label'] === 'Runs')[0]
  assert.ok(buttons(runs).some((button) => textOf(button) === 'Show more runs'))
})

test('chief C1: an open first-page run is inspected on every poll, and each read updates it', async () => {
  const page = windowOf([runningOf('alpha', 0, 12)], null)
  const brokkr = stubBrokkr([
    { match: ['runs'], responses: [json(page)] },
    { match: ['inspect'], responses: [12, 13, 14].map((seq) => json(runViewDoc({ seq }))) },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const host = await attached(poller, {
    view: 'realm',
    realm: ONE_HEARTH,
    run: { realm: ONE_HEARTH, run_id: 'alpha-run-0000' },
  })
  const inspections = () => brokkr.calls().filter((argv) => argv[0] === 'inspect').length
  assert.equal(inspections(), 1)

  const updates = []
  for (let poll = 0; poll < 2; poll += 1) {
    host.sent.length = 0
    await poller.tick()
    updates.push(host.sent.find((message) => message.type === 'run')?.run.view.summary.seq ?? null)
  }
  assert.deepEqual([inspections(), updates], [3, [13, 14]])
})

test('chief C1: an open run a partial listing omits is read and still paints its detail', async () => {
  const page = windowOf([runOf('alpha', 0)], 'needs-you:1:alpha-run-0000')
  const missing = 'alpha-run-0100'
  const held = row({ run_id: missing, seq: 12 })
  const brokkr = stubBrokkr([
    { match: ['runs', `--run=${missing}`], responses: [json(oneRow(held))] },
    { match: ['runs'], responses: [json(page)] },
    { match: ['inspect'], responses: [12, 13, 14].map((seq) => json(runViewDoc({ seq }))) },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const host = await attached(poller, { view: 'realm', realm: ONE_HEARTH, run: { realm: ONE_HEARTH, run_id: missing } })
  const snapshot = host.sent[0]
  assert.ok(snapshot.fleet.hearths[0].runs.some((row) => row.run_id === missing), 'the inspected run is held in the fleet')

  const painted = mountClient('main')
  painted.push(snapshot)
  buttons(painted.tree()).find((button) => textOf(button).startsWith(ONE_HEARTH)).props.onClick()
  const runs = findAll(painted.tree(), (node) => node.props['aria-label'] === 'Runs')[0]
  buttons(runs).find((button) => textOf(button).startsWith('Encrypt the token store')).props.onClick()
  painted.push({ type: 'run', run: snapshot.run })
  assert.equal(findAll(painted.tree(), (node) => node.props['aria-label'] === 'Graph').length, 1)

  const inspections = () => brokkr.calls().filter((argv) => argv[0] === 'inspect').length
  host.sent.length = 0
  await poller.tick()
  assert.equal(inspections(), 2)
})

test('chief C2: a first-page fresh completed summary replaces its row and its counts', async () => {
  const page = windowOf([runningOf('alpha', 0, 12)], null)
  const completed = runViewDoc({ seq: 13 })
  completed.summary = { ...completed.summary, status: 'completed', phase: 'done', park_reason: null }
  const done = { ...runningOf('alpha', 0, 13), status: 'completed', phase: 'done', verdict: { standing: 'shipped', rule: 'REVIEW-CLEAN', residual: null, reason: null, text: 'CLEAN' } }
  const brokkr = stubBrokkr([
    { match: ['runs', '--run=alpha-run-0000'], responses: [json(oneRow(done))] },
    { match: ['runs'], responses: [json(page)] },
    { match: ['inspect'], responses: [json(completed)] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const host = await attached(poller, {
    view: 'realm',
    realm: ONE_HEARTH,
    run: { realm: ONE_HEARTH, run_id: 'alpha-run-0000' },
  })
  const snapshot = host.sent[0]
  const row = snapshot.fleet.hearths[0].runs[0]
  assert.deepEqual([row.seq, row.verdict.standing], [13, 'shipped'])

  const painted = mountClient('main')
  painted.push(snapshot)
  assert.equal(textOf(findAll(painted.tree(), (node) => node.props.className === 'bk-status')[0]), '0 running · 0 waiting on you')
  buttons(painted.tree()).find((button) => textOf(button).startsWith(ONE_HEARTH)).props.onClick()
  const runs = findAll(painted.tree(), (node) => node.props['aria-label'] === 'Runs')[0]
  assert.match(textOf(buttons(runs).find((button) => textOf(button).startsWith('alpha run 0'))), /shipped/)
})

test('chief C3: a single mapped hearth owns its pages by journal and keeps the new first-page cursor', async () => {
  const cursor = 'needs-you:1:alpha-run-0000'
  const oldFirst = windowOf([runOf('alpha', 0)], cursor)
  const oldOnly = windowOf([runOf('alpha', 1)], null)
  const newFirst = windowOf([runOf('alpha', 2)], cursor)
  const realmsFor = (journal) => ({
    ...realmsDoc(['alpha']),
    realms: [{ default_branch: 'main', head: '0000000', journal, name: 'alpha', path: '../alpha' }],
  })
  const brokkr = stubBrokkr([
    {
      match: ['realms'],
      responses: [json(realmsFor('/old/forge.db')), json(realmsFor('/new/forge.db')), json(realmsFor('/new/forge.db'))],
    },
    { match: ['runs'], responses: [json(oldFirst), json(oldOnly), json(newFirst)] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const host = await attached(poller, { view: 'realm', realm: 'alpha', run: null })
  assert.equal(host.sent[0].fleet.hearths[0].realm, 'alpha', 'a single mapped hearth keeps the realm its map names')
  host.sent.length = 0
  host.receive({ type: 'more', realm: 'alpha' })
  const more = await heard(host)
  assert.ok(more.fleet.hearths[0].runs.some((row) => row.run_id === 'alpha-run-0001'))

  host.sent.length = 0
  await poller.tick()
  const first = await heard(host)
  host.sent.length = 0
  await poller.tick()
  host.receive({ type: 'watch', view: 'realm', realm: 'alpha', run: null })
  const settled = await heard(host)
  for (const fleet of [first.fleet, settled.fleet]) {
    const alpha = fleet.hearths.find((hearth) => hearth.realm === 'alpha')
    assert.deepEqual(alpha.runs.map((row) => row.run_id), ['alpha-run-0002'])
    assert.equal(alpha.journal, '/new/forge.db')
    assert.equal(alpha.next, cursor)
  }

  const painted = mountClient('main')
  painted.push({ type: 'snapshot', ask: false, fleet: settled.fleet, overview: null, run: null })
  buttons(painted.tree()).find((button) => textOf(button).startsWith('alpha')).props.onClick()
  const runs = findAll(painted.tree(), (node) => node.props['aria-label'] === 'Runs')[0]
  assert.ok(buttons(runs).some((button) => textOf(button) === 'Show more runs'))
})

test('chief C4: a delayed inspect cannot patch the run now watched, nor reach it', async () => {
  const parked = windowOf([{ ...runOf('alpha', 0), seq: 5 }], 'needs-you:1:alpha-run-0000')
  const running = runningOf('alpha', 100, 12)
  const second = windowOf([running], null)
  const completed = runViewDoc({ seq: 13 })
  completed.summary = { ...completed.summary, status: 'completed', phase: 'done', park_reason: null }
  const done = { ...running, seq: 13, status: 'completed', phase: 'done', verdict: { standing: 'shipped', rule: 'REVIEW-CLEAN', residual: null, reason: null, text: 'CLEAN' } }
  const brokkr = stubBrokkr([
    { match: ['runs', `--run=${running.run_id}`], responses: [json(oneRow(done))] },
    { match: ['runs'], responses: [json(parked), json(second), json(parked)] },
    {
      match: ['inspect'],
      responses: [json(runningRead(12)), { ...json(completed), sleep_ms: 800 }, json(runViewDoc({ seq: 5 }))],
    },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const host = await attached(poller, { view: 'realm', realm: ONE_HEARTH, run: null })
  host.sent.length = 0
  host.receive({ type: 'more', realm: ONE_HEARTH })
  await heard(host)
  host.receive({ type: 'watch', view: 'realm', realm: ONE_HEARTH, run: { realm: ONE_HEARTH, run_id: running.run_id } })
  await heard(host)

  const inspections = () => brokkr.calls().filter((argv) => argv[0] === 'inspect').length
  host.sent.length = 0
  const ticking = poller.tick()
  for (let waited = 0; inspections() < 2; waited += 10) {
    assert.ok(waited < 10_000, 'the delayed inspect did not start within 10s')
    await new Promise((wake) => setTimeout(wake, 10))
  }
  host.receive({ type: 'watch', view: 'realm', realm: ONE_HEARTH, run: { realm: ONE_HEARTH, run_id: 'alpha-run-0000' } })
  const snapshot = await heard(host)
  await ticking
  const after = host.sent.slice()
  assert.deepEqual(after.filter((message) => message.type === 'run'), [], 'the old run message is not sent to the new watch')
  const fleet = after.find((message) => message.type === 'fleet')?.fleet
  assert.ok(fleet, 'the fresh summary still reaches the fleet')
  const alpha = fleet.hearths[0]
  const row = (runId) => alpha.runs.find((each) => each.run_id === runId)
  assert.deepEqual([row('alpha-run-0000').seq, row('alpha-run-0000').verdict.standing], [5, 'parked'])
  assert.deepEqual([row(running.run_id).seq, row(running.run_id).verdict.standing], [13, 'shipped'])
  assert.equal(snapshot.run.run_id, 'alpha-run-0000')

  const painted = mountClient('main')
  painted.push({ type: 'snapshot', ask: false, fleet, overview: null, run: snapshot.run })
  buttons(painted.tree()).find((button) => textOf(button).startsWith(ONE_HEARTH)).props.onClick()
  buttons(painted.tree()).find((button) => textOf(button).startsWith('alpha run 0')).props.onClick()
  assert.equal(findAll(painted.tree(), (node) => node.props['aria-label'] === 'Graph').length, 1)
})

test('chief C5: a fresh stop replaces the cached review park’s rule and words', async () => {
  const page = windowOf([runOf('alpha', 0)], null)
  const stopped = runViewDoc({ seq: 13 })
  stopped.summary = { ...stopped.summary, status: 'stopped', phase: 'stop', park_reason: null }
  stopped.ruling = { ...stopped.ruling, rule_id: 'VERIFY-FAIL-TWICE', from: 'verify' }
  const held = { ...runOf('alpha', 0), seq: 13, status: 'stopped', phase: 'stop', verdict: { standing: 'stopped', rule: 'VERIFY-FAIL-TWICE', residual: null, reason: null, text: 'FAIL-TWICE' } }
  const brokkr = stubBrokkr([
    { match: ['runs', '--run=alpha-run-0000'], responses: [json(oneRow(held))] },
    { match: ['runs'], responses: [json(page)] },
    { match: ['inspect'], responses: [json(runViewDoc({ seq: 12 })), json(stopped)] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const host = await attached(poller, {
    view: 'realm',
    realm: ONE_HEARTH,
    run: { realm: ONE_HEARTH, run_id: 'alpha-run-0000' },
  })
  host.sent.length = 0
  await poller.tick()
  const fleet = host.sent.find((message) => message.type === 'fleet').fleet
  const run = host.sent.find((message) => message.type === 'run').run
  const row = fleet.hearths[0].runs[0]
  assert.deepEqual(
    [row.verdict.standing, row.verdict.rule, row.verdict.text, row.verdict.reason],
    ['stopped', 'VERIFY-FAIL-TWICE', 'FAIL-TWICE', null],
  )

  const painted = mountClient('main')
  painted.push({ type: 'snapshot', ask: false, fleet, overview: null, run })
  buttons(painted.tree()).find((button) => textOf(button).startsWith(ONE_HEARTH)).props.onClick()
  buttons(painted.tree()).find((button) => textOf(button).startsWith('alpha run 0')).props.onClick()
  const why = findAll(painted.tree(), (node) => node.props['aria-label'] === 'Why it waits')[0]
  assert.doesNotMatch(textOf(why), /SECURITY-RESI|REVIEW-SECURITY-RESIDUAL/)
  assert.match(textOf(why), /FAIL-TWICE/)
  assert.match(textOf(why), /stopped by VERIFY-FAIL-TWICE/)
})

/** A fresh inspect read of a running run: the summary the listing already states, with no ruling. */
const runningRead = (seq) => {
  const doc = runViewDoc({ seq })
  doc.summary = { ...doc.summary, status: 'running', status_word: 'running', standing: 'running', phase: 'review', park_reason: null }
  doc.ruling = null
  return doc
}

test('review R1: an unchanged parked read keeps the rule and verdict Brokkr derived', async () => {
  const page = windowOf([row({ seq: 0 })], null)
  const read = runViewDoc({ seq: 0 })
  read.ruling = { ...read.ruling, next: '?' }
  const brokkr = stubBrokkr([
    { match: ['runs'], responses: [json(page)] },
    { match: ['inspect'], responses: [json(read)] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const host = await attached(poller, { view: 'realm', realm: ONE_HEARTH, run: { realm: ONE_HEARTH, run_id: RUN } })
  const snapshot = host.sent[0]
  const held = snapshot.fleet.hearths[0].runs[0]
  assert.deepEqual([held.verdict.rule, held.verdict.text], ['REVIEW-SECURITY-RESIDUAL', 'SECURITY-RESI…'])

  const painted = mountClient('main')
  painted.push(snapshot)
  buttons(painted.tree()).find((button) => textOf(button).startsWith(ONE_HEARTH)).props.onClick()
  const runs = findAll(painted.tree(), (node) => node.props['aria-label'] === 'Runs')[0]
  const runButton = buttons(runs).find((button) => textOf(button).startsWith('Encrypt the token store'))
  assert.match(textOf(runButton), /SECURITY-RESI…/)
  runButton.props.onClick()
  const why = findAll(painted.tree(), (node) => node.props['aria-label'] === 'Why it waits')[0]
  assert.match(textOf(why), /REVIEW-SECURITY-RESIDUAL/)
})

test('review R2: a refused inspect keeps a displaced open run and paints the refusal and hint', async () => {
  const { first, running, second, arrival } = displaced()
  const words = 'brokkr inspect was refused: temporary read failure'
  const brokkr = stubBrokkr([
    { match: ['runs'], responses: [json(first), json(second), json(first), json(arrival)] },
    { match: ['inspect'], responses: [json(runViewDoc({ seq: 12 })), { stderr: words, code: 1 }] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const host = await attached(poller, { view: 'realm', realm: ONE_HEARTH, run: null })
  host.sent.length = 0
  host.receive({ type: 'more', realm: ONE_HEARTH })
  const more = await heard(host)
  host.receive({ type: 'watch', view: 'realm', realm: ONE_HEARTH, run: { realm: ONE_HEARTH, run_id: running.run_id } })
  const opened = await heard(host)
  assert.equal(opened.run.view.summary.seq, 12)

  host.sent.length = 0
  await poller.tick()
  const fleet = host.sent.find((message) => message.type === 'fleet')?.fleet
  const refused = host.sent.find((message) => message.type === 'run')?.run
  const kept = fleet?.hearths[0].runs.find((row) => row.run_id === running.run_id)
  assert.deepEqual([kept?.seq, refused?.refused, refused?.hint], [12, words, `brokkr inspect --run=${running.run_id}`])

  const painted = mountClient('main')
  painted.push({ type: 'snapshot', ask: false, fleet: more.fleet, overview: null, run: opened.run })
  buttons(painted.tree()).find((button) => textOf(button).startsWith(ONE_HEARTH)).props.onClick()
  buttons(painted.tree()).find((button) => textOf(button).startsWith('alpha run 100')).props.onClick()
  painted.push({ type: 'fleet', fleet })
  painted.push({ type: 'run', run: refused })
  assert.equal(findAll(painted.tree(), (node) => node.props['aria-label'] === 'Runs').length, 0)
  const error = findAll(painted.tree(), (node) => node.props.className === 'bk-error').map(textOf)
  assert.deepEqual(error, [words])
  const hint = findAll(painted.tree(), (node) => String(node.props.className).includes('bk-hint')).map(textOf)
  assert.deepEqual(hint, [refused.hint])
})

test('review R3: an unchanged first-page open run pushes no fleet and reuses the cached day', async () => {
  const page = windowOf([runningOf('alpha', 0, 12)], null)
  const brokkr = stubBrokkr([
    { match: ['runs'], responses: [json(page)] },
    { match: ['inspect'], responses: [json(runningRead(12))] },
    { match: ['queue'], responses: [json([])] },
    { match: ['muninn'], responses: [json([])] },
    { match: ['analytics'], responses: [json(analyticsDoc())] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const realms = await attached(poller, { view: 'realms', run: null })
  const watcher = await attached(poller, { view: 'realm', realm: ONE_HEARTH, run: { realm: ONE_HEARTH, run_id: 'alpha-run-0000' } })
  const analytics = () => brokkr.calls().filter((argv) => argv[0] === 'analytics').length
  const before = analytics()
  realms.sent.length = 0
  watcher.sent.length = 0
  for (let poll = 0; poll < 3; poll += 1) await poller.tick()
  const pushes = (client) => client.sent.filter((message) => message.type === 'fleet').length
  assert.deepEqual([pushes(realms), pushes(watcher), analytics()], [0, 0, before])
})

test('review R3: an unchanged open run a partial page omits also pushes no fleet', async () => {
  const page = windowOf([runOf('alpha', 0)], 'needs-you:1:alpha-run-0000')
  const missing = 'alpha-run-0100'
  const held = runningOf('alpha', 100, 12)
  const brokkr = stubBrokkr([
    { match: ['runs', `--run=${missing}`], responses: [json(oneRow(held))] },
    { match: ['runs'], responses: [json(page)] },
    { match: ['inspect'], responses: [json(runningRead(12))] },
    { match: ['queue'], responses: [json([])] },
    { match: ['muninn'], responses: [json([])] },
    { match: ['analytics'], responses: [json(analyticsDoc())] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const realms = await attached(poller, { view: 'realms', run: null })
  const watcher = await attached(poller, { view: 'realm', realm: ONE_HEARTH, run: { realm: ONE_HEARTH, run_id: missing } })
  const analytics = () => brokkr.calls().filter((argv) => argv[0] === 'analytics').length
  const before = analytics()
  realms.sent.length = 0
  watcher.sent.length = 0
  for (let poll = 0; poll < 3; poll += 1) await poller.tick()
  const pushes = (client) => client.sent.filter((message) => message.type === 'fleet').length
  assert.deepEqual([pushes(realms), pushes(watcher), analytics()], [0, 0, before])
})

test('review R4: a row read from inspect takes the timestamps its journal records', async () => {
  const listed = row({ run_id: 'listed-run', title: 'same feature', feature: 'same feature', seq: 0 })
  const page = windowOf([listed], 'needs-you:1:listed-run')
  const read = runViewDoc({ seq: 12 })
  read.summary = { ...read.summary, run_id: 'missing-run', feature: 'same feature' }
  const served = row({ run_id: 'missing-run', feature: 'same feature', title: 'same feature', seq: 12, created_at: '2026-10-13T09:00:00Z', last_recorded_at: '2026-10-13T09:00:00Z' })
  const brokkr = stubBrokkr([
    { match: ['runs', '--run=missing-run'], responses: [json(oneRow(served))] },
    { match: ['runs'], responses: [json(page)] },
    { match: ['inspect'], responses: [json(read)] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const host = await attached(poller, { view: 'realm', realm: ONE_HEARTH, run: { realm: ONE_HEARTH, run_id: 'missing-run' } })
  const snapshot = host.sent[0]
  const missing = snapshot.fleet.hearths[0].runs.find((row) => row.run_id === 'missing-run')
  assert.deepEqual([missing.created_at, missing.last_recorded_at], ['2026-10-13T09:00:00Z', '2026-10-13T09:00:00Z'])

  const painted = mountClient('main')
  painted.push(snapshot)
  buttons(painted.tree()).find((button) => textOf(button).startsWith(ONE_HEARTH)).props.onClick()
  const runs = findAll(painted.tree(), (node) => node.props['aria-label'] === 'Runs')[0]
  assert.doesNotMatch(textOf(runs), /Invalid Date/)
  assert.match(textOf(runs), /1h/)
})

test('review R1: an omitted run paints the row Brokkr served it, absent destination and all', async () => {
  const page = windowOf([runOf('alpha', 0)], 'needs-you:1:alpha-run-0000')
  const read = runViewDoc({ seq: 12 })
  read.ruling = { ...read.ruling, next: '?' }
  const served = row({ run_id: 'alpha-run-0100', title: 'Encrypt the token store', feature: 'Encrypt the token store' })
  const brokkr = stubBrokkr([
    { match: ['runs', '--run=alpha-run-0100'], responses: [json(oneRow(served))] },
    { match: ['runs'], responses: [json(page)] },
    { match: ['inspect'], responses: [json(read)] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const host = await attached(poller, { view: 'realm', realm: ONE_HEARTH, run: { realm: ONE_HEARTH, run_id: 'alpha-run-0100' } })
  const held = host.sent[0].fleet.hearths[0].runs.find((row) => row.run_id === 'alpha-run-0100')
  assert.deepEqual([held.verdict.rule, held.verdict.text], ['REVIEW-SECURITY-RESIDUAL', 'SECURITY-RESI…'])
})

/** A hundred parked first-page runs ending at cursor C, and the last page holding `last` for that cursor. */
function retained(last) {
  const parked = Array.from({ length: FLEET_WINDOW }, (_, index) => runOf('alpha', index))
  return {
    first: windowOf(parked, `needs-you:${FLEET_WINDOW}:alpha-run-0099`),
    second: windowOf([last], null),
  }
}

test('review M1: a retained run’s fresh ruling and words move with its seq', async () => {
  const run = { ...runOf('alpha', 100), seq: 12 }
  const { first, second } = retained(run)
  const before = runViewDoc({ seq: 12 })
  before.summary = { ...before.summary, run_id: run.run_id }
  const after = runViewDoc({ seq: 25 })
  after.summary = { ...after.summary, run_id: run.run_id, park_reason: 'review exhausted its reforge budget' }
  after.ruling = { ...after.ruling, rule_id: 'REVIEW-REFORGE-EXHAUSTED-MEDIUM', from: 'review', next: '?' }
  const fresh = {
    ...run,
    seq: 25,
    verdict: { standing: 'parked', rule: 'REVIEW-REFORGE-EXHAUSTED-MEDIUM', residual: 'serious', reason: 'review exhausted its reforge budget', text: 'REFORGE-EXHAU…' },
  }
  const brokkr = stubBrokkr([
    { match: ['runs', `--run=${run.run_id}`], responses: [json(oneRow(fresh))] },
    { match: ['runs'], responses: [json(first), json(second), json(first)] },
    { match: ['inspect'], responses: [json(before), json(after)] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const host = await attached(poller, { view: 'realm', realm: ONE_HEARTH, run: null })
  host.sent.length = 0
  host.receive({ type: 'more', realm: ONE_HEARTH })
  await heard(host)
  host.receive({ type: 'watch', view: 'realm', realm: ONE_HEARTH, run: { realm: ONE_HEARTH, run_id: run.run_id } })
  await heard(host)

  host.sent.length = 0
  await poller.tick()
  const fleet = host.sent.find((message) => message.type === 'fleet')?.fleet
  const update = host.sent.find((message) => message.type === 'run')?.run
  const row = fleet.hearths[0].runs.find((each) => each.run_id === run.run_id)
  assert.deepEqual(
    [row.seq, row.verdict.rule, row.verdict.reason, row.verdict.text],
    [25, 'REVIEW-REFORGE-EXHAUSTED-MEDIUM', 'review exhausted its reforge budget', 'REFORGE-EXHAU…'],
  )

  const painted = mountClient('main')
  painted.push({ type: 'snapshot', ask: false, fleet, overview: null, run: null })
  buttons(painted.tree()).find((button) => textOf(button).startsWith(ONE_HEARTH)).props.onClick()
  const runs = findAll(painted.tree(), (node) => node.props['aria-label'] === 'Runs')[0]
  const button = buttons(runs).find((each) => textOf(each).startsWith('alpha run 100'))
  assert.match(textOf(button), /REFORGE-EXHAU…/)
  button.props.onClick()
  painted.push({ type: 'run', run: update })
  const why = findAll(painted.tree(), (node) => node.props['aria-label'] === 'Why it waits')[0]
  assert.match(textOf(why), /REVIEW-REFORGE-EXHAUSTED-MEDIUM/)
  assert.match(textOf(why), /review exhausted its reforge budget/)
  assert.doesNotMatch(textOf(why), /SECURITY-RESI/)
})

test('review M2: a retained run’s completed stamp moves its time bucket', async () => {
  const run = { ...runningOf('alpha', 100, 12), last_recorded_at: '2026-10-10T09:00:00Z' }
  const { first, second } = retained(run)
  const before = runningRead(12)
  before.summary = { ...before.summary, run_id: run.run_id }
  const after = runViewDoc({ seq: 25 })
  after.summary = { ...after.summary, run_id: run.run_id, status: 'completed', status_word: 'shipped', standing: 'shipped', phase: 'done', park_reason: null }
  after.ruling = { ...after.ruling, rule_id: 'REVIEW-CLEAN', from: 'review', next: 'done' }
  const fresh = {
    ...run,
    seq: 25,
    status: 'completed',
    phase: 'done',
    last_recorded_at: '2026-10-13T09:00:00Z',
    verdict: { standing: 'shipped', rule: 'REVIEW-CLEAN', residual: null, reason: null, text: 'CLEAN' },
  }
  const brokkr = stubBrokkr([
    { match: ['runs', `--run=${run.run_id}`], responses: [json(oneRow(fresh))] },
    { match: ['runs'], responses: [json(first), json(second), json(first)] },
    { match: ['inspect'], responses: [json(before), json(after)] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const host = await attached(poller, { view: 'realm', realm: ONE_HEARTH, run: null })
  host.sent.length = 0
  host.receive({ type: 'more', realm: ONE_HEARTH })
  await heard(host)
  host.receive({ type: 'watch', view: 'realm', realm: ONE_HEARTH, run: { realm: ONE_HEARTH, run_id: run.run_id } })
  await heard(host)

  host.sent.length = 0
  await poller.tick()
  const fleet = host.sent.find((message) => message.type === 'fleet')?.fleet
  const row = fleet.hearths[0].runs.find((each) => each.run_id === run.run_id)
  assert.deepEqual([row.seq, row.last_recorded_at, row.verdict.standing], [25, '2026-10-13T09:00:00Z', 'shipped'])

  const painted = mountClient('main')
  painted.push({ type: 'snapshot', ask: false, fleet, overview: null, run: null })
  buttons(painted.tree()).find((button) => textOf(button).startsWith(ONE_HEARTH)).props.onClick()
  const runs = findAll(painted.tree(), (node) => node.props['aria-label'] === 'Runs')[0]
  const button = buttons(runs).find((each) => textOf(each).startsWith('alpha run 100'))
  assert.match(textOf(button), /1h/)
  const groups = findAll(runs, (node) => node.props.className === 'bk-grp').map(textOf)
  assert.ok(groups.includes('Last 24 hours · 1+'), JSON.stringify(groups))
  assert.ok(!groups.includes('Older · 1+'), JSON.stringify(groups))
})

test('review M3: a row’s verdict text is Brokkr’s display-column clamp, not the host’s slice', async () => {
  const run = { ...runOf('alpha', 100), seq: 12 }
  const { first, second } = retained(run)
  const rule = 'REVIEW-审审审审审审审审'
  const before = runViewDoc({ seq: 12 })
  before.summary = { ...before.summary, run_id: run.run_id }
  const after = runViewDoc({ seq: 13 })
  after.summary = { ...after.summary, run_id: run.run_id }
  after.ruling = { ...after.ruling, rule_id: rule, from: 'review', next: '?' }
  const fresh = { ...run, seq: 13, verdict: { standing: 'parked', rule, residual: null, reason: null, text: '审审审审审审…' } }
  const brokkr = stubBrokkr([
    { match: ['runs', `--run=${run.run_id}`], responses: [json(oneRow(fresh))] },
    { match: ['runs'], responses: [json(first), json(second), json(first)] },
    { match: ['inspect'], responses: [json(before), json(after)] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const host = await attached(poller, { view: 'realm', realm: ONE_HEARTH, run: null })
  host.sent.length = 0
  host.receive({ type: 'more', realm: ONE_HEARTH })
  await heard(host)
  host.receive({ type: 'watch', view: 'realm', realm: ONE_HEARTH, run: { realm: ONE_HEARTH, run_id: run.run_id } })
  await heard(host)

  host.sent.length = 0
  await poller.tick()
  const fleet = host.sent.find((message) => message.type === 'fleet')?.fleet
  const row = fleet.hearths[0].runs.find((each) => each.run_id === run.run_id)
  assert.deepEqual([row.verdict.rule, row.verdict.text], [rule, '审审审审审审…'])

  const painted = mountClient('main')
  painted.push({ type: 'snapshot', ask: false, fleet, overview: null, run: null })
  buttons(painted.tree()).find((button) => textOf(button).startsWith(ONE_HEARTH)).props.onClick()
  const runs = findAll(painted.tree(), (node) => node.props['aria-label'] === 'Runs')[0]
  const button = buttons(runs).find((each) => textOf(each).startsWith('alpha run 100'))
  assert.match(textOf(button), /审审审审审审…/)
  assert.doesNotMatch(textOf(button), /审审审审审审审审/)
})

test('review M4: an inspect from a replaced journal publishes nothing', async () => {
  const runId = 'alpha-run-0000'
  const rowAt = (seq) => ({ ...row({ run_id: runId, title: 'alpha run 0', feature: 'alpha run 0' }), seq })
  const oldPage = windowOf([rowAt(12)], null)
  const newPage = windowOf([rowAt(25)], null)
  const betaPage = windowOf([runOf('beta', 0)], null)
  const world = (journal) => ({
    journal: './.forge/forge.db',
    map: './realms.json',
    realms: ['alpha', 'beta'].map((name) => ({ default_branch: 'main', head: '0000000', journal: name === 'alpha' ? journal : '/hearths/beta/forge.db', name, path: `../${name}` })),
  })
  const oldRead = runViewDoc({ seq: 12 })
  oldRead.summary = { ...oldRead.summary, run_id: runId }
  const newRead = runViewDoc({ seq: 25 })
  newRead.summary = { ...newRead.summary, run_id: runId }
  const brokkr = stubBrokkr([
    { match: ['realms'], responses: [json(world('/hearths/alpha/old.db')), json(world('/hearths/alpha/new.db')), json(world('/hearths/alpha/new.db'))] },
    { match: ['runs', '--db=/hearths/alpha/old.db'], responses: [json(oldPage)] },
    { match: ['runs', '--db=/hearths/alpha/new.db'], responses: [json(newPage)] },
    { match: ['runs', '--db=/hearths/beta/forge.db'], responses: [json(betaPage)] },
    { match: ['inspect'], responses: [{ ...json(oldRead), sleep_ms: 700 }, json(newRead)] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const client = peer()
  client.receive = poller.attach(client)
  const inspections = () => brokkr.calls().filter((argv) => argv[0] === 'inspect').length
  client.receive({ type: 'watch', view: 'realm', realm: 'alpha', run: { realm: 'alpha', run_id: runId } })
  for (let waited = 0; inspections() === 0; waited += 10) {
    assert.ok(waited < 10_000, 'the old journal’s inspect did not start within 10s')
    await new Promise((wake) => setTimeout(wake, 10))
  }
  client.receive({ type: 'watch', view: 'realm', realm: 'alpha', run: { realm: 'alpha', run_id: runId } })
  for (let waited = 0; client.sent.length === 0; waited += 10) {
    assert.ok(waited < 10_000, 'the new journal’s snapshot did not arrive within 10s')
    await new Promise((wake) => setTimeout(wake, 10))
  }
  await new Promise((wake) => setTimeout(wake, 900))

  assert.equal(client.sent.length, 1, 'the obsolete snapshot paints nothing')
  const [snapshot] = client.sent
  const alpha = snapshot.fleet.hearths.find((hearth) => hearth.realm === 'alpha')
  assert.deepEqual([alpha.journal, alpha.runs[0].seq, snapshot.run.view.summary.seq], ['/hearths/alpha/new.db', 25, 25])
})

test('review M5: a retained last page never proves a new running run is gone', async () => {
  const parked = Array.from({ length: FLEET_WINDOW }, (_, index) => runOf('alpha', index))
  const first = windowOf(parked, `needs-you:${FLEET_WINDOW}:alpha-run-0099`)
  const second = windowOf([runOf('alpha', 200)], null)
  const running = runningOf('alpha', 300, 12)
  const reads = [12, 13, 14].map((seq) => {
    const read = runningRead(seq)
    read.summary = { ...read.summary, run_id: running.run_id }
    return read
  })
  const brokkr = stubBrokkr([
    { match: ['runs', `--run=${running.run_id}`], responses: [12, 13, 14].map((seq) => json(oneRow(runningOf('alpha', 300, seq)))) },
    { match: ['runs'], responses: [json(first), json(second), json(first)] },
    { match: ['inspect'], responses: reads.map((read) => json(read)) },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const host = await attached(poller, { view: 'realm', realm: ONE_HEARTH, run: null })
  host.sent.length = 0
  host.receive({ type: 'more', realm: ONE_HEARTH })
  await heard(host)
  host.receive({ type: 'watch', view: 'realm', realm: ONE_HEARTH, run: { realm: ONE_HEARTH, run_id: running.run_id } })
  const snapshot = await heard(host)
  const inspections = () => brokkr.calls().filter((argv) => argv[0] === 'inspect').length
  const counts = [inspections()]
  const updates = [snapshot.run.view.summary.seq]
  assert.ok(snapshot.fleet.hearths[0].runs.some((row) => row.run_id === running.run_id), 'the new running run is held')

  for (let poll = 0; poll < 2; poll += 1) {
    host.sent.length = 0
    await poller.tick()
    counts.push(inspections())
    updates.push(host.sent.find((message) => message.type === 'run')?.run.view.summary.seq ?? null)
  }
  assert.deepEqual(counts, [1, 2, 3])
  assert.deepEqual(updates, [12, 13, 14])
  const fleet = host.sent.filter((message) => message.type === 'fleet').at(-1)?.fleet
  assert.equal(fleet.hearths[0].runs.find((row) => row.run_id === running.run_id).seq, 14)
})
