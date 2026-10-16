// The console as a tab in a conversation, beside Chat and Trajectory: it
// opens on the realm the conversation's folder belongs to. The run graph
// draws Brokkr's phases, steps and roads back. The look follows DSH's theme.
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'

import { fleetOf } from '../lib/host/readouts.js'
import { buttons, dshStub, findAll, mountClient, textOf } from './support/client.js'
import { fleetDoc, row } from './support/world.js'

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
  assert.deepEqual(client.fetches.filter((fetched) => fetched.url.startsWith(HERE)).map((fetched) => fetched.url), [`${HERE}?dir=%2Fhome%2Fop%2Fsource%2Fswarmail%2Fsrc`])
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
  assert.equal(findAll(client.tree(), (node) => node.type === 'button' && String(node.props.className).startsWith('bk-realm')).length, 2)
})

test('a realm lookup Brokkr refuses is said, not painted as no realm', async () => {
  const client = mountClient('conversation.view', { sessionId: 's1' }, {}, inFolder('/home/op/source/brokkr'))
  await settled()
  client.push({ type: 'snapshot', fleet: twoRealms(), overview: null, run: null })
  assert.match(textOf(client.tree()), /this conversation’s realm could not be read: no answer/)
  assert.doesNotMatch(textOf(client.tree()), /in no realm Brokkr lists/)
})

test('the realms pane lists each realm as one row: needing you first, running next, and one Brokkr cannot read last', () => {
  const client = mountClient('main')
  const doc = fleetDoc([
    ['lanetally', []],
    ['swarmail', [row({ run_id: 'finalize-gates-1a2b3c4d', title: 'Finalize the gates', verdict: { standing: 'running', rule: null, reason: null, text: '' } })]],
    ['brokkr', [row()]],
  ])
  doc.realms[0].detail = 'sqlite: unable to open database file: ./../lanetally/.forge/forge.db'
  client.push({ type: 'snapshot', fleet: fleetOf(doc), overview: null, run: null })
  const fleet = findAll(client.tree(), (node) => node.props['aria-label'] === 'Realms')[0]
  // lanetally's journal does not open: the stub fleet names it with a detail.
  const rows = buttons(fleet).filter((button) => String(button.props.className).startsWith('bk-realm'))
  assert.deepEqual(rows.map((each) => textOf(findAll(each, (node) => node.type === 'strong')[0])), ['brokkr', 'swarmail', 'lanetally'])
  assert.match(textOf(rows[2]), /^lanetallycannot read——sqlite: unable to open database file/)
  assert.match(textOf(rows[0]), /^brokkr1 needs you0 running—Encrypt the token store · review/)
  assert.match(textOf(rows[1]), /^swarmailall moving1 running—Finalize the gates · review · running/)
  const dots = findAll(rows[1], (node) => node.type === 'circle').map((dot) => dot.props.className)
  assert.deepEqual(dots, ['bk-dot-halo violet', 'bk-dot-fill violet live'], 'a realm with a running run pulses')
})

test('a new view opens at the top of whatever scrolls the console', () => {
  const client = mountClient('main')
  client.push({ type: 'snapshot', fleet: twoRealms(), overview: null, run: null })
  const scroller = { scrollHeight: 2000, clientHeight: 600, scrollTop: 900, parentElement: null }
  const console = { scrollHeight: 600, clientHeight: 600, parentElement: scroller }
  client.tree().props.ref(console)
  buttons(client.tree()).find((button) => textOf(button).startsWith('brokkr')).props.onClick()
  assert.equal(scroller.scrollTop, 0)
})

test('the realms view marks the conversation’s realm', async () => {
  const client = await tabIn('/home/op/source/brokkr', 'brokkr')
  buttons(client.tree()).find((button) => textOf(button) === '← All realms').props.onClick()
  const here = findAll(client.tree(), (node) => (node.props.className ?? '').includes(' here'))
  assert.equal(here.length, 1)
  assert.match(textOf(here[0]), /^brokkrthis conversation/)
  assert.match(textOf(here[0]), /this conversation/)
})

