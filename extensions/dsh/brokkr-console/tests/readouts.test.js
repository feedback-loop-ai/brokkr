// Each readout is read once, at the host's edge: a view version the console
// was not built for is refused by name, and a one-hearth listing becomes the
// one hearth the console paints.
import assert from 'node:assert/strict'
import { test } from 'node:test'

import { analyticsOf, fleetOf, graphOf, ONE_HEARTH, ReadoutRefused, runViewOf, VIEW_VERSION } from '../lib/host/readouts.js'
import { analyticsDoc, fleetDoc, graphDoc, provenance, recipeRow, row, runViewDoc, seat } from './support/world.js'

for (const [what, parse, doc] of [
  ['brokkr runs', fleetOf, fleetDoc([['brokkr', [row()]]])],
  ['brokkr inspect', runViewOf, runViewDoc()],
  ['brokkr analytics', analyticsOf, analyticsDoc()],
]) {
  test(`${what} at another view version is refused, not painted`, () => {
    assert.throws(
      () => parse({ ...doc, view_version: VIEW_VERSION - 1 }),
      new ReadoutRefused('version', `${what} is at view version ${VIEW_VERSION - 1}; this console paints version ${VIEW_VERSION} only`),
    )
  })
}

test('the graph is read at its own wire version, 2, and any other is refused naming the version found', () => {
  assert.equal(graphOf(graphDoc()).recipe, 'self')
  for (const found of [1, 3]) {
    assert.throws(() => graphOf(graphDoc({ view_version: found })), new ReadoutRefused('version', `brokkr graph is at view version ${found}; this console paints version 2 only`))
  }
  // The shared wire's number is not the graph's.
  assert.throws(() => graphOf(graphDoc({ view_version: VIEW_VERSION })), new ReadoutRefused('version', `brokkr graph is at view version ${VIEW_VERSION}; this console paints version 2 only`))
})

test('a graph is read whole: a park rule has no next, a terminal no seat, and a table not found says where', () => {
  const view = graphOf(graphDoc({ table: { kind: 'not_found', text: 'this recipe’s table was not found in brokkr' } }))
  assert.deepEqual(view.table, { kind: 'not_found', text: 'this recipe’s table was not found in brokkr', reason: null, bundle: null })
  assert.deepEqual(graphOf(graphDoc()).table, { kind: 'resolved', text: null, reason: null, bundle: null })
  assert.deepEqual(view.rules.find((rule) => rule.id === 'REVIEW-HOLD'), { id: 'REVIEW-HOLD', from: 'review', next: null, result: 'hold', when: [], severity: null, count: 1, kind: 'forward', text: 'REVIEW-HOLD · 1', latest: [] })
  assert.deepEqual(view.states.find((state) => state.name === 'stop').seats, [])
  assert.deepEqual(view.states.find((state) => state.name === 'review').cost, { cost: { kind: 'reported', usd: 1.5 }, text: '$1.50', silent: ['codex'], note: 'claude only · codex report none' })
  assert.throws(() => graphOf(graphDoc({ rules: [{ ...graphDoc().rules[0], kind: 'sideways' }] })), new ReadoutRefused('shape', 'rules[0].kind is not one of forward, self, back, terminal'))
})

test('a recipe that does not compile is read with the compiler’s reason and the bundle it refused, and the realm’s that do not are named', () => {
  const table = { kind: 'refused', text: "triage doesn't compile against this Brokkr", reason: "dialect 'speckit' declares phase 'specify' validate unsupported (composed: triage -> fast)", bundle: 'recipes/triage' }
  const view = graphOf(graphDoc({ recipe: 'triage', does_not_compile: ['self', 'triage'], table }))
  assert.deepEqual([view.table, view.does_not_compile], [table, ['self', 'triage']])
  assert.throws(() => graphOf(graphDoc({ table: { ...table, reason: undefined } })), new ReadoutRefused('shape', 'table.reason is not text'))
  assert.throws(() => graphOf(graphDoc({ table: { ...table, bundle: 7 } })), new ReadoutRefused('shape', 'table.bundle is not text'))
  assert.throws(() => graphOf(graphDoc({ does_not_compile: undefined })), new ReadoutRefused('shape', 'does_not_compile is not a list'))
  assert.throws(() => graphOf(graphDoc({ table: { kind: 'broken' } })), new ReadoutRefused('shape', 'table.kind is not one of resolved, not_found, refused'))
})

