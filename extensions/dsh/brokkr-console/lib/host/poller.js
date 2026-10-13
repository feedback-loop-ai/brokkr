// Live refresh (design D5). While a client is connected, the host reads the
// fleet every few seconds. A run is re-read only when its head moved, and
// only for a client that has it open. The fleet is pushed whole when it
// changed: it is one readout over every hearth, and the panel-list count on
// every page reads every hearth. The overview's own readouts (the queue,
// Muninn's record and the last day's figures) go only to a client on the
// Realms view. A client that connects or changes what it watches gets a full
// snapshot. Nothing here touches a run: closing the panel only stops reading.
import { ReadoutRefused } from './readouts.js'

const VIEWS = ['none', 'realms', 'realm', 'analytics']

const keyOf = (realm, runId) => `${realm}\u0000${runId}`

/** A fault in the console itself, which no readout explains: DSH's log shows it. */
const report = (error) => console.error('[brokkr-console]', error)

/** Each listed run's head, by realm and run id. */
function headsOf(fleet) {
  const heads = new Map()
  for (const hearth of fleet.hearths) {
    for (const row of hearth.runs) heads.set(keyOf(hearth.realm, row.run_id), row.seq)
  }
  return heads
}

/** A watch message, checked; `null` for one the host does not read. */
function watchOf(message) {
  if (message === null || typeof message !== 'object' || message.type !== 'watch') return null
  if (!VIEWS.includes(message.view)) return null
  const run = message.run
  if (run === null || run === undefined) return { view: message.view, run: null }
  if (typeof run.realm !== 'string' || typeof run.run_id !== 'string') return null
  return { view: message.view, run: { realm: run.realm, run_id: run.run_id } }
}

/** `{value}` when the readout parsed, `{refused}` with its text when Brokkr's answer was refused. */
async function settled(read) {
  try {
    return { value: await read() }
  } catch (error) {
    if (error instanceof ReadoutRefused) return { refused: error.message }
    throw error
  }
}

/**
 * @param {{brokkr: object, pollMs: number}} parts
 */
export function pollerOf({ brokkr, pollMs }) {
  const clients = new Set()
  const state = { fleet: null, heads: new Map(), fleetText: null, day: null, overviewText: null, timer: null, busy: false }

  function locate(realm, runId) {
    const hearth = state.fleet?.hearths.find((each) => each.realm === realm)
    if (hearth === undefined || !hearth.runs.some((row) => row.run_id === runId)) return null
    return { run_id: runId, journal: hearth.journal }
  }

  /** Read the fleet; whether it differs from the last one read. */
  async function readFleet() {
    const fleet = await brokkr.fleet()
    const text = JSON.stringify(fleet)
    const changed = text !== state.fleetText
    Object.assign(state, { fleet, fleetText: text })
    return changed
  }

  async function runOf(ref) {
    const target = locate(ref.realm, ref.run_id)
    if (target === null) return { ...ref, view: null }
    return { ...ref, view: (await settled(() => brokkr.inspect(target))).value ?? null }
  }

  async function overview(fleetMoved) {
    if (fleetMoved || state.day === null) state.day = await settled(() => brokkr.analytics('24h', null))
    return { queue: await settled(brokkr.queue), muninn: await settled(brokkr.muninn), day: state.day }
  }

  async function snapshot(client) {
    try {
      const moved = await readFleet()
      state.heads = headsOf(state.fleet)
      const { view, run } = client.watch
      client.peer.send({
        type: 'snapshot',
        fleet: state.fleet,
        overview: view === 'realms' ? await overview(moved) : null,
        run: run === null ? null : await runOf(run),
      })
    } catch (error) {
      if (!(error instanceof ReadoutRefused)) throw error
      client.peer.send({ type: 'error', message: error.message })
    }
  }

  /** Re-read each open run whose head moved, once, for the clients that have it open. */
  async function pushRuns(before) {
    const open = new Map()
    for (const client of clients) {
      const run = client.watch.run
      if (run === null) continue
      const key = keyOf(run.realm, run.run_id)
      if (state.heads.get(key) !== before.get(key)) open.set(key, [...(open.get(key) ?? []), client])
    }
    for (const watchers of open.values()) {
      const run = await runOf(watchers[0].watch.run)
      for (const client of watchers) client.peer.send({ type: 'run', run })
    }
  }

  async function tick() {
    if (state.busy || clients.size === 0) return
    state.busy = true
    try {
      const before = state.heads
      const moved = await readFleet()
      state.heads = headsOf(state.fleet)
      if (moved) for (const client of clients) client.peer.send({ type: 'fleet', fleet: state.fleet })
      await pushRuns(before)
      const onOverview = [...clients].filter((client) => client.watch.view === 'realms')
      if (onOverview.length === 0) return
      const next = await overview(moved)
      const text = JSON.stringify(next)
      if (text === state.overviewText) return
      state.overviewText = text
      for (const client of onOverview) client.peer.send({ type: 'overview', overview: next })
    } catch (error) {
      if (!(error instanceof ReadoutRefused)) throw error
      for (const client of clients) client.peer.send({ type: 'error', message: error.message })
    } finally {
      state.busy = false
    }
  }

  function attach(peer) {
    const client = { peer, watch: { view: 'none', run: null } }
    clients.add(client)
    peer.onClose = () => {
      clients.delete(client)
      if (clients.size === 0 && state.timer !== null) {
        clearInterval(state.timer)
        state.timer = null
      }
    }
    if (state.timer === null) {
      state.timer = setInterval(() => tick().catch(report), pollMs)
      state.timer.unref()
    }
    return (message) => {
      const watch = watchOf(message)
      if (watch === null) return
      client.watch = watch
      snapshot(client).catch(report)
    }
  }

  return { locate, attach, tick }
}
