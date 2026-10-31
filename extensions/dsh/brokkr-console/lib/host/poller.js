// Live refresh (design D5). While a client is connected, the host reads the
// fleet every few seconds. Every open run is re-read by `inspect` on every
// poll, and so is every running run the latest first page shows, whether a
// retained page holds it or not: a page never says a run was dropped. A run's
// row is read whole from Brokkr's one derivation (`runs --run`), never
// derived here, and it is fenced by the journal it was read from: a result
// from a journal the realm no longer names paints nothing. The fleet is
// pushed whole when it changed: it is one readout over every hearth, and the
// panel-list count on every page reads every hearth. The overview's own
// readouts (the queue, Muninn's record and the last day's figures) go only to
// a client on the Realms view. A client that connects or changes what it
// watches gets a full snapshot. Nothing here touches a run: closing the panel
// only stops reading.
import { realpath } from 'node:fs/promises'
import { isAbsolute, resolve } from 'node:path'

import { REALM_BOUND, ReadoutRefused } from './readouts.js'

const VIEWS = ['none', 'realms', 'realm', 'analytics']

const keyOf = (realm, runId) => `${realm}\u0000${runId}`

/**
 * Whether a hearth's listing is the whole hearth: no later page, no run
 * behind, no retained page, no refusal. A page kept from an older read is
 * still one page, so its last page states no whole fleet (M5).
 */
const whole = (hearth) => hearth.retained !== true && hearth.detail === null && (hearth.next === null || hearth.next === undefined) && hearth.behind === 0

/** A fault in the console itself, which no readout explains: DSH's log shows it. */
const report = (error) => console.error('[brokkr-console]', error)

/** Each listed run's head, by realm and run id, from the fleet a poll is about to replace. */
function headsOf(fleet) {
  const heads = new Map()
  for (const hearth of fleet?.hearths ?? []) {
    for (const row of hearth.runs) heads.set(keyOf(hearth.realm, row.run_id), row.seq)
  }
  return heads
}

/** The key a retained page is held by: the realm and the journal it was read from. */
const pageKey = (realm, journal) => `${realm}\u0000${journal}`

/** Whether a fresh inspect summary shows the held row moved: its head, phase or standing. */
const moved = (row, summary) => row.seq !== summary.seq || row.phase !== summary.phase || row.verdict.standing !== summary.standing

/** A watch message, checked; `null` for one the host does not read. */
function watchOf(message) {
  if (message === null || typeof message !== 'object' || message.type !== 'watch') return null
  if (!VIEWS.includes(message.view)) return null
  const run = message.run
  const named = message.realm
  const realm = typeof named === 'string' && named.length <= REALM_BOUND ? named : typeof run?.realm === 'string' ? run.realm : null
  if (run === null || run === undefined) return { view: message.view, run: null, realm }
  if (typeof run.realm !== 'string' || typeof run.run_id !== 'string') return null
  return { view: message.view, run: { realm: run.realm, run_id: run.run_id }, realm }
}

