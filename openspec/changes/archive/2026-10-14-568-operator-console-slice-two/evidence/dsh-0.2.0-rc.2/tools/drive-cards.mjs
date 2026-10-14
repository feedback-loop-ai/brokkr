// Continuation driver: reopen the sent Session, unfold its turn, press the
// suggestion card's button, use the conversation-header "open spec" action,
// and ask the host half for an inline answer.
// Usage: node drive-cards.mjs <authenticated-url> <ff-profile-dir> <shot-dir>
import { createRequire } from 'node:module'
const require = createRequire(process.argv[1].replace(/tools\/drive-cards\.mjs$/, 'browser/package.json'))
const puppeteer = require('puppeteer-core')
const [url, profileDir, shots] = process.argv.slice(2)
const log = (...a) => console.log(new Date().toISOString(), ...a)
const sleep = (ms) => new Promise((r) => setTimeout(r, ms))
let step = 0
const shot = async (page, name) => { const p = `${shots}/shot-${String(++step).padStart(2, '0')}-${name}.png`; await page.screenshot({ path: p }); log('screenshot', p.split('/').pop()) }
const clickText = (page, re) => page.evaluate((src) => {
  const re = new RegExp(src)
  const el = [...document.querySelectorAll('button,[role="button"],summary')].find((e) => re.test(e.textContent.trim()))
  el?.click(); return el ? el.tagName + ':' + el.textContent.trim().slice(0, 60) : null
}, re.source)
const browser = await puppeteer.launch({ browser: 'firefox', executablePath: '/snap/bin/firefox', headless: true, userDataDir: profileDir, args: ['--width=1500', '--height=950'] })
try {
  const page = await browser.newPage()
  await page.setViewport({ width: 1500, height: 950 })
  page.on('pageerror', (e) => log('pageerror:', e.message))
  await page.goto(url, { waitUntil: 'networkidle0', timeout: 60000 })
  await sleep(1500)
  log('dismiss modal:', await page.evaluate(() => { const d = document.querySelector('[role="dialog"],[role="alertdialog"]'); const b = d && [...d.querySelectorAll('button')].pop(); b?.click(); return b?.textContent ?? null }))
  await sleep(800)
  // Open the most recent sent Session from the left sidebar.
  const row = await page.waitForFunction(() => [...document.querySelectorAll('span,div,a,button')].find((e) => e.children.length === 0 && /^Stub answer: Fix the login/.test(e.textContent.trim())), { timeout: 20000 })
  log('open session row:', await page.evaluate((e) => e.tagName + ':' + e.textContent.trim(), row))
  await row.asElement().click()
  await page.waitForSelector('[data-testid="brokkr-suggest-card"]', { timeout: 20000 })
  await sleep(1000)
  log('unfold turn:', await clickText(page, /^Completed in /))
  await sleep(800)
  log('unfold tools:', await clickText(page, /^Called tools/))
  await sleep(1200)
  const vis = await page.evaluate(() => [...document.querySelectorAll('[data-testid="brokkr-runs-card"],[data-testid="brokkr-suggest-card"]')].map((e) => { const r = e.getBoundingClientRect(); return `${e.dataset.testid}[${e.dataset.phase}] ${Math.round(r.width)}x${Math.round(r.height)}: ${e.textContent}` }))
  for (const v of vis) log('DOM card', v)
  await shot(page, 'cards-unfolded')
  const press = await page.$('[data-testid="brokkr-suggest-press"]')
  await press.click()
  await page.waitForSelector('[data-testid="brokkr-suggest-pressed"]', { timeout: 10000 })
  log('DOM', await page.evaluate(() => document.querySelector('[data-testid="brokkr-suggest-pressed"]').textContent))
  await shot(page, 'card-pressed')
  const hb = await page.$('[data-testid="brokkr-slice2-header-open-spec"]')
  log('header action present:', !!hb)
  await hb.click()
  await sleep(2500)
  log('spec preview visible:', await page.evaluate(() => /Demo change: fix the login fence/.test(document.body.innerText)))
  await shot(page, 'header-open-spec')
  // Inline answer in the probe's own panel.
  const icon = await page.$('[data-testid="brokkr-slice2-icon"]')
  await (await page.evaluateHandle((el) => el.closest('button,[role="button"],a') ?? el, icon)).asElement().click()
  await page.waitForSelector('[data-testid="btn-inline"]', { timeout: 10000 })
  await (await page.$('[data-testid="btn-inline"]')).click()
  await page.waitForFunction(() => /Inline answer: /.test(document.querySelector('[data-testid="brokkr-slice2-out"]')?.textContent ?? ''), { timeout: 30000 })
  log('DOM inline:', await page.evaluate(() => document.querySelector('[data-testid="brokkr-slice2-out"]').textContent))
  await shot(page, 'inline-answer')
} catch (e) {
  log('DRIVER ERROR', e?.message ?? e)
  try { const ps = await browser.pages(); await ps[ps.length - 1].screenshot({ path: `${shots}/shot-error.png` }) } catch {}
  process.exitCode = 1
} finally { await browser.close() }
