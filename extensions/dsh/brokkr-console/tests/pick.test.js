// The run page, as the D board lays it out. The rail draws Brokkr's phases
// as brokkr tui draws them. A phase, a step or an arrow can be picked; the
// reader shows the pick and the transcript of the seat behind it; every seat
// is compared in one table. A question is about the pick first and its run
// second. Picks outlive live refreshes, and live steps wear Brokkr's pulse.
import assert from 'node:assert/strict'
import { test } from 'node:test'

import { fleetOf, runViewOf } from '../lib/host/readouts.js'
import { buttons, findAll, mountClient, textOf } from './support/client.js'
import { fleetDoc, journalRow, provenance, row, runViewDoc, seat } from './support/world.js'

const RUN = row().run_id
const TRANSCRIPT = '/brokkr-console/api/transcript'
const settled = async () => {
  for (let tick = 0; tick < 5; tick += 1) await new Promise((wake) => setTimeout(wake, 0))
}
const member = (key, label, stateClass = 'on-phosphor', state = 'finished') => ({ label, key, state, state_class: stateClass, model: { text: 'gpt-5.5', absent: false, note: null }, boundary: { text: 'namespace', absent: false, note: null } })
const ruled = (seq, from, next, rule) => ({ ...journalRow(seq, 'transition/decided', { from, next, rule_id: rule, result: 'ok', severity: 'normal' }, `${rule} ${from} → ${next} · ok`), what: { text: `${rule} ${from} → ${next} · ok`, badge: rule } })

/** Implement, then a review whose two positions ran side by side, the security one parked. */
const reviewed = (over = {}) => runViewDoc({
  phases: [
    { name: 'implement', strategy: 'code', visits: 2, current: false, plain: true, columns: [{ label: null, nodes: [member('seat-implement-1', 'implement')] }], returns: ['review'] },
    { name: 'review', strategy: null, visits: 1, current: true, plain: false, columns: [{ label: 'positions', nodes: [member('seat-correctness-1', 'correctness'), member('seat-security-1', 'security', 'on-park', 'indeterminate')] }], returns: [] },
  ],
  participants: [
    seat({ key: 'seat-implement-1', label: 'implement', phase: 'implement', cost_cell: { text: '$0.8100', absent: false, note: null }, tokens: { ...seat().tokens, cache_hit: { text: '—', absent: true, note: 'no cache reads and input reported to compare' } } }),
    seat({ key: 'seat-correctness-1', label: 'correctness', phase: 'review', cost_cell: { text: '$0.4000', absent: false, note: null } }),
    seat(),
  ],
  journal: [ruled(9, 'implement', 'review', 'IMPL-OK'), ruled(10, 'review', 'implement', 'REVIEW-REFORGE'), ruled(11, 'implement', 'review', 'IMPL-OK')],
  ...over,
})
const TURNS = { seat: 'seat-security-1', turns: [{ role: 'assistant', ts: '2026-10-13T16:52:00Z', blocks: [{ kind: 'text', text: 'Reading the token store.' }] }, { role: 'assistant', ts: '2026-10-13T16:53:00Z', blocks: [{ kind: 'tool', text: 'Bash · stat -c %a tokens' }] }], earlier: 0, truncated: false, unavailable: null, full_session: null }

function onRun(view = reviewed(), answers = { [TRANSCRIPT]: TURNS }) {
  const client = mountClient('main', {}, answers)
  client.push({ type: 'snapshot', workspace: '/w/console', ask: true, fleet: fleetOf(fleetDoc([['brokkr', [row()]]])), overview: null, run: null })
  buttons(client.tree()).find((button) => textOf(button).startsWith('brokkr')).props.onClick()
  buttons(client.tree()).find((button) => textOf(button).startsWith('Encrypt the token store')).props.onClick()
  client.push({ type: 'run', run: { realm: 'brokkr', run_id: RUN, view: runViewOf(view) } })
  return client
}

