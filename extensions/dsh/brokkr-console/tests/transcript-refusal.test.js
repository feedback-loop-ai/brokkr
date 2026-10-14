// A failed `brokkr transcript`: a seat that keeps no transcript is answered in
// Brokkr's words, not as a failure, decided by the document the CLI prints on
// stdout whatever its exit code; and the notes Brokkr prints about finding
// the workspace's map never read as the refusal.
import assert from 'node:assert/strict'
import { test } from 'node:test'

import { brokkrOf } from '../lib/host/brokkr.js'
import { ReadoutRefused } from '../lib/host/readouts.js'
import { stubBrokkr } from './support/world.js'

const NOTE = 'note: the map ./realms.json found in this workspace is adopted (journal unchanged); --realms names it explicitly\n'
const WORDS = 'transcript unavailable: none: the recorded reference is explicitly none'
const TARGET = { run_id: 'landing-3a87d008', journal: './.forge/forge.db' }
/** The document `brokkr transcript --json` prints for the verify seat, with no transcript unless told otherwise. */
const doc = (over = {}) => ({ schema: 'brokkr.transcript/v1', run_id: TARGET.run_id, seat: 'seat-verify-1', turns: [], truncated: false, unavailable: 'none', full_session: null, ...over })

test('a seat that keeps no transcript is answered in Brokkr’s words, not as a failure', async () => {
  const stub = stubBrokkr([{ match: ['transcript'], responses: [{ stdout: JSON.stringify(doc()), stderr: `${NOTE}${WORDS}\n`, code: 1 }] }])
  const read = await brokkrOf(stub.config).transcript(TARGET, 'seat-verify-1')
  assert.deepEqual(read, { seat: 'seat-verify-1', turns: [], earlier: 0, truncated: false, unavailable: WORDS, full_session: null })
})

test('the document decides: the same words with no document behind them, or a document that names no reason, are the console failing', async () => {
  for (const stdout of ['', JSON.stringify(doc({ unavailable: null }))]) {
    const stub = stubBrokkr([{ match: ['transcript'], responses: [{ stdout, stderr: `${NOTE}${WORDS}\n`, code: 1 }] }])
    await assert.rejects(brokkrOf(stub.config).transcript(TARGET, 'seat-verify-1'), (error) => error instanceof ReadoutRefused && error.kind === 'failed' && error.message === WORDS, stdout)
  }
})

test('a reason Brokkr typed but did not explain is kept as its word', async () => {
  const stub = stubBrokkr([{ match: ['transcript'], responses: [{ stdout: JSON.stringify(doc({ unavailable: 'turn-not-retained' })), stderr: NOTE, code: 1 }] }])
  assert.equal((await brokkrOf(stub.config).transcript(TARGET, 'seat-verify-1')).unavailable, 'turn-not-retained')
})

test('any other refusal is reported, without Brokkr’s note about the map', async () => {
  const stub = stubBrokkr([{ match: ['transcript'], responses: [{ stdout: '', stderr: `${NOTE}error: no run "landing-3a87d008"\n`, code: 2 }] }])
  await assert.rejects(brokkrOf(stub.config).transcript(TARGET, 'seat-verify-1'), (error) => error instanceof ReadoutRefused && error.message === 'error: no run "landing-3a87d008"')
})
