// The assistant's tools (slice two, design D2 and D3): each runs one Brokkr
// readout, checks its arguments before any process starts, says when an
// answer was cut, and `brokkr_suggest` answers with a card only for a command
// Brokkr admits right now. No tool runs a verb that writes. The `brokkr` here
// is the stub binary; its call log is what was run.
import assert from 'node:assert/strict'
import { test } from 'node:test'

import { apply } from '../lib/index.js'
import { brokkrOf } from '../lib/host/brokkr.js'
import { TOOL_BOUND_BYTES, toolsOf } from '../lib/host/tools.js'
import { fleetDoc, json, row, runViewDoc, stubBrokkr, stubHost, withEnv } from './support/world.js'

const RUN = 'encrypt-the-token-store-41d0a9c2'
const DB = '--db=/hearths/brokkr/forge.db'
const FLEET = { match: ['runs'], responses: [json(fleetDoc([['brokkr', [row()]], ['looper', [row({ run_id: 'journal-compaction-55aa55aa', title: 'Journal compaction' })]]]))] }
const WRITES = ['operator', 'resume', 'conclude', 'run', 'queue add', 'queue hold']

function toolsWith(rules) {
  const brokkr = stubBrokkr(rules)
  const tools = new Map(toolsOf({ config: brokkr.config, brokkr: brokkrOf(brokkr.config) }).map((definition) => [definition.name, definition]))
  const calls = () => brokkr.calls().map((argv) => argv.join(' '))
  return { tools, calls }
}

const wrote = (calls) => calls.filter((call) => WRITES.some((verb) => call === verb || call.startsWith(`${verb} `)))

test('the console registers exactly the four read tools and the suggestion tool with DSH', async () => {
  const host = stubHost()
  await withEnv({ BROKKR_CONSOLE_BROKKR: '/nonexistent/brokkr' }, () => apply(host.ctx))
  assert.deepEqual([...host.tools.keys()].sort(), ['brokkr_inspect', 'brokkr_queue', 'brokkr_runs', 'brokkr_suggest', 'brokkr_transcript'])
  for (const definition of host.tools.values()) {
    assert.equal(definition.parameters.type, 'object')
    assert.equal(definition.parameters.additionalProperties, false)
    assert.equal(typeof definition.execute, 'function')
    assert.equal(typeof definition.output.render, 'function')
  }
})

test('a run id outside the grammar is refused and nothing is run', async () => {
  const { tools, calls } = toolsWith([FLEET])
  for (const run of ['../etc', '', '-rf', 'a b', 'x'.repeat(200)]) {
    const answer = await tools.get('brokkr_inspect').execute({ run })
    assert.match(answer.refused, /"run" is not a run id/, run)
  }
  assert.deepEqual(await tools.get('brokkr_inspect').execute(null), { refused: 'the arguments are not an object' })
  assert.deepEqual(calls(), [])
})

test('inspect reads the fleet to find the run’s journal, then the run, and nothing else', async () => {
  const { tools, calls } = toolsWith([FLEET, { match: ['inspect'], responses: [json(runViewDoc())] }])
  const answer = await tools.get('brokkr_inspect').execute({ run: RUN })
  assert.equal(answer.cut, false)
  assert.equal(answer.readout.summary.run_id, RUN)
  assert.equal(answer.source, `brokkr inspect --run=${RUN} --json`)
  assert.deepEqual(calls(), ['runs --json', `inspect --run=${RUN} ${DB} --json`])
})

test('a run no realm lists, or one listed twice without a realm, is refused', async () => {
  const twice = { match: ['runs'], responses: [json(fleetDoc([['brokkr', [row()]], ['looper', [row()]]]))] }
  const { tools } = toolsWith([twice])
  assert.match((await tools.get('brokkr_inspect').execute({ run: 'not-a-listed-run' })).refused, /no run "not-a-listed-run" is listed/)
  assert.match((await tools.get('brokkr_inspect').execute({ run: RUN })).refused, /more than one realm; name the realm/)
})

test('a readout past the bound is cut, and the answer says so', async () => {
  const big = { stdout: JSON.stringify({ view_version: 28, pad: 'x'.repeat(TOOL_BOUND_BYTES + 10) }) }
  const { tools } = toolsWith([{ match: ['queue'], responses: [big] }])
  const answer = await tools.get('brokkr_queue').execute({})
  assert.equal(answer.cut, true)
  assert.equal(Buffer.byteLength(answer.text), TOOL_BOUND_BYTES)
  assert.match(answer.note, new RegExp(`cut at ${TOOL_BOUND_BYTES} of \\d+ bytes`))
})