const rail = (tree) => findAll(tree, (node) => node.type === 'svg' && node.props['aria-label'] === 'Run graph')[0]
const steps = (tree) => findAll(rail(tree), (node) => node.props.role === 'button' && String(node.props['aria-label']).startsWith('the step '))
const picked = (tree) => steps(tree).filter((node) => node.props['aria-pressed']).map((node) => node.props['aria-label'].slice('the step '.length))
const press = (tree, label) => findAll(rail(tree), (node) => node.props['aria-label'] === label)[0].props.onClick()
const reader = (tree) => findAll(tree, (node) => node.props['aria-label'] === 'Reader')[0]
const seats = (tree) => findAll(tree, (node) => node.props['aria-label'] === 'Seats')[0]
const crumbs = (tree) => findAll(tree, (node) => ['bk-crumb', 'bk-crumb first'].includes(node.props.className)).map(textOf)

test('the rail draws a dot for each step, a bracket for the members of one step, phase names under it and the current phase boxed', () => {
  const client = onRun()
  assert.deepEqual(steps(client.tree()).map((node) => node.props['aria-label']), ['the step implement', 'the step correctness', 'the step security'])
  const dots = findAll(rail(client.tree()), (node) => node.type === 'circle' && String(node.props.className).startsWith('bk-dot-fill'))
  assert.deepEqual(dots.map((dot) => dot.props.className), ['bk-dot-fill phosphor', 'bk-dot-fill phosphor', 'bk-dot-fill park'])
  // The two positions share an x and sit either side of the rail.
  assert.equal(dots[1].props.cx, dots[2].props.cx)
  assert.ok(dots[1].props.cy < dots[0].props.cy && dots[2].props.cy > dots[0].props.cy)
  assert.equal(findAll(rail(client.tree()), (node) => node.props.className === 'bk-bracket').length, 3)
  const words = textOf(rail(client.tree()))
  assert.match(words, /IMPLEMENT · 2 visits · code/)
  assert.match(words, /NOW · REVIEW/)
  assert.match(words, /positions/)
  assert.deepEqual(findAll(rail(client.tree()), (node) => node.type === 'rect' && String(node.props.className).startsWith('bk-now')).map((box) => box.props.className), ['bk-now park'])
})

test('a phase name picks every step in it, the reader reads the phase, and the seats table shows only its seats', () => {
  const client = onRun()
  press(client.tree(), 'the phase review')
  assert.deepEqual(picked(client.tree()), ['correctness', 'security'])
  assert.match(textOf(reader(client.tree())), /^phase · review/)
  assert.match(textOf(seats(client.tree())), /^seats · 2 in review/)
  assert.doesNotMatch(textOf(seats(client.tree())), /implement/)
  assert.deepEqual(crumbs(client.tree()), ['phase review', 'run Encrypt the token store', 'realm brokkr'])
  press(client.tree(), 'the phase review')
  assert.deepEqual(picked(client.tree()), [])
})

test('a step picks itself; the reader shows its seat with the tokens read apart, and its transcript', async () => {
  const client = onRun()
  press(client.tree(), 'the step security')
  await settled()
  assert.deepEqual(picked(client.tree()), ['security'])
  const text = textOf(reader(client.tree()))
  assert.match(text, /^step · security/)
  assert.match(text, /TokensΣ 3\.01M tok↑ 3\.00M↓ 1\.2k/)
  assert.match(text, /Cache hit99\.9%/)
  assert.match(text, /Cost\$2\.4179/)
  assert.match(text, /Turns541 attempt/)
  assert.match(text, /gpt-5\.5 \(claimed\)/)
  assert.ok(client.fetches.some((call) => call.url === `${TRANSCRIPT}?realm=brokkr&run=${RUN}&seat=seat-security-1`))
  assert.match(text, /Reading the token store\./)
  assert.match(text, /tool · Bash · stat -c %a tokens/)
})

test('a transcript turn’s time reads in every spelling Brokkr records: ISO, DSH’s epoch milliseconds, or none', async () => {
  const at = (ts, text) => ({ role: 'assistant', ts, blocks: [{ kind: 'text', text }] })
  const hhmm = (instant) => new Date(instant).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
  const turns = [at('2026-10-13T16:52:00Z', 'Claude.'), at('1792110878410', 'DSH.'), at('', 'Unrecorded.'), at('not a time', 'Unreadable.')]
  const client = onRun(reviewed(), { [TRANSCRIPT]: { ...TURNS, turns } })
  press(client.tree(), 'the step security')
  await settled()
  const text = textOf(reader(client.tree()))
  assert.doesNotMatch(text, /Invalid Date/)
  assert.ok(text.includes(`${hhmm('2026-10-13T16:52:00Z')}Claude.`))
  assert.ok(text.includes(`${hhmm(1792110878410)}DSH.`))
  assert.ok(text.includes(`DSH.Unrecorded.Unreadable.`))
})

