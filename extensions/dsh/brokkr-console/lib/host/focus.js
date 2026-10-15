// What the operator is looking at in a conversation's Brokkr tab, told to
// that conversation's model as context on its next turn. It is a `snapshot`
// form message from this plugin's own source, as dsh's tmux-context tells the
// model where it runs: a later snapshot supersedes an earlier one, it is told
// only when it changed, and nothing is typed into the operator's message box.
import { randomUUID } from 'node:crypto'

import { RequestRefused } from './commands.js'

export const SOURCE = 'brokkr-console'
const SESSION = /^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$/
const TEXT_BOUND = 4000
/** The most conversations whose focus is held; the oldest is let go first. */
const SESSIONS_BOUND = 256

function frozen(value) {
  if (value !== null && typeof value === 'object') {
    for (const inner of Object.values(value)) frozen(inner)
    Object.freeze(value)
  }
  return value
}

/**
 * Anything a model could read as closing the frame: `</system-reminder` in
 * any case, with any whitespace around the slash. The text is built from run
 * titles, labels and seat names, which an operator can paste anything into.
 */
const CLOSING = /<\s*\/\s*system-reminder/gi

/** The model-facing message: the focus in a `<system-reminder>` frame the text cannot close. */
export function messageOf(text) {
  const framed = `<system-reminder>\n${text.replace(CLOSING, '<\\/system-reminder')}\n</system-reminder>`
  return frozen({
    id: randomUUID(),
    role: 'user',
    content: [{ type: 'text', text: framed }],
    source: { kind: SOURCE, form: 'snapshot', sections: [{ name: SOURCE, text: framed }] },
  })
}

export function focusOf() {
  const latest = new Map()
  const told = new Map()
  return {
    /** `{session, text}` from a conversation's tab: what its model is told next. */
    set(body) {
      const { session, text } = body ?? {}
      if (typeof session !== 'string' || !SESSION.test(session)) throw new RequestRefused('the focus names no conversation')
      if (typeof text !== 'string' || text.length === 0 || text.length > TEXT_BOUND) throw new RequestRefused('the focus is not text of a bounded length')
      latest.delete(session)
      latest.set(session, text)
      if (latest.size > SESSIONS_BOUND) {
        const [oldest] = latest.keys()
        latest.delete(oldest)
        told.delete(oldest)
      }
    },
    /**
     * The `agent/pre-step` listener: on a turn's first step, a conversation
     * whose focus changed since it was last told gets it before the operator's
     * message, as context.
     */
    async preStep({ agent, step, signal }, next) {
      const decision = await next()
      if (decision.kind === 'reject' || signal.aborted || step !== 1) return decision
      const session = agent.session?.id
      const text = latest.get(session)
      if (text === undefined || told.get(session) === text) return decision
      told.set(session, text)
      // An accepted step may carry no messages of its own; the focus is still told.
      return { ...decision, messages: [messageOf(text), ...(decision.messages ?? [])] }
    },
  }
}
