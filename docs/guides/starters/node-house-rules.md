Read `package.json` before changing code. Its scripts, module system, test
runner, package manager and TypeScript strictness are this repository's
conventions. Do not loosen them to make a change pass.

Install from the committed lockfile. For npm, run `npm ci`; then run the
repository's type check, tests, build and lint scripts that apply. Do not
commit `node_modules/`, coverage output, or generated build output unless the
repository already tracks it. Review every snapshot update rather than
accepting it mechanically.

Prefer the standard library and existing dependencies. When a new dependency
is necessary, commit its manifest and lockfile changes together and explain
the addition. Commit completed work in the repository's style and never push,
publish, tag or merge from a seat.