test('an absent figure says why, in Brokkr’s words', () => {
  const client = onRun()
  press(client.tree(), 'the step implement')
  assert.match(textOf(reader(client.tree())), /Cache hit—no cache reads and input reported to compare/)
})

test('an arrow picks the ruling that moved the run, and the seat behind it', async () => {
  const client = onRun()
  press(client.tree(), 'the arrow implement → review')
  await settled()
  assert.equal(findAll(rail(client.tree()), (node) => node.type === 'path' && node.props.className === 'bk-edge sel').length, 1)
  assert.match(textOf(rail(client.tree())), /IMPL-OK/)
  const text = textOf(reader(client.tree()))
  assert.match(text, /^arrow · implement → review/)
  assert.match(text, /IMPL-OK implement → review · ok/)
  assert.match(text, /The seat that moved itimplement/)
  assert.ok(client.fetches.some((call) => call.url.endsWith('seat=seat-implement-1')))
  assert.deepEqual(crumbs(client.tree()), ['arrow implement → review', 'run Encrypt the token store', 'realm brokkr'])
  // A seat with no recorded tokens reads as one absence, not three.
  const quiet = reviewed()
  for (const cell of ['total', 'input', 'output']) quiet.participants[0].tokens[cell] = { text: '—', absent: true, note: 'not reported' }
  const plain = onRun(quiet)
  press(plain.tree(), 'the step implement')
  assert.match(textOf(reader(plain.tree())), /Tokens—not reported/)
})

test('the question is about the pick first and the run second', async () => {
  const client = onRun()
  press(client.tree(), 'the arrow implement → review')
  const bar = () => findAll(client.tree(), (node) => node.props['aria-label'] === 'Asking about')[0]
  findAll(bar(), (node) => node.type === 'input')[0].props.onChange({ target: { value: 'Why did it move on?' } })
  buttons(bar()).find((button) => textOf(button) === 'Ask').props.onClick()
  await settled()
  const prompt = client.dsh.calls.find((call) => call[0] === 'setDraft')[2]
  assert.match(prompt, /^About the ruling that moved the run from implement to review \(IMPL-OK, seq 11\), and the seat behind it, “implement” \(participant key seat-implement-1; its transcript is brokkr_transcript with seat seat-implement-1\)\. Its outer context is the Brokkr run “Encrypt the token store” \(realm brokkr, run id encrypt-the-token-store-41d0a9c2\): read the run for it\./)
  assert.match(prompt, /Question: Why did it move on\?$/)
})

test('with nothing picked, the reader shows where the run is now: the latest seat in the phase the view marks current', async () => {
  const client = onRun()
  await settled()
  assert.match(textOf(reader(client.tree())), /^now · securityWhere the run is now/)
  assert.deepEqual(crumbs(client.tree()), ['run Encrypt the token store', 'realm brokkr'])
  // Sent back to implement: its seat is where the run is now, not the last seat listed.
  const returned = reviewed()
  returned.phases[0].current = true
  returned.phases[1].current = false
  assert.match(textOf(reader(onRun(returned).tree())), /^now · implementWhere the run is now/)
  // Sent back with no seat recorded there yet: the phase itself, never the seat of the phase before.
  returned.participants.shift()
  assert.match(textOf(reader(onRun(returned).tree())), /^now · implementWhere the run is now.*Brokkr recorded no seat in this phase\./)
})

