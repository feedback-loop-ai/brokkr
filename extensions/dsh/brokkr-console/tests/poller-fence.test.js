// The journal fence: a result publishes only while the journal it was read
// from is still the one its realm names, whatever the world's shape. A map
// moved, vanished or appeared while a read was in flight makes that read
// unlabelable. The fixtures these tests share live in tests/support/poller.js.
import assert from 'node:assert/strict'
import { test } from 'node:test'

import { brokkrOf } from '../lib/host/brokkr.js'
import { FLEET_WINDOW } from '../lib/host/config.js'
import { pollerOf } from '../lib/host/poller.js'
import { ONE_HEARTH } from '../lib/host/readouts.js'
import { buttons, findAll, mountClient, textOf } from './support/client.js'
import { attached, heard, oneRow, peer, realmsDoc, realmsMap, runOf, runningOf, runningRead, windowOf } from './support/poller.js'
import { json, row, runViewDoc, stubBrokkr } from './support/world.js'

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
    { match: ['realms'], responses: [json(realmsWith('/hearths/alpha/forge.db')), json(realmsWith('/hearths/alpha/forge.db')), json(realmsWith('/hearths/alpha/forge.db')), json(realmsWith('/hearths/alpha-new/forge.db'))] },
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

test('review M4: an inspect from a replaced journal publishes nothing', async () => {
  const runId = 'alpha-run-0000'
  const rowAt = (seq) => ({ ...row({ run_id: runId, title: 'alpha run 0', feature: 'alpha run 0' }), seq })
  const oldPage = windowOf([rowAt(12)], null)
  const newPage = windowOf([rowAt(25)], null)
  const betaPage = windowOf([runOf('beta', 0)], null)
  const world = (journal) => realmsMap(journal)
  const oldRead = runViewDoc({ seq: 12 })
  oldRead.summary = { ...oldRead.summary, run_id: runId }
  const newRead = runViewDoc({ seq: 25 })
  newRead.summary = { ...newRead.summary, run_id: runId }
  const brokkr = stubBrokkr([
    { match: ['realms'], responses: [json(world('/hearths/alpha/old.db')), json(world('/hearths/alpha/old.db')), json(world('/hearths/alpha/new.db'))] },
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

test('review M4: a delayed run-row read from a replaced journal paints no snapshot', async () => {
  const watched = { ...runOf('alpha', 0), seq: 12 }
  const oldPage = windowOf([watched], null)
  const newPage = windowOf([{ ...watched, seq: 25 }], null)
  const betaPage = windowOf([runOf('beta', 0)], null)
  const world = (journal) => realmsMap(journal)
  const oldRead = runViewDoc({ seq: 13 })
  oldRead.summary = { ...oldRead.summary, run_id: watched.run_id }
  const newRead = runViewDoc({ seq: 25 })
  newRead.summary = { ...newRead.summary, run_id: watched.run_id }
  const brokkr = stubBrokkr([
    { match: ['runs', `--run=${watched.run_id}`], responses: [{ ...json(oneRow({ ...watched, seq: 13 })), sleep_ms: 700 }] },
    { match: ['runs', '--db=/hearths/alpha/old.db'], responses: [json(oldPage)] },
    { match: ['runs', '--db=/hearths/alpha/new.db'], responses: [json(newPage)] },
    { match: ['runs', '--db=/hearths/beta/forge.db'], responses: [json(betaPage)] },
    { match: ['realms'], responses: [json(world('/hearths/alpha/old.db')), json(world('/hearths/alpha/old.db')), json(world('/hearths/alpha/old.db')), json(world('/hearths/alpha/new.db'))] },
    { match: ['inspect'], responses: [json(oldRead), json(newRead)] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const client = peer()
  client.receive = poller.attach(client)
  client.receive({ type: 'watch', view: 'realm', realm: 'alpha', run: { realm: 'alpha', run_id: watched.run_id } })
  const started = () => brokkr.calls().some((argv) => argv[0] === 'runs' && argv.includes(`--run=${watched.run_id}`))
  for (let waited = 0; !started(); waited += 10) {
    assert.ok(waited < 10_000, 'the old journal’s run-row read did not start within 10s')
    await new Promise((wake) => setTimeout(wake, 10))
  }

  // The realm moves to a new journal while the old row is still held open.
  client.sent.length = 0
  await poller.tick()
  const moved = client.sent.find((message) => message.type === 'run')
  assert.equal(moved?.run.view.summary.seq, 25, 'the current journal’s read reaches the client')

  await new Promise((wake) => setTimeout(wake, 900))
  assert.deepEqual(client.sent.filter((message) => message.type === 'snapshot'), [], 'the obsolete snapshot paints nothing')
  assert.deepEqual(client.sent.filter((message) => message.type === 'run' && message.run.view.summary.seq === 13), [], 'the old journal’s row never reaches the client')
})

test('review M4: a delayed first page cannot roll a repointed realm back', async () => {
  const oldPage = windowOf([{ ...runOf('alpha', 0), seq: 12 }], null)
  const newPage = windowOf([{ ...runOf('alpha', 0), seq: 25 }], null)
  const world = (journal) => realmsMap(journal, ['alpha'])
  const brokkr = stubBrokkr([
    { match: ['runs'], responses: [{ ...json(oldPage), sleep_ms: 700 }, json(newPage)] },
    { match: ['realms'], responses: [json(world('/hearths/alpha/old.db')), json(world('/hearths/alpha/new.db')), json(world('/hearths/alpha/new.db'))] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const first = peer()
  first.receive = poller.attach(first)
  first.receive({ type: 'watch', view: 'realm', realm: 'alpha', run: null })
  const pages = () => brokkr.calls().filter((argv) => argv[0] === 'runs').length
  for (let waited = 0; pages() === 0; waited += 10) {
    assert.ok(waited < 10_000, 'the old first page did not start within 10s')
    await new Promise((wake) => setTimeout(wake, 10))
  }

  // A second client reads the realm after the journal moved, before the first
  // page read under the old journal lands.
  const second = await attached(poller, { view: 'realm', realm: 'alpha', run: null })
  assert.equal(second.sent[0].fleet.hearths[0].journal, '/hearths/alpha/new.db')
  await new Promise((wake) => setTimeout(wake, 900))
  assert.equal(first.sent.length, 1, 'the delayed page paints one snapshot, not a rollback')
  assert.deepEqual(first.sent[0].fleet.hearths.map((hearth) => [hearth.journal, hearth.runs[0].seq]), [['/hearths/alpha/new.db', 25]])
})

test('review M4: a Show more page from a replaced journal paints nothing', async () => {
  const cursor = 'needs-you:1:alpha-run-0000'
  const first = windowOf([{ ...runOf('alpha', 0), seq: 12 }], cursor)
  const second = windowOf([runOf('alpha', 1)], null)
  const fresh = windowOf([{ ...runOf('alpha', 2), seq: 25 }], cursor)
  const world = (journal) => realmsMap(journal, ['alpha'])
  const brokkr = stubBrokkr([
    { match: ['runs'], responses: [json(first), { ...json(second), sleep_ms: 700 }, json(fresh)] },
    { match: ['realms'], responses: [json(world('/hearths/alpha/old.db')), json(world('/hearths/alpha/old.db')), json(world('/hearths/alpha/new.db'))] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const client = await attached(poller, { view: 'realm', realm: 'alpha', run: null })
  client.sent.length = 0
  client.receive({ type: 'more', realm: 'alpha' })
  const reads = () => brokkr.calls().filter((argv) => argv[0] === 'runs').length
  for (let waited = 0; reads() < 2; waited += 10) {
    assert.ok(waited < 10_000, 'the Show more page did not start within 10s')
    await new Promise((wake) => setTimeout(wake, 10))
  }

  // The realm moves to a new journal with the same cursor while the old
  // journal’s page is still in flight.
  await poller.tick()
  client.sent.length = 0
  await new Promise((wake) => setTimeout(wake, 900))
  assert.deepEqual(client.sent, [], 'the replaced journal’s page paints nothing')
})

test('review M4: a poll’s queued update from a replaced journal reaches no client', async () => {
  const watched = runningOf('alpha', 0, 12)
  const other = runningOf('alpha', 1, 12)
  const oldPage = windowOf([watched, other], null)
  const newPage = windowOf([{ ...watched, seq: 25 }, other], null)
  const betaPage = windowOf([runOf('beta', 0)], null)
  const world = (journal) => realmsMap(journal)
  const held = runningRead(12)
  held.summary = { ...held.summary, run_id: watched.run_id }
  const moved = runningRead(13)
  moved.summary = { ...moved.summary, run_id: watched.run_id }
  const late = runningRead(12)
  late.summary = { ...late.summary, run_id: other.run_id }
  const brokkr = stubBrokkr([
    { match: ['runs', `--run=${watched.run_id}`], responses: [json(oneRow({ ...watched, seq: 13 }))] },
    { match: ['runs', '--db=/hearths/alpha/old.db'], responses: [json(oldPage)] },
    { match: ['runs', '--db=/hearths/alpha/new.db'], responses: [json(newPage)] },
    { match: ['runs', '--db=/hearths/beta/forge.db'], responses: [json(betaPage)] },
    {
      match: ['realms'],
      responses: [json(world('/hearths/alpha/old.db')), json(world('/hearths/alpha/old.db')), json(world('/hearths/alpha/old.db')), json(world('/hearths/alpha/old.db')), json(world('/hearths/alpha/old.db')), json(world('/hearths/alpha/old.db')), json(world('/hearths/alpha/old.db')), json(world('/hearths/alpha/new.db'))],
    },
    { match: ['inspect'], responses: [json(held), json(moved), { ...json(late), sleep_ms: 700 }] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const client = await attached(poller, { view: 'realm', realm: 'alpha', run: { realm: 'alpha', run_id: watched.run_id } })
  const inspections = () => brokkr.calls().filter((argv) => argv[0] === 'inspect').length

  // A poll queues the watched run's moved read, then waits on the other
  // first-page running run before it can send anything.
  const ticking = poller.tick()
  for (let waited = 0; inspections() < 3; waited += 10) {
    assert.ok(waited < 10_000, 'the poll’s second run did not start within 10s')
    await new Promise((wake) => setTimeout(wake, 10))
  }
  client.sent.length = 0

  // The realm moves to a new journal while the poll waits.
  const second = await attached(poller, { view: 'realm', realm: 'alpha', run: null })
  assert.equal(second.sent[0].fleet.hearths.find((hearth) => hearth.realm === 'alpha').journal, '/hearths/alpha/new.db')
  await ticking

  assert.deepEqual(client.sent.filter((message) => message.type === 'run'), [], 'the queued old-journal update reaches no client')
  const fleet = client.sent.find((message) => message.type === 'fleet')?.fleet
  assert.equal(fleet?.hearths.find((hearth) => hearth.realm === 'alpha').journal, '/hearths/alpha/new.db', 'the poll paints the current journal')
})

test('review M4: a newer watch drops the snapshot its older watch read', async () => {
  const oldPage = windowOf([{ ...runOf('alpha', 0), seq: 12 }], null)
  const newPage = windowOf([{ ...runOf('alpha', 0), seq: 25 }], null)
  const world = (journal) => realmsMap(journal, ['alpha'])
  const brokkr = stubBrokkr([
    { match: ['runs'], responses: [{ ...json(oldPage), sleep_ms: 700 }, json(newPage)] },
    { match: ['realms'], responses: [json(world('/hearths/alpha/old.db')), json(world('/hearths/alpha/new.db')), json(world('/hearths/alpha/new.db'))] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const client = peer()
  client.receive = poller.attach(client)
  client.receive({ type: 'watch', view: 'realm', realm: 'alpha', run: null })
  const pages = () => brokkr.calls().filter((argv) => argv[0] === 'runs').length
  for (let waited = 0; pages() === 0; waited += 10) {
    assert.ok(waited < 10_000, 'the old first page did not start within 10s')
    await new Promise((wake) => setTimeout(wake, 10))
  }

  // The operator asks again before the first page lands: the newer watch is
  // the one that paints.
  client.receive({ type: 'watch', view: 'realm', realm: 'alpha', run: null })
  for (let waited = 0; client.sent.length === 0; waited += 10) {
    assert.ok(waited < 10_000, 'the new journal’s snapshot did not arrive within 10s')
    await new Promise((wake) => setTimeout(wake, 10))
  }
  await new Promise((wake) => setTimeout(wake, 900))
  assert.equal(client.sent.length, 1, 'the replaced watch paints one snapshot')
  assert.deepEqual(client.sent[0].fleet.hearths.map((hearth) => [hearth.journal, hearth.runs[0].seq]), [['/hearths/alpha/new.db', 25]])
})

test('review F1: a one-realm map names its realm journal, not the map’s world journal', async () => {
  const page = windowOf([runOf('alpha', 0)], null)
  const realms = {
    journal: './.forge/world.db',
    map: './realms.json',
    realms: [{ default_branch: 'main', head: '0000000', journal: '/hearths/alpha/realm.db', name: 'alpha', path: '../alpha' }],
  }
  const brokkr = stubBrokkr([
    { match: ['realms'], responses: [json(realms)] },
    { match: ['runs'], responses: [json(page)] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const client = await attached(poller, { view: 'realm', realm: 'alpha', run: null })
  assert.deepEqual(
    brokkr.calls().filter((argv) => argv[0] === 'runs'),
    [['runs', '--json', '--db=/hearths/alpha/realm.db', `--limit=${FLEET_WINDOW}`]],
  )
  assert.equal(client.sent[0].fleet.hearths[0].journal, '/hearths/alpha/realm.db')
})

test('review F1: a one-realm map repointed before the inspect paints no snapshot from the old journal', async () => {
  const runId = 'alpha-run-0000'
  const oldPage = windowOf([{ ...runOf('alpha', 0), seq: 12 }], null)
  const newPage = windowOf([{ ...runOf('alpha', 0), seq: 25 }], null)
  const world = (journal) => realmsMap(journal, ['alpha'])
  const newRead = runViewDoc({ seq: 25 })
  newRead.summary = { ...newRead.summary, run_id: runId }
  const served = row({ run_id: runId, title: 'alpha run 0', feature: 'alpha run 0', seq: 25 })
  const brokkr = stubBrokkr([
    { match: ['realms'], responses: [json(world('/hearths/alpha/old.db')), json(world('/hearths/alpha/old.db')), json(world('/hearths/alpha/new.db'))] },
    { match: ['runs', `--run=${runId}`], responses: [json(oneRow(served))] },
    { match: ['runs'], responses: [json(oldPage), json(newPage)] },
    { match: ['inspect'], responses: [json(newRead), json(newRead)] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const client = peer()
  client.receive = poller.attach(client)
  client.receive({ type: 'watch', view: 'realm', realm: 'alpha', run: { realm: 'alpha', run_id: runId } })
  const calls = (verb) => brokkr.calls().filter((argv) => argv[0] === verb).length
  for (let waited = 0; calls('inspect') === 0 || (calls('realms') < 3 && client.sent.length === 0); waited += 10) {
    assert.ok(waited < 10_000, 'the old journal’s inspect and its map re-read did not finish within 10s')
    await new Promise((wake) => setTimeout(wake, 10))
  }
  await new Promise((wake) => setTimeout(wake, 50))

  // The map moved before the inspect, so its new-journal view cannot be
  // painted under the old fleet: the snapshot is dropped whole.
  assert.deepEqual(client.sent, [], 'the repointed read paints nothing')
  await poller.tick()
  assert.equal(client.sent.filter((message) => message.type === 'snapshot').length, 0, 'no snapshot labels the old journal')
  const fleet = client.sent.find((message) => message.type === 'fleet')?.fleet
  assert.deepEqual(
    [fleet?.hearths[0].journal, fleet?.hearths[0].runs[0].seq],
    ['/hearths/alpha/new.db', 25],
    'the next poll paints the journal the run is now read from',
  )
})

test('review F1: a poll whose one-realm map moves while it reads paints nothing', async () => {
  const runId = 'alpha-run-0000'
  const oldPage = windowOf([{ ...runningOf('alpha', 0, 12) }], null)
  const movedPage = windowOf([{ ...runningOf('alpha', 0, 13) }], null)
  const world = (journal) => realmsMap(journal, ['alpha'])
  const oldRead = runningRead(12)
  oldRead.summary = { ...oldRead.summary, run_id: runId }
  const movedRead = runningRead(13)
  movedRead.summary = { ...movedRead.summary, run_id: runId }
  const brokkr = stubBrokkr([
    { match: ['realms'], responses: [json(world('/hearths/alpha/old.db')), json(world('/hearths/alpha/old.db')), json(world('/hearths/alpha/old.db')), json(world('/hearths/alpha/old.db')), json(world('/hearths/alpha/old.db')), json(world('/hearths/alpha/new.db'))] },
    { match: ['runs'], responses: [json(oldPage), json(movedPage)] },
    { match: ['inspect'], responses: [json(oldRead), { ...json(movedRead), sleep_ms: 500 }] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const client = await attached(poller, { view: 'realm', realm: 'alpha', run: { realm: 'alpha', run_id: runId } })
  client.sent.length = 0
  const inspections = () => brokkr.calls().filter((argv) => argv[0] === 'inspect').length

  // The poll reads a moved head, then the map moves while its inspect is in
  // flight: the poll's own page and runs are no longer labelable.
  const ticking = poller.tick()
  for (let waited = 0; inspections() < 2; waited += 10) {
    assert.ok(waited < 10_000, 'the poll’s inspect did not start within 10s')
    await new Promise((wake) => setTimeout(wake, 10))
  }
  await ticking
  assert.deepEqual(client.sent, [], 'the moved map’s poll paints nothing')
})

test('review F1: a one-realm map repointed while a Show more page is in flight paints nothing', async () => {
  const cursor = 'needs-you:1:alpha-run-0000'
  const first = windowOf([{ ...runOf('alpha', 0), seq: 12 }], cursor)
  const second = windowOf([runOf('alpha', 1)], null)
  const world = (journal) => realmsMap(journal, ['alpha'])
  const brokkr = stubBrokkr([
    { match: ['runs'], responses: [json(first), { ...json(second), sleep_ms: 500 }] },
    { match: ['realms'], responses: [json(world('/hearths/alpha/old.db')), json(world('/hearths/alpha/old.db')), json(world('/hearths/alpha/new.db'))] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const client = await attached(poller, { view: 'realm', realm: 'alpha', run: null })
  client.sent.length = 0
  client.receive({ type: 'more', realm: 'alpha' })
  const reads = () => brokkr.calls().filter((argv) => argv[0] === 'runs').length
  for (let waited = 0; reads() < 2; waited += 10) {
    assert.ok(waited < 10_000, 'the Show more page did not start within 10s')
    await new Promise((wake) => setTimeout(wake, 10))
  }

  // The map moves while the page is in flight; its rows cannot be appended
  // under the old journal, even though the cached fleet still names it.
  await new Promise((wake) => setTimeout(wake, 700))
  assert.deepEqual(client.sent, [], 'the moved map’s page paints nothing')
})

test('review F2: a replaced watch’s refused fleet read paints no error banner', async () => {
  const words = 'brokkr runs was refused: the journal could not be read'
  const page = windowOf([runOf('alpha', 0)], null)
  const brokkr = stubBrokkr([
    { match: ['runs'], responses: [{ stderr: words, code: 1, sleep_ms: 700 }, json(page)] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const client = peer()
  client.receive = poller.attach(client)
  client.receive({ type: 'watch', view: 'realm', realm: ONE_HEARTH, run: null })
  const reads = () => brokkr.calls().filter((argv) => argv[0] === 'runs').length
  for (let waited = 0; reads() === 0; waited += 10) {
    assert.ok(waited < 10_000, 'the refused fleet read did not start within 10s')
    await new Promise((wake) => setTimeout(wake, 10))
  }

  // A newer watch answers while the older watch’s read is still refused.
  client.receive({ type: 'watch', view: 'realm', realm: ONE_HEARTH, run: null })
  for (let waited = 0; client.sent.length === 0; waited += 10) {
    assert.ok(waited < 10_000, 'the newer watch’s snapshot did not arrive within 10s')
    await new Promise((wake) => setTimeout(wake, 10))
  }
  await new Promise((wake) => setTimeout(wake, 900))
  assert.deepEqual(client.sent.map((message) => message.type), ['snapshot'], 'the replaced watch paints no error beside it')
  assert.equal(client.sent[0].fleet.hearths[0].runs.length, 1)
})

test('review F2: a one-hearth world’s refused fleet read keeps its detail, not an error banner', async () => {
  const words = 'brokkr runs was refused: the journal could not be read'
  const brokkr = stubBrokkr([{ match: ['runs'], responses: [{ stderr: words, code: 1 }] }])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const client = await attached(poller, { view: 'realm', realm: ONE_HEARTH, run: null })
  assert.deepEqual(client.sent.map((message) => message.type), ['snapshot'])
  const hearth = client.sent[0].fleet.hearths[0]
  assert.deepEqual([hearth.realm, hearth.detail, hearth.runs.length, hearth.next], [ONE_HEARTH, words, 0, null])
})

test('review F3: an absence from a fleet this poll has replaced reaches no client', async () => {
  const goneId = 'alpha-run-0000'
  const slowId = 'alpha-run-0001'
  const gone = { ...runOf('alpha', 0), run_id: goneId, seq: 12 }
  const slow = runningOf('alpha', 1, 12)
  const page1 = windowOf([gone, slow], null)
  const page2 = windowOf([slow], null)
  const page3 = windowOf([{ ...gone, seq: 25 }, slow], null)
  const realms = {
    journal: './.forge/forge.db',
    map: './realms.json',
    realms: [{ default_branch: 'main', head: '0000000', journal: '/hearths/alpha/forge.db', name: 'alpha', path: '../alpha' }],
  }
  const goneRead = runViewDoc({ seq: 12 })
  goneRead.summary = { ...goneRead.summary, run_id: goneId }
  const slowRead = runningRead(13)
  slowRead.summary = { ...slowRead.summary, run_id: slowId }
  const servedSlow = runningOf('alpha', 1, 13)
  const brokkr = stubBrokkr([
    { match: ['runs', `--run=${slowId}`], responses: [json(oneRow(servedSlow))] },
    { match: ['runs'], responses: [json(page1), json(page2), json(page3)] },
    { match: ['realms'], responses: [json(realms)] },
    { match: ['inspect'], responses: [json(goneRead), { ...json(slowRead), sleep_ms: 500 }, json(slowRead)] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const client = await attached(poller, { view: 'realm', realm: 'alpha', run: { realm: 'alpha', run_id: goneId } })
  client.sent.length = 0
  const inspections = () => brokkr.calls().filter((argv) => argv[0] === 'inspect').length

  // This poll lists the run gone, then waits on the other running run.
  const ticking = poller.tick()
  for (let waited = 0; inspections() < 2; waited += 10) {
    assert.ok(waited < 10_000, 'the poll’s second run did not start within 10s')
    await new Promise((wake) => setTimeout(wake, 10))
  }

  // A second client’s snapshot installs a fleet that lists the run again
  // while the first poll is still deciding its absence.
  const second = await attached(poller, { view: 'realm', realm: 'alpha', run: { realm: 'alpha', run_id: slowId } })
  assert.ok(second.sent[0].fleet.hearths[0].runs.some((row) => row.run_id === goneId), 'the newer fleet lists the run')
  await ticking

  assert.deepEqual(client.sent.filter((message) => message.type === 'run' && message.run.listed === false), [], 'the stale absence reaches no client')
  const fleet = client.sent.find((message) => message.type === 'fleet')?.fleet
  assert.ok(fleet?.hearths[0].runs.some((row) => row.run_id === goneId), 'the fleet still lists the run')
})

test('review F1: a map repointed before the inspect publishes nothing in a two-hearth world as in a one-hearth one', async () => {
  const runId = 'alpha-run-0000'
  const world = (journal, names) => realmsMap(journal, names)
  const oldPage = windowOf([{ ...runOf('alpha', 0), seq: 12 }], null)
  const newPage = windowOf([{ ...runOf('alpha', 0), seq: 25 }], null)
  const betaPage = windowOf([runOf('beta', 0)], null)
  const oldRead = runViewDoc({ seq: 12 })
  oldRead.summary = { ...oldRead.summary, run_id: runId }
  const newRead = runViewDoc({ seq: 25 })
  newRead.summary = { ...newRead.summary, run_id: runId }

  for (const names of [['alpha'], ['alpha', 'beta']]) {
    const brokkr = stubBrokkr([
      {
        match: ['realms'],
        responses: [json(world('/hearths/alpha/old.db', names)), json(world('/hearths/alpha/old.db', names)), json(world('/hearths/alpha/new.db', names))],
      },
      { match: ['runs', '--db=/hearths/alpha/old.db'], responses: [json(oldPage)] },
      { match: ['runs', '--db=/hearths/alpha/new.db'], responses: [json(newPage)] },
      ...(names.length === 2 ? [{ match: ['runs', '--db=/hearths/beta/forge.db'], responses: [json(betaPage)] }] : []),
      { match: ['inspect'], responses: [{ ...json(oldRead), sleep_ms: 100 }, json(newRead)] },
    ])
    const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
    const client = peer()
    client.receive = poller.attach(client)
    const inspections = () => brokkr.calls().filter((argv) => argv[0] === 'inspect').length
    const realms = () => brokkr.calls().filter((argv) => argv[0] === 'realms').length
    client.receive({ type: 'watch', view: 'realm', realm: 'alpha', run: { realm: 'alpha', run_id: runId } })
    for (let waited = 0; inspections() === 0; waited += 10) {
      assert.ok(waited < 10_000, 'the old journal’s inspect did not start within 10s')
      await new Promise((wake) => setTimeout(wake, 10))
    }
    await new Promise((wake) => setTimeout(wake, 300))

    // One hearth or two, the map moved under the inspect: nothing is painted.
    assert.deepEqual(client.sent, [], `the repointed ${names.length}-hearth map paints nothing`)
    assert.ok(realms() >= 3, `the map is re-read after the read (${realms()} realms reads)`)

    client.sent.length = 0
    await poller.tick()
    const fleet = client.sent.find((message) => message.type === 'fleet')?.fleet
    const alpha = fleet?.hearths.find((hearth) => hearth.realm === 'alpha')
    assert.deepEqual([alpha?.journal, alpha?.runs[0].seq], ['/hearths/alpha/new.db', 25], 'the next poll reads the journal the map now names')
  }
})

test('review F1: a map appearing after a no-map fleet read drops the inspect, and the next poll reads the world whole', async () => {
  const runId = 'alpha-run-0000'
  // The no-map listing names no journal: its row is all the console has.
  const oldPage = { view_version: 28, runs: [{ ...runOf('alpha', 0), seq: 25 }], count: 1 }
  const newPage = windowOf([{ ...runOf('alpha', 0), seq: 25 }], null)
  const world = {
    journal: './.forge/forge.db',
    map: './realms.json',
    realms: [{ default_branch: 'main', head: '0000000', journal: '/hearths/alpha/new.db', name: 'alpha', path: '../alpha' }],
  }
  const refused = { stderr: 'brokkr realms was refused: this workspace names no realms map', code: 1 }
  const oldRead = runViewDoc({ seq: 12 })
  oldRead.summary = { ...oldRead.summary, run_id: runId }
  const newRead = runViewDoc({ seq: 25 })
  newRead.summary = { ...newRead.summary, run_id: runId }
  const brokkr = stubBrokkr([
    { match: ['realms'], responses: [refused, refused, json(world)] },
    { match: ['runs', '--db=/hearths/alpha/new.db'], responses: [json(newPage)] },
    { match: ['runs'], responses: [json(oldPage)] },
    { match: ['inspect'], responses: [{ ...json(oldRead), sleep_ms: 100 }, json(newRead)] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const client = peer()
  client.receive = poller.attach(client)
  const inspections = () => brokkr.calls().filter((argv) => argv[0] === 'inspect').length
  const realms = () => brokkr.calls().filter((argv) => argv[0] === 'realms').length
  client.receive({ type: 'watch', view: 'realm', realm: ONE_HEARTH, run: { realm: ONE_HEARTH, run_id: runId } })
  for (let waited = 0; inspections() === 0; waited += 10) {
    assert.ok(waited < 10_000, 'the no-map world’s inspect did not start within 10s')
    await new Promise((wake) => setTimeout(wake, 10))
  }
  await new Promise((wake) => setTimeout(wake, 300))

  // The fleet came from the default journal and the inspect from the map's:
  // the map appeared, so neither is labelable and nothing is painted.
  assert.deepEqual(client.sent, [], 'the appearing map paints no snapshot')
  assert.ok(realms() >= 3, `the map is re-read after the read (${realms()} realms reads)`)

  client.sent.length = 0
  await poller.tick()
  const fleet = client.sent.find((message) => message.type === 'fleet')?.fleet
  const alpha = fleet?.hearths.find((hearth) => hearth.realm === 'alpha')
  assert.deepEqual([alpha?.journal, alpha?.runs[0].seq], ['/hearths/alpha/new.db', 25], 'the next poll reads the new world whole')
})

test('review residual F1-A: a dropped one-run row fence withholds the snapshot in a one- and a two-hearth world', async () => {
  const runId = 'alpha-run-0000'
  const world = (journal, names) => realmsMap(journal, names)
  const oldPage = windowOf([{ ...runOf('alpha', 0), seq: 12 }], null)
  const newPage = windowOf([{ ...runOf('alpha', 0), seq: 25 }], null)
  const betaPage = windowOf([runOf('beta', 0)], null)
  const oldRead = runViewDoc({ seq: 13 })
  oldRead.summary = { ...oldRead.summary, run_id: runId }
  const newRead = runViewDoc({ seq: 25 })
  newRead.summary = { ...newRead.summary, run_id: runId }

  for (const names of [['alpha'], ['alpha', 'beta']]) {
    const brokkr = stubBrokkr([
      {
        match: ['realms'],
        responses: [
          json(world('/hearths/alpha/old.db', names)),
          json(world('/hearths/alpha/old.db', names)),
          json(world('/hearths/alpha/old.db', names)),
          json(world('/hearths/alpha/new.db', names)),
        ],
      },
      { match: ['runs', `--run=${runId}`], responses: [json(oneRow({ ...runOf('alpha', 0), seq: 13 }))] },
      { match: ['runs', '--db=/hearths/alpha/old.db'], responses: [json(oldPage)] },
      { match: ['runs', '--db=/hearths/alpha/new.db'], responses: [json(newPage)] },
      ...(names.length === 2 ? [{ match: ['runs', '--db=/hearths/beta/forge.db'], responses: [json(betaPage)] }] : []),
      { match: ['inspect'], responses: [json(oldRead), json(newRead)] },
    ])
    const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
    const client = peer()
    client.receive = poller.attach(client)
    const realms = () => brokkr.calls().filter((argv) => argv[0] === 'realms').length
    const rows = () => brokkr.calls().filter((argv) => argv[0] === 'runs' && argv.includes(`--run=${runId}`)).length
    client.receive({ type: 'watch', view: 'realm', realm: 'alpha', run: { realm: 'alpha', run_id: runId } })
    for (let waited = 0; rows() === 0 || realms() < 4; waited += 10) {
      assert.ok(waited < 10_000, 'the row read and its map re-read did not finish within 10s')
      await new Promise((wake) => setTimeout(wake, 10))
    }
    await new Promise((wake) => setTimeout(wake, 100))

    // The row's own journal re-read finds the map moved: the whole snapshot
    // is withheld, though the fleet and the inspect read were still labelable.
    assert.deepEqual(client.sent, [], `the dropped row fence paints nothing in a ${names.length}-hearth world`)
    assert.equal(realms(), 4, 'one realms read per read: the fleet, its fence, the inspect, the row')

    client.sent.length = 0
    await poller.tick()
    const fleet = client.sent.find((message) => message.type === 'fleet')?.fleet
    const alpha = fleet?.hearths.find((hearth) => hearth.realm === 'alpha')
    assert.deepEqual([alpha?.journal, alpha?.runs[0].seq], ['/hearths/alpha/new.db', 25], 'the next poll reads the new world whole')
  }
})

test('review residual F1-A: a poll whose row fence drops paints no fleet and no run', async () => {
  const runId = 'alpha-run-0000'
  const held = runningOf('alpha', 0, 12)
  const moved = runningOf('alpha', 0, 13)
  const world = (journal) => realmsMap(journal, ['alpha'])
  const page = windowOf([held], null)
  const oldRead = runningRead(12)
  oldRead.summary = { ...oldRead.summary, run_id: runId }
  const movedRead = runningRead(13)
  movedRead.summary = { ...movedRead.summary, run_id: runId }
  const brokkr = stubBrokkr([
    {
      match: ['realms'],
      responses: [json(world('/hearths/alpha/old.db')), json(world('/hearths/alpha/old.db')), json(world('/hearths/alpha/old.db')), json(world('/hearths/alpha/old.db')), json(world('/hearths/alpha/old.db')), json(world('/hearths/alpha/old.db')), json(world('/hearths/alpha/new.db'))],
    },
    { match: ['runs', `--run=${runId}`], responses: [json(oneRow(moved))] },
    { match: ['runs'], responses: [json(page)] },
    { match: ['inspect'], responses: [json(oldRead), json(movedRead)] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const client = await attached(poller, { view: 'realm', realm: 'alpha', run: { realm: 'alpha', run_id: runId } })
  client.sent.length = 0

  // The poll's running run moves, its served row is read, and the map moves
  // under that row: the watcher is painted neither the fleet nor the inspect.
  await poller.tick()
  assert.deepEqual(client.sent, [], 'the dropped row fence paints no fleet and no run')
})

test('review residual F1-B: a journal another realm names after a swap does not fence the old realm’s result', async () => {
  const runId = 'alpha-run-0000'
  const alphaPage = windowOf([{ ...runOf('alpha', 0), seq: 12 }], null)
  const betaPage = windowOf([runOf('beta', 0)], null)
  const world = (alpha, beta) => realmsDoc(['alpha', 'beta'], (name) => (name === 'alpha' ? alpha : beta))
  const oldRead = runViewDoc({ seq: 12 })
  oldRead.summary = { ...oldRead.summary, run_id: runId }
  const brokkr = stubBrokkr([
    {
      match: ['realms'],
      responses: [
        json(world('/hearths/alpha/old.db', '/hearths/beta/forge.db')),
        json(world('/hearths/alpha/old.db', '/hearths/beta/forge.db')),
        json(world('/hearths/alpha/new.db', '/hearths/alpha/old.db')),
      ],
    },
    { match: ['runs', '--db=/hearths/alpha/old.db'], responses: [json(alphaPage)] },
    { match: ['runs', '--db=/hearths/beta/forge.db'], responses: [json(betaPage)] },
    { match: ['inspect'], responses: [json(oldRead)] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const client = peer()
  client.receive = poller.attach(client)
  const realms = () => brokkr.calls().filter((argv) => argv[0] === 'realms').length
  client.receive({ type: 'watch', view: 'realm', realm: 'alpha', run: { realm: 'alpha', run_id: runId } })
  for (let waited = 0; realms() < 3; waited += 10) {
    assert.ok(waited < 10_000, 'the inspect’s map re-read did not finish within 10s')
    await new Promise((wake) => setTimeout(wake, 10))
  }
  await new Promise((wake) => setTimeout(wake, 100))

  // alpha no longer names old.db; beta naming it does not make the read
  // alpha's, so the old realm's snapshot paints nothing.
  assert.deepEqual(client.sent, [], 'a journal another realm names paints nothing for alpha')
  assert.equal(realms(), 3, 'the inspect is fenced by the map read after it')
})

test('review residual M1: a joined hearth publishes nothing when one of its names no longer binds its journal', async () => {
  const runId = 'alpha-run-0000'
  const shared = '/hearths/shared/old.db'
  const page = windowOf([{ ...runOf('alpha', 0), seq: 12 }], null)
  const oldRead = runViewDoc({ seq: 12 })
  oldRead.summary = { ...oldRead.summary, run_id: runId }
  const brokkr = stubBrokkr([
    {
      match: ['realms'],
      responses: [
        json(realmsDoc(['alpha', 'beta'], () => shared)),
        json(realmsDoc(['alpha', 'beta'], () => shared)),
        json(realmsDoc(['alpha', 'beta'], (name) => (name === 'alpha' ? '/hearths/alpha/new.db' : shared))),
      ],
    },
    { match: ['runs', `--db=${shared}`], responses: [json(page)] },
    { match: ['inspect'], responses: [json(oldRead)] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const client = peer()
  client.receive = poller.attach(client)
  const realms = () => brokkr.calls().filter((argv) => argv[0] === 'realms').length
  client.receive({ type: 'watch', view: 'realm', realm: 'alpha+beta', run: { realm: 'alpha+beta', run_id: runId } })
  for (let waited = 0; realms() < 3; waited += 10) {
    assert.ok(waited < 10_000, 'the joined hearth’s map re-read did not finish within 10s')
    await new Promise((wake) => setTimeout(wake, 10))
  }
  await new Promise((wake) => setTimeout(wake, 100))

  // beta still names old.db, but alpha has moved: the label alpha+beta no
  // longer binds that journal, so the snapshot and its run paint nothing.
  assert.deepEqual(client.sent, [], 'a joined hearth with one moved name paints nothing')
  assert.equal(realms(), 3, 'the joined snapshot is fenced by one map read after its reads')
})

test('review residual M2: a poll withholds a fleet its own last map read already contradicts', async () => {
  const heldAlpha = runningOf('alpha', 0, 12)
  const heldBeta = runningOf('beta', 0, 12)
  const movedAlpha = runningOf('alpha', 0, 13)
  const freshAlpha = runningOf('alpha', 0, 25)
  const oldMap = realmsMap('/hearths/alpha/old.db')
  const newMap = realmsMap('/hearths/alpha/new.db')
  const readOf = (run, seq) => {
    const read = runningRead(seq)
    read.summary = { ...read.summary, run_id: run.run_id }
    return read
  }
  const brokkr = stubBrokkr([
    { match: ['realms'], responses: [json(oldMap), json(oldMap), json(oldMap), json(oldMap), json(oldMap), json(newMap), json(newMap), json(newMap), json(newMap), json(newMap)] },
    { match: ['runs', '--db=/hearths/alpha/old.db'], responses: [json(windowOf([heldAlpha], null)), json(windowOf([movedAlpha], null))] },
    { match: ['runs', '--db=/hearths/alpha/new.db'], responses: [json(windowOf([freshAlpha], null))] },
    { match: ['runs', '--db=/hearths/beta/forge.db'], responses: [json(windowOf([heldBeta], null)), json(windowOf([heldBeta], null)), json(windowOf([heldBeta], null))] },
    { match: ['inspect'], responses: [json(readOf(heldAlpha, 13)), json(readOf(heldBeta, 12)), json(readOf(freshAlpha, 25)), json(readOf(heldBeta, 12))] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const client = await attached(poller, { view: 'realm', realm: 'alpha', run: null })
  client.sent.length = 0
  const realms = () => brokkr.calls().filter((argv) => argv[0] === 'realms').length

  // The poll's beta fence resolves the moved map after its alpha fence did
  // not: the old-world fleet it read is no longer labelable and paints nothing.
  await poller.tick()
  assert.deepEqual(client.sent, [], 'the fleet the poll’s last map read contradicts paints nothing')
  assert.equal(realms(), 6, 'one realms read per read: the fleet, its fence, and each running run')

  // The next poll starts from the moved map and reads the new world whole.
  await poller.tick()
  const fleet = client.sent.find((message) => message.type === 'fleet')?.fleet
  const alpha = fleet?.hearths.find((hearth) => hearth.realm === 'alpha')
  const beta = fleet?.hearths.find((hearth) => hearth.realm === 'beta')
  assert.deepEqual([alpha?.journal, alpha?.runs[0].seq], ['/hearths/alpha/new.db', 25], 'the next poll reads the new journal')
  assert.deepEqual([beta?.journal, beta?.runs[0].seq], ['/hearths/beta/forge.db', 12], 'the unchanged hearth is still read')
  assert.equal(realms(), 10, 'the next poll adds one map read per read again')
})

/** The two-hearth map that moves beta to new.db on the `move`th `realms` read; alpha stays on old.db. */
function movedMap(move) {
  const oldMap = realmsMap('/hearths/alpha/old.db')
  const newMap = realmsDoc(['alpha', 'beta'], (name) => (name === 'alpha' ? '/hearths/alpha/old.db' : '/hearths/beta/new.db'))
  return Array.from({ length: 16 }, (_, index) => json(index + 1 < move ? oldMap : newMap))
}

/** A poller over `rules` with a peer watching `watch`, its host messages recorded. */
function watching(rules, watch) {
  const brokkr = stubBrokkr(rules)
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const client = peer()
  client.receive = poller.attach(client)
  client.receive({ type: 'watch', ...watch })
  return { brokkr, poller, client }
}

/** Wait until `done`, then let any last paint land. */
async function until(done, what) {
  for (let waited = 0; !done(); waited += 10) {
    assert.ok(waited < 10_000, `${what} did not settle within 10s`)
    await new Promise((wake) => setTimeout(wake, 10))
  }
  await new Promise((wake) => setTimeout(wake, 100))
}

/** The calls of one verb the stub Brokkr answered. */
const callsOf = (brokkr, verb) => brokkr.calls().filter((argv) => argv[0] === verb).length

/** The moved-map world's rules: alpha and beta pages, and the inspect answers. */
function movedRules({ move, alpha, beta, inspect = [] }) {
  return [
    { match: ['realms'], responses: movedMap(move) },
    { match: ['runs', '--db=/hearths/alpha/old.db'], responses: alpha.map(json) },
    { match: ['runs', '--db=/hearths/beta/forge.db'], responses: beta.map(json) },
    ...(inspect.length === 0 ? [] : [{ match: ['inspect'], responses: inspect.map(json) }]),
  ]
}

test('review 907p6 R-1(a): a snapshot withholds a fleet its own last map read contradicts', async () => {
  const heldAlpha = runningOf('alpha', 0, 12)
  const heldBeta = runningOf('beta', 0, 25)
  const inspect = runningRead(12)
  inspect.summary = { ...inspect.summary, run_id: heldAlpha.run_id }
  const { brokkr, client } = watching(
    movedRules({ move: 3, alpha: [windowOf([heldAlpha], null)], beta: [windowOf([heldBeta], null)], inspect: [inspect] }),
    { view: 'realm', realm: 'alpha', run: { realm: 'alpha', run_id: heldAlpha.run_id } },
  )
  await until(() => callsOf(brokkr, 'realms') >= 3, 'the snapshot’s map reads')
  assert.equal(callsOf(brokkr, 'realms'), 3, 'the fleet, its fence and the inspect read one map each')
  assert.deepEqual(client.sent, [], 'a fleet beta left behind paints nothing')
})

test('review 907p6 R-1(b): Show more withholds a fleet whose other hearth moved', async () => {
  const cursor = 'needs-you:1:alpha-run-0000'
  const { brokkr, client } = watching(
    movedRules({
      move: 3,
      alpha: [windowOf([runOf('alpha', 0)], cursor), windowOf([runOf('alpha', 1)], null)],
      beta: [windowOf([runOf('beta', 0)], null)],
    }),
    { view: 'realm', realm: 'alpha', run: null },
  )
  await until(() => client.sent.length > 0, 'the snapshot')
  client.sent.length = 0
  client.receive({ type: 'more', realm: 'alpha' })
  await until(() => callsOf(brokkr, 'realms') >= 3 && brokkr.calls().some((argv) => argv.some((word) => word.startsWith('--after='))), 'the Show more page')
  assert.equal(callsOf(brokkr, 'realms'), 3, 'the snapshot and the page each read the map once more')
  assert.deepEqual(client.sent, [], 'the paged fleet beta left behind paints nothing')
})

test('review 907p6 R-2: a withheld snapshot leaves its client owed the fleet the next poll sends', async () => {
  const heldAlpha = { ...runOf('alpha', 0), seq: 12 }
  const heldBeta = { ...runOf('beta', 0), seq: 25 }
  const map = realmsMap('/hearths/alpha/forge.db')
  const inspect = runViewDoc({ seq: 12 })
  inspect.summary = { ...inspect.summary, run_id: heldAlpha.run_id }
  const { brokkr, poller, client } = watching(
    [
      { match: ['realms'], responses: [json(map), json(map), { stderr: 'transient realms read failure', code: 1 }, json(map), json(map), json(map)] },
      { match: ['runs', '--db=/hearths/alpha/forge.db'], responses: [json(windowOf([heldAlpha], null))] },
      { match: ['runs', '--db=/hearths/beta/forge.db'], responses: [json(windowOf([heldBeta], null))] },
      { match: ['inspect'], responses: [json(inspect)] },
    ],
    { view: 'realm', realm: 'alpha', run: { realm: 'alpha', run_id: heldAlpha.run_id } },
  )
  await until(() => callsOf(brokkr, 'realms') >= 3, 'the withheld snapshot')
  assert.equal(callsOf(brokkr, 'realms'), 3, 'the snapshot reads the map twice and the inspect fence once')
  assert.deepEqual(client.sent, [], 'the failed fence paints no snapshot')
  client.sent.length = 0
  await poller.tick()
  assert.equal(callsOf(brokkr, 'realms'), 6, 'the poll reads the map for the fleet, its fence and the inspect fence')
  const fleet = client.sent.find((message) => message.type === 'fleet')
  assert.ok(fleet, 'the next successful poll sends the withheld fleet')
  assert.deepEqual(
    fleet.fleet.hearths.map((hearth) => [hearth.realm, hearth.journal]),
    [['alpha', '/hearths/alpha/forge.db'], ['beta', '/hearths/beta/forge.db']],
    'the client is owed the fleet installed while its snapshot was withheld',
  )
})

test('review 907p6: every publishing path withholds a fleet one moved map contradicts', async () => {
  const heldAlpha = runningOf('alpha', 0, 12)
  const heldBeta = runningOf('beta', 0, 12)
  const parkedBeta = { ...runOf('beta', 0), seq: 12 }
  const readOf = (run, seq) => {
    const read = runningRead(seq)
    read.summary = { ...read.summary, run_id: run.run_id }
    return read
  }
  const paths = [
    {
      name: 'snapshot',
      async run() {
        const { brokkr, client } = watching(
          movedRules({ move: 3, alpha: [windowOf([heldAlpha], null)], beta: [windowOf([heldBeta], null)], inspect: [readOf(heldAlpha, 12)] }),
          { view: 'realm', realm: 'alpha', run: { realm: 'alpha', run_id: heldAlpha.run_id } },
        )
        await until(() => callsOf(brokkr, 'realms') >= 3, 'the snapshot')
        return client
      },
    },
    {
      name: 'Show more',
      async run() {
        const cursor = 'needs-you:1:alpha-run-0000'
        const { brokkr, client } = watching(
          movedRules({
            move: 3,
            alpha: [windowOf([runOf('alpha', 0)], cursor), windowOf([runOf('alpha', 1)], null)],
            beta: [windowOf([runOf('beta', 0)], null)],
          }),
          { view: 'realm', realm: 'alpha', run: null },
        )
        await until(() => client.sent.length > 0, 'the snapshot')
        client.sent.length = 0
        client.receive({ type: 'more', realm: 'alpha' })
        await until(() => callsOf(brokkr, 'realms') >= 3, 'the Show more page')
        return client
      },
    },
    {
      name: 'poll',
      async run() {
        const { brokkr, poller, client } = watching(
          [
            { match: ['runs', `--run=${heldAlpha.run_id}`], responses: [json(oneRow({ ...heldAlpha, seq: 13 }))] },
            ...movedRules({ move: 6, alpha: [windowOf([heldAlpha], null)], beta: [windowOf([parkedBeta], null)], inspect: [readOf(heldAlpha, 13)] }),
          ],
          { view: 'realm', realm: 'alpha', run: null },
        )
        await until(() => client.sent.length > 0, 'the snapshot')
        client.sent.length = 0
        await poller.tick()
        assert.equal(callsOf(brokkr, 'realms'), 6, 'the poll’s step fences read the moved map')
        return client
      },
    },
    {
      name: 'open run’s row',
      async run() {
        const { brokkr, poller, client } = watching(
          [
            { match: ['runs', `--run=${heldAlpha.run_id}`], responses: [json(oneRow({ ...heldAlpha, seq: 13 }))] },
            ...movedRules({ move: 7, alpha: [windowOf([heldAlpha], null)], beta: [windowOf([parkedBeta], null)], inspect: [readOf(heldAlpha, 12), readOf(heldAlpha, 13)] }),
          ],
          { view: 'realm', realm: 'alpha', run: { realm: 'alpha', run_id: heldAlpha.run_id } },
        )
        await until(() => client.sent.length > 0, 'the snapshot')
        client.sent.length = 0
        await poller.tick()
        assert.equal(callsOf(brokkr, 'realms'), 7, 'the poll’s row fence reads the moved map')
        return client
      },
    },
  ]
  for (const path of paths) {
    const client = await path.run()
    assert.deepEqual(client.sent, [], `${path.name} paints nothing from the moved map`)
  }
})