/** A client asking for one hearth's next page, checked; `null` for one the host does not read. */
function moreOf(message) {
  if (message === null || typeof message !== 'object' || message.type !== 'more') return null
  return typeof message.realm === 'string' && message.realm.length <= REALM_BOUND ? { realm: message.realm } : null
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
  const state = { fleet: null, extras: new Map(), repin: new Set(), fleetText: null, oneMap: false, shown: [], day: null, overviewText: null, timer: null, busy: false }

  /** Whether the journal `target` was read from is still the one its realm names (M4). */
  function current(target) {
    if (target === null) return false
    const hearth = state.fleet?.hearths.find((each) => each.realm === target.realm)
    return hearth !== undefined && hearth.journal === target.journal
  }

  /**
   * A run's journal from the latest fleet, or `null` when the listing is
   * whole and does not hold it. A page that says more or behind cannot show
   * a run was dropped: the run is read through the journal its realm names.
   */
  function locate(realm, runId) {
    const hearth = state.fleet?.hearths.find((each) => each.realm === realm)
    if (hearth === undefined) return null
    if (whole(hearth) && !hearth.runs.some((row) => row.run_id === runId)) return null
    // The journal owns this realm's pages even when a one-hearth world keeps
    // part 1's argv without `--db`; the two facts are kept apart.
    return { realm, run_id: runId, journal: hearth.journal, db: state.oneMap ? null : hearth.journal }
  }

  /** The realms whose hearths a client has on screen: what a refresh keeps paged into. */
  function watched() {
    const realms = new Set()
    for (const client of clients) if (client.watch.realm !== null) realms.add(client.watch.realm)
    return realms
  }

  /**
   * Each hearth's first page; a hearth Brokkr cannot read keeps its refusal.
   * A one-hearth world is read without `--db` (part 1's hint rule), but a
   * map of one realm still names that realm's journal, so retained pages are
   * owned by the journal the map resolved.
   */
  async function readHearths(hearths) {
    if (hearths === null || hearths.length === 0) return { hearths: [await brokkr.fleetPage(null)] }
    const one = hearths.length === 1
    const each = async (hearth) => {
      const read = await settled(() => brokkr.fleetPage(one ? null : hearth.journal))
      return read.refused === undefined
        ? { ...read.value, realm: hearth.realm, journal: hearth.journal, detail: null }
        : { realm: hearth.realm, journal: hearth.journal, detail: read.refused, runs: [], next: null, behind: 0 }
    }
    return { hearths: await Promise.all(hearths.map(each)) }
  }

  /**
   * A partial listing that omits an open run cannot say it was dropped: its
   * row is carried from the fleet this poll is about to replace, under the
   * same journal. A whole listing does say it is gone, and a changed journal
   * owns none of the pages read from the old one.
   */
  function pinOpen(fleet) {
    for (const client of clients) {
      const ref = client.watch.run
      if (ref === null) continue
      const hearth = fleet.hearths.find((each) => each.realm === ref.realm)
      if (hearth === undefined || hearth.detail !== null || whole(hearth)) continue
      if (hearth.runs.some((row) => row.run_id === ref.run_id)) continue
      const before = state.fleet?.hearths.find((each) => each.realm === ref.realm && each.journal === hearth.journal)
      const held = before?.runs.find((row) => row.run_id === ref.run_id)
      if (held !== undefined) hearth.runs.push(held)
    }
  }

  /**
   * Read the fleet and say whether it differs from the last one read. Every
   * poll reads each hearth's first page only; the pages an operator paged
   * into are kept for the hearth on screen, keyed by its realm and journal,
   * while the first page still ends where they began, and let go otherwise.
   * An open run is never taken from or closed by this listing: it is read by
   * `inspect` in the poll that follows.
   */
  async function readFleet() {
    const hearths = await hearthsOf(brokkr, workspace)
    state.oneMap = hearths !== null && hearths.length === 1
    const fleet = await readHearths(hearths)
    // The latest first page's running rows, before any retained page is
    // merged: each is inspected on every poll, whatever the held pages say.
    const shown = []
    for (const hearth of fleet.hearths) {
      if (hearth.detail !== null) continue
      for (const row of hearth.runs) {
        if (row.verdict.standing === 'running') shown.push({ realm: hearth.realm, run_id: row.run_id })
      }
    }
    state.shown = shown
    const onScreen = watched()
    const repin = new Set()
    for (const [key, extra] of [...state.extras]) {
      const hearth = fleet.hearths.find((each) => each.realm === extra.realm && each.journal === extra.journal)
      if (!onScreen.has(extra.realm) || hearth === undefined || hearth.next !== extra.from) {
        for (const row of extra.runs) repin.add(keyOf(extra.realm, row.run_id))
        state.extras.delete(key)
        continue
      }
      const listed = new Set(hearth.runs.map((row) => row.run_id))
      for (const row of extra.runs) {
        if (listed.has(row.run_id)) continue
        listed.add(row.run_id)
        hearth.runs.push(row)
      }
      hearth.next = extra.next
      hearth.retained = true
    }
    pinOpen(fleet)
    state.repin = repin
    const text = JSON.stringify(fleet)
    const changed = text !== state.fleetText
    Object.assign(state, { fleet, fleetText: text })
    return { changed }
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
    // A one-hearth world keeps part 1's no-`--db` argv; a map's journal still owns its pages.
    const read = await settled(() => brokkr.fleetPage(state.oneMap ? null : first.journal, cursor))
    // A poll may have replaced the fleet while this page was in flight: a
    // result read from a journal the realm no longer names is dropped, and
    // nothing is painted from it (M4).
    const hearth = state.fleet?.hearths.find((each) => each.realm === realm)
    if (hearth === undefined || hearth.journal !== first.journal) return
    if (read.refused !== undefined) {
      for (const client of clients) client.peer.send({ type: 'error', message: read.refused })
      return
    }
    if (hearth.next !== cursor) return
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
    hearth.retained = true
    const key = pageKey(hearth.realm, hearth.journal)
    const kept = state.extras.get(key)
    const runs = [...(kept?.runs ?? [])]
    const held = new Set(runs.map((row) => row.run_id))
    for (const row of read.value.runs) {
      if (held.has(row.run_id)) continue
      held.add(row.run_id)
      runs.push(row)
    }
    state.extras.set(key, { realm: hearth.realm, journal: hearth.journal, from: kept?.from ?? cursor, runs, next: read.value.next })
    state.fleetText = JSON.stringify(state.fleet)
    for (const client of clients) client.peer.send({ type: 'fleet', fleet: state.fleet })
  }

  /**
   * An open run's view and the journal identity it was read from; a refused
   * read's own words and the command that reads it beside `view: null`; or
   * `listed: false` when the whole listing no longer holds the run, which is
   * a fact about the run rather than a read still in flight. The target is
   * captured before the await, so a watch that moves while the read is in
   * flight cannot repoint this result at another run.
   */
  async function runOf(ref) {
    const target = locate(ref.realm, ref.run_id)
    if (target === null) return { target: null, run: { ...ref, view: null, listed: false } }
    const read = await settled(() => brokkr.inspect(target))
    const run = read.refused === undefined
      ? { ...ref, view: read.value ?? null }
      : { ...ref, view: null, refused: read.refused, hint: brokkr.inspectHint(target) }
    return { target, run }
  }

  /**
   * The served row for `target` when a fresh summary shows the held row
   * moved, or when a partial listing left the run out. Brokkr's one
   * derivation is read whole; this host derives none of its standing,
   * verdict or text (M1-M3). A read from a journal the realm no longer
   * names is dropped (M4).
   */
  async function syncRow(target, summary) {
    const hearth = state.fleet?.hearths.find((each) => each.realm === target.realm && each.journal === target.journal)
    if (hearth === undefined || hearth.detail !== null) return false
    const held = hearth.runs.find((each) => each.run_id === target.run_id) ?? null
    if (held !== null && !moved(held, summary)) return false
    if (held !== null && held.seq !== null && summary.seq < held.seq) return false
    const read = await settled(() => brokkr.runRow(target))
    if (read.refused !== undefined || !current(target) || read.value.run_id !== target.run_id) return false
    return holdRow(target, read.value)
  }

  /**
   * Replace the fleet's row for `target` whole with `row`, the one Brokkr
   * served, adding it where a partial listing left the run out. A whole
   * listing that omits the run still says the run is gone; a retained page
   * never does (M5). An older read never replaces a newer head.
   */
  function holdRow(target, row) {
    const hearth = state.fleet?.hearths.find((each) => each.realm === target.realm && each.journal === target.journal)
    if (hearth === undefined || hearth.detail !== null) return false
    const index = hearth.runs.findIndex((each) => each.run_id === target.run_id)
    if (index === -1) {
      if (whole(hearth)) return false
      hearth.runs.push(row)
      state.fleetText = JSON.stringify(state.fleet)
      return 'added'
    }
    const held = hearth.runs[index]
    if (held.seq !== null && row.seq !== null && row.seq < held.seq) return false
    if (JSON.stringify(row) === JSON.stringify(held)) return false
    hearth.runs[index] = row
    for (const extra of state.extras.values()) {
      if (extra.realm !== target.realm || extra.journal !== target.journal) continue
      const at = extra.runs.findIndex((each) => each.run_id === target.run_id)
      if (at !== -1) extra.runs[at] = row
    }
    state.fleetText = JSON.stringify(state.fleet)
    return 'patched'
  }

  async function overview(fleetMoved) {
    if (fleetMoved || state.day === null) state.day = await settled(() => brokkr.analytics('24h', null))
    return { queue: await settled(brokkr.queue), muninn: await settled(brokkr.muninn), day: state.day }
  }

  async function snapshot(client) {
    try {
      const moved = await readFleet()
      const { view, run: ref } = client.watch
      let run = null
      if (ref !== null) {
        const read = await runOf(ref)
        // A snapshot read from a journal the realm no longer names paints
        // nothing, not even the current fleet it was read beside (M4).
        if (read.target !== null && !current(read.target)) return
        run = read.run
        if (read.target !== null && run.view?.summary !== null && run.view?.summary !== undefined) await syncRow(read.target, run.view.summary)
        client.runText.set(keyOf(ref.realm, ref.run_id), JSON.stringify(run))
      }
      client.peer.send({
        type: 'snapshot',
        workspace,
        ask,
        fleet: state.fleet,
        overview: view === 'realms' ? await overview(moved.changed) : null,
        run,
      })
    } catch (error) {
      if (!(error instanceof ReadoutRefused)) throw error
      client.peer.send({ type: 'error', message: error.message })
    }
  }

  /**
   * Read every open run and every running run the latest first page shows
   * once this poll, wherever the fleet holds them: first page, retained
   * page or neither. A fresh summary opens the run's served row when it
   * moved, and a message goes only to the clients that have the run open.
   * A read from a journal the realm no longer names reaches no one (M4);
   * a retained page never proves a run is gone (M5).
   */
  async function pushRuns(before) {
    const reads = new Map()
    const want = (ref, watcher = null) => {
      const key = keyOf(ref.realm, ref.run_id)
      const held = reads.get(key)
      if (held === undefined) reads.set(key, { ref: { realm: ref.realm, run_id: ref.run_id }, watchers: watcher === null ? [] : [watcher] })
      else if (watcher !== null) held.watchers.push(watcher)
    }
    for (const client of clients) if (client.watch.run !== null) want(client.watch.run, client)
    for (const ref of state.shown ?? []) want(ref)
    const updates = []
    let patched = false
    for (const [key, { ref, watchers }] of reads) {
      const read = await runOf(ref)
      if (read.target !== null && !current(read.target)) continue
      const summary = read.run.view?.summary ?? null
      if (summary !== null && (await syncRow(read.target, summary)) !== false) patched = true
      if (watchers.length === 0) continue
      const text = JSON.stringify(read.run)
      // A refusal has no summary to compare; it is sent again when the
      // listing's head for this run moved, so a failed read still follows it.
      // A run put back in the fleet after its page moved is sent once even
      // when the read itself is unchanged.
      const seq = read.target === null ? undefined
        : state.fleet?.hearths.find((each) => each.realm === ref.realm && each.journal === read.target.journal)?.runs.find((each) => each.run_id === ref.run_id)?.seq
      const refusalMoved = read.run.refused !== undefined && before.get(key) !== seq
      const reach = watchers.filter((client) => client.runText.get(key) !== text || refusalMoved || state.repin.has(key))
      if (reach.length === 0) continue
      for (const client of reach) client.runText.set(key, text)
      updates.push([reach, read.run])
    }
    return { patched, updates }
  }

  /** Send `run` only to the watchers that still have it open, never to a client that has moved to another run. */
  function sendRun(watchers, run) {
    for (const client of watchers) {
      const watched = client.watch.run
      if (watched !== null && watched.realm === run.realm && watched.run_id === run.run_id) client.peer.send({ type: 'run', run })
    }
  }

  async function tick() {
    if (state.busy || clients.size === 0) return
    state.busy = true
    try {
      const before = headsOf(state.fleet)
      const { changed: moved } = await readFleet()
      const { patched, updates } = await pushRuns(before)
      if (moved || patched) for (const client of clients) client.peer.send({ type: 'fleet', fleet: state.fleet })
      for (const [watchers, run] of updates) sendRun(watchers, run)
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
    const client = { peer, watch: { view: 'none', run: null, realm: null }, runText: new Map() }
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
