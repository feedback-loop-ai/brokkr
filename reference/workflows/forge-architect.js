export const meta = {
  name: 'forge-architect',
  description: 'Native Claude-only architect council: three contrarian Opus seats write independent position papers, clash adversarially, and a fork adjudicator separates agreed from contested questions; provider-neutral council output can also be supplied to a Fable chief (Opus fallback) for synthesis',
  whenToUse: "Invoked by /forge Phase A. mode:'council' is the Claude-only council (read-only, pre-branch); mode:'synthesize' accepts either that result or a provider-neutral council assembled by the shared Forge control plane. The intake ask happens between the modes.",
  phases: [
    { title: 'Positions', detail: 'three seats write independent papers', model: 'opus/max' },
    { title: 'Clash', detail: 'mandatory objections + concessions + revised positions', model: 'opus/max' },
    { title: 'Adjudicate', detail: 'agreed vs genuinely contested questions', model: 'sonnet' },
    { title: 'Synthesize', detail: 'chief architect: steal shamelessly, decide ruthlessly, record dissent', model: 'fable' },
  ],
}

// args (mode 'council'):    {mode, featureDescription, candidateRepos?, charterPath?}
// args (mode 'synthesize'): {mode, featureDescription, council?, councilPath?, userAnswers?, resume?}
//   council: the full mode:'council' result (or null → solo chief, today's behavior)
//   councilPath: absolute path to the full council result JSON on disk. When set, the bulky
//     sections (positions, clash revisions, objections, concessions) are handed to the chief as
//     a file to Read rather than inlined — the council result routinely exceeds 250KB, which is
//     impractical to pass through an inline tool argument. agreed/contested stay inline because
//     they are small and decision-critical. `council` may then carry only those two arrays.
const A = typeof args === 'string' ? (() => { try { return JSON.parse(args) } catch { return {} } })() : (args ?? {})
const {
  mode = 'council',
  featureDescription,
  candidateRepos = null,
  charterPath = '.claude/skills/forge/references/architect-council.md',
  council = null,
  councilPath = null,
  userAnswers = null,
  resume = null,
} = A
if (!featureDescription) return { status: 'noop', reason: 'no featureDescription provided', mode }

const SEATS = ['simplifier', 'systems-architect', 'operator']

const POSITION_SCHEMA = {
  type: 'object',
  required: ['seat', 'summary', 'affected_repos', 'open_questions'],
  properties: {
    seat: { type: 'string' },
    approach_title: { type: 'string' },
    summary: { type: 'string' },
    affected_repos: { type: 'array', items: { type: 'object', required: ['name'], properties: { name: { type: 'string' }, role: { type: 'string' }, wave: { type: 'number' } } } },
    contracts: { type: 'array', items: { type: 'object', properties: { name: { type: 'string' }, producer: { type: 'string' }, consumers: { type: 'array', items: { type: 'string' } }, check: { type: 'string' } } } },
    risk_register: { type: 'array', items: { type: 'object', properties: { risk: { type: 'string' }, likelihood: { type: 'string' }, impact: { type: 'string' }, level: { type: 'string' } } } },
    assumptions: { type: 'array', items: { type: 'string' } },
    open_questions: { type: 'array', items: { type: 'object', required: ['id', 'question', 'my_answer'], properties: { id: { type: 'string' }, question: { type: 'string' }, my_answer: { type: 'string' }, confidence: { enum: ['high', 'medium', 'low'] } } } },
    what_would_make_me_wrong: { type: 'array', items: { type: 'string' } },
    objections_expected: { type: 'array', items: { type: 'string' } },
  },
}
const CLASH_SCHEMA = {
  type: 'object',
  required: ['seat', 'objections', 'final_answers'],
  properties: {
    seat: { type: 'string' },
    objections: { type: 'array', items: { type: 'object', required: ['target_seat', 'assumption', 'why_load_bearing'], properties: { target_seat: { type: 'string' }, assumption: { type: 'string' }, why_load_bearing: { type: 'string' }, evidence: { type: 'string' } } } },
    concessions: { type: 'array', items: { type: 'object', required: ['target_seat', 'point'], properties: { target_seat: { type: 'string' }, point: { type: 'string' } } } },
    revised: { type: 'object', properties: { summary: { type: 'string' }, key_changes: { type: 'array', items: { type: 'string' } } } },
    final_answers: { type: 'array', items: { type: 'object', required: ['id', 'answer'], properties: { id: { type: 'string' }, question: { type: 'string' }, answer: { type: 'string' } } } },
  },
}
const ADJ_SCHEMA = {
  type: 'object',
  required: ['agreed', 'contested'],
  properties: {
    agreed: { type: 'array', items: { type: 'object', required: ['id', 'answer'], properties: { id: { type: 'string' }, question: { type: 'string' }, answer: { type: 'string' } } } },
    contested: { type: 'array', items: { type: 'object', required: ['id', 'question', 'options'], properties: { id: { type: 'string' }, question: { type: 'string' }, options: { type: 'array', items: { type: 'string' } }, why_it_matters: { type: 'string' } } } },
  },
}
const ARCH_SCHEMA = {
  type: 'object',
  required: ['feature_id', 'spec_dir', 'repos'],
  properties: {
    feature_id: { type: 'string' }, spec_dir: { type: 'string' }, workspace_branch: { type: 'string' }, story_url: { type: 'string' },
    repos: { type: 'array', items: { type: 'object', required: ['name'], properties: { name: { type: 'string' }, base: { type: 'string' }, gates: { type: 'array', items: { type: 'string' } } } } },
    waves: { type: 'array', items: { type: 'array', items: { type: 'string' } } },
    contracts: { type: 'array', items: { type: 'object' } },
    verification: { type: 'object' }, lead_repo: { type: 'string' },
    clarify_loops: { type: 'number' }, analyze_loops: { type: 'number' },
    assumptions: { type: 'array', items: { type: 'string' } }, risks: { type: 'array', items: { type: 'string' } },
    dissents: { type: 'array', items: { type: 'object', properties: { seat: { type: 'string' }, objection: { type: 'string' }, ruling: { type: 'string' }, risk_register_entry: { type: 'string' } } } },
  },
}

