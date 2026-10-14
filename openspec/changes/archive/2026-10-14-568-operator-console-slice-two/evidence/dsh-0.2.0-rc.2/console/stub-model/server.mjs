// Loopback stand-in for an OpenAI chat-completions endpoint. Test harness
// only: no real model, no credential. Scripted for the brokkr-console tools:
//   user turn (tools offered)        -> call brokkr_runs {}
//   after brokkr_runs' result        -> call brokkr_inspect {"run": RUN}, RUN read from that result
//   after brokkr_inspect's result    -> call brokkr_suggest {"run": RUN, "command": "stop", "reason": "live check"}
//   after brokkr_suggest's result    -> final text
//   anything else (session title)    -> short text
// It logs the tool list, the asked prompt, and every tool result it receives (truncated).
// Usage: node server.mjs <port> <log file>
import { createServer } from 'node:http'
import { appendFileSync } from 'node:fs'

const [port, logFile] = [Number(process.argv[2]), process.argv[3]]
const PREFERRED = 'landing-pr-660-at-4f6a1931-the-d-08a428d4'
const log = (line) => appendFileSync(logFile, `${new Date().toISOString()} ${line}\n`)
const cut = (s, n) => (s.length > n ? `${s.slice(0, n)}… [${s.length} chars]` : s)
let n = 0

function textOf(content) {
  if (typeof content === 'string') return content
  if (Array.isArray(content)) return content.map((c) => c.text ?? `[${c.type}]`).join('')
  return ''
}

/** The run id the brokkr_runs answer lists: the preferred one when present, else the first. */
function runFrom(runsText) {
  if (runsText.includes(`"${PREFERRED}"`)) return PREFERRED
  const m = runsText.match(/"run_id"\s*:\s*"([^"]+)"/)
  return m ? m[1] : PREFERRED
}

function plan(body, id) {
  const tools = (body.tools ?? []).map((t) => t.function?.name ?? t.name)
  const msgs = body.messages ?? []
  const last = msgs[msgs.length - 1]
  const callName = (callId) => { for (const m of msgs) for (const c of m.tool_calls ?? []) if (c.id === callId) return c.function?.name }
  const resultOf = (name) => { for (const m of msgs) if (m.role === 'tool' && callName(m.tool_call_id) === name) return textOf(m.content); return '' }
  if (last?.role === 'tool') {
    const name = callName(last.tool_call_id)
    if (name === 'brokkr_runs') return { tool: { name: 'brokkr_inspect', arguments: { run: runFrom(resultOf('brokkr_runs')) } } }
    if (name === 'brokkr_inspect') return { tool: { name: 'brokkr_suggest', arguments: { run: runFrom(resultOf('brokkr_runs')), command: 'stop', reason: 'live check' } } }
    return { text: `Stub answer (live check): I read the runs, inspected ${runFrom(resultOf('brokkr_runs'))} and asked Brokkr about stopping it; Brokkr's answer is above.` }
  }
  if (tools.includes('brokkr_runs') && msgs.some((m) => m.role === 'user')) return { tool: { name: 'brokkr_runs', arguments: {} } }
  return { text: 'Stub title for the live check' }
}

const server = createServer((req, res) => {
  const chunks = []
  req.on('data', (c) => chunks.push(c))
  req.on('end', () => {
    const id = ++n
    if (req.method === 'GET') { log(`#${id} GET ${req.url}`); res.setHeader('content-type', 'application/json'); return res.end(JSON.stringify({ object: 'list', data: [{ id: 'stub-model', object: 'model' }] })) }
    let body = {}
    try { body = JSON.parse(Buffer.concat(chunks).toString('utf8')) } catch {}
    const tools = (body.tools ?? []).map((t) => t.function?.name ?? t.name)
    const msgs = body.messages ?? []
    log(`#${id} ${req.method} ${req.url} auth=${req.headers.authorization ? 'present(redacted)' : 'absent'} model=${body.model} roles=${msgs.map((m) => m.role).join(',')} tools(${tools.length})=${tools.join(',')}`)
    const brokkrTools = tools.filter((t) => t.startsWith('brokkr_'))
    if (brokkrTools.length) log(`#${id} brokkr tools offered: ${brokkrTools.join(',')}`)
    const asked = msgs.filter((m) => m.role === 'user').map((m) => textOf(m.content)).find((t) => t.includes('Question:'))
    if (asked !== undefined) log(`#${id} asked prompt: ${JSON.stringify(cut(asked, 1500))}`)
    const where = msgs.map((m) => textOf(m.content)).join('\n').match(/session workspace: "([^"]+)"/)
    if (where) log(`#${id} session workspace (from DSH's runtime context): ${where[1]}`)
    else if (!tools.length) log(`#${id} user text: ${JSON.stringify(cut(textOf(msgs.filter((m) => m.role === 'user').pop()?.content), 300))}`)
    const last = msgs[msgs.length - 1]
    if (last?.role === 'tool') {
      const name = (() => { for (const m of msgs) for (const c of m.tool_calls ?? []) if (c.id === last.tool_call_id) return c.function?.name })()
      const text = textOf(last.content)
      log(`#${id} tool result received from ${name} (${text.length} chars): ${cut(text, name === 'brokkr_suggest' ? 2000 : 900)}`)
    }
    const p = plan(body, id)
    log(`#${id} reply ${p.tool ? `tool_call ${p.tool.name} ${JSON.stringify(p.tool.arguments)}` : `text ${JSON.stringify(p.text)}`}`)
    const created = Math.floor(Date.now() / 1000)
    const base = { id: `chatcmpl-stub-${id}`, object: 'chat.completion.chunk', created, model: body.model }
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
    sse({ ...base, choices: [], usage: { prompt_tokens: 10, completion_tokens: 5, total_tokens: 15 } })
    res.end('data: [DONE]\n\n')
  })
})
server.listen(port, '127.0.0.1', () => log(`stub listening on 127.0.0.1:${port}`))
