// Picking on the run graph: a phase picks every node in it, a step picks
// itself, and what is picked shows its seats as the TUI reads them and
// becomes the closest context of the next question, the run its outer one.
// A pick outlives the view's live refreshes. Live nodes wear Brokkr's pulse.
import assert from 'node:assert/strict'
import { test } from 'node:test'

import { fleetOf, runViewOf } from '../lib/host/readouts.js'
import { buttons, findAll, mountClient, textOf } from './support/client.js'
import { fleetDoc, row, runViewDoc, seat } from './support/world.js'

const RUN = row().run_id
const settled = async () => {
  for (let tick = 0; tick < 5; tick += 1) await new Promise((wake) => setTimeout(wake, 0))
}
const member = (key, label, stateClass = 'on-phosphor', state = 'finished') => ({ label, key, state, state_class: stateClass, model: { text: 'gpt-5.5', absent: false, note: null }, boundary: { text: 'namespace', absent: false, note: null } })

/** Implement, then a review whose two members ran side by side, the security one parked. */
const reviewed = (over = {}) => runViewDoc({
  phases: [
    { name: 'implement', strategy: 'code', visits: 1, current: false, plain: true, columns: [{ label: null, nodes: [member('seat-implement-1', 'implement')] }], returns: [] },
    { name: 'review', strategy: null, visits: 1, current: true, plain: false, columns: [{ label: 'positions', nodes: [member('seat-correctness-1', 'correctness'), member('seat-security-1', 'security', 'on-park', 'indeterminate')] }], returns: [] },
  ],
  participants: [
    seat({ key: 'seat-implement-1', label: 'implement', phase: 'implement', cost_cell: { text: '$0.8100', absent: false, note: null }, cache_hit_cell: { text: '—', absent: true, note: 'no cache reads and input reported to compare' } }),
    seat({ key: 'seat-correctness-1', label: 'correctness', phase: 'review', cost_cell: { text: '$0.4000', absent: false, note: null } }),
    seat(),
  ],
  ...over,
})

function onRun(view = reviewed()) {
  const client = mountClient('main')
  client.push({ type: 'snapshot', workspace: '/w/console', fleet: fleetOf(fleetDoc([['brokkr', [row()]]])), overview: null, run: null })
  buttons(client.tree()).find((button) => textOf(button).startsWith('brokkr')).props.onClick()
  buttons(client.tree()).find((button) => textOf(button).startsWith('Encrypt the token store')).props.onClick()
  client.push({ type: 'run', run: { realm: 'brokkr', run_id: RUN, view: runViewOf(view) } })
  return client
}

const graph = (tree) => findAll(tree, (node) => node.props['aria-label'] === 'Run graph')[0]
const nodes = (tree) => findAll(graph(tree), (node) => (node.props.className ?? '').startsWith('bk-node'))
const chosen = (tree) => nodes(tree).filter((node) => node.props['aria-pressed']).map((node) => textOf(node.children[0]))
const picked = (tree) => findAll(tree, (node) => node.props['aria-label'] === 'Picked')[0] ?? null
const crumbs = (tree) => findAll(tree, (node) => ['bk-crumb', 'bk-crumb first'].includes(node.props.className)).map(textOf)
const press = (tree, text) => buttons(graph(tree)).find((button) => textOf(button).startsWith(text)).props.onClick()

test('a phase picks every node in it, and shows each of its seats', () => {
  const client = onRun()
  press(client.tree(), 'review')
  assert.deepEqual(chosen(client.tree()), ['correctness', 'security'])
  const text = textOf(picked(client.tree()))
  assert.match(text, /^PhasereviewStep|^Phasereview/)
  assert.match(text, /correctness/)
  assert.match(text, /security/)
  assert.doesNotMatch(text, /implement/)
  // Pressing it again lets it go.
  press(client.tree(), 'review')
  assert.deepEqual(chosen(client.tree()), [])
  assert.equal(picked(client.tree()), null)
})

test('a step picks itself and reads as the TUI’s participant pane: model, tokens, cache hit, cost', () => {
  const client = onRun()
  nodes(client.tree()).find((node) => textOf(node.children[0]) === 'security').props.onClick()
  assert.deepEqual(chosen(client.tree()), ['security'])
  const text = textOf(picked(client.tree()))
  assert.match(text, /Modelgpt-5\.5claimed · boundary not applicable/)
  assert.match(text, /Tokens3\.01M tok · in 3\.00M tok · out 1k tok · cache read 3\.00M tok · cache write 267k tok/)
  assert.match(text, /Cache hit99\.9%/)
  assert.match(text, /Cost\$2\.4179/)
  assert.match(text, /Turns541 attempt/)
})

test('an absent figure says why, in Brokkr’s words', () => {
  const client = onRun()
  nodes(client.tree()).find((node) => textOf(node.children[0]) === 'implement').props.onClick()
  assert.match(textOf(picked(client.tree())), /Cache hit—no cache reads and input reported to compare/)
})

test('the question is about the pick first and the run second', async () => {
  const client = onRun()
  assert.deepEqual(crumbs(client.tree()), ['run Encrypt the token store', 'realm brokkr'])
  nodes(client.tree()).find((node) => textOf(node.children[0]) === 'security').props.onClick()
  assert.deepEqual(crumbs(client.tree()), ['step security', 'run Encrypt the token store', 'realm brokkr'])
  const aside = findAll(client.tree(), (node) => node.type === 'aside' && node.props['aria-label'] === 'Ask about this run')[0]
  findAll(aside, (node) => node.type === 'input')[0].props.onChange({ target: { value: 'Is the finding real?' } })
  buttons(findAll(client.tree(), (node) => node.type === 'aside' && node.props['aria-label'] === 'Ask about this run')[0]).find((button) => textOf(button) === 'Ask').props.onClick()
  await settled()
  const prompt = client.dsh.calls.find((call) => call[0] === 'setDraft')[2]
  assert.match(prompt, /^About the seat “security” \(participant key seat-security-1, phase review, succeeded, model gpt-5\.5; its transcript is brokkr_transcript with seat seat-security-1\)\. Its outer context is the Brokkr run “Encrypt the token store” \(realm brokkr, run id encrypt-the-token-store-41d0a9c2\): read the run for it\./)
  assert.match(prompt, /Question: Is the finding real\?$/)
})

