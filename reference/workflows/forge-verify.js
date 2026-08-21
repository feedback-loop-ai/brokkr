export const meta = {
  name: 'forge-verify',
  description: 'Live test-pyramid verification of a workspace vertical feature: boot the feature stack from worktrees, run API/it-spec + test-suites + gql-live tracks in parallel with pseudo-manual acceptance walks (headless Playwright per user story), fix attributed failures with Sonnet, persist passing acceptance walks as durable Playwright specs, always tear down',
  whenToUse: 'Invoked by /forge after forge-implement (full mode) and again after forge-review fixes (regression mode). Requires forge.verification in the feature repos.yaml.',
  phases: [
    { title: 'Boot', detail: 'stack runner: compose deps + services from worktrees', model: 'sonnet' },
    { title: 'Verify r1', detail: 'test tracks ∥ acceptance story walks', model: 'sonnet' },
    { title: 'Fix r1', detail: 'attributed product-bug fixes per repo', model: 'sonnet' },
    { title: 'Verify r2', detail: 're-run failed items only', model: 'sonnet' },
    { title: 'Teardown', detail: 'manifest-scoped stack teardown', model: 'sonnet' },
  ],
}

// args: {
//   featureId, specDir,
//   repos: [{repo, worktree, base, gates: [..]}],           // affected repos w/ worktrees
//   verification: {stack: {...}, tracks: [...]},             // forge.verification from repos.yaml
//   leadRepo, maxFixRounds = 2,
//   mode: 'full' | 'regression',
//   regressionScope: {repos: [..], scenarioIds: [..]},       // regression mode only
// }
const A = typeof args === 'string' ? (() => { try { return JSON.parse(args) } catch { return {} } })() : (args ?? {})
const { featureId, specDir, repos = [], verification = {}, leadRepo, maxFixRounds = 2, mode = 'full', regressionScope = { repos: [], scenarioIds: [] } } = A
if (!featureId) return { status: 'noop', reason: 'no featureId provided' }
const allTracks = verification.tracks ?? []
if (!allTracks.length) return { featureId, mode, status: 'not-applicable', reason: 'architect declared no verification tracks' }

const byName = Object.fromEntries(repos.map(r => [r.repo, r]))
const defaultDefectRepo = leadRepo ?? repos[0]?.repo ?? null
const evidenceDir = `worktrees/${featureId}/.forge/evidence`

const STACK_SCHEMA = {
  type: 'object', required: ['status'],
  properties: {
    action: { type: 'string' }, status: { enum: ['up', 'port-conflict', 'failed', 'torn-down', 'degraded'] },
    isolation: { type: 'object' }, volumes_created: { type: 'array', items: { type: 'string' } },
    endpoints: { type: 'object' }, manifest: { type: 'string' },
    services: { type: 'array', items: { type: 'object', properties: { repo: { type: 'string' }, state: { type: 'string' }, pid: { type: 'number' }, log: { type: 'string' } } } },
    conflicts: { type: 'array', items: { type: 'object', properties: { port: { type: 'number' }, occupant: { type: 'string' } } } },
    notes: { type: 'array', items: { type: 'string' } },
  },
}
const TRACK_SCHEMA = {
  type: 'object', required: ['track', 'status'],
  properties: {
    track: { type: 'string' }, status: { enum: ['pass', 'fail', 'env-broken'] },
    totals: { type: 'object' },
    failures: { type: 'array', items: { type: 'object', required: ['test', 'attribution', 'symptom'], properties: {
      test: { type: 'string' }, attribution: { enum: ['product-bug', 'test-bug', 'env-issue'] },
      repo: { type: 'string' }, symptom: { type: 'string' }, evidence: { type: 'string' }, fix_direction: { type: 'string' },
    } } },
    commands_run: { type: 'array', items: { type: 'object' } },
  },
}
const SCENARIO_SCHEMA = {
  type: 'object', required: ['story', 'status', 'results'],
  properties: {
    story: { type: 'string' }, status: { enum: ['pass', 'fail', 'blocked'] },
    results: { type: 'array', items: { type: 'object', required: ['id', 'status'], properties: {
      id: { type: 'string' }, status: { enum: ['pass', 'fail', 'blocked'] }, evidence: { type: 'string' },
      defect: { type: 'object', properties: { repo: { type: 'string' }, symptom: { type: 'string' }, fix_direction: { type: 'string' }, console_or_network: { type: 'string' } } },
    } } },
    emitted_spec: { type: 'object', properties: { path: { type: 'string' }, repo: { type: 'string' } } },
  },
}
const FIX_SCHEMA = {
  type: 'object', required: ['repo', 'status', 'dispositions'],
  properties: {
    repo: { type: 'string' }, status: { enum: ['green', 'broken', 'blocked'] },
    dispositions: { type: 'array', items: { type: 'object', required: ['id', 'action'], properties: { id: { type: 'string' }, action: { enum: ['fixed', 'declined', 'escalated'] }, note: { type: 'string' } } } },
    gates: { type: 'array', items: { type: 'object' } }, commits: { type: 'array', items: { type: 'string' } },
  },
}

