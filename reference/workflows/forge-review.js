export const meta = {
  name: 'forge-review',
  description: 'Adversarial multi-dimensional review of a workspace vertical feature: Opus panel per repo × dimension (correctness, spec-compliance, quality, security w/ SOC 2 + ISO 27001 catalog), two-lens verification of every finding, Sonnet fix rounds, security-hold gate',
  whenToUse: 'Invoked by /forge after forge-implement returns green. Also runnable standalone on any feature with worktrees.',
  phases: [
    { title: 'Review r1', detail: 'panel matrix: repo × dimension', model: 'opus' },
    { title: 'Verify r1', detail: 'two adversarial lenses per finding', model: 'opus' },
    { title: 'Fix r1', detail: 'one fixer per affected repo', model: 'sonnet' },
    { title: 'Review r2', detail: 'scoped re-review after fixes', model: 'opus' },
  ],
}

// args: {
//   featureId, specDir, catalogPath,
//   repos: [{repo, worktree, base, tasksFile, gates: [..]}],
//   dimensions: ['correctness','spec-compliance','quality','security'],
//   maxRounds: 3,
// }
// Defensive: args may arrive JSON-stringified depending on the caller.
const A = typeof args === 'string' ? (() => { try { return JSON.parse(args) } catch { return {} } })() : (args ?? {})
const {
  featureId, specDir,
  catalogPath = '.claude/skills/forge/references/security-review-catalog.md',
  repos = [],
  maxRounds = 3,
} = A
if (!featureId || !repos.length) return { status: 'noop', reason: 'no repos provided', featureId: featureId ?? null }
// FR-008: the security dimension is non-removable — enforce it in code on every
// entry path (this workflow is also a standalone entry point), not just in SKILL prose.
let dimensions = A.dimensions ?? ['correctness', 'spec-compliance', 'quality', 'security']
if (!dimensions.includes('security')) {
  log("'security' was missing from dimensions — re-added (non-removable, FR-008)")
  dimensions = [...dimensions, 'security']
}

const SEV = { critical: 4, high: 3, medium: 2, low: 1, info: 0 }
const sevAtLeast = (s, floor) => (SEV[s] ?? 0) >= (SEV[floor] ?? 0)

const FINDING_ITEM = {
  type: 'object',
  required: ['id', 'file', 'severity', 'summary'],
  properties: {
    id: { type: 'string' }, file: { type: 'string' }, line: { type: 'number' },
    severity: { enum: ['critical', 'high', 'medium', 'low', 'info'] },
    category: { type: 'string' }, summary: { type: 'string' },
    failure_scenario: { type: 'string' }, abuse_path: { type: 'string' },
    evidence: { type: 'string' }, fix_hint: { type: 'string' }, remediation: { type: 'string' },
    controls: { type: 'object', properties: { soc2: { type: 'array', items: { type: 'string' } }, iso27001: { type: 'array', items: { type: 'string' } } } },
    residual_risk_if_unfixed: { type: 'string' },
  },
}
const REVIEW_SCHEMA = {
  type: 'object', required: ['findings'],
  properties: { repo: { type: 'string' }, dimension: { type: 'string' }, findings: { type: 'array', items: FINDING_ITEM }, clean_areas: { type: 'array', items: { type: 'string' } } },
}
const SEC_SCHEMA = {
  type: 'object', required: ['findings', 'verdict'],
  properties: {
    repo: { type: 'string' }, dimension: { type: 'string' },
    verdict: { enum: ['pass', 'conditional', 'fail'] }, verdict_rationale: { type: 'string' },
    findings: { type: 'array', items: FINDING_ITEM },
    families_checked: { type: 'array', items: { type: 'object', properties: { family: { type: 'string' }, status: { type: 'string' } } } },
    notes_for_release_risk_profile: { type: 'array', items: { type: 'string' } },
  },
}
const VERDICT_SCHEMA = {
  type: 'object', required: ['refuted', 'reason'],
  properties: { refuted: { type: 'boolean' }, reason: { type: 'string' }, severity: { enum: ['critical', 'high', 'medium', 'low', 'info'] }, confidence: { enum: ['high', 'medium', 'low'] } },
}
const FIX_SCHEMA = {
  type: 'object', required: ['repo', 'status', 'dispositions'],
  properties: {
    repo: { type: 'string' }, status: { enum: ['green', 'broken', 'blocked'] },
    dispositions: { type: 'array', items: { type: 'object', required: ['id', 'action'], properties: { id: { type: 'string' }, action: { enum: ['fixed', 'declined', 'escalated'] }, note: { type: 'string' } } } },
    gates: { type: 'array', items: { type: 'object', properties: { cmd: { type: 'string' }, pass: { type: 'boolean' }, tail: { type: 'string' } } } },
    commits: { type: 'array', items: { type: 'string' } },
  },
}

const prior = {}  // `${repo}:${dim}` -> {fixed: [], declined: []}
const priorFor = (repo, dim) => prior[`${repo}:${dim}`] ?? { fixed: [], declined: [] }

