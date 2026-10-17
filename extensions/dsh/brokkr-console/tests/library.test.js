// The Library, slice one (change 2026-10-17-568): a recipe's graph and the
// Graph Pareto frontier. The host reads `brokkr graph --json` through one
// fenced route, its values checked before anything runs; the page draws
// Brokkr's figures as the boards lay them out, judges only the quadrant a
// recipe falls in at the viewer's own targets, and sends no command.
import assert from 'node:assert/strict'
import { test } from 'node:test'

import { apply } from '../lib/index.js'
import { analyticsOf, fleetOf, graphOf } from '../lib/host/readouts.js'
import { buttons, clientModule, dshStub, findAll, mountClient, refusedWith, textOf } from './support/client.js'
import { analyticsDoc, fleetDoc, graphDoc, json, realmRow, recipeRow, request, row, stubBrokkr, stubHost, withEnv } from './support/world.js'

const GRAPH = '/brokkr-console/api/graph'
const ANALYTICS = '/brokkr-console/api/analytics'
const HERE = '/brokkr-console/api/here'
const LIVE = 'add-the-graph-7c1d2e3f'
const DEFAULTS = { done: 50, cost: 3 }
const settled = async () => {
  for (let tick = 0; tick < 5; tick += 1) await new Promise((wake) => setTimeout(wake, 0))
}

// ------------------------------------------------------------ the route

/** The host applied over a stub `brokkr`, and a way to ask its graph route. */
async function hosted(rules) {
  const brokkr = stubBrokkr(rules)
  const host = stubHost()
  await withEnv({ BROKKR_CONSOLE_BROKKR: brokkr.program, BROKKR_STUB_DIR: brokkr.dir }, () => apply(host.ctx))
  const ask = (query) => request(host.routes.get(GRAPH).handler, { url: `${GRAPH}?${new URLSearchParams(query)}` })
  return { ask, calls: () => brokkr.calls().map((argv) => argv.join(' ')) }
}

test('a graph is asked for by a recipe’s plain name, a range Brokkr takes and a bounded realm; anything else is refused and nothing runs', async () => {
  const { ask, calls } = await hosted([])
  const recipe = 'a graph is asked for by its recipe’s name'
  const realm = 'a realm is named in 1 to 256 characters'
  for (const [query, error] of [
    [{ recipe: 'self', range: '1y' }, 'the range is one of 24h, 7d or 30d'],
    [{ recipe: '../self' }, recipe],
    [{ recipe: '--realms=/etc/passwd' }, recipe],
    [{ recipe: '' }, recipe],
    [{ recipe: 'self', realm: 'x'.repeat(257) }, realm],
    [{ recipe: 'self', realm: '' }, realm],
  ]) {
    const answer = await ask(query)
    assert.deepEqual([answer.status, JSON.parse(answer.body)], [400, { error }], JSON.stringify(query))
  }
  assert.deepEqual(calls(), [])
})

test('a graph is read with each value a flag, the recipe and the realm only when named, and reaches the page as the console read it', async () => {
  const { ask, calls } = await hosted([{ match: ['graph'], responses: [json(graphDoc())] }])
  const answer = await ask({ realm: 'brokkr', recipe: 'self', range: '30d' })
  assert.deepEqual([answer.status, JSON.parse(answer.body)], [200, graphOf(graphDoc())])
  await ask({ recipe: 'self' })
  await ask({ realm: 'brokkr' })
  assert.deepEqual(calls(), ['graph --recipe=self --realm=brokkr --range=30d --json', 'graph --recipe=self --range=7d --json', 'graph --realm=brokkr --range=7d --json'])
})

test('Brokkr’s refusal of a recipe it does not know, and a graph at another version, reach the page as their own words', async () => {
  const unknown = "no recipe 'nosuch' in brokkr: the recipes it knows are frontier and self"
  const { ask } = await hosted([
    { match: ['--recipe=nosuch'], responses: [{ code: 2, stderr: `${unknown}\n` }] },
    { match: ['graph'], responses: [json(graphDoc({ view_version: 2 }))] },
  ])
  const refused = await ask({ realm: 'brokkr', recipe: 'nosuch' })
  assert.deepEqual([refused.status, JSON.parse(refused.body)], [502, { error: unknown }])
  const newer = await ask({ realm: 'brokkr', recipe: 'self' })
  assert.deepEqual([newer.status, JSON.parse(newer.body)], [502, { error: 'brokkr graph is at view version 2; this console paints version 1 only' }])
})