// ---- Scope selection (full vs regression) ----------------------------------
const acceptanceTrack = allTracks.find(t => t.type === 'acceptance')
let tracks = allTracks.filter(t => t.type !== 'acceptance')
let stories = acceptanceTrack?.stories ?? []
if (mode === 'regression') {
  tracks = tracks.filter(t => !t.repo || (regressionScope.repos ?? []).includes(t.repo))
  stories = stories.filter(s => s.priority === 'P1' || (s.scenarios ?? []).some(sc => (regressionScope.scenarioIds ?? []).includes(sc.id)))
  log(`Regression scope: ${tracks.length} tracks, ${stories.length} stories`)
}
// Risk-based ordering (ISTQB): high-risk items first — under the concurrency
// cap, earlier thunks start sooner, so the highest risks fail fastest.
const riskRank = (x) => ({ high: 0, medium: 1, low: 2 })[x?.risk] ?? (x?.priority === 'P1' ? 0 : 1)
tracks = [...tracks].sort((a, b) => riskRank(a) - riskRank(b))
stories = [...stories].sort((a, b) => riskRank(a) - riskRank(b))

const stackPrompt = (action, extra = '') => `FEATURE_ID=${featureId}
ACTION=${action}
STACK=${JSON.stringify(verification.stack ?? null, null, 2)}
${extra}
Execute per your agent instructions and return the report JSON.`

const trackPrompt = (t, prior) => `FEATURE_ID=${featureId}
SPEC_DIR=${specDir}
TRACK=${JSON.stringify(t, null, 2)}
ENDPOINTS=${JSON.stringify(stackState?.endpoints ?? {}, null, 2)}
PRIOR (already fixed — verify, don't re-litigate): ${JSON.stringify(prior, null, 2)}
Execute this track per your agent instructions and return the report JSON.`

const storyPrompt = (s, prior) => `You are verifying user story ${s.story} (priority ${s.priority ?? '?'}) of workspace feature ${featureId} like a manual QA tester, headlessly.

Context to read first: ${specDir}/spec.md (this story's acceptance scenarios are ground truth), ${specDir}/plan.md.
Running stack endpoints: ${JSON.stringify(stackState?.endpoints ?? {}, null, 2)}
Stack notes (auth/test users): ${JSON.stringify(verification.stack?.notes ?? [], null, 2)}

Walk EVERY scenario below in order (serially — they may share state). For each: perform the Given/When/Then through the real UI/API, capture a screenshot at the decisive assertion into ${evidenceDir}/${s.story}/, and on failure escalate to console/network inspection before concluding. A scenario nobody can reach (broken nav, login wall) is "blocked", not "fail".
${JSON.stringify(s.scenarios ?? [], null, 2)}

Previously fixed defects (verify the fix instead of re-reporting): ${JSON.stringify(prior, null, 2)}
${s.persist_spec_to ? `If ALL scenarios pass, emit a durable Playwright spec reproducing these walks to ${s.persist_spec_to} (tag it @forge-acceptance; deterministic selectors; no sleeps) and report its path as emitted_spec.` : ''}
For each defect: name the repo at fault (${repos.map(r => r.repo).join(', ')}; default ${defaultDefectRepo}) and a concrete fix_direction.
Return the report JSON only.`

