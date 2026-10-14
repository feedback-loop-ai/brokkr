// Brokkr's readouts, each parsed once here into the shape the console paints
// (decision 0071 ruling 3). A readout pinned by `view_version` is refused at
// any version but the one this console was built against; a field it paints
// that is missing or of the wrong kind refuses the whole readout, so nothing
// half-read is painted. Fields the console does not paint are left unread.

/** The view wire this console paints: `brokkr_view::VIEW_VERSION`. */
export const VIEW_VERSION = 21

/** The Muninn record version this console reads. */
const MUNINN_RECORD_VERSION = 1

/** The label a one-hearth world's runs go under: its listing names no realm. */
export const ONE_HEARTH = 'this workspace'

const STANDINGS = ['quarantined', 'parked', 'running', 'shipped', 'stopped', 'operator_stopped']
const COMMANDS = ['retry', 'stop', 'resume', 'conclude']
const ACTIONS = ['dismiss_finding', 'talk_to_reviewer', 'hold', 'answer_here']
const ADMISSIONS = ['admissible', 'waiting', 'held']
const RANGES = ['24h', '7d', '30d']

/** A readout the console will not paint, with the operator's text. */
export class ReadoutRefused extends Error {
  /**
   * @param {'unstarted'|'timeout'|'overflow'|'failed'|'not_json'|'version'|'shape'} kind
   * @param {string} message
   */
  constructor(kind, message) {
    super(message)
    this.name = 'ReadoutRefused'
    this.kind = kind
  }
}

// ------------------------------------------------------------ field readers

function refuse(path, what) {
  throw new ReadoutRefused('shape', `${path} is not ${what}`)
}

function object(value, path) {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) refuse(path, 'an object')
  return value
}

function text(value, path) {
  return typeof value === 'string' ? value : refuse(path, 'text')
}

function count(value, path) {
  return Number.isFinite(value) ? value : refuse(path, 'a number')
}

function word(value, words, path) {
  return words.includes(value) ? value : refuse(path, `one of ${words.join(', ')}`)
}

function list(value, path, each) {
  if (!Array.isArray(value)) refuse(path, 'a list')
  return value.map((item, index) => each(item, `${path}[${index}]`))
}

/** `read(value)` when the value is present, `null` when it is null or absent. */
function maybe(value, path, read) {
  return value === null || value === undefined ? null : read(value, path)
}

const optText = (value, path) => maybe(value, path, text)
const texts = (value, path) => list(value, path, text)

// --------------------------------------------------------------- the edge

/** The text a finished call printed, parsed as JSON, or why it was not. */
export function jsonOf(ran, what) {
  switch (ran.kind) {
    case 'unstarted':
      throw new ReadoutRefused('unstarted', `brokkr could not be started for ${what}: ${ran.message}`)
    case 'timeout':
      throw new ReadoutRefused('timeout', `${what} took longer than the console waits`)
    case 'overflow':
      throw new ReadoutRefused('overflow', `${what} printed more than the console reads`)
    case 'exited':
      if (ran.code !== 0) throw new ReadoutRefused('failed', ran.stderr.trim() || `${what} failed`)
      try {
        return JSON.parse(ran.stdout)
      } catch {
        throw new ReadoutRefused('not_json', `${what} did not print JSON`)
      }
  }
}

function versioned(doc, what) {
  object(doc, what)
  if (doc.view_version !== VIEW_VERSION) {
    throw new ReadoutRefused(
      'version',
      `${what} is at view version ${doc.view_version}; this console paints version ${VIEW_VERSION} only`,
    )
  }
  return doc
}

// --------------------------------------------------------------- the fleet

function verdict(value, path) {
  object(value, path)
  return {
    standing: word(value.standing, STANDINGS, `${path}.standing`),
    rule: optText(value.rule, `${path}.rule`),
    reason: optText(value.reason, `${path}.reason`),
    text: text(value.text, `${path}.text`),
  }
}

function runRow(value, path) {
  object(value, path)
  return {
    run_id: text(value.run_id, `${path}.run_id`),
    title: text(value.title, `${path}.title`),
    feature: text(value.feature, `${path}.feature`),
    phase: optText(value.phase, `${path}.phase`),
    seq: maybe(value.seq, `${path}.seq`, count),
    created_at: text(value.created_at, `${path}.created_at`),
    last_recorded_at: optText(value.last_recorded_at, `${path}.last_recorded_at`),
    detail: optText(value.detail, `${path}.detail`),
    acknowledgement: maybe(value.acknowledgement, `${path}.acknowledgement`, acknowledgement),
    verdict: verdict(value.verdict, `${path}.verdict`),
  }
}

/** An operator's acknowledgement of a quarantined run (#511). */
function acknowledgement(value, path) {
  object(value, path)
  return {
    seq: count(value.seq, `${path}.seq`),
    operator: text(value.operator, `${path}.operator`),
    reason: text(value.reason, `${path}.reason`),
    recorded_at: text(value.recorded_at, `${path}.recorded_at`),
  }
}

