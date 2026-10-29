// The builders every test shares: Brokkr's readouts as its `--json` prints
// them, a stub `brokkr` binary that answers from a script, and a stub DSH host.
import { chmodSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { Readable } from 'node:stream'

const STUB = new URL('./stub-brokkr.mjs', import.meta.url).pathname

// ------------------------------------------------------------- readouts

/** One `RunRow` as `brokkr runs --json` prints it: parked at review unless told otherwise. */
export function row(over = {}) {
  return {
    run_id: 'encrypt-the-token-store-41d0a9c2',
    status: 'awaiting_operator',
    status_known: true,
    phase: 'review',
    seq: 12,
    created_at: '2026-10-13T08:00:00Z',
    last_recorded_at: '2026-10-13T09:00:00Z',
    feature: 'Encrypt the token store\n\nAt rest, owner-only.',
    detail: null,
    quarantine: null,
    acknowledgement: null,
    residuals: [],
    title: 'Encrypt the token store',
    verdict: {
      standing: 'parked',
      rule: 'REVIEW-SECURITY-RESIDUAL',
      residual: 'serious',
      reason: 'security review left a serious residual finding: token file is readable by every user',
      text: 'SECURITY-RESI…',
    },
    hire: null,
    ...over,
  }
}

/** A world's `FleetView`: `[[realm, rows], ...]`. */
export function fleetDoc(realms, over = {}) {
  return {
    view_version: 28,
    realms: realms.map(([realm, runs]) => ({ realm, journal: `/hearths/${realm}/forge.db`, runs, count: runs.length, detail: null })),
    count: realms.reduce((sum, [, runs]) => sum + runs.length, 0),
    ...over,
  }
}

/** One offered command as `RunView.commands` carries it. */
export function offered(command, admission = 'admitted', extra = {}) {
  const refused = admission === 'refused' ? { refusal: 'awaiting_operator', text: `refused operator ${command}` } : {}
  return { command, ...(command === 'retry' ? { phase: 'review' } : {}), admission, ...refused, ...extra }
}

/** One journal row; `payload` is what its `payload_json` holds. */
export function journalRow(seq, eventType, payload = {}, text = eventType) {
  return {
    seq,
    causation_seq: null,
    event_type: eventType,
    recorded_at: '2026-10-13T09:00:00Z',
    in_trail: true,
    phases: [],
    what: { text, badge: null, badge_class: null, arrow: null, problem: null },
    label: { text: '', absent: true, note: null },
    model: { text: '—', absent: true, note: null },
    boundary: { text: '—', absent: true, note: null },
    payload_json: JSON.stringify(payload),
  }
}

const cellOf = (text, note = null) => (text === null ? { text: '—', absent: true, note } : { text, absent: false, note: null })

/** A seat's plan provenance as `brokkr inspect --json` prints it: the security agent's first choice, served through dsh, unless told otherwise. */
export function provenance(over = {}) {
  return { agent: 'security', model: 'glm-flash', effort: 'high', provider: 'dsh', chain_index: 0, fallback: false, line: 'security · selected glm-flash via dsh', ...over }
}

/** One participant as `brokkr inspect --json` prints it: the security review seat, finished, unless told otherwise. */
export function seat(over = {}) {
  return {
    key: 'seat-security-1',
    label: 'security',
    member: null,
    phase: 'review',
    visit: 1,
    status: 'succeeded',
    status_class: 'completed',
    working: false,
    attempts: 1,
    turns: 54,
    turns_aggregated: false,
    turns_cell: cellOf('54'),
    cost: 2.41794575,
    last_attempt_cost: 2.41794575,
    cost_aggregated: false,
    cost_cell: cellOf('$2.4179'),
    usage: { input_tokens: 3004371, output_tokens: 1181, cache_read_tokens: 3002733, cache_write_tokens: 267151, reasoning_output_tokens: null, total_tokens: 3005552 },
    usage_aggregated: false,
    usage_cell: cellOf('3.01M tok · in 3.00M tok · out 1k tok · cache read 3.00M tok · cache write 267k tok'),
    tokens: { total: cellOf('3.01M tok'), input: cellOf('3.00M'), output: cellOf('1.2k'), cache_hit: cellOf('99.9%') },
    model: cellOf('gpt-5.5'),
    boundary: cellOf('not applicable'),
    effort: cellOf('high'),
    effort_pin: cellOf(null, 'no effort pinned by this run’s resolution'),
    activity: { ...cellOf(null, 'no activity recorded'), tool: null, target_short: null, target_full: null },
    member_count: 0,
    transcript: null,
    transcript_cell: cellOf('claude-session'),
    session_id: null,
    terminal_line: cellOf(null),
    checkpoints: [],
    provenance: null,
    ...over,
  }
}

/** A `RunView` as `brokkr inspect --json` prints it: parked at review, head `seq`. */
export function runViewDoc(over = {}) {
  const { seq = 12, ...rest } = over
  return {
    view_version: 28,
    summary: {
      consecutive_failures: {},
      cursor: 'park',
      feature: 'Encrypt the token store',
      last_decision: null,
      park_reason: 'security review left a serious residual finding: token file is readable by every user',
      phase: 'review',
      run_id: 'encrypt-the-token-store-41d0a9c2',
      seq,
      status: 'awaiting_operator',
      standing: 'parked',
      detail: null,
      strategy: null,
    },
    ruling: { rule_id: 'REVIEW-SECURITY-RESIDUAL', severity_class: 'awaiting_operator', from: 'review', next: '', result: null, inputs: [], problem: null, superseded: null },
    participants: [],
    live: [],
    phases: [{ name: 'implement', strategy: null, visits: 2, current: false, plain: true, columns: [], returns: [{ from: 'review', taken: 1, rules: ['REVIEW-REFORGE'] }] }],
    notices: [],
    boundary: { word: { text: '—', absent: true, note: null }, unboxed: false, text: '' },
    journal: [journalRow(seq, 'transition/decided', {}, 'review parked the run')],
    event_count: seq,
    dashboard: { run_id: 'encrypt-the-token-store-41d0a9c2', path: [{ phase: 'review', ruled: 'parked', rule: 'REVIEW-SECURITY-RESIDUAL', residual: 'serious', model: 'gpt-5.5', duration: '12m' }], decision: null, last_seat: null, last_review: null },
    scopes: null,
    commands: [offered('retry'), offered('stop'), offered('resume', 'refused'), offered('conclude')],
    unavailable: [
      { action: 'dismiss_finding', reason: 'Brokkr only lets you dismiss findings once a run has finished.' },
      { action: 'talk_to_reviewer', reason: 'Reviews start fresh each time; your note can’t reach the reviewer yet.' },
      { action: 'hold', reason: 'A parked run already waits for you; it holds nothing else back.' },
    ],
    clarify: null,
    worktree: '/work/brokkr',
    recipe: 'triage',
    ...rest,
  }
}

/** A `CostFigure` as Brokkr prints it: `usd` reported by Claude's seats with Codex's silent, or not reported at all when `usd` is null. */
export function costFigure(usd, over = {}) {
  const figure = usd === null
    ? { cost: { kind: 'not_reported' }, text: 'not reported', reporting: [], silent: ['dsh'], note: 'dsh reports none' }
    : { cost: { kind: 'reported', usd }, text: `$${usd.toFixed(2)}`, reporting: ['claude'], silent: ['codex'], note: 'claude only · codex report none' }
  return { ...figure, ...over }
}

/** One realm's row of `brokkr analytics --json`: lanetally's, unless told otherwise. */
export function realmRow(realm = 'lanetally', over = {}) {
  return { realm, shipped: 3, stopped: 1, median: { millis: 15_120_000, text: '4h 12m' }, waiting_millis: 3_600_000, waiting_text: '1.0h', cost: { kind: 'not_reported' }, cost_text: 'not reported', unreadable_runs: 0, ...over }
}

/**
 * One `recipes[]` row of `brokkr analytics --json` (design D6): lanetally's
 * `self`, 3 of 4 runs done at $2.00 a run, on the frontier, unless told
 * otherwise; `usd: null` is a recipe whose seats reported no cost.
 */
export function recipeRow(over = {}) {
  const { usd = 2, ...rest } = over
  return {
    realm: 'lanetally',
    recipe: 'self',
    runs: 4,
    done: 3,
    done_rate: 75,
    done_text: '75%',
    cost_per_run: costFigure(usd),
    tokens_per_run: { total: cellOf('1.20M tok'), input: cellOf('1.18M'), output: cellOf('20k'), cache_hit: cellOf('95.0%') },
    on_frontier: usd === null ? null : true,
    ...rest,
  }
}

/** `brokkr analytics --json` for one realm whose seats reported no cost. */
export function analyticsDoc(over = {}) {
  return {
    view_version: 28,
    range: '7d',
    realm: null,
    from: '2026-10-06T09:00:00Z',
    to: '2026-10-13T09:00:00Z',
    headline: {
      shipped: { count: 3, previous: 1, change: 2, change_text: '▲ 2 vs previous 7d' },
      median: { millis: 15_120_000, text: '4h 12m' },
      waiting: { waiting_millis: 3_600_000, parked_millis: 7_200_000, text: '1.0h', share_percent: 50, share_text: '50% of all parked time' },
      stopped_by_rule: { count: 1, top: { rule_id: 'ARCH-FAIL-TWICE', count: 1 }, text: '1 by ARCH-FAIL-TWICE' },
      cost: { cost: { kind: 'not_reported' }, text: 'not reported', reporting: [], silent: ['dsh'], note: 'dsh reports none' },
    },
    per_day: [{ date: '2026-10-12', shipped: 2, stopped: 1 }, { date: '2026-10-13', shipped: 1, stopped: 0 }],
    waiting_by_step: [{ phase: 'review', waiting_millis: 3_600_000, runs: 1, text: '1.0h · 1 run' }],
    rework: { runs: 3, implement_visits: 1.3, implement_visits_text: '1.3', sent_back: 1, second_try_passed: 1, second_try_of: 1, second_try_percent: 100, second_try_text: '100%' },
    models: [
      { model: 'gpt-5.5', attempts: 3, percent: 75, text: 'gpt-5.5 75%' },
      { model: null, attempts: 1, percent: 25, text: 'not reported 25%' },
    ],
    realms: [realmRow()],
    quarantined: [],
    not_recorded: ['machine capacity over time', 'cost limits', 'costs for Codex and dsh', 'reasons findings were dismissed'],
    recipes: [recipeRow()],
    ...over,
  }
}

/** One state of a `GraphView`: a row state with one Claude seat and no cost reported, or a terminal with no seat, unless told otherwise. */
export function graphState(name, place = 'row', over = {}) {
  const row = place === 'row'
  return {
    name,
    place,
    entered: row ? 5 : 0,
    median: row ? { millis: 600_000, text: '10m' } : { millis: null, text: '—' },
    p90: row ? { millis: 1_200_000, text: '20m' } : { millis: null, text: '—' },
    cost: costFigure(null),
    tokens: { total: cellOf(null), input: cellOf(null), output: cellOf(null), cache_hit: cellOf(null) },
    seats: row ? [{ key: `${name}:main`, provider: 'claude', model: 'sonnet', text: `${name}:main · claude sonnet` }] : [],
    ...over,
  }
}

/** One rule of a `GraphView`, its text Brokkr's `<ID> · <count>`. */
export function graphRule(id, from, next, kind, count, over = {}) {
  return { id, from, next, result: 'ok', when: [], severity: null, count, kind, text: `${id} · ${count}`, latest: [], ...over }
}

/**
 * The links Brokkr draws from `rules`: one per state and next state in the
 * order its first rule holds, each result that fired summed, the most runs
 * first; a park has none. The fixture's own derivation, so a graph document
 * holds both as Brokkr prints them.
 */
export function linksOf(rules) {
  const links = []
  for (const rule of rules) {
    if (rule.next === null) continue
    let link = links.find((each) => each.from === rule.from && each.next === rule.next)
    if (link === undefined) links.push((link = { from: rule.from, next: rule.next, kind: rule.kind, count: 0, rules: 0, outcomes: [], text: '' }))
    link.count += rule.count
    link.rules += 1
    if (rule.count === 0) continue
    const outcome = link.outcomes.find((each) => each.result === rule.result)
    if (outcome === undefined) link.outcomes.push({ result: rule.result, count: rule.count, text: '' })
    else outcome.count += rule.count
  }
  for (const link of links) {
    link.outcomes.sort((a, b) => b.count - a.count)
    for (const outcome of link.outcomes) outcome.text = `${outcome.result} ${outcome.count}`
    link.text = link.outcomes.map((outcome) => outcome.text).join(' · ')
  }
  return links
}

/**
 * `brokkr graph --recipe self --realm brokkr --json`: intake, implement,
 * verify, review and ship in a row with done at its end and stop beneath; a
 * self-loop at implement, a back edge from review and a dormant one, forward
 * edges, a terminal into done and two into stop, a dormant rule into stop, a
 * park rule, a live run in verify and one in review, a retired rule and a
 * drift note. Synthetic figures only.
 */
export function graphDoc(over = {}) {
  const seat = (key, provider, model) => ({ key, provider, model, text: `${key} · ${provider} ${model}` })
  const doc = {
    view_version: 2,
    realm: 'brokkr',
    recipe: 'self',
    range: '7d',
    from: '2026-10-06T09:00:00Z',
    to: '2026-10-13T09:00:00Z',
    recipes: ['charter', 'frontier', 'lean', 'leaky', 'muninn', 'self'],
    does_not_compile: [],
    table: { kind: 'resolved' },
    runs: 10,
    states: [
      graphState('intake'),
      graphState('implement'),
      graphState('verify', 'row', { seats: [] }),
      graphState('review', 'row', {
        entered: 4,
        median: { millis: 1_800_000, text: '30m' },
        p90: { millis: 3_600_000, text: '1h 00m' },
        cost: costFigure(1.5),
        tokens: { total: cellOf('1.5k tok'), input: cellOf('1.2k'), output: cellOf('300'), cache_hit: cellOf('83.3%') },
        seats: [seat('review:security', 'claude', 'opus'), seat('review:correctness', 'codex', 'gpt-5.5')],
      }),
      graphState('ship'),
      graphState('done', 'end'),
      graphState('stop', 'beneath'),
    ],
    rules: [
      graphRule('INTAKE-OK', 'intake', 'implement', 'forward', 40),
      graphRule('IMPL-RETRY', 'implement', 'implement', 'self', 2, { result: 'broken' }),
      graphRule('IMPL-OK', 'implement', 'verify', 'forward', 6),
      graphRule('VERIFY-PASS', 'verify', 'review', 'forward', 4),
      graphRule('VERIFY-FAIL', 'verify', 'stop', 'terminal', 3, { result: 'fail', when: ['not fixes_applied'], severity: 'normal', latest: [
        { run_id: 'encrypt-the-token-store-41d0a9c2', label: 'Encrypt the token store', at: '2026-10-12T09:00:00Z' },
        { run_id: 'tidy-the-seats-2b3c4d5e', label: 'Tidy the seats', at: '2026-10-10T09:00:00Z' },
      ] }),
      graphRule('REVIEW-OK', 'review', 'ship', 'forward', 2),
      graphRule('REVIEW-REFORGE', 'review', 'implement', 'back', 1, { result: 'fail' }),
      graphRule('REVIEW-CLEAN', 'review', 'verify', 'back', 0),
      graphRule('REVIEW-RESIDUAL-SECURITY', 'review', 'stop', 'terminal', 2, { result: 'fail', severity: 'high' }),
      graphRule('REVIEW-HOLD', 'review', null, 'forward', 1, { result: 'hold' }),
      graphRule('SHIP-COMPLETE', 'ship', 'done', 'terminal', 2),
      graphRule('SHIP-DIRTY', 'ship', 'stop', 'terminal', 0, { result: 'dirty' }),
    ],
    never_fired: { count: 2, of: 12, text: '2 of 12' },
    retired: [{ rule_id: 'OLD-RULE', count: 2 }],
    drifted_runs: 4,
    drift_text: '4 runs ran under an earlier version of this table',
    ends: {
      done: 2,
      stop: [{ rule_id: 'VERIFY-FAIL', count: 3 }, { rule_id: 'REVIEW-RESIDUAL-SECURITY', count: 2 }],
      stopped: 5,
      operator: 1,
      open: 2,
      text: '2 done · 5 stop by a rule · 1 stopped by you · 2 open',
    },
    live: [
      { run_id: 'add-the-graph-7c1d2e3f', label: 'Add the graph', state: 'verify' },
      { run_id: 'tidy-the-seats-2b3c4d5e', label: 'Tidy the seats', state: 'review' },
    ],
    ...over,
  }
  return { ...doc, links: over.links ?? linksOf(doc.rules) }
}

// ---------------------------------------------------------- the binary

/** A stub answer: printed JSON, or `{stderr, code}`, `{sleep_ms}`, `{bytes}`. */
export const json = (doc) => ({ stdout: JSON.stringify(doc) })

/**
 * A scratch directory with a stub `brokkr` that answers `rules`
 * (`[{match: [...words], responses: [...]}]`), and the runner configuration
 * that calls it.
 */
export function stubBrokkr(rules, over = {}) {
  const dir = mkdtempSync(join(tmpdir(), 'brokkr-console-test-'))
  writeFileSync(join(dir, 'script.json'), JSON.stringify({ rules }))
  writeFileSync(join(dir, 'calls.ndjson'), '')
  const program = join(dir, 'brokkr')
  writeFileSync(program, `#!/bin/sh\nexec "${process.execPath}" "${STUB}" "$@"\n`)
  chmodSync(program, 0o755)
  const config = {
    program: [program],
    cwd: dir,
    env: { ...process.env, BROKKR_STUB_DIR: dir },
    timeoutMs: 5000,
    maxBytes: 1024 * 1024,
    pollMs: 60_000,
    stateDir: join(dir, 'state'),
    graceMs: 300,
    resume: { kind: 'recipe', name: 'self' },
    ...over,
  }
  const calls = () =>
    readFileSync(join(dir, 'calls.ndjson'), 'utf8')
      .split('\n')
      .filter((line) => line !== '')
      .map((line) => JSON.parse(line))
  return { dir, program, config, calls }
}

// ------------------------------------------------------------ the host

/** The RFC 6455 sample nonce a test's upgrade request carries. */
export const WS_KEY = 'dGhlIHNhbXBsZSBub25jZQ=='

/** A client text frame carrying `text`, masked unless `masked` is false. */
export function clientFrame(text, masked = true) {
  const payload = Buffer.from(text)
  const mask = Buffer.from([1, 2, 3, 4])
  const head = Buffer.from([0x81, (masked ? 0x80 : 0) | payload.length])
  if (!masked) return Buffer.concat([head, payload])
  const body = Buffer.from(payload.map((byte, index) => byte ^ mask[index % 4]))
  return Buffer.concat([head, mask, body])
}

/** A stub DSH host: the routes, upgrade, model tools and event listeners (each `{listener, options}`) a plugin registers, and its fence. */
export function stubHost(rejection = undefined) {
  const host = { routes: new Map(), upgrades: new Map(), tools: new Map(), listeners: new Map(), rejection }
  host.ctx = {
    effect: (register, _label) => register(),
    on: (event, listener, options = {}) => host.listeners.set(event, { listener, options }),
    tools: { register: (definition) => host.tools.set(definition.name, definition) },
    webServer: {
      register: ({ kind, path, handler }) => host.routes.set(path, { kind, handler }),
      registerUpgrade: ({ path, handler }) => host.upgrades.set(path, handler),
    },
    connection: { requestRejection: () => host.rejection },
  }
  return host
}

/** Call an HTTP handler with a request and settle with the answer. */
export function request(handler, { method = 'GET', url = '/', body = null, headers = {} } = {}) {
  const req = Readable.from(body === null ? [] : [Buffer.from(typeof body === 'string' ? body : JSON.stringify(body))])
  Object.assign(req, { method, url, headers })
  return new Promise((settle) => {
    const res = {
      statusCode: 200,
      headers: {},
      setHeader: (name, value) => (res.headers[name] = value),
      end: (text = '') => settle({ status: res.statusCode, headers: res.headers, body: String(text) }),
    }
    handler(req, res)
  })
}

/** Run `work` with `vars` set in the environment, restoring it however `work` ends. */
export async function withEnv(vars, work) {
  const before = Object.fromEntries(Object.keys(vars).map((name) => [name, process.env[name]]))
  Object.assign(process.env, vars)
  try {
    return await work()
  } finally {
    for (const [name, value] of Object.entries(before)) {
      if (value === undefined) delete process.env[name]
      else process.env[name] = value
    }
  }
}
