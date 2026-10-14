// A seat's transcript reaches the page through one fenced route: the run and
// the seat are checked against the tools' id grammar before anything runs,
// the journal is the listing's and never the browser's, and Brokkr's readout
// is read once, its last turns only, each block cut to a bound.
import assert from 'node:assert/strict'
import { EventEmitter } from 'node:events'
import { test } from 'node:test'

import { apply } from '../lib/index.js'
import { ReadoutRefused, transcriptOf } from '../lib/host/readouts.js'
import { clientFrame, fleetDoc, json, request, row, stubBrokkr, stubHost, withEnv, WS_KEY } from './support/world.js'

const RUN = row().run_id
const SEAT = 'seat-security-1'
const ROUTE = '/brokkr-console/api/transcript'
const SCHEMA = 'brokkr.transcript/v1'

/** One assistant turn whose one text block says `text`. */
const turn = (text) => ({ role: 'assistant', ts: '2026-10-13T16:52:00Z', blocks: [{ kind: 'text', text }] })

/** `brokkr transcript --json` for the security seat: one retained turn unless told otherwise. */
const doc = (over = {}) => ({ schema: SCHEMA, seat: SEAT, turns: [turn('Reading the token store.')], truncated: false, unavailable: null, full_session: null, ...over })

test('a transcript at another schema is refused, not painted', () => {
  assert.throws(
    () => transcriptOf(doc({ schema: 'brokkr.transcript/v2' })),
    new ReadoutRefused('version', 'brokkr transcript is at brokkr.transcript/v2; this console reads brokkr.transcript/v1 only'),
  )
})

test('only the last 80 turns are kept, and the count of earlier ones is said', () => {
  const read = transcriptOf(doc({ turns: Array.from({ length: 90 }, (_, index) => turn(`turn ${index}`)) }))
  assert.deepEqual([read.turns.length, read.earlier, read.turns[0].blocks[0].text, read.turns.at(-1).blocks[0].text], [80, 10, 'turn 10', 'turn 89'])
  assert.equal(transcriptOf(doc()).earlier, 0)
})

test('a block past 4,000 characters is cut and marked; one at the bound is whole', () => {
  const read = transcriptOf(doc({ turns: [turn('x'.repeat(4001)), turn('y'.repeat(4000))] }))
  assert.equal(read.turns[0].blocks[0].text, `${'x'.repeat(4000)}… (cut; the seat's transcript has the rest)`)
  assert.equal(read.turns[1].blocks[0].text, 'y'.repeat(4000))
})

test('with no turns, Brokkr’s word for why is kept and nothing is invented', () => {
  const read = transcriptOf({ schema: SCHEMA, seat: SEAT, unavailable: 'no transcript is retained for this seat' })
  assert.deepEqual(read, { seat: SEAT, turns: [], earlier: 0, truncated: false, unavailable: 'no transcript is retained for this seat', full_session: null })
})

/**
 * The host applied over a stub `brokkr`, its listing read once through a
 * watching peer, as the page's socket reads it before the page asks for a
 * transcript. Polling is set far off so the listing is read exactly once.
 */
async function hosted(rules) {
  const brokkr = stubBrokkr([{ match: ['runs'], responses: [json(fleetDoc([['brokkr', [row()]]]))] }, ...rules])
  const host = stubHost()
  await withEnv({ BROKKR_CONSOLE_BROKKR: brokkr.program, BROKKR_STUB_DIR: brokkr.dir, BROKKR_CONSOLE_POLL_MS: '600000' }, () => apply(host.ctx))
  const socket = new EventEmitter()
  const written = []
  socket.write = (bytes) => written.push(bytes)
  socket.end = () => {}
  host.upgrades.get('/brokkr-console/ws')({ headers: { upgrade: 'websocket', 'sec-websocket-key': WS_KEY } }, socket, Buffer.alloc(0))
  socket.emit('data', clientFrame('{"type":"watch","view":"realm","run":null}'))
  // The handshake is written at once; the snapshot follows the listing's read.
  for (let waited = 0; written.length < 2; waited += 50) {
    assert.ok(waited < 10_000, 'the listing was not read within 10s')
    await new Promise((wake) => setTimeout(wake, 50))
  }
  const ask = (query) => request(host.routes.get(ROUTE).handler, { url: `${ROUTE}?${new URLSearchParams(query)}` })
  return { ask, calls: () => brokkr.calls().map((argv) => argv.join(' ')) }
}

test('a run or seat outside the id grammar, or a realm past its bound, is refused and nothing runs', async () => {
  const { ask, calls } = await hosted([])
  const asked = [
    { realm: 'brokkr', run: '../etc', seat: SEAT },
    { realm: 'brokkr', run: RUN, seat: 'review; rm -rf /' },
    { realm: 'brokkr', run: RUN },
    { realm: 'x'.repeat(257), run: RUN, seat: SEAT },
    {},
  ]
  for (const query of asked) {
    const answer = await ask(query)
    assert.deepEqual([answer.status, JSON.parse(answer.body)], [400, { error: 'a transcript is asked for by realm, run and seat' }], JSON.stringify(query))
  }
  assert.deepEqual(calls(), ['runs --json'])
})

test('a run the listing does not hold in that realm is refused, and nothing runs', async () => {
  const { ask, calls } = await hosted([])
  for (const query of [{ realm: 'brokkr', run: 'not-a-listed-run', seat: SEAT }, { realm: 'looper', run: RUN, seat: SEAT }]) {
    const answer = await ask(query)
    assert.deepEqual([answer.status, JSON.parse(answer.body)], [400, { error: 'Brokkr no longer lists this run' }], JSON.stringify(query))
  }
  assert.deepEqual(calls(), ['runs --json'])
})

test('a transcript is read against the listing’s journal, each value a flag, and reaches the page as the console read it', async () => {
  const { ask, calls } = await hosted([{ match: ['transcript'], responses: [json(doc())] }])
  const answer = await ask({ realm: 'brokkr', run: RUN, seat: SEAT })
  assert.deepEqual([answer.status, JSON.parse(answer.body)], [200, transcriptOf(doc())])
  assert.deepEqual(calls(), ['runs --json', `transcript --run=${RUN} --db=/hearths/brokkr/forge.db --seat=${SEAT} --json`])
})

test('a readout Brokkr refuses reaches the page as its own words', async () => {
  const { ask } = await hosted([{ match: ['transcript'], responses: [{ code: 2, stderr: 'no participant named seat-security-1\n' }] }])
  const answer = await ask({ realm: 'brokkr', run: RUN, seat: SEAT })
  assert.deepEqual([answer.status, JSON.parse(answer.body)], [502, { error: 'no participant named seat-security-1' }])
})
