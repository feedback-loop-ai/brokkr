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
    view_version: 19,
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

/** A `RunView` as `brokkr inspect --json` prints it: parked at review, head `seq`. */
export function runViewDoc(over = {}) {
  const { seq = 12, ...rest } = over
  return {
    view_version: 19,
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
      strategy: null,
    },
    ruling: { rule_id: 'REVIEW-SECURITY-RESIDUAL', severity_class: 'awaiting_operator', from: 'review', next: '', result: null, inputs: [], problem: null, superseded: null },
    participants: [],
    live: [],
    phases: [{ name: 'implement', strategy: null, visits: 2, current: false, plain: true, columns: [], returns: ['review'] }],
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
    ...rest,
  }
}

/** `brokkr analytics --json` for one realm whose seats reported no cost. */
export function analyticsDoc(over = {}) {
  return {
    view_version: 19,
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
    models: [{ model: 'gpt-5.5', attempts: 4, percent: 100, text: 'gpt-5.5 100%' }],
    realms: [{ realm: 'lanetally', shipped: 3, stopped: 1, median: { millis: 15_120_000, text: '4h 12m' }, waiting_millis: 3_600_000, waiting_text: '1.0h', cost: { kind: 'not_reported' }, cost_text: 'not reported', unreadable_runs: 0 }],
    quarantined: [],
    not_recorded: ['machine capacity over time', 'cost limits', 'costs for Codex and dsh', 'reasons findings were dismissed'],
    ...over,
  }
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

/** A stub DSH host: the routes and upgrade a plugin registers, and its fence. */
export function stubHost(rejection = undefined) {
  const host = { routes: new Map(), upgrades: new Map(), rejection }
  host.ctx = {
    effect: (register, _label) => register(),
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
