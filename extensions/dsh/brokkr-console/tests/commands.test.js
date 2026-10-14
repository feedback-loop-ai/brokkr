// The command path (design D4): the head check, the command ids, the
// reconciliation of a lost answer, retry then resume, and Brokkr's refusal
// carried verbatim. The `brokkr` here is the stub binary; its call log is
// what was sent.
import assert from 'node:assert/strict'
import { test } from 'node:test'

import { brokkrOf } from '../lib/host/brokkr.js'
import { deskOf, RequestRefused } from '../lib/host/commands.js'
import { journalRow, json, runViewDoc, stubBrokkr } from './support/world.js'

const RUN = 'encrypt-the-token-store-41d0a9c2'
const ID = '0b6f1c2e-5d4a-4e8b-9f10-3c2d1e0f9a8b'
const TAG = ` (console command ${ID})`
const DB = '--db=/hearths/brokkr/forge.db'

function deskWith(rules, over = {}) {
  const brokkr = stubBrokkr(rules, over)
  const desk = deskOf({
    brokkr: brokkrOf(brokkr.config),
    resume: brokkr.config.resume,
    locate: (realm, runId) => (realm === 'brokkr' && runId === RUN ? { run_id: RUN, journal: '/hearths/brokkr/forge.db' } : null),
  })
  return { desk, calls: () => brokkr.calls().map((argv) => argv.join(' ')) }
}

const press = (command, over = {}) => ({ command_id: ID, realm: 'brokkr', run_id: RUN, command, reason: 'the owner-only change landed', expected_seq: 12, ...over })
const inspect = `inspect --run=${RUN} ${DB} --json`

test('a head that moved since the card was opened is not sent, and the fresh view comes back', async () => {
  const { desk, calls } = deskWith([{ match: ['inspect'], responses: [json(runViewDoc({ seq: 13 }))] }])
  const outcome = await desk.submit(press('stop'))
  assert.deepEqual([outcome.outcome, outcome.why, outcome.view.summary.seq], ['not_sent', 'head_moved', 13])
  assert.deepEqual(calls(), [inspect])
})

test('a repeated command id gets the first answer and sends nothing', async () => {
  const { desk, calls } = deskWith([
    { match: ['inspect'], responses: [json(runViewDoc())] },
    { match: ['operator', 'stop'], responses: [{ stdout: 'accepted' }] },
  ])
  const first = await desk.submit(press('stop'))
  const sent = calls()
  const again = await desk.submit(press('stop', { reason: 'pressed twice' }))
  assert.deepEqual(first, { command: 'stop', outcome: 'done' })
  assert.equal(again, first)
  assert.deepEqual(calls(), sent)
})

test('retry is followed by resume, under the configured recipe, in the run’s worktree', async () => {
  const { desk, calls } = deskWith([
    { match: ['inspect'], responses: [json(runViewDoc())] },
    { match: ['operator', 'retry'], responses: [{ stdout: 'accepted' }] },
    { match: ['resume'], responses: [{ sleep_ms: 2000 }] },
  ])
  const outcome = await desk.submit(press('retry'))
  assert.deepEqual(outcome, { command: 'retry', outcome: 'done', stage: 'resume' })
  assert.deepEqual(calls(), [
    inspect,
    `operator retry --run=${RUN} ${DB} --reason=the owner-only change landed${TAG}`,
    `resume --run=${RUN} ${DB} --recipe=self --repo=/work/brokkr --no-view`,
  ])
})

for (const command of ['stop', 'conclude']) {
  test(`${command} is not followed by resume`, async () => {
    const { desk, calls } = deskWith([
      { match: ['inspect'], responses: [json(runViewDoc())] },
      { match: [command === 'stop' ? 'operator' : 'conclude'], responses: [{ stdout: 'done' }] },
    ])
    assert.deepEqual(await desk.submit(press(command)), { command, outcome: 'done' })
    assert.equal(calls().filter((argv) => argv.startsWith('resume')).length, 0)
  })
}

test('an empty reason is refused before anything is read or sent', async () => {
  const { desk, calls } = deskWith([])
  assert.throws(() => desk.submit(press('stop', { reason: '   ' })), new RequestRefused('a reason is required'))
  assert.deepEqual(calls(), [])
})

test('a verb that refuses answers with its own words, verbatim, and nothing is retried', async () => {
  const words = 'error: run concluded: the journal moved beneath the conclusion; something is still driving this run'
  const { desk, calls } = deskWith([
    { match: ['inspect'], responses: [json(runViewDoc())] },
    { match: ['conclude'], responses: [{ stderr: `${words}\n`, code: 1 }] },
  ])
  assert.deepEqual(await desk.submit(press('conclude')), { command: 'conclude', outcome: 'refused', text: words })
  assert.equal(calls().filter((argv) => argv.startsWith('conclude')).length, 1)
})