// ---- mode: council ----------------------------------------------------------
if (mode === 'council') {
  const seatPrompt = (seat) => `You are the **${seat}** seat of the Forge Architect Council.
Read your charter and the protocol FIRST: ${charterPath} (your seat's section + the shared mechanics). You are read-only: research and write your position paper only — no branches, no edits, no worktrees.

FEATURE_DESCRIPTION (verbatim):
${featureDescription}

CANDIDATE_REPOS: ${candidateRepos ? JSON.stringify(candidateRepos) : 'not constrained — derive from repos.yaml (skip archived)'}

Gather the evidence YOUR charter names (not the other seats'), then return your position paper as JSON per the charter's format. Write it blind — you have not seen the other seats' papers. Include objections_expected: where you expect the conventional design to be wrong.`

  log('Council: three seats writing independent position papers')
  const positions = await parallel(SEATS.map(seat => () =>
    agent(seatPrompt(seat), { model: 'opus', effort: 'max', label: `seat:${seat}`, phase: 'Positions', schema: POSITION_SCHEMA })))
  const seated = SEATS.map((seat, i) => ({ seat, position: positions[i] })).filter(s => s.position)
  if (seated.length < 2) return { status: 'council-failed', reason: `only ${seated.length}/3 seats returned a position`, positions: seated }

  // Union of open questions, namespaced by originating seat.
  const questions = seated.flatMap(s => (s.position.open_questions ?? []).map(q => ({ ...q, id: `${s.seat}:${q.id}` })))

  if (budget.total && budget.remaining() < 60_000) {
    log('budget floor — returning positions without a clash round (degraded)')
    return { status: 'degraded-no-clash', mode, positions: seated, questions, clashes: [], agreed: [], contested: [], objections: [] }
  }

  const clashPrompt = (s) => `You are the **${s.seat}** seat of the Forge Architect Council — clash round.
Re-read the protocol in ${charterPath}. Objection is a deliverable: you MUST name the weakest load-bearing assumption in EACH other paper (≥1 per paper); "no objections" is a failed deliverable. Concede what is simply right. Then revise your own position.

YOUR position paper:
${JSON.stringify(s.position, null, 2)}

THE OTHER SEATS' papers:
${JSON.stringify(seated.filter(x => x.seat !== s.seat).map(x => x.position), null, 2)}

UNION OF OPEN QUESTIONS — answer EVERY one in final_answers, including ones you did not raise:
${JSON.stringify(questions.map(({ id, question }) => ({ id, question })), null, 2)}

Return the clash JSON per the protocol.`

  log('Council: clash round — mandatory objections and concessions')
  const clashes = (await parallel(seated.map(s => () =>
    agent(clashPrompt(s), { model: 'opus', effort: 'max', label: `clash:${s.seat}`, phase: 'Clash', schema: CLASH_SCHEMA })))).filter(Boolean)

  const answersByQ = questions.map(q => ({
    id: q.id, question: q.question,
    answers: clashes.map(c => ({ seat: c.seat, answer: c.final_answers?.find(a => a.id === q.id)?.answer ?? null })).filter(a => a.answer),
  }))
  const adjudication = await agent(`You adjudicate the Forge Architect Council's questions. For each question below, compare the seats' final answers. Classify as "agreed" (answers are materially the same or trivially reconcilable — synthesize the single answer) or "contested" (answers imply materially different designs, scope, or irreversible effects — list the distinct options verbatim-condensed and one line on why the fork matters). Be strict: only genuine forks are contested; wording differences are agreement. Return JSON {agreed, contested}.
${JSON.stringify(answersByQ, null, 2)}`,
    { model: 'sonnet', effort: 'low', label: 'adjudicate', phase: 'Adjudicate', schema: ADJ_SCHEMA })

  return {
    status: 'ok', mode,
    positions: seated,
    clashes,
    objections: clashes.flatMap(c => (c.objections ?? []).map(o => ({ from: c.seat, ...o }))),
    concessions: clashes.flatMap(c => (c.concessions ?? []).map(o => ({ from: c.seat, ...o }))),
    agreed: adjudication?.agreed ?? [],
    contested: adjudication?.contested ?? [],
  }
}