const reviewPrompt = (r, dim, round) => `REPO=${r.repo}
WORKTREE=${r.worktree}
BASE_BRANCH=${r.base}
DIMENSION=${dim}
SPEC_DIR=${specDir}
${dim === 'security' ? `CATALOG=${catalogPath}` : ''}
Review round ${round}. PRIOR findings from earlier rounds (do not re-report fixed items unless the fix is wrong; contest a decline only with a refutation of its reason):
${JSON.stringify(priorFor(r.repo, dim), null, 2)}

Review the feature diff per your agent instructions and return the report JSON.`

const verifyPrompt = (f, lens) => lens === 'refute'
  ? `You are an adversarial verifier. Try to PROVE this review finding is a false positive by reading the actual code in the worktree. Follow the real call paths; check guards, validation, and framework behavior the reviewer may have missed. If you cannot concretely reproduce the failure/abuse scenario from the code, return refuted=true.
FINDING: ${JSON.stringify(f, null, 2)}
WORKTREE: ${f._worktree} (base branch: ${f._base})
Return JSON: {refuted, reason, confidence}.`
  : `You are an independent impact assessor. Assume this finding's technical claim, then verify from the code whether the scenario is actually reachable in practice, and rate its true severity (critical/high/medium/low/info). If the scenario is unreachable or meaningless in context, return refuted=true.
FINDING: ${JSON.stringify(f, null, 2)}
WORKTREE: ${f._worktree} (base branch: ${f._base})
Return JSON: {refuted, reason, severity, confidence}.`

const fixPrompt = (r, findings) => `MODE=fix
FEATURE_ID=${featureId}
REPO=${r.repo}
WORKTREE=${r.worktree}
BASE_BRANCH=${r.base}
TASKS_FILE=${r.tasksFile ?? ''}

CONTEXT — confirmed review findings for this repo. Address every item (security findings are never declined for convenience), re-run the full gate pass, return the report JSON with per-finding dispositions keyed by finding id:
${JSON.stringify(findings.map(({ _worktree, _base, _verdicts, ...f }) => f), null, 2)}

Gate commands:
${(r.gates ?? []).map(g => `- ${g}`).join('\n') || '- derive from the repo CLAUDE.md'}`

// ---- Round loop ------------------------------------------------------------
const history = []
const allConfirmed = []   // findings with dispositions across rounds
const advisory = []       // confirmed low/info — recorded, not fixed
const secVerdicts = {}    // repo -> {verdict, rationale, round}
let residual = []         // confirmed but unfixed at exit
let scopeRepos = repos, scopeDims = dimensions
let fixesUnverified = false  // true when a fix round ran but no review round confirmed it after
let lastFixHadSecurity = false // true when that unverified fix round touched a security finding

