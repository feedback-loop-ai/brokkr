# 0044 — Research is an intake: the registry, the researcher, and the ruling stays the operator's

Status: proposed
Date: 2026-09-04

## Context

Brokkr is built in the deep-tech mould: the product is supposed to be
argued from what the field has found, not from taste. On 2026-09-04 the
operator had seven articles read against the tree in one sitting, a
benchmark of whole-repository migrations, a study of context privilege
escalation across twelve harnesses, a governance model for agent-built
software, a multi-day loop harness, a mining study of agent loops in
the wild, a practitioner's evaluation workflow, and a paper on whether
engineered scaffolding survives the next frontier model. The reading
found the product ahead of four of them as committed artifact and
behind the others in ways the decision record had never named: no
outcome calibration of a gate's verdict, no structural audit that the
intended change happened, no failure-source axis, and commit messages
reaching review seats at instruction privilege.

None of that had a place to live. The decisions record what the
product rules; the essays record what it argues; nothing records what
it has read and what it did about each finding, so the next reading
starts from nothing and the same paper is rediscovered by whoever
reads it next. Four shapes were weighed.

- **A wiki page or an essay per reading.** Rejected: prose with no
  grammar rots into opinion within a month, and a paper's five findings
  do not share one fate. One status per article would be wrong for
  nearly every paper read.
- **Labels on issues.** Rejected: an issue has one state and no
  per-finding rows, and the classification the operator cares about,
  what the product does about a finding and where to check, is not a
  workflow state.
- **A seat that reads and classifies.** Rejected as ruling: an agent
  grading the product's adoption of research it just read is the
  verifier theater the loop-engineering paper names. It may propose
  with evidence. It may not rule.
- **A registry with the decision record's own grammar.** Taken.
  Numbered files, a derived index, a closed vocabulary, a citation on
  every claim, and a status line only the operator flips. The
  researcher is an office in the library, its charter the configurable
  prompt, its egress an explicit grant, its cap the operator's number.

The operator's rulings in chat, 2026-09-04: classify per finding, not
per article; every class carries a citation; the classification is the
operator's, and that is the operator's job; the explicit egress grant;
at most ten articles per run; custom issue types are to be enabled so
an intake is an issue of type Research; and the operator wants a skill
so that the operator can hand the researcher articles as well. The
operator also declined to build on the single-prompt paper's headline
until it is replicated, which the registry records as that paper's
first row.

## Rulings

1. **The registry is a record with a grammar, and its index is
   derived.** `docs/research/` holds one numbered file per article the
   product has read: a source link, authors, the date read, a status, an
   intake line naming the run or the skill, a summary with the article's
   numbers, and a findings table with one row per adoptable practice.
   Each row's classification is one of a closed vocabulary:
   `implemented`, `alternative`, `declined`, `planned`, `not-planned`.
   The index in `docs/research/README.md` is exactly the entries in
   order with their findings count and status, and it is union-merged
   so that two intakes appending rows never conflict.

   **Enforcement binding:** `crates/brokkr-cli/tests/research_registry.rs`
   refuses a class outside the vocabulary, a malformed header, and an
   index that is not the entries; `.gitattributes` marks the index
   `merge=union`.

2. **Every classification carries a citation that resolves.**
   `implemented` and `alternative` cite a decision number or a path
   that exists in the tree; `declined` cites the decision or the charter
   it conflicts with; `planned` cites the issue the operator opened;
   `not-planned` cites nothing, and a reason belongs in the summary. A
   citation to a decision that does not exist, a path that is not in
   the tree, or an issue with no number is a test failure, not a
   review finding.

   **Enforcement binding:** the same test resolves every `decision
   NNNN` against `docs/decisions/`, every backticked path against the
   tree, and every `#NNN` on a planned row.

3. **The classification is the operator's.** An entry is written
   `Status: proposed`; only the operator writes `Status: ruled (<date>)`,
   and only the operator writes `planned`, because that class names an
   issue only the operator opens. A ruled row is never edited into a
   different class: a later ruling is a dated erratum inside the file,
   as with a decision. The researcher, human or seat, proposes with the
   evidence it found.

   **Enforcement binding:** the status grammar and the `planned` rule
   are held by the test; who flips the status is judgment guidance and
   is said so here, as it is for decisions. The researcher charter
   forbids `ruled` and `planned` in its own text.