test('a transcript read the head has moved past is dropped when it lands late', async () => {
  let release = null
  const held = new Promise((settle) => {
    release = settle
  })
  const said = (text) => ({ ...TURNS, turns: [{ role: 'assistant', ts: '2026-10-13T16:54:00Z', blocks: [{ kind: 'text', text }] }] })
  let reads = 0
  const client = onRun(reviewed(), { [TRANSCRIPT]: () => ((reads += 1) === 1 ? held : said('The later read.')) })
  press(client.tree(), 'the step security')
  await settled()
  client.push({ type: 'run', run: { realm: 'brokkr', run_id: RUN, view: runViewOf(reviewed({ seq: 13 })) } })
  await settled()
  assert.match(textOf(reader(client.tree())), /The later read\./)
  release(said('The earlier read, landing late.'))
  await settled()
  assert.match(textOf(reader(client.tree())), /The later read\./)
  assert.doesNotMatch(textOf(reader(client.tree())), /landing late/)
})

test('every seat is compared in one table; the seat behind the pick is marked, and a row picks it', () => {
  const client = onRun()
  const table = () => seats(client.tree())
  assert.match(textOf(table()), /implementsucceededΣ 3\.01M tok↑ 3\.00M↓ 1\.2k—\$0\.8100gpt-5\.5/)
  assert.match(textOf(table()), /▸ securitysucceeded/)
  buttons(table()).find((button) => textOf(button) === 'correctness').props.onClick()
  assert.deepEqual(picked(client.tree()), ['correctness'])
  assert.match(textOf(table()), /▸ correctness/)
})

test('a pick outlives the run’s live refresh, and is dropped once the view no longer holds it', () => {
  const client = onRun()
  press(client.tree(), 'the step security')
  client.push({ type: 'run', run: { realm: 'brokkr', run_id: RUN, view: runViewOf(reviewed({ seq: 13 })) } })
  assert.deepEqual(picked(client.tree()), ['security'])
  const gone = reviewed({ seq: 14 })
  gone.phases[1].columns[0].nodes.pop()
  gone.participants.pop()
  client.push({ type: 'run', run: { realm: 'brokkr', run_id: RUN, view: runViewOf(gone) } })
  assert.deepEqual(crumbs(client.tree()), ['run Encrypt the token store', 'realm brokkr'])
})

test('a seat Brokkr recorded no transcript for says so in Brokkr’s words, and nothing is asked', async () => {
  const view = reviewed()
  view.participants[0].transcript_cell = { text: '—', absent: true, note: 'no transcript reference was recorded' }
  const client = onRun(view)
  press(client.tree(), 'the step implement')
  await settled()
  assert.match(textOf(reader(client.tree())), /Transcriptimplementno transcript reference was recorded/)
  assert.ok(!client.fetches.some((call) => call.url.endsWith('seat=seat-implement-1')))
})

test('the transcript is read again when the run moves, and says why when Brokkr has none', async () => {
  const client = onRun(reviewed(), { [TRANSCRIPT]: { ...TURNS, turns: [], unavailable: 'no transcript is retained for this seat' } })
  press(client.tree(), 'the step security')
  await settled()
  const reads = () => client.fetches.filter((call) => call.url.startsWith(TRANSCRIPT) && call.url.endsWith('seat-security-1')).length
  const before = reads()
  assert.match(textOf(reader(client.tree())), /no transcript is retained for this seat/)
  client.push({ type: 'run', run: { realm: 'brokkr', run_id: RUN, view: runViewOf(reviewed({ seq: 13 })) } })
  await settled()
  assert.equal(reads(), before + 1)
})

