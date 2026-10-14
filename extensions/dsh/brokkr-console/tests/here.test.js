// A conversation's folder is placed in a realm by Brokkr's own `realms`
// readout: inside a realm's folder, or in a git worktree of one. Nothing
// else is guessed, and git is never run.
import assert from 'node:assert/strict'
import { mkdirSync, mkdtempSync, realpathSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { test } from 'node:test'

import { RequestRefused } from '../lib/host/commands.js'
import { apply } from '../lib/index.js'
import { hereOf } from '../lib/host/here.js'
import { ONE_HEARTH, ReadoutRefused, realmsOf } from '../lib/host/readouts.js'
import { json, request, stubBrokkr, stubHost, withEnv } from './support/world.js'

/** A workspace holding the brokkr checkout, a sibling swarmail, a worktree of brokkr, and a stray folder. */
function world() {
  const root = realpathSync(mkdtempSync(join(tmpdir(), 'brokkr-console-here-')))
  const brokkr = join(root, 'brokkr')
  for (const dir of [join(brokkr, '.git', 'worktrees', 'brokkr-tab'), join(brokkr, 'crates', 'view'), join(root, 'swarmail'), join(root, 'brokkr-tab', 'lib'), join(root, 'elsewhere')]) {
    mkdirSync(dir, { recursive: true })
  }
  writeFileSync(join(root, 'brokkr-tab', '.git'), `gitdir: ${join(brokkr, '.git', 'worktrees', 'brokkr-tab')}\n`)
  writeFileSync(join(brokkr, '.git', 'worktrees', 'brokkr-tab', 'commondir'), '../..\n')
  return { root, brokkr }
}

const mapped = (realms) => ({ realms: async () => realms })

test('a folder inside a realm, or the realm itself, is that realm', async () => {
  const { root, brokkr } = world()
  const here = hereOf({ brokkr: mapped([{ name: 'brokkr', path: '.' }, { name: 'swarmail', path: '../swarmail' }]), workspace: brokkr })
  assert.deepEqual(await here(brokkr), { realm: 'brokkr' })
  assert.deepEqual(await here(join(brokkr, 'crates', 'view')), { realm: 'brokkr' })
  assert.deepEqual(await here(join(root, 'swarmail')), { realm: 'swarmail' })
})

test('a git worktree of a realm is that realm', async () => {
  const { root, brokkr } = world()
  const here = hereOf({ brokkr: mapped([{ name: 'brokkr', path: '.' }]), workspace: brokkr })
  assert.deepEqual(await here(join(root, 'brokkr-tab', 'lib')), { realm: 'brokkr' })
})

test('a folder in no realm, or one that does not exist, is in none', async () => {
  const { root, brokkr } = world()
  const here = hereOf({ brokkr: mapped([{ name: 'brokkr', path: '.' }]), workspace: brokkr })
  assert.deepEqual(await here(join(root, 'elsewhere')), { realm: null })
  assert.deepEqual(await here(join(root, 'no-such-folder')), { realm: null })
})

test('with no realms map, the workspace folder is the one hearth', async () => {
  const { brokkr } = world()
  const refused = { realms: async () => { throw new ReadoutRefused('failed', 'no map: this workspace has no realms.json') } }
  const here = hereOf({ brokkr: refused, workspace: brokkr })
  assert.deepEqual(await here(join(brokkr, 'crates')), { realm: ONE_HEARTH })
})

test('a readout that times out is not mistaken for a world with no map', async () => {
  const { brokkr } = world()
  const slow = { realms: async () => { throw new ReadoutRefused('timeout', 'brokkr realms took longer than the console waits') } }
  await assert.rejects(hereOf({ brokkr: slow, workspace: brokkr })(brokkr), ReadoutRefused)
})

test('only an absolute folder is asked about', async () => {
  const here = hereOf({ brokkr: mapped([]), workspace: '/' })
  for (const dir of [null, 'brokkr', '../etc', `/${'x'.repeat(5000)}`]) await assert.rejects(here(dir), RequestRefused)
})

test('the realms readout reads each realm’s name and folder', () => {
  const doc = { journal: './.forge/forge.db', map: './realms.json', realms: [{ default_branch: 'main', head: '67a1886e', journal: './.forge/forge.db', name: 'brokkr', path: '.' }] }
  assert.deepEqual(realmsOf(doc), [{ name: 'brokkr', path: '.' }])
  assert.throws(() => realmsOf({ realms: [{ name: 'brokkr' }] }), ReadoutRefused)
})

test('the route answers the realm through the fence, and the stylesheet is revalidated on every load', async () => {
  const { root, brokkr: workspace } = world()
  const realms = { journal: './.forge/forge.db', map: './realms.json', realms: [{ name: 'brokkr', path: '.' }, { name: 'swarmail', path: '../swarmail' }] }
  const brokkr = stubBrokkr([{ match: ['realms'], responses: [json(realms)] }])
  const host = stubHost()
  await withEnv({ BROKKR_CONSOLE_BROKKR: brokkr.program, BROKKR_STUB_DIR: brokkr.dir, BROKKR_CONSOLE_WORKSPACE: workspace }, () => apply(host.ctx))
  const asked = await request(host.routes.get('/brokkr-console/api/here').handler, { url: `/brokkr-console/api/here?${new URLSearchParams({ dir: join(root, 'swarmail') })}` })
  assert.deepEqual([asked.status, JSON.parse(asked.body)], [200, { realm: 'swarmail' }])
  const relative = await request(host.routes.get('/brokkr-console/api/here').handler, { url: '/brokkr-console/api/here?dir=swarmail' })
  assert.deepEqual([relative.status, JSON.parse(relative.body)], [400, { error: 'the folder is an absolute path' }])
  const sheet = await request(host.routes.get('/brokkr-console/console.css').handler, { url: '/brokkr-console/console.css' })
  assert.equal(sheet.headers['cache-control'], 'no-cache')
})