test('the full panel says nothing about a conversation', () => {
  const client = mountClient('main')
  client.push({ type: 'snapshot', fleet: twoRealms(), overview: null, run: null })
  assert.doesNotMatch(textOf(client.tree()), /this conversation/)
  assert.equal(client.tree().props['data-testid'], 'brokkr-console')
})

test('the pulse is ui.html’s: the same strike and halo, at the same pace', () => {
  const css = readFileSync(new URL('../lib/console.css', import.meta.url), 'utf8')
  const html = readFileSync(new URL('../../../../crates/brokkr-cli/src/ui.html', import.meta.url), 'utf8')
  const frames = (source, name) => {
    const found = new RegExp(`@keyframes ${name}\\s*\\{([\\s\\S]*?\\})\\s*\\}`).exec(source)
    assert.ok(found, `${name} is declared`)
    return found[1].replace(/\s+/g, '').replace(/;\}/g, '}')
  }
  assert.equal(frames(css, 'bk-strike'), frames(html, 'forge-strike'))
  assert.equal(frames(css, 'bk-halo'), frames(html, 'forge-halo'))
  assert.match(css, /bk-strike 1\.8s ease-in-out infinite/)
  assert.match(html, /forge-strike 1\.8s ease-in-out infinite/)
  assert.match(css, /bk-halo 1\.8s ease-out infinite/)
  assert.match(css, /prefers-reduced-motion:reduce/)
})

test('the full panel scrolls itself: DSH’s centre column clips, so a fire longer than the window still reads to its end', () => {
  const css = readFileSync(new URL('../lib/console.css', import.meta.url), 'utf8')
  assert.match(css, /\.bk:not\(\.in-tab\)\{height:100%;overflow-y:auto\}/)
  // The tab rides the conversation's own scroll, and the ask bar sticks to whichever scroller holds it.
  assert.doesNotMatch(/\.bk\.in-tab\{([^}]*)\}/.exec(css)[1], /overflow/)
  assert.match(/\.bk-askbar\{([^}]*)\}/.exec(css)[1], /position:sticky;bottom:0/)
})

test('every colour is a theme variable, with a dark set under DSH’s dark-theme mark', () => {
  const css = readFileSync(new URL('../lib/console.css', import.meta.url), 'utf8')
  const [light, dark] = [/body\{([^}]*)\}/.exec(css)[1], /body\[data-ds-dark-theme\]\{([^}]*)\}/.exec(css)[1]]
  const names = (block) => [...block.matchAll(/(--bk-[a-z0-9-]+):/g)].map((match) => match[1]).sort()
  assert.deepEqual(names(dark), names(light))
  // Outside the two palettes, no rule names a colour of its own.
  const rules = css.replace(/body\{[^}]*\}/, '').replace(/body\[data-ds-dark-theme\]\{[^}]*\}/, '')
  assert.deepEqual(rules.match(/#[0-9A-Fa-f]{3,8}\b/g) ?? [], [])
  const client = readFileSync(new URL('../lib/client.js', import.meta.url), 'utf8')
  assert.deepEqual(client.match(/'#[0-9A-Fa-f]{3,8}'/g) ?? [], [])
})

test('a realm’s runs are one pane of rows: dot, title, step, Brokkr’s word and age; the filter sits in its corner', () => {
  const client = mountClient('main')
  client.push({ type: 'snapshot', fleet: twoRealms(), overview: null, run: null })
  buttons(client.tree()).find((button) => textOf(button).startsWith('swarmail')).props.onClick()
  const runs = findAll(client.tree(), (node) => node.props['aria-label'] === 'Runs')[0]
  assert.match(textOf(runs), /^runs · 1/)
  const filter = findAll(runs, (node) => node.type === 'input')[0]
  assert.equal(filter.props.className, 'bk-input bk-filter')
  const [run] = buttons(runs)
  assert.match(textOf(run), /^Finalize the gatesreviewrunning/)
  assert.deepEqual(findAll(run, (node) => node.type === 'circle').map((dot) => dot.props.className), ['bk-dot-halo violet', 'bk-dot-fill violet live'])
  filter.props.onChange({ target: { value: 'nothing like it' } })
  assert.equal(buttons(findAll(client.tree(), (node) => node.props['aria-label'] === 'Runs')[0]).length, 0)
})
