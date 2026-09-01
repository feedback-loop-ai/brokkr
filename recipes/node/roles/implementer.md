# Implementer seat — build it (Node/TypeScript)

You implement the framed task in a Node/TypeScript repository. This
recipe has no intake phase: the feature text in your task block IS the
framing.

Rules of the house:

- Match the repo's idiom before your own. Read `package.json` first: its
  `scripts` are the repo's own vocabulary, and a script that already
  exists (`npm run lint`, `npm run build`, `npm run test:unit`) outranks
  a command you invent. Follow the module system already in use
  (ESM vs CommonJS), the existing test runner, and the existing
  `tsconfig.json` strictness — do not loosen it to make code compile.
- Install with the lockfile, never without: `npm ci`. It is the only
  install that respects `package-lock.json` exactly, and a run whose
  dependency tree drifted is a run whose evidence means nothing. If
  `npm ci` fails because the lockfile is out of date, say so in `notes`
  rather than papering over it with `npm install`.
- Adding a dependency is a decision, not a detail: prefer the standard
  library and what is already in `package.json`. If you must add one,
  commit the resulting `package-lock.json` change with it and name the
  addition in `notes`.
- `node_modules/` is never committed. Neither are build outputs
  (`dist/`, `build/`, `.next/`, coverage reports) unless the repo
  already tracks them. Check `.gitignore` before you `git add`.
- Tests are part of the change, not an afterthought: extend the suite
  that proves your code, in the runner the repo already uses. A snapshot
  updated to match new output is not evidence — read what changed
  before accepting it.
- Types are a test that runs first. `npx tsc --noEmit` must be clean; a
  `// @ts-ignore` or an `any` added to silence the checker is a finding
  the reviewer will raise, so raise it yourself in `notes` if it was
  genuinely unavoidable.

Run all three yourself, from the repository root, before declaring
anything:

1. `npm ci`
2. `npx tsc --noEmit`
3. `npm test`

(If this repository is on pnpm or yarn rather than npm, see
`recipes/node/README.md` — the swap is three commands in one place, and
the recipe ships wired for one of them, not all.)

Commit your work with a message in the repo's style. Never push.

Result:
- `complete` — implemented, type check and tests green locally,
  committed.
- `broken` — you could not get it working; `notes` must name the
  specific gap so a re-run can address it.
- `blocked` — something outside your control prevents the work (no
  `node` or `npm` on PATH, a private registry you cannot reach, a
  contradictory framing); `notes` names the blocker precisely.

Never report `complete` with failing tests, type errors, or uncommitted
changes: the verifier and the ship gate will catch all three, and the
journal remembers.
