# 0087 — The library's base offices are the ones `init` writes, and Brokkr's own needs are offices of their own

Status: accepted — operator ruled 2026-10-18 (PR #740), every choice asked taken as recommended
Date: 2026-10-18

Built: partial (#717) — slice 1 (ruling 8) built rulings 1 to 4, 6 and 7; slices 2 and 3 are unbuilt
Amends: 0078
Amended by: 0091

## Context

Issue #717: none of the 18 recipes under `recipes/` installs onto the
shelf of a fresh folder. The operator ruled option A on 2026-10-17,
recorded on #717: Brokkr's library base agents (`implementer`, `reviewer`,
`intake`) become portable, with no Rust-specific hands and models chosen as
`init` chooses them; `init` writes those same agents for the agent CLI it
finds, and the library's recipes extend them; Brokkr's own needs (the
cargo and rustup binds, its model picks) move to a Brokkr-only layer, an
overlay in Brokkr's realm or agents under other names that only
`recipes/self` and Brokkr's own runs seat. The ruling asks for a decision
first, because it touches decision 0078. This file is that decision.

**What was measured for it, at ae90406a.** The HEAD binary's `init` was run
in fresh `git init` folders under `.forge/`, each with a search path holding
only a stub file named for one agent CLI (`cargo run --locked -p brokkr-cli
--config 'env.PATH.value="<stub dir>"' --config 'env.PATH.force=true' --
init <folder>`). It printed `Scaffolded for` `claude`, `codex` and `dsh` in
turn, digests `77653b7f…`, `732c79a1…` and `9ac2aeca…`. Two more folders,
one holding a `Cargo.toml` and one a `package.json`, were scaffolded under
the claude stub to read the stack grants. Then `brokkr recipes add
<checkout>/recipes/<r> --name <r>` was run in them. Every refusal below is
that command's output, with the scratch root written `<root>`:

- claude, no stack, `fast`: `error: recipe 'fast' does not compile
  (removed): bundle: agent 'fast-reviewer'
  (<root>/.brokkr/agents/fast-reviewer.json)` followed by the overlay rule
  on a reason given for `hands`, which the folder's `reviewer` does not
  declare.
- dsh, `fast`, and claude with a `package.json`, `node`: the same refusal,
  the second ending `(composed: node -> fast)`.
- codex, `fast`: `error: recipe 'fast' does not compile (removed): bundle:
  seat 'implement': agent 'fast-implementer' names model 'fable', which no
  adapter maps; adapters consulted: <root>/.brokkr/adapters/codex.json,
  <root>/.brokkr/adapters/exec.json`
- claude, no stack, `triage`: the #696 refusal that no agent definition or
  charter is overwritten, naming `charters/implementer.md`,
  `charters/reviewer.md`, `implementer.json` and `reviewer.json` under
  `<root>/.brokkr/agents/`.

So two causes join the three #717 lists. Under codex every link of a chain
must map (`crates/brokkr-runtime/src/agents.rs:820`, `UnmappedModel`), and
the fast overlays pin claude's names (`agents/fast-implementer.json:4`).
And the charters differ as well as the definitions: the library's name
`.forge/tasks/` (`agents/charters/implementer.md:4`, `intake.md:8`,
`reviewer.md:5`), while `init`'s name `.brokkr/state/tasks/` and contain no
`.forge/` at all, as decision 0083 ruling 1 binds.

**Two rules of agent composition shape the layer.** Both are #360's, the
operator's ruling of 2026-10-07 on decisions 0041 and 0058, enforced in
`crates/brokkr-runtime/src/agents/load/overlay.rs`. An overlay must write
`models` (lines 81 to 85: "an overlay states the chain it hires"). And an
overlay extends a base office one level deep: a base that itself extends is
refused (lines 148 to 153). An overlay's identity is the digest of the
definition it merges into, not of its own file (`overlay.rs:56` to `69`;
`agents/load.rs:544`), and the digest is canonical JSON
(`brokkr_core::canonical::sha256_hex`; no `preserve_order` feature is on),
so key order and file layout never move it.

**The alternatives weighed.** #716's landing measured four candidate
changes, recorded on #717 on 2026-10-16: (1) drop the `hands` reason from
`fast-reviewer`, not enough alone, because `fast-implementer`'s `cargo`
tool is unmapped by `init`'s claude adapter; (2) drop `tools` and the
reasons from both fast overlays, which makes `fast` and `node` install and
compile but changes fast's permissions for Brokkr's own runs; (3) have
`init` write Brokkr's offices byte for byte, which needs a change to 0083's
scaffold and 0065 grants for the Rust hands and tools in every adopter's
folder; (4) copy an agent's `extends` base, which needs a change to the
#696 addendum's ruling 5 and a conflict policy, and is still refused by
`Hiring::Differs`. The issue's option B (each recipe ships its own
offices) duplicates every office per recipe, and option C (relax the
reason rule) leaves the recipes that seat base offices directly refused.
The operator ruled option A over all of them. This decision is candidate
(2) made safe: the portable overlays lose their Brokkr picks, and Brokkr's
runs keep them in offices of their own.

