// The fixtures the poller's tests share: a stub Brokkr's answers, a
// connected peer, a client waiting for its snapshot, and the readouts the
// tests build a world from. One builder, imported, never copied.
import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'

import { brokkrOf } from '../../lib/host/brokkr.js'
import { FLEET_WINDOW } from '../../lib/host/config.js'
import { pollerOf } from '../../lib/host/poller.js'
import { fleetOf } from '../../lib/host/readouts.js'
import { openedOn } from './client.js'
import { analyticsDoc, fleetDoc, json, row, runViewDoc, stubBrokkr } from './world.js'

export const RUN = row().run_id
export const OTHER = 'lane-detection-v2-77aa01bc'

export const fleetAt = (seq, other = 3) => json(fleetDoc([['brokkr', [row({ seq }), row({ run_id: OTHER, seq: other, title: 'Lane detection v2' })]]]))

/** Wait until `client` has been sent its snapshot, then hand it back. */
async function settle(client) {
  for (let waited = 0; client.sent.length === 0; waited += 50) {
    assert.ok(waited < 10_000, 'no snapshot within 10s')
    await new Promise((wake) => setTimeout(wake, 50))
  }
  return client
}

/** A connected peer whose every host message is recorded as one framed text would be. */
export function peer() {
  const sent = []
  let writer = (text) => sent.push(JSON.parse(text))
  const takeWriter = () => {
    if (writer === null) throw new Error('the raw writer has already been taken')
    const write = writer
    writer = null
    return write
  }
  return { sent, send: (message) => sent.push(message), onClose: () => {}, takeWriter }
}

/** Attach a client watching `watch` to `poller`, wait for its first message, and hand it back with its inbound channel. */
export async function attached(poller, watch) {
  const client = peer()
  client.receive = poller.attach(client)
  client.receive({ type: 'watch', ...watch })
  return settle(client)
}

export function polled(fleets, ask = undefined, inspections = [json(runViewDoc())]) {
  const brokkr = stubBrokkr([
    { match: ['realms'], responses: [json(realmsDoc(['brokkr']))] },
    { match: ['runs'], responses: fleets },
    { match: ['inspect'], responses: inspections },
    { match: ['queue'], responses: [json([])] },
    { match: ['muninn'], responses: [json([])] },
    { match: ['analytics'], responses: [json(analyticsDoc())] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000, ...(ask === undefined ? {} : { ask }) })
  return { poller, calls: () => brokkr.calls().map((argv) => argv[0]) }
}
/** The argv a POSIX shell hands `brokkr` for a pasted hint: the function only prints them. */
export function pasted(hint) {
  return execFileSync('bash', ['-c', `brokkr() { printf '%s\\n' "$@"; }; ${hint}`], { encoding: 'utf8' }).trimEnd().split('\n')
}

/** The browser half opened on the parked run, painting the host's `run` message. */
export function openedPage(fleetView, run, realm = 'brokkr') {
  const page = openedOn(fleetView, realm)
  page.push({ type: 'run', run })
  return page
}

/** A refused inspect for the parked run in the one-hearth world whose journal is `journal`, read and painted. */
export async function refusedHint(journal) {
  const words = 'brokkr inspect was refused: the journal could not be read'
  const world = fleetDoc([['Project Alpha', [row({ seq: 12 })]]])
  world.realms[0].journal = journal
  const realms = {
    journal: './.forge/forge.db',
    map: './realms.json',
    realms: [{ default_branch: 'main', head: '0000000', journal, name: 'Project Alpha', path: '../Project Alpha' }],
  }
  const brokkr = stubBrokkr([
    { match: ['realms'], responses: [json(realms)] },
    { match: ['runs'], responses: [json(world)] },
    { match: ['inspect'], responses: [{ stderr: words, code: 1 }] },
  ])
  const poller = pollerOf({ brokkr: brokkrOf(brokkr.config), pollMs: 60_000 })
  const client = await attached(poller, { view: 'realm', run: { realm: 'Project Alpha', run_id: RUN } })
  const run = client.sent[0].run
  return { run, page: openedPage(fleetOf(world), run, 'Project Alpha'), brokkr }
}
/** A paged `brokkr runs --json`: one window and where the next page starts, as Brokkr states both. */
export const windowOf = (runs, next = null) => ({ view_version: 28, runs, count: runs.length, next, behind: 0 })

/** `brokkr runs --json --run <id>`: the same page wire, holding that run's one row. */
export const oneRow = (row) => ({ view_version: 28, runs: [row], count: 1 })

/** `brokkr realms --json` for `names`, each in `journalOf(name)`'s journal (its own unless told otherwise). */
export const realmsDoc = (names, journalOf = (name) => `/hearths/${name}/forge.db`) => ({
  journal: './.forge/forge.db',
  map: './realms.json',
  realms: names.map((name) => ({ default_branch: 'main', head: '0000000', journal: journalOf(name), name, path: `../${name}` })),
})

/** `brokkr realms --json` for `names`: alpha reads `journal`, every other realm its own. */
export const realmsMap = (journal, names = ['alpha', 'beta']) => realmsDoc(names, (name) => (name === 'alpha' ? journal : `/hearths/${name}/forge.db`))

/** A run row of `name`'s hearth, its title and feature naming it. */
export const runOf = (name, index) => row({
  run_id: `${name}-run-${String(index).padStart(4, '0')}`,
  title: `${name} run ${index}`,
  feature: `${name} run ${index}`,
  seq: index,
})

/** Wait for `client`'s next message, then clear what it has. */
export async function heard(client) {
  for (let waited = 0; client.sent.length === 0; waited += 50) {
    assert.ok(waited < 10_000, 'no message within 10s')
    await new Promise((wake) => setTimeout(wake, 50))
  }
  const message = client.sent.at(-1)
  client.sent.length = 0
  return message
}
/** A run row standing running, so a hearth's second page can hold one. */
export const runningOf = (name, index, seq) => ({
  ...runOf(name, index),
  seq,
  verdict: { ...row().verdict, standing: 'running', rule: null, reason: null, text: 'running' },
})
/** The fleet an open run was displaced from: a hundred parked first-page runs and a running run on the page after. */
export function displaced() {
  const parked = Array.from({ length: FLEET_WINDOW }, (_, index) => runOf('alpha', index))
  const first = windowOf(parked, `needs-you:${FLEET_WINDOW}:alpha-run-0099`)
  const running = runningOf('alpha', 100, 12)
  const second = windowOf([running], null)
  // A new arrival takes the last first-page place and moves its cursor.
  const arrival = windowOf([runOf('alpha', 200), ...parked.slice(0, FLEET_WINDOW - 1)], `needs-you:${FLEET_WINDOW}:alpha-run-0098`)
  return { first, running, second, arrival }
}
/** A fresh inspect read of a running run: the summary the listing already states, with no ruling. */
export const runningRead = (seq) => {
  const doc = runViewDoc({ seq })
  doc.summary = { ...doc.summary, status: 'running', status_word: 'running', standing: 'running', phase: 'review', park_reason: null }
  doc.ruling = null
  return doc
}
/** A hundred parked first-page runs ending at cursor C, and the last page holding `last` for that cursor. */
export function retained(last) {
  const parked = Array.from({ length: FLEET_WINDOW }, (_, index) => runOf('alpha', index))
  return {
    first: windowOf(parked, `needs-you:${FLEET_WINDOW}:alpha-run-0099`),
    second: windowOf([last], null),
  }
}
