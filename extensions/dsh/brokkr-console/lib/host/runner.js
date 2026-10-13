// Runs `brokkr` as the OS user, the way the CLI is run by hand. Every call is
// bounded: output past `maxBytes` and a call past `timeoutMs` end the process,
// and the call settles only once the process has exited, so whatever it wrote
// to a journal is there to be read when the caller looks.
import { spawn } from 'node:child_process'
import { closeSync, constants, lstatSync, mkdirSync, openSync, readFileSync, statSync } from 'node:fs'
import { join } from 'node:path'

/** How long a resumed run's verb is watched before the console reports on it. */
const RESUME_GRACE_MS = 3000

/** The most of a resume's own output the console reads back. */
const LOG_TAIL_BYTES = 64 * 1024

/**
 * What one call came to: `exited` with its code and output, `timeout`,
 * `overflow` (more output than the bound), or `unstarted` (the program
 * could not be spawned).
 * @typedef {{kind: 'exited', code: number, stdout: string, stderr: string}
 *   | {kind: 'timeout'} | {kind: 'overflow'} | {kind: 'unstarted', message: string}} Ran
 */

/**
 * Run `brokkr <argv>` under `config` and settle once it has exited.
 * @param {{program: string[], cwd: string, env: object, timeoutMs: number, maxBytes: number}} config
 * @param {string[]} argv
 * @returns {Promise<Ran>}
 */
export function run(config, argv) {
  const [command, ...prefix] = config.program
  return new Promise((settle) => {
    const child = spawn(command, [...prefix, ...argv], {
      cwd: config.cwd,
      env: config.env,
      stdio: ['ignore', 'pipe', 'pipe'],
    })
    const out = { stdout: [], stderr: [], bytes: 0, ended: null }
    const end = (kind) => {
      if (out.ended === null) {
        out.ended = kind
        child.kill('SIGKILL')
      }
    }
    const timer = setTimeout(() => end('timeout'), config.timeoutMs)
    const take = (stream) => (chunk) => {
      out.bytes += chunk.length
      if (out.bytes > config.maxBytes) end('overflow')
      else out[stream].push(chunk)
    }
    child.stdout.on('data', take('stdout'))
    child.stderr.on('data', take('stderr'))
    child.on('error', (error) => {
      clearTimeout(timer)
      out.ended = 'unstarted'
      settle({ kind: 'unstarted', message: error.message })
    })
    child.on('close', (code) => {
      clearTimeout(timer)
      if (out.ended === 'unstarted') return
      if (out.ended !== null) return settle({ kind: out.ended })
      const text = (stream) => Buffer.concat(out[stream]).toString('utf8')
      settle({ kind: 'exited', code: code ?? -1, stdout: text('stdout'), stderr: text('stderr') })
    })
  })
}

/**
 * Start `brokkr <argv>` detached, its output written to `<stateDir>/<name>.log`,
 * and watch it for a short grace: a verb that drives a run keeps running
 * after the console stops watching, and closing the panel leaves it be.
 * @returns {Promise<{kind: 'running'} | {kind: 'exited', code: number, output: string}
 *   | {kind: 'unstarted', message: string}>}
 */
export function launch(config, argv, name) {
  const refusal = privateDir(config.stateDir)
  if (refusal !== null) return Promise.resolve({ kind: 'unstarted', message: refusal })
  const path = join(config.stateDir, `${name}.log`)
  // The directory is the operator's own and closed to others, and the log is
  // never written through a symlink, so a planted link cannot be truncated.
  let fd
  try {
    fd = openSync(path, constants.O_WRONLY | constants.O_CREAT | constants.O_TRUNC | constants.O_NOFOLLOW, 0o600)
  } catch (error) {
    return Promise.resolve({ kind: 'unstarted', message: `the console could not open its log ${path}: ${error.message}` })
  }
  const [command, ...prefix] = config.program
  return new Promise((settle) => {
    const child = spawn(command, [...prefix, ...argv], {
      cwd: config.cwd,
      env: config.env,
      detached: true,
      stdio: ['ignore', fd, fd],
    })
    closeSync(fd)
    const timer = setTimeout(() => {
      child.unref()
      settle({ kind: 'running' })
    }, config.graceMs ?? RESUME_GRACE_MS)
    child.on('error', (error) => {
      clearTimeout(timer)
      settle({ kind: 'unstarted', message: error.message })
    })
    child.on('exit', (code) => {
      clearTimeout(timer)
      settle({ kind: 'exited', code: code ?? -1, output: tail(path) })
    })
  })
}

/**
 * Make `dir` if it is missing, then refuse it unless it is a real directory
 * (not a symlink) owned by this user and closed to everyone else: the state
 * directory sits under a shared temporary root by default, where another
 * user could have made it first. `null` when it is fit to write into.
 */
export function privateDir(dir) {
  mkdirSync(dir, { recursive: true, mode: 0o700 })
  const found = lstatSync(dir)
  if (!found.isDirectory()) return `the console's state directory ${dir} is not a directory`
  if (found.uid !== process.getuid()) return `the console's state directory ${dir} belongs to another user`
  if ((found.mode & 0o077) !== 0) return `the console's state directory ${dir} is open to other users`
  return null
}

/** The last `LOG_TAIL_BYTES` of the file at `path`, as text. */
function tail(path) {
  const size = statSync(path).size
  const bytes = readFileSync(path)
  return bytes.subarray(Math.max(0, size - LOG_TAIL_BYTES)).toString('utf8')
}