// ------------------------------------------------------------- the page

const running = { standing: 'running', rule: null, reason: null, text: '' }

/** Two realms; brokkr's runs "Add the graph" and "Tidy the seats" are live. */
const fleet = () =>
  fleetOf(fleetDoc([
    ['brokkr', [row(), row({ run_id: LIVE, title: 'Add the graph', phase: 'verify', verdict: running }), row({ run_id: 'tidy-the-seats-2b3c4d5e', title: 'Tidy the seats', phase: 'review', verdict: running })]],
    ['swarmail', [row({ run_id: 'finalize-gates-1a2b3c4d', title: 'Finalize the gates', verdict: running })]],
  ]))

/**
 * Brokkr's figures over every realm: brokkr's five recipes (one exactly at
 * 50% and $3.00, one whose seats report no cost, one off the frontier) and
 * swarmail's one.
 */
const figures = () =>
  analyticsOf(analyticsDoc({
    realms: [realmRow('brokkr'), realmRow('swarmail')],
    recipes: [
      recipeRow({ realm: 'brokkr', recipe: 'frontier', runs: 6, done: 5, done_rate: 83.3, done_text: '83%', usd: 4.5 }),
      recipeRow({ realm: 'brokkr', recipe: 'lean', runs: 4, done: 2, done_rate: 50, done_text: '50%', usd: 3 }),
      recipeRow({ realm: 'brokkr', recipe: 'leaky', runs: 5, done: 1, done_rate: 20, done_text: '20%', usd: 3.5, on_frontier: false }),
      recipeRow({ realm: 'brokkr', recipe: 'muninn', runs: 2, done: 1, done_rate: 50, done_text: '50%', usd: null }),
      recipeRow({ realm: 'brokkr', recipe: 'self', runs: 10, done: 4, done_rate: 40, done_text: '40%', usd: 2 }),
      recipeRow({ realm: 'swarmail', recipe: 'triage', runs: 3, done: 1, done_rate: 33.3, done_text: '33%', usd: 1 }),
    ],
  }))

const answers = (over = {}) => ({ [ANALYTICS]: figures(), [GRAPH]: graphOf(graphDoc()), ...over })

/** The sidebar console, opened on its Library. */
async function onLibrary(given = answers(), storage = undefined) {
  const client = mountClient('main', {}, given, dshStub(), storage)
  client.push({ type: 'snapshot', fleet: fleet(), overview: null, run: null })
  buttons(client.tree()).find((button) => textOf(button) === 'Library').props.onClick()
  await settled()
  return client
}

const labelled = (tree, label) => findAll(tree, (node) => node.props['aria-label'] === label)[0]
const press = (tree, label) => labelled(tree, label).props.onClick()
const fetched = (client, path) => client.fetches.map((each) => String(each.url)).filter((url) => url.startsWith(path))
const heading = (tree) => textOf(findAll(tree, (node) => node.type === 'h1')[0])
const machine = (tree) => findAll(tree, (node) => node.type === 'svg' && node.props['aria-label'] === 'The recipe’s state machine')[0]
const edge = (tree, id) => findAll(machine(tree), (node) => node.type === 'path' && node.props['data-rule'] === id)[0]
const label = (tree, id) => textOf(findAll(machine(tree), (node) => node.type === 'text' && node.props['data-rule'] === id)[0])
const stroke = (tree, id) => [edge(tree, id).props.style.strokeWidth, edge(tree, id).props.style.strokeDasharray]
const box = (tree, name) => findAll(labelled(machine(tree), `the state ${name}`), (node) => node.type === 'rect')[0].props
/** The y of every point a path's `M` and `C` commands name. */
const ys = (d) => d.match(/-?[\d.]+/g).map(Number).filter((_, index) => index % 2 === 1)

