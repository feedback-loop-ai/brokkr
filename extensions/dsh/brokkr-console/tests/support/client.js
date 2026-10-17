// The browser half, loaded as DSH loads it, into a sandbox: the script calls
// window.__ModuleLoader__.load, and its factory's `require('react')` gets a
// small stand-in that renders function components with `useState` and
// `useEffect` into a plain tree. The tests read that tree's text and press
// its buttons, so they exercise the panel the operator sees, not a copy.
import { readFileSync } from 'node:fs'
import { runInNewContext } from 'node:vm'

const SOURCE = readFileSync(new URL('../../lib/client.js', import.meta.url), 'utf8')
const Fragment = Symbol('Fragment')

/**
 * The panel's clock: an hour after the fixtures' last row (support/world.js,
 * 2026-10-13). The panel reads the time for "how long ago" and for the runs
 * it lists as finished in the last 24 hours, so with the wall clock a test
 * would pass or fail by the day it ran.
 */
export const NOW = Date.parse('2026-10-13T10:00:00Z')
class PinnedDate extends Date {
  constructor(...at) {
    super(...(at.length === 0 ? [NOW] : at))
  }
  static now() {
    return NOW
  }
}

/** A stand-in for React that renders its roots into `{type, props, children}` nodes. */
function reactStub() {
  const slots = new Map()
  const run = { roots: [], trees: [], flushing: false, dirty: false, effects: [], slot: null, hook: 0 }
  const createElement = (type, props, ...children) => ({
    type,
    props: { ...(props ?? {}), children: children.length === 0 ? props?.children : children.length === 1 ? children[0] : children },
  })
  function schedule() {
    if (run.flushing) run.dirty = true
    else flush()
  }
  function useState(initial) {
    const [slot, index] = [run.slot, run.hook++]
    if (!(index in slot.hooks)) slot.hooks[index] = initial
    const set = (value) => {
      slot.hooks[index] = typeof value === 'function' ? value(slot.hooks[index]) : value
      schedule()
    }
    return [slot.hooks[index], set]
  }
  function useEffect(effect, deps) {
    const [slot, index] = [run.slot, run.hook++]
    const before = slot.hooks[index]
    if (before !== undefined && deps.every((dep, at) => Object.is(dep, before.deps[at]))) return
    run.effects.push(() => {
      before?.cleanup?.()
      slot.hooks[index] = { deps, cleanup: effect() }
    })
  }
  function render(node, path) {
    if (node === null || node === undefined || typeof node === 'boolean') return null
    if (typeof node === 'string' || typeof node === 'number') return String(node)
    if (Array.isArray(node)) return node.map((child, index) => render(child, `${path}.${child?.props?.key ?? index}`))
    if (node.type === Fragment) return render(node.props.children, path)
    if (typeof node.type === 'function') {
      const key = `${path}:${node.type.name}:${node.props.key ?? ''}`
      if (!slots.has(key)) slots.set(key, { hooks: [] })
      Object.assign(run, { slot: slots.get(key), hook: 0 })
      return render(node.type(node.props), key)
    }
    const { children, ...props } = node.props
    return { type: node.type, props, children: [render(children, `${path}/${node.type}`)].flat(Infinity).filter((child) => child !== null) }
  }
  function flush() {
    run.flushing = true
    do {
      run.dirty = false
      run.trees = run.roots.map((root, index) => render(root, `root${index}`))
      const effects = run.effects.splice(0)
      for (const effect of effects) effect()
    } while (run.dirty)
    run.flushing = false
  }
  const React = { createElement, Fragment, useState, useEffect }
  /** Mount one more root; its getter reads that root's tree. */
  function mount(element) {
    const index = run.roots.push(element) - 1
    flush()
    return () => run.trees[index]
  }
  return { React, mount }
}

/** The text a rendered node reads as. */
export function textOf(node) {
  if (typeof node === 'string') return node
  if (node === null) return ''
  if (Array.isArray(node)) return node.map(textOf).join('')
  return node.children.map(textOf).join('')
}

/** Every rendered element `where` holds true for. */
export function findAll(node, where) {
  if (node === null || typeof node === 'string') return []
  if (Array.isArray(node)) return node.flatMap((child) => findAll(child, where))
  return [...(where(node) ? [node] : []), ...node.children.flatMap((child) => findAll(child, where))]
}