test('a picked phase is the closest context too', () => {
  const client = onRun()
  press(client.tree(), 'review')
  assert.deepEqual(crumbs(client.tree()), ['phase review', 'run Encrypt the token store', 'realm brokkr'])
})

test('a pick outlives the run’s live refresh, and is dropped once the view no longer holds it', () => {
  const client = onRun()
  nodes(client.tree()).find((node) => textOf(node.children[0]) === 'security').props.onClick()
  client.push({ type: 'run', run: { realm: 'brokkr', run_id: RUN, view: runViewOf(reviewed({ seq: 13 })) } })
  assert.deepEqual(chosen(client.tree()), ['security'])
  const gone = reviewed({ seq: 14 })
  gone.phases[1].columns[0].nodes.pop()
  client.push({ type: 'run', run: { realm: 'brokkr', run_id: RUN, view: runViewOf(gone) } })
  assert.equal(picked(client.tree()), null)
  assert.deepEqual(crumbs(client.tree()), ['run Encrypt the token store', 'realm brokkr'])
})

test('a realm with no run picked is asked about as the realm', () => {
  const client = mountClient('main')
  client.push({ type: 'snapshot', workspace: '/w/console', fleet: fleetOf(fleetDoc([['brokkr', [row()]]])), overview: null, run: null })
  buttons(client.tree()).find((button) => textOf(button).startsWith('brokkr')).props.onClick()
  const aside = findAll(client.tree(), (node) => node.type === 'aside')[0]
  assert.equal(aside.props['aria-label'], 'Ask about this realm')
  assert.deepEqual(crumbs(client.tree()), ['realm brokkr'])
  assert.equal(findAll(aside, (node) => node.type === 'input')[0].props.disabled, false)
})

const dots = (tree) => findAll(graph(tree), (node) => (node.props.className ?? '').startsWith('bk-dot')).map((node) => node.props.className)

test('a live step wears the pulse while the run is driven, and only then', () => {
  const running = reviewed({ summary: { ...runViewDoc().summary, status: 'running', park_reason: null } })
  running.phases[1].columns[0].nodes[1] = member('seat-security-1', 'security', 'in-active', 'active')
  const client = onRun(running)
  assert.deepEqual(dots(client.tree()), ['bk-dot phosphor', 'bk-dot phosphor', 'bk-dot violet live'])
  const halo = findAll(graph(client.tree()), (node) => node.props.className === 'bk-halo')
  assert.equal(halo.length, 1)
  // A parked run does not pulse, whatever its nodes say.
  const parked = onRun(reviewed())
  assert.ok(dots(parked.tree()).every((name) => !name.endsWith('live')))
})

test('the road back is solid, in the rail’s colour, with the forward arrow', () => {
  const view = reviewed()
  view.phases[0].returns = ['review']
  const client = onRun(view)
  const back = findAll(graph(client.tree()), (node) => node.type === 'path' && node.props.className === 'bk-edge back')
  assert.equal(back.length, 1)
  assert.equal(back[0].props.markerEnd, 'url(#bk-arrow)')
})

test('the brand mark strikes while any run in the fleet is running', () => {
  const quiet = mountClient('main')
  quiet.push({ type: 'snapshot', fleet: fleetOf(fleetDoc([['brokkr', [row()]]])), overview: null, run: null })
  const mark = (tree) => findAll(tree, (node) => (node.props.className ?? '').startsWith('bk-mark'))[0].props.className
  assert.equal(mark(quiet.tree()), 'bk-mark')
  const live = mountClient('main')
  live.push({ type: 'snapshot', fleet: fleetOf(fleetDoc([['brokkr', [row({ verdict: { standing: 'running', rule: null, reason: null, text: '' } })]]])), overview: null, run: null })
  assert.equal(mark(live.tree()), 'bk-mark forging')
  const icon = mountClient('sidebar.panellist', { size: 16, active: false })
  icon.push({ type: 'snapshot', fleet: fleetOf(fleetDoc([['brokkr', [row({ verdict: { standing: 'running', rule: null, reason: null, text: '' } })]]])), overview: null, run: null })
  assert.equal(mark(icon.tree()), 'bk-mark forging')
})

test('a graph wider than its column opens scrolled to the current phase, once per run', () => {
  const client = onRun()
  const scroller = findAll(graph(client.tree()), (node) => node.props.className === 'bk-graph')[0]
  const element = { dataset: {}, clientWidth: 200, scrollLeft: 0 }
  scroller.props.ref(element)
  // review, the current phase, ends at 174 + 144 = 318; the column shows 200, so it scrolls past the gap.
  assert.equal(element.scrollLeft, 148)
  element.scrollLeft = 0
  scroller.props.ref(element)
  assert.equal(element.scrollLeft, 0, 'the operator’s own scroll is left alone')
  const wide = { dataset: {}, clientWidth: 900, scrollLeft: 0 }
  scroller.props.ref(wide)
  assert.equal(wide.scrollLeft, 0, 'a graph that fits is not scrolled')
})
