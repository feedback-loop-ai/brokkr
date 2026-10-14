// The console as a tab in a conversation, beside Chat and Trajectory: it
// opens on the realm the conversation's folder belongs to. The run graph
// draws Brokkr's phases, steps and roads back. The look follows DSH's theme.
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'

import { fleetOf, runViewOf } from '../lib/host/readouts.js'
import { buttons, dshStub, findAll, mountClient, textOf } from './support/client.js'
import { fleetDoc, row, runViewDoc } from './support/world.js'

const settled = () => new Promise((wake) => setTimeout(wake, 0))
const HERE = '/brokkr-console/api/here'

const twoRealms = () =>
  fleetOf(fleetDoc([
    ['brokkr', [row()]],
    ['swarmail', [row({ run_id: 'finalize-gates-1a2b3c4d', title: 'Finalize the gates', verdict: { standing: 'running', rule: null, reason: null, text: '' } })]],
  ]))

/** A DSH whose session list puts session `s1` in `cwd`. */
function inFolder(cwd) {
  const dsh = dshStub()
  dsh.services.sessions.list = { getSnapshot: () => ({ ids: ['s1'], byId: { s1: { sessionId: 's1', cwd } } }), subscribe: () => () => {} }
  return dsh
}

async function tabIn(cwd, realm) {
  const client = mountClient('conversation.view', { sessionId: 's1' }, { [HERE]: { realm } }, inFolder(cwd))
  await settled()
  client.push({ type: 'snapshot', fleet: twoRealms(), overview: null, run: null })
  return client
}

test('the tab sits in the conversation view ring after Chat and Trajectory, labelled Brokkr', () => {
  const client = mountClient('sidebar.panellist', { size: 16, active: false })
  const { slot } = client.registered['conversation.view']
  assert.deepEqual([slot.id, slot.order, slot.label()], ['brokkr-console', 20, 'Brokkr'])
  assert.equal(slot.inject('s1').sessionId, 's1')
  // The sidebar entry stays.
  assert.equal(client.registered['sidebar.panellist'].slot.id, 'brokkr-console')
})

test('the tab opens on the realm its conversation’s folder belongs to, and says so', async () => {
  const client = await tabIn('/home/op/source/swarmail/src', 'swarmail')
  assert.deepEqual(client.fetches.map((fetched) => fetched.url), [`${HERE}?dir=%2Fhome%2Fop%2Fsource%2Fswarmail%2Fsrc`])
  const text = textOf(client.tree())
  assert.match(text, /this conversation is in swarmail/)
  assert.match(text, /← All realms/)
  assert.match(text, /Finalize the gates/)
  const switcher = findAll(client.tree(), (node) => node.props['aria-label'] === 'Realms')[0]
  assert.deepEqual(buttons(switcher).map((button) => [textOf(button), button.props['aria-pressed']]), [['brokkr · 1', false], ['swarmail', true]])
  buttons(switcher).find((button) => textOf(button).startsWith('brokkr')).props.onClick()
  assert.match(textOf(client.tree()), /Encrypt the token store/)
})

test('a conversation in no realm gets every realm, and says why', async () => {
  const client = await tabIn('/tmp/scratch', null)
  const text = textOf(client.tree())
  assert.match(text, /this conversation’s folder is in no realm Brokkr lists/)
  assert.equal(findAll(client.tree(), (node) => (node.props.className ?? '').includes('bk-realm')).length, 2)
})

test('the realms view marks the conversation’s realm', async () => {
  const client = await tabIn('/home/op/source/brokkr', 'brokkr')
  buttons(client.tree()).find((button) => textOf(button) === '← All realms').props.onClick()
  const here = findAll(client.tree(), (node) => (node.props.className ?? '').includes(' here'))
  assert.deepEqual(here.map((card) => textOf(card).split('nothing')[0].split('1 need')[0]), ['brokkr'])
  assert.match(textOf(here[0]), /this conversation/)
})

test('the full panel says nothing about a conversation', () => {
  const client = mountClient('main')
  client.push({ type: 'snapshot', fleet: twoRealms(), overview: null, run: null })
  assert.doesNotMatch(textOf(client.tree()), /this conversation/)
  assert.equal(client.tree().props['data-testid'], 'brokkr-console')
})

const member = (label, state = 'finished', stateClass = 'on-phosphor', model = 'claude-fable-5-1') => ({ label, key: label, state, state_class: stateClass, model: { text: model, absent: false, note: null }, boundary: { text: 'namespace', absent: false, note: null } })

