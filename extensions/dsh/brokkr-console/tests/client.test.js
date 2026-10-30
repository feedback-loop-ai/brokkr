// The browser half paints what the host pushes and decides nothing: the
// waiting count, the buttons Brokkr admits named for their effect, the
// actions it cannot do yet with its reasons, a title before any run id, and
// the analytics as the readout gives them.
import assert from 'node:assert/strict'
import { test } from 'node:test'

import { analyticsOf, fleetOf, runViewOf } from '../lib/host/readouts.js'
import { NOW, buttons, findAll, mountClient, openedOn, textOf } from './support/client.js'
import { analyticsDoc, fleetDoc, journalRow, offered, row, runViewDoc } from './support/world.js'

const RUN = row().run_id
const settled = () => new Promise((wake) => setTimeout(wake, 0))

/** Two realms, three runs needing the operator between them, one running. */
const fleet = () =>
  fleetOf(
    fleetDoc([
      ['brokkr', [row(), row({ run_id: 'search-latency-targets-0a1b2c3d', title: 'Search latency targets', phase: 'clarify' }), row({ run_id: 'kv-cache-paging-9f8e7d6c', title: 'KV-cache paging', verdict: { standing: 'running', rule: null, reason: null, text: '' } })]],
      ['looper', [row({ run_id: 'journal-compaction-55aa55aa', title: 'Journal compaction', verdict: { standing: 'quarantined', rule: null, reason: null, text: 'does not fold' }, detail: 'the chain does not verify at seq 4' })]],
    ]),
  )

/** The panel, opened on the parked run in `brokkr`, before any read of the run lands. */
function openedRun(answers = {}, ask = false) {
  return openedOn(fleet(), 'brokkr', answers, ask)
}

/** The panel, opened on the parked run in `brokkr`, with `view` as its run view. */
async function onRun(view = runViewDoc(), answers = {}, ask = false) {
  const client = openedRun(answers, ask)
  client.push({ type: 'run', run: { realm: 'brokkr', run_id: RUN, view: runViewOf(view) } })
  return client
}

test('the panel-list icon counts the runs that need the operator across every hearth', () => {
  const client = mountClient('sidebar.panellist', { size: 16, active: false })
  client.push({ type: 'snapshot', fleet: fleet(), overview: null, run: null })
  assert.equal(textOf(client.tree()), '3')
  assert.equal(client.tree().props['aria-label'], 'Brokkr: 3 runs waiting on you')
})

test('the Realms header reads the same count', () => {
  const client = mountClient('main')
  client.push({ type: 'snapshot', fleet: fleet(), overview: null, run: null })
  const status = findAll(client.tree(), (node) => node.props.className === 'bk-status')[0]
  assert.equal(textOf(status), '1 running · 3 waiting on you')
})

test('a run whose accepted stop has no ending needs you, and is not counted as running (#909)', () => {
  const detail = 'stop accepted at seq 93; no ending written. If no driver is running it, `brokkr conclude` closes it.'
  const stopping = row({ run_id: 'stopping-without-ending-0000aaaa', title: 'Stopping without an ending', verdict: { standing: 'stopping', rule: null, reason: null, text: '', detail } })
  const running = row({ run_id: 'still-running-0000bbbb', title: 'Still running', phase: 'verify', verdict: { standing: 'running', rule: null, reason: null, text: '', detail: null } })
  const doc = fleetDoc([['brokkr', [stopping, running]]])
  const client = mountClient('sidebar.panellist', { size: 16, active: false })
  client.push({ type: 'snapshot', fleet: fleetOf(doc), overview: null, run: null })
  assert.equal(client.tree().props['aria-label'], 'Brokkr: 1 run waiting on you')
  const main = mountClient('main')
  main.push({ type: 'snapshot', fleet: fleetOf(doc), overview: null, run: null })
  const status = findAll(main.tree(), (node) => node.props.className === 'bk-status')[0]
  assert.equal(textOf(status), '1 running · 1 waiting on you')
  buttons(main.tree()).find((button) => textOf(button).startsWith('brokkr')).props.onClick()
  const runs = findAll(main.tree(), (node) => node.props['aria-label'] === 'Runs')[0]
  assert.deepEqual(findAll(runs, (node) => node.props.className === 'bk-grp').map(textOf), ['Needs you · 1', 'Running · 1'])
  const cell = (button, className) => textOf(findAll(button, (node) => node.props.className === className)[0])
  assert.deepEqual(
    buttons(runs).map((button) => [cell(button, 't'), cell(button, 's')]),
    [
      ['Stopping without an ending', detail],
      ['Still running', 'running'],
    ],
  )
})