test('the Library is a tab in both mounts, after Realms and Analytics, and a conversation’s tab opens it on the conversation’s realm', async () => {
  const nav = (tree) => buttons(labelled(tree, 'Brokkr views')).map(textOf)
  assert.deepEqual(nav(mountClient('main').tree()), ['Realms', 'Analytics', 'Library'])
  const dsh = dshStub()
  dsh.services.sessions.list = { getSnapshot: () => ({ ids: ['s1'], byId: { s1: { sessionId: 's1', cwd: '/home/op/source/swarmail' } } }), subscribe: () => () => {} }
  const triage = graphOf(graphDoc({ realm: 'swarmail', recipe: 'triage', recipes: ['triage'] }))
  const tab = mountClient('conversation.view', { sessionId: 's1' }, answers({ [HERE]: { realm: 'swarmail' }, [GRAPH]: triage }), dsh)
  await settled()
  tab.push({ type: 'snapshot', fleet: fleet(), overview: null, run: null })
  assert.deepEqual(nav(tab.tree()), ['Realms', 'Analytics', 'Library'])
  buttons(tab.tree()).find((button) => textOf(button) === 'Library').props.onClick()
  await settled()
  assert.deepEqual(fetched(tab, GRAPH), [`${GRAPH}?range=7d&realm=swarmail`])
  assert.equal(heading(tab.tree()), 'swarmail / triage')
})

test('the Library opens on the first realm and on the recipe Brokkr draws when asked for none, and sends no command', async () => {
  const client = await onLibrary()
  assert.deepEqual(fetched(client, ANALYTICS), [`${ANALYTICS}?range=7d`])
  // Brokkr picks the recipe with the most runs in range; the console names none.
  assert.deepEqual(fetched(client, GRAPH), [`${GRAPH}?range=7d&realm=brokkr`])
  assert.equal(heading(client.tree()), 'brokkr / self')
  // The recipe picker names the realm's recipes as the graph names them, measured or not.
  assert.deepEqual(buttons(labelled(client.tree(), 'Recipe')).map((button) => [textOf(button), button.props['aria-pressed']]), [
    ['charter', false], ['frontier', false], ['lean', false], ['leaky', false], ['muninn', false], ['self', true],
  ])
  assert.deepEqual(buttons(labelled(client.tree(), 'Realm')).map((button) => [textOf(button), button.props['aria-pressed']]), [['brokkr', true], ['swarmail', false]])
  assert.deepEqual(client.fetches.filter((each) => String(each.url).includes('/api/command')), [])
  assert.deepEqual(client.sockets[0].sent.at(-1), { type: 'watch', view: 'analytics', run: null })
  // Another range is asked of Brokkr again, figures and graph alike.
  buttons(client.tree()).find((button) => textOf(button) === '30d').props.onClick()
  await settled()
  assert.deepEqual(fetched(client, GRAPH).at(-1), `${GRAPH}?range=30d&realm=brokkr`)
  // Switching to Pareto keeps the graph already read: nothing is asked again.
  buttons(labelled(client.tree(), 'View')).find((button) => textOf(button) === 'Pareto').props.onClick()
  await settled()
  assert.equal(fetched(client, GRAPH).length, 2)
})

test('a realm none of whose recipes ran in range still lists them: the dormant graph Brokkr draws, and Pareto names each as not measured yet', async () => {
  const dormant = graphOf(graphDoc({ recipe: 'charter', runs: 0, live: [] }))
  const client = await onLibrary(answers({ [ANALYTICS]: analyticsOf(analyticsDoc({ realms: [realmRow('brokkr')], recipes: [] })), [GRAPH]: dormant }))
  assert.deepEqual(fetched(client, GRAPH), [`${GRAPH}?range=7d&realm=brokkr`])
  assert.equal(heading(client.tree()), 'brokkr / charter')
  assert.deepEqual(buttons(labelled(client.tree(), 'Recipe')).map(textOf), ['charter', 'frontier', 'lean', 'leaky', 'muninn', 'self'])
  buttons(labelled(client.tree(), 'View')).find((button) => textOf(button) === 'Pareto').props.onClick()
  await settled()
  const tree = client.tree()
  assert.deepEqual(findAll(tree, (node) => String(node.props['aria-label']).startsWith('the recipe ')), [])
  assert.equal(textOf(labelled(tree, 'Not measured yet')), 'not measured yet: charter · frontier · lean · leaky · muninn · self · no runs in this range')
})

