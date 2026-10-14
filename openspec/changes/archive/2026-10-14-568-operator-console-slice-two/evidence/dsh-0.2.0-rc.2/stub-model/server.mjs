// Loopback stand-in for an OpenAI chat-completions endpoint. Test harness
// only: no real model, no credential. It logs what DSH sends (tool names,
// message roles, the last user text) and scripts the replies:
//   tools offered + no tool result yet      -> call brokkr_runs {}
//   last message is brokkr_runs' result     -> call brokkr_suggest {...}
//   last message is brokkr_suggest's result -> final text
//   anything else (title, one-shot)         -> short text
// Usage: node server.mjs <port> <log file>
import { createServer } from 'node:http'
import { appendFileSync } from 'node:fs'

const [port, logFile] = [Number(process.argv[2]), process.argv[3]]
const log = (line) => appendFileSync(logFile, `${new Date().toISOString()} ${line}\n`)
let n = 0

function textOf(content) {
  if (typeof content === 'string') return content
  if (Array.isArray(content)) return content.map((c) => c.text ?? `[${c.type}]`).join('')
  return ''
}

function plan(body) {
  const tools = (body.tools ?? []).map((t) => t.function?.name ?? t.name)
  const msgs = body.messages ?? []
  const last = msgs[msgs.length - 1]
  const toolNameOf = (id) => {
    for (const m of msgs) for (const c of m.tool_calls ?? []) if (c.id === id) return c.function?.name
    return undefined
  }
  if (last?.role === 'tool') {
    const name = toolNameOf(last.tool_call_id)
    if (name === 'brokkr_runs') {
      return { tool: { name: 'brokkr_suggest', arguments: { run: 'Fix the login fence', verb: 'open-spec', reason: 'It is running; its spec says what it waits on.' } } }
    }
    return { text: `Stub answer: the run "Fix the login fence" is running (I read it with brokkr_runs and suggested a button).` }
  }
  if (tools.includes('brokkr_runs') && last?.role === 'user') return { tool: { name: 'brokkr_runs', arguments: {} } }
  return { text: 'Stub answer: Fix the login fence is running.' }
}

const server = createServer((req, res) => {
  const chunks = []
  req.on('data', (c) => chunks.push(c))
  req.on('end', () => {
    const raw = Buffer.concat(chunks).toString('utf8')
    const id = ++n
    if (req.method === 'GET') {
      log(`#${id} GET ${req.url}`)
      res.setHeader('content-type', 'application/json')
      return res.end(JSON.stringify({ object: 'list', data: [{ id: 'stub-model', object: 'model' }] }))
    }
    let body = {}
    try { body = JSON.parse(raw) } catch {}
    const tools = (body.tools ?? []).map((t) => t.function?.name ?? t.name)
    const msgs = body.messages ?? []
    const lastUser = [...msgs].reverse().find((m) => m.role === 'user')
    log(`#${id} ${req.method} ${req.url} auth=${req.headers.authorization ? 'present(redacted)' : 'absent'} model=${body.model} stream=${!!body.stream} roles=${msgs.map((m) => m.role).join(',')} tools(${tools.length})=${tools.join(',')}`)
    log(`#${id} last user text=${JSON.stringify(textOf(lastUser?.content)).slice(0, 300)}`)
    const ours = (body.tools ?? []).filter((t) => (t.function?.name ?? '').startsWith('brokkr_'))
    if (ours.length) log(`#${id} brokkr tool schemas as sent: ${JSON.stringify(ours)}`)
    const p = plan(body)
    log(`#${id} reply ${p.tool ? `tool_call ${p.tool.name} ${JSON.stringify(p.tool.arguments)}` : `text ${JSON.stringify(p.text)}`}`)
    const created = Math.floor(Date.now() / 1000)
    const base = { id: `chatcmpl-stub-${id}`, object: 'chat.completion.chunk', created, model: body.model }
    const usage = { prompt_tokens: 10, completion_tokens: 5, total_tokens: 15 }
    if (!body.stream) {
      res.setHeader('content-type', 'application/json')
      const message = p.tool
        ? { role: 'assistant', content: null, tool_calls: [{ id: `call_${id}`, type: 'function', function: { name: p.tool.name, arguments: JSON.stringify(p.tool.arguments) } }] }
        : { role: 'assistant', content: p.text }
      return res.end(JSON.stringify({ id: base.id, object: 'chat.completion', created, model: body.model, choices: [{ index: 0, message, finish_reason: p.tool ? 'tool_calls' : 'stop' }], usage }))
    }
    res.writeHead(200, { 'content-type': 'text/event-stream', 'cache-control': 'no-cache', connection: 'keep-alive' })
    const sse = (obj) => res.write(`data: ${JSON.stringify(obj)}\n\n`)
    if (p.tool) {
      sse({ ...base, choices: [{ index: 0, delta: { role: 'assistant', content: null, tool_calls: [{ index: 0, id: `call_${id}`, type: 'function', function: { name: p.tool.name, arguments: '' } }] }, finish_reason: null }] })
      sse({ ...base, choices: [{ index: 0, delta: { tool_calls: [{ index: 0, function: { arguments: JSON.stringify(p.tool.arguments) } }] }, finish_reason: null }] })
      sse({ ...base, choices: [{ index: 0, delta: {}, finish_reason: 'tool_calls' }] })
    } else {
      sse({ ...base, choices: [{ index: 0, delta: { role: 'assistant', content: '' }, finish_reason: null }] })
      for (const piece of p.text.match(/.{1,24}/g)) sse({ ...base, choices: [{ index: 0, delta: { content: piece }, finish_reason: null }] })
      sse({ ...base, choices: [{ index: 0, delta: {}, finish_reason: 'stop' }] })
    }
    sse({ ...base, choices: [], usage })
    res.end('data: [DONE]\n\n')
  })
})
server.listen(port, '127.0.0.1', () => log(`stub listening on 127.0.0.1:${port}`))
