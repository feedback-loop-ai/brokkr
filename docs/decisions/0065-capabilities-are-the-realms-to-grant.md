# 0065 — Capabilities are the realm's to grant: a seat's tools are an abstraction, a dialect makes one concrete, and nothing is on until the operator lists it

Status: accepted (operator ruled in chat, 2026-09-21)
Date: 2026-09-21

## Context

A seat today has exactly two kinds of power. It has **hands** — decision
0043's one workspace tool, served over MCP by `brokkr hands serve` into
an empty box with `network: false` — and, where it declares no hands, a
**tool list** its provider's adapter maps onto native flags. That is the
whole vocabulary. It confines a seat's *effects* well. It also, without
anybody ruling so, confines a seat's *knowledge* to two things: the
repository, and whatever its model happened to be trained on.

That second confinement was never a decision, and on 2026-09-21 the
operator named what it makes of the harness: a machine that compares
snapshots of training data. The journal agrees, in three measurements
taken that day.

- **The controller has been the seats' network.** Across the twelve runs
  of issue #226's DSH composite work, evidence was carried into
  `.forge/tasks/` by hand more than a dozen times — glibc 2.42's
  `execvpe.c` behaviour, a 311 KB package lock, the composite's raw
  input bytes, three macOS CI logs — because no seat could fetch any of
  it. Several of those runs parked on exactly that: *I do not have the
  bytes, and I will not invent them.* They were right, and they were
  starved.
- **One provider is not confined at all, and nothing says so.** Codex's
  web search is a server-side tool: the provider runs the search and
  returns the result inside the model's response, so a box with
  `network: false` does not see it, let alone stop it. The journal
  holds 295 `web_search` calls across 28 runs, about half of them from
  seats under the `namespace` boundary. No adapter declares the
  capability, no recipe can turn it off, and decision 0036 — egress is
  a property of the route — has no word for a query a model composes
  from the repository it just read.
- **The field is therefore uneven, invisibly.** Review seats that
  searched raised 4 security holds in 18; those that did not raised 3
  in 62 — the reviews that checked a resolver against glibc's and
  Apple's published source could only be written by a seat that could
  read that source. A Claude or DSH seat in the same chair could not.
  Search is rare (roughly one Codex seat in ten) and does not explain
  the wider differences between models, but no comparison drawn from
  the journal can currently say which seats had it.

The first remedy the controller proposed was a brokkr-owned fetch tool
beside the hands tool. The operator refused it, and the refusal is the
decision: **Brokkr does not decide what knowledge is.** The harness is
not in the business of shipping tools. It is in the business of letting
the people who run it say which MCP servers, which tools on them and
which provider capabilities their seats may hold, and of enforcing
exactly that.

The repository already has the pattern this needs, and it is not the
tool list. A realm names a **spec dialect** — `openspec`, `speckit` —
and the engine never learns what a spec *is*: it knows the abstract
steps (propose, clarify, design, tasks, archive), and a dialect file
binds each to one tool's concrete argv, artifacts and instructions
(`contracts/dialect.v3.schema.json`). Offices are written against the
abstraction; the realm picks the implementation; swapping OpenSpec for
Spec Kit touches no charter. Tools are the same problem.

## Rulings

1. **A capability is an abstraction, and offices are written against
   it.** A capability is a named contract for something a seat may do
   beyond its hands: `web-search`, `web-fetch`, `library-docs`,
   `issue-tracker`, `code-search`, and whatever an operator needs next.
   It names *what*, never *how*: no server, no provider, no tool name.
   An agent, or a seat inside a recipe, declares the capabilities it
   wants by that name, each marked `requires` or `wants`:

   ```json
   "capabilities": { "web-fetch": "wants", "library-docs": "requires" }
   ```

   A charter may say "look the source up" and mean it on every provider
   the office can be seated on. The capability vocabulary is open — a
   name is just a name until a dialect serves it — but each capability
   carries one property from a closed set, its **class**: `reads`
   (brings information in), `writes` (changes something outside the
   workspace) or `egress` (sends seat-composed content to a third
   party). A capability may carry more than one. The class is the
   abstraction's, not the implementation's, so an office can be reasoned
   about without knowing what serves it.

