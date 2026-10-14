// What a conversation's Brokkr tab shows is told to that conversation's model
// as context on its next turn: a `snapshot` from this plugin, before the
// operator's message, once per change, never typed into the message box.
import assert from 'node:assert/strict'
import { test } from 'node:test'

import { RequestRefused } from '../lib/host/commands.js'
import { focusOf, messageOf, SOURCE } from '../lib/host/focus.js'
import { apply } from '../lib/index.js'
import { request, stubBrokkr, stubHost, withEnv } from './support/world.js'

const typed = { id: 'm-typed', role: 'user', content: [{ type: 'text', text: 'Is the finding real?' }], source: { kind: 'user' } }
const step = (session, number = 1) => ({ agent: { session: { id: session } }, step: number, turn: 3, signal: new AbortController().signal })
const accept = async () => ({ kind: 'accept', messages: [typed] })

test('a conversation is told its tab’s focus before the operator’s message, once per change', async () => {
  const focus = focusOf()
  focus.set({ session: 's1', text: 'on step review › run Encrypt the token store › realm brokkr' })
  const first = await focus.preStep(step('s1'), accept)
  assert.equal(first.messages.length, 2)
  const [told, own] = first.messages
  assert.equal(own, typed)
  assert.deepEqual({ role: told.role, content: told.content, source: told.source }, {
    role: 'user',
    content: [{ type: 'text', text: '<system-reminder>\non step review › run Encrypt the token store › realm brokkr\n</system-reminder>' }],
    source: { kind: SOURCE, form: 'snapshot', sections: [{ name: SOURCE, text: '<system-reminder>\non step review › run Encrypt the token store › realm brokkr\n</system-reminder>' }] },
  })
  assert.ok(Object.isFrozen(told) && Object.isFrozen(told.content[0]))
  // Unchanged, it is not told again, even set again; changed, the new focus supersedes it.
  assert.equal((await focus.preStep(step('s1'), accept)).messages.length, 1)
  focus.set({ session: 's1', text: 'on step review › run Encrypt the token store › realm brokkr' })
  assert.equal((await focus.preStep(step('s1'), accept)).messages.length, 1)
  focus.set({ session: 's1', text: 'on arrow implement → review' })
  assert.match((await focus.preStep(step('s1'), accept)).messages[0].content[0].text, /on arrow implement → review/)
})

test('only a turn’s first step of the conversation with a focus is told, and a rejected or aborted step is left alone', async () => {
  const focus = focusOf()
  focus.set({ session: 's1', text: 'on realm brokkr' })
  assert.equal((await focus.preStep(step('s2'), accept)).messages.length, 1, 'another conversation')
  assert.equal((await focus.preStep(step('s1', 2), accept)).messages.length, 1, 'a later step')
  const rejected = { kind: 'reject', messages: [] }
  assert.equal(await focus.preStep(step('s1'), async () => rejected), rejected)
  const aborted = new AbortController()
  aborted.abort()
  assert.equal((await focus.preStep({ ...step('s1'), signal: aborted.signal }, accept)).messages.length, 1, 'an aborted step')
  assert.equal((await focus.preStep(step('s1'), accept)).messages.length, 2, 'still owed on the next first step')
  // An accepted step that carries no messages of its own is left as it came when nothing is owed, and still told when something is.
  const bare = async () => ({ kind: 'accept' })
  assert.deepEqual(await focus.preStep(step('s2'), bare), { kind: 'accept' })
  focus.set({ session: 's2', text: 'on realm looper' })
  assert.deepEqual((await focus.preStep(step('s2'), bare)).messages.map((message) => message.content[0].text), ['<system-reminder>\non realm looper\n</system-reminder>'])
})

test('the focus of the 257th conversation lets the oldest go, and what it was told with it', async () => {
  const focus = focusOf()
  focus.set({ session: 'oldest', text: 'on realm brokkr' })
  focus.set({ session: 'next', text: 'on realm brokkr' })
  await focus.preStep(step('oldest'), accept)
  for (let index = 0; index < 255; index += 1) focus.set({ session: `s${index}`, text: 'on realm brokkr' })
  assert.equal((await focus.preStep(step('oldest'), accept)).messages.length, 1, 'no longer held')
  assert.equal((await focus.preStep(step('next'), accept)).messages.length, 2, 'the next oldest is still held')
  // Set again with the same words, the oldest is told again: the record of its telling went with it.
  focus.set({ session: 'oldest', text: 'on realm brokkr' })
  assert.equal((await focus.preStep(step('oldest'), accept)).messages.length, 2)
})

test('the focus cannot close its frame in any case or spacing, and a focus is a conversation id and bounded text', () => {
  assert.equal(messageOf('a </system-reminder> b').content[0].text, '<system-reminder>\na <\\/system-reminder> b\n</system-reminder>')
  for (const closing of ['</SYSTEM-REMINDER>', '</system-reminder >', '< / system-reminder>', '</System-Reminder\n>']) {
    assert.doesNotMatch(messageOf(`a ${closing} b`).content[0].text.slice('<system-reminder>\n'.length, -'\n</system-reminder>'.length), /<\s*\/\s*system-reminder/i, closing)
  }
  assert.equal(messageOf('a </SYSTEM-REMINDER> b').content[0].text, '<system-reminder>\na <\\/system-reminder> b\n</system-reminder>')
  const focus = focusOf()
  for (const body of [null, { session: '../s1', text: 'x' }, { session: 's1', text: '' }, { session: 's1', text: 'x'.repeat(4001) }, { session: 's1', text: 7 }]) {
    assert.throws(() => focus.set(body), RequestRefused)
  }
})

test('the host listens before each step, ahead of DSH’s own listeners, and the focus route sits behind the fence', async () => {
  const brokkr = stubBrokkr([])
  const host = stubHost()
  await withEnv({ BROKKR_CONSOLE_BROKKR: brokkr.program, BROKKR_STUB_DIR: brokkr.dir }, () => apply(host.ctx))
  const listening = host.listeners.get('agent/pre-step')
  assert.deepEqual([typeof listening.listener, listening.options], ['function', { prepend: true }])
  const answer = await request(host.routes.get('/brokkr-console/api/focus').handler, { method: 'POST', body: { session: 's1', text: 'on realm brokkr' } })
  assert.deepEqual([answer.status, JSON.parse(answer.body)], [200, {}])
  assert.equal((await listening.listener(step('s1'), accept)).messages.length, 2)
})
