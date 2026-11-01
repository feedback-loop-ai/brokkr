// The open run and its row: every open run is re-read on every poll,
// wherever the fleet holds it, and its row is the one Brokkr serves whole
// (`runs --json --run <id>`), never derived here. The fixtures these tests
// share live in tests/support/poller.js.
import assert from 'node:assert/strict'
import { test } from 'node:test'

import { brokkrOf } from '../lib/host/brokkr.js'
import { FLEET_WINDOW } from '../lib/host/config.js'
import { pollerOf } from '../lib/host/poller.js'
import { ONE_HEARTH } from '../lib/host/readouts.js'
import { buttons, findAll, mountClient, textOf } from './support/client.js'
import { RUN, attached, displaced, heard, oneRow, realmsDoc, retained, runOf, runningOf, runningRead, windowOf } from './support/poller.js'
import { analyticsDoc, json, row, runViewDoc, stubBrokkr } from './support/world.js'

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
      responses: [json(realmsFor('/old/forge.db')), json(realmsFor('/old/forge.db')), json(realmsFor('/old/forge.db')), json(realmsFor('/old/forge.db')), json(realmsFor('/new/forge.db'))],
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

test('review M5: a running run the first page shows but no client watches is inspected', async () => {
  const page = windowOf([runningOf('alpha', 0, 12)], null)
  const brokkr = stubBrokkr([
    { match: ['runs', '--run=alpha-run-0000'], responses: [json(oneRow(runningOf('alpha', 0, 13)))] },
    { match: ['runs'], responses: [json(page)] },
    { match: ['inspect'], responses: [json(runningRead(13))] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const client = await attached(poller, { view: 'realm', realm: ONE_HEARTH, run: null })
  const inspections = () => brokkr.calls().filter((argv) => argv[0] === 'inspect').length
  assert.equal(inspections(), 0, 'no client watches the run')
  client.sent.length = 0
  await poller.tick()
  assert.equal(inspections(), 1, 'the first page’s running run is inspected')
  const fleet = client.sent.find((message) => message.type === 'fleet')?.fleet
  assert.equal(fleet.hearths[0].runs[0].seq, 13)
})
