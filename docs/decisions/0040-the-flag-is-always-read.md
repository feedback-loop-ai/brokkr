# 0040 — `--model` is always read: the flag grammar of the pin, and the two riders beside it

Status: proposed (ruling 1 is the operator's, given in chat on 2026-09-03; rulings 2–5 are this draft's reading of the same axis and await the operator)
Date: 2026-09-03

## Context

Three decisions read the same argv for three different facts. Decision
0016 made the flag a provider is told its model on adapter data:
`model_flag`, one string per adapter, or `"unsupported"`. Decision 0031
ruling 2 asks whether an inline seat on one of the four model-bearing
built-ins STATES a concrete model, and reads `--model`, the flag this
engine composes for them. Decision 0036 ruling 2 asks WHERE the material
goes, and since #161 reads the flag the adapter declares — because a
provider the operator adds that takes `-m` is told its route on `-m`, and
reading a hardcoded `--model` there found nothing, called the site
unpinned, and handed it the adapter's own class: the fail-open 0036's
first rejected alternative describes, through a second door.

#161 closed that door and the reviewer proved it red/green. What
survived, parked lawfully at `REVIEW-REFORGE-EXHAUSTED-UNFIXED` and
shipped on the operator's ruling with the residuals queued, is one
question the seats twice declined to answer because it is grammar, and
grammar is a ruling: which spellings does a declared flag cover, and is
`--model` one of them on every adapter?

Five findings rest on it. From #161's reviewer:

1. On an adapter declaring `model_flag: "-m"` whose real CLI also
   honours `--model` — as model CLIs commonly do — a secret-binding seat
   writing `--model elsewhere/large-1` compiles on the adapter's own
   class while the material goes to the unruled route. Fail-open. Zero
   exposure today: all five shipped adapters declare `--model`, so the
   declared flag and the real flag coincide.
2. The strict unread-spelling reading (#161's third commit: a word
   carrying the flag in a form the walker does not read is `Unreadable`,
   not `Absent`) is keyed to the CALLER, not to the flag's shape, so the
   route reader applies it to long flags too. On a `--model` adapter,
   `--model nearby/small-1 --model-fallback partner/large-1` is refused as
   "not one readable concrete model id" although it readably pins a
   declared route. Fail-closed, and the wrong problem named. No shipped
   argv trips it.
3. `adapter.model_flag.as_deref().unwrap_or("--model")` in the
   `Unreadable` refusal is unreachable by construction and says so in
   its own comment.

And from #160's reviewer, two riders on the same axis that no flag
ruling touches but the same small slice can carry:

4. `brokkr doctor`'s ambient-credential report (0036 ruling 5) tests
   whether the variable is in the bindings STORE, not whether any seat
   BINDS it. A name sitting in the store that no seat declares is never
   bound to the driver, so if the launching shell exports it the driver
   still takes it ambiently and doctor says nothing — a false negative
   on exactly the channel ruling 5 exists to make visible.
5. `routes` and `credentials` keys are checked against the agent-name
   grammar `^[a-z][a-z0-9-]*$`, but `resolve_route` splits a concrete
   model id on its first `/`, and the id alphabet admits `.`, `_`, `:`
   and upper case in that prefix. A route like `us.east` or
   `openai_compat` therefore resolves uncontracted forever, with no
   operator data able to say otherwise. Fail-closed, but ruling 1 of
   0036 says class assignment is operator data, and here it is
   unstatable.

The alternative to a ruling is what #161 did: keep the two readers
apart (`UnreadSpelling` keeps 0031's question from answering 0036's) and
refuse to guess. That is the honest compiler answer and it is not a
grammar. It leaves finding 1 open on every adapter the operator adds,
which is the population 0036 was written for.

## Rulings

