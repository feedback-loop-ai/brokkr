export const meta = {
  name: 'forge-implement',
  description: 'Implement a workspace vertical feature across repos: parallel Sonnet implementers per wave, cross-repo contract checks between waves, independent gate audits, bounded fix loops',
  whenToUse: 'Invoked by /forge after the architect has produced spec/plan/tasks and worktrees exist. Args carry the feature repos.yaml forge section (waves, contracts, gates).',
  phases: [
    { title: 'Wave 1', detail: 'parallel vf-implementer per repo', model: 'sonnet' },
    { title: 'Wave 2', detail: 'parallel vf-implementer per repo', model: 'sonnet' },
    { title: 'Contracts', detail: 'cross-repo contract checks + targeted fixes', model: 'sonnet' },
    { title: 'Gates', detail: 'independent per-repo gate audit', model: 'sonnet' },
  ],
}

// args: {
//   featureId: 'NNN-slug',
//   specDir: '/abs/path/specs/NNN-slug',
//   waves: [[{repo, worktree, base, tasksFile, gates: [..]}], ...],   // worktree/tasksFile absolute
//   contracts: [{name, producer, consumers: [..], check}],
//   architectNotes: 'risks/assumptions relevant to implementers',
//   maxFixRounds: 2,
// }
// Defensive: args may arrive JSON-stringified depending on the caller.
const A = typeof args === 'string' ? (() => { try { return JSON.parse(args) } catch { return {} } })() : (args ?? {})
const { featureId, specDir, waves = [], contracts = [], architectNotes = '', maxFixRounds = 2 } = A
if (!featureId || !waves.length) return { status: 'noop', reason: 'no waves provided', featureId: featureId ?? null }

const allRepos = waves.flat()
const byName = Object.fromEntries(allRepos.map(r => [r.repo, r]))

const IMPL_SCHEMA = {
  type: 'object',
  required: ['repo', 'status', 'gates'],
  properties: {
    repo: { type: 'string' },
    status: { enum: ['green', 'broken', 'blocked'] },
    commits: { type: 'array', items: { type: 'string' } },
    tasks_done: { type: 'number' }, tasks_total: { type: 'number' },
    gates: { type: 'array', items: { type: 'object', required: ['cmd', 'pass'], properties: { cmd: { type: 'string' }, pass: { type: 'boolean' }, tail: { type: 'string' } } } },
    contracts_produced: { type: 'array', items: { type: 'string' } },
    dispositions: { type: 'array', items: { type: 'object', required: ['id', 'action'], properties: { id: { type: 'string' }, action: { enum: ['fixed', 'declined', 'escalated'] }, note: { type: 'string' } } } },
    deviations: { type: 'array', items: { type: 'string' } },
  },
}

const CHECK_SCHEMA = {
  type: 'object',
  required: ['contract', 'status'],
  properties: {
    contract: { type: 'string' },
    status: { enum: ['pass', 'fail', 'check-broken'] },
    artifact: { type: 'object', properties: { path: { type: 'string' }, breaking_changes: { type: 'string' } } },
    failures: { type: 'array', items: { type: 'object', required: ['consumer', 'attribution', 'fix_direction'], properties: {
      consumer: { type: 'string' }, attribution: { enum: ['producer-moved', 'consumer-stale', 'check-broken'] },
      symbol: { type: 'string' }, expected: { type: 'string' }, actual: { type: 'string' },
      evidence: { type: 'string' }, fix_direction: { type: 'string' },
    } } },
    commands_run: { type: 'array', items: { type: 'object', properties: { repo: { type: 'string' }, cmd: { type: 'string' }, exit: { type: 'number' } } } },
  },
}

const GATE_SCHEMA = {
  type: 'object',
  required: ['repo', 'pass', 'results'],
  properties: {
    repo: { type: 'string' }, pass: { type: 'boolean' },
    results: { type: 'array', items: { type: 'object', required: ['cmd', 'pass'], properties: { cmd: { type: 'string' }, pass: { type: 'boolean' }, exit: { type: 'number' }, tail: { type: 'string' } } } },
  },
}