test('runs can be kept to one realm, read from the one listing', async () => {
  const { tools, calls } = toolsWith([FLEET])
  const answer = await tools.get('brokkr_runs').execute({ realm: 'looper' })
  assert.deepEqual(answer.readout.realms.map((each) => each.realm), ['looper'])
  assert.deepEqual(calls(), ['runs --json'])
})

test('a transcript is read for one seat through brokkr transcript, its seat checked first', async () => {
  const { tools, calls } = toolsWith([FLEET, { match: ['transcript'], responses: [json({ schema: 'brokkr.transcript/v1', turns: [] })] }])
  assert.match((await tools.get('brokkr_transcript').execute({ run: RUN, seat: 'review; rm -rf /' })).refused, /"seat" is not a seat id/)
  const answer = await tools.get('brokkr_transcript').execute({ run: RUN, seat: 'review:security' })
  assert.equal(answer.readout.schema, 'brokkr.transcript/v1')
  assert.deepEqual(calls(), ['runs --json', `transcript --run=${RUN} ${DB} --seat=review:security --json`])
})

test('a refused readout reaches the model as Brokkr’s own words', async () => {
  const { tools } = toolsWith([{ match: ['queue'], responses: [{ code: 2, stderr: 'the queue table is locked' }] }])
  assert.deepEqual(await tools.get('brokkr_queue').execute({}), { refused: 'the queue table is locked' })
})

test('a suggestion for a command Brokkr admits now is a card naming the run, the command and the head it was read at', async () => {
  const { tools, calls } = toolsWith([FLEET, { match: ['inspect'], responses: [json(runViewDoc())] }])
  const suggest = tools.get('brokkr_suggest')
  const value = await suggest.execute({ run: RUN, command: 'retry', reason: '  the fix for the finding landed  ' })
  assert.deepEqual(value, {
    suggested: true,
    realm: 'brokkr',
    run_id: RUN,
    title: 'Encrypt the token store',
    command: 'retry',
    phase: runViewDoc().commands[0].phase ?? null,
    seq: 12,
    reason: 'the fix for the finding landed',
  })
  assert.deepEqual(suggest.output.presentationMeta({}, value), value)
  assert.deepEqual(wrote(calls()), [], 'a suggestion runs no verb that writes')
})

test('a suggestion Brokkr would refuse is no card: the model gets the refusal', async () => {
  const { tools } = toolsWith([FLEET, { match: ['inspect'], responses: [json(runViewDoc())] }])
  const value = await tools.get('brokkr_suggest').execute({ run: RUN, command: 'resume', reason: 'pick it up again' })
  assert.equal(value.suggested, false)
  assert.equal(value.text, `If pressed now, Brokkr would refuse it: ${runViewDoc().commands.find((entry) => entry.command === 'resume').text}`)
})

test('a suggestion of something Brokkr lacks is no card: the model gets the reason it is not available', async () => {
  const { tools } = toolsWith([FLEET, { match: ['inspect'], responses: [json(runViewDoc())] }])
  const lacking = await tools.get('brokkr_suggest').execute({ run: RUN, command: 'dismiss_finding', reason: 'the finding is stale' })
  assert.deepEqual(lacking, { suggested: false, text: 'Brokkr only lets you dismiss findings once a run has finished.' })
  const unlisted = await tools.get('brokkr_suggest').execute({ run: RUN, command: 'answer_here', reason: 'just answer it' })
  assert.deepEqual(unlisted, { suggested: false, text: 'Brokkr has no "answer_here" for this run' })
})

test('a suggestion with no reason, or a command Brokkr does not have, is refused before anything is read', async () => {
  const { tools, calls } = toolsWith([FLEET])
  const suggest = tools.get('brokkr_suggest')
  assert.match((await suggest.execute({ run: RUN, command: 'retry', reason: ' ' })).refused, /a reason of one sentence is required/)
  assert.match((await suggest.execute({ run: RUN, command: 'delete_run', reason: 'tidy up' })).refused, /not a command Brokkr has/)
  assert.deepEqual(calls(), [])
})

test('a suggestion on a run whose record does not fold is no card', async () => {
  const { tools } = toolsWith([FLEET, { match: ['inspect'], responses: [json(runViewDoc({ summary: null, commands: [] }))] }])
  const value = await tools.get('brokkr_suggest').execute({ run: RUN, command: 'stop', reason: 'it is stuck' })
  assert.deepEqual(value, { suggested: false, text: 'the run’s record does not fold, so Brokkr admits nothing' })
})

test('every tool renders its answer to the model as JSON text', async () => {
  const { tools } = toolsWith([])
  for (const definition of tools.values()) {
    assert.deepEqual(definition.output.render({}, { a: 1 }), [{ type: 'text', text: '{"a":1}' }])
  }
})
