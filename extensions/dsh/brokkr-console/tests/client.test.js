// The browser half paints what the host pushes and decides nothing: the
// waiting count, the buttons Brokkr admits named for their effect, the
// actions it cannot do yet with its reasons, a title before any run id, and
// the analytics as the readout gives them.
import assert from 'node:assert/strict'
import { test } from 'node:test'

import { analyticsOf, fleetOf, runViewOf } from '../lib/host/readouts.js'
import { buttons, findAll, mountClient, textOf } from './support/client.js'
import { analyticsDoc, fleetDoc, offered, row, runViewDoc } from './support/world.js'

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

/** The panel, opened on the parked run in `brokkr`, with `view` as its run view. */
async function onRun(view = runViewDoc(), answers = {}) {
  const client = mountClient('main', {}, answers)
  client.push({ type: 'snapshot', fleet: fleet(), overview: null, run: null })
  buttons(client.tree()).find((button) => textOf(button).startsWith('brokkr')).props.onClick()
  buttons(client.tree()).find((button) => textOf(button).startsWith('Encrypt the token store')).props.onClick()
  client.push({ type: 'run', run: { realm: 'brokkr', run_id: RUN, view: runViewOf(view) } })
  return client
}

const decision = (tree) => findAll(tree, (node) => node.props['aria-label'] === 'Decision')[0]

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

test('only admitted commands are buttons, named for their effect; the rest is under "Not available yet" with its reason', async () => {
  const client = await onRun()
  const card = decision(client.tree())
  assert.deepEqual(
    buttons(card).map((button) => [textOf(button), button.props.disabled === true]),
    [
      ['Retry review', false],
      ['Stop the run', false],
      ['Close it', false],
      ['Dismiss the finding', true],
      ['Talk to the reviewer', true],
      ['Hold', true],
    ],
  )
  const unavailable = findAll(card, (node) => node.props.className === 'bk-unavailable')[0]
  assert.equal(
    textOf(unavailable),
    'Not available yet' +
      'Dismiss the findingBrokkr only lets you dismiss findings once a run has finished.' +
      'Talk to the reviewerReviews start fresh each time; your note can’t reach the reviewer yet.' +
      'HoldA parked run already waits for you; it holds nothing else back.',
  )
})

test('clarify’s buttons read Retry clarify, and Retry implement for an implement park', async () => {
  const clarify = await onRun(runViewDoc({ commands: [offered('retry', 'admitted', { phase: 'clarify' }), offered('stop')], clarify: { seq: 9, questions: ['Which target counts?'] } }))
  assert.deepEqual(buttons(decision(clarify.tree())).map(textOf).slice(0, 2), ['Retry clarify', 'Stop the run'])
  const implement = await onRun(runViewDoc({ commands: [offered('retry', 'admitted', { phase: 'implement' })] }))
  assert.deepEqual(buttons(decision(implement.tree())).map(textOf).slice(0, 1), ['Retry implement'])
})

test('a run is led by its title, and its id appears only under "Details for audit"', async () => {
  const client = await onRun()
  const audit = findAll(client.tree(), (node) => node.type === 'details' && node.props.className === 'bk-audit')
  assert.equal(audit.length, 1)
  assert.equal(textOf(audit[0]).startsWith('Details for audit'), true)
  assert.equal(textOf(audit[0]).includes(RUN), true)
  const outside = textOf(client.tree()).replace(textOf(audit[0]), '')
  assert.equal(outside.includes(RUN), false)
  assert.equal(textOf(findAll(client.tree(), (node) => node.type === 'h1')[0]), 'Encrypt the token store')
})

test('a press with no reason sends nothing and asks for one', async () => {
  const client = await onRun()
  buttons(decision(client.tree())).find((button) => textOf(button) === 'Stop the run').props.onClick()
  await settled()
  assert.deepEqual(client.fetches.filter((call) => call.url.includes('/api/command')), [])
  const reason = findAll(client.tree(), (node) => node.type === 'textarea')[0]
  assert.deepEqual([reason.props.className, reason.props.placeholder], ['bk-reason missing', 'Say why first: a reason is required.'])
})

test('a press carries its command id and the head the operator saw, and paints "not sent" when the run moved', async () => {
  const fresh = runViewOf(runViewDoc({ seq: 13 }))
  const client = await onRun(runViewDoc(), { '/brokkr-console/api/command': { command: 'stop', outcome: 'not_sent', why: 'head_moved', view: fresh } })
  findAll(client.tree(), (node) => node.type === 'textarea')[0].props.onChange({ target: { value: 'the owner-only change landed' } })
  buttons(decision(client.tree())).find((button) => textOf(button) === 'Stop the run').props.onClick()
  for (let tick = 0; tick < 5; tick += 1) await settled()
  const [sent] = client.fetches.filter((call) => call.url.includes('/api/command'))
  assert.deepEqual(JSON.parse(sent.init.body), {
    command_id: '0b6f1c2e-5d4a-4e8b-9f10-3c2d1e0f9a8b',
    realm: 'brokkr',
    run_id: RUN,
    command: 'stop',
    reason: 'the owner-only change landed',
    expected_seq: 12,
  })
  const outcome = findAll(client.tree(), (node) => String(node.props.className).startsWith('bk-outcome'))[0]
  assert.equal(textOf(outcome), 'Stop the runnot sentthe run changed after you opened it. Nothing was done. Here is what it looks like now.See current state')
})

test('a refusal is painted as Brokkr’s own words', async () => {
  const words = 'the journal moved beneath the conclusion; something is still driving this run'
  const client = await onRun(runViewDoc(), { '/brokkr-console/api/command': { command: 'conclude', outcome: 'refused', text: words } })
  findAll(client.tree(), (node) => node.type === 'textarea')[0].props.onChange({ target: { value: 'believed dead' } })
  buttons(decision(client.tree())).find((button) => textOf(button) === 'Close it').props.onClick()
  for (let tick = 0; tick < 5; tick += 1) await settled()
  const outcome = findAll(client.tree(), (node) => String(node.props.className).startsWith('bk-outcome'))[0]
  assert.equal(textOf(outcome), `Close itrefusedBrokkr said: “${words}”`)
})

test('the two assistant panels are titled Ask Brokkr and Ask about this run', async () => {
  const realms = mountClient('main')
  realms.push({ type: 'snapshot', fleet: fleet(), overview: null, run: null })
  const titles = (tree) => findAll(tree, (node) => node.props.className === 'bk-aside-title').map(textOf)
  assert.deepEqual(titles(realms.tree()), ['Ask Brokkr'])
  const run = await onRun()
  assert.deepEqual(titles(run.tree()), ['Ask about this run'])
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
})
