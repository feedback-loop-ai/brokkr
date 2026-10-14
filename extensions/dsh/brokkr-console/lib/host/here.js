// Which realm a DSH conversation's folder belongs to, so the conversation's
// Brokkr tab opens on that realm (design D1 of the view-tab change). Brokkr's
// own `realms` readout names each realm's folder. A folder inside one belongs
// to it, and so does a git worktree of one: its `.git` file points into the
// realm's `.git/worktrees/`. The link is read from those files, never by
// running git in a folder the browser named, whose config could run commands.
import { readFile, realpath, stat } from 'node:fs/promises'
import { dirname, isAbsolute, join, relative, resolve } from 'node:path'

import { RequestRefused } from './commands.js'
import { ONE_HEARTH, ReadoutRefused } from './readouts.js'

const DIR_BOUND = 4096

const inside = (dir, root) => {
  const rel = relative(root, dir)
  return rel === '' || (!rel.startsWith('..') && !isAbsolute(rel))
}

async function real(path) {
  try {
    return await realpath(path)
  } catch {
    return null
  }
}

/**
 * The main checkout a git worktree at or above `dir` belongs to, or `null`.
 * A worktree's `.git` is a file, `gitdir: <main>/.git/worktrees/<name>`, and
 * that folder's `commondir` names `<main>/.git`.
 */
async function mainCheckoutOf(dir) {
  for (let at = dir; ; at = dirname(at)) {
    const dotGit = join(at, '.git')
    const found = await stat(dotGit).catch(() => null)
    if (found !== null) {
      if (!found.isFile()) return null
      const pointer = /^gitdir:\s*(.+)$/m.exec(await readFile(dotGit, 'utf8').catch(() => ''))
      if (pointer === null) return null
      const gitdir = resolve(at, pointer[1].trim())
      const common = (await readFile(join(gitdir, 'commondir'), 'utf8').catch(() => null))?.trim()
      return common === undefined ? null : real(dirname(resolve(gitdir, common)))
    }
    if (dirname(at) === at) return null
  }
}

/**
 * `realmAt(dir)`: `{realm}`, the realm `dir` belongs to as the map names
 * it, or `null` when it belongs to none. In a world with no realms map, the
 * workspace's folder is the one hearth.
 * @param {{brokkr: object, workspace: string}} parts
 */
export function hereOf({ brokkr, workspace }) {
  return async function realmAt(dir) {
    if (typeof dir !== 'string' || dir.length > DIR_BOUND || !isAbsolute(dir)) throw new RequestRefused('the folder is an absolute path')
    const at = await real(dir)
    if (at === null) return { realm: null }
    let realms
    try {
      realms = await brokkr.realms()
    } catch (error) {
      if (!(error instanceof ReadoutRefused) || error.kind !== 'failed') throw error
      realms = [{ name: ONE_HEARTH, path: '.' }]
    }
    const roots = (await Promise.all(realms.map(async (realm) => ({ name: realm.name, root: await real(resolve(workspace, realm.path)) }))))
      .filter((realm) => realm.root !== null)
      .sort((a, b) => b.root.length - a.root.length)
    const owner = (folder) => roots.find((realm) => inside(folder, realm.root)) ?? null
    const found = owner(at) ?? (await mainCheckoutOf(at).then((main) => (main === null ? null : owner(main))))
    return { realm: found?.name ?? null }
  }
}