/** The buttons, by their text. */
export const buttons = (tree) => findAll(tree, (node) => node.type === 'button')

const REFUSED = Symbol('refused')
/** An answer the console's route refuses: the page reads a failed response whose body carries `error`. */
export const refusedWith = (error) => ({ [REFUSED]: { error } })

/** The client's exports, loaded as DSH loads them but mounting nothing: for the pure readings it exports beside `apply`. */
export function clientModule() {
  let loaded = null
  runInNewContext(SOURCE, { window: { __ModuleLoader__: { load: (module) => (loaded = module) } } })
  return loaded.factory((name) => ({ react: reactStub().React })[name])
}

/**
 * Load the client in a sandbox and mount `component` ('main' or 'sidebar.panellist').
 * `storage` is the browser's `localStorage` the page sees (none unless given).
 * @returns the rendered tree's getter, the sockets the client opened, and the fetches it made.
 */
export function mountClient(component, props = {}, answers = {}, dsh = dshStub(), storage = undefined) {
  const { React, mount } = reactStub()
  const sockets = []
  const fetches = []
  class WebSocket {
    constructor(url) {
      Object.assign(this, { url, readyState: 0, sent: [] })
      sockets.push(this)
    }
    send(text) {
      this.sent.push(JSON.parse(text))
    }
    close() {}
  }
  let loaded = null
  const sandbox = {
    window: { __ModuleLoader__: { load: (module) => (loaded = module) }, localStorage: storage },
    WebSocket,
    location: { origin: 'http://127.0.0.1:3080' },
    document: { getElementById: () => ({}), createElement: () => ({}), head: { appendChild: () => {} } },
    crypto: { randomUUID: () => '0b6f1c2e-5d4a-4e8b-9f10-3c2d1e0f9a8b' },
    Date: PinnedDate,
    URLSearchParams,
    setTimeout,
    // An answer is a body, or a function of the url that settles with one, so a test can hold a read back.
    fetch: async (url, init) => {
      fetches.push({ url, init })
      const path = String(url).split('?')[0]
      const answer = answers[path] ?? { error: 'no answer' }
      const body = typeof answer === 'function' ? await answer(String(url)) : answer
      const refused = body?.[REFUSED]
      return { ok: path in answers && refused === undefined, json: async () => refused ?? body }
    },
  }
  runInNewContext(SOURCE, sandbox)
  const exports = loaded.factory((name) => ({ react: React })[name])
  const registered = {}
  exports.apply({
    slots: {
      inject: (_name, register) => register(),
      register: (slot, Component) => (registered[slot.name === 'tool.call.toolview' ? `toolview:${slot.key}` : slot.name] = { slot, Component }),
    },
    get: (name) => dsh.services[name],
  })
  const tree = mount(React.createElement(registered[component].Component, props))
  if (sockets[0] !== undefined) {
    sockets[0].readyState = 1
    sockets[0].onopen()
  }
  /** Deliver a message on the socket the client opened. */
  const push = (message) => sockets[0].onmessage({ data: JSON.stringify(message) })
  /** Mount another registered component beside the first, in the same client. */
  const also = (name, extra = {}) => mount(React.createElement(registered[name].Component, extra))
  return { tree, push, sockets, fetches, registered, also, dsh, inject: exports.inject }
}

/**
 * Stand-ins for the DSH client services the assistant uses (slice two,
 * evidence D6b and D7), each call recorded in `calls`.
 */
export function dshStub() {
  const calls = []
  const scopes = new Map()
  const services = {
    workspaces: { create: async ({ path }) => (calls.push(['workspaces.create', path]), { workspaceId: `ws:${path}`, path }) },
    uiWorkspace: {
      openWorkspace: async (workspaceId, beforeOpen) => {
        const sessionId = `session:${workspaceId}`
        scopes.set(sessionId, { sessionId })
        calls.push(['openWorkspace', workspaceId])
        beforeOpen?.(sessionId)
      },
    },
    sessions: { scope: (sessionId) => scopes.get(sessionId) },
    conversation: {
      input: {
        for: (scope) => ({
          setDraft: (text) => calls.push(['setDraft', scope.sessionId, text]),
          submit: () => calls.push(['submit', scope.sessionId]),
        }),
      },
    },
    sidebarRight: { openTab: (kind) => calls.push(['openTab', kind]) },
  }
  return { services, calls }
}