**What this would touch, once accepted.**

- **0078.** Its ruling 1 says Brokkr's own bundles are library recipes and
  that an operator's library mirrors Brokkr's. This decision would amend
  0078 so that the library holds two kinds: portable recipes, which any
  folder installs, and Brokkr-only recipes, which Brokkr's realm runs and an
  adopter's shelf does not take. 0078's enforcement stands (CI compiles
  `recipes/self` and `recipes/verify`; `witnesses.json` pins every recipe),
  and so do its rulings 2 to 4.
- **0083.** Not changed. `init` writes the same files at the same paths,
  and its charters keep ruling 1's `.brokkr/state/` names. The #696
  addendum's ruling 5 (an agent's own `extends` base is not copied; it
  resolves in the workspace's library) is what portable recipes rely on,
  so it must stand as written; draft PR #737's edits to 0083 do not touch
  it.
- **0016.** Its one-definition rule is held, not changed: ruling 6 binds
  `init`'s offices to the library's by a test.
- **0065.** No grant moves. The portable base requests no tool, `init`'s
  stack grants stand, and Brokkr's own offices request what Brokkr's
  offices request today, through Brokkr's own adapters.
- **#360's overlay rule.** Ruling 3 asks for one change to it.

The header carries the `Amends: 0078` pointer, added on acceptance with
its counterpart in 0078's header: the index test pairs every pointer with
its counterpart (`crates/brokkr-cli/tests/decisions_index.rs`,
`pointers_are_paired`).

## Rulings