test('an opened stopping run shows the shared detail, not a live page (#909)', () => {
  const detail = 'stop accepted at seq 93; no ending written. If no driver is running it, `brokkr conclude` closes it.'
  const stopping = row({ verdict: { standing: 'stopping', rule: null, reason: null, text: '', detail } })
  const doc = fleetDoc([['brokkr', [stopping]]])
  const client = mountClient('main')
  client.push({ type: 'snapshot', fleet: fleetOf(doc), overview: null, run: null })
  buttons(client.tree()).find((button) => textOf(button).startsWith('brokkr')).props.onClick()
  buttons(client.tree()).find((button) => textOf(button).startsWith('Encrypt the token store')).props.onClick()
  const view = runViewDoc({ summary: { ...runViewDoc().summary, status: 'running', status_word: 'stopping', standing: 'stopping', detail, park_reason: null } })
  client.push({ type: 'run', run: { realm: 'brokkr', run_id: stopping.run_id, view: runViewOf(view) } })
  assert.match(textOf(client.tree()), /stopping · waiting for an ending/)
  const panes = findAll(client.tree(), (node) => node.props['aria-label'] === 'Why it waits')
  assert.equal(panes.length, 1)
  assert.equal(textOf(panes[0]), `why it waits${detail}`)
})

test('the rail paints the served status word, not a standing re-derived (#909)', async () => {
  // The wire contradicts itself on purpose: the view served `stopping`,
  // while the fold's status and the standing still read `running`. The
  // rail must paint the served word, so it is not driven.
  const view = runViewDoc({
    summary: {
      ...runViewDoc().summary,
      status: 'running',
      standing: 'running',
      status_word: 'stopping',
      park_reason: null,
    },
    phases: [
      { name: 'implement', strategy: null, visits: 1, current: false, plain: true, columns: [], returns: [] },
      { name: 'verify', strategy: null, visits: 1, current: true, plain: true, columns: [], returns: [] },
    ],
    participants: [],
    journal: [],
  })
  const client = await onRun(view)
  const rail = findAll(client.tree(), (node) => node.type === 'svg' && node.props['aria-label'] === 'Run graph')[0]
  const dots = findAll(rail, (node) => node.type === 'circle' && String(node.props.className).startsWith('bk-dot')).map((dot) => dot.props.className)
  assert.deepEqual(dots, ['bk-dot-fill visited', 'bk-dot-fill park'])
  assert.deepEqual(
    findAll(rail, (node) => String(node.props.className).startsWith('bk-now')).map((node) => node.props.className),
    ['bk-now park', 'bk-now-label park'],
  )
})

test('an acknowledged quarantine is not counted as waiting on the operator (#511)', () => {
  const answered = { seq: 4, event_hash: 'ab12', operator: 'valentin', reason: 'history', recorded_at: '2026-10-14T08:00:00Z' }
  const quarantined = { standing: 'quarantined', rule: null, reason: null, text: 'does not fold' }
  const doc = fleetDoc([['looper', [row({ run_id: 'journal-compaction-55aa55aa', verdict: quarantined, acknowledgement: answered }), row()]]])
  const client = mountClient('sidebar.panellist', { size: 16, active: false })
  client.push({ type: 'snapshot', fleet: fleetOf(doc), overview: null, run: null })
  assert.equal(client.tree().props['aria-label'], 'Brokkr: 1 run waiting on you')
})