/**
 * `brokkr runs --json`: one hearth's `RunsView` or a world's `FleetView`, as
 * the hearths the console lists.
 */
export function fleetOf(doc) {
  versioned(doc, 'brokkr runs')
  const rows = (value, path) => list(value, path, runRow)
  if (doc.realms === undefined) {
    return { hearths: [{ realm: ONE_HEARTH, journal: null, detail: null, runs: rows(doc.runs, 'runs') }] }
  }
  return {
    hearths: list(doc.realms, 'realms', (value, path) => ({
      realm: text(object(value, path).realm, `${path}.realm`),
      journal: text(value.journal, `${path}.journal`),
      detail: optText(value.detail, `${path}.detail`),
      runs: rows(value.runs, `${path}.runs`),
    })),
  }
}

// ---------------------------------------------------------------- one run

function offered(value, path) {
  object(value, path)
  const admission = word(value.admission, ['admitted', 'refused'], `${path}.admission`)
  return {
    command: word(value.command, COMMANDS, `${path}.command`),
    phase: optText(value.phase, `${path}.phase`),
    admission,
    text: admission === 'refused' ? text(value.text, `${path}.text`) : null,
  }
}

/** The reason an operator event recorded, read from its payload. */
function payloadReason(row, path) {
  if (!row.event_type.startsWith('operator/')) return null
  let payload
  try {
    payload = JSON.parse(text(row.payload_json, `${path}.payload_json`))
  } catch (error) {
    if (error instanceof ReadoutRefused) throw error
    refuse(`${path}.payload_json`, 'JSON')
  }
  return optText(object(payload, `${path}.payload_json`).reason, `${path}.payload_json.reason`)
}

function journalRow(value, path) {
  object(value, path)
  const what = object(value.what, `${path}.what`)
  const row = {
    seq: count(value.seq, `${path}.seq`),
    event_type: text(value.event_type, `${path}.event_type`),
    recorded_at: text(value.recorded_at, `${path}.recorded_at`),
    in_trail: value.in_trail === true,
    what: { text: text(what.text, `${path}.what.text`), badge: optText(what.badge, `${path}.what.badge`) },
  }
  return { ...row, reason: payloadReason({ ...row, payload_json: value.payload_json }, path) }
}

function phase(value, path) {
  object(value, path)
  return {
    name: text(value.name, `${path}.name`),
    visits: count(value.visits, `${path}.visits`),
    current: value.current === true,
    returns: texts(value.returns, `${path}.returns`),
  }
}

function visit(value, path) {
  object(value, path)
  return {
    phase: text(value.phase, `${path}.phase`),
    ruled: text(value.ruled, `${path}.ruled`),
    rule: optText(value.rule, `${path}.rule`),
    model: optText(value.model, `${path}.model`),
    duration: optText(value.duration, `${path}.duration`),
  }
}

function summary(value, path) {
  object(value, path)
  return {
    run_id: text(value.run_id, `${path}.run_id`),
    seq: count(value.seq, `${path}.seq`),
    status: text(value.status, `${path}.status`),
    phase: optText(value.phase, `${path}.phase`),
    park_reason: optText(value.park_reason, `${path}.park_reason`),
  }
}

/** `brokkr inspect --run <id> --json`: the run view, as the console paints it. */
export function runViewOf(doc) {
  versioned(doc, 'brokkr inspect')
  const ruling = maybe(doc.ruling, 'ruling', (value, path) => ({
    rule_id: text(object(value, path).rule_id, `${path}.rule_id`),
    from: text(value.from, `${path}.from`),
  }))
  return {
    summary: maybe(doc.summary, 'summary', summary),
    ruling,
    phases: list(doc.phases, 'phases', phase),
    path: list(object(doc.dashboard, 'dashboard').path, 'dashboard.path', visit),
    journal: list(doc.journal, 'journal', journalRow),
    commands: list(doc.commands, 'commands', offered),
    unavailable: list(doc.unavailable, 'unavailable', (value, path) => ({
      action: word(object(value, path).action, ACTIONS, `${path}.action`),
      reason: text(value.reason, `${path}.reason`),
    })),
    clarify: maybe(doc.clarify, 'clarify', (value, path) => ({
      seq: count(object(value, path).seq, `${path}.seq`),
      questions: texts(value.questions, `${path}.questions`),
    })),
    worktree: optText(doc.worktree, 'worktree'),
    recipe: optText(doc.recipe, 'recipe'),
  }
}

// --------------------------------------------------------------- the queue

function launchName(bundle, path) {
  object(bundle, path)
  if (typeof bundle.dir === 'string') return bundle.dir
  return text(object(bundle.recipe, `${path}.recipe`).name, `${path}.recipe.name`)
}

