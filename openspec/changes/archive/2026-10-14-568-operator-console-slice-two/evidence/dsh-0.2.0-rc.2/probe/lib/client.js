// brokkr-slice2-probe, browser half. Hand-written in the CJS-factory shape
// DSH's client bundles take (slice one's measurement). It measures:
//   D6  a panel action that creates a Session, shows it, and prefills (or
//       sends) a prompt; an inline answer from the host half; transcript cards
//       for the probe's own tools through the keyed `tool.call.toolview` slot;
//   D7  opening a file in the right Sidebar (ctx.sidebarRight.openResource)
//       and opening the Files tab (ctx.sidebarRight.openTab('files')).
// Every step reports to the host half's /brokkr-slice2/report beacon.
window.__ModuleLoader__.load({
  id: 'brokkr-slice2-probe',
  factory: (require) => {
    const module = { exports: {} }
    Object.defineProperty(module.exports, Symbol.toStringTag, { value: 'Module' })
    const React = require('react')
    const h = React.createElement
    const { useState } = React

    const PANEL = 'brokkr-slice2'
    const BASE = '/brokkr-slice2'
    const CFG = (window.__BROKKR_SLICE2__ = window.__BROKKR_SLICE2__ || {})

    function report(event, detail) {
      return fetch(BASE + '/report', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ event, detail }),
      }).catch(() => {})
    }
    const describe = (e) => (e && (e.stack || e.message)) ? String(e.message || e) : String(e)
    const sleep = (ms) => new Promise((r) => setTimeout(r, ms))

    /** The session-file address grammar of dsh-util-workspace-path (sessionFileAddress), restated. */
    function fileAddress(sessionId, path) {
      const enc = (s) => encodeURIComponent(s).replace(/%3A/gi, ':')
      const normalized = path.replace(/\\/g, '/').replace(/^(?:\.\/)+/, '')
      return 'dsh-resource:' + '//file/session/' + enc(sessionId) + '/' + normalized.split('/').map(enc).join('/')
    }

    function apply(ctx) {
      report('client-apply', {
        services: ['sessions', 'workspaces', 'uiWorkspace', 'conversation', 'sidebarRight', 'layout', 'uiSession'].map((n) => n + '=' + !!ctx.get(n)).join(' '),
      })

      // ---------------------------------------------------------------- D6
      async function scopeFor(sessions, id) {
        for (let i = 0; i < 100; i++) {
          const scope = sessions.scope(id)
          if (scope) return scope
          await sleep(100)
        }
        return undefined
      }

      /**
       * "Ask about this run": get a Session rooted at `cwd`, show it, prefill
       * (and maybe submit) `text`. `via` picks the route:
       *   'session'   ctx.sessions.create({ cwd }) + ctx.uiWorkspace.openSession(id)
       *   'workspace' ctx.workspaces.create({ path: cwd }) + ctx.uiWorkspace.openWorkspace(id, beforeOpen)
       */
      async function askAboutRun({ cwd, text, sendIt, via = 'workspace' }) {
        const sessions = ctx.get('sessions')
        const uiWorkspace = ctx.get('uiWorkspace')
        const conversation = ctx.get('conversation')
        const workspaces = ctx.get('workspaces')
        const steps = []
        try {
          let id
          if (via === 'workspace') {
            const ws = await workspaces.create({ path: cwd })
            steps.push('workspace ' + ws.workspaceId + ' path=' + ws.path)
            await uiWorkspace.openWorkspace(ws.workspaceId, (sid) => {
              id = sid
              const early = sessions.scope(sid)
              steps.push('beforeOpen ' + sid + ' scope=' + !!early)
              if (early) conversation.input.for(early).setDraft(text)
            })
            steps.push('openWorkspace resolved')
          } else {
            id = await sessions.create(cwd ? { cwd } : {})
            steps.push('created ' + id)
            uiWorkspace.openSession(id)
            steps.push('openSession called')
          }
          await report('ask-opened', { via, sessionId: id, cwd, steps })
          const scope = await scopeFor(sessions, id)
          steps.push('scope ' + (scope ? 'present' : 'absent'))
          if (!scope) return { id, steps }
          const input = conversation.input.for(scope)
          input.setDraft(text)
          steps.push('setDraft done')
          await report('ask-prefilled', { via, sessionId: id, chars: text.length, draftState: JSON.stringify(input.state.getSnapshot?.() ?? null).slice(0, 300) })
          if (sendIt) {
            await sleep(500)
            input.submit()
            steps.push('submit called')
            await report('ask-submitted', { via, sessionId: id })
          }
          CFG.lastSessionId = id
          return { id, steps }
        } catch (e) {
          steps.push('error ' + describe(e))
          await report('ask-error', { via, steps, error: describe(e) })
          return { steps }
        }
      }

      // ---------------------------------------------------------------- D7
      /** Open one file in the right Sidebar's preview, through the on-screen Session. */
      async function openSpec({ path, sessionId }) {
        const sidebarRight = ctx.get('sidebarRight')
        const mounted = sidebarRight.mounted
        const onScreen = mounted?.getSnapshot?.()
        const sid = sessionId || onScreen
        const address = sid ? fileAddress(sid, path) : '(no session)'
        try {
          if (!sid) throw new Error('no Session on screen')
          sidebarRight.openResource(address)
          await report('open-spec-ok', { address, onScreen: onScreen ?? null })
          return { ok: true, address }
        } catch (e) {
          await report('open-spec-error', { address, onScreen: onScreen ?? null, error: describe(e) })
          return { ok: false, address, error: describe(e) }
        }
      }
      async function openFiles() {
        const sidebarRight = ctx.get('sidebarRight')
        try {
          sidebarRight.openTab('files')
          await report('open-files-ok', {})
          return { ok: true }
        } catch (e) {
          await report('open-files-error', { error: describe(e) })
          return { ok: false, error: describe(e) }
        }
      }

      async function inlineAsk(question) {
        const res = await fetch(BASE + '/ask', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ question }) })
        const body = await res.json()
        await report('inline-answer', { status: res.status, answer: body.answer ?? body.error })
        return body
      }

      /** Directory listing and file stat through the Host's workspaceFiles Remote (what the Files tab uses). */
      async function probeFiles({ sessionId, dir, file }) {
        const files = ctx.remote.workspaceFiles
        const out = {}
        for (const [label, run] of [
          ['list ' + dir, () => files.list(sessionId, dir)],
          ['stat ' + file, () => files.stat(sessionId, file)],
        ]) {
          try {
            const r = await run()
            out[label] = JSON.stringify(r).slice(0, 300)
          } catch (e) {
            out[label] = 'threw ' + describe(e)
          }
        }
        await report('probe-files', out)
        return out
      }
      CFG.probeFiles = probeFiles

      // The driver calls the same functions the buttons call.
      CFG.askAboutRun = askAboutRun
      CFG.openSpec = openSpec
      CFG.openFiles = openFiles
      CFG.inlineAsk = inlineAsk

      // ---------------------------------------------------------- the panel
      function Page() {
        const [out, setOut] = useState('')
        const run = (label, fn) => async () => setOut(label + ': ' + JSON.stringify(await fn()))
        const summary = 'About the Brokkr run "Fix the login fence" (realm brokkr, standing: running). Spec: openspec/changes/demo/spec.md. What is it waiting on?'
        const button = (id, label, fn) => h('button', { 'data-testid': id, onClick: run(label, fn), style: { margin: 4 } }, label)
        React.useEffect(() => { report('main-mounted', PANEL) }, [])
        return h('div', { 'data-testid': 'brokkr-slice2-page', style: { padding: 24, fontFamily: 'monospace' } },
          h('h1', null, 'Brokkr slice-two probe'),
          button('btn-ask-prefill', 'Ask about this run (prefill)', () => askAboutRun({ cwd: CFG.worktreeA, text: summary, sendIt: false })),
          button('btn-ask-send', 'Ask about this run (send)', () => askAboutRun({ cwd: CFG.worktreeA, text: summary, sendIt: true })),
          button('btn-open-spec-here', 'Open spec (no Session on screen)', () => openSpec({ path: 'openspec/changes/demo/spec.md' })),
          button('btn-inline', 'Inline answer', () => inlineAsk('Which Brokkr run needs me?')),
          h('pre', { 'data-testid': 'brokkr-slice2-out', style: { whiteSpace: 'pre-wrap' } }, out))
      }
      function Icon({ size, active }) {
        return h('span', { 'data-testid': 'brokkr-slice2-icon', 'data-active': String(active), style: { display: 'inline-block', width: size, textAlign: 'center', fontWeight: 700 } }, 'B2')
      }

      // A conversation-header action: "Open spec" from inside a Session.
      function HeaderAction(props) {
        const sid = props.sessionId
        return h('button', {
          'data-testid': 'brokkr-slice2-header-open-spec',
          title: 'Brokkr: open the spec',
          onClick: () => openSpec({ path: 'openspec/changes/demo/spec.md', sessionId: sid }),
          style: { fontSize: 12 },
        }, 'Brokkr: open spec')
      }

      // Transcript cards for the probe's own tools.
      function argsOf(props) {
        const raw = props.phase === 'start' ? props.block.argsRaw : props.phase === 'result' ? props.block.call?.argsRaw : undefined
        try { return raw ? JSON.parse(raw) : {} } catch { return {} }
      }
      function SuggestCard(props) {
        const [pressed, setPressed] = useState(null)
        const args = argsOf(props)
        React.useEffect(() => { report('toolview-mounted', { tool: props.toolName, phase: props.phase, callId: props.callId, hasMeta: props.phase === 'result' && props.block.meta !== undefined }) }, [props.phase])
        const press = async () => {
          const res = await fetch(BASE + '/press', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ verb: args.verb, run: args.run, from: 'suggestion-card' }) })
          setPressed(res.status + ' ' + JSON.stringify(await res.json()))
        }
        return h('div', { 'data-testid': 'brokkr-suggest-card', 'data-phase': props.phase, style: { border: '1px solid var(--dsw-alias-border-default, #888)', borderRadius: 8, padding: 8, margin: '4px 0' } },
          h('div', null, 'Brokkr suggests: ', h('b', null, args.verb ?? '…'), ' on “', args.run ?? '…', '”'),
          h('div', { style: { opacity: 0.8 } }, args.reason ?? ''),
          props.phase === 'result' ? h('button', { 'data-testid': 'brokkr-suggest-press', onClick: press }, 'Press ' + (args.verb ?? '')) : null,
          pressed ? h('div', { 'data-testid': 'brokkr-suggest-pressed' }, 'pressed: ' + pressed) : null)
      }
      function RunsCard(props) {
        React.useEffect(() => { report('toolview-mounted', { tool: props.toolName, phase: props.phase, callId: props.callId, hasMeta: props.phase === 'result' && props.block.meta !== undefined }) }, [props.phase])
        const runs = props.phase === 'result' && props.block.meta && props.block.meta.runs ? props.block.meta.runs : []
        return h('div', { 'data-testid': 'brokkr-runs-card', 'data-phase': props.phase, style: { border: '1px dashed var(--dsw-alias-border-default, #888)', borderRadius: 8, padding: 8, margin: '4px 0' } },
          'Brokkr runs (' + props.phase + '): ', runs.map((r) => r.title + ' — ' + r.standing).join('; '))
      }

      ctx.slots.inject('main', () => ctx.slots.register({ name: 'main', key: PANEL }, Page))
      ctx.slots.inject('sidebar.panellist', () => ctx.slots.register({ name: 'sidebar.panellist', id: PANEL, order: 51, label: () => 'Brokkr slice 2' }, Icon))
      ctx.slots.inject('conversation.session.header.actions', () => ctx.slots.register({ name: 'conversation.session.header.actions', id: 'brokkr-slice2-open-spec', order: 90 }, HeaderAction))
      ctx.slots.inject('tool.call.toolview', () => ctx.slots.register({ name: 'tool.call.toolview', key: 'brokkr_suggest' }, SuggestCard))
      ctx.slots.inject('tool.call.toolview', () => ctx.slots.register({ name: 'tool.call.toolview', key: 'brokkr_runs' }, RunsCard))
      report('client-registered', 'main+sidebar.panellist+header.actions+toolview(brokkr_suggest,brokkr_runs)')
    }

    module.exports.apply = apply
    module.exports.inject = ['slots', 'sessions', 'workspaces', 'uiWorkspace', 'conversation', 'sidebarRight', 'remote', 'remote.workspaceFiles']
    module.exports.name = 'brokkr-slice2-probe'
    return module.exports
  },
})
