// Brokkr's readouts, each parsed once here into the shape the console paints
// (decision 0071 ruling 3). A readout pinned by `view_version` is refused at
// any version but the one this console was built against; a field it paints
// that is missing or of the wrong kind refuses the whole readout, so nothing
// half-read is painted. Fields the console does not paint are left unread.

/** The view wire this console paints: `brokkr_view::VIEW_VERSION`. */
export const VIEW_VERSION = 28

/** The graph wire this console paints: `brokkr_view::GRAPH_VIEW_VERSION`, pinned apart from `VIEW_VERSION`. */
export const GRAPH_VIEW_VERSION = 2

/** The Muninn record version this console reads. */
const MUNINN_RECORD_VERSION = 1

/** The label a one-hearth world's runs go under: its listing names no realm. */
export const ONE_HEARTH = 'this workspace'

const STANDINGS = ['quarantined', 'parked', 'running', 'stopping', 'shipped', 'stopped', 'operator_stopped']
const COMMANDS = ['retry', 'stop', 'resume', 'conclude']
const ACTIONS = ['dismiss_finding', 'talk_to_reviewer', 'hold', 'answer_here']
const ADMISSIONS = ['admissible', 'waiting', 'held']
const RANGES = ['24h', '7d', '30d']
const COSTS = ['reported', 'not_reported']
const PLACES = ['row', 'end', 'beneath']
const RULE_KINDS = ['forward', 'self', 'back', 'terminal']
const TABLES = ['resolved', 'not_found', 'refused']

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

/**
 * A failed readout's own words, without the `note:` lines Brokkr prints
 * about how it found the workspace's map, which are not the refusal.
 */
const refusalOf = (stderr) => stderr.split('\n').filter((line) => !line.startsWith('note: ')).join('\n').trim()

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
      if (ran.code !== 0) throw new ReadoutRefused('failed', refusalOf(ran.stderr) || `${what} failed`)
      try {
        return JSON.parse(ran.stdout)
      } catch {
        throw new ReadoutRefused('not_json', `${what} did not print JSON`)
      }
  }
}

