// brokkr-console, browser half. Hand-written in the shape DSH's client
// bundles take (task 0.2's probe): running the script registers a lazy
// factory with window.__ModuleLoader__, and `require` resolves React from the
// shell's platform table. It paints what the host half pushes from Brokkr's
// readouts and decides nothing: a run is led by its title, a button exists
// only for a command Brokkr admits, and every reason, refusal and figure is
// Brokkr's own text (design D1). Layout and copy follow the step-1 boards.
window.__ModuleLoader__.load({
  id: 'brokkr-console',
  factory: (require) => {
    const module = { exports: {} }
    Object.defineProperty(module.exports, Symbol.toStringTag, { value: 'Module' })
    const React = require('react')
    const h = React.createElement
    const { useEffect, useState } = React

    const PANEL = 'brokkr-console'
    const BASE = '/brokkr-console'
    const DAY_MS = 24 * 60 * 60 * 1000
    const RETRY_CHECK_MS = 3000
    const RETRY_CHECKS = 10

    // ------------------------------------------------------------- the store

    /** One socket for the panel and its list entry; reconnects with a fresh snapshot. */
    const store = {
      state: { fleet: null, overview: null, run: null, error: null, live: false },
      listeners: new Set(),
      watch: { type: 'watch', view: 'none', run: null },
      socket: null,
      wait: 1000,
    }
    function emit(change) {
      store.state = { ...store.state, ...change }
      for (const listener of store.listeners) listener()
    }
    function connect() {
      const socket = new WebSocket(location.origin.replace(/^http/, 'ws') + BASE + '/ws')
      store.socket = socket
      socket.onopen = () => {
        store.wait = 1000
        socket.send(JSON.stringify(store.watch))
        emit({ live: true })
      }
      socket.onmessage = (event) => receive(JSON.parse(event.data))
      socket.onclose = () => {
        store.socket = null
        emit({ live: false })
        if (store.listeners.size > 0) setTimeout(connect, store.wait)
        store.wait = Math.min(store.wait * 2, 10000)
      }
    }
    function receive(message) {
      switch (message.type) {
        case 'snapshot':
          return emit({ fleet: message.fleet, overview: message.overview, run: message.run, error: null })
        case 'fleet':
          return emit({ fleet: message.fleet, error: null })
        case 'overview':
          return emit({ overview: message.overview })
        case 'run':
          return emit({ run: message.run })
        case 'error':
          return emit({ error: message.message })
      }
    }
    function watch(view, run) {
      store.watch = { type: 'watch', view, run }
      if (store.socket?.readyState === 1) store.socket.send(JSON.stringify(store.watch))
    }
    function useStore() {
      const [, redraw] = useState(0)
      useEffect(() => {
        const listener = () => redraw((tick) => tick + 1)
        store.listeners.add(listener)
        if (store.socket === null) connect()
        return () => {
          store.listeners.delete(listener)
          if (store.listeners.size === 0) store.socket?.close()
        }
      }, [])
      return store.state
    }

    // ------------------------------------------------------------- painting

    const needsYou = (row) => row.verdict.standing === 'parked' || row.verdict.standing === 'quarantined'
    const waiting = (fleet) => (fleet === null ? 0 : fleet.hearths.reduce((sum, hearth) => sum + hearth.runs.filter(needsYou).length, 0))
    const runningIn = (rows) => rows.filter((row) => row.verdict.standing === 'running').length
    const plural = (n, one, many) => `${n} ${n === 1 ? one : many}`
    const STANDING = {
      quarantined: ['quarantined', 'bk-bad'],
      parked: ['parked · waiting on you', 'bk-park'],
      running: ['running', 'bk-run'],
      shipped: ['shipped', 'bk-ship'],
      stopped: ['stopped by a rule', 'bk-stop'],
      operator_stopped: ['stopped by you', 'bk-stop'],
    }
    const chip = (text, tone) => h('span', { className: `bk-chip ${tone}` }, text)

    /** How long ago `iso` was, the way the boards write it. */
    function ago(iso) {
      const ms = Date.now() - Date.parse(iso)
      if (!Number.isFinite(ms) || ms < 60_000) return 'just now'
      const minutes = Math.floor(ms / 60_000)
      if (minutes < 60) return `${minutes}m`
      const hours = Math.floor(minutes / 60)
      return hours < 48 ? `${hours}h` : `${Math.floor(hours / 24)}d`
    }

    /**
     * Each run's label within one hearth: Brokkr's title, and where titles
     * read alike, as much more of the feature as tells them apart (up to 20
     * words), else the local time each started, in brackets.
     */
    function labelsOf(runs) {
      const labels = new Map(runs.map((row) => [row.run_id, row.title]))
      const alike = new Map()
      for (const row of runs) alike.set(row.title, [...(alike.get(row.title) ?? []), row])
      for (const rows of alike.values()) {
        if (rows.length < 2) continue
        const words = rows.map((row) => row.feature.split(/\s+/).filter(Boolean))
        const prefix = (index, k) => words[index].slice(0, k).join(' ') + (words[index].length > k ? '…' : '')
        const tells = (k) => new Set(rows.map((_, index) => prefix(index, k))).size === rows.length
        const k = Array.from({ length: 20 }, (_, n) => n + 1).find(tells)
        rows.forEach((row, index) => labels.set(row.run_id, k === undefined
          ? `${row.title} (${new Date(row.created_at).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })})`
          : prefix(index, k)))
      }
      return labels
    }

    /** Why a row needs the operator, in Brokkr's words: the refusal, or the verdict. */
    const shortReason = (row) => (row.verdict.standing === 'quarantined' ? row.detail ?? row.verdict.text : row.verdict.text)

    /** Each command's button, named for its effect, and what it does. */
    const COMMAND = {
      retry: (offered) => [`Retry ${offered.phase ?? 'the run'}`, `Retry runs ${offered.phase ?? 'the run'} again, fresh, on the current code.`],
      stop: () => ['Stop the run', 'Ends the run as stopped by you.'],
      resume: () => ['Resume', 'Picks the run up again where its record ends.'],
      conclude: () => ['Close it', 'Ends the run as stopped by you. Refused if it turns out something is still driving it.'],
    }
    const ACTION = {
      dismiss_finding: 'Dismiss the finding',
      talk_to_reviewer: 'Talk to the reviewer',
      hold: 'Hold',
      answer_here: 'Answer here',
    }
    const ADMISSION = { admissible: ['could start', 'bk-ship'], waiting: ['waiting', 'bk-park'], held: ['held', 'bk-stop'] }

    // ------------------------------------------------------------ the icon

    function Icon({ size, active }) {
      const count = waiting(useStore().fleet)
      const box = { position: 'relative', display: 'inline-block', width: size, height: size, boxSizing: 'border-box', border: '1.5px solid currentColor', borderRadius: 5, opacity: active ? 1 : 0.85 }
      const dot = { position: 'absolute', right: -7, top: -7, minWidth: 16, height: 16, borderRadius: 8, background: '#D98A2B', color: '#16181D', fontSize: 10, fontWeight: 600, display: 'flex', alignItems: 'center', justifyContent: 'center', padding: '0 3px', boxSizing: 'border-box' }
      return h('span', { style: box, 'aria-label': `Brokkr: ${plural(count, 'run', 'runs')} waiting on you` }, count > 0 ? h('span', { style: dot }, String(count)) : null)
    }

    // --------------------------------------------------------- the assistant

    /**
     * An assistant panel. Its connection to DSH's model is not built in this
     * slice, so it says so rather than offering a box nobody answers.
     */
    function Ask({ title, scope, children }) {
      return h('aside', { className: 'bk-aside', 'aria-label': title },
        h('div', { className: 'bk-aside-head' }, h('div', { className: 'bk-aside-title' }, title), h('div', { className: 'bk-note' }, scope)),
        h('div', { className: 'bk-aside-body' },
          h('div', { className: 'bk-coming' }, chip('coming later', 'bk-later'), 'Asking needs DSH’s model connected to Brokkr’s readouts, which this console does not do yet.')),
        children)
    }

    function Muninn({ muninn }) {
      if (muninn === undefined || muninn.value === null) return null
      if (muninn.refused !== undefined) return h('div', { className: 'bk-muninn' }, `Muninn’s record could not be read: ${muninn.refused}`)
      const last = muninn.value
      return h('div', { className: 'bk-muninn' },
        h('strong', null, 'Muninn’s last review'), ` · ${ago(last.recorded_at)} ago · flagged ${plural(last.parked, 'parked run', 'parked runs')} and ${plural(last.queued, 'item', 'items')} of work. `,
        h('details', null, h('summary', null, 'Read it'), last.summary))
    }

    // ------------------------------------------------------------- realms

    function RealmCard({ hearth, day, open }) {
      const needs = hearth.runs.filter(needsYou)
      const running = runningIn(hearth.runs)
      const shown = [...needs, ...hearth.runs.filter((row) => row.verdict.standing === 'running')].slice(0, 4)
      const labels = labelsOf(hearth.runs)
      const tone = needs.length > 0 ? chip(`${needs.length} need${needs.length === 1 ? 's' : ''} you`, 'bk-park') : chip(running > 0 ? 'all moving' : 'nothing running', running > 0 ? 'bk-run' : 'bk-stop')
      const counts = [`${running} running`]
      if (day !== null) counts.push(`shipped 24h: ${day.shipped}`, `stopped: ${day.stopped}`)
      return h('button', { type: 'button', className: `bk-cell bk-realm${needs.length > 0 ? ' needs' : ''}`, onClick: () => open(hearth.realm) },
        h('div', { className: 'bk-row' }, h('span', { className: 'bk-name' }, hearth.realm), hearth.detail === null ? tone : chip('quarantined', 'bk-bad')),
        hearth.detail === null ? null : h('div', { className: 'bk-error' }, hearth.detail),
        h('div', { className: 'bk-mini' }, shown.map((row) =>
          h('span', { key: row.run_id }, h('strong', null, labels.get(row.run_id)), ` · ${row.phase ?? 'not started'} · ${needsYou(row) ? shortReason(row) : 'running'} · ${ago(row.last_recorded_at ?? row.created_at)}`))),
        h('div', { className: 'bk-d' }, `${counts.join(' · ')} · `, h('span', { className: 'bk-link' }, 'open →')))
    }

    function Queue({ queue }) {
      if (queue === undefined) return null
      const body = queue.refused !== undefined
        ? h('div', { className: 'bk-error' }, queue.refused)
        : queue.value.filter((entry) => entry.admission !== null).length === 0
          ? h('div', { className: 'bk-note' }, 'Nothing is queued.')
          : h('div', { className: 'bk-queue' }, h('span', { className: 'bk-k' }, 'Run'), h('span', { className: 'bk-k' }, 'Bundle'), h('span', { className: 'bk-k' }, 'Brokkr says'),
            queue.value.filter((entry) => entry.admission !== null).flatMap((entry) => {
              const [word, tone] = ADMISSION[entry.admission.standing]
              return [
                h('span', { key: `r${entry.entry}` }, h('strong', null, entry.feature.split('\n')[0]), h('br'), h('span', { className: 'bk-d' }, `queued ${ago(entry.added_at)}`)),
                h('span', { key: `b${entry.entry}` }, entry.bundle),
                h('span', { key: `s${entry.entry}` }, chip(word, tone), ' ', entry.admission.says.join(' · ')),
              ]
            }))
      return h('section', { className: 'bk-cell bk-section', 'aria-label': 'Queue' },
        h('div', { className: 'bk-row' }, h('h2', { style: { margin: 0, fontSize: 15 } }, 'Queued runs'), h('span', { className: 'bk-note' }, 'what’s waiting to start, and why Brokkr would hold it back right now')),
        body,
        h('div', { className: 'bk-coming' }, chip('coming later', 'bk-later'), 'Starting queued runs from here comes with Brokkr’s dispatcher. Today, start them with ', h('code', null, 'brokkr run'), '.'))
    }

    function Realms({ state, open }) {
      const day = state.overview?.day?.value?.realms ?? []
      const hearths = state.fleet.hearths
      const dayOf = (hearth) => day.find((row) => row.realm === hearth.realm) ?? (hearths.length === 1 && day.length === 1 ? day[0] : null)
      return h('div', { className: 'bk-body' },
        h('main', { className: 'bk-main' },
          h('section', { className: 'bk-cards', 'aria-label': 'Realms' }, hearths.map((hearth) => h(RealmCard, { key: hearth.realm, hearth, day: dayOf(hearth), open }))),
          h(Queue, { queue: state.overview?.queue })),
        h(Ask, { title: 'Ask Brokkr', scope: 'Reads every realm. It can explain and suggest; you press the buttons.' }, h(Muninn, { muninn: state.overview?.muninn })))
    }

    // ------------------------------------------------------------ one realm

    function RunList({ hearth, selected, select }) {
      const [filter, setFilter] = useState('')
      const labels = labelsOf(hearth.runs)
      const shown = hearth.runs.filter((row) => `${labels.get(row.run_id)} ${row.feature}`.toLowerCase().includes(filter.toLowerCase()))
      const finished = (row) => !needsYou(row) && row.verdict.standing !== 'running' && Date.now() - Date.parse(row.last_recorded_at ?? row.created_at) < DAY_MS
      const groups = [['Needs you', needsYou], ['Running', (row) => row.verdict.standing === 'running'], ['Last 24 hours', finished]]
      return h('nav', { className: 'bk-runs', 'aria-label': 'Runs' },
        h('input', { className: 'bk-input', 'aria-label': 'Filter runs', placeholder: 'Filter by name…', value: filter, onChange: (event) => setFilter(event.target.value) }),
        groups.flatMap(([label, within]) => {
          const rows = shown.filter(within)
          return [h('div', { key: label, className: 'bk-grp' }, `${label} · ${rows.length}`), ...rows.map((row) =>
            h('button', { key: row.run_id, type: 'button', title: row.feature.slice(0, 400), className: `bk-q${row.run_id === selected ? ' on' : ''}`, onClick: () => select(row.run_id) },
              h('span', { className: 't' }, labels.get(row.run_id), chip(row.phase ?? STANDING[row.verdict.standing][0], STANDING[row.verdict.standing][1])),
              h('span', { className: 's' }, `${needsYou(row) ? shortReason(row) : STANDING[row.verdict.standing][0]} · ${ago(row.last_recorded_at ?? row.created_at)}`)))]
        }))
    }

    function Graph({ view }) {
      const returns = view.phases.filter((phase) => phase.returns.length > 0)
      return h('section', { className: 'bk-cell bk-section', 'aria-label': 'Run graph' },
        h('span', { className: 'bk-k' }, 'Steps'),
        h('div', { className: 'bk-graph' }, view.path.map((visit, index) =>
          h('div', { key: index, className: `bk-node ${visit.ruled}` }, h('strong', null, visit.phase),
            [visit.model, visit.duration, visit.rule].filter((part) => part !== null).map((part) => h('span', { key: part, className: 'e' }, part))))),
        returns.map((phase) => h('div', { key: phase.name, className: 'bk-note' }, `sent back to ${phase.name} by ${phase.returns.join(', ')}`)))
    }

    /** The timeline text with the badge's words not said twice beside it. */
    function besideBadge({ badge, text }) {
      if (badge === null || !text.startsWith(badge)) return text
      return text.slice(badge.length).trim()
    }

    function Timeline({ view }) {
      const rows = view.journal.filter((row) => row.in_trail).reverse().slice(0, 50)
      return h('section', { className: 'bk-cell bk-section', 'aria-label': 'Timeline' },
        h('div', { className: 'bk-row' }, h('span', { className: 'bk-k' }, 'Timeline'), h('span', { className: 'bk-note' }, 'newest first')),
        h('div', { className: 'bk-timeline' }, rows.flatMap((row) => [
          h('span', { key: `t${row.seq}`, className: 'bk-note' }, ago(row.recorded_at)),
          h('span', { key: `w${row.seq}` }, row.what.badge === null ? null : chip(row.what.badge, 'bk-stop'), ' ', besideBadge(row.what))])))
    }

    /** What happened to a pressed command: done, not sent, refused or checking. */
    function Outcome({ result, label, see }) {
      const resumed = result.stage === 'resume'
      const texts = {
        sending: () => ['sending…', 'Asking Brokkr.'],
        done: () => ['done', resumed ? 'Brokkr took it, and the run is being driven again. This card follows the run until it next needs you.' : 'Brokkr took it. This card follows the run until it next needs you.'],
        refused: () => ['refused', h('span', null, resumed ? 'The retry was recorded; resume was refused. ' : '', 'Brokkr said: ', h('em', null, `“${result.text}”`))],
        checking: () => ['checking…', 'The connection dropped before Brokkr answered. Checking the run’s record; it won’t be sent twice.'],
        not_sent: () => ['not sent', {
          head_moved: 'the run changed after you opened it. Nothing was done. Here is what it looks like now.',
          resume_unconfigured: 'Nothing was sent: the console names no bundle to resume runs under. Set BROKKR_CONSOLE_RESUME_BUNDLE or BROKKR_CONSOLE_RESUME_RECIPE where DSH starts.',
          unconfirmed: 'Brokkr’s record shows no trace of it, so nothing was done, and nothing was sent twice.',
          unknown_run: 'Brokkr no longer lists this run. Nothing was done.',
          unreadable: `Nothing was done: ${result.text}`,
          unstarted: `${resumed ? 'The retry was recorded, but resume did not start' : 'Nothing was done'}: ${result.text}`,
        }[result.why]],
      }
      const [word, text] = texts[result.outcome]()
      return h('div', { className: `bk-outcome ${result.outcome}` },
        h('div', { className: 'bk-row' }, h('strong', null, label), chip(word, result.outcome === 'done' ? 'bk-ship' : result.outcome === 'refused' ? 'bk-bad' : 'bk-park')),
        h('div', null, text),
        result.outcome !== 'not_sent' || result.view === undefined ? null : h('button', { type: 'button', className: 'bk-btn', onClick: () => see(result.view) }, 'See current state'))
    }

    async function post(body) {
      try {
        const response = await fetch(`${BASE}/api/command`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) })
        const answer = await response.json()
        return response.ok ? answer : { outcome: 'not_sent', why: 'unreadable', text: answer.error }
      } catch {
        return { outcome: 'checking' }
      }
    }

    function Clarify({ view }) {
      return [
        h('div', { key: 'q', className: 'bk-quote' }, h('div', { className: 'bk-k', style: { color: '#8A3F06' } }, 'The clarifier asks'),
          view.clarify.questions.map((question, index) => h('div', { key: index }, `${index + 1}. ${question}`))),
        h('div', { key: 'a', className: 'bk-note' }, 'To answer: write your answers into the spec, then retry clarify. The next round reads the spec.',
          view.worktree === null ? null : h('div', null, 'This run’s worktree: ', h('code', null, view.worktree))),
      ]
    }

    function Decision({ hearth, row, view, see }) {
      const [reason, setReason] = useState('')
      const [missing, setMissing] = useState(false)
      const [typedAt, setTypedAt] = useState(null)
      const [pressed, setPressed] = useState(null)
      const seq = view.summary.seq
      const admitted = view.commands.filter((offered) => offered.admission === 'admitted')
      async function press(offered) {
        if (reason.trim() === '') return setMissing(true)
        const label = COMMAND[offered.command](offered)[0]
        const body = { command_id: crypto.randomUUID(), realm: hearth.realm, run_id: row.run_id, command: offered.command, reason, expected_seq: seq }
        setPressed({ label, result: { outcome: 'sending' } })
        let result = await post(body)
        for (let check = 0; result.outcome === 'checking' && check < RETRY_CHECKS; check += 1) {
          setPressed({ label, result })
          await new Promise((wake) => setTimeout(wake, RETRY_CHECK_MS))
          result = await post(body)
        }
        setPressed({ label, result })
      }
      return h('section', { className: 'bk-cell bk-decision', 'aria-label': 'Decision' },
        h('div', { className: 'bk-row' }, h('span', { className: 'bk-k' }, row.verdict.standing === 'parked' ? 'Waiting on you' : 'What Brokkr would take now'),
          row.verdict.standing !== 'parked' || row.verdict.rule === null ? null : h('span', { className: 'bk-note' }, 'parked at ', h('strong', null, row.phase), ' by rule ', h('strong', null, row.verdict.rule))),
        view.summary.park_reason === null ? null : h('div', { className: 'bk-quote' }, `“${view.summary.park_reason}”`, h('div', { className: 'bk-d', style: { color: '#8A3F06' } }, 'Brokkr’s park reason, word for word')),
        view.clarify === null ? null : h(Clarify, { view }),
        h('label', { className: 'bk-note', htmlFor: 'bk-reason' }, 'Your reason (kept in the run’s record)'),
        h('textarea', { id: 'bk-reason', className: `bk-reason${missing ? ' missing' : ''}`, value: reason, placeholder: missing ? 'Say why first: a reason is required.' : '', onChange: (event) => {
          setReason(event.target.value)
          setMissing(false)
          if (typedAt === null) setTypedAt(seq)
        } }),
        typedAt !== null && typedAt !== seq ? h('div', { className: 'bk-note' }, 'The run changed while you were here; the buttons show what Brokkr takes now.') : null,
        h('div', { className: 'bk-row' }, admitted.map((offered, index) => {
          const [label] = COMMAND[offered.command](offered)
          return h('button', { key: offered.command, type: 'button', className: `bk-btn${index === 0 ? ' primary' : ''}`, onClick: () => press(offered) }, label)
        }), admitted.length === 0 ? null : h('span', { className: 'bk-d' }, COMMAND[admitted[0].command](admitted[0])[1])),
        pressed === null ? null : h(Outcome, { ...pressed, see }),
        view.unavailable.length === 0 ? null : h('div', { className: 'bk-unavailable' }, h('span', { className: 'bk-k' }, 'Not available yet'),
          view.unavailable.map((entry) => h('div', { key: entry.action, className: 'bk-row' }, h('button', { type: 'button', className: 'bk-btn', disabled: true }, ACTION[entry.action]), h('span', { className: 'bk-note' }, entry.reason)))),
        h('details', { className: 'bk-audit' }, h('summary', null, 'Details for audit'),
          h('div', null, `run ${row.run_id} · head seq ${seq}${view.ruling === null ? '' : ` · ruled by ${view.ruling.rule_id} from ${view.ruling.from}`}${view.clarify === null ? '' : ` · questions read at seq ${view.clarify.seq}`} · commands go through brokkr operator retry / stop, brokkr resume and brokkr conclude`)))
    }

    function Run({ hearth, row, run, see }) {
      const [word, tone] = STANDING[row.verdict.standing]
      const view = run?.realm === hearth.realm && run?.run_id === row.run_id ? run.view : null
      const actionable = (view) => view.summary !== null &&
        (view.commands.some((offered) => offered.admission === 'admitted') || view.unavailable.length > 0 || view.clarify !== null)
      return h('main', { className: 'bk-main' },
        h('div', { className: 'bk-row' }, h('h1', { style: { margin: 0, fontSize: 22 }, title: row.feature.slice(0, 400) }, labelsOf(hearth.runs).get(row.run_id)), chip(word, tone), h('span', { className: 'bk-note' }, `started ${ago(row.created_at)} ago`)),
        row.detail === null ? null : h('div', { className: 'bk-error' }, row.detail),
        view === null ? h('div', { className: 'bk-note' }, 'Reading the run…') : [
          h(Graph, { key: 'g', view }),
          actionable(view) ? h(Decision, { key: `d${row.run_id}`, hearth, row, view, see }) : null,
          h(Timeline, { key: 't', view }),
        ])
    }

    function Realm({ state, realm, runId, select, back, see }) {
      const hearth = state.fleet.hearths.find((each) => each.realm === realm)
      if (hearth === undefined) return h('main', { className: 'bk-main' }, h('div', { className: 'bk-note' }, `Brokkr no longer lists ${realm}.`))
      const row = hearth.runs.find((each) => each.run_id === runId) ?? null
      const needs = hearth.runs.filter(needsYou).length
      return h(React.Fragment, null,
        h('div', { className: 'bk-bar' }, h('button', { type: 'button', className: 'bk-link', onClick: back }, '← All realms'), h('span', { style: { fontSize: 17, fontWeight: 600 } }, hearth.realm),
          h('span', { className: 'bk-note' }, `${runningIn(hearth.runs)} running`), needs > 0 ? chip(`${needs} need${needs === 1 ? 's' : ''} you`, 'bk-park') : null),
        h('div', { className: 'bk-body' }, h(RunList, { hearth, selected: runId, select }),
          row === null ? h('main', { className: 'bk-main' }, h('div', { className: 'bk-note' }, 'Pick a run on the left.')) : h(Run, { hearth, row, run: state.run, see }),
          h(Ask, { title: 'Ask about this run', scope: 'Reads this run’s record and sessions. It can suggest one of the buttons; it can’t press them.' })))
    }

    // ------------------------------------------------------------ analytics

    function PerDay({ days }) {
      const top = Math.max(1, ...days.flatMap((day) => [day.shipped, day.stopped]))
      const step = 474 / Math.max(days.length, 1)
      const y = (n) => 140 - (n / top) * 120
      return h('svg', { viewBox: '0 0 520 170', width: '100%', height: 170, role: 'img', 'aria-label': 'Runs shipped and stopped per day' },
        h('line', { x1: 36, y1: 140, x2: 510, y2: 140, stroke: '#C9C6BC' }), h('text', { className: 'bk-ax', x: 30, y: 24, textAnchor: 'end' }, String(top)),
        days.map((day, index) => h('g', { key: day.date },
          h('rect', { x: 40 + index * step, y: y(day.shipped), width: step * 0.4, height: 140 - y(day.shipped), rx: 3, fill: '#1F5FAD' }, h('title', null, `${day.date}: ${day.shipped} shipped`)),
          h('rect', { x: 42 + index * step + step * 0.4, y: y(day.stopped), width: step * 0.25, height: 140 - y(day.stopped), rx: 3, fill: '#8E8B82' }, h('title', null, `${day.date}: ${day.stopped} stopped`)),
          h('text', { className: 'bk-ax', x: 40 + index * step + step * 0.35, y: 160, textAnchor: 'middle' }, day.date.slice(5)))))
    }

    function Waits({ steps }) {
      if (steps.length === 0) return h('div', { className: 'bk-note' }, 'Nothing waited on you in this range.')
      const top = Math.max(1, ...steps.map((step) => step.waiting_millis))
      return h('svg', { viewBox: `0 0 520 ${Math.max(34 * steps.length, 34)}`, width: '100%', role: 'img', 'aria-label': 'Hours waiting on you, by step' },
        steps.map((step, index) => h('g', { key: index },
          h('text', { className: 'bk-ax', x: 106, y: 26 + index * 34, textAnchor: 'end' }, step.phase ?? 'no step'),
          h('rect', { x: 116, y: 14 + index * 34, width: Math.max(2, (step.waiting_millis / top) * 300), height: 18, rx: 3, fill: '#1F5FAD' }),
          h('text', { className: 'bk-ax', x: 122 + (step.waiting_millis / top) * 300, y: 27 + index * 34 }, step.text))))
    }

    const MODEL_TONES = ['#1F5FAD', '#5A8BC9', '#A9C3E3', '#D1CEC4']
    const figure = (title, note, ...body) => h('figure', { className: 'bk-cell bk-figure' }, h('figcaption', { className: 'bk-row' }, h('strong', null, title), h('span', { className: 'bk-d' }, note)), ...body)
    const tile = (key, value, note) => h('div', { className: 'bk-cell bk-tile' }, h('span', { className: 'bk-k' }, key), h('span', { className: 'bk-num' }, value), h('span', { className: 'bk-d' }, note))

    function Figures({ data }) {
      const { headline, rework } = data
      return [
        h('section', { key: 'tiles', className: 'bk-tiles', 'aria-label': 'Headline numbers' },
          tile(`Shipped · ${data.range}`, String(headline.shipped.count), headline.shipped.change_text),
          tile('Median start → end', headline.median.text, 'includes time parked'),
          tile('Waiting on you', headline.waiting.text, headline.waiting.share_text),
          tile('Stopped by a rule', String(headline.stopped_by_rule.count), headline.stopped_by_rule.text),
          tile('Reported cost', headline.cost.text, headline.cost.note ?? '')),
        h('section', { key: 'charts', className: 'bk-charts' },
          figure('Runs finished per day', 'shipped vs stopped · blue shipped, grey stopped', h(PerDay, { days: data.per_day })),
          figure('Where runs wait for you', 'hours parked, by step', h(Waits, { steps: data.waiting_by_step })),
          figure('Rework', 'how often work was sent back', h('div', { className: 'bk-tiles' },
            tile('implement visits per run', rework.implement_visits_text, ''), tile('sent back by review', `${rework.sent_back} of ${rework.runs}`, ''), tile('pass review on the 2nd try', rework.second_try_text, ''))),
          figure('Who did the work', 'share of step attempts, by model',
            h('div', { style: { display: 'flex', height: 22, borderRadius: 6, overflow: 'hidden', gap: 2 } }, data.models.map((model, index) =>
              h('span', { key: model.model, title: model.text, style: { width: `${model.percent}%`, background: MODEL_TONES[Math.min(index, 3)] } }))),
            h('div', { className: 'bk-row bk-d' }, data.models.map((model) => h('span', { key: model.model }, model.text))))),
      ]
    }

    function ByRealm({ data, open }) {
      return h('section', { className: 'bk-cell bk-section' }, h('div', { className: 'bk-row' }, h('strong', null, 'By realm'), h('span', { className: 'bk-d' }, 'click a row to open the realm')),
        h('div', { style: { overflowX: 'auto' } }, h('table', { className: 'bk-table' },
          h('thead', null, h('tr', null, ['Realm', 'Shipped', 'Stopped', 'Median start → end', 'Waiting on you', 'Reported cost'].map((name, index) => h('th', { key: name, scope: 'col', className: index > 0 ? 'n' : '' }, name)))),
          h('tbody', null, data.realms.map((row) => h('tr', { key: row.realm, onClick: () => open(row.realm), style: { cursor: 'pointer' } },
            h('td', null, h('strong', null, row.realm)), h('td', { className: 'n' }, String(row.shipped)), h('td', { className: 'n' }, String(row.stopped)),
            h('td', { className: 'n' }, row.median.text), h('td', { className: 'n' }, row.waiting_text), h('td', { className: 'n' }, row.cost_text)))))),
        data.quarantined.map((hearth) => h('div', { key: hearth.realm, className: 'bk-error' }, `${hearth.realm} is quarantined: ${hearth.refusal}`)))
    }

    function Analytics({ open }) {
      const [range, setRange] = useState('7d')
      const [realm, setRealm] = useState(null)
      const [realms, setRealms] = useState([])
      const [data, setData] = useState(null)
      const [error, setError] = useState(null)
      useEffect(() => {
        const query = new URLSearchParams({ range })
        if (realm !== null) query.set('realm', realm)
        fetch(`${BASE}/api/analytics?${query}`).then((response) => response.json().then((body) => {
          if (!response.ok) return setError(body.error)
          setError(null)
          setData(body)
          if (realm === null) setRealms(body.realms.map((row) => row.realm))
        })).catch(() => setError('the console could not be reached'))
      }, [range, realm])
      const choice = (on, text, pick) => h('button', { key: text, type: 'button', className: `bk-fchip${on ? ' on' : ''}`, onClick: pick }, text)
      return h('div', { className: 'bk-main', style: { maxWidth: 1200, margin: '0 auto', width: '100%' } },
        h('div', { className: 'bk-row' }, h('span', { className: 'bk-k' }, 'Range'), ['24h', '7d', '30d'].map((word) => choice(word === range, word, () => setRange(word))),
          h('span', { className: 'bk-k' }, 'Realm'), choice(realm === null, 'All', () => setRealm(null)), realms.map((name) => choice(name === realm, name, () => setRealm(name)))),
        error === null ? null : h('div', { className: 'bk-error' }, error),
        data === null ? null : [h(Figures, { key: 'f', data }), h(ByRealm, { key: 'r', data, open }),
          h('div', { key: 'n', className: 'bk-coming' }, chip('not recorded yet', 'bk-later'), `: ${data.not_recorded.join(' · ')}`)])
    }

    // ------------------------------------------------------------ the panel

    function Page() {
      const state = useStore()
      const [route, setRoute] = useState({ view: 'realms', realm: null, run: null })
      useEffect(() => {
        if (document.getElementById('bk-styles') === null) {
          const link = Object.assign(document.createElement('link'), { id: 'bk-styles', rel: 'stylesheet', href: `${BASE}/console.css` })
          document.head.appendChild(link)
        }
      }, [])
      useEffect(() => {
        watch(route.view, route.run === null ? null : { realm: route.realm, run_id: route.run })
        return () => watch('none', null)
      }, [route.view, route.realm, route.run])
      const open = (realm) => setRoute({ view: 'realm', realm, run: null })
      const select = (run) => setRoute({ ...route, run })
      const see = (view) => emit({ run: { realm: route.realm, run_id: route.run, view } })
      const count = waiting(state.fleet)
      const tab = (view, text) => h('button', { type: 'button', className: `bk-tab${route.view === view || (view === 'realms' && route.view === 'realm') ? ' on' : ''}`, onClick: () => setRoute({ view, realm: null, run: null }) }, text)
      const body = state.fleet === null
        ? h('div', { className: 'bk-main' }, state.error === null ? h('div', { className: 'bk-note' }, state.live ? 'Reading Brokkr…' : 'Connecting…') : null)
        : route.view === 'analytics' ? h(Analytics, { open })
          : route.view === 'realm' ? h(Realm, { state, realm: route.realm, runId: route.run, select, back: () => setRoute({ view: 'realms', realm: null, run: null }), see })
            : h(Realms, { state, open })
      return h('div', { className: 'bk', 'data-testid': 'brokkr-console' },
        h('nav', { className: 'bk-nav', 'aria-label': 'Brokkr views' }, h('span', { className: 'bk-brand' }, 'Brokkr'), tab('realms', 'Realms'), tab('analytics', 'Analytics'),
          state.fleet === null ? null : h('span', { className: 'bk-status' }, `${state.fleet.hearths.reduce((sum, hearth) => sum + runningIn(hearth.runs), 0)} running · `, h('strong', null, `${count} waiting on you`), state.live ? '' : ' · reconnecting')),
        state.error === null ? null : h('div', { className: 'bk-main' }, h('div', { className: 'bk-error' }, state.error)),
        body)
    }

    function apply(ctx) {
      ctx.slots.inject('main', () => ctx.slots.register({ name: 'main', key: PANEL }, Page))
      ctx.slots.inject('sidebar.panellist', () => ctx.slots.register({ name: 'sidebar.panellist', id: PANEL, order: 50, label: () => 'Brokkr' }, Icon))
    }

    module.exports.apply = apply
    module.exports.inject = ['slots']
    module.exports.name = 'brokkr-console'
    return module.exports
  },
})
