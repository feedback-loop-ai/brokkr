// Drive headless Firefox (WebDriver BiDi via puppeteer-core) against the probe.
// Usage: node drive.mjs <authenticated-url> <profile-dir> <screenshot-prefix>
import puppeteer from 'puppeteer-core'
const [url, profileDir, shot] = process.argv.slice(2)
const log = (...a) => console.log(new Date().toISOString(), ...a)
const browser = await puppeteer.launch({
  browser: 'firefox',
  executablePath: '/snap/bin/firefox',
  headless: true,
  userDataDir: profileDir,
  args: ['--width=1400', '--height=900'],
})
try {
  const page = await browser.newPage()
  await page.setViewport({ width: 1400, height: 900 })
  page.on('console', (m) => { const t = m.text(); if (/brokkr|error/i.test(t)) log('console:', m.type(), t) })
  page.on('pageerror', (e) => log('pageerror:', e.message))
  log('goto', url.replace(/token=[^&]+/, 'token=<redacted>'))
  await page.goto(url, { waitUntil: 'networkidle0', timeout: 60000 })
  log('landed', page.url())
  const icon = await page.waitForSelector('[data-testid="brokkr-probe-icon"]', { timeout: 30000 })
  log('sidebar.panellist icon present:', !!icon)
  const btn = await page.evaluateHandle((el) => el.closest('button,[role="button"],a'), icon)
  const label = await page.evaluate((b) => b && (b.getAttribute('aria-label') ?? b.getAttribute('title') ?? b.textContent), btn)
  log('panel row element label/aria:', JSON.stringify(label))
  await page.screenshot({ path: `${shot}-1-sidebar.png` })
  // First launch shows a modal "Preview Notice"; it intercepts clicks. Dismiss it.
  await new Promise((r) => setTimeout(r, 1500))
  const dismissed = await page.evaluate(() => {
    const dlg = document.querySelector('[role="dialog"],[role="alertdialog"]')
    if (!dlg) return null
    const b = [...dlg.querySelectorAll('button')].pop()
    const t = b?.textContent; b?.click(); return t ?? '(no button)'
  })
  log('dismissed modal via button:', JSON.stringify(dismissed))
  await new Promise((r) => setTimeout(r, 800))
  await (btn.asElement() ?? icon).click()
  const pageEl = await page.waitForSelector('[data-testid="brokkr-probe-page"]', { timeout: 15000 })
  log('main panel mounted:', !!pageEl)
  await page.waitForFunction(() => document.querySelector('[data-testid="brokkr-probe-ws"]')?.textContent.includes('echo:'), { timeout: 15000 })
  const text = await page.evaluate(() => [...document.querySelectorAll('[data-testid^="brokkr-probe-"]')].map((e) => `${e.dataset.testid}${e.dataset.active ? `[active=${e.dataset.active}]` : ''}: ${e.textContent}`))
  for (const t of text) log('DOM', t)
  await page.screenshot({ path: `${shot}-2-main.png` })
} finally {
  await browser.close()
}