const implPrompt = (r, waveIdx, producedArtifacts) => `MODE=implement
FEATURE_ID=${featureId}
REPO=${r.repo}
WORKTREE=${r.worktree}
BASE_BRANCH=${r.base}
TASKS_FILE=${r.tasksFile}

You are wave ${waveIdx + 1} of ${waves.length}. Contract artifacts already produced by earlier waves (build against these, not assumptions):
${producedArtifacts.length ? producedArtifacts.map(a => `- ${a.contract}: ${a.path ?? 'see producer worktree'} (breaking: ${a.breaking ?? 'n/a'})`).join('\n') : '- none'}

Architect notes (risks/assumptions to honor):
${architectNotes || '- none'}

Gate commands for this repo (run all, in order, in one uninterrupted pass before reporting):
${(r.gates ?? []).map(g => `- ${g}`).join('\n') || '- derive from the repo CLAUDE.md'}

Execute your slice per your agent instructions and return the report JSON.`

const fixPrompt = (r, failures, source) => `MODE=fix
FEATURE_ID=${featureId}
REPO=${r.repo}
WORKTREE=${r.worktree}
BASE_BRANCH=${r.base}
TASKS_FILE=${r.tasksFile}

CONTEXT — ${source} failures assigned to this repo. Address every item, re-run the full gate pass, return the report JSON with per-item dispositions:
${JSON.stringify(failures, null, 2)}

Gate commands:
${(r.gates ?? []).map(g => `- ${g}`).join('\n') || '- derive from the repo CLAUDE.md'}`

const checkPrompt = (c) => `CONTRACT=${JSON.stringify({ name: c.name, producer: c.producer, consumers: c.consumers, check: c.check }, null, 2)}
WORKTREES=${JSON.stringify(Object.fromEntries(allRepos.map(r => [r.repo, r.worktree])), null, 2)}
BASES=${JSON.stringify(Object.fromEntries(allRepos.map(r => [r.repo, r.base])), null, 2)}

Verify this contract per your agent instructions and return the report JSON.`

const gateAuditPrompt = (r) => `You are an independent gate auditor. In the git worktree at ${r.worktree} (repo: ${r.repo}), run EXACTLY these commands in order, capturing exit codes. Do not fix anything, do not interpret beyond pass/fail, do not run anything else. If a command fails, capture the last ~15 lines of its output as "tail".
${(r.gates ?? []).map(g => `- ${g}`).join('\n')}
Return JSON: {repo, pass (all commands exited 0), results: [{cmd, pass, exit, tail?}]}.`

// ---- Waves ----------------------------------------------------------------
const implReports = {}
const producedArtifacts = []   // {contract, path, breaking} accumulated across waves
const implemented = new Set()