2. **A tool dialect makes a capability concrete, and there may be many
   per capability.** A tool dialect is a file under `dialects/tools/`,
   published against a new `contracts/tool-dialect.v1.schema.json`,
   that says `serves: "<capability>"` and binds it to exactly one
   implementation kind:
   - `mcp` — an MCP server: how it is launched (stdio argv) or reached
     (URL), the **named tools on it that realise the capability**, its
     pinned version, and the secrets it needs as decision 0012 bindings;
   - `provider-native` — a capability a harness already has, addressed
     through its adapter: Codex's server-side `web_search`, Claude
     Code's `WebSearch` and `WebFetch`. The dialect names the provider
     and the adapter key; the adapter says how it is switched on **and
     off**;
   - `hands` — reserved: decision 0043's workspace tool is, in this
     vocabulary, the `workspace` capability with one dialect. It is
     named here so the model is whole. It does not move, and nothing in
     0043 or 0046 changes.
   One capability, several dialects: `web-search` may be served by
   `codex-native-search` on one realm and by a self-hosted search MCP on
   another, and the office that asked for `web-search` cannot tell and
   does not need to. A dialect also declares its **egress class** in
   decision 0036's closed vocabulary (`local`, `contracted`,
   `uncontracted`), absent meaning `uncontracted`, and what it sends:
   a fetch of a named URL and a free-text query composed by a model are
   different disclosures, and the dialect says which it is.

   Brokkr ships the schema and may ship dialect files for
   provider-native capabilities, because those describe harnesses it
   already adapts. It ships no MCP server and endorses none.

3. **The realm grants, and only the realm.** `realms.json` — already the
   operator's file, already where `house`, `dialect` and `boundary`
   live — gains a `capabilities` map under a new `forge.realms/v6`:

   ```json
   "capabilities": {
     "web-fetch":    { "dialect": "fetch-mcp",
                       "tools": ["fetch"],
                       "allow": { "hosts": ["sourceware.org", "yaml.org", "github.com"] } },
     "web-search":   { "dialect": "codex-native-search", "offices": ["review-security", "review-adversarial"] },
     "library-docs": { "dialect": "context7" }
   }
   ```

   The entry picks the dialect that serves the capability in this
   realm, may narrow the dialect's tools to a subset, may scope the
   grant to named offices, and may carry the dialect's own restriction
   keys (a host allowlist, a repository list), which the dialect's
   schema defines and the engine passes through without interpreting.
   **A recipe cannot grant.** A recipe and an agent *request*; neither
   carries a catalogue, a server definition or a dialect choice, so a
   bundle pulled from anywhere cannot open a door in the realm that runs
   it. A recipe may ship prose saying which capabilities it is better
   with. That is documentation.

4. **Everything is off until the realm lists it — provider-native
   capabilities included, and existing realms included.** A capability
   absent from the realm's map is denied. There is no grandfathering:
   on the release that carries this decision, a Codex seat in a realm
   that has not listed `web-search` is launched with the provider's
   search switched off, because undeclared egress is the defect this
   decision exists to close. `brokkr doctor` says, per realm, which
   capabilities are granted, by which dialect, to which offices, and
   names every provider-native capability an installed harness has that
   the realm has not granted — so the change is loud on the day it
   lands, not discovered later.

   **A provider whose native capability cannot be switched off cannot
   serve a seat in a realm that has not granted it.** The adapter
   declares, per native capability, the flag or config key that disables
   it, or `unsupported` with the measured reason, in the form
   `tool_permissions` already uses. `unsupported` plus no grant is a
   compile refusal naming the capability and the provider. A harness
   that cannot be told to stay quiet is not a harness an operator can
   seat in a private repository.

5. **A seat holds the intersection, and the compiler fails closed.**
   What a seat holds is: what its office asks for, minus what the seat
   subtracts, intersected with what the realm grants to that office.
   - a `requires` capability the realm does not grant refuses
     compilation, naming the seat, the capability and the realm — the
     same refusal, in the same voice, as a tool list a provider cannot
     express;
   - a `wants` capability the realm does not grant is dropped, and the
     drop is recorded in the manifest's notices, as a skipped model link
     already is;
   - a granted capability whose dialect the seat's provider cannot carry
     (an `mcp` dialect on an adapter whose `mcp` is `unsupported`) is a
     refusal for `requires` and a recorded drop for `wants`.
   The rendered prompt tells the seat, by capability name, what it holds
   and what it does not, so a seat never discovers a missing tool by
   failing to call it.