test('a seat says what its plan selected in Brokkr’s own line, in the seats table and the reader, and nothing when Brokkr recorded none', () => {
  const line = 'security · selected glm-flash via dsh (fallback #1; not the agent\'s first choice)'
  const view = reviewed()
  view.participants[2].provenance = provenance({ chain_index: 1, fallback: true, line })
  const client = onRun(view)
  assert.equal(textOf(seats(client.tree())).split(line).length, 2, 'once, under the security seat')
  assert.doesNotMatch(textOf(seats(client.tree())), /a fallback/)
  press(client.tree(), 'the step security')
  assert.match(textOf(reader(client.tree())), /Modelgpt-5\.5security · selected glm-flash via dsh \(fallback #1; not the agent's first choice\)/)
  press(client.tree(), 'the step security')
  press(client.tree(), 'the step implement')
  assert.match(textOf(reader(client.tree())), /Modelgpt-5\.5claimed · boundary not applicable/)
})

test('a live step wears the pulse while the run is driven, and only then', () => {
  const running = reviewed({ summary: { ...runViewDoc().summary, status: 'running', park_reason: null } })
  running.phases[1].columns[0].nodes[1] = member('seat-security-1', 'security', 'in-active', 'active')
  const client = onRun(running)
  const dots = (tree) => findAll(rail(tree), (node) => node.type === 'circle' && String(node.props.className).startsWith('bk-dot')).map((dot) => dot.props.className)
  assert.deepEqual(dots(client.tree()), ['bk-dot-fill phosphor', 'bk-dot-fill phosphor', 'bk-dot-halo violet', 'bk-dot-fill violet live'])
  assert.deepEqual(findAll(rail(client.tree()), (node) => String(node.props.className).startsWith('bk-now')).map((node) => node.props.className), ['bk-now violet', 'bk-now-label violet'])
  const parked = onRun(reviewed())
  assert.ok(dots(parked.tree()).every((name) => !name.includes('live') && !name.includes('halo')))
})

test('a phase with no recorded steps is one dot, coloured as the TUI colours it', () => {
  const plain = runViewDoc({ summary: { ...runViewDoc().summary, status: 'running' }, phases: [
    { name: 'intake', strategy: null, visits: 1, current: false, plain: true, columns: [], returns: [] },
    { name: 'design', strategy: null, visits: 1, current: true, plain: true, columns: [], returns: [] },
  ] })
  const client = onRun(plain)
  const dots = findAll(rail(client.tree()), (node) => node.type === 'circle' && String(node.props.className).startsWith('bk-dot-fill')).map((dot) => dot.props.className)
  assert.deepEqual(dots, ['bk-dot-fill visited', 'bk-dot-fill phosphor live'])
  assert.doesNotMatch(textOf(rail(client.tree())), /reached|current/)
})

test('the road back is solid, with the forward arrowhead and the ruling that sent it back', () => {
  const client = onRun()
  const back = findAll(rail(client.tree()), (node) => node.type === 'path' && node.props.className === 'bk-edge back')
  assert.equal(back.length, 1)
  assert.equal(back[0].props.markerEnd, 'url(#bk-tip)')
  assert.match(textOf(rail(client.tree())), /sent back to implement by review · REVIEW-REFORGE/)
})

test('a rail wider than its pane opens scrolled to the current phase, once per run', () => {
  const client = onRun()
  const scroller = findAll(client.tree(), (node) => node.props.className === 'bk-graph')[0]
  const element = { dataset: {}, clientWidth: 120, scrollLeft: 0 }
  scroller.props.ref(element)
  assert.ok(element.scrollLeft > 0)
  element.scrollLeft = 0
  scroller.props.ref(element)
  assert.equal(element.scrollLeft, 0, 'the operator’s own scroll is left alone')
  const wide = { dataset: {}, clientWidth: 4000, scrollLeft: 0 }
  scroller.props.ref(wide)
  assert.equal(wide.scrollLeft, 0, 'a rail that fits is not scrolled')
})

test('the brand mark strikes while any run in the fleet is running', () => {
  const quiet = mountClient('main')
  quiet.push({ type: 'snapshot', fleet: fleetOf(fleetDoc([['brokkr', [row()]]])), overview: null, run: null })
  const mark = (tree) => findAll(tree, (node) => (node.props.className ?? '').startsWith('bk-mark'))[0].props.className
  assert.equal(mark(quiet.tree()), 'bk-mark')
  const running = fleetOf(fleetDoc([['brokkr', [row({ verdict: { standing: 'running', rule: null, reason: null, text: '' } })]]]))
  const live = mountClient('main')
  live.push({ type: 'snapshot', fleet: running, overview: null, run: null })
  assert.equal(mark(live.tree()), 'bk-mark forging')
  const icon = mountClient('sidebar.panellist', { size: 16, active: false })
  icon.push({ type: 'snapshot', fleet: running, overview: null, run: null })
  assert.equal(mark(icon.tree()), 'bk-mark forging')
})
