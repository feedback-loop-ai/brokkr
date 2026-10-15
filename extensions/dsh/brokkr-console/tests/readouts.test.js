// Each readout is read once, at the host's edge: a view version the console
// was not built for is refused by name, and a one-hearth listing becomes the
// one hearth the console paints.
import assert from 'node:assert/strict'
import { test } from 'node:test'

import { analyticsOf, fleetOf, ONE_HEARTH, ReadoutRefused, runViewOf } from '../lib/host/readouts.js'
import { analyticsDoc, fleetDoc, provenance, row, runViewDoc, seat } from './support/world.js'

for (const [what, parse, doc] of [
  ['brokkr runs', fleetOf, fleetDoc([['brokkr', [row()]]])],
  ['brokkr inspect', runViewOf, runViewDoc()],
  ['brokkr analytics', analyticsOf, analyticsDoc()],
]) {
  test(`${what} at another view version is refused, not painted`, () => {
    assert.throws(
      () => parse({ ...doc, view_version: 23 }),
      new ReadoutRefused('version', `${what} is at view version 23; this console paints version 22 only`),
    )
  })
}

test('a one-hearth listing is one hearth with no journal to name', () => {
  const fleet = fleetOf({ view_version: 22, runs: [row()], count: 1 })
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