test('an acknowledged quarantine lists under Older whatever its age, keeping its mark and refusal (#511)', () => {
  const since = (hours) => new Date(NOW - hours * 60 * 60 * 1000).toISOString()
  const answered = { seq: 4, event_hash: 'ab12', operator: 'valentin', reason: 'history', recorded_at: since(1) }
  const quarantined = { standing: 'quarantined', rule: null, reason: null, text: 'does not fold' }
  const acknowledgedAt = (runId, title, hours) =>
    row({ run_id: runId, title, feature: title, phase: null, last_recorded_at: since(hours), verdict: quarantined, detail: `${title}: event 9: event after terminal status`, acknowledgement: answered })
  const doc = fleetDoc([['brokkr', [acknowledgedAt('rebuild-unit-12-c0c83ac6', 'Rebuild unit 12', 24 * 18), acknowledgedAt('slice-i-proofs-a4d1a721', 'Slice i proofs', 1)]]])
  const client = mountClient('main')
  client.push({ type: 'snapshot', fleet: fleetOf(doc), overview: null, run: null })
  buttons(client.tree()).find((button) => textOf(button).startsWith('brokkr')).props.onClick()
  const runs = findAll(client.tree(), (node) => node.props['aria-label'] === 'Runs')[0]
  assert.deepEqual(findAll(runs, (node) => node.props.className === 'bk-grp').map(textOf), ['Older · 2'])
  const cell = (button, className) => textOf(findAll(button, (node) => node.props.className === className)[0])
  assert.deepEqual(
    buttons(runs).map((button) => [cell(button, 't'), cell(button, 's'), cell(button, 'bk-muted bk-ago')]),
    [
      ['Rebuild unit 12', 'Rebuild unit 12: event 9: event after terminal status', '18d'],
      ['Slice i proofs', 'Slice i proofs: event 9: event after terminal status', '1h'],
    ],
  )
})

test('every run of a realm lists, and one that finished before the last day lists under Older with its standing', () => {
  const since = (hours) => new Date(NOW - hours * 60 * 60 * 1000).toISOString()
  const finishedAt = (runId, title, standing, hours) =>
    row({ run_id: runId, title, feature: title, phase: null, last_recorded_at: since(hours), verdict: { standing, rule: null, reason: null, text: standing } })
  const doc = fleetDoc([['brokkr', [
    row(),
    row({ run_id: 'landing-today-0000aaaa', title: 'Landing today', feature: 'Landing today', verdict: { standing: 'running', rule: null, reason: null, text: 'running' } }),
    finishedAt('shipped-an-hour-ago-0000bbbb', 'Shipped an hour ago', 'shipped', 1),
    finishedAt('shipped-three-days-ago-0000cccc', 'Shipped three days ago', 'shipped', 72),
    finishedAt('stopped-last-month-0000dddd', 'Stopped last month', 'stopped', 24 * 30),
  ]]])
  const client = mountClient('main')
  client.push({ type: 'snapshot', fleet: fleetOf(doc), overview: null, run: null })
  buttons(client.tree()).find((button) => textOf(button).startsWith('brokkr')).props.onClick()
  const runs = findAll(client.tree(), (node) => node.props['aria-label'] === 'Runs')[0]
  assert.deepEqual(findAll(runs, (node) => node.props.className === 'bk-grp').map(textOf), ['Needs you · 1', 'Running · 1', 'Last 24 hours · 1', 'Older · 2'])
  const cell = (button, className) => textOf(findAll(button, (node) => node.props.className === className)[0])
  assert.deepEqual(
    buttons(runs).slice(-2).map((button) => [cell(button, 't'), cell(button, 's'), cell(button, 'bk-muted bk-ago')]),
    [
      ['Shipped three days ago', 'shipped', '3d'],
      ['Stopped last month', 'stopped by a rule', '30d'],
    ],
  )
})

