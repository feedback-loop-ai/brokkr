// The readouts and verbs the console uses, each one argv built in one place
// (decision 0071 ruling 10) and each readout parsed by `readouts.js`. A run is
// addressed by its id and, in a world of several hearths, the journal its
// realm's listing names (`--db`), never by a path the browser sent. Every
// value is passed as `--flag=value`, so a value that opens with `-` (a reason
// can) is never read as a flag.
import { launch, run } from './runner.js'
import { analyticsOf, fleetOf, jsonOf, muninnOf, queueOf, runViewOf } from './readouts.js'

/** The two `brokkr operator` commands the console presses. */
const OPERATOR = ['retry', 'stop']

const flag = (name, value) => `--${name}=${value}`

/** `--run=<id>`, and `--db=<journal>` for a run in a many-hearth world. */
function runOf(target) {
  const journal = target.journal === null ? [] : [flag('db', target.journal)]
  return [flag('run', target.run_id), ...journal]
}

/**
 * The console's view of the `brokkr` binary.
 * @param {object} config the runner's configuration, with `resume` (see config.js)
 */
export function brokkrOf(config) {
  const read = async (argv, what, parse) => parse(jsonOf(await run(config, argv), what))
  return {
    fleet: () => read(['runs', '--json'], 'brokkr runs', fleetOf),
    inspect: (target) => read(['inspect', ...runOf(target), '--json'], 'brokkr inspect', runViewOf),
    queue: () => read(['queue', 'list', '--json'], 'brokkr queue list', queueOf),
    muninn: () => read(['muninn', 'list', '--json'], 'brokkr muninn list', muninnOf),
    analytics: (range, realm) => {
      const only = realm === null ? [] : [flag('realm', realm)]
      return read(['analytics', flag('range', range), ...only, '--json'], 'brokkr analytics', analyticsOf)
    },
    /** `brokkr operator retry|stop` or `brokkr conclude`, with the reason recorded. */
    command(target, command, reason) {
      const verb = OPERATOR.includes(command) ? ['operator', command] : ['conclude']
      return run(config, [...verb, ...runOf(target), flag('reason', reason)])
    },
    /** `brokkr resume`, detached: it drives the run for as long as the run moves. */
    resume(target, worktree, name) {
      const source = config.resume.kind === 'bundle' ? flag('bundle', config.resume.path) : flag('recipe', config.resume.name)
      const repo = worktree === null ? [] : [flag('repo', worktree)]
      return launch(config, ['resume', ...runOf(target), source, ...repo, '--no-view'], name)
    },
  }
}