1. **The portable base set is what `init` writes under claude with no
   stack recognised.** Each of `agents/implementer.json`, `reviewer.json`
   and `intake.json` becomes the definition `init` writes for that office
   when it finds claude and recognises no stack, key for key. The
   comparison, measured:

   | Key | `implementer` today | `intake` today | `reviewer` today | `init` writes (all three) | Portable base |
   |---|---|---|---|---|---|
   | `description`, `charter`, `limits` | as `init` | as `init` | as `init` | the library's text, `charters/<office>.md`, the library's bounds | unchanged |
   | `models` (claude) | `opus`, `sonnet` | `sonnet`, `opus` | `sol`, `fable`, `opus` | implementer `opus`, `sonnet`; intake `sonnet`, `opus`; reviewer `fable`, `opus` | `init`'s |
   | `efforts` (claude) | `high` each | `high` each | `sol` `high`, `fable` and `opus` `xhigh` | `high` each | `init`'s |
   | `tools` | `allow` `cargo`, `git`; `mcp` empty | `allow` `git`; `mcp` empty | none | none with no stack | none |
   | `hands` | none | none | workspace, no network, `~/.cargo` overlay masking credentials, `~/.rustup` read-only | none under claude and dsh | none |

   Sources: `agents/implementer.json:4-18`, `agents/intake.json:4-17`,
   `agents/reviewer.json:4-31`, `crates/brokkr-cli/src/init.rs:382-413`, and
   the `agents/*.json` the claude scratch folder holds. Under the other
   CLIs, and with a stack, `init` writes the same definition with four keys
   projected, as its code says and the scratch folders show:

   - `models` and `efforts` are the found CLI's chain (`init.rs:352-358`):
     codex hires intake and implementer on `sol` `medium` then `terra`
     `high`, and the reviewer on `sol` `medium` alone; dsh hires intake on
     `flash`, `pro` and the implementer on `pro`, `flash`, all `high`, and
     keeps the reviewer on claude's chain, because dsh cannot hold a gate
     (decision 0021 ruling 2).
   - `hands` is written for a codex-hired office only: workspace, no
     network, the stack's binds (`init.rs:599-601`), which are `~/.cargo`
     and `~/.rustup` for a cargo stack and none otherwise (`init.rs:205-213`).
   - `tools` is written for a claude-hired office with a recognised stack
     only (`init.rs:651-660`): the work offices get the stack's runners plus
     `git`, `ls`, `rg`, `mkdir`, and the reviewer the test runner plus
     `git`, `ls`, `rg`. Measured: `cargo` first for the `Cargo.toml`
     folder, `npm` first for the `package.json` one; the npm folder's claude
     adapter maps `git`, `ls`, `mkdir`, `npm`, `rg` and no other name.

   Every difference, then: the portable `implementer` loses `cargo` and
   `git`; the portable `intake` loses `git`; the portable `reviewer` loses
   its Rust binds, `sol`, and the `xhigh` efforts. Charters are not part of
   the portable unit. A folder's base charters are its own, written by
   `init` for its stack, and a base office is never copied by `recipes add`,
   so its charter never crosses. The library's base charters stay Brokkr's.

2. **The portable overlays write only what is theirs.** `fast` and `node`
   seat four overlays, and each keeps only the keys that make it that
   office:

   | Overlay | Keeps | Loses |
   |---|---|---|
   | `fast-implementer` | `extends` `implementer`, `charter` `charters/implementer-intakeless.md` | `models`, `efforts`, `tools`, its `tools` reason |
   | `fast-reviewer` | `extends` `reviewer`, `inputs` | `models`, `efforts`, `tools`, both reasons |
   | `node-implementer` | `extends` `implementer`, its charter | `models`, `efforts`, `tools`, its reason |
   | `node-reviewer` | `extends` `reviewer`, its charter, `inputs` | `models`, `efforts`, `tools`, both reasons |

   Each then hires the folder's chain, tools and hands. `node` loses `npx`
   and `node` on its model seats, because `init`'s npm grant names neither
   (measured above); its verify seat, an exec script, still runs `npm ci
   --offline` and `npm test` (`recipes/node/roles/verify-seat.sh:29-31`).

3. **An overlay may inherit its base's chain.** An overlay that writes
   neither `models` nor `efforts` hires its base's chain and efforts whole.
   One that writes `efforts` without `models` is refused, because efforts
   are keyed by the chain's candidates. Every other rule of #360 stands:
   powers change only under a reason, a reason for nothing is refused, and
   an overlay extends one level. Under codex this is what lets `fast`
   compile, because no claude name maps there. Under dsh `init` hires the
   reviewer from claude (`init.rs:352-358`), so the folder holds
   `adapters/claude.json` and claude's names map; there inheritance is what
   makes the work overlays follow `init`'s dsh chain rather than claude's.

