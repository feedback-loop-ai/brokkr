// brokkr-slice2-probe, host half. Measures, on DSH 0.2.0-rc.2, the APIs slice
// two of the Brokkr console needs:
//   D6  model tools a third-party plugin registers (ctx.tools.register), a
//       one-shot model call from the host half (ctx.llm.stream), and the
//       routes the browser half uses to report what it observed;
//   D7  nothing host-side (the Files sidebar is browser-only), except the
//       report beacon.
// Every route goes through DSH's connection fence (ctx.connection.requestRejection).
import { appendFileSync } from 'node:fs'

export const name = 'brokkr-slice2-probe'
export const inject = ['webServer', 'connection', 'tools']

const BASE = '/brokkr-slice2'
const LOG = process.env.BROKKR_PROBE_LOG

function log(line) {
  const text = `${new Date().toISOString()} ${line}\n`
  if (LOG) appendFileSync(LOG, text)
  else process.stderr.write(`[${name}] ${text}`)
}

function send(res, status, body) {
  res.statusCode = status
  res.setHeader('content-type', 'application/json; charset=utf-8')
  res.setHeader('cache-control', 'no-store')
  res.end(JSON.stringify(body))
}

function bodyOf(req) {
  return new Promise((settle, fail) => {
    const chunks = []
    req.on('data', (c) => chunks.push(c))
    req.on('end', () => {
      try { settle(chunks.length ? JSON.parse(Buffer.concat(chunks).toString('utf8')) : {}) } catch (e) { fail(e) }
    })
    req.on('error', fail)
  })
}

// ------------------------------------------------------------------ tools
// Hand-written ToolDefinition objects: defineTool lives in
// @deepseek-ai/dsh-tools, which a package linked from outside DSH's tree
// cannot resolve, and register() only requires { name, description,
// parameters, output: { schema, render, presentationMeta? }, execute }.

const RUNS = [
  { title: 'Fix the login fence', realm: 'brokkr', standing: 'running', spec: 'openspec/changes/demo/spec.md' },
  { title: 'Teach the poller to back off', realm: 'brokkr', standing: 'waiting for review', spec: 'openspec/changes/demo/spec.md' },
]

const runsTool = {
  name: 'brokkr_runs',
  description: 'Read-only. List the Brokkr runs the operator console can see, as JSON. Use it to answer questions about runs.',
  parameters: {
    type: 'object',
    properties: { realm: { type: 'string', description: 'Only runs in this realm' } },
    additionalProperties: false,
  },
  output: {
    schema: { type: 'object' },
    render: (_args, value) => [{ type: 'text', text: JSON.stringify(value) }],
    presentationMeta: (_args, value) => value,
  },
  isConcurrencySafe: () => true,
  async execute(args, exec) {
    log(`tool brokkr_runs execute args=${JSON.stringify(args)} agent=${exec?.agent ? 'present' : 'absent'} callId=${exec?.callId ?? '-'}`)
    const realm = args && typeof args === 'object' ? args.realm : undefined
    return { runs: RUNS.filter((r) => !realm || r.realm === realm) }
  },
}

const suggestTool = {
  name: 'brokkr_suggest',
  description: 'Show the operator a suggestion card naming one console button they can press. It presses nothing itself.',
  parameters: {
    type: 'object',
    properties: {
      run: { type: 'string', description: 'The run title' },
      verb: { type: 'string', description: 'The console verb the operator may press', enum: ['resume', 'retry', 'open-spec'] },
      reason: { type: 'string', description: 'Why, in one sentence' },
    },
    required: ['run', 'verb', 'reason'],
    additionalProperties: false,
  },
  output: {
    schema: { type: 'object' },
    render: (_args, value) => [{ type: 'text', text: `Suggestion shown to the operator: ${JSON.stringify(value)}` }],
    presentationMeta: (_args, value) => value,
  },
  async execute(args) {
    log(`tool brokkr_suggest execute args=${JSON.stringify(args)}`)
    return { shown: true, ...args }
  },
}