const fixPrompt = (r, items) => `MODE=fix
FEATURE_ID=${featureId}
REPO=${r.repo}
WORKTREE=${r.worktree}
BASE_BRANCH=${r.base}

CONTEXT — live-verification failures attributed to this repo (from test tracks and headless acceptance walks). Address every item, re-run the full gate pass, return the report JSON with per-item dispositions keyed by id:
${JSON.stringify(items, null, 2)}

Gate commands:
${(r.gates ?? []).map(g => `- ${g}`).join('\n') || '- derive from the repo CLAUDE.md'}`

// ---- Boot -------------------------------------------------------------------
let stackState = null
let booted = false
if (verification.stack) {
  log('Booting verification stack from feature worktrees')
  stackState = await agent(stackPrompt('boot'), { agentType: 'vf-stack-runner', model: 'sonnet', effort: 'medium', label: 'stack:boot', phase: 'Boot', schema: STACK_SCHEMA })
  if (!stackState || (stackState.status !== 'up' && stackState.status !== 'degraded')) {
    // "Teardown, always": a failed boot may have partially started the ephemeral
    // project — tear it down before returning. port-conflict starts nothing.
    let cleanup = 'not-needed'
    if (stackState?.status !== 'port-conflict') {
      const down = await agent(stackPrompt('teardown', `MANIFEST=${stackState?.manifest ?? ''}`), { agentType: 'vf-stack-runner', model: 'sonnet', effort: 'medium', label: 'stack:down', phase: 'Teardown', schema: STACK_SCHEMA }).catch(() => null)
      cleanup = down?.status ?? 'teardown-unconfirmed'
      if (cleanup !== 'torn-down') log('WARNING: failed-boot teardown did not confirm — check the stack manifest by hand')
    }
    return { featureId, mode, status: stackState?.status === 'port-conflict' ? 'stack-conflict' : 'stack-failed', stack: stackState, cleanup }
  }
  booted = true
}

const trackKey = (t) => `${t.type}:${t.repo ?? '*'}:${t.cmd ?? t.suites?.join(',') ?? ''}`
const lastTrack = {}   // key -> last result
const lastStory = {}   // story -> last result
const prior = { tracks: {}, stories: {} }
const externalDebt = [] // failures forge must not fix here (repo w/o worktree)
let fixRoundsRun = 0
let persistedSpecs = [] // emitted specs that were actually routed to a commit (ledger truth)