test('an edge is 1.5 + 0.35 × its count pixels wide, at most 12, labelled with Brokkr’s words, a fired edge to the next state with its count alone; a rule that never fired is dashed at 1.5 and reads its name with 0', async () => {
  const { edgeWidth } = clientModule()
  assert.deepEqual([edgeWidth(0), edgeWidth(4), edgeWidth(40)], [1.5, 1.5 + 0.35 * 4, 12])
  const tree = (await onLibrary()).tree()
  for (const [id, count] of [['IMPL-OK', 6], ['VERIFY-FAIL', 3], ['REVIEW-REFORGE', 1], ['IMPL-RETRY', 2]]) {
    assert.deepEqual(stroke(tree, id), [1.5 + 0.35 * count, undefined], id)
    // Between neighbours there is room for the count only; the rule's name is its title and the panel's.
    assert.equal(label(tree, id), id === 'IMPL-OK' ? String(count) : `${id} · ${count}`)
  }
  assert.deepEqual(stroke(tree, 'INTAKE-OK'), [12, undefined])
  for (const id of ['REVIEW-CLEAN', 'SHIP-DIRTY']) {
    assert.deepEqual(stroke(tree, id), [1.5, '5 4'], id)
    assert.match(edge(tree, id).props.className, /\bdormant\b/)
    assert.equal(label(tree, id), `${id} · 0`)
  }
  // A park rule goes to no state, so it has no edge; the state's panel lists it.
  assert.equal(edge(tree, 'REVIEW-HOLD'), undefined)
})

test('every edge into stop carries the stop tone, and no other does', async () => {
  const tree = (await onLibrary()).tree()
  const edges = findAll(machine(tree), (node) => node.type === 'path' && node.props['data-rule'] !== undefined)
  const [toned, plain] = [edges.filter((path) => /\bstop\b/.test(path.props.className)), edges.filter((path) => !/\bstop\b/.test(path.props.className))]
  assert.deepEqual(toned.map((path) => path.props['data-rule']), ['VERIFY-FAIL', 'REVIEW-RESIDUAL-SECURITY', 'SHIP-DIRTY'])
  assert.deepEqual(toned.map((path) => path.props.markerEnd), ['url(#bk-lib-tip-stop)', 'url(#bk-lib-tip-stop)', 'url(#bk-lib-tip-stop)'])
  assert.ok(plain.length > 0 && plain.every((path) => path.props.markerEnd === 'url(#bk-lib-tip)'))
  const labels = findAll(machine(tree), (node) => node.type === 'text' && /\bstop\b/.test(String(node.props.className)) && node.props['data-rule'] !== undefined)
  assert.deepEqual(labels.map((text) => text.props['data-rule']), ['VERIFY-FAIL', 'REVIEW-RESIDUAL-SECURITY', 'SHIP-DIRTY'])
})

test('the row runs left to right in Brokkr’s order with done at its end and stop beneath; a self-loop and the edges back arc above the row, an edge into stop goes down to it', async () => {
  const tree = (await onLibrary()).tree()
  const row = ['intake', 'implement', 'verify', 'review', 'ship', 'done'].map((name) => box(tree, name))
  assert.ok(row.every((rect, index) => index === 0 || rect.x > row[index - 1].x))
  const middle = (rect) => rect.y + rect.height / 2
  assert.ok(row.every((rect) => middle(rect) === middle(row[0])))
  const top = Math.min(...row.map((rect) => rect.y))
  assert.ok(box(tree, 'stop').y > Math.max(...row.map((rect) => rect.y + rect.height)))
  for (const id of ['IMPL-RETRY', 'REVIEW-REFORGE', 'REVIEW-CLEAN']) assert.ok(Math.min(...ys(edge(tree, id).props.d)) < top, id)
  const verify = box(tree, 'verify')
  assert.ok(Math.min(...ys(edge(tree, 'VERIFY-FAIL').props.d)) >= verify.y + verify.height)
  assert.equal(Math.max(...ys(edge(tree, 'VERIFY-FAIL').props.d)), box(tree, 'stop').y)
  // Each state's box holds its seats and their models.
  assert.match(textOf(labelled(machine(tree), 'the state review')), /review:security · claude opus/)
})

