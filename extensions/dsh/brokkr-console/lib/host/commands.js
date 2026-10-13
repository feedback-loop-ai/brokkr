// The console's half of a fenced command (design D4, decision 0029). Each
// press carries a command id and the run head the operator saw. The host
// re-reads the run before invoking anything: a moved head is answered "not
// sent" with the run as it is now, and a command the fresh view refuses is
// answered with Brokkr's own refusal text. A repeated command id gets the
// first answer. An answer lost to a timeout is reconciled from the run's
// record, by the command id the console writes into the recorded reason, and
// nothing is sent twice. Commands on one run are taken one at a time.
import { ReadoutRefused } from './readouts.js'

const COMMANDS = ['retry', 'stop', 'resume', 'conclude']

/** The most command ids the ledger remembers. */
const LEDGER_SIZE = 500

const ID = /^[A-Za-z0-9-]{8,128}$/

/** A command request the host will not take, with the text the console shows. */
export class RequestRefused extends Error {
  constructor(message) {
    super(message)
    this.name = 'RequestRefused'
  }
}

/** The suffix that ties a recorded reason to the console command that wrote it. */
function tagOf(commandId) {
  return ` (console command ${commandId})`
}

/** The request body, checked: every field the desk reads, of the kind it reads. */
function requestOf(body) {
  if (body === null || typeof body !== 'object') throw new RequestRefused('the command is not an object')
  const { command_id: id, realm, run_id: runId, command, reason, expected_seq: seq } = body
  if (typeof id !== 'string' || !ID.test(id)) throw new RequestRefused('the command carries no usable command id')
  if (typeof realm !== 'string' || typeof runId !== 'string') throw new RequestRefused('the command names no run')
  if (!COMMANDS.includes(command)) throw new RequestRefused('the command is not one Brokkr has')
  if (typeof reason !== 'string' || reason.trim() === '') throw new RequestRefused('a reason is required')
  if (!Number.isInteger(seq)) throw new RequestRefused('the command names no run head')
  return { id, realm, run_id: runId, command, reason: reason.trim(), expected_seq: seq }
}

/** What Brokkr printed when it refused, verbatim. */
function said(ran) {
  return ran.stderr.trim() || ran.stdout.trim()
}

/**
 * The command desk.
 * @param {{brokkr: object, resume: {kind: string}, locate: (realm: string, runId: string) => object | null}} parts
 *   `locate` names the run's journal from the latest fleet, or `null` for a run it does not list.
 */
export function deskOf({ brokkr, resume, locate }) {
  const ledger = new Map()
  const locks = new Map()

  /** `work`, after every earlier command on the same run. */
  function serialized(request, work) {
    const key = `${request.realm}\u0000${request.run_id}`
    const next = (locks.get(key) ?? Promise.resolve()).then(work, work)
    locks.set(key, next)
    return next
  }

  async function resumed(request, target, worktree, stage) {
    const started = await brokkr.resume(target, worktree, request.id)
    const base = { command: request.command, stage }
    switch (started.kind) {
      case 'running':
        return { ...base, outcome: 'done' }
      case 'exited':
        return started.code === 0
          ? { ...base, outcome: 'done' }
          : { ...base, outcome: 'refused', text: started.output.trim() }
      case 'unstarted':
        return { ...base, outcome: 'not_sent', why: 'unstarted', text: started.message }
    }
  }

  /** The command was accepted: a retry is followed by resume, the rest are done. */
  function accepted(request, target, worktree) {
    return request.command === 'retry'
      ? resumed(request, target, worktree, 'resume')
      : { command: request.command, outcome: 'done' }
  }

  /** The answer was lost: read the record, and send nothing again. */
  async function reconcile(request, target) {
    let view
    try {
      view = await brokkr.inspect(target)
    } catch (error) {
      if (error instanceof ReadoutRefused) return { command: request.command, outcome: 'checking' }
      throw error
    }
    const after = view.journal.filter((row) => row.seq > request.expected_seq)
    const tag = tagOf(request.id)
    if (after.some((row) => row.event_type === 'operator/accepted' && row.reason?.endsWith(tag))) {
      return accepted(request, target, view.worktree)
    }
    const rejected = after.find((row) => row.event_type === 'operator/rejected')
    if (rejected !== undefined) return { command: request.command, outcome: 'refused', text: rejected.what.text }
    return { command: request.command, outcome: 'not_sent', why: 'unconfirmed', view }
  }

  async function press(request, target, view) {
    if (request.command === 'resume') return resumed(request, target, view.worktree, null)
    const ran = await brokkr.command(target, request.command, request.reason + tagOf(request.id))
    switch (ran.kind) {
      case 'exited':
        return ran.code === 0
          ? accepted(request, target, view.worktree)
          : { command: request.command, outcome: 'refused', text: said(ran) }
      case 'unstarted':
        return { command: request.command, outcome: 'not_sent', why: 'unstarted', text: ran.message }
      case 'timeout':
      case 'overflow':
        return reconcile(request, target)
    }
  }

  async function decide(request) {
    const target = locate(request.realm, request.run_id)
    if (target === null) return { command: request.command, outcome: 'not_sent', why: 'unknown_run' }
    let view
    try {
      view = await brokkr.inspect(target)
    } catch (error) {
      if (!(error instanceof ReadoutRefused)) throw error
      return { command: request.command, outcome: 'not_sent', why: 'unreadable', text: error.message }
    }
    if (view.summary === null || view.summary.seq !== request.expected_seq) {
      return { command: request.command, outcome: 'not_sent', why: 'head_moved', view }
    }
    const offered = view.commands.find((entry) => entry.command === request.command)
    if (offered.admission === 'refused') return { command: request.command, outcome: 'refused', text: offered.text, view }
    const drives = request.command === 'retry' || request.command === 'resume'
    if (drives && resume.kind === 'none') {
      return { command: request.command, outcome: 'not_sent', why: 'resume_unconfigured', view }
    }
    return press(request, target, view)
  }

  function remember(request, work) {
    const entry = { request, outcome: null }
    entry.promise = serialized(request, work).then((outcome) => (entry.outcome = outcome))
    ledger.set(request.id, entry)
    if (ledger.size > LEDGER_SIZE) ledger.delete(ledger.keys().next().value)
    return entry.promise
  }

  return {
    /**
     * Take one command. Throws `RequestRefused` for a request the host will
     * not take; answers `done`, `not_sent`, `refused` or `checking` otherwise.
     */
    submit(body) {
      const request = requestOf(body)
      const first = ledger.get(request.id)
      if (first === undefined) return remember(request, () => decide(request))
      if (first.outcome?.outcome !== 'checking') return first.promise
      const target = locate(first.request.realm, first.request.run_id)
      if (target === null) return first.promise
      return remember(first.request, () => reconcile(first.request, target))
    },
  }
}
