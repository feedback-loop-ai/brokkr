// Each readout is read once, at the host's edge: a view version the console
// was not built for is refused by name, and a one-hearth listing becomes the
// one hearth the console paints.
import assert from 'node:assert/strict'
import { test } from 'node:test'

import { analyticsOf, fleetOf, ONE_HEARTH, ReadoutRefused, runViewOf } from '../lib/host/readouts.js'
import { analyticsDoc, fleetDoc, row, runViewDoc } from './support/world.js'

for (const [what, parse, doc] of [
  ['brokkr runs', fleetOf, fleetDoc([['brokkr', [row()]]])],
  ['brokkr inspect', runViewOf, runViewDoc()],
  ['brokkr analytics', analyticsOf, analyticsDoc()],
]) {
  test(`${what} at another view version is refused, not painted`, () => {
    assert.throws(
      () => parse({ ...doc, view_version: 22 }),
      new ReadoutRefused('version', `${what} is at view version 22; this console paints version 21 only`),
    )
  })
}

test('a one-hearth listing is one hearth with no journal to name', () => {
  const fleet = fleetOf({ view_version: 21, runs: [row()], count: 1 })
  assert.deepEqual(fleet.hearths.map((hearth) => [hearth.realm, hearth.journal, hearth.runs.length]), [[ONE_HEARTH, null, 1]])
})

test('a field the console paints that is missing refuses the whole readout', () => {
  const broken = fleetDoc([['brokkr', [{ ...row(), title: undefined }]]])
  assert.throws(() => fleetOf(broken), new ReadoutRefused('shape', 'realms[0].runs[0].title is not text'))
})

test('an operator event’s recorded reason is read from its payload, and nothing else of it', () => {
  const view = runViewOf(runViewDoc())
  assert.deepEqual(Object.keys(view.journal[0]).sort(), ['event_type', 'in_trail', 'reason', 'recorded_at', 'seq', 'what'])
})
