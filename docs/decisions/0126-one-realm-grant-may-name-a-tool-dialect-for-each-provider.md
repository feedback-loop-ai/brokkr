# 0126 — One realm grant may name a tool dialect for each provider

Status: proposed
Date: 2026-11-24

Built: unbuilt (#1329)
Amends: 0065

## Context

Decision 0065 ruling 3 has a realm grant pick "the dialect that serves
the capability in this realm". A capability entry therefore names one
`dialect` (`crates/brokkr-core/src/realms/grants.rs:19`,
`contracts/realms.v8.schema.json`). A provider-native dialect names one
provider (ruling 2; `dialects/tools/claude-native-fetch.json`). A seat
on any other provider cannot carry it: a `requires` ask refuses
compilation, and a `wants` ask is a recorded drop (ruling 5;
`crates/brokkr-runtime/src/capabilities.rs:1250-1255`).

On 2026-11-24 the operator granted the researcher office read-only web
access ("read only web access", #1329). #1330 enacted that grant for
Claude seats. `realms.json` grants `web-fetch` through
`claude-native-fetch` and `web-search` through `claude-native-search`,
each to `offices: ["researcher"]`.

The weekly sweep seats the same office on dsh (decision 0044 ruling 5,
`recipes/research-dsh`). The same day, dsh 0.2.1-rc.2's native web
tools were measured ON and OFF, one tool at a time (#1329, comment
7142489320). `tool-web`'s `fetch` and `search` config keys each remove
one tool from the session's catalog. So dsh can carry the grant once
its dialects exist. The realm, though, cannot say so: the entry's one
`dialect` slot already holds Claude's dialect.

Two shapes were weighed:

- **(a)** Let one grant name a dialect for each provider. This needs a
  new realms version.
- **(b)** Keep one dialect per entry, and give the weekly workflow a
  second realm map that grants the web through dsh. This changes no
  rule. But the operator's one grant would then live in two files that
  can drift apart: a change to the offices, or a later revocation, made
  in one file and not the other.

The operator ruled (a) on 2026-11-24, in chat: "go with a" (#1329,
comment 7145511421). This decision writes that choice down. Accepting
this text is the operator's act.

## Rulings

1. **Under `forge.realms/v9`, a grant may write `dialects` in place of
   `dialect`.** `dialects` is a map from a provider's name to the tool
   dialect that serves the capability on that provider's seats:

   ```json
   "web-fetch": {
     "dialects": { "claude": "claude-native-fetch", "dsh": "dsh-native-fetch" },
     "offices": ["researcher"]
   }
   ```

   - A grant writes exactly one of `dialect` and `dialects`.
     - Writing both is refused at load.
     - Writing neither is refused at load.
     - An empty map is refused at load.
     - Each refusal names the realm and the capability.
   - A provider key is the name an adapter declares as its `provider`
     (`adapters/claude.json:2`, `adapters/dsh.json:2`).
   - v1 to v8 stay frozen.
     - Under them, `dialects` stays a restriction key, which the
       selected dialect's schema judges as it does today.
     - An older map therefore never gains the per-provider form just by
       spelling the key.
   - A v9 map that writes no `dialects` reads exactly as a v8 map does.

   Enforcement binding:
   - `contracts/realms.v9.schema.json`;
   - the grant parser in `crates/brokkr-core/src/realms/grants.rs`;
   - the tests of both.

2. **Each named dialect is judged as ruling 3 judges the single one.**
   - Each dialect must exist under `dialects/tools/` and must serve the
     entry's capability.
   - A provider-native dialect must sit under its own provider's key.
     - `"dsh": "claude-native-fetch"` is refused at load.
     - The refusal names the key and the dialect's provider.
   - An `mcp` dialect may sit under any provider's key. Whether that
     provider's adapter can carry `mcp` is judged per seat, as it is
     today.

   Enforcement binding: the realm loader's refusals and their tests.

3. **A seat holds the dialect under its own provider's key, and nothing
   else.**
   - The rest of the grant, `offices` and `retain`, applies to whichever
     dialect the seat holds.
   - A seat whose provider has no key holds nothing through this grant.
     - A `requires` ask refuses compilation.
     - A `wants` ask is dropped and the drop is recorded.
     - This is exactly how ruling 5's third bullet treats a dialect the
       seat's provider cannot carry.
     - The refusal or the notice names the providers the grant does
       serve.
   - There is no fallback across keys. A dsh seat never holds Claude's
     dialect, and a provider with no key never borrows another
     provider's dialect.

   Enforcement binding: the holding judgment in
   `crates/brokkr-runtime/src/capabilities.rs` and its tests.

4. **A per-provider grant does not narrow.**
   - A grant that writes `dialects` writes no `tools` subset and no
     dialect restriction keys.
   - Both are spelled in one dialect's vocabulary, so one list cannot
     speak for several dialects.
   - A grant that must narrow stays a single-`dialect` grant.

   Enforcement binding: the v9 schema's closed property list for a
   per-provider grant, and the parser's refusal.

5. **The record says which dialect served each seat.**
   - `brokkr doctor` names each provider's dialect for a per-provider
     grant.
   - A seat's capability record already names the one dialect it holds
     (`through dialect '<name>'`), so the run manifest records one
     dialect per holding, as it does now.
   - The run manifest's realms pin carries the v9 bytes.

   Enforcement binding: the doctor's capability tests.

6. **Brokkr's own map moves to v9 only together with the dsh dialects.**
   - That move lands in the same change as four other parts:
     - the `dsh-native-fetch` dialect;
     - the `dsh-native-search` dialect;
     - dsh's measured native inventory in `adapters/dsh.json`;
     - the composition of that inventory into the staged overlay.
   - That change moves the researcher grant to `dialects`, scoped to
     `["researcher"]` exactly as it is today.
   - Until that change lands, the v6 map stands.
   - The grant adds no office, and names no provider beyond Claude and
     dsh. Widening either needs a new operator ruling (#1329).

   Enforcement binding: two tests pin the map whole,
   `realms::the_repository_carries_the_bootstrap_map` and the doctor's
   shipped-realm test.

## Consequences

- The researcher's web grant stays one entry in one file. Revoking it,
  or changing its offices, is one edit that reaches both providers.
- A realm can grant one capability on several providers without a
  second map. Every other grant reads as it did.
- Decision 0082's lifecycle gains one contract version, `realms.v9`.
  v1 to v8 keep their readers.
- dsh's `tool-bash` remains a residual risk, which the operator accepted
  on 2026-11-24 ("accept it as known risk for now", #1329). Until the
  shell is switched off, the research-dsh seat can reach the web through
  it without any grant. Every PR that builds this decision names that
  residual.
- Decision 0107 rules what a gate grant must prove. Its condition 6
  says `tool-web` "switches both" native web fetch and native web
  search. The research seat is a work seat, not a gate, so 0107 does not
  bind it, and this decision does not amend 0107. The measurement that
  each `tool-web` config key switches one tool belongs on #1052.
