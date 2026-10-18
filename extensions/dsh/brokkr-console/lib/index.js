// brokkr-console, host half: a DSH plugin that serves Brokkr's readouts and
// its four operator verbs to the console's browser half. It paints nothing
// and decides nothing: every standing, admission, refusal, reason and figure
// comes from a `brokkr ... --json` readout (design D1, decision 0013). Every
// route and the one upgrade go through DSH's connection fence first. DSH's
// model gets read-only tools over the same readouts (`host/tools.js`).
import { readFileSync } from 'node:fs'

import { brokkrOf } from './host/brokkr.js'
import { deskOf, RequestRefused } from './host/commands.js'
import { configFrom } from './host/config.js'
import { fencedRoute, fencedUpgrade } from './host/fence.js'
import { focusOf } from './host/focus.js'
import { hereOf } from './host/here.js'
import { pollerOf } from './host/poller.js'
import { isRange, ReadoutRefused } from './host/readouts.js'
import { accept } from './host/socket.js'
import { GRAMMAR, toolsOf } from './host/tools.js'

export const name = 'brokkr-console'
export const inject = ['webServer', 'connection', 'tools']

const BASE = '/brokkr-console'
const BODY_BOUND_BYTES = 64 * 1024
const STYLES = readFileSync(new URL('./console.css', import.meta.url))

function send(res, status, body) {
  res.statusCode = status
  res.setHeader('content-type', 'application/json; charset=utf-8')
  res.setHeader('cache-control', 'no-store')
  res.end(JSON.stringify(body))
}

/** The request's JSON body, or a refusal when it is too long or not JSON. */
function bodyOf(req) {
  return new Promise((settle, fail) => {
    const chunks = []
    let bytes = 0
    req.on('data', (chunk) => {
      bytes += chunk.length
      if (bytes > BODY_BOUND_BYTES) fail(new RequestRefused('the command is too long'))
      else chunks.push(chunk)
    })
    req.on('end', () => {
      try {
        settle(JSON.parse(Buffer.concat(chunks).toString('utf8')))
      } catch {
        fail(new RequestRefused('the command is not JSON'))
      }
    })
    req.on('error', fail)
  })
}

/** The request's query string. */
const queryOf = (req) => new URLSearchParams(String(req.url).split('?')[1] ?? '')

/** The range a readout is asked over: `7d` unless the query names one Brokkr takes. */
function rangeOf(query) {
  const range = query.get('range') ?? '7d'
  if (!isRange(range)) throw new RequestRefused('the range is one of 24h, 7d or 30d')
  return range
}

/** A handler for `method`, answering a refused request or readout with its text. */
function handled(method, handle) {
  return async (req, res) => {
    if (req.method !== method) {
      res.statusCode = 405
      res.setHeader('allow', method)
      return res.end()
    }
    try {
      await handle(req, res)
    } catch (error) {
      if (error instanceof RequestRefused) return send(res, 400, { error: error.message })
      if (error instanceof ReadoutRefused) return send(res, 502, { error: error.message })
      console.error('[brokkr-console]', error)
      send(res, 500, { error: 'the console failed; DSH’s log has the detail' })
    }
  }
}

/** The routes, each `[method, path, handler]`. */
function routes({ brokkr, desk, here, locate, focus }) {
  return [
    ['POST', `${BASE}/api/command`, async (req, res) => send(res, 200, await desk.submit(await bodyOf(req)))],
    // What a conversation's Brokkr tab shows, told to that conversation's model on its next turn.
    // The tab reads only that it was taken; there is nothing else to say.
    [
      'POST',
      `${BASE}/api/focus`,
      async (req, res) => {
        focus.set(await bodyOf(req))
        send(res, 200, {})
      },
    ],
    [
      'GET',
      `${BASE}/api/here`,
      async (req, res) => send(res, 200, await here(queryOf(req).get('dir'))),
    ],
    [
      'GET',
      `${BASE}/api/transcript`,
      async (req, res) => {
        const query = queryOf(req)
        const [realm, run, seat] = ['realm', 'run', 'seat'].map((name) => query.get(name) ?? '')
        if (!GRAMMAR.run.test(run) || !GRAMMAR.seat.test(seat) || realm.length > 256) throw new RequestRefused('a transcript is asked for by realm, run and seat')
        // The journal is the listing's, never a path the browser sent.
        const target = locate(realm, run)
        if (target === null) throw new RequestRefused('Brokkr no longer lists this run')
        send(res, 200, await brokkr.transcript(target, seat))
      },
    ],
    [
      'GET',
      `${BASE}/api/analytics`,
      async (req, res) => {
        const query = queryOf(req)
        send(res, 200, await brokkr.analytics(rangeOf(query), query.get('realm')))
      },
    ],
    // The Library's Graph: one recipe of one realm (or of a one-hearth world, with no realm named);
    // with no recipe named, the one Brokkr picks, so a realm none of whose recipes ran still lists them.
    [
      'GET',
      `${BASE}/api/graph`,
      async (req, res) => {
        const query = queryOf(req)
        const range = rangeOf(query)
        const [recipe, realm] = [query.get('recipe'), query.get('realm')]
        if (recipe !== null && !GRAMMAR.recipe.test(recipe)) throw new RequestRefused('a graph is asked for by its recipe’s name')
        if (realm !== null && (realm === '' || realm.length > 256)) throw new RequestRefused('a realm is named in 1 to 256 characters')
        send(res, 200, await brokkr.graph(realm, recipe, range))
      },
    ],
    [
      'GET',
      `${BASE}/console.css`,
      async (_req, res) => {
        res.setHeader('content-type', 'text/css; charset=utf-8')
        // Revalidated on every load, so an upgraded console never paints with the last version's sheet.
        res.setHeader('cache-control', 'no-cache')
        res.end(STYLES)
      },
    ],
  ]
}

export function apply(ctx) {
  const config = configFrom(process.env, process.cwd())
  const brokkr = brokkrOf(config)
  const poller = pollerOf({ brokkr, pollMs: config.pollMs, workspace: config.cwd, ask: config.ask })
  const desk = deskOf({ brokkr, resume: config.resume, locate: poller.locate })
  const here = hereOf({ brokkr, workspace: config.cwd })
  const focus = focusOf()
  for (const [method, path, handle] of routes({ brokkr, desk, here, locate: poller.locate, focus })) {
    const handler = fencedRoute(ctx.connection, handled(method, handle))
    ctx.effect(() => ctx.webServer.register({ kind: 'exact', path, handler }), `${name}: ${method} ${path}`)
  }
  const upgrade = fencedUpgrade(ctx.connection, (req, socket, head) => {
    let receive = () => {}
    const peer = accept(req, socket, head, (message) => receive(message))
    if (peer !== null) receive = poller.attach(peer)
  })
  ctx.effect(() => ctx.webServer.registerUpgrade({ path: `${BASE}/ws`, handler: upgrade }), `${name}: UPGRADE ${BASE}/ws`)
  // A conversation is told what its Brokkr tab shows before its next turn, never in its message box.
  ctx.on('agent/pre-step', (payload, next) => focus.preStep(payload, next), { prepend: true })
  // The assistant's read-only tools (slice two, design D2 and D3).
  for (const definition of toolsOf({ config, brokkr })) {
    ctx.effect(() => ctx.tools.register(definition), `${name}: tool ${definition.name}`)
  }
}