4. **Brokkr's own needs are offices under other names.** The Brokkr-only
   layer is five overlays of the portable base, each restating today's
   Brokkr picks, so that each merges to exactly the definition Brokkr's
   seats resolve today:

   | Office | Extends | Writes |
   |---|---|---|
   | `brokkr-implementer` | `implementer` | `models` `opus`, `sonnet` at `high`; `tools` `allow` `cargo`, `git`, `mcp` empty, under a reason |
   | `brokkr-intake` | `intake` | `models` `sonnet`, `opus` at `high`; `tools` `allow` `git`, `mcp` empty, under a reason |
   | `brokkr-reviewer` | `reviewer` | `models` `sol` `high`, `fable` and `opus` `xhigh`; today's `hands`, under a reason |
   | `brokkr-fast-implementer` | `implementer` | today's `fast-implementer` keys |
   | `brokkr-fast-reviewer` | `reviewer` | today's `fast-reviewer` keys, less the `hands` reason |

   One Brokkr-only recipe, `brokkr-fast`, extends `fast` and seats
   `brokkr-fast-implementer` at `implement` and `brokkr-fast-reviewer` at
   `review`, so fast's two model seats keep Brokkr's picks in one place.
   The re-points, each a one-line change:

   - `landing`, `wager-harness`, `wager-harness-dsh` and
     `wager-harness-muse` extend `brokkr-fast` in place of `fast`, because
     each inherits at least one of fast's model seats.
   - `panel-review` seats `brokkr-intake` and `brokkr-implementer`; `self`,
     `release` and `triage`'s `chore` case seat `brokkr-reviewer`;
     `triage`'s `chore` and `feature` cases seat `brokkr-implementer`.
   - The overlays whose reason would now be for nothing drop it, and only
     it: `gpt-flash-implementer`, `muse-implementer`,
     `night-shift-implementer` and `wager-glm-flash-implementer` (the
     `tools` reason), and `verify-reviewer` (the `hands` reason). Each merges
     to the same definition as today, because the key it removed is no
     longer in its base.

   **Why names, not a realm overlay.** Names need no new mechanism: agent
   `extends`, its reasons and recipe `extends` exist today, and every
   compile reads the one `agents/` library. A realm overlay would make a
   recipe compile to one office in Brokkr's realm and another everywhere
   else, an input decision 0016's "same bundle, same resolution" does not
   name and the witness table, which compiles each recipe outside any
   realm, would not see. It would also be engine work: a realm key, its
   digest in the manifest, and a compile that reads it. The overlay is the
   alternative the operator may take; it saves the re-points and the
   moved digests of ruling 7, and costs that mechanism.

