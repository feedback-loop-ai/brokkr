// Drive headless Firefox (WebDriver BiDi via puppeteer-core) through the
// slice-two probe. Usage: node drive.mjs <authenticated-url> <ff-profile-dir> <shot-dir> <worktreeA> <worktreeB>
import { createRequire } from 'node:module'
const require = createRequire(process.argv[1].replace(/tools\/drive\.mjs$/, 'browser/package.json'))
const puppeteer = require('puppeteer-core')

const [url, profileDir, shots, worktreeA, worktreeB] = process.argv.slice(2)
const t0 = Date.now()
const log = (...a) => console.log(new Date().toISOString(), ...a)
const sleep = (ms) => new Promise((r) => setTimeout(r, ms))

const browser = await puppeteer.launch({
  browser: 'firefox',
  executablePath: '/snap/bin/firefox',
  headless: true,
  userDataDir: profileDir,
  args: ['--width=1500', '--height=950'],
})
let step = 0
async function shot(page, name) {
  const path = `${shots}/shot-${String(++step).padStart(2, '0')}-${name}.png`
  await page.screenshot({ path })
  log('screenshot', path.split('/').pop())
}
async function dismissModal(page) {
  const t = await page.evaluate(() => {
    const dlg = document.querySelector('[role="dialog"],[role="alertdialog"]')
    if (!dlg) return null
    const b = [...dlg.querySelectorAll('button')].pop()
    const text = b?.textContent; b?.click(); return text ?? '(no button)'
  })
  if (t !== null) log('dismissed modal via button:', JSON.stringify(t))
}
async function openPanel(page) {
  const icon = await page.waitForSelector('[data-testid="brokkr-slice2-icon"]', { timeout: 30000 })
  const btn = await page.evaluateHandle((el) => el.closest('button,[role="button"],a') ?? el, icon)
  await btn.asElement().click()
  await page.waitForSelector('[data-testid="brokkr-slice2-page"]', { timeout: 15000 })
  log('probe main panel mounted')
}
const call = (page, fn, arg) => page.evaluate(async (fn, arg) => window.__BROKKR_SLICE2__[fn](arg), fn, arg)
async function composerText(page) {
  return page.evaluate(() => [...document.querySelectorAll('[contenteditable],[role="textbox"],textarea')].map((e) => `${e.tagName.toLowerCase()}[ce=${e.getAttribute('contenteditable')}]: ${(e.textContent || e.value || '').slice(0, 160)}`).join(' | '))
}
async function sidebarSummary(page) {
  return page.evaluate(() => {
    const text = document.body.innerText
    const pick = (re) => (text.match(re) || [null])[0]
    return {
      specTitle: pick(/Demo change: fix the login fence/),
      otherTitle: pick(/This file lives in worktree B, outside the Session cwd\./),
      readmeB: pick(/# Worktree B|Worktree B/),
      failure: pick(/(outside-workspace|not-found|unknown-workspace|not-regular-file)[^\n]*/),
    }
  })
}

try {
  const page = await browser.newPage()
  await page.setViewport({ width: 1500, height: 950 })
  page.on('console', (m) => { const t = m.text(); if (/brokkr|slot entry crashed|error/i.test(t)) log('console:', m.type(), t.slice(0, 400)) })
  page.on('pageerror', (e) => log('pageerror:', e.message))
  log('goto', url.replace(/token=[^&]+/, 'token=<redacted>'))
  await page.goto(url, { waitUntil: 'networkidle0', timeout: 60000 })
  await sleep(1500)
  await dismissModal(page)
  await sleep(500)
  await page.evaluate((a, b) => { window.__BROKKR_SLICE2__.worktreeA = a; window.__BROKKR_SLICE2__.worktreeB = b }, worktreeA, worktreeB)

  // ---- D7 negative: open a file while the probe's global panel replaces the conversation
  await openPanel(page)
  await shot(page, 'probe-panel')
  log('openSpec with no Session on screen ->', JSON.stringify(await call(page, 'openSpec', { path: 'openspec/changes/demo/spec.md' })))

  // ---- D6: Ask about this run -> new Session (cwd = worktree A), shown, prefilled
  await (await page.$('[data-testid="btn-ask-prefill"]')).click()
  await page.waitForFunction(() => /Ask about this run \(prefill\): /.test(document.querySelector('[data-testid="brokkr-slice2-out"]')?.textContent ?? '') || !document.querySelector('[data-testid="brokkr-slice2-page"]'), { timeout: 30000 })
  await sleep(4000)
  log('after prefill: probe panel still mounted =', !!(await page.$('[data-testid="brokkr-slice2-page"]')))
  log('composer text after prefill:', JSON.stringify(await composerText(page)))
  log('header action present:', !!(await page.$('[data-testid="brokkr-slice2-header-open-spec"]')))
  await shot(page, 'session-prefilled')

  // ---- D7: open the spec from the conversation-header action (relative path in the Session cwd)
  const headerBtn = await page.$('[data-testid="brokkr-slice2-header-open-spec"]')
  if (headerBtn) { await headerBtn.click(); await sleep(2500) }
  log('sidebar after header open-spec:', JSON.stringify(await sidebarSummary(page)))
  await shot(page, 'open-spec-relative')

  // ---- D7: absolute path OUTSIDE the Session cwd (worktree B)
  log('openSpec abs outside cwd ->', JSON.stringify(await call(page, 'openSpec', { path: `${worktreeB}/specs/other.md` })))
  await sleep(2500)
  log('sidebar after abs open-spec:', JSON.stringify(await sidebarSummary(page)))
  await shot(page, 'open-spec-outside-cwd')

  // ---- D7: Files tab (rooted at the Session cwd)
  log('openFiles ->', JSON.stringify(await call(page, 'openFiles')))
  await sleep(2500)
  const tree = await page.evaluate(() => document.body.innerText.match(/worktree-a[^\n]*|openspec|README\.md/g))
  log('Files tab text hits:', JSON.stringify(tree))
  await shot(page, 'files-tab-worktree-a')

  // ---- D7: a Session created with cwd = worktree B roots its Files tab there
  log('askAboutRun via=session cwd=worktreeB ->', JSON.stringify(await call(page, 'askAboutRun', { cwd: worktreeB, text: 'Worktree B session', sendIt: false, via: 'session' })))
  await sleep(3000)
  log('composer text (via=session, no Workspace):', JSON.stringify(await composerText(page)))
  await shot(page, 'session-without-workspace')
  log('openFiles (session B) ->', JSON.stringify(await call(page, 'openFiles')))
  await sleep(2500)
  const treeB = await page.evaluate(() => document.body.innerText.match(/worktree-b[^\n]*|specs|README\.md/g))
  log('Files tab text hits (session B):', JSON.stringify(treeB))
  await shot(page, 'files-tab-worktree-b')

  // ---- D6: Ask and send -> the stub model calls brokkr_runs then brokkr_suggest
  await openPanel(page)
  await (await page.$('[data-testid="btn-ask-send"]')).click()
  log('waiting for the suggestion card result phase')
  await page.waitForSelector('[data-testid="brokkr-suggest-card"][data-phase="result"]', { timeout: 60000 })
  await sleep(2500)
  const cards = await page.evaluate(() => [...document.querySelectorAll('[data-testid="brokkr-runs-card"],[data-testid="brokkr-suggest-card"]')].map((e) => `${e.dataset.testid}[${e.dataset.phase}]: ${e.textContent}`))
  for (const c of cards) log('DOM card', c)
  const answer = await page.evaluate(() => (document.body.innerText.match(/Stub answer: the run[^\n]*/) || [null])[0])
  log('final assistant text in transcript:', JSON.stringify(answer))
  await shot(page, 'transcript-collapsed')
  // The turn's steps fold under "Completed in …"; open it so the cards show.
  const opened = await page.evaluate(() => {
    const el = [...document.querySelectorAll('button,[role="button"],summary,div,span')].find((e) => /^Completed in /.test(e.textContent.trim()) && e.children.length <= 3)
    const target = el?.closest('button,[role="button"],summary') ?? el
    target?.click(); return target ? target.tagName + ':' + target.textContent.trim().slice(0, 40) : null
  })
  log('expanded turn steps via', JSON.stringify(opened))
  await sleep(1500)
  log('suggestion card visible:', await page.evaluate(() => { const e = document.querySelector('[data-testid="brokkr-suggest-card"]'); return !!e && e.getClientRects().length > 0 }))
  await shot(page, 'transcript-cards')
  await (await page.$('[data-testid="brokkr-suggest-press"]')).click()
  await page.waitForSelector('[data-testid="brokkr-suggest-pressed"]', { timeout: 10000 })
  log('DOM', await page.evaluate(() => document.querySelector('[data-testid="brokkr-suggest-pressed"]').textContent))
  await shot(page, 'card-pressed')

  // ---- D7: the conversation-header action opens the spec (relative to the Session cwd)
  const hb = await page.$('[data-testid="brokkr-slice2-header-open-spec"]')
  log('header action present (non-blank Session):', !!hb)
  if (hb) { await hb.click(); await sleep(2500) }
  log('sidebar after header open-spec:', JSON.stringify(await sidebarSummary(page)))
  await shot(page, 'header-open-spec-relative')

  // ---- D6: inline answer from the host half, rendered in the panel
  await openPanel(page)
  await (await page.$('[data-testid="btn-inline"]')).click()
  await page.waitForFunction(() => /Inline answer: /.test(document.querySelector('[data-testid="brokkr-slice2-out"]')?.textContent ?? ''), { timeout: 30000 })
  log('DOM inline:', await page.evaluate(() => document.querySelector('[data-testid="brokkr-slice2-out"]').textContent))
  await shot(page, 'inline-answer')
} catch (e) {
  log('DRIVER ERROR', e?.message ?? e)
  try { const pages = await browser.pages(); await pages[pages.length - 1].screenshot({ path: `${shots}/shot-error.png` }) } catch {}
  process.exitCode = 1
} finally {
  await browser.close()
  log(`done in ${Date.now() - t0}ms`)
}