test('the graph draws each step, the members of a step side by side, and the road back', async () => {
  const view = runViewDoc({
    phases: [
      { name: 'design', strategy: null, visits: 1, current: false, plain: false, columns: [{ label: 'positions', nodes: [member('simplicity'), member('robustness')] }, { label: null, nodes: [member('chief')] }], returns: [] },
      { name: 'implement', strategy: 'code', visits: 2, current: false, plain: true, columns: [{ label: null, nodes: [member('implement')] }], returns: ['review'] },
      { name: 'review', strategy: null, visits: 1, current: true, plain: true, columns: [{ label: null, nodes: [member('review', 'failed', 'on-halt', 'gpt-5.5')] }], returns: [] },
    ],
  })
  const client = mountClient('main')
  client.push({ type: 'snapshot', fleet: twoRealms(), overview: null, run: null })
  buttons(client.tree()).find((button) => textOf(button).startsWith('brokkr')).props.onClick()
  buttons(client.tree()).find((button) => textOf(button).startsWith('Encrypt the token store')).props.onClick()
  client.push({ type: 'run', run: { realm: 'brokkr', run_id: row().run_id, view: runViewOf(view) } })
  const graph = findAll(client.tree(), (node) => node.props['aria-label'] === 'Run graph')[0]
  const nodes = findAll(graph, (node) => (node.props.className ?? '').startsWith('bk-node'))
  assert.deepEqual(nodes.map((node) => node.children[0].children[0]), ['simplicity', 'robustness', 'chief', 'implement', 'review'])
  // The two positions share a column; chief follows them.
  const [simplicity, robustness, chief] = nodes.map((node) => node.props.style)
  assert.equal(simplicity.left, robustness.left)
  assert.notEqual(simplicity.top, robustness.top)
  assert.ok(chief.left > simplicity.left)
  // The visit Brokkr ruled parks the review node, with its rule beside the model.
  assert.match(nodes[4].props.className, /warn/)
  assert.match(textOf(nodes[4]), /failed · gpt-5\.5/)
  assert.match(textOf(nodes[4]), /12m · REVIEW-SECURITY-RESIDUAL/)
  // The two positions fan into chief, then one edge each: 2 + 1 + 1 forward edges, and one road back.
  const edges = findAll(graph, (node) => node.type === 'path' && (node.props.className ?? '').startsWith('bk-edge'))
  assert.equal(edges.filter((edge) => edge.props.className === 'bk-edge').length, 4)
  assert.equal(edges.filter((edge) => edge.props.className === 'bk-edge back').length, 1)
  const labels = findAll(graph, (node) => (node.props.className ?? '').startsWith('bk-phase')).map(textOf)
  assert.deepEqual(labels, ['design', 'implement · 2 visits · code', 'review'])
  assert.match(textOf(graph), /positions/)
  assert.match(textOf(graph), /sent back to implement by review/)
})

test('a phase with no recorded steps is drawn as one node', async () => {
  const client = mountClient('main')
  client.push({ type: 'snapshot', fleet: twoRealms(), overview: null, run: null })
  buttons(client.tree()).find((button) => textOf(button).startsWith('brokkr')).props.onClick()
  buttons(client.tree()).find((button) => textOf(button).startsWith('Encrypt the token store')).props.onClick()
  client.push({ type: 'run', run: { realm: 'brokkr', run_id: row().run_id, view: runViewOf(runViewDoc()) } })
  const graph = findAll(client.tree(), (node) => node.props['aria-label'] === 'Run graph')[0]
  assert.deepEqual(findAll(graph, (node) => (node.props.className ?? '').startsWith('bk-node')).map((node) => node.children[0].children[0]), ['implement'])
})

test('every colour is a theme variable, with a dark set under DSH’s dark-theme mark', () => {
  const css = readFileSync(new URL('../lib/console.css', import.meta.url), 'utf8')
  const [light, dark] = [/body\{([^}]*)\}/.exec(css)[1], /body\[data-ds-dark-theme\]\{([^}]*)\}/.exec(css)[1]]
  const names = (block) => [...block.matchAll(/(--bk-[a-z0-9-]+):/g)].map((match) => match[1]).sort()
  assert.deepEqual(names(dark), names(light))
  // Outside the two palettes, no rule names a colour of its own (the badge's ink reads on amber in both).
  const rules = css.replace(/body\{[^}]*\}/, '').replace(/body\[data-ds-dark-theme\]\{[^}]*\}/, '')
  assert.deepEqual(rules.match(/#[0-9A-Fa-f]{3,8}\b/g) ?? [], [])
  const client = readFileSync(new URL('../lib/client.js', import.meta.url), 'utf8')
  assert.deepEqual(client.match(/'#[0-9A-Fa-f]{3,8}'/g), ["'#16181D'"])
})
