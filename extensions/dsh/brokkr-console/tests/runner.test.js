// The resume log lives under a shared temporary root by default, so the
// console writes it only into a directory that is its own and closed, and
// never through a symlink another user planted.
import assert from 'node:assert/strict'
import { chmodSync, mkdirSync, mkdtempSync, readFileSync, symlinkSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { test } from 'node:test'

import { launch, privateDir } from '../lib/host/runner.js'

const scratch = () => mkdtempSync(join(tmpdir(), 'brokkr-console-runner-'))
const config = (stateDir) => ({ stateDir, program: [process.execPath, '-e', 'process.exit(0)'], cwd: process.cwd(), env: process.env, graceMs: 2000 })

test('a missing state directory is made closed to others and accepted', () => {
  const dir = join(scratch(), 'state')
  assert.equal(privateDir(dir), null)
})

test('a state directory open to others, or a symlink standing in for one, is refused', () => {
  const root = scratch()
  const open = join(root, 'open')
  mkdirSync(open, { mode: 0o755 })
  chmodSync(open, 0o755)
  assert.match(privateDir(open), /is open to other users/)
  const target = join(root, 'target')
  mkdirSync(target, { mode: 0o700 })
  const link = join(root, 'link')
  symlinkSync(target, link)
  assert.match(privateDir(link), /is not a directory/)
})

test('a log path planted as a symlink is not written through, and nothing is started', async () => {
  const root = scratch()
  const state = join(root, 'state')
  mkdirSync(state, { mode: 0o700 })
  const victim = join(root, 'victim')
  writeFileSync(victim, 'keep me')
  symlinkSync(victim, join(state, 'cmd-1.log'))
  const outcome = await launch(config(state), [], 'cmd-1')
  assert.equal(outcome.kind, 'unstarted')
  assert.match(outcome.message, /could not open its log/)
  assert.equal(readFileSync(victim, 'utf8'), 'keep me')
})

test('in a private directory the log is written and the run is started', async () => {
  const state = join(scratch(), 'state')
  const outcome = await launch(config(state), [], 'cmd-2')
  assert.ok(['exited', 'running'].includes(outcome.kind), JSON.stringify(outcome))
})

test('a state directory that cannot be made answers unstarted, and nothing is started', async () => {
  const root = scratch()
  const file = join(root, 'a-file')
  writeFileSync(file, 'not a directory')
  const state = join(file, 'state')
  assert.match(privateDir(state), /cannot be made/)
  const outcome = await launch(config(state), [], 'cmd-3')
  assert.equal(outcome.kind, 'unstarted')
  assert.match(outcome.message, /cannot be made/)
})