/** `doc`, refused unless it is at `version`: the shared view wire, or a readout's own (the graph's). */
function versioned(doc, what, version = VIEW_VERSION) {
  object(doc, what)
  if (doc.view_version !== version) {
    throw new ReadoutRefused(
      'version',
      `${what} is at view version ${doc.view_version}; this console paints version ${version} only`,
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
    detail: optText(value.detail, `${path}.detail`),
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

/** `brokkr realms --json`: each realm's name and its folder, as the map names it (relative to the workspace). */
export function realmsOf(doc) {
  object(doc, 'realms')
  return list(doc.realms, 'realms', (value, path) => ({
    name: text(object(value, path).name, `${path}.name`),
    path: text(value.path, `${path}.path`),
  }))
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

/** A journal row's payload, the JSON object Brokkr wrote in `payload_json`, parsed once; its path for the fields read from it. */
function payloadOf(row, path) {
  const at = `${path}.payload_json`
  const written = text(row.payload_json, at)
  let payload
  try {
    payload = JSON.parse(written)
  } catch {
    refuse(at, 'JSON')
  }
  return [object(payload, at), at]
}

/**
 * What a ruling decided, from the fields its payload records: the phase it
 * ruled, the one it moved the run to, its rule, result and severity. `null`
 * for any other event.
 */
function rulingOf(row, path) {
  if (row.event_type !== 'transition/decided') return null
  const [payload, at] = payloadOf(row, path)
  return {
    from: optText(payload.from, `${at}.from`),
    next: optText(payload.next, `${at}.next`),
    rule: optText(payload.rule_id, `${at}.rule_id`),
    result: optText(payload.result, `${at}.result`),
    severity: optText(payload.severity, `${at}.severity`),
  }
}

/** The reason an operator event recorded, read from its payload. */
function payloadReason(row, path) {
  if (!row.event_type.startsWith('operator/')) return null
  const [payload, at] = payloadOf(row, path)
  return optText(payload.reason, `${at}.reason`)
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
  const raw = { ...row, payload_json: value.payload_json }
  return { ...row, reason: payloadReason(raw, path), ruling: rulingOf(raw, path) }
}

/** One seat or member the graph draws: its label, Brokkr's state word and class, and the model it served. */
function node(value, path) {
  object(value, path)
  return {
    key: text(value.key, `${path}.key`),
    label: text(value.label, `${path}.label`),
    state: text(value.state, `${path}.state`),
    state_class: text(value.state_class, `${path}.state_class`),
    model: maybe(value.model, `${path}.model`, (model, at) => optText(object(model, at).text, `${at}.text`)),
  }
}

/** A phase's steps in order; the nodes of one column ran side by side. */
function column(value, path) {
  object(value, path)
  return { label: optText(value.label, `${path}.label`), nodes: list(value.nodes, `${path}.nodes`, node) }
}

function phase(value, path) {
  object(value, path)
  return {
    name: text(value.name, `${path}.name`),
    strategy: optText(value.strategy, `${path}.strategy`),
    visits: count(value.visits, `${path}.visits`),
    current: value.current === true,
    columns: list(value.columns, `${path}.columns`, column),
    returns: list(value.returns, `${path}.returns`, road),
  }
}

/** A road back into a phase: the phase that sent the run back, how often, and the rules that did, the latest last. */
function road(value, path) {
  object(value, path)
  return { from: text(value.from, `${path}.from`), taken: count(value.taken, `${path}.taken`), rules: texts(value.rules, `${path}.rules`) }
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

/** A cell as Brokkr words it: its text, and why it is absent when it is. */
function cell(value, path) {
  object(value, path)
  return { text: text(value.text, `${path}.text`), absent: value.absent === true, note: optText(value.note, `${path}.note`) }
}

/** Tokens read apart as Brokkr gives them to every surface: total, in, out and the cache hit, each its own cell. */
function tokenCells(value, path) {
  object(value, path)
  return {
    total: cell(value.total, `${path}.total`),
    input: cell(value.input, `${path}.input`),
    output: cell(value.output, `${path}.output`),
    cache_hit: cell(value.cache_hit, `${path}.cache_hit`),
  }
}

/**
 * One seat of the run, with the figures the TUI's seat pane prints, each
 * Brokkr's own cell: model (claimed) and boundary, effort pinned and
 * applied, attempts, turns, cost, and tokens read apart (total, in, out,
 * cache hit).
 */
function participant(value, path) {
  object(value, path)
  return {
    key: text(value.key, `${path}.key`),
    label: text(value.label, `${path}.label`),
    member: optText(value.member, `${path}.member`),
    phase: optText(value.phase, `${path}.phase`),
    // Which visit to its phase the seat was asked in, from 1, as Brokkr counted it.
    visit: maybe(value.visit, `${path}.visit`, count),
    status: text(value.status, `${path}.status`),
    carried_from: maybe(value.carried_from, `${path}.carried_from`, (source, at) => ({
      attempt_id: text(object(source, at).attempt_id, `${at}.attempt_id`),
      ordinal: count(source.ordinal, `${at}.ordinal`),
    })),
    working: value.working === true,
    attempts: count(value.attempts, `${path}.attempts`),
    turns: cell(value.turns_cell, `${path}.turns_cell`),
    cost: cell(value.cost_cell, `${path}.cost_cell`),
    usage: cell(value.usage_cell, `${path}.usage_cell`),
    tokens: tokenCells(value.tokens, `${path}.tokens`),
    model: cell(value.model, `${path}.model`),
    boundary: cell(value.boundary, `${path}.boundary`),
    effort: cell(value.effort, `${path}.effort`),
    effort_pin: cell(value.effort_pin, `${path}.effort_pin`),
    activity: cell(value.activity, `${path}.activity`),
    transcript: cell(value.transcript_cell, `${path}.transcript_cell`),
    // What the plan selected for the seat, beside what the harness reported serving: brokkr-view's
    // one provenance sentence (the TUI's "selected by" line), carried whole so no surface drops
    // the "not the agent's first choice" half (decision 0071 ruling 7).
    selected: maybe(value.provenance, `${path}.provenance`, (provenance, at) => text(object(provenance, at).line, `${at}.line`)),
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
    participants: list(doc.participants, 'participants', participant),
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
    entry: maybe(doc.entry, 'entry', (value, path) => ({ text: text(object(value, path).text, `${path}.text`) })),
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

/**
 * A cost as Brokkr figures it: its dollars when any seat reported them, its
 * text, and the providers whose seats reported none (`silent`, with Brokkr's
 * note naming them).
 */
function costFigure(value, path) {
  object(value, path)
  const kind = word(object(value.cost, `${path}.cost`).kind, COSTS, `${path}.cost.kind`)
  return {
    cost: kind === 'reported' ? { kind, usd: count(value.cost.usd, `${path}.cost.usd`) } : { kind },
    text: text(value.text, `${path}.text`),
    silent: texts(value.silent, `${path}.silent`),
    note: optText(value.note, `${path}.note`),
  }
}

/** One recipe of one realm over the range (design D6); `on_frontier` is Brokkr's reading, false where it gave none. */
function recipeRow(value, path) {
  object(value, path)
  return {
    realm: text(value.realm, `${path}.realm`),
    recipe: text(value.recipe, `${path}.recipe`),
    runs: count(value.runs, `${path}.runs`),
    done: count(value.done, `${path}.done`),
    done_rate: count(value.done_rate, `${path}.done_rate`),
    done_text: text(value.done_text, `${path}.done_text`),
    cost_per_run: costFigure(value.cost_per_run, `${path}.cost_per_run`),
    tokens_per_run: tokenCells(value.tokens_per_run, `${path}.tokens_per_run`),
    on_frontier: value.on_frontier === true,
  }
}

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
    recipes: list(doc.recipes, 'recipes', recipeRow),
  }
}

/** The range words `brokkr analytics --range` and `brokkr graph --range` take. */
export function isRange(value) {
  return RANGES.includes(value)
}

// ------------------------------------------------------------------ graph

/** A state of the recipe, where the view places it, and its figures over the range, each in Brokkr's words (design D4). */
function graphState(value, path) {
  object(value, path)
  return {
    name: text(value.name, `${path}.name`),
    place: word(value.place, PLACES, `${path}.place`),
    entered: count(value.entered, `${path}.entered`),
    median: span(value.median, `${path}.median`),
    p90: span(value.p90, `${path}.p90`),
    cost: costFigure(value.cost, `${path}.cost`),
    tokens: tokenCells(value.tokens, `${path}.tokens`),
    seats: list(value.seats, `${path}.seats`, (seat, at) => ({
      key: text(object(seat, at).key, `${at}.key`),
      text: text(seat.text, `${at}.text`),
    })),
  }
}

/** A rule of the table, or one that fired where no table resolved: its edge, its count and Brokkr's `<ID> · <count>` (design D3). */
function graphRule(value, path) {
  object(value, path)
  return {
    id: text(value.id, `${path}.id`),
    from: text(value.from, `${path}.from`),
    next: optText(value.next, `${path}.next`),
    result: optText(value.result, `${path}.result`),
    when: texts(value.when, `${path}.when`),
    severity: optText(value.severity, `${path}.severity`),
    count: count(value.count, `${path}.count`),
    kind: word(value.kind, RULE_KINDS, `${path}.kind`),
    text: text(value.text, `${path}.text`),
    latest: list(value.latest, `${path}.latest`, (run, at) => ({
      run_id: text(object(run, at).run_id, `${at}.run_id`),
      label: text(run.label, `${at}.label`),
      at: text(run.at, `${at}.at`),
    })),
  }
}

/** The rules joining one state to one next state, as one link: the runs it moved, how many rules it joins, and each result that fired with its runs (design D3). */
function graphLink(value, path) {
  object(value, path)
  return {
    from: text(value.from, `${path}.from`),
    next: text(value.next, `${path}.next`),
    kind: word(value.kind, RULE_KINDS, `${path}.kind`),
    count: count(value.count, `${path}.count`),
    rules: count(value.rules, `${path}.rules`),
    outcomes: list(value.outcomes, `${path}.outcomes`, (outcome, at) => ({
      result: text(object(outcome, at).result, `${at}.result`),
      count: count(outcome.count, `${at}.count`),
      text: text(outcome.text, `${at}.text`),
    })),
    text: typeof value.text === 'string' ? value.text : text(value.text, `${path}.text`),
  }
}

/**
 * Whether the recipe's table resolved: Brokkr's words when it was not found,
 * and when it does not compile against this Brokkr, the compiler's refusal
 * and the bundle directory it refused.
 */
function graphTable(value, path) {
  const kind = word(object(value, path).kind, TABLES, `${path}.kind`)
  const refused = kind === 'refused'
  return {
    kind,
    text: kind === 'resolved' ? null : text(value.text, `${path}.text`),
    reason: refused ? text(value.reason, `${path}.reason`) : null,
    bundle: refused ? text(value.bundle, `${path}.bundle`) : null,
  }
}

/**
 * `brokkr graph --recipe <name> --json`: the recipe's state machine with the
 * journal's traffic over the range, at the graph's own wire version. The
 * client places it by `place` and `kind` and computes nothing of it.
 * `does_not_compile` names the realm's recipes that do not compile against
 * this Brokkr, so the picker can mark them.
 */
export function graphOf(doc) {
  versioned(doc, 'brokkr graph', GRAPH_VIEW_VERSION)
  return {
    realm: optText(doc.realm, 'realm'),
    recipe: text(doc.recipe, 'recipe'),
    range: word(doc.range, RANGES, 'range'),
    recipes: texts(doc.recipes, 'recipes'),
    does_not_compile: texts(doc.does_not_compile, 'does_not_compile'),
    table: graphTable(doc.table, 'table'),
    runs: count(doc.runs, 'runs'),
    states: list(doc.states, 'states', graphState),
    rules: list(doc.rules, 'rules', graphRule),
    links: list(doc.links, 'links', graphLink),
    never_fired: span(doc.never_fired, 'never_fired'),
    retired: list(doc.retired, 'retired', (value, path) => ({
      rule_id: text(object(value, path).rule_id, `${path}.rule_id`),
      count: count(value.count, `${path}.count`),
    })),
    drift_text: optText(doc.drift_text, 'drift_text'),
    ends: span(doc.ends, 'ends'),
    live: list(doc.live, 'live', (value, path) => ({
      run_id: text(object(value, path).run_id, `${path}.run_id`),
      label: text(value.label, `${path}.label`),
      state: optText(value.state, `${path}.state`),
    })),
  }
}

// ------------------------------------------------------------- transcripts

const TRANSCRIPT_SCHEMA = 'brokkr.transcript/v1'
/** The most of a transcript the console paints: its last turns, each block cut to a bound. */
const TRANSCRIPT_TURNS = 80
const BLOCK_CHARS = 4000

/** `stdout` parsed as a JSON object, or `null` when it is not one. */
function objectOf(stdout) {
  try {
    const value = JSON.parse(stdout)
    return value !== null && typeof value === 'object' && !Array.isArray(value) ? value : null
  } catch {
    return null
  }
}

/**
 * `brokkr transcript --json` as it ran. The CLI prints the document on stdout
 * in both outcomes; for a seat that keeps no transcript it exits non-zero with
 * the document's `unavailable` naming why, and the sanitized explanation on
 * stderr. The document decides that this is the seat's answer and not the
 * console failing; stderr's words are what the operator reads.
 */
export function transcriptReadOf(ran, what) {
  if (ran.kind === 'exited' && ran.code !== 0) {
    const doc = objectOf(ran.stdout)
    if (doc !== null && typeof doc.unavailable === 'string') return transcriptOf(doc, refusalOf(ran.stderr) || null)
  }
  return transcriptOf(jsonOf(ran, what))
}

/**
 * `brokkr transcript --json`: one seat's retained turns, newest last, as
 * Brokkr's bounded derivation reads them; its last turns only, each block cut
 * and marked when long. `unavailable` is Brokkr's word when there is none:
 * the `explanation` it printed when it gave one, else the reason it typed.
 */
export function transcriptOf(doc, explanation = null) {
  object(doc, 'brokkr transcript')
  if (doc.schema !== TRANSCRIPT_SCHEMA) {
    throw new ReadoutRefused('version', `brokkr transcript is at ${doc.schema}; this console reads ${TRANSCRIPT_SCHEMA} only`)
  }
  const turns = list(doc.turns ?? [], 'turns', (value, path) => ({
    role: text(object(value, path).role, `${path}.role`),
    ts: optText(value.ts, `${path}.ts`),
    blocks: list(value.blocks, `${path}.blocks`, (block, at) => {
      const words = text(object(block, at).text, `${at}.text`)
      return { kind: text(block.kind, `${at}.kind`), text: words.length > BLOCK_CHARS ? `${words.slice(0, BLOCK_CHARS)}… (cut; the seat's transcript has the rest)` : words }
    }),
  }))
  return {
    seat: text(doc.seat, 'seat'),
    turns: turns.slice(-TRANSCRIPT_TURNS),
    earlier: Math.max(0, turns.length - TRANSCRIPT_TURNS),
    truncated: doc.truncated === true,
    unavailable: maybe(doc.unavailable, 'unavailable', (reason, path) => explanation ?? text(reason, path)),
    full_session: optText(doc.full_session, 'full_session'),
  }
}