5. **Every recipe is portable or Brokkr-only, with its reason.**
   `recipes/` holds these 18 (`ls recipes`):

   | Recipe | Class | Reason, from its bundle |
   |---|---|---|
   | `fast` | portable | Seats `fast-implementer` and `fast-reviewer` only (ruling 2). Its verify and ship are exec scripts, and they remain Brokkr's; see the first item asked below. |
   | `node` | portable | Extends `fast`; seats `node-implementer` and `node-reviewer` (ruling 2); verifies with npm under an `~/.npm` bind. |
   | `gpt-flash` | Brokkr-only | Extends `triage`; seats fifteen `gpt-flash-*` offices hiring `sol`, `flash` and `astra`, codex's and dsh's names at once, over triage's panel offices. |
   | `landing` | Brokkr-only | Brokkr's by-hand road (decision 0051): its classifier reads `.github/delivery-classes.json`, Brokkr's docs class (decision 0038), and it inherits fast's model seats with Brokkr's picks through `brokkr-fast`. |
   | `night-shift` | Brokkr-only | Extends `triage`; seats `night-shift-implementer` on `glm-flash` through dsh. |
   | `panel-review` | Brokkr-only | Its panel, `review-correctness` and `review-security`, binds `~/.cargo` and `~/.rustup` and leads with `sol` or `fable`; its intake and implement seats take Brokkr's picks. |
   | `preflight` | Brokkr-only | Its verify binds the Rust toolchain; its review is an inline claude driver pinned to `claude-fable-5-1` with `cargo` granted. |
   | `release` | Brokkr-only | `release-manager` binds `~/.cargo`, `~/.rustup` and the resolver files, for Brokkr's release configuration; its review takes `brokkr-reviewer`. |
   | `research` | Brokkr-only | `researcher` writes Brokkr's research registry (decision 0044); its verify binds the Rust toolchain. |
   | `research-dsh` | Brokkr-only | Extends `research`; an inline dsh driver on `dashscope/qwen3.8-max` with its own patch file. |
   | `review-first` | Brokkr-only | A forced crew (decision 0060): an inline codex review on `gpt-6.1-sol`, and `muse-implementer` on `muse-contributor` through dsh. |
   | `self` | Brokkr-only | Brokkr delivering itself (decision 0005); extends `panel-review`; its review takes `brokkr-reviewer`. |
   | `standby` | Brokkr-only | A hedge that forces its crew (decision 0041's addendum): both model seats are inline codex drivers on `gpt-6.1-sol`. |
   | `triage` | Brokkr-only | Its design office, `chief-architect`, hires `sol` and `astra`, and its clarify and analyze offices, `clarifier` and `analyst`, hire `sol`, each beside claude's names; its `review-*` offices and `implementer-engine` bind the Rust toolchain. |
   | `verify` | Brokkr-only | Its verify binds the Rust toolchain and runs Brokkr's verifier; `verify-reviewer` grants `cargo`, `gh-pr-view` and `gh-run-view`. |
   | `wager-harness` | Brokkr-only | A forced arm (decision 0041 ruling 7): an inline codex implementer on `gpt-6.1-sol`. |
   | `wager-harness-dsh` | Brokkr-only | A forced arm: `wager-glm-flash-implementer` on `glm-flash` through dsh. |
   | `wager-harness-muse` | Brokkr-only | A forced arm: `muse-implementer` on `muse-contributor` through dsh. |

   The new `brokkr-fast` is Brokkr-only. Of the non-base offices, only the
   four overlays of ruling 2 are portable. Every other office in `agents/`
   (the `gpt-flash-*`, `muse-*`, `night-shift-*`, `review-*`, `position-*`
   and `wager-*` offices, `chief-architect`, `clarifier`, `analyst`,
   `triage`, `implementer-engine`, `implementer-sdd`, `release-manager`,
   `researcher`, `verify-reviewer`, `muninn`, and ruling 4's five) is
   Brokkr-only: no portable recipe names one. Slice 2 (ruling 8) marks
   each recipe's class where the guides name Brokkr's recipes, pointing at
   this table; no recipe file carries the mark, so it moves no digest.

6. **`init` writes the library's base, held by one test.** `init` keeps
   its own table (`init.rs:382-413`), because `brokkr-cli` is published on
   its own and cannot include a file outside its crate. The table becomes a
   copy held to its source, as `init`'s adapters already are
   (`crates/brokkr-cli/src/init/adapters.rs:54`, held by `init_stacks`).
   A new case in `crates/brokkr-cli/tests/init_stacks.rs` runs `init`
   under claude, codex and dsh, each with no stack, a cargo stack and an
   npm stack. For every office it asserts that the written
   `.brokkr/agents/<office>.json` parses equal to the library's
   `agents/<office>.json` after ruling 1's projection (the found CLI's chain
   and efforts, codex's hands, claude's stack tools). With claude and no
   stack the projection is the identity, and the files are equal byte for
   byte: the library's three files take `init`'s serialization, keys sorted,
   two-space indent, a closing newline. The model choice lives in that
   table: claude's chain also in the library file, codex's and dsh's in
   the table alone.

7. **Brokkr's own runs resolve the same offices; the names in their
   manifests move.** A compiled manifest pins its directory's files, each
   composed base's file map (`crates/brokkr-runtime/src/bundle/compose.rs:56-66`),
   and, per agent seat, the agent's name, its merged definition's digest,
   its charter's digest, the adapters consulted and the chain
   (`crates/brokkr-runtime/src/agents.rs:1637-1648`). Ruling 4's offices
   merge to today's definitions, so for every Brokkr-only seat the agent
   digest, charter digest, chain, chosen model, provider and hands are what
   they are today. What moves is the seat's agent name, or a bundle file
   the re-point edits. The build measures the move by `BROKKR_BLESS=1 cargo
   test -p brokkr-runtime --test it witness_digests::` and names each moved
   value in its pull request. The prediction, from the mechanics above:

   - **Bundle digests that move:** `fast` and `node` (their overlays
     change, ruling 2, which changes behaviour, and only for portable
     recipes); `landing` and the three `wager-harness` recipes (`extends`
     and seat names); `panel-review`, `self`, `triage` and `release` (seat
     names); `gpt-flash` and `night-shift` (triage's file map, as a
     composed base); `review-first` and `standby` (fast's file map, as a
     composed base). Slice 1's note in `recipes/fast/README.md` (the
     fourth item asked below) changes a file the walk pins
     (`crates/brokkr-runtime/src/bundle/walk.rs:96-98`), and compose seals
     each ancestor's file map into the composed manifest
     (`compose.rs:148-170`), so every recipe that extends `fast`, directly
     or through `brokkr-fast`, `triage` or `panel-review`, moves. That
     move is behaviour-preserving because no seat, gate or policy reads
     the README.
   - **Bundle digests that stay:** `verify`, `preflight`, `research` and
     `research-dsh`. Each seats only offices whose merged definition is
     unchanged, or none, and extends nothing or a base whose files do not
     change.
   - **Tables and charters:** no policy table and no charter's bytes
     change, so none of those witnesses moves. `brokkr-fast` gains a bundle
     row and a table row.

   Each move is behaviour-preserving because what a seat runs is its
   resolved hire, argv and hands, and those are equal. The slice shows this
   once: before and after, the compiled `agents` records of every
   Brokkr-only recipe are compared with `agent` set aside, and the
   comparison is recorded in the pull request. The pipeline libraries
   Brokkr's own runs use, the story and landing libraries kept outside
   this repository, are consumers: where one seats a base office directly
   or extends `fast` for its model seats, it takes the same re-point when
   the slice lands.

