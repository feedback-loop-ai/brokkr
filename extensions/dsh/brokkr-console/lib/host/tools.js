// The assistant's tools (slice two, design D2 and D3): DSH's model reads
// Brokkr through its own readouts, and may suggest one command the operator
// can press. Nothing here appends to a journal: every tool runs a readout,
// and `brokkr_suggest` only reads the run and answers with a card or with
// Brokkr's own refusal. The definitions are plain objects, because a package
// outside DSH's tree cannot import `defineTool`, so each tool checks its own
// arguments before any process starts.
import { run } from './runner.js'

/** The most of a readout the model is given; past it the answer says it was cut. */
export const TOOL_BOUND_BYTES = 256 * 1024

const COMMANDS = ['retry', 'stop', 'resume', 'conclude']
const ACTIONS = ['dismiss_finding', 'talk_to_reviewer', 'hold', 'answer_here']

/** The id grammar the command desk reads, and those of a realm and a seat. */
const GRAMMAR = {
  run: /^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$/,
  realm: /^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$/,
  seat: /^[A-Za-z0-9][A-Za-z0-9._:/-]{0,127}$/,
}

/** A call the tool will not make, with the words the model is given. */
export class ToolRefused extends Error {
  constructor(message) {
    super(message)
    this.name = 'ToolRefused'
  }
}

/** The arguments object, each named field checked against its grammar. */
function argsOf(args, required, optional = []) {
  if (args === null || typeof args !== 'object' || Array.isArray(args)) throw new ToolRefused('the arguments are not an object')
  const checked = {}
  for (const name of [...required, ...optional]) {
    const value = args[name]
    if (value === undefined && optional.includes(name)) continue
    if (typeof value !== 'string' || !GRAMMAR[name].test(value)) throw new ToolRefused(`"${name}" is not a ${name} id this console reads`)
    checked[name] = value
  }
  return checked
}

/** A readout's output: its JSON, or its first bytes when it is past the bound. */
function boundedOf(ran, what) {
  if (ran.kind === 'unstarted') throw new ToolRefused(`brokkr could not be started for ${what}: ${ran.message}`)
  if (ran.kind === 'timeout') throw new ToolRefused(`${what} took longer than the console waits`)
  if (ran.kind === 'overflow') throw new ToolRefused(`${what} printed more than the console reads`)
  if (ran.code !== 0) throw new ToolRefused(ran.stderr.trim() || `${what} failed`)
  const bytes = Buffer.from(ran.stdout)
  if (bytes.length > TOOL_BOUND_BYTES) {
    return { cut: true, text: bytes.subarray(0, TOOL_BOUND_BYTES).toString('utf8'), note: `cut at ${TOOL_BOUND_BYTES} of ${bytes.length} bytes` }
  }
  try {
    return { cut: false, readout: JSON.parse(ran.stdout) }
  } catch {
    throw new ToolRefused(`${what} printed something that is not JSON`)
  }
}

const object = { type: 'object' }
const prop = (description) => ({ type: 'string', description })

/** A plain tool definition whose answer is shown to the model as JSON. */
function tool(name, description, parameters, execute, presentationMeta) {
  return {
    name,
    description,
    parameters: { type: 'object', additionalProperties: false, ...parameters },
    output: {
      schema: object,
      render: (_args, value) => [{ type: 'text', text: JSON.stringify(value) }],
      ...(presentationMeta === undefined ? {} : { presentationMeta }),
    },
    isConcurrencySafe: () => true,
    async execute(args) {
      try {
        return await execute(args)
      } catch (error) {
        if (error instanceof ToolRefused) return { refused: error.message }
        throw error
      }
    },
  }
}

/**
 * The tools, over the configured `brokkr` and the console's parsed readouts.
 * @param {{config: object, brokkr: object}} parts
 */
