# 0030 — A scaffold grants only the stack it can name

Status: proposed
Date: 2026-09-02

## Context

`brokkr init` already detects a repository's stack and writes that
stack's build, test and lint commands into the implementer and verifier
charters. It did not carry the conclusion through to execution. The
scaffolded Claude adapter declared an empty `tool_permissions.names`
map, the bundle inlined its seats, and no scaffold-local agent library
declared `tools.allow`. The prose named commands the seat could not be
granted through the provider's least-privilege flag.

Copying this repository's shipped adapter and agents into every new
scaffold is not an answer. Those files carry this repository's tool
vocabulary and operator rulings; a Bun project should not inherit Cargo
or another agent driver merely because Brokkr itself uses them.

There is also no honest generic default. When no marker matches, init
does not know the executable behind the placeholder commands. A guessed
permission name would turn the existing explicit `NO STACK WAS
RECOGNIZED` charter into a hidden claim that Brokkr had recognized one.

## Rulings

1. **Detection owns the executable name.** Every stack arm names the
   executable at the front of its build, test and lint commands. The
   monorepo runner table does the same for `bunx`, `pnpm exec`, `yarn
   exec`, and the `npx` fallback. Permission scaffolding consumes these
   tables and maintains no second stack vocabulary. Enforced by the
   `Stack::tool` / `RUNNERS` data and the all-recognized-stacks grant
   test in `init_stacks.rs`.

2. **Init writes a local agent library.** The starter bundle references
   five definitions under its own `agents/`, and their charters live
   under `agents/charters/`. Compilation resolves the starter against
   those definitions and the starter's own `adapters/`; it never borrows
   the Brokkr repository's shipped library. Enforced by `init` passing
   those two roots to `Bundle::compile_with`, with explicit Bun and Cargo
   resolution tests.

3. **Work and gate grants differ.** Intake and implement receive the
   detected executable plus `git`, `ls`, `rg`, and `mkdir`. Verify,
   review and ship receive `git`, `ls`, `rg`, and the executable needed
   to run the detected stack's proof commands; they do not receive
   `mkdir`. Every adapter value is the Claude spelling
   `Bash(<executable>:*)`, and each agent's ordered `tools.allow` is the
   source of its ordered `--allowedTools` argument. Enforced by
   `grants`, `adapter`, and `agent`, with exact per-class assertions for
   every production detection arm.

4. **The granularity is executable, not subcommand.** Claude's declared
   capability form in this adapter is `Bash(<executable>:*)`. When one
   executable serves both build and test — `bun`, `cargo`, and every
   current stack arm do — a gate's test grant necessarily names the same
   executable as the build command. The scaffold does not pretend this
   is subcommand confinement; it grants no separate install/build
   executable and relies on the gate charter's read-only command set.
   This is judgment-guidance at the provider capability boundary; the
   deterministic part is the exact executable list asserted in tests.

5. **Unknown means empty and disclosed.** With no detected stack, the
   adapter's names map is literally empty. An empty `tools.allow` is
   invalid by decision 0016's loader because it is ambiguous, so the
   unknown-stack definitions omit `allow` and the generated README says
   plainly that this means no tool narrowing was scaffolded. It tells
   the operator to correct charter commands, adapter mappings, and agent
   grants together before the first run. Enforced by the generic fixture
   asserting the empty map, absent `allow`, and disclosure together.

6. **Scaffold data is not clobbered.** An existing adapter, agent
   definition, agent charter, or scaffold README is operator-owned text.
   Init refuses before writing rather than overwriting any of them.
   Enforced by init's pre-write occupied-path check and overwrite tests.

## Consequences

A recognized fresh scaffold can run the commands its own charters name
without inheriting unrelated executables from Brokkr's development
workspace. The compiled manifest pins the local agent and adapter
digests, so changing a grant changes bundle identity as it should.

The cost is more visible starter data: five small definitions and five
charters instead of five inline roles. That is deliberate. Permissions
which only exist inside a generator cannot be reviewed or narrowed by
the operator, while ordinary JSON beside the bundle can.

An unrecognized scaffold remains compilable but is intentionally not a
least-privilege claim. Its README makes that gap an action item instead
of hiding it behind invented names.
