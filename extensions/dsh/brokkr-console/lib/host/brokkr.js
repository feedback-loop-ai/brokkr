// The readouts and verbs the console uses, each one argv built in one place
// (decision 0071 ruling 10) and each readout parsed by `readouts.js`. A run is
// addressed by its id and, in a world of several hearths, the journal its
// realm's listing names (`--db`), never by a path the browser sent. Every
// value is passed as `--flag=value`, so a value that opens with `-` (a reason
// can) is never read as a flag.
import { launch, run } from './runner.js'
import { analyticsOf, fleetOf, jsonOf, muninnOf, queueOf, realmsOf, runViewOf, transcriptReadOf } from './readouts.js'

/** The two `brokkr operator` commands the console presses. */
const OPERATOR = ['retry', 'stop']

const flag = (name, value) => `--${name}=${value}`

/**
 * The bundle a run is resumed under: the operator's override when one is
 * set, else the recipe its start recorded, else `null` and nothing is sent.
 */
export function resumeSource(override, recipe) {
  if (override.kind === 'bundle') return flag('bundle', override.path)
  if (override.kind === 'recipe') return flag('recipe', override.name)
  return recipe === null || recipe === undefined ? null : flag('recipe', recipe)
}

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
    realms: () => read(['realms', '--json'], 'brokkr realms', realmsOf),
    /**
     * One seat's transcript, by the participant key the run view names. A seat
     * that keeps no transcript (a script seat records `none`) is answered by the
     * readout's own document, read whatever the exit code (`transcriptReadOf`).
     */
    transcript: async (target, seat) => transcriptReadOf(await run(config, ['transcript', ...runOf(target), flag('seat', seat), '--json']), 'brokkr transcript'),
    analytics: (range, realm) => {
      const only = realm === null ? [] : [flag('realm', realm)]
      return read(['analytics', flag('range', range), ...only, '--json'], 'brokkr analytics', analyticsOf)
    },
    /** `brokkr operator retry|stop` or `brokkr conclude`, with the reason recorded. */
    command(target, command, reason) {
      const verb = OPERATOR.includes(command) ? ['operator', command] : ['conclude']
      return run(config, [...verb, ...runOf(target), flag('reason', reason)])
    },
    /**
     * `brokkr resume`, detached: it drives the run for as long as the run
     * moves, under the operator's override or else the recipe the run's
     * start recorded (`view.recipe`).
     */
    resume(target, view, name) {
      const source = resumeSource(config.resume, view.recipe)
      const repo = view.worktree === null ? [] : [flag('repo', view.worktree)]
      return launch(config, ['resume', ...runOf(target), source, ...repo, '--no-view'], name)
    },
  }
}