const controls = (tree) => findAll(tree, (node) => node.props['aria-label'] === 'Commands')[0]
const commandRow = (tree) => findAll(controls(tree), (node) => node.props.className === 'bk-row bk-commands')[0]
/** Press `button` on the graph, give `reason`, and send it. */
async function command(client, button, reason) {
  buttons(controls(client.tree())).find((each) => textOf(each) === button).props.onClick()
  const bar = findAll(client.tree(), (node) => node.props['aria-label'] === 'Your reason')[0]
  findAll(bar, (node) => node.type === 'input')[0].props.onChange({ target: { value: reason } })
  const send = buttons(findAll(client.tree(), (node) => node.props['aria-label'] === 'Your reason')[0]).find((each) => each.props.className === 'bk-btn primary')
  send.props.onClick()
  for (let tick = 0; tick < 5; tick += 1) await settled()
}
const outcomeOf = (tree) => textOf(findAll(tree, (node) => String(node.props.className).startsWith('bk-outcome'))[0])

test('the graph carries Stop, Resume and Close, and Retry when Brokkr admits it; a refused one is disabled with Brokkr’s reason', async () => {
  const client = await onRun()
  assert.deepEqual(
    buttons(commandRow(client.tree())).map((button) => [textOf(button), button.props.disabled === true, button.props.title]),
    [
      ['↻ Retry review', false, 'Retry runs review again, fresh, on the current code.'],
      ['■ Stop', false, 'Ends the run as stopped by you.'],
      ['▶ Resume', true, 'refused operator resume'],
      ['✓ Close', false, 'Ends the run as stopped by you. Refused if it turns out something is still driving it.'],
    ],
  )
  const unavailable = findAll(controls(client.tree()), (node) => node.props.className === 'bk-unavailable')[0]
  assert.equal(
    textOf(unavailable),
    'Not available yet · 3' +
      'Dismiss the findingBrokkr only lets you dismiss findings once a run has finished.' +
      'Talk to the reviewerReviews start fresh each time; your note can’t reach the reviewer yet.' +
      'HoldA parked run already waits for you; it holds nothing else back.',
  )
})

test('Retry is named for its phase, and is not offered when Brokkr would refuse it', async () => {
  const clarify = await onRun(runViewDoc({ commands: [offered('retry', 'admitted', { phase: 'clarify' }), offered('stop')], clarify: { seq: 9, questions: ['Which target counts?'] } }))
  assert.deepEqual(buttons(commandRow(clarify.tree())).map(textOf), ['↻ Retry clarify', '■ Stop'])
  const refused = await onRun(runViewDoc({ commands: [offered('retry', 'refused'), offered('stop')] }))
  assert.deepEqual(buttons(commandRow(refused.tree())).map(textOf), ['■ Stop'])
})

test('a run is led by its title, and its id appears only under "Details for audit"', async () => {
  const client = await onRun()
  const audit = findAll(client.tree(), (node) => node.type === 'details' && node.props.className === 'bk-audit')
  assert.equal(audit.length, 1)
  assert.equal(textOf(audit[0]).startsWith('Details for audit'), true)
  assert.equal(textOf(audit[0]).includes(RUN), true)
  const outside = textOf(client.tree()).replace(textOf(audit[0]), '')
  assert.equal(outside.includes(RUN), false)
  const graph = findAll(client.tree(), (node) => node.props['aria-label'] === 'Graph')[0]
  assert.equal(textOf(graph.children[0]), 'graph · Encrypt the token store')
})

test('a press opens a reason bar; with no reason nothing is sent, and Cancel closes it', async () => {
  const client = await onRun()
  buttons(controls(client.tree())).find((button) => textOf(button) === '■ Stop').props.onClick()
  const bar = findAll(client.tree(), (node) => node.props['aria-label'] === 'Your reason')[0]
  assert.match(textOf(bar), /^Stop the run/)
  const send = buttons(bar).find((each) => each.props.className === 'bk-btn primary')
  assert.equal(send.props.disabled, true)
  send.props.onClick()
  await settled()
  assert.deepEqual(client.fetches.filter((call) => call.url.includes('/api/command')), [])
  buttons(bar).find((each) => textOf(each) === 'Cancel').props.onClick()
  assert.equal(findAll(client.tree(), (node) => node.props['aria-label'] === 'Your reason').length, 0)
})