for (let round = 1; round <= maxRounds; round++) {
  if (budget.total && budget.remaining() < 80_000) {
    log(`budget floor — stopping before round ${round}`)
    // Fail closed: floored before the first panel ever ran — never report 'clean' for an unreviewed feature.
    if (round === 1) return { featureId, status: 'budget-exhausted', reason: 'insufficient budget to run any review round', rounds: [] }
    break
  }

  // 1) Panel matrix — every cell in parallel.
  const cells = scopeRepos.flatMap(r => scopeDims.map(d => ({ r, d })))
  log(`Round ${round}: reviewing ${cells.length} cells (${scopeRepos.map(x => x.repo).join(', ')} × ${scopeDims.join(', ')})`)
  const reviews = await parallel(cells.map(({ r, d }) => () =>
    agent(reviewPrompt(r, d, round), {
      agentType: d === 'security' ? 'vf-security-reviewer' : 'vf-reviewer',
      model: 'opus', effort: d === 'security' ? 'xhigh' : 'high',
      label: `${d}:${r.repo}`, phase: `Review r${round}`,
      schema: d === 'security' ? SEC_SCHEMA : REVIEW_SCHEMA,
    })))
  cells.forEach(({ r, d }, i) => {
    if (d === 'security' && reviews[i]?.verdict) secVerdicts[r.repo] = { verdict: reviews[i].verdict, rationale: reviews[i].verdict_rationale ?? '', round }
  })

  // 2) Dedup across dimensions (barrier justified: dedupe before expensive verification).
  const seen = new Map()
  cells.forEach(({ r, d }, i) => {
    for (const f of reviews[i]?.findings ?? []) {
      const key = `${r.repo}:${f.file}:${f.line ?? 0}:${f.category ?? 'x'}`
      if (seen.has(key)) { const prev = seen.get(key); prev.dimensions = [...new Set([...prev.dimensions, d])]; continue }
      seen.set(key, { ...f, id: `${d}:${r.repo}:${f.id}`, repo: r.repo, dimensions: [d], _worktree: r.worktree, _base: r.base })
    }
  })
  const findings = [...seen.values()]
  fixesUnverified = false; lastFixHadSecurity = false  // this round's panel has now re-examined any prior fixes in scope
  if (!findings.length) { history.push({ round, cells: cells.length, found: 0, confirmed: 0, fixed: 0 }); log(`Round ${round}: panel clean`); residual = []; break }

  // 3) Two-lens adversarial verification, all findings concurrently.
  const verified = await parallel(findings.map(f => () =>
    parallel(['refute', 'impact'].map(lens => () =>
      agent(verifyPrompt(f, lens), { model: 'opus', effort: 'high', label: `verify:${f.id}:${lens}`, phase: `Verify r${round}`, schema: VERDICT_SCHEMA })))
      .then(vs => {
        const [refuter, impact] = vs
        const refutes = vs.filter(v => v?.refuted).length
        const severity = impact?.severity ?? f.severity
        return { ...f, severity, _verdicts: { refuter, impact }, confirmed: refutes < 2 }
      })))
  const confirmedAll = verified.filter(Boolean).filter(f => f.confirmed)
  const confirmed = confirmedAll.filter(f => sevAtLeast(f.severity, 'medium'))
  advisory.push(...confirmedAll.filter(f => !sevAtLeast(f.severity, 'medium')).map(({ _worktree, _base, ...f }) => f))
  history.push({ round, cells: cells.length, found: findings.length, confirmed: confirmed.length, killed: verified.filter(f => f && !f.confirmed).length })
  if (!confirmed.length) { log(`Round ${round}: all findings refuted or advisory — clean`); residual = []; break }

  if (round === maxRounds) {
    residual = confirmed.map(({ _worktree, _base, _verdicts, ...f }) => ({ ...f, disposition: 'unfixed-max-rounds' }))
    log(`Round ${round}: max rounds reached with ${confirmed.length} confirmed findings — reporting residual, not fixing blind`)
    break
  }

  // 4) Fix — one fixer per affected repo, parallel across repos.
  const byRepo = {}
  confirmed.forEach(f => (byRepo[f.repo] ??= []).push(f))
  const fixRepos = repos.filter(r => byRepo[r.repo])
  log(`Round ${round}: fixing ${confirmed.length} findings across ${fixRepos.map(r => r.repo).join(', ')}`)
  const fixes = await parallel(fixRepos.map(r => () =>
    agent(fixPrompt(r, byRepo[r.repo]), { agentType: 'vf-implementer', model: 'sonnet', effort: 'high', label: `fix:${r.repo}`, phase: `Fix r${round}`, schema: FIX_SCHEMA })))
  fixRepos.forEach((r, i) => {
    const disp = Object.fromEntries((fixes[i]?.dispositions ?? []).map(d => [d.id, d]))
    for (const f of byRepo[r.repo]) {
      const d = disp[f.id] ?? { action: 'escalated', note: 'fixer returned no disposition' }
      allConfirmed.push({ ...f, _worktree: undefined, _base: undefined, _verdicts: undefined, disposition: d.action, disposition_note: d.note ?? '', round })
      for (const dim of f.dimensions) {
        const p = (prior[`${f.repo}:${dim}`] ??= { fixed: [], declined: [] })
        ;(d.action === 'fixed' ? p.fixed : p.declined).push({ id: f.id, summary: f.summary, reason: d.note ?? d.action })
      }
    }
  })
  history[history.length - 1].fixed = allConfirmed.filter(f => f.round === round && f.disposition === 'fixed').length
  fixesUnverified = true
  lastFixHadSecurity = confirmed.some(f => f.dimensions.includes('security'))

  // 5) Scoped re-review: repos that were fixed × (dims that fired ∪ correctness ∪ security).
  scopeRepos = fixRepos
  const firedDims = new Set(confirmed.flatMap(f => f.dimensions))
  firedDims.add('correctness'); firedDims.add('security')
  scopeDims = dimensions.filter(d => firedDims.has(d))
}

// ---- Security hold ----------------------------------------------------------
const secOpen = [
  ...allConfirmed.filter(f => f.dimensions?.includes('security') && f.disposition !== 'fixed'),
  ...residual.filter(f => f.dimensions?.includes('security')),
]
const securityHold =
  // SKILL Phase R doctrine: NO security residual ships, any severity. secOpen already
  // contains every non-fixed confirmed security finding incl. declined/escalated.
  secOpen.length > 0 ||
  Object.values(secVerdicts).some(v => v.verdict === 'fail') ||
  // Fail closed: a security fix was applied but no panel round re-verified it
  // (budget floor after a fix round) — never ship an unre-reviewed security change.
  (fixesUnverified && lastFixHadSecurity)

const clean = residual.length === 0 && !fixesUnverified
return {
  featureId,
  status: securityHold ? 'security-hold' : clean ? 'clean' : fixesUnverified ? 'clean-unverified' : 'residual',
  rounds: history,
  confirmed: allConfirmed,
  residual, advisory,
  security: { verdicts: secVerdicts, hold: securityHold, open: secOpen.map(f => ({ id: f.id, severity: f.severity, summary: f.summary, disposition: f.disposition ?? 'unfixed' })) },
}