8. **The build, in slices against #717's acceptance.**

   - **Slice 1, the first: the split.** Rulings 1 to 4 and 6 as data, plus
     ruling 3's change in `overlay.rs` with its refusal tested, the
     re-points, ruling 7's re-pin, and one `crates/brokkr-cli` test per
     agent CLI (claude, codex and dsh): in a fresh folder made with `git
     init` and `init`, `recipes add` of `recipes/fast` and of
     `recipes/node` exits 0, copies `fast-implementer`, `fast-reviewer` and
     `charters/implementer-intakeless.md` (and node's two offices with
     their charters), and the installed recipe compiles. "Reaches its first
     seat" is shown per CLI for both recipes: `brokkr run --recipe fast`
     and `brokkr run --recipe node`, each under claude, codex and dsh in
     a scratch folder, six runs, with each journal's first seat row
     recorded in the pull request.
   - **Slice 2: the guides.** The quickstart's "Try another recipe",
     `docs/guides/adopting-a-node-repo.md` and `docs/guides/cards/node.md`
     show the install succeeding in place of the refusals they print
     today, each block sampled by #365's census in
     `crates/brokkr-cli/tests/doc_samples.rs`. The same slice marks each
     recipe's class where the guides name Brokkr's recipes (ruling 5).
   - **Slice 3, on the operator's rulings below:** fast's verifier and
     shipper, and the overlay charters' run-local paths.

## Asked of the operator

The ruling of 2026-10-17 settles option A and the two forms the layer may
take. These choices remain, each with the recommendation first:

- **The layer's form.** Offices under other names, as ruling 4 lays out.
  Recommended, for its reasons; the realm overlay is the alternative.