4. **The researcher is an office in the library, and its charter is
   the prompt.** `agents/researcher.json` and
   `agents/charters/researcher.md`, class `work`, one attempt. The
   charter reads the registry and the decision index before any
   article, writes proposed entries only, commits them on the branch
   the commission names and touches nothing else, and reads at most ten
   articles in one run. The cap is the operator's and the gate counts
   it.

   **Enforcement binding:** the roster test (0041) walks the office
   like any other; `recipes/research/roles/verify-seat.sh` refuses a
   run that added more than ten entries or changed a file outside
   `docs/research/`; the recipe's witness digest moves when the charter
   does.

5. **The fetch tools are an explicit grant, held by one office.** The
   claude adapter gains `webfetch` and `websearch` as nameable tools,
   Claude Code's `WebFetch` and `WebSearch`.
   The researcher is the only shipped office that holds them; no
   gate-class site may, and no office with secret bindings may. The
   route is the adapter's own class under decision 0036; this grant
   widens what the seat may read, not where its prompts go.

   On the dsh lane the same grant has a different shape, because dsh
   expresses no tool list: `recipes/research-dsh` seats the researcher
   inline on dsh, as night-shift seats its implementer, hired as
   `dashscope/qwen3.8-max` by the operator's ruling, with one overlay
   file beside it that turns the harness's page fetch on
   (`tool-web.fetch: true`; search is on by default) and states the
   Model Studio route, keyed from the environment. The overlay is in
   the bundle and therefore in its digest; the role file is the library
   charter's bytes, so the prompt stays one text. The operator ruled the
   weekly sweep runs on this lane.

   **Enforcement binding:** `crates/brokkr-runtime/tests/roster.rs`
   refuses any agent other than `researcher` that lists either tool,
   refuses either tool beside `class: gate` or a bindings block, admits
   a `--patch` on a dsh site only in `research-dsh`, and holds that
   recipe's role file byte-equal to the library charter.

6. **The recipe proposes and does not ship.** `recipes/research` is
   `research` → `verify` → `done`/`stop`, the protected phase `verify`,
   a boxed `exec` gate that runs the registry test and the cap. Nothing
   ships from the table; a research run produces a branch of
   proposals, and the entries reach `main` through an ordinary delivery
   run under decisions 0033 and 0038, after the operator has ruled the
   rows or left them proposed.

   **Enforcement binding:** the bundle compiles under the constitutional
   lint; the tree-wide compile test; the witness pin in
   `crates/brokkr-runtime/tests/witness_digests.rs`.

7. **Two intake surfaces, one issue type.** The operator's
   `research-intake` skill hands the researcher named articles and wraps
   the run; a scheduled workflow invokes the dsh lane of the same
   recipe with the weekly-sweep commission. Each intake that wrote entries ends in a
   GitHub issue of type `Research` naming the run id and the branch,
   with the proposals summarised, which is where the operator rules. The
   issue type was created in the organisation on 2026-09-04. When the
   standing operator of decision 0025 exists, it takes the schedule over
   from the workflow; until then the workflow is the trigger and the
   bundle is the stop condition and the budget.

   **Enforcement binding:** `.github/workflows/research-weekly.yml`;
   the skill; the issue type. The weekly workflow needs the Model
   Studio and DeepSeek credentials in the repository's secrets, the
   model's and the search's, and fails loudly without them rather than
   running unauthenticated.

## Consequences

The product now has a place to say, per finding, what it does about
what the field found, with a citation a reader can follow. The seven
articles read on 2026-09-04 are the first seven entries, all
`proposed`, forty-one rows between them, awaiting the operator's
ruling. Every future reading appends rather than rediscovers.

The researcher's charter is the configurable prompt the operator asked
for, and because it is a charter in the library its digest is part of
the recipe's identity: a softened prompt is a different recipe, loudly.
The ten-article cap and the docs-only write are counted by a script, not
promised by a model.

The claude adapter's tool vocabulary grows by two names, which moves
every witness digest in the tree once; the witness comment says why.

What this decision does not do: it does not calibrate any gate against
any outcome, does not build the structural audit, the failure-source
axis or the canary test the readings surfaced. Those are candidates in
the entries, for the operator to rule `planned` with an issue or leave.
