// A failed `brokkr transcript`: a seat that keeps no transcript is answered in
// Brokkr's words, not as a failure, and the notes Brokkr prints about finding
// the workspace's map never read as the refusal.
import assert from 'node:assert/strict'
import { test } from 'node:test'

import { brokkrOf } from '../lib/host/brokkr.js'
import { ReadoutRefused } from '../lib/host/readouts.js'
import { stubBrokkr } from './support/world.js'

const NOTE = 'note: the map ./realms.json found in this workspace is adopted (journal unchanged); --realms names it explicitly\n'
const TARGET = { run_id: 'landing-3a87d008', journal: './.forge/forge.db' }

test('a seat that keeps no transcript is answered in Brokkr’s words, not as a failure', async () => {
  const stub = stubBrokkr([{ match: ['transcript'], responses: [{ stdout: '', stderr: `${NOTE}transcript unavailable: none: the recorded reference is explicitly none\n`, code: 1 }] }])
  const read = await brokkrOf(stub.config).transcript(TARGET, 'seat-verify-1')
  assert.deepEqual(read, { seat: 'seat-verify-1', turns: [], earlier: 0, truncated: false, unavailable: 'transcript unavailable: none: the recorded reference is explicitly none', full_session: null })
})

test('any other refusal is reported, without Brokkr’s note about the map', async () => {
  const stub = stubBrokkr([{ match: ['transcript'], responses: [{ stdout: '', stderr: `${NOTE}error: no run "landing-3a87d008"\n`, code: 2 }] }])
  await assert.rejects(brokkrOf(stub.config).transcript(TARGET, 'seat-verify-1'), (error) => error instanceof ReadoutRefused && error.message === 'error: no run "landing-3a87d008"')
})