try {
  let pendingTracks = tracks, pendingStories = stories
  for (let round = 0; (pendingTracks.length || pendingStories.length) && round <= maxFixRounds; round++) {
    if (budget.total && budget.remaining() < 60_000) { log('budget floor — stopping verify loop'); break }
    log(`Verify round ${round + 1}: ${pendingTracks.length} tracks ∥ ${pendingStories.length} stories`)
    const thunks = [
      ...pendingTracks.map(t => () => agent(trackPrompt(t, prior.tracks[trackKey(t)] ?? []), { agentType: 'vf-live-tester', model: 'sonnet[1m]', effort: 'medium', label: `track:${t.type}${t.repo ? ':' + t.repo : ''}`, phase: `Verify r${round + 1}`, schema: TRACK_SCHEMA })),
      ...pendingStories.map(s => () => agent(storyPrompt(s, prior.stories[s.story] ?? []), { agentType: 'playwright-verifier', model: 'sonnet[1m]', effort: 'high', label: `qa:${s.story}`, phase: `Verify r${round + 1}`, schema: SCENARIO_SCHEMA })),
    ]
    const results = await parallel(thunks)
    const trackRes = results.slice(0, pendingTracks.length)
    const storyRes = results.slice(pendingTracks.length)
    pendingTracks.forEach((t, i) => { lastTrack[trackKey(t)] = trackRes[i] ?? { track: trackKey(t), status: 'env-broken', failures: [], note: 'agent died' } })
    pendingStories.forEach((s, i) => { lastStory[s.story] = storyRes[i] ?? { story: s.story, status: 'blocked', results: [], note: 'agent died' } })

    // Collect fixable failures, attributed per repo with a worktree.
    const byRepo = {}
    const envIssues = []
    pendingTracks.forEach((t, i) => {
      for (const f of trackRes[i]?.failures ?? []) {
        const id = `${trackKey(t)}::${f.test}`
        if (f.attribution === 'env-issue') { envIssues.push({ id, ...f }); continue }
        const target = byName[f.repo] ? f.repo : (f.attribution === 'test-bug' ? f.repo : defaultDefectRepo)
        if (byName[target]) (byRepo[target] ??= []).push({ id, source: 'track', ...f })
        else externalDebt.push({ id, source: 'track', ...f, note: `repo '${f.repo ?? target}' has no feature worktree — not fixed by this run` })
      }
    })
    pendingStories.forEach((s, i) => {
      for (const sc of storyRes[i]?.results ?? []) {
        if (sc.status === 'pass' || !sc.defect) continue
        const target = byName[sc.defect.repo] ? sc.defect.repo : defaultDefectRepo
        if (byName[target]) (byRepo[target] ??= []).push({ id: sc.id, source: 'acceptance', test: `${s.story}/${sc.id}`, ...sc.defect, symptom: sc.defect.symptom ?? sc.evidence })
        else externalDebt.push({ id: sc.id, source: 'acceptance', ...sc.defect })
      }
    })

    // Next pending = what actually failed/blocked this round.
    pendingTracks = pendingTracks.filter((t, i) => (trackRes[i]?.status ?? 'env-broken') !== 'pass')
    pendingStories = pendingStories.filter((s, i) => (storyRes[i]?.status ?? 'blocked') !== 'pass')
    const targets = Object.keys(byRepo)
    if (!targets.length && !envIssues.length) break
    if (round === maxFixRounds) { log('max fix rounds reached — reporting residual'); break }

    if (targets.length) {
      log(`Fix round ${round + 1}: ${targets.map(t => `${t}(${byRepo[t].length})`).join(', ')}`)
      const fixes = await parallel(targets.map(t => () =>
        agent(fixPrompt(byName[t], byRepo[t]), { agentType: 'vf-implementer', model: 'sonnet[1m]', effort: 'high', label: `fix:${t}`, phase: `Fix r${round + 1}`, schema: FIX_SCHEMA })))
      fixRoundsRun++
      targets.forEach((t, i) => {
        for (const d of fixes[i]?.dispositions ?? []) {
          const item = byRepo[t].find(x => x.id === d.id)
          if (!item) continue
          const entry = { id: d.id, symptom: item.symptom, action: d.action, note: d.note ?? '' }
          if (item.source === 'acceptance') { const st = stories.find(s => (s.scenarios ?? []).some(sc => sc.id === d.id))?.story; if (st) (prior.stories[st] ??= []).push(entry) }
          else { const k = Object.keys(lastTrack).find(k2 => d.id.startsWith(k2)); if (k) (prior.tracks[k] ??= []).push(entry) }
        }
      })
      // Restart services owned by fixed repos so re-runs hit fresh code.
      if (booted) await agent(stackPrompt('restart', `RESTART_REPOS=${JSON.stringify(targets)}\nMANIFEST=${stackState?.manifest ?? ''}`), { agentType: 'vf-stack-runner', model: 'sonnet', effort: 'medium', label: 'stack:restart', phase: `Fix r${round + 1}`, schema: STACK_SCHEMA })
    } else if (envIssues.length) {
      // Env-only failures: one stack heal attempt, then re-run.
      log('Env issues only — asking the stack runner to heal and re-checking')
      const st = await agent(stackPrompt('restart', `RESTART_REPOS=${JSON.stringify(repos.map(r => r.repo))}\nENV_ISSUES=${JSON.stringify(envIssues)}\nMANIFEST=${stackState?.manifest ?? ''}`), { agentType: 'vf-stack-runner', model: 'sonnet', effort: 'medium', label: 'stack:heal', phase: `Fix r${round + 1}`, schema: STACK_SCHEMA })
      if (!st || st.status === 'failed') { log('stack heal failed — stopping'); break }
    }
  }

  // Persist emitted acceptance specs via a short implementer commit per repo.
  // The ledger's emitted_specs must reflect COMMITTED specs only — gate on success.
  const emitted = Object.values(lastStory).map(s => s?.emitted_spec).filter(e => e?.path && byName[e.repo])
  if (emitted.length) {
    const perRepo = {}
    emitted.forEach(e => (perRepo[e.repo] ??= []).push(e.path))
    const repoNames = Object.keys(perRepo)
    const persistRes = await parallel(repoNames.map(t => () =>
      agent(`MODE=fix\nFEATURE_ID=${featureId}\nREPO=${t}\nWORKTREE=${byName[t].worktree}\nBASE_BRANCH=${byName[t].base}\n\nCONTEXT — commit these @forge-acceptance Playwright specs emitted by the acceptance walk. Place them per the repo's e2e test conventions, keep them OUT of the default gate commands (they need a live stack), lint them, commit. Report dispositions.\n${JSON.stringify(perRepo[t], null, 2)}`,
        { agentType: 'vf-implementer', model: 'sonnet', effort: 'medium', label: `persist:${t}`, phase: 'Persist', schema: FIX_SCHEMA })))
    const okRepos = new Set(repoNames.filter((t, i) => persistRes[i]?.status === 'green'))
    persistedSpecs = emitted.filter(e => okRepos.has(e.repo))
    const failed = repoNames.filter(t => !okRepos.has(t))
    if (failed.length) log(`WARNING: acceptance-spec persist did not confirm for ${failed.join(', ')} — excluded from emitted_specs`)
  }
} finally {
  if (booted) {
    const down = await agent(stackPrompt('teardown', `MANIFEST=${stackState?.manifest ?? ''}`), { agentType: 'vf-stack-runner', model: 'sonnet', effort: 'medium', label: 'stack:down', phase: 'Teardown', schema: STACK_SCHEMA }).catch(() => null)
    if (!down || down.status !== 'torn-down') log('WARNING: teardown did not confirm — check the stack manifest by hand')
  }
}