/** `brokkr queue list --json`: each entry and admission's word on it. */
export function queueOf(doc) {
  return list(doc, 'queue', (value, path) => {
    object(value, path)
    const launch = object(value.launch, `${path}.launch`)
    return {
      entry: count(value.entry, `${path}.entry`),
      state: text(value.state, `${path}.state`),
      added_at: text(value.added_at, `${path}.added_at`),
      feature: text(launch.feature, `${path}.launch.feature`),
      bundle: launchName(launch.bundle, `${path}.launch.bundle`),
      admission: maybe(value.admission, `${path}.admission`, (admission, at) => ({
        standing: word(object(admission, at).standing, ADMISSIONS, `${at}.standing`),
        says: list(admission.reasons, `${at}.reasons`, (reason, where) =>
          text(object(reason, where).says, `${where}.says`),
        ),
      })),
    }
  })
}

// ------------------------------------------------------------------ Muninn

/** `brokkr muninn list --json`: the last review, or `null` when none is recorded. */
export function muninnOf(doc) {
  const entries = list(doc, 'muninn', (value, path) => {
    object(value, path)
    if (value.record_version !== MUNINN_RECORD_VERSION) {
      throw new ReadoutRefused(
        'version',
        `Muninn's record is at version ${value.record_version}; this console reads version ${MUNINN_RECORD_VERSION} only`,
      )
    }
    return {
      recorded_at: text(value.recorded_at, `${path}.recorded_at`),
      summary: text(value.fleet_summary, `${path}.fleet_summary`),
      parked: list(value.parked_runs, `${path}.parked_runs`, (run) => run).length,
      queued: list(value.work_queue, `${path}.work_queue`, (item) => item).length,
    }
  })
  return entries.length === 0 ? null : entries[entries.length - 1]
}

// --------------------------------------------------------------- analytics

const span = (value, path) => ({ text: text(object(value, path).text, `${path}.text`) })

function headline(value, path) {
  object(value, path)
  const shipped = object(value.shipped, `${path}.shipped`)
  const waiting = object(value.waiting, `${path}.waiting`)
  const stopped = object(value.stopped_by_rule, `${path}.stopped_by_rule`)
  const cost = object(value.cost, `${path}.cost`)
  return {
    shipped: { count: count(shipped.count, `${path}.shipped.count`), change_text: text(shipped.change_text, `${path}.shipped.change_text`) },
    median: span(value.median, `${path}.median`),
    waiting: { text: text(waiting.text, `${path}.waiting.text`), share_text: text(waiting.share_text, `${path}.waiting.share_text`) },
    stopped_by_rule: { count: count(stopped.count, `${path}.stopped_by_rule.count`), text: text(stopped.text, `${path}.stopped_by_rule.text`) },
    cost: { text: text(cost.text, `${path}.cost.text`), note: optText(cost.note, `${path}.cost.note`) },
  }
}

function rework(value, path) {
  object(value, path)
  return {
    runs: count(value.runs, `${path}.runs`),
    implement_visits_text: text(value.implement_visits_text, `${path}.implement_visits_text`),
    sent_back: count(value.sent_back, `${path}.sent_back`),
    second_try_text: text(value.second_try_text, `${path}.second_try_text`),
  }
}

/** `brokkr analytics --json`: the figures, each with the text Brokkr renders it in. */
export function analyticsOf(doc) {
  versioned(doc, 'brokkr analytics')
  return {
    range: word(doc.range, RANGES, 'range'),
    realm: optText(doc.realm, 'realm'),
    headline: headline(doc.headline, 'headline'),
    per_day: list(doc.per_day, 'per_day', (value, path) => ({
      date: text(object(value, path).date, `${path}.date`),
      shipped: count(value.shipped, `${path}.shipped`),
      stopped: count(value.stopped, `${path}.stopped`),
    })),
    waiting_by_step: list(doc.waiting_by_step, 'waiting_by_step', (value, path) => ({
      phase: optText(object(value, path).phase, `${path}.phase`),
      waiting_millis: count(value.waiting_millis, `${path}.waiting_millis`),
      text: text(value.text, `${path}.text`),
    })),
    rework: rework(doc.rework, 'rework'),
    models: list(doc.models, 'models', (value, path) => ({
      model: optText(object(value, path).model, `${path}.model`),
      percent: count(value.percent, `${path}.percent`),
      text: text(value.text, `${path}.text`),
    })),
    realms: list(doc.realms, 'realms', (value, path) => ({
      realm: text(object(value, path).realm, `${path}.realm`),
      shipped: count(value.shipped, `${path}.shipped`),
      stopped: count(value.stopped, `${path}.stopped`),
      median: span(value.median, `${path}.median`),
      waiting_text: text(value.waiting_text, `${path}.waiting_text`),
      cost_text: text(value.cost_text, `${path}.cost_text`),
    })),
    quarantined: list(doc.quarantined, 'quarantined', (value, path) => ({
      realm: text(object(value, path).realm, `${path}.realm`),
      refusal: text(value.refusal, `${path}.refusal`),
    })),
    not_recorded: texts(doc.not_recorded, 'not_recorded'),
  }
}

/** The range words `brokkr analytics --range` takes. */
export function isRange(value) {
  return RANGES.includes(value)
}
