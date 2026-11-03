// The transport's guard: only `publish` may send world data. The poller keeps
// each client's publish path private and replaces `peer.send` with the facade
// below, so no spelling of a send outside `publish` can carry a fleet, snapshot,
// run, overview, workspace or ask. A refused send throws by name; it is never
// dropped quietly, so the attempt fails closed (M1).

/**
 * Every message type that carries world-derived data. This is the one list:
 * the transport's refusal and the tests both read it, and a type absent from
 * it is still refused when the message carries one of its fields.
 */
export const WORLD_DATA_TYPES = Object.freeze(['fleet', 'snapshot', 'run', 'overview', 'workspace', 'ask'])

/** A send outside `publish` that would carry world data: the transport refuses it. */
export class WorldDataRefused extends Error {
  constructor(type) {
    super(type === undefined || type === null
      ? 'a message that is not one stable typed JSON object may only be sent by publish'
      : `a “${type}” message carries world data and may only be sent by publish`)
    this.name = 'WorldDataRefused'
    this.type = type ?? null
  }
}

/** A peer registered with the poller a second time: the registration grants nothing. */
export class PeerRegistered extends Error {
  constructor() {
    super('a peer may be registered with the poller only once')
    this.name = 'PeerRegistered'
  }
}

/** Whether `message` is world data, by its type or by any world field it carries. */
const carriesWorldData = (message) => message !== null && typeof message === 'object' &&
  (WORLD_DATA_TYPES.includes(message.type) || WORLD_DATA_TYPES.some((field) => field in message))

/**
 * `message` as the one text the socket will write and the value that text
 * carries, judged from the serialized string rather than the live object. A
 * message that cannot be serialized, or that serializes to no object or no
 * string `type`, refuses, so nothing the guard cannot judge is written.
 */
function wireOf(message) {
  let text
  try {
    text = JSON.stringify(message)
  } catch {
    throw new WorldDataRefused(undefined)
  }
  let value
  try {
    value = JSON.parse(text)
  } catch {
    throw new WorldDataRefused(undefined)
  }
  if (value === null || typeof value !== 'object' || Array.isArray(value) || typeof value.type !== 'string') {
    throw new WorldDataRefused(undefined)
  }
  return { text, value }
}

/** The type `message` shows a second read, so one whose answer changes refuses instead of reaching the wire. */
function liveTypeOf(message) {
  try {
    return message?.type
  } catch {
    return undefined
  }
}

/** Each peer's guarded facade, so a later guard returns the same one and wraps no other sender. */
const facades = new WeakMap()

/**
 * Serialize `message` once, judge the `type` of that serialized string, and
 * write exactly those bytes through `write`: an unminted world-data type
 * refuses, the minted path permits it because `publish` minted it. A live
 * `type` that differs from the serialized one refuses too, so a getter cannot
 * show the check and the wire different answers.
 */
function judged(write, message, minted) {
  const wire = wireOf(message)
  if (!minted && carriesWorldData(wire.value)) throw new WorldDataRefused(wire.value.type)
  const live = liveTypeOf(message)
  if (live !== wire.value.type) throw new WorldDataRefused(WORLD_DATA_TYPES.includes(live) ? live : undefined)
  write(wire.text)
}

/**
 * Replace `peer.send` with the facade the rest of lib/ sees and return it.
 * `write` is the raw writer, handed over once by the poller's registration;
 * only the first guard uses it. A later guard of the same peer returns that
 * same facade, so re-guarding never grants a sender that bypasses the judgment.
 */
export function guardTransport(peer, write) {
  const held = facades.get(peer)
  if (held !== undefined) return held
  const record = typeof peer.send === 'function' ? peer.send.bind(peer) : null
  const sink = typeof write === 'function' ? write : record
  if (sink === null) throw new Error('a peer needs a writer to guard')
  const send = (message) => judged(sink, message, false)
  peer.send = send
  facades.set(peer, send)
  return send
}

/**
 * Install the facade on `peer` and return the one publish sender, built from
 * the raw writer at registration; only `publish` holds it.
 */
export function publisherFor(peer, write) {
  if (typeof write !== 'function') throw new Error('publish needs a peer’s writer')
  guardTransport(peer, write)
  return (message) => judged(write, message, true)
}
