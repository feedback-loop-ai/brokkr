# research — the product reads the field and proposes what to do about it

Two seats: `research` → `verify` → `done`/`stop`. No implement, no
review of code, no ship. The work seat is the library's `researcher`
office; the gate is a pinned exec script. What a run produces is a branch of
proposed entries under `docs/research/`, each finding classified from
the registry's closed vocabulary with a citation that the gate proves
resolves. The classifications are proposals: the operator reads them,
rules them, and delivers the branch through an ordinary delivery run
(decision 0044).

```
brokkr run --recipe research --repo . --feature "$(cat recipes/research/commissions/named.md) <urls>"
brokkr run --recipe research --repo . --feature "$(cat recipes/research/commissions/weekly-sweep.md)"
```

The first form hands the researcher the articles the operator chose.
The second asks it to sweep the week's work on the topics the
commission names. Either way the researcher reads at most ten articles
in one run, and the gate refuses an eleventh entry.

## What each seat does

- **research** (`agents/researcher.json`, `agents/charters/researcher.md`,
  class `work`): reads the registry README and index and the decision
  index first, then the articles, then writes one entry per article and
  one index row each, and commits them on the branch the commission
  names. It asks for `web-fetch` and `web-search` as `wants`
  (`capabilities`), and holds neither unless the realm grants it through
  a tool dialect in `realms.json` (decision 0065); otherwise the run
  proceeds without them, with a notice in the run manifest and in the
  seat's prompt. This repository's own `realms.json` grants nothing. The
  charter is the configurable prompt, and editing it moves this recipe's
  witness digest.
- **verify** (`roles/verify-seat.sh`, in Brokkr's box with no network
  where the realm's boundary is `namespace`; under `harness` it runs
  unboxed and no network denial is reported): the tree is
  clean, nothing outside `docs/research/` changed, at most ten entries
  were added, and `cargo test --test it research_registry::` passes: the
  index equals the entries, every class is in the vocabulary, every
  citation resolves. It runs none of `cargo fmt`, clippy or
  `scripts/lint-non-rust.sh` (#427): the branch may change only prose
  under `docs/research/`, which no Rust check reads, and the pull
  request's required `non-Rust lints` check still spells and links it.

## The dsh lane

`recipes/research-dsh` extends this recipe and seats the researcher
inline on dsh, the way night-shift seats its implementer: the same
charter bytes as a role file, `dashscope/qwen3.8-max`, and one
overlay (`drivers/research-web.yml`) that names the Model Studio route,
because dsh expresses no tool list. The weekly workflow runs that lane;
the operator's skill runs this one.

## What a run does not do

Its charter opens no issue and no pull request and writes no
`Status: ruled`. The seat runs unboxed with a tool list that removes no
tool, so that is its charter and your own settings, not a control. The operator's `research-intake` skill wraps the run:
it opens the Research-typed issue that carries the run id and the
proposals, and, once the operator has ruled the rows, delivers the
branch under decisions 0033 and 0038.
