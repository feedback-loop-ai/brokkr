// Live refresh (design D5). While a client is connected, the host reads the
// fleet every few seconds. A run is re-read only when its head moved, and
// only for a client that has it open. The fleet is pushed whole when it
// changed: it is one readout over every hearth, and the panel-list count on
// every page reads every hearth. The overview's own readouts (the queue,
// Muninn's record and the last day's figures) go only to a client on the
// Realms view. A client that connects or changes what it watches gets a full
// snapshot. Nothing here touches a run: closing the panel only stops reading.
import { realpath } from 'node:fs/promises'
import { isAbsolute, resolve } from 'node:path'

import { ReadoutRefused } from './readouts.js'

const VIEWS = ['none', 'realms', 'realm', 'analytics']

const keyOf = (realm, runId) => `${realm}\u0000${runId}`

/** Whether a hearth's listing is the whole hearth: no later page, no run behind, no refusal. */
const whole = (hearth) => hearth.detail === null && (hearth.next === null || hearth.next === undefined) && hearth.behind === 0

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
  const named = message.realm
  const realm = typeof named === 'string' && named.length <= 256 ? named : typeof run?.realm === 'string' ? run.realm : null
  if (run === null || run === undefined) return { view: message.view, run: null, realm }
  if (typeof run.realm !== 'string' || typeof run.run_id !== 'string') return null
  return { view: message.view, run: { realm: run.realm, run_id: run.run_id }, realm }
}

/** A client asking for one hearth's next page, checked; `null` for one the host does not read. */
function moreOf(message) {
  if (message === null || typeof message !== 'object' || message.type !== 'more') return null
  return typeof message.realm === 'string' && message.realm.length <= 256 ? { realm: message.realm } : null
}

/**
 * A journal's identity as Brokkr's `World::hearths` reads it: its canonical
 * path, or its resolved path when the file does not exist yet, so two realm
 * rows naming one journal through different spellings are one hearth. The
 * named path is handed to the OS as written (only made absolute), never
 * normalized first: a symlink followed by `..` must resolve at the kernel,
 * as `realpathSync.native` and Rust's `canonicalize` both do.
 */
async function identityOf(journal, workspace) {
  const base = workspace ?? process.cwd()
  const named = isAbsolute(journal) ? journal : `${base}/${journal}`
  try {
    return await realpath(named)
  } catch {
    return resolve(base, journal)
  }
}

/**
 * The hearths `brokkr realms --json` names, one per distinct journal, each
 * under the realm names sharing it (Brokkr's `a+b` label and map order);
 * `null` when the workspace has no map and is the one hearth it always was.
 */
