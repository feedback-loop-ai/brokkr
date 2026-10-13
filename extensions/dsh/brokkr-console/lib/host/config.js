// The console's configuration, read once from the environment DSH is started
// in. Everything has a default except what `brokkr resume` needs: Brokkr's
// readouts do not name the bundle a run was started under, so the operator
// names the one runs are resumed under, and a retry is not sent without it.
import { tmpdir } from 'node:os'
import { join } from 'node:path'

const READOUT_TIMEOUT_MS = 30_000
const OUTPUT_BOUND_BYTES = 16 * 1024 * 1024
const POLL_MS = 5000
const POLL_FLOOR_MS = 1000

/**
 * The bundle `brokkr resume` is given: `--bundle <path>` or `--recipe <name>`,
 * or `none` when the environment names neither or both.
 * @typedef {{kind: 'bundle', path: string} | {kind: 'recipe', name: string} | {kind: 'none'}} ResumeSource
 */

/** @returns {ResumeSource} */
function resumeSource(env) {
  const bundle = env.BROKKR_CONSOLE_RESUME_BUNDLE || null
  const recipe = env.BROKKR_CONSOLE_RESUME_RECIPE || null
  if (bundle !== null && recipe === null) return { kind: 'bundle', path: bundle }
  if (recipe !== null && bundle === null) return { kind: 'recipe', name: recipe }
  return { kind: 'none' }
}

/** The configuration `env` describes, `cwd` the directory DSH runs in. */
export function configFrom(env, cwd) {
  const poll = Number(env.BROKKR_CONSOLE_POLL_MS)
  return {
    program: [env.BROKKR_CONSOLE_BROKKR || 'brokkr'],
    cwd: env.BROKKR_CONSOLE_WORKSPACE || cwd,
    env: { ...env },
    timeoutMs: READOUT_TIMEOUT_MS,
    maxBytes: OUTPUT_BOUND_BYTES,
    pollMs: Number.isFinite(poll) && poll > 0 ? Math.max(poll, POLL_FLOOR_MS) : POLL_MS,
    stateDir: join(tmpdir(), `brokkr-console-${process.getuid()}`),
    resume: resumeSource(env),
  }
}