test('a command the fresh view refuses is answered with Brokkr’s text and never invoked', async () => {
  const { desk, calls } = deskWith([{ match: ['inspect'], responses: [json(runViewDoc())] }])
  const outcome = await desk.submit(press('resume'))
  assert.deepEqual([outcome.outcome, outcome.text], ['refused', 'refused operator resume'])
  assert.deepEqual(calls(), [inspect])
})

test('a lost answer is reconciled from the record by command id, and nothing is resent', async () => {
  const accepted = runViewDoc({ journal: [journalRow(13, 'operator/commanded', { command: 'stop' }), journalRow(14, 'operator/accepted', { reason: `the owner-only change landed${TAG}` })] })
  const { desk, calls } = deskWith(
    [
      { match: ['inspect'], responses: [json(runViewDoc()), json(accepted)] },
      { match: ['operator', 'stop'], responses: [{ sleep_ms: 3000 }] },
    ],
    { timeoutMs: 1000 },
  )
  assert.deepEqual(await desk.submit(press('stop')), { command: 'stop', outcome: 'done' })
  assert.deepEqual(calls().filter((argv) => argv.startsWith('operator')).length, 1)
  assert.equal(calls().at(-1), inspect)
})

test('an answer past the output bound is never done: with no trace in the record it is not sent', async () => {
  const { desk, calls } = deskWith(
    [
      { match: ['inspect'], responses: [json(runViewDoc())] },
      { match: ['operator', 'stop'], responses: [{ bytes: 200_000 }] },
    ],
    { maxBytes: 64 * 1024 },
  )
  const outcome = await desk.submit(press('stop'))
  assert.deepEqual([outcome.outcome, outcome.why], ['not_sent', 'unconfirmed'])
  assert.deepEqual(calls().filter((argv) => argv.startsWith('operator')).length, 1)
})

test('a timeout whose record cannot be read stays checking, and a repeat re-reads without resending', async () => {
  const { desk, calls } = deskWith(
    [
      { match: ['inspect'], responses: [json(runViewDoc()), { sleep_ms: 3000 }, json(runViewDoc())] },
      { match: ['operator', 'stop'], responses: [{ sleep_ms: 3000 }] },
    ],
    { timeoutMs: 1000 },
  )
  assert.deepEqual(await desk.submit(press('stop')), { command: 'stop', outcome: 'checking' })
  const again = await desk.submit(press('stop'))
  assert.deepEqual([again.outcome, again.why], ['not_sent', 'unconfirmed'])
  assert.equal(calls().filter((argv) => argv.startsWith('operator')).length, 1)
})

test('with no override, a retry resumes under the recipe the run’s start recorded', async () => {
  const { desk, calls } = deskWith(
    [
      { match: ['inspect'], responses: [json(runViewDoc())] },
      { match: ['operator', 'retry'], responses: [{ stdout: 'accepted' }] },
      { match: ['resume'], responses: [{ sleep_ms: 2000 }] },
    ],
    { resume: { kind: 'none' } },
  )
  const outcome = await desk.submit(press('retry'))
  assert.deepEqual(outcome, { command: 'retry', outcome: 'done', stage: 'resume' })
  assert.equal(calls().at(-1), `resume --run=${RUN} ${DB} --recipe=triage --repo=/work/brokkr --no-view`)
})

test('a retry is not sent when neither an override nor the run’s start names a recipe', async () => {
  const { desk, calls } = deskWith([{ match: ['inspect'], responses: [json(runViewDoc({ recipe: null }))] }], { resume: { kind: 'none' } })
  const outcome = await desk.submit(press('retry'))
  assert.deepEqual([outcome.outcome, outcome.why], ['not_sent', 'resume_unconfigured'])
  assert.deepEqual(calls(), [inspect])
})

test('an override names the bundle every run is resumed under, over the run’s own recipe', async () => {
  const { desk, calls } = deskWith(
    [
      { match: ['inspect'], responses: [json(runViewDoc())] },
      { match: ['operator', 'retry'], responses: [{ stdout: 'accepted' }] },
      { match: ['resume'], responses: [{ sleep_ms: 2000 }] },
    ],
    { resume: { kind: 'bundle', path: '/bundles/night' } },
  )
  await desk.submit(press('retry'))
  assert.equal(calls().at(-1), `resume --run=${RUN} ${DB} --bundle=/bundles/night --repo=/work/brokkr --no-view`)
})

test('a run the fleet does not list is not sent and nothing is read', async () => {
  const { desk, calls } = deskWith([])
  assert.deepEqual(await desk.submit(press('stop', { run_id: 'someone-else' })), { command: 'stop', outcome: 'not_sent', why: 'unknown_run' })
  assert.deepEqual(calls(), [])
})
