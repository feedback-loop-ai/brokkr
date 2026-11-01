// Paging and Show more: a hearth's listing is a window, its next page is
// read only when the operator asks, a refresh keeps it only while the hearth
// is on screen, and behind makes the painted count a lower bound. The
// fixtures these tests share live in tests/support/poller.js.
import assert from 'node:assert/strict'
import { mkdirSync, mkdtempSync, realpathSync, symlinkSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { test } from 'node:test'

import { brokkrOf } from '../lib/host/brokkr.js'
import { FLEET_WINDOW } from '../lib/host/config.js'
import { pollerOf } from '../lib/host/poller.js'
import { ONE_HEARTH } from '../lib/host/readouts.js'
import { buttons, findAll, mountClient, textOf } from './support/client.js'
import { OTHER, attached, heard, realmsDoc, runOf, runningOf, windowOf } from './support/poller.js'
import { fleetDoc, json, row, runViewDoc, stubBrokkr } from './support/world.js'

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
  assert.deepEqual(brokkr.calls().filter((argv) => argv[0] === 'runs'), [['runs', '--json', '--db=/hearths/a/forge.db', `--limit=${FLEET_WINDOW}`]])
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
