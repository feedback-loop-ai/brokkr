// The transport's guard: only `publish` may send world data. The poller keeps
// each client's raw send private and replaces it with the facade below, so no
// spelling of a send outside `publish` can carry a fleet, snapshot, run,
// overview, workspace or ask. A refused send throws by name; it is never
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
      ? 'a message carrying world data may only be sent by publish'
      : `a “${type}” message carries world data and may only be sent by publish`)
    this.name = 'WorldDataRefused'
    this.type = type ?? null
  }
}

/** Whether `message` is world data, by its type or by any world field it carries. */
const carriesWorldData = (message) => message !== null && typeof message === 'object' &&
  (WORLD_DATA_TYPES.includes(message.type) || WORLD_DATA_TYPES.some((field) => field in message))

/**
 * Replace `peer.send` with the facade the rest of lib/ sees and hand back the
 * raw send, kept private to `publish`. The facade refuses every world-data
 * message by name and passes the error helper's own message, which carries no
 * world data, through unchanged.
 */
export function guardTransport(peer) {
  const raw = peer.send.bind(peer)
  peer.send = (message) => {
    if (carriesWorldData(message)) throw new WorldDataRefused(message.type)
    raw(message)
  }
  return raw
}