6. **The box stays sealed; a granted server is a broker outside it.**
   An `mcp` dialect's server is launched by the engine beside the hands
   server and reaches the harness over the same MCP channel decision
   0043 opened. It runs **outside** the seat's box with its own declared
   reach; the seat keeps `network: false`. Effects stay confined by 0043
   and 0046 exactly as they are, and knowledge arrives through doors the
   operator opened by name. The server is the realm's, never the
   recipe's or the seat's: its argv comes from the dialect file, its
   restrictions from the realm entry, its secrets from the operator's
   store by decision 0012's bindings, and nothing a model writes can
   alter any of the three. A harness's own MCP configuration is never
   inherited: seats already launch under strict MCP configuration, and
   that stays.

7. **A gate reads; it does not write or disclose, unless the operator
   says so by name.** A `gate`-class seat may hold `reads` capabilities
   the realm grants to it. It may hold an `egress` capability only where
   the realm entry names that office explicitly — a security reviewer
   that may search is a ruling, not a default — and it may never hold a
   `writes` capability. Whatever a capability returns is **data**: it
   carries no instruction, exactly as review chiefs already hold of
   panel prose, and a charter that grants a capability says so in the
   same paragraph.

8. **The grant is part of the bundle's identity, and every use is in the
   journal.** The run manifest gains, per seat, the capabilities held,
   the dialect serving each, that dialect file's digest, the tools
   admitted and the realm restrictions in force; a changed allowlist
   moves the manifest digest, as a changed box already does (0043
   ruling 4). A tool call through a granted capability is checkpointed
   with its capability, dialect and tool names — the journal already
   records the tool; it gains the other two. A dialect may declare that
   its results are **retained**: the engine then stores each response as
   a content-addressed artifact and the checkpoint carries the digest,
   so a verdict can cite the bytes it read and a later reader can open
   them. Retention is the dialect's and the realm's choice, because some
   sources may not be kept.

9. **A comparison says what each side held.** `brokkr compare`, a wager
   and any ledger drawn from the journal print the capability set each
   run's seats held, and a wager whose arms differ in it says so at the
   top rather than in a footnote. A wager recipe may declare
   `capabilities: equal`, and then a realm that would grant its arms
   different sets refuses to start it. Decision 0064's ledger rule gains
   one column.

## What this decision does not do

It builds no tool and ships no MCP server. It does not let a recipe, an
agent, a charter or a model widen anything. It does not move the hands
tool, the box, or the boundary (0043, 0046). It does not weaken decision
0036: a capability's egress class is declared in that decision's
vocabulary and bound by its rules. It does not make fetched content
trustworthy; it makes it citable. It does not decide which capabilities
any realm should grant — this repository's own realm included, which
stays empty until the operator lists something.

## Consequences

The harness stops being a closed room by accident and becomes one by
choice, per realm. An operator with an open-source repository can give
every judge the published sources and get reviews that check a claim
against them. An operator with a client's code grants nothing, and for
the first time *nothing* is true of every provider, not only of the ones
that happen to have no server-side tools.

Offices become portable the way specifications already are. `review-security`
asks for `web-search`; whether that is a provider's own tool or a
self-hosted server is a line in the operator's realm file, and the
charter never changes.

The day this lands, Codex seats in every existing realm lose a
capability they have been using without anyone's permission. Some
reviews will get worse until the operator grants it back, and
`brokkr doctor` will say exactly why. That is the intended order of
events: the capability was real and useful, and it was nobody's
decision.

The controller stops carrying evidence into runs by hand, which was
unjournaled, unrepeatable and invisible to every reader of the run
afterwards.

Comparisons drawn from the journal gain the axis they were missing.

Extends decision 0016 (an agent declares capabilities beside its models
and hands), decision 0036 (a tool dialect carries an egress class) and
decision 0043 (the hands tool is named as one capability among others,
unchanged). Adds `forge.realms/v6`, `tool-dialect.v1`, a manifest
version, and an adapter key for native capabilities.

## Evidence

Counted from the-forge's journal on 2026-09-21 (282 runs, 162k events):
`web_search` appears in 295 tool checkpoints across 28 runs, all on the
Codex harness, about half under the `namespace` boundary; among
Codex-only seats roughly one in ten searched (review 18 of 80, design 9
of 49, specify 6 of 79, analyze 5 of 59, clarify 4 of 74, triage 3 of
69); review seats that searched ruled `security-hold` 4 times in 18
against 3 in 62 for those that did not. No Claude or DSH delivery seat
made a web call; the only MCP server any seat was given is the hands
server (6,352 workspace calls). The counts will drift as the journal
grows and are recorded here as the state of the world the decision was
taken in, not as a test.