export function toolsOf({ config, brokkr }) {
  const raw = async (argv, what) => boundedOf(await run(config, argv), what)

  /** The run's journal and listing row, from a fresh fleet: a realm, or the one hearth listing it. */
  async function locate(runId, realm) {
    const fleet = await brokkr.fleet()
    const hearths = fleet.hearths.filter((each) => realm === undefined || each.realm === realm)
    const found = hearths.flatMap((hearth) => hearth.runs.filter((row) => row.run_id === runId).map((row) => ({ hearth, row })))
    if (found.length === 0) throw new ToolRefused(`no run "${runId}" is listed${realm === undefined ? '' : ` in realm ${realm}`}`)
    if (found.length > 1) throw new ToolRefused(`run "${runId}" is listed in more than one realm; name the realm`)
    const [{ hearth, row }] = found
    return { target: { run_id: runId, journal: hearth.journal }, realm: hearth.realm, row }
  }

  const runFlags = (target) => [`--run=${target.run_id}`, ...(target.journal === null ? [] : [`--db=${target.journal}`])]

  return [
    tool(
      'brokkr_runs',
      'Read-only. Every Brokkr run in every realm the console reads, as brokkr runs --json prints it: title, standing, phase, what it needs. Cite runs by title.',
      { properties: { realm: prop('Only this realm') } },
      async (args) => {
        const { realm } = argsOf(args, [], ['realm'])
        const answer = await raw(['runs', '--json'], 'brokkr runs')
        // `brokkr runs` lists every hearth; one realm is kept here, after the read.
        if (realm !== undefined && !answer.cut && Array.isArray(answer.readout.realms)) {
          answer.readout = { ...answer.readout, realms: answer.readout.realms.filter((each) => each.realm === realm) }
        }
        return { source: 'brokkr runs --json', ...answer }
      },
    ),
    tool(
      'brokkr_inspect',
      'Read-only. One run as brokkr inspect --json prints it: its rulings with their sequence numbers, seats, timeline, the commands Brokkr would admit and the actions it lacks. Cite rulings by sequence number.',
      { properties: { run: prop('The run id'), realm: prop('The realm, when the id is listed in more than one') }, required: ['run'] },
      async (args) => {
        const { run: runId, realm } = argsOf(args, ['run'], ['realm'])
        const { target } = await locate(runId, realm)
        return { source: `brokkr inspect --run=${runId} --json`, ...(await raw(['inspect', ...runFlags(target), '--json'], 'brokkr inspect')) }
      },
    ),
    tool(
      'brokkr_queue',
      'Read-only. The queued runs and Brokkr’s admission verdict on each, as brokkr queue list --json prints them.',
      { properties: {} },
      async (args) => {
        argsOf(args, [])
        return { source: 'brokkr queue list --json', ...(await raw(['queue', 'list', '--json'], 'brokkr queue list')) }
      },
    ),
    tool(
      'brokkr_transcript',
      'Read-only. One seat’s retained transcript of a run, through Brokkr’s bounded derivation (brokkr transcript --json). Seats are named in brokkr_inspect’s participants.',
      { properties: { run: prop('The run id'), seat: prop('A participant key or unique label'), realm: prop('The realm, when needed') }, required: ['run', 'seat'] },
      async (args) => {
        const { run: runId, seat, realm } = argsOf(args, ['run', 'seat'], ['realm'])
        const { target } = await locate(runId, realm)
        return { source: `brokkr transcript --run=${runId} --seat=${seat} --json`, ...(await raw(['transcript', ...runFlags(target), `--seat=${seat}`, '--json'], 'brokkr transcript')) }
      },
    ),
    tool(
      'brokkr_suggest',
      'Show the operator one command card for a run. It presses nothing: the operator decides. A card appears only when Brokkr admits the command for the run right now; otherwise you get Brokkr’s reason, which you should tell the operator.',
      {
        properties: {
          run: prop('The run id'),
          realm: prop('The realm, when needed'),
          command: { type: 'string', enum: [...COMMANDS, ...ACTIONS], description: 'The command to suggest' },
          reason: { type: 'string', description: 'Why, in one sentence the operator reads' },
        },
        required: ['run', 'command', 'reason'],
      },
      async (args) => {
        const { run: runId, realm: named } = argsOf(args, ['run'], ['realm'])
        const { command, reason } = args
        if (![...COMMANDS, ...ACTIONS].includes(command)) throw new ToolRefused(`"${command}" is not a command Brokkr has`)
        if (typeof reason !== 'string' || reason.trim() === '' || reason.length > 2000) throw new ToolRefused('a reason of one sentence is required')
        const { target, realm, row } = await locate(runId, named)
        const view = await brokkr.inspect(target)
        if (ACTIONS.includes(command)) {
          const lacking = view.unavailable.find((entry) => entry.action === command)
          return { suggested: false, text: lacking?.reason ?? `Brokkr has no "${command}" for this run` }
        }
        const offered = view.commands.find((entry) => entry.command === command)
        if (offered === undefined || view.summary === null) return { suggested: false, text: 'the run’s record does not fold, so Brokkr admits nothing' }
        // Brokkr's own words for the refusal, said as what would happen: the suggestion journals nothing.
        if (offered.admission === 'refused') return { suggested: false, text: `If pressed now, Brokkr would refuse it: ${offered.text}` }
        return { suggested: true, realm, run_id: runId, title: row.title, command, phase: offered.phase, seq: view.summary.seq, reason: reason.trim() }
      },
      (_args, value) => value,
    ),
  ]
}