test('a press carries its command id and the head the operator saw, and paints "not sent" when the run moved', async () => {
  const fresh = runViewOf(runViewDoc({ seq: 13 }))
  const client = await onRun(runViewDoc(), { '/brokkr-console/api/command': { command: 'stop', outcome: 'not_sent', why: 'head_moved', view: fresh } })
  await command(client, '■ Stop', 'the owner-only change landed')
  const [sent] = client.fetches.filter((call) => call.url.includes('/api/command'))
  assert.deepEqual(JSON.parse(sent.init.body), {
    command_id: '0b6f1c2e-5d4a-4e8b-9f10-3c2d1e0f9a8b',
    realm: 'brokkr',
    run_id: RUN,
    command: 'stop',
    reason: 'the owner-only change landed',
    expected_seq: 12,
  })
  assert.equal(outcomeOf(client.tree()), 'Stop the runnot sentthe run changed after you opened it. Nothing was done. Here is what it looks like now.See current state')
})

test('a retry on a run whose start recorded no recipe is "not sent", naming the missing recipe and the settings', async () => {
  const client = await onRun(runViewDoc(), { '/brokkr-console/api/command': { command: 'retry', outcome: 'not_sent', why: 'resume_unconfigured', view: runViewOf(runViewDoc()) } })
  await command(client, '↻ Retry review', 'try again on the new code')
  assert.equal(
    outcomeOf(client.tree()),
    'Retry reviewnot sentNothing was sent: this run’s start recorded no recipe to resume it under, and the console names none. ' +
      'To resume every run under one bundle, set BROKKR_CONSOLE_RESUME_BUNDLE or BROKKR_CONSOLE_RESUME_RECIPE where DSH starts.See current state',
  )
})

test('a refusal is painted as Brokkr’s own words', async () => {
  const words = 'the journal moved beneath the conclusion; something is still driving this run'
  const client = await onRun(runViewDoc(), { '/brokkr-console/api/command': { command: 'conclude', outcome: 'refused', text: words } })
  await command(client, '✓ Close', 'believed dead')
  assert.equal(outcomeOf(client.tree()), `Close itrefusedBrokkr said: “${words}”`)
})

test('the park reason is shown word for word, with its rule, beside the reader', async () => {
  const client = await onRun()
  const why = findAll(client.tree(), (node) => node.props['aria-label'] === 'Why it waits')[0]
  assert.match(textOf(why), /^why it waitsPark reason, word for word“security review left a serious residual finding: token file is readable by every user”REVIEW-SECURITY-RESIDUAL · parked at review/)
  const split = findAll(client.tree(), (node) => node.props.className === 'bk-split')[0]
  assert.deepEqual(split.children.map((child) => child.props['aria-label']), ['Why it waits', 'Reader'])
})

test('a running run has nothing to wait on, and the reader takes the row', async () => {
  const client = await onRun(runViewDoc({ summary: { ...runViewDoc().summary, status: 'running', status_word: 'running', standing: 'running', park_reason: null } }))
  const split = findAll(client.tree(), (node) => node.props.className === 'bk-split')[0]
  assert.deepEqual(split.children.map((child) => child.props['aria-label']), ['Reader'])
})

test('the question boxes are hidden unless BROKKR_CONSOLE_ASK turns them on: none on the realms nor on a run', async () => {
  const bars = (tree) => findAll(tree, (node) => node.props['aria-label'] === 'Asking about')
  const realms = mountClient('main')
  realms.push({ type: 'snapshot', fleet: fleet(), overview: null, run: null })
  assert.equal(bars(realms.tree()).length, 0)
  assert.equal(bars((await onRun()).tree()).length, 0)
})

test('turned on, one place asks: the Asking about bar, on the realms and on a run', async () => {
  const realms = mountClient('main')
  realms.push({ type: 'snapshot', ask: true, fleet: fleet(), overview: null, run: null })
  const bars = (tree) => findAll(tree, (node) => node.props['aria-label'] === 'Asking about')
  assert.equal(bars(realms.tree()).length, 1)
  assert.equal(findAll(realms.tree(), (node) => node.type === 'aside').length, 0)
  const run = await onRun(runViewDoc(), {}, true)
  assert.equal(bars(run.tree()).length, 1)
  assert.equal(findAll(run.tree(), (node) => node.type === 'aside').length, 0)
})