test('the pulse sits on each live run’s state, and picking it opens that run in Realms as a fleet row does', async () => {
  const client = await onLibrary()
  const pulses = findAll(machine(client.tree()), (node) => String(node.props['aria-label']).startsWith('the live run '))
  assert.deepEqual(pulses.map((pulse) => pulse.props['aria-label']), ['the live run Add the graph', 'the live run Tidy the seats'])
  for (const [pulse, state] of [[pulses[0], 'verify'], [pulses[1], 'review']]) {
    const [x, y] = /translate\(([\d.]+) ([\d.]+)\)/.exec(pulse.props.transform).slice(1).map(Number)
    const rect = box(client.tree(), state)
    assert.ok(x > rect.x && x < rect.x + rect.width && y > rect.y && y < rect.y + rect.height, state)
    assert.deepEqual(findAll(pulse, (node) => node.type === 'svg').map((mark) => mark.props.className), ['bk-mark forging'])
  }
  pulses[0].props.onClick()
  assert.deepEqual(client.sockets[0].sent.at(-1), { type: 'watch', view: 'realm', run: { realm: 'brokkr', run_id: LIVE } })
  const text = textOf(client.tree())
  assert.match(text, /← brokkr’s runs/)
  assert.match(text, /Add the graph/)
})

test('picking review shows its rules with their counts, the runs that entered, its median and p90, its cost naming the provider that reported none, and its seats with their models', async () => {
  const client = await onLibrary()
  const panel = () => labelled(client.tree(), 'Selected state')
  const rules = () => findAll(panel(), (node) => node.props.className === 'bk-lib-rulerow').map(textOf)
  assert.match(textOf(panel()), /Pick a state/)
  press(client.tree(), 'the state review')
  assert.deepEqual(rules(), ['REVIEW-OK → ship2', 'REVIEW-REFORGE → implement1', 'REVIEW-CLEAN → verify0', 'REVIEW-RESIDUAL-SECURITY → stop2', 'REVIEW-HOLD → parks1'])
  assert.deepEqual(findAll(panel(), (node) => String(node.props.className).startsWith('bk-cell bk-tile')).map(textOf), [
    'Entered4runs in range',
    'Time30mmedian · p90 1h 00m',
    'Cost$1.50claude only · codex report none',
    'Tokensin 1k · out 300 · cache read 1k',
  ])
  assert.equal(textOf(labelled(panel(), 'Its seats')), 'Seatsreview:security · claude opusreview:correctness · codex gpt-5.5')
  // The chip row picks as the boxes do, and says which state is picked; a terminal lists the rules that end there.
  const chips = () => buttons(labelled(client.tree(), 'Pick a state'))
  assert.deepEqual(chips().filter((chip) => chip.props['aria-pressed']).map(textOf), ['review'])
  chips().find((chip) => textOf(chip) === 'stop').props.onClick()
  assert.deepEqual(rules(), ['VERIFY-FAIL from verify3', 'REVIEW-RESIDUAL-SECURITY from review2', 'SHIP-DIRTY from ship0'])
})

test('Brokkr’s words for the recipe head the graph, retired rules and the drift note lie beneath it, and a table not found is said', async () => {
  const client = await onLibrary()
  assert.equal(textOf(labelled(client.tree(), 'The recipe in range')), 'Runs · 7d102 done · 5 stop by a rule · 1 stopped by you · 2 openRules that never fired2 of 12drawn dashed')
  assert.equal(textOf(labelled(client.tree(), 'Beneath the graph')), 'Retired rules · fired in range, not in the table now: OLD-RULE · 2' + '4 runs ran under an earlier version of this table')
  const missing = 'this recipe’s table was not found in brokkr'
  const bare = await onLibrary(answers({ [GRAPH]: graphOf(graphDoc({ table: { kind: 'not_found', text: missing }, retired: [], drifted_runs: 0, drift_text: null })) }))
  assert.equal(textOf(findAll(bare.tree(), (node) => node.props.className === 'bk-coming')[0]), missing)
  assert.equal(labelled(bare.tree(), 'Beneath the graph'), undefined)
})