for (let w = 0; w < waves.length; w++) {
  const wave = waves[w]
  log(`Wave ${w + 1}/${waves.length}: ${wave.map(r => r.repo).join(', ')}`)
  const results = await parallel(wave.map(r => () =>
    agent(implPrompt(r, w, producedArtifacts), {
      agentType: 'vf-implementer', model: 'sonnet[1m]', effort: 'high',
      label: `impl:${r.repo}`, phase: `Wave ${w + 1}`, schema: IMPL_SCHEMA,
    })))
  wave.forEach((r, i) => {
    implReports[r.repo] = results[i] ?? { repo: r.repo, status: 'blocked', gates: [], note: 'agent died or was skipped' }
    implemented.add(r.repo)
  })

  // Contracts ready = producer and all consumers implemented. Check → fix → re-check,
  // always ending on a check so final statuses reflect reality (maxFixRounds fixes max).
  const upsertArtifact = (a) => {
    const i = producedArtifacts.findIndex(x => x.contract === a.contract)
    if (i >= 0) producedArtifacts[i] = a; else producedArtifacts.push(a)
  }
  const ready = contracts.filter(c => implemented.has(c.producer) && (c.consumers ?? []).every(x => implemented.has(x)))
  let pending = ready.filter(c => c._status !== 'pass')
  for (let round = 0; pending.length && round <= maxFixRounds; round++) {
    if (budget.total && budget.remaining() < 60_000) {
      log('budget floor reached — stopping contract fix loop')
      pending.forEach(c => { c._status ??= 'skipped' })  // ready but never verified ≠ pass
      break
    }
    const checks = await parallel(pending.map(c => () =>
      agent(checkPrompt(c), { agentType: 'vf-contract-checker', model: 'sonnet[1m]', effort: 'medium', label: `check:${c.name}`, phase: 'Contracts', schema: CHECK_SCHEMA })))
    const fixByRepo = {}
    pending.forEach((c, i) => {
      const res = checks[i]
      c._status = res?.status ?? 'check-broken'
      c._lastReport = res ?? c._lastReport
      if (res?.artifact?.path) upsertArtifact({ contract: c.name, path: res.artifact.path, breaking: res.artifact.breaking_changes })
      if (res?.status === 'fail') for (const f of res.failures ?? []) {
        // fix_direction names the repo to change; default consumer-stale → consumer, producer-moved → producer
        const target = allRepos.find(r => (f.fix_direction ?? '').includes(r.repo))?.repo
          ?? (f.attribution === 'producer-moved' ? c.producer : f.consumer)
        ;(fixByRepo[target] ??= []).push({ contract: c.name, ...f })
      }
    })
    pending = pending.filter(c => c._status === 'fail')  // check-broken is not implementer-fixable
    const targets = Object.keys(fixByRepo).filter(t => byName[t])
    if (!pending.length || !targets.length || round === maxFixRounds) break
    log(`Contract fix round ${round + 1}: ${targets.join(', ')}`)
    const fixes = await parallel(targets.map(t => () =>
      agent(fixPrompt(byName[t], fixByRepo[t], 'contract-check'), { agentType: 'vf-implementer', model: 'sonnet[1m]', effort: 'high', label: `fix:${t}`, phase: 'Contracts', schema: IMPL_SCHEMA })))
    targets.forEach((t, i) => { if (fixes[i]) implReports[t] = { ...implReports[t], ...fixes[i], fixed_contracts: true } })
  }
}

// ---- Independent gate audit (trust, but verify) ---------------------------
log('Independent gate audit across all repos')
let audits = await parallel(allRepos.map(r => () =>
  agent(gateAuditPrompt(r), { model: 'sonnet', effort: 'low', label: `gates:${r.repo}`, phase: 'Gates', schema: GATE_SCHEMA })))
let failing = allRepos.filter((r, i) => !(audits[i]?.pass))
if (failing.length && (!budget.total || budget.remaining() > 60_000)) {
  log(`Gate audit failures: ${failing.map(r => r.repo).join(', ')} — one fix round`)
  const fixes = await parallel(failing.map((r) => () =>
    agent(fixPrompt(r, audits[allRepos.indexOf(r)]?.results?.filter(x => !x.pass) ?? [], 'gate-audit'), { agentType: 'vf-implementer', model: 'sonnet[1m]', effort: 'high', label: `fix:${r.repo}`, phase: 'Gates', schema: IMPL_SCHEMA })))
  failing.forEach((r, i) => { if (fixes[i]) implReports[r.repo] = { ...implReports[r.repo], ...fixes[i] } })
  const reAudits = await parallel(failing.map(r => () =>
    agent(gateAuditPrompt(r), { model: 'sonnet', effort: 'low', label: `gates:${r.repo}#2`, phase: 'Gates', schema: GATE_SCHEMA })))
  failing.forEach((r, i) => { audits[allRepos.indexOf(r)] = reAudits[i] ?? audits[allRepos.indexOf(r)] })
}

const gateStatus = Object.fromEntries(allRepos.map((r, i) => [r.repo, audits[i]?.pass ? 'green' : 'red']))
const contractStatus = Object.fromEntries(contracts.map(c => [c.name, c._status ?? 'not-run']))
// Fail closed: only 'pass' counts. 'skipped' (budget floor) and 'not-run' (a
// producer/consumer never appeared in any wave — misdeclared contract) must not
// roll up green — genuinely inapplicable contracts belong out of `contracts`.
const status = Object.values(gateStatus).every(s => s === 'green')
  && Object.values(contractStatus).every(s => s === 'pass')
  ? 'green' : 'broken'

return {
  featureId, status,
  repos: implReports,
  gates: gateStatus,
  contracts: Object.fromEntries(contracts.map(c => [c.name, { status: c._status ?? 'not-run', last: c._lastReport ?? null }])),
  artifacts: producedArtifacts,
}