// ---- mode: synthesize ---------------------------------------------------------
const chiefPrompt = `${council ? 'You are the CHIEF ARCHITECT synthesizing the Forge Architect Council\'s work.' : 'You are the solo Forge Architect (no council convened for this feature size).'}
FEATURE_DESCRIPTION (verbatim):
${featureDescription}
CANDIDATE_REPOS: ${candidateRepos ? JSON.stringify(candidateRepos) : 'derive from repos.yaml (skip archived)'}
USER_ANSWERS (immutable constraints from the intake gate): ${userAnswers ? JSON.stringify(userAnswers, null, 2) : 'none'}
${resume ? `RESUME: ${JSON.stringify(resume)}` : ''}
${council ? `
COUNCIL OUTPUT — advisory, you decide:
${councilPath ? `- FULL COUNCIL RECORD ON DISK: ${councilPath}
  Read it FIRST (it is large by design), before any other research. It is the mode:'council'
  result JSON with keys: positions[] (each {seat, position} — approach_title, summary,
  affected_repos, contracts, risk_register, assumptions, open_questions,
  what_would_make_me_wrong), clashes[] (each {seat, objections, concessions, revised,
  final_answers}), objections[] (live unless conceded — each {from, target_seat, assumption,
  why_load_bearing, evidence}), concessions[], agreed[], contested[]. The seats cite exact
  file:line evidence; verify the load-bearing claims you build on rather than trusting them.`
  : `- Positions (post-clash revisions included): ${JSON.stringify(council.positions?.map(p => ({ seat: p.seat, position: p.position })), null, 2)}
- Clash revisions: ${JSON.stringify(council.clashes?.map(c => ({ seat: c.seat, revised: c.revised })), null, 2)}
- Objections (live unless conceded): ${JSON.stringify(council.objections, null, 2)}
- Concessions: ${JSON.stringify(council.concessions, null, 2)}`}
- Agreed answers (adopt unless clearly wrong; deviation = recorded decision): ${council.agreed ? JSON.stringify(council.agreed, null, 2) : 'see the agreed[] array in the full council record above'}
- Contested questions + operator ruling in USER_ANSWERS (or decide yourself if none): ${council.contested ? JSON.stringify(council.contested, null, 2) : 'see the contested[] array in the full council record above'}

Chief duties, non-negotiable:
1. Steal shamelessly: pick the winning skeleton, graft the best ideas from the losing positions.
2. Decide ruthlessly: no consensus-seeking, no splitting differences into incoherence.
3. Record dissent: every live objection you overrule goes into the spec's Clarifications/decision log as {seat, objection, ruling}; every overruled RISK objection additionally becomes a forge.verification.risk_register entry with a mitigated_by test — dissent gets a mechanical afterlife.
4. Return the dissents array in your output JSON alongside your standard contract.` : ''}

Execute your full agent flow (research as needed → specify → self-clarify → plan → tasks → analyze to zero) and return your output JSON contract${council ? ' extended with the dissents array' : ''}.`

log(council ? 'Chief architect synthesizing council output' : 'Solo architect (no council)')
let architectModel = 'fable'
let chief = null
try {
  chief = await agent(chiefPrompt, { agentType: 'vf-architect', model: 'fable', effort: 'max', label: 'chief:fable', phase: 'Synthesize', schema: ARCH_SCHEMA })
} catch (e) { chief = null }
if (!chief) {
  log('fable unavailable or failed — falling back to opus chief (recorded)')
  architectModel = 'opus (fable unavailable)'
  try {
    chief = await agent(chiefPrompt, { agentType: 'vf-architect', model: 'opus', effort: 'max', label: 'chief:opus', phase: 'Synthesize', schema: ARCH_SCHEMA })
  } catch (e) { chief = null }
}
if (!chief) return { status: 'architect-failed', mode, architect_model: architectModel }

return { status: 'ok', mode, architect_model: architectModel, ...chief }