test('a graph at a version the console does not know says which version it found, and draws nothing', async () => {
  const words = 'brokkr graph is at view version 2; this console paints version 1 only'
  assert.throws(() => graphOf(graphDoc({ view_version: 2 })), { message: words })
  const client = await onLibrary(answers({ [GRAPH]: refusedWith(words) }))
  assert.deepEqual(findAll(client.tree(), (node) => node.props.className === 'bk-error').map(textOf), [`The graph could not be read: ${words}`])
  assert.equal(machine(client.tree()), undefined)
  assert.equal(labelled(client.tree(), 'Selected state'), undefined)
})

// ------------------------------------------------------------ the frontier

async function onPareto(given = answers(), storage = undefined) {
  const client = await onLibrary(given, storage)
  buttons(labelled(client.tree(), 'View')).find((button) => textOf(button) === 'Pareto').props.onClick()
  await settled()
  return client
}

const dot = (tree, name) => findAll(labelled(tree, `the recipe ${name}`), (node) => node.type === 'circle')[0].props
const target = (tree, which) => findAll(tree, (node) => node.type === 'line' && node.props.className === `bk-lib-target ${which}`)[0].props
/** Each row of the figure table: its recipe and the quadrant it reads. */
const quadrants = (tree) => findAll(findAll(tree, (node) => node.type === 'tbody')[0], (node) => node.type === 'tr').map((tr) => [textOf(findAll(tr, (node) => node.type === 'strong')[0]), textOf(tr.children.at(-1))])
const quadrantOfLean = (tree) => quadrants(tree).find(([name]) => name === 'lean')[1]
const input = (tree, name) => findAll(tree, (node) => node.type === 'input' && node.props['aria-label'] === name)[0].props
const DONE = 'Done-rate target, percent'
const COST = 'Cost-a-run target, dollars'

test('a recipe’s quadrant splits at the targets, and a point on a line goes to the better side', () => {
  const { quadrantOf } = clientModule()
  const at = (rate, usd, targets = DEFAULTS) => quadrantOf(rate, usd, targets)
  assert.deepEqual([at(50, 3), at(49.9, 3), at(50, 3.01), at(49.9, 3.01), at(100, 0), at(0, 100)], [
    'Pareto frontier quadrant', 'Cheap but leaky', 'Reliable but costly', 'Costly and leaky', 'Pareto frontier quadrant', 'Costly and leaky',
  ])
  assert.deepEqual([at(60, 2.5, { done: 60, cost: 2.5 }), at(59.9, 2.5, { done: 60, cost: 2.5 }), at(60, 2.51, { done: 60, cost: 2.5 })], ['Pareto frontier quadrant', 'Cheap but leaky', 'Reliable but costly'])
})

test('Pareto plots each recipe with a reported cost in its quadrant at the default targets; one exactly at 50% and $3.00 is in the Pareto frontier quadrant', async () => {
  const tree = (await onPareto()).tree()
  assert.deepEqual(findAll(tree, (node) => String(node.props['aria-label']).startsWith('the recipe ')).map((node) => node.props['aria-label']), ['the recipe frontier', 'the recipe lean', 'the recipe leaky', 'the recipe self'])
  assert.deepEqual(quadrants(tree), [['frontier', 'Reliable but costly'], ['lean', 'Pareto frontier quadrant'], ['leaky', 'Costly and leaky'], ['muninn', 'cost not reported'], ['self', 'Cheap but leaky']])
  // The two target lines cross where lean sits.
  assert.deepEqual([target(tree, 'done').y1, target(tree, 'cost').x1], [dot(tree, 'lean').cy, dot(tree, 'lean').cx])
  assert.equal(heading(tree), 'brokkr')
})