test('Analytics ends with the readout’s "not recorded yet" list, and a realm with no reporting seat reads "not reported"', async () => {
  const client = mountClient('main', {}, { '/brokkr-console/api/analytics': analyticsOf(analyticsDoc()) })
  client.push({ type: 'snapshot', fleet: fleet(), overview: null, run: null })
  buttons(client.tree()).find((button) => textOf(button) === 'Analytics').props.onClick()
  for (let tick = 0; tick < 5; tick += 1) await settled()
  const main = findAll(client.tree(), (node) => node.type === 'div' && node.props.className === 'bk-main')[0]
  const footer = main.children.at(-1)
  assert.equal(textOf(footer), 'not recorded yet: machine capacity over time · cost limits · costs for Codex and dsh · reasons findings were dismissed')
  const cells = findAll(main, (node) => node.type === 'td').map(textOf)
  assert.deepEqual(cells, ['lanetally', '3', '1', '4h 12m', '1.0h', 'not reported'])
  // A seat that reported no model is `null` on the wire (view 20) and reads by its text.
  assert.match(textOf(main), /gpt-5\.5 75%not reported 25%/)
})

test('two runs whose titles read alike are told apart by more of their feature, and the run header says the same', async () => {
  const alike = (runId, tail) =>
    // Dated by the clock: the run list shows a finished run for 24 hours after it last moved.
    row({ run_id: runId, title: 'Deliver the console…', feature: `Deliver the console plugin for the operator, ${tail}`, last_recorded_at: new Date().toISOString(), verdict: { standing: 'stopped', rule: 'IMPL-BLOCKED', reason: null, text: '' } })
  const client = mountClient('main')
  client.push({ type: 'snapshot', fleet: fleetOf(fleetDoc([['brokkr', [alike('first-try-0000aaaa', 'first try'), alike('second-try-0000bbbb', 'second try')]]])), overview: null, run: null })
  buttons(client.tree()).find((button) => textOf(button).startsWith('brokkr')).props.onClick()
  const listed = findAll(client.tree(), (node) => node.props.className === 't').map(textOf)
  assert.deepEqual(listed.map((text) => text.split('…')[0]), ['Deliver the console plugin for the operator, first', 'Deliver the console plugin for the operator, second'])
  buttons(client.tree()).find((button) => textOf(button).startsWith('Deliver the console plugin for the operator, second')).props.onClick()
  const header = findAll(client.tree(), (node) => node.type === 'h1')[0]
  assert.equal(textOf(header), 'Deliver the console plugin for the operator, second…')
})

test('a timeline row does not repeat its badge’s words beside the badge', async () => {
  const badged = (seq, badge, text) => ({ ...journalRow(seq, 'transition/decided', {}, text), what: { text, badge, badge_class: null, arrow: null, problem: null } })
  const client = await onRun(runViewDoc({ journal: [badged(13, 'IMPL-BLOCKED', 'IMPL-BLOCKED implement → stop · blocked'), badged(14, 'stop', 'stop')] }))
  const timeline = findAll(client.tree(), (node) => node.props['aria-label'] === 'Timeline')[0]
  const text = textOf(timeline)
  assert.ok(!text.includes('IMPL-BLOCKED IMPL-BLOCKED'), text)
  assert.ok(!text.includes('stop stop'), text)
  assert.ok(text.includes('implement → stop · blocked'), text)
})

test('"Where runs wait for you" says so when nothing waited, and the assistant panels offer no box to type in', async () => {
  const client = mountClient('main', {}, { '/brokkr-console/api/analytics': analyticsOf(analyticsDoc({ waiting_by_step: [] })) })
  client.push({ type: 'snapshot', fleet: fleet(), overview: null, run: null })
  assert.equal(findAll(client.tree(), (node) => node.type === 'input' && node.props.placeholder === 'Ask Brokkr…').length, 0)
  buttons(client.tree()).find((button) => textOf(button) === 'Analytics').props.onClick()
  for (let tick = 0; tick < 5; tick += 1) await settled()
  assert.ok(textOf(client.tree()).includes('Nothing waited on you in this range.'))
})