// ------------------------------------------------------------------ apply

export function apply(ctx) {
  const fence = (handler) => async (req, res) => {
    const refusal = ctx.connection.requestRejection(req)
    if (refusal !== undefined) {
      log(`fence refused ${req.method} ${req.url} status=${refusal}`)
      res.statusCode = refusal
      return res.end()
    }
    try { await handler(req, res) } catch (e) { log(`route error ${req.url} ${e?.stack ?? e}`); send(res, 500, { error: String(e?.message ?? e) }) }
  }
  const route = (path, handler) =>
    ctx.effect(() => ctx.webServer.register({ kind: 'exact', path, handler: fence(handler) }), `${name}: ${path}`)

  for (const tool of [runsTool, suggestTool]) {
    ctx.effect(() => ctx.tools.register(tool), `${name}: tool ${tool.name}`)
  }
  log(`apply: registered tools; global view now has ${ctx.tools.schemas().length} schemas; ours: ${ctx.tools.schemas().map((s) => s.name).filter((n) => n.startsWith('brokkr_')).join(',')}`)
  ctx.on('tools/change', () => log(`tools/change: global names with brokkr_: ${ctx.tools.schemas().map((s) => s.name).filter((n) => n.startsWith('brokkr_')).join(',') || '(none)'}`))
  ctx.on('tools/result', (exec, result) => log(`tools/result name=${exec.name} isError=${result?.isError} agent=${exec.agent ? 'present' : 'absent'}`))

  // The browser half's beacons.
  route(`${BASE}/report`, async (req, res) => {
    const body = await bodyOf(req)
    log(`client-report event=${body.event}${body.detail !== undefined ? ` detail=${typeof body.detail === 'string' ? body.detail : JSON.stringify(body.detail)}` : ''}`)
    send(res, 200, { ok: true })
  })

  // What the global tool registry holds now (no agent scope).
  route(`${BASE}/tools`, async (_req, res) => {
    send(res, 200, { global: ctx.tools.schemas().map((s) => s.name) })
  })

  // A suggestion card's button press lands here (user-only action).
  route(`${BASE}/press`, async (req, res) => {
    const body = await bodyOf(req)
    log(`press verb=${body.verb} run=${JSON.stringify(body.run)} from=${body.from}`)
    send(res, 200, { pressed: body.verb, run: body.run })
  })

  // Inline answer: one-shot model call from the host half with the default model.
  route(`${BASE}/ask`, async (req, res) => {
    if (req.method !== 'POST') { res.statusCode = 405; return res.end() }
    const body = await bodyOf(req)
    const llm = ctx.get('llm')
    const defaults = ctx.get('agentDefaultModel')
    if (!llm) return send(res, 503, { error: 'no llm service' })
    const selection = defaults?.currentSelection?.() ?? null
    log(`ask: llm=${!!llm} defaultModel=${JSON.stringify(selection)} question=${JSON.stringify(body.question)}`)
    if (!selection) return send(res, 503, { error: 'no default model selection' })
    const chunks = []
    let text = ''
    for await (const chunk of llm.stream({
      provider: selection.provider,
      model: selection.model,
      system: 'You answer an operator about Brokkr runs, briefly.',
      messages: [{ role: 'user', content: [{ type: 'text', text: String(body.question ?? '') }] }],
      maxTokens: 256,
    })) {
      chunks.push(chunk.type)
      if (chunk.type === 'text-delta') text += chunk.text
      if (chunk.type === 'finish' || chunk.type === 'error') log(`ask: terminal chunk ${JSON.stringify(chunk).slice(0, 400)}`)
    }
    log(`ask: chunk types=${[...new Set(chunks)].join(',')} text=${JSON.stringify(text)}`)
    send(res, 200, { answer: text, chunkTypes: [...new Set(chunks)], selection })
  })

  log(`apply: routes registered under ${BASE}; webServer ${ctx.webServer.host}:${ctx.webServer.port}`)
}