test('the targets sit at the middle of their axes, so the four quadrants keep one size however far a cost runs past its target', async () => {
  const client = await onPareto()
  const sizes = (tree) => Object.fromEntries(findAll(tree, (node) => node.type === 'rect' && /^bk-lib-q /.test(String(node.props.className)))
    .map((rect) => [rect.props.className.slice('bk-lib-q '.length), [rect.props.width, rect.props.height]]))
  const equal = (tree) => {
    const rects = sizes(tree)
    const [width, height] = rects.frontier
    assert.ok(width > 0 && height > 0)
    for (const quadrant of ['costly', 'leaky', 'dominated']) assert.deepEqual(rects[quadrant], [width, height], quadrant)
  }
  equal(client.tree())
  // lean sits exactly on both targets, at the centre of the plane.
  const { cx, cy } = dot(client.tree(), 'lean')
  assert.deepEqual([target(client.tree(), 'cost').x1, target(client.tree(), 'done').y1], [cx, cy])
  // A cost target far below the recipes' costs still splits the plane in four equal parts.
  input(client.tree(), COST).onChange({ target: { value: '0.5' } })
  input(client.tree(), DONE).onChange({ target: { value: '90' } })
  equal(client.tree())
})

test('the step line joins Brokkr’s frontier points in order of cost, and the recipes it cannot plot are named beneath it', async () => {
  const tree = (await onPareto()).tree()
  const [self, lean, frontier] = ['self', 'lean', 'frontier'].map((name) => dot(tree, name))
  const steps = findAll(tree, (node) => node.type === 'path' && node.props.className === 'bk-lib-frontier').map((path) => path.props.d)
  assert.deepEqual(steps, [`M${self.cx} ${self.cy}H${lean.cx}V${lean.cy}H${frontier.cx}V${frontier.cy}`])
  assert.equal(textOf(labelled(tree, 'Cost not reported')), 'cost not reported: muninn (dsh reports none)')
  assert.equal(textOf(labelled(tree, 'Not measured yet')), 'not measured yet: charter · no runs in this range')
})

test('clicking a recipe’s point opens its graph', async () => {
  const client = await onPareto()
  press(client.tree(), 'the recipe frontier')
  await settled()
  assert.equal(fetched(client, GRAPH).at(-1), `${GRAPH}?recipe=frontier&range=7d&realm=brokkr`)
  assert.equal(heading(client.tree()), 'brokkr / frontier')
  assert.deepEqual(buttons(labelled(client.tree(), 'View')).map((button) => [textOf(button), button.props['aria-pressed']]), [['Graph', true], ['Pareto', false]])
})

/** A browser's storage, kept in a map. */
function storageOf() {
  const kept = new Map()
  return { kept, getItem: (key) => (kept.has(key) ? kept.get(key) : null), setItem: (key, value) => kept.set(key, String(value)) }
}

test('the targets are kept in the browser for this console and read back on the next page', async () => {
  const storage = storageOf()
  const first = await onPareto(answers(), storage)
  assert.deepEqual([input(first.tree(), DONE).value, input(first.tree(), COST).value], ['50', '3.00'])
  input(first.tree(), DONE).onChange({ target: { value: '60' } })
  input(first.tree(), COST).onChange({ target: { value: '2.5' } })
  assert.deepEqual([...storage.kept.values()].map((text) => JSON.parse(text)), [{ done: 60, cost: 2.5 }])
  assert.equal(quadrantOfLean(first.tree()), 'Costly and leaky')
  // A word that is no target yet moves nothing.
  input(first.tree(), DONE).onChange({ target: { value: '' } })
  assert.equal(quadrantOfLean(first.tree()), 'Costly and leaky')
  const again = await onPareto(answers(), storage)
  assert.deepEqual([input(again.tree(), DONE).value, input(again.tree(), COST).value], ['60', '2.50'])
  assert.equal(quadrantOfLean(again.tree()), 'Costly and leaky')
})

test('a browser that refuses storage draws at 50% and $3.00, and editing a target still moves its line for this page', async () => {
  const refuse = () => {
    throw new Error('SecurityError: the operation is insecure')
  }
  const client = await onPareto(answers(), { getItem: refuse, setItem: refuse })
  const lean = () => dot(client.tree(), 'lean')
  assert.deepEqual([target(client.tree(), 'done').y1, target(client.tree(), 'cost').x1], [lean().cy, lean().cx])
  assert.deepEqual([input(client.tree(), DONE).value, input(client.tree(), COST).value], ['50', '3.00'])
  input(client.tree(), DONE).onChange({ target: { value: '75' } })
  assert.ok(target(client.tree(), 'done').y1 < lean().cy)
  assert.equal(input(client.tree(), DONE).value, '75')
  assert.equal(quadrantOfLean(client.tree()), 'Cheap but leaky')
})