test('runs alike for 20 words of their feature are told apart by the time each started, said once', () => {
  const same = 'Deliver the console plugin for the operator with the same twenty words and then some more words that never differ early enough'
  const alike = (runId, at) => row({ run_id: runId, title: 'Deliver the console…', feature: `${same} ${runId}`, created_at: at })
  const client = mountClient('main')
  client.push({ type: 'snapshot', fleet: fleetOf(fleetDoc([['brokkr', [alike('early-0000aaaa', '2026-10-13T07:45:00Z'), alike('late-0000bbbb', '2026-10-13T08:51:00Z')]]])), overview: null, run: null })
  buttons(client.tree()).find((button) => textOf(button).startsWith('brokkr')).props.onClick()
  const listed = findAll(client.tree(), (node) => node.props.className === 't').map(textOf)
  assert.equal(new Set(listed).size, 2, listed.join(' | '))
  for (const label of listed) assert.match(label, /^Deliver the console… \(\d{2}:\d{2}( [AP]M)?\)/)
  assert.ok(listed.every((label) => !label.includes('started')), listed.join(' | '))
})

test('the run header paints the entry sentence and an unreadable entry as text', async () => {
  for (const entry of [
    { kind: 'entered', phase: 'verify', continues: [{ title: 'land the parked slice', feature: 'land the parked slice\nsecond\nthird', run_id: 'a-12345678' }], text: 'entered at verify, continuing land the parked slice (a-12345678)' },
    { kind: 'unreadable', error: 'synthetic error', text: 'entry unreadable: synthetic error' },
  ]) {
    const client = await onRun({ ...runViewDoc(), entry })
    const painted = findAll(client.tree(), (node) => node.type === 'div' && textOf(node) === entry.text)
    assert.equal(painted.length, 1)
    assert.deepEqual(painted[0].children, [entry.text])
  }
})

test('a refused read paints the run’s title, Brokkr’s words and the host’s hint, never reading forever, and a later read paints the run', () => {
  const words = 'brokkr inspect printed more than the console reads'
  const hint = `brokkr inspect --run=${RUN} --db=/hearths/brokkr/forge.db`
  const client = openedRun()
  assert.equal(findAll(client.tree(), (node) => textOf(node) === 'Reading the run…').length, 1)

  client.push({ type: 'run', run: { realm: 'brokkr', run_id: RUN, view: null, refused: words, hint } })
  assert.deepEqual(findAll(client.tree(), (node) => node.type === 'h1').map(textOf), ['Encrypt the token store'])
  assert.deepEqual(findAll(client.tree(), (node) => node.props.className === 'bk-error').map(textOf), [words])
  assert.deepEqual(findAll(client.tree(), (node) => String(node.props.className).includes('bk-hint') && textOf(node) === hint).map(textOf), [hint])
  assert.equal(textOf(client.tree()).includes('Reading the run…'), false)

  client.push({ type: 'run', run: { realm: 'brokkr', run_id: RUN, view: runViewOf(runViewDoc()) } })
  assert.equal(findAll(client.tree(), (node) => node.props['aria-label'] === 'Graph').length, 1)
  assert.equal(findAll(client.tree(), (node) => node.props.className === 'bk-error').length, 0)
  assert.equal(textOf(client.tree()).includes(words), false)
  assert.equal(textOf(client.tree()).includes(hint), false)
})

test('a run the fleet no longer lists says so, not that it is being read', () => {
  const client = openedRun()
  const other = row({ run_id: 'search-latency-targets-0a1b2c3d', title: 'Search latency targets', phase: 'clarify' })
  client.push({
    type: 'snapshot',
    fleet: fleetOf(fleetDoc([['brokkr', [other]]])),
    overview: null,
    run: { realm: 'brokkr', run_id: RUN, view: null, listed: false },
  })
  const main = findAll(client.tree(), (node) => node.type === 'main' && node.props.className === 'bk-main').map(textOf)
  assert.deepEqual(main, ['Brokkr no longer lists this run.'])
  assert.equal(textOf(client.tree()).includes('Reading the run…'), false)
})
