// A stand-in for the `brokkr` binary. It answers from `script.json` in
// $BROKKR_STUB_DIR and appends each argv it was called with to `calls.ndjson`
// there. A rule answers the first argv holding every word of its `match`;
// its `responses` are used in order, the last one repeating.
import { appendFileSync, existsSync, readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'

const dir = process.env.BROKKR_STUB_DIR
const argv = process.argv.slice(2)
appendFileSync(join(dir, 'calls.ndjson'), JSON.stringify(argv) + '\n')

const script = JSON.parse(readFileSync(join(dir, 'script.json'), 'utf8'))
const index = script.rules.findIndex((rule) => rule.match.every((word) => argv.includes(word)))
if (index === -1) {
  process.stderr.write(`stub: no rule for ${argv.join(' ')}\n`)
  process.exit(97)
}
const countsPath = join(dir, 'counts.json')
const counts = existsSync(countsPath) ? JSON.parse(readFileSync(countsPath, 'utf8')) : {}
const used = counts[index] ?? 0
counts[index] = used + 1
writeFileSync(countsPath, JSON.stringify(counts))

const responses = script.rules[index].responses
const response = responses[Math.min(used, responses.length - 1)]
if (response.bytes) process.stdout.write('x'.repeat(response.bytes))
if (response.stdout !== undefined) process.stdout.write(response.stdout)
if (response.stderr !== undefined) process.stderr.write(response.stderr)
setTimeout(() => process.exit(response.code ?? 0), response.sleep_ms ?? 0)