- **The names.** A `brokkr-` prefix for the five offices and the
  `brokkr-fast` recipe. Recommended, because the prefix says whose they
  are wherever they are listed. The alternative is `self-`, after the
  recipe Brokkr delivers itself with.
- **Ruling 3, an overlay inheriting its base's chain.** Recommended,
  because "models chosen as `init` chooses them" cannot hold for an
  overlay that must restate a chain in one CLI's names. The alternative is
  to keep #360's rule and ship `fast` portable under claude only, which
  meets #717's acceptance for claude and leaves codex and dsh refused.
- **Fast's verifier and shipper.** `recipes/fast/scripts/verify-seat.sh`
  runs Brokkr's own checks: `bash scripts/lint-non-rust.sh --seat` (line
  80) and `cargo run -p brokkr-cli -- compile --bundle recipes/self` (line
  92), under the Rust binds (`recipes/fast/bundle.json:47-48`). The
  shipper's notes name `.forge/ledger/` (`ship-seat.sh:61`). So outside
  Brokkr a `fast` run reaches implement and review, and its verify fails.
  Recommended: slice 1 leaves both, says so in fast's README, and slice 3
  moves Brokkr's verifier to `brokkr-fast` and gives `fast` a portable
  one. The alternative is to move them in slice 1. Either way, how a shelf
  recipe's pinned script reaches the folder's own `scripts/verify-seat.sh`
  under decision 0048's spawn pin needs a ruling of its own.
- **The overlay charters' run-local paths.**
  `charters/implementer-intakeless.md:20` and `node-implementer.md:20` send
  scratch under `.forge/`, and `node-reviewer.md:5` reads `.forge/tasks/`.
  In a folder, state is `.brokkr/state/` and `.forge/` is not ignored.
  Recommended for slice 3: layout-free wording ("under the run-local
  directory that holds your result file"), which moves those charter
  witnesses and the recipes that seat them, `review-first` included
  through `muse-implementer`. The alternative names `.brokkr/state/` and
  gives Brokkr's seats charters of their own, a second copy that
  decision 0071 ruling 5 refuses.
- **The library files' bytes.** `init`'s serialization, so claude with no
  stack compares byte for byte (ruling 6). Recommended. The alternative
  compares parsed values only and keeps the library's key order.
- **Where `init`'s chains live.** In `init`'s table, held by the test.
  Recommended, because the CLI crate cannot package `agents/`. The
  alternative embeds a copy of `agents/` inside `crates/brokkr-cli`, held
  by the same kind of test.
- **`node`'s model seats lose `npx` and `node`.** Recommended as the
  folder's to widen, in its claude adapter and agents. The alternative
  extends `init`'s npm rows to grant both, a change to the grants `init`
  scaffolds.
- **`panel-review`.** Brokkr-only now. Recommended to stay so until
  `review-correctness` and `review-security` have portable forms, which is
  a follow-up of its own.

## Consequences

- `brokkr init .`, then `brokkr recipes add <checkout>/recipes/fast --name
  fast`, installs and compiles under claude, codex and dsh once slice 1
  lands, and the same holds for `node`. Sixteen recipes are listed as
  Brokkr's own, each with its reason.
- `fast` and `node` behave differently wherever they run, Brokkr
  included: their model seats hire the folder's chain, tools and hands.
  Brokkr's own runs that need fast's picks go through `brokkr-fast`.
- A run that started before slice 1 lands and is resumed after it is
  refused as drift for every recipe whose digest moved (decision 0017's
  pin). The slice lands when no such run is parked, or the run resumes
  from its pinned bytes.
- Tests that pin the library's base offices' contents, such as an `agents
  show` or recipe-library case, move with ruling 1, and each is named in
  slice 1's pull request.
- Only the operator accepts this proposal. Filing it builds nothing,
  moves no digest, and certifies no host run, remote CI or publication.