1. **`--model` is always read.** The operator's ruling, 2026-09-03. On
   every adapter, the route reader reads two flags: the flag the adapter
   declares in `model_flag`, and `--model`. Where the two are the same
   string that is one read, as today. Where they differ, a concrete pin
   on either names the route; two pins that name different ids are
   `Unreadable`, refused naming both, because the material can only go
   one place and the argv says two. An adapter declaring `"unsupported"`
   still has `--model` read: a provider that cannot be told a model has
   no route to name, so a pin found there is a site telling a binary
   something the adapter says it cannot hear — `Unreadable`, not the
   adapter's own class. `model_flag` remains what the engine COMPOSES
   with; this ruling says what it READS, and it reads the declaration
   plus the one flag every model CLI this fleet has met accepts.

   **Enforcement binding:** `enforce_model_policy` in
   `crates/brokkr-runtime/src/bundle.rs` reads `route_pin` on the
   declared flag and on `--model` and joins the two readings; a test on
   a `-m` adapter with egress `contracted` and a secret-binding seat
   writing `--model elsewhere/large-1` is refused (the reviewer's
   scenario, now red before the fix); a test with `-m a/x --model b/y`
   is refused naming both flags; a test with `model_flag: "unsupported"`
   and `--model spark/x` is refused; a test with `-m spark/x` alone on a
   `spark: local` route still binds. Decision 0031's and 0035's readers
   (`model_pin`, `command_pins_effort`) do not change: they ask a
   different question of the four built-ins, all of which declare
   `--model`.

2. **A spelling is decided by the flag's shape, not by who is reading.**
   A LONG flag (two dashes) has exactly two spellings, `FLAG VALUE` and
   `FLAG=VALUE`; any other word beginning with it is a different flag of
   the same family, walked past by every reader — `--model-fallback` is
   not a way of writing `--model`, for 0036 as much as for 0031. A SHORT
   flag (one dash, one character) has three: `FLAG VALUE`, `FLAG=VALUE`,
   and the attached `FLAGVALUE` of the getopt convention, which is read
   as a pin whose value is the remainder; a remainder that is not one
   concrete model id is `Unreadable`, so `-march=native` under `-m`
   costs a secret-binding seat a refusal naming the flag, and nothing
   else.

   **Enforcement binding:** `UnreadSpelling` is replaced by a reading
   derived from the flag string in `command_pin`; tests pin the long
   neighbour walked past on both readers, the short attached form read
   as a route, and the short attached non-id refused.

3. **A refusal names the flags it read.** The `Unreadable` refusal
   names the declared flag, `--model`, or both, exactly as ruling 1's
   read used them; no constant stands in for a flag the read did not
   use.

   **Enforcement binding:** the `unwrap_or("--model")` fallback goes;
   the refusal text is asserted in ruling 1's tests.

4. **Ambient means unbound by any seat, not absent from the store.**
   `brokkr doctor --bundle` reports a route's credential as ambient when
   the process environment holds the variable and NO seat of the
   inspected bundle declares it in `secrets` — store membership is
   necessary for a binding but is not one. Without a bundle to inspect,
   doctor reports store membership and says that is what it checked.

   **Enforcement binding:** `report_ambient_credentials` in
   `crates/brokkr-cli/src/doctor.rs` takes the inspected bundle's
   declared secret names; tests cover held-and-declared (silent),
   held-and-undeclared with the variable exported (warn), and the
   no-bundle wording.

5. **A route name is whatever a model id may begin with.** `routes` and
   `credentials` keys share the concrete model id's alphabet — ASCII
   letters of either case, digits, `-`, `_`, `.` and `:` — and exclude
   `/`, because a route is the prefix of a concrete model id and must be
   able to name every prefix `resolve_route` can produce. The agent-name
   grammar stays what it is for agents, adapters and abstract model
   names.

   **Enforcement binding:** a `ROUTE_GRAMMAR` constant in
   `crates/brokkr-runtime/src/agents/load.rs`, used for both maps and
   quoted in their refusals; a test declares `us.east` and
   `openai_compat` and resolves a pin through each.

## Consequences

The population 0036 was written for — adapters the operator adds — loses
its last fail-open on the flag axis without the engine inventing a
grammar for any CLI it has not met: it reads the declaration and the one
flag that is universal in practice, and refuses where they disagree. The
two remaining questions from #160 close in the same small slice, one a
false negative on the ambient channel, one a route name that could not
be written.

No adapter file changes, so no bundle digest moves and no witness is
re-pinned; the wire protocol is untouched; no journal row is rewritten.
The cost is one refusal shape the shipped tree cannot trip and a
doctor wording that becomes more exact.

What this does not rule: whether a provider's CLI accepts `--model` at
all. That is the provider's grammar, and a pin the provider ignores is
still a pin the operator wrote — the material goes where the argv says,
and the compiler's job is to read what is written.