test('analytics carries each recipe of each realm through, its cost as Brokkr figured it and its frontier as Brokkr read it', () => {
  const doc = analyticsDoc({ recipes: [recipeRow(), recipeRow({ recipe: 'muninn', usd: null })] })
  assert.deepEqual(analyticsOf(doc).recipes, [
    { realm: 'lanetally', recipe: 'self', runs: 4, done: 3, done_rate: 75, done_text: '75%', cost_per_run: { cost: { kind: 'reported', usd: 2 }, text: '$2.00', silent: ['codex'], note: 'claude only · codex report none' }, tokens_per_run: { total: { text: '1.20M tok', absent: false, note: null }, input: { text: '1.18M', absent: false, note: null }, output: { text: '20k', absent: false, note: null }, cache_hit: { text: '95.0%', absent: false, note: null } }, on_frontier: true },
    { realm: 'lanetally', recipe: 'muninn', runs: 4, done: 3, done_rate: 75, done_text: '75%', cost_per_run: { cost: { kind: 'not_reported' }, text: 'not reported', silent: ['dsh'], note: 'dsh reports none' }, tokens_per_run: { total: { text: '1.20M tok', absent: false, note: null }, input: { text: '1.18M', absent: false, note: null }, output: { text: '20k', absent: false, note: null }, cache_hit: { text: '95.0%', absent: false, note: null } }, on_frontier: false },
  ])
  const { recipes, ...before } = analyticsDoc()
  assert.ok(recipes.length > 0)
  assert.throws(() => analyticsOf(before), new ReadoutRefused('shape', 'recipes is not a list'))
})

test('a one-hearth listing is one hearth with no journal to name', () => {
  const fleet = fleetOf({ view_version: 28, runs: [row()], count: 1 })
  assert.deepEqual(fleet.hearths.map((hearth) => [hearth.realm, hearth.journal, hearth.runs.length]), [[ONE_HEARTH, null, 1]])
})

test('a field the console paints that is missing refuses the whole readout', () => {
  const broken = fleetDoc([['brokkr', [{ ...row(), title: undefined }]]])
  assert.throws(() => fleetOf(broken), new ReadoutRefused('shape', 'realms[0].runs[0].title is not text'))
})

test('an operator event’s recorded reason is read from its payload, and nothing else of it', () => {
  const view = runViewOf(runViewDoc())
  assert.deepEqual(Object.keys(view.journal[0]).sort(), ['event_type', 'in_trail', 'reason', 'recorded_at', 'ruling', 'seq', 'what'])
})

test('a payload that is not text, not JSON or not an object refuses the row by what it is not, for a ruling as for an operator event', () => {
  const doc = runViewDoc()
  for (const [payload_json, what] of [[null, 'text'], ['nope', 'JSON'], ['[]', 'an object']]) {
    for (const event_type of ['transition/decided', 'operator/commanded']) {
      const broken = { ...doc, journal: [{ ...doc.journal[0], event_type, payload_json }] }
      assert.throws(() => runViewOf(broken), new ReadoutRefused('shape', `journal[0].payload_json is not ${what}`), `${event_type} with ${payload_json}`)
    }
  }
})

test('what a seat’s plan selected is Brokkr’s provenance line, whole; a provenance without one refuses the readout', () => {
  const selected = (over) => runViewOf(runViewDoc({ participants: [seat({ provenance: over })] })).participants[0].selected
  assert.equal(selected(null), null)
  const fallback = 'security · selected glm-flash via dsh (fallback #1; not the agent\'s first choice)'
  assert.equal(selected(provenance({ chain_index: 1, fallback: true, line: fallback })), fallback)
  assert.throws(() => selected(provenance({ line: undefined })), new ReadoutRefused('shape', 'participants[0].provenance.line is not text'))
})

test('a ruling is read from the fields its payload records, and only for a ruling', () => {
  const doc = runViewDoc()
  const decided = { ...doc.journal[0], seq: 17, payload_json: JSON.stringify({ from: 'implement', next: 'review', rule_id: 'IMPL-OK', result: 'ok', severity: 'normal', inputs: {}, problem: null }) }
  const started = { ...doc.journal[0], seq: 18, event_type: 'effect/started', payload_json: '{}' }
  const view = runViewOf({ ...doc, journal: [decided, started] })
  assert.deepEqual(view.journal[0].ruling, { from: 'implement', next: 'review', rule: 'IMPL-OK', result: 'ok', severity: 'normal' })
  assert.equal(view.journal[1].ruling, null)
})

test('a carried verdict keeps the view’s status, activity and original attempt at wire version 27', () => {
  const activity = { text: 'carried from review attempt 2', absent: false, note: 'source-attempt' }
  const source = { attempt_id: 'source-attempt', ordinal: 2 }
  const doc = runViewDoc({ participants: [seat({ status: 'carried', working: false, activity, carried_from: source })] })
  assert.equal(doc.view_version, 28)
  const part = runViewOf(doc).participants[0]
  assert.deepEqual([part.status, part.working, part.activity, part.carried_from], ['carried', false, activity, source])
  assert.equal(runViewOf(runViewDoc({ participants: [seat()] })).participants[0].carried_from, null)
  assert.throws(() => runViewOf({ ...doc, view_version: 23 }), new ReadoutRefused('version', 'brokkr inspect is at view version 23; this console paints version 28 only'))
})

test('a run view summary keeps the shared standing and its why (#909)', () => {
  const detail = 'stop accepted at seq 93; no ending written. If no driver is running it, `brokkr conclude` closes it.'
  const doc = runViewDoc({ summary: { ...runViewDoc().summary, status: 'running', standing: 'stopping', detail, park_reason: null } })
  const summary = runViewOf(doc).summary
  assert.deepEqual([summary.status, summary.standing, summary.detail], ['running', 'stopping', detail])
})
