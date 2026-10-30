// Live refresh (design D5). While a client is connected, the host reads the
// fleet every few seconds. Every open run is re-read by `inspect` on every
// poll, whether the listing holds it or not: the listing is a page, and a
// page never says a run was dropped. A fresh summary replaces the run's row
// wherever the fleet holds it. The fleet is pushed whole when it changed: it
// is one readout over every hearth, and the panel-list count on every page
// reads every hearth. The overview's own readouts (the queue, Muninn's record
// and the last day's figures) go only to a client on the Realms view. A
// client that connects or changes what it watches gets a full snapshot.
// Nothing here touches a run: closing the panel only stops reading.
import { realpath } from 'node:fs/promises'
import { isAbsolute, resolve } from 'node:path'

import { REALM_BOUND, ReadoutRefused } from './readouts.js'

const VIEWS = ['none', 'realms', 'realm', 'analytics']

const keyOf = (realm, runId) => `${realm}\u0000${runId}`

/** Whether a hearth's listing is the whole hearth: no later page, no run behind, no refusal. */
const whole = (hearth) => hearth.detail === null && (hearth.next === null || hearth.next === undefined) && hearth.behind === 0

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

/** The fleet's verdict cell, in display columns, as `brokkr-view` clamps it. */
const VERDICT_COLUMNS = 14

/**
 * The standing a fresh inspect summary states, by the status and phase
 * `brokkr-view` derives it from; `null` for a status this console does not
 * know, so the cached row is kept rather than guessed at.
 */
function standingOf(summary) {
  if (summary === null) return null
  if (summary.status === 'running') return 'running'
  if (summary.status === 'awaiting_operator') return 'parked'
  if (summary.status === 'completed') return 'shipped'
  if (summary.status === 'stopped') return summary.phase === 'stop' ? 'stopped' : 'operator_stopped'
  return null
}

/** A rule id as the fleet's verdict cell writes it: the phase prefix dropped, then clamped to fourteen columns. */
function abbreviated(rule, from) {
  const [head, ...rest] = rule.split('-')
  const short = rest.length > 0 && from !== null && from.startsWith(head.toLowerCase()) ? rest.join('-') : rule
  return short.length <= VERDICT_COLUMNS ? short : `${short.slice(0, VERDICT_COLUMNS - 1)}…`
}

/**
 * The verdict a fresh inspect summary and its ruling state, as `brokkr-view`'s
 * `Facts::standing` and `verdict_of` derive it. `null` for a status this
 * console does not know, so the cached verdict is kept rather than guessed at.
 */
function verdictOf(summary, ruling, prior = null) {
  const standing = standingOf(summary)
  if (standing === null) return null
  const routed = ruling === null || ruling === undefined ? null : ruling.next !== ''
  const named = standing === 'shipped' || standing === 'stopped' || (standing === 'parked' && routed === false)
  const rule = named ? ruling?.rule_id ?? null : null
  const text = standing === 'running' ? ''
    : standing === 'operator_stopped' ? 'by operator'
      : rule === null ? '—' : abbreviated(rule, ruling?.from ?? null)
  return { standing, rule, residual: prior?.residual ?? null, reason: summary.park_reason ?? null, text }
}

/** The row a fresh inspect summary states over the row the listing gave: its head, phase, verdict and park reason follow the read. */
function rowOf(summary, ruling, row) {
  const verdict = verdictOf(summary, ruling, row.verdict)
  return verdict === null ? row : { ...row, seq: summary.seq, phase: summary.phase, verdict }
}

/**
 * A row for an open run a partial listing omits, from the one read that
 * states it: its inspect summary, over the row the same journal held last
 * time when there was one, so its title and start time survive the page.
 */