async function hearthsOf(brokkr, workspace) {
  let realms
  try {
    realms = await brokkr.realms()
  } catch (error) {
    if (error instanceof ReadoutRefused) return null
    throw error
  }
  const byJournal = new Map()
  for (const realm of realms) {
    const journal = await identityOf(realm.journal, workspace)
    const held = byJournal.get(journal)
    if (held === undefined) byJournal.set(journal, { journal, names: [realm.name] })
    else held.names.push(realm.name)
  }
  return [...byJournal.values()].map(({ journal, names }) => ({ realm: names.join('+'), journal }))
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
 * @param {{brokkr: object, pollMs: number, workspace?: string, ask?: boolean}} parts
 *   `workspace` is the directory the console runs Brokkr in, where Ask Brokkr opens a session;
 *   `ask` shows the console's question boxes (`BROKKR_CONSOLE_ASK=1`), hidden otherwise.
 */
export function pollerOf({ brokkr, pollMs, workspace = null, ask = false }) {
  const clients = new Set()
  const state = { fleet: null, extras: new Map(), heads: new Map(), fleetText: null, day: null, overviewText: null, timer: null, busy: false }

  /**
   * A run's journal from the latest fleet, or `null` when the listing is
   * whole and does not hold it. A page that says more or behind cannot show
   * a run was dropped: the run is read through the journal its realm names.
   */
  function locate(realm, runId) {
    const hearth = state.fleet?.hearths.find((each) => each.realm === realm)
    if (hearth === undefined) return null
    if (whole(hearth) && !hearth.runs.some((row) => row.run_id === runId)) return null
    return { run_id: runId, journal: hearth.journal }
  }

  /** The realms whose hearths a client has on screen: what a refresh keeps paged into. */
  function watched() {
    const realms = new Set()
    for (const client of clients) if (client.watch.realm !== null) realms.add(client.watch.realm)
    return realms
  }

  /** One hearth's first page, or each hearth's in a world of several; a hearth Brokkr cannot read keeps its refusal. */
  async function readHearths(hearths) {
    if (hearths === null || hearths.length < 2) return { hearths: [await brokkr.fleetPage(null)] }
    const each = async (hearth) => {
      const read = await settled(() => brokkr.fleetPage(hearth.journal))
      return read.refused === undefined
        ? { ...read.value, realm: hearth.realm, journal: hearth.journal, detail: null }
        : { realm: hearth.realm, journal: hearth.journal, detail: read.refused, runs: [], next: null, behind: 0 }
    }
    return { hearths: await Promise.all(hearths.map(each)) }
  }

  /**
   * Read the fleet; whether it differs from the last one read, and the rows
   * a refresh kept from a page already loaded. Every poll reads each
   * hearth's first page only; the pages an operator paged into are kept for
   * the hearth on screen while the first page still ends where they began,
   * and let go otherwise so no run is left unreachable.
   */
  async function readFleet() {
    const fleet = await readHearths(await hearthsOf(brokkr, workspace))
    const onScreen = watched()
    const paged = new Set()
    for (const [realm, extra] of [...state.extras]) {
      const hearth = fleet.hearths.find((each) => each.realm === realm)
      if (!onScreen.has(realm) || hearth === undefined || hearth.next !== extra.from) {
        state.extras.delete(realm)
        continue
      }
      const listed = new Set(hearth.runs.map((row) => row.run_id))
      for (const row of extra.runs) {
        if (listed.has(row.run_id)) continue
        listed.add(row.run_id)
        hearth.runs.push(row)
        paged.add(keyOf(realm, row.run_id))
      }
      hearth.next = extra.next
    }
    const text = JSON.stringify(fleet)
    const changed = text !== state.fleetText
    Object.assign(state, { fleet, fleetText: text })
    return { changed, paged }
  }

  /**
   * Read the page after `realm`'s `next`, append it to that hearth, and push
   * the fleet: the operator asked for more, and a poll keeps it only while
   * this realm is on screen. A second request for the page another request
   * already read appends nothing, and no row is listed twice.
   */
  async function morePage(realm) {
    const first = state.fleet?.hearths.find((each) => each.realm === realm)
    if (first === undefined || first.next === null || first.next === undefined) return
    const cursor = first.next
    const read = await settled(() => brokkr.fleetPage(first.journal, cursor))
    if (read.refused !== undefined) {
      for (const client of clients) client.peer.send({ type: 'error', message: read.refused })
      return
    }
    // A poll may have replaced the fleet while this page was in flight: append
    // to the hearth that is current now, never to the one the read started on.
    const hearth = state.fleet?.hearths.find((each) => each.realm === realm)
    if (hearth === undefined || hearth.journal !== first.journal || hearth.next !== cursor) return
    const listed = new Set(hearth.runs.map((row) => row.run_id))
    const added = []
    for (const row of read.value.runs) {
      if (listed.has(row.run_id)) continue
      listed.add(row.run_id)
      added.push(row)
    }
    hearth.runs.push(...added)
    hearth.next = read.value.next
    hearth.behind = read.value.behind
    const kept = state.extras.get(realm)
    const runs = [...(kept?.runs ?? [])]
    const held = new Set(runs.map((row) => row.run_id))
    for (const row of read.value.runs) {
      if (held.has(row.run_id)) continue
      held.add(row.run_id)
      runs.push(row)
    }
    state.extras.set(realm, { from: kept?.from ?? cursor, runs, next: read.value.next })
    state.fleetText = JSON.stringify(state.fleet)
    for (const client of clients) client.peer.send({ type: 'fleet', fleet: state.fleet })
  }

  /**
   * A listed run's view; a refused read's own words and the command that reads
   * it beside `view: null`; or `listed: false` when the fleet no longer holds
   * the run, which is a fact about the run rather than a read still in flight.
   */
  async function runOf(ref) {
    const target = locate(ref.realm, ref.run_id)
    if (target === null) return { ...ref, view: null, listed: false }
    const read = await settled(() => brokkr.inspect(target))
    return read.refused === undefined
      ? { ...ref, view: read.value ?? null }
      : { ...ref, view: null, refused: read.refused, hint: brokkr.inspectHint(target) }
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
        workspace,
        ask,
        fleet: state.fleet,
        overview: view === 'realms' ? await overview(moved.changed) : null,
        run: run === null ? null : await runOf(run),
      })
    } catch (error) {
      if (!(error instanceof ReadoutRefused)) throw error
      client.peer.send({ type: 'error', message: error.message })
    }
  }

  /**
   * Re-read each open run whose head moved, once, for the clients that have
   * it open. A run loaded only from a page past the first has no fresh head
   * to compare, so it is read each poll and pushed only when its head moved.
   */
  async function pushRuns(before, paged) {
    const open = new Map()
    for (const client of clients) {
      const run = client.watch.run
      if (run === null) continue
      const key = keyOf(run.realm, run.run_id)
      if (!paged.has(key) && state.heads.get(key) === before.get(key)) continue
      open.set(key, [...(open.get(key) ?? []), client])
    }
    for (const [key, watchers] of open) {
      const run = await runOf(watchers[0].watch.run)
      if (paged.has(key)) {
        const seq = run.view?.summary?.seq
        // The head that was just read is the one to compare against next; the
        // retained row's older seq must not put it back.
        if (seq !== undefined) state.heads.set(key, seq)
        if (seq === before.get(key)) continue
      }
      for (const client of watchers) client.peer.send({ type: 'run', run })
    }
  }

  async function tick() {
    if (state.busy || clients.size === 0) return
    state.busy = true
    try {
      const before = state.heads
      const { changed: moved, paged } = await readFleet()
      state.heads = headsOf(state.fleet)
      if (moved) for (const client of clients) client.peer.send({ type: 'fleet', fleet: state.fleet })
      await pushRuns(before, paged)
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
    const client = { peer, watch: { view: 'none', run: null, realm: null } }
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
      const more = moreOf(message)
      if (more !== null) {
        morePage(more.realm).catch(report)
        return
      }
      const watch = watchOf(message)
      if (watch === null) return
      client.watch = watch
      snapshot(client).catch(report)
    }
  }

  return { locate, attach, tick }
}