// ---- Verdict ------------------------------------------------------------------
const trackOut = tracks.map(t => ({ ...t, result: lastTrack[trackKey(t)] ?? null }))
const storyOut = stories.map(s => ({ story: s.story, priority: s.priority, risk: s.risk, result: lastStory[s.story] ?? null }))
const anyFail = trackOut.some(t => t.result?.status === 'fail') || storyOut.some(s => s.result?.status === 'fail')
const anyEnv = trackOut.some(t => (t.result?.status ?? 'env-broken') === 'env-broken') || storyOut.some(s => (s.result?.status ?? 'blocked') === 'blocked')
const status = anyFail ? 'fail' : anyEnv ? 'env-degraded' : 'pass'

// Risk coverage: map the architect's product-risk register onto what actually
// ran — residual risk is reported, never silently dropped (ISTQB exit report).
const scenarioStatus = {}
storyOut.forEach(s => (s.result?.results ?? []).forEach(sc => { scenarioStatus[sc.id] = sc.status }))
const refStatus = (ref) => {
  if (typeof ref !== 'string') return 'unresolved'
  if (ref.startsWith('track:')) {
    const [, type, repo] = ref.split(':')
    const t = trackOut.find(t2 => t2.type === type && (!repo || t2.repo === repo))
    return t ? (t.result?.status ?? 'not-run') : 'unresolved'
  }
  if (scenarioStatus[ref]) return scenarioStatus[ref]
  return 'out-of-band' // task/test IDs proven by implementer gates, not this run
}
const riskCoverage = (verification.risk_register ?? []).map(r => {
  const st = (r.mitigated_by ?? []).map(refStatus)
  const rstatus = !st.length ? 'unmitigated'
    : st.some(s => s === 'fail' || s === 'env-broken' || s === 'blocked') ? 'at-risk'
    : st.every(s => s === 'pass' || s === 'out-of-band') ? 'mitigated'
    : 'partially-verified'
  return { id: r.id, risk: r.risk, level: r.level, status: rstatus, refs: (r.mitigated_by ?? []).map((m, i) => `${m}=${st[i]}`) }
})

return {
  featureId, mode, status,
  stack: { status: stackState?.status ?? 'not-declared', manifest: stackState?.manifest ?? null,
           isolation: stackState?.isolation ?? verification.stack?.isolation ?? null },
  tracks: trackOut, acceptance: storyOut,
  risk_coverage: riskCoverage,
  fix_rounds: fixRoundsRun,
  external_test_debt: externalDebt,
  emitted_specs: persistedSpecs,
  evidence_dir: evidenceDir,
}