function rowFor(ref, summary, ruling, held = null) {
  const feature = summary.feature ?? held?.feature ?? ''
  return {
    run_id: ref.run_id,
    title: held?.title ?? (feature.split('\n')[0] || ref.run_id),
    feature,
    phase: summary.phase,
    seq: summary.seq,
    created_at: held?.created_at ?? '',
    last_recorded_at: held?.last_recorded_at ?? null,
    detail: held?.detail ?? null,
    acknowledgement: held?.acknowledgement ?? null,
    verdict: verdictOf(summary, ruling, held?.verdict),
  }
}

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
  const state = { fleet: null, prior: null, extras: new Map(), fleetText: null, oneMap: false, day: null, overviewText: null, timer: null, busy: false }

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
    const onScreen = watched()
    for (const [key, extra] of [...state.extras]) {
      const hearth = fleet.hearths.find((each) => each.realm === extra.realm && each.journal === extra.journal)
      if (!onScreen.has(extra.realm) || hearth === undefined || hearth.next !== extra.from) {
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
    const text = JSON.stringify(fleet)
    const changed = text !== state.fleetText
    Object.assign(state, { prior: state.fleet, fleet, fleetText: text })
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
   * Write a fresh inspect summary over the row wherever the fleet holds it;
   * a partial listing that omits the open run gets a row from that same read.
   * An older read never replaces a newer head, and a whole listing that does
   * not hold the run says the run is gone, not that a row is invented.
   */
  function holdRow(target, summary, ruling) {
    const hearth = state.fleet?.hearths.find((each) => each.realm === target.realm && (target.journal === null || each.journal === target.journal))
    if (hearth === undefined || hearth.detail !== null) return false
    const index = hearth.runs.findIndex((each) => each.run_id === target.run_id)
    if (index === -1) {
      if (whole(hearth)) return false
      const before = state.prior?.hearths.find((each) => each.realm === target.realm && each.journal === target.journal)
      const held = before?.runs.find((each) => each.run_id === target.run_id) ?? null
      const fresh = rowFor(target, summary, ruling, held)
      if (fresh.verdict === null) return false
      hearth.runs.push(fresh)
      state.fleetText = JSON.stringify(state.fleet)
      return 'added'
    }
    const held = hearth.runs[index]
    if (held.seq !== null && summary.seq < held.seq) return false
    const fresh = rowOf(summary, ruling, held)
    if (JSON.stringify(fresh) === JSON.stringify(held)) return false
    hearth.runs[index] = fresh
    for (const extra of state.extras.values()) {
      if (extra.realm !== target.realm || extra.journal !== target.journal) continue
      const at = extra.runs.findIndex((each) => each.run_id === target.run_id)
      if (at !== -1) extra.runs[at] = fresh
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
        run = read.run
        if (read.target !== null && run.view?.summary !== null && run.view?.summary !== undefined) {
          holdRow(read.target, run.view.summary, run.view.ruling)
        }
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
   * Read every open run once this poll, for the clients that have it open,
   * wherever the fleet holds it: first page, retained page or neither. The
   * fresh summary replaces the held row, and a run message goes only when the
   * read differs from the last one this client was sent. A message is fenced
   * against the watch that is current when the read lands, so a read that
   * started for one run cannot repaint or reach another.
   */
  async function pushRuns(before) {
    const open = new Map()
    for (const client of clients) {
      const run = client.watch.run
      if (run === null) continue
      const key = keyOf(run.realm, run.run_id)
      open.set(key, [...(open.get(key) ?? []), client])
    }
    const updates = []
    let patched = false
    for (const [key, watchers] of open) {
      const ref = { ...watchers[0].watch.run }
      const read = await runOf(ref)
      const summary = read.run.view?.summary ?? null
      const held = read.target !== null && summary !== null ? holdRow(read.target, summary, read.run.view.ruling) : false
      if (held !== false) patched = true
      const text = JSON.stringify(read.run)
      // A refusal has no summary to compare; it is sent again when the
      // listing's head for this run moved, so a failed read still follows it.
      // A run put back in the fleet after its page moved is sent once even
      // when the read itself is unchanged.
      const seq = read.target === null ? undefined
        : state.fleet?.hearths.find((each) => each.realm === ref.realm && each.journal === read.target.journal)?.runs.find((each) => each.run_id === ref.run_id)?.seq
      const refusalMoved = read.run.refused !== undefined && before.get(key) !== seq
      const reach = watchers.filter((client) => client.runText.get(key) !== text || refusalMoved || held === 'added')
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
