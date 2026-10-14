# DSH operator console, slice two — tasks

Status: proposed. Fired by the operator on 2026-10-14. Each task closes on observed evidence.

## 0. Evidence

- [x] 0.1 Measure on DSH `0.2.0-rc.2` the model tools, the prefilled session, the tool-view card, and the Files tab and file opening a third-party plugin can use. Observed 2026-10-14 against a loopback stub model, in `evidence/dsh-0.2.0-rc.2/`.

## 1. Brokkr side

- [x] 1.1 `brokkr-view`: `RunView.recipe` from the `run/started` manifest's `bundle_name`; view wire 20; tests. `crates/brokkr-view/src/clarify.rs` `recipe()`, `RunView.recipe`, view 20; `clarify/tests.rs`.
- [x] 1.2 `brokkr-view`: `ModelShare.model` and `RuleCount.rule_id` as `Option<String>`; tests. `analytics.rs`, `analytics/figures.rs`; `analytics/tests.rs` `a_missing_rule_and_a_missing_model_are_absent_not_words`.
- [x] 1.3 The public-API snapshots, measured. `quality/public-api/brokkr-view.txt` gains `RunView::recipe`, measured with cargo-public-api 0.52.0.

## 2. Plugin

- [x] 2.1 Host: the four read-only tools and `brokkr_suggest` (design D2, D3), with argument checks and bounds; tests. `lib/host/tools.js`; `tests/tools.test.js`.
- [x] 2.2 Client: the `brokkr_suggest` card and Use this; the decision card showing a suggestion; tests. `lib/client.js` `SuggestionCard`, the decision card; `tests/assistant.test.js`.
- [x] 2.3 Client: Ask Brokkr and Ask about this run open a prefilled session in the right workspace (D4); tests. `lib/client.js` `Ask`, `openSession`; `tests/assistant.test.js`.
- [x] 2.4 Client: Open the spec (D5); tests. `lib/client.js` `Clarify`; `tests/assistant.test.js`.
- [x] 2.5 Resume with the view's recipe, settings as overrides (D1); view version 20; tests. `lib/host/brokkr.js` `resumeSource`, `commands.js`; `tests/commands.test.js`.
- [x] 2.6 Debt: an unprepared state folder answers unstarted; the WebSocket refuses unmasked client frames and stops reading once closed; tests. `lib/host/runner.js` `privateDir`, `lib/host/socket.js`; `tests/runner.test.js`, `tests/socket.test.js`.

## 3. CI and docs

- [x] 3.1 The `plugin tests` job on Linux and macOS (D6). `.github/workflows/ci.yml` `plugin-tests`.
- [x] 3.2 The guide, the provenance note and the status row; slice one's built tasks ticked. `docs/guides/dsh-console.md`, `extensions/dsh/PROVENANCE.md`, `docs/status.md`; slice one archived.
- [x] 3.3 The console loaded into an isolated DSH against a real journal, read-only, with a stub model: tools, cards, both panels and Open the spec observed. Observed 2026-10-14 in `evidence/dsh-0.2.0-rc.2/console/`: the five tools against the real journal, the refusal answer, both Ask flows, the fence; the journal byte-identical. Open the spec and an admitted suggestion card had no parked run to show them; the probe observed both, and the plugin tests cover them.
