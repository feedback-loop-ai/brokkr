// Third driver: client services through `inject`, and the workspaceFiles
// Remote's containment (directory listing vs file stat) for one Session.
// Usage: node drive-files.mjs <authenticated-url> <ff-profile-dir> <worktreeA> <worktreeB>
import { createRequire } from 'node:module'
const require = createRequire(process.argv[1].replace(/tools\/drive-files\.mjs$/, 'browser/package.json'))
const puppeteer = require('puppeteer-core')
const [url, profileDir, a, b] = process.argv.slice(2)
const log = (...x) => console.log(new Date().toISOString(), ...x)
const sleep = (ms) => new Promise((r) => setTimeout(r, ms))
const browser = await puppeteer.launch({ browser: 'firefox', executablePath: '/snap/bin/firefox', headless: true, userDataDir: profileDir, args: ['--width=1500', '--height=950'] })
try {
  const page = await browser.newPage()
  await page.goto(url, { waitUntil: 'networkidle0', timeout: 60000 })
  await sleep(1500)
  await page.evaluate(() => { const d = document.querySelector('[role="dialog"],[role="alertdialog"]'); [...(d?.querySelectorAll('button') ?? [])].pop()?.click() })
  await page.waitForFunction(() => typeof window.__BROKKR_SLICE2__?.probeFiles === 'function', { timeout: 20000 })
  log('panel row present:', !!(await page.$('[data-testid="brokkr-slice2-icon"]')))
  const ask = await page.evaluate((a) => window.__BROKKR_SLICE2__.askAboutRun({ cwd: a, text: 'files probe', sendIt: false, via: 'workspace' }), a)
  log('session for worktree A:', JSON.stringify(ask))
  const r = await page.evaluate(async (id, a, b) => {
    const P = window.__BROKKR_SLICE2__.probeFiles
    return [
      await P({ sessionId: id, dir: a + '/openspec', file: a + '/openspec/changes/demo/spec.md' }),
      await P({ sessionId: id, dir: b, file: b + '/specs/other.md' }),
      await P({ sessionId: id, dir: b + '/specs', file: '/etc/hostname' }),
    ]
  }, ask.id, a, b)
  for (const x of r) log('probeFiles', JSON.stringify(x))
} catch (e) { log('DRIVER ERROR', e?.message ?? e); process.exitCode = 1 } finally { await browser.close() }
