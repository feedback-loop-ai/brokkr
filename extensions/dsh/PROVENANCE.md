# Provenance — DSH extension boundary

This note records the repository-owned bytes that live under `extensions/dsh/`.
It is outside every hashed runtime set: no Brokkr runtime code reads, builds,
loads or executes it — only the composite reader's own tests open the directory,
to prove the recorded digests reproduce — and the runtime composite (design D6)
reads installed provider files only, never this note. Decision 0009's extension boundary is why the
bytes live here rather than under `crates/` or `adapters/`.

## `plugin-cli-session/` — adapted `dsh-plugin-cli-session` 0.2.0

Upstream repository: <https://github.com/ghbhiee/dsh-plugin-cli-session>
Upstream commit: `0f487e74c81ed102c6899440d9f5d65e8e9eabda`
Published package: `dsh-plugin-cli-session` 0.2.0
Source counterpart: `src/index.ts:252`
Licence: MIT (the set's own `LICENSE` is the upstream licence, byte-identical)

The committed set is the plugin's six published files and nothing else. Five
are byte-identical to upstream; `lib/index.js` changes exactly one expression
on line 253:

```diff
-	const events = agent.session.events;
+	const events = agent.session.snapshotEvents(firstSeq);
```

`@deepseek-ai/dsh` 0.1.5-rc.1 removed the public `session.events` accessor; the
documented replacement `snapshotEvents(fromSeq?)` returns the half-open interval
from `firstSeq` to the current end, which is exactly what the plugin's
`summarize`, `collectUsage` and `countTurns` folds already select by
`seq >= firstSeq`. Nothing is rebuilt and no source, lockfile, test, build
configuration or CI file is vendored.

Delta digest (SHA-256 of the canonical text `lib/index.js:253`, the upstream
line prefixed `-`, and the adapted line prefixed `+`, each newline-terminated):

```
78256d2e114f7ae8caec22987c5793b7398018cd59cd24cf36e79d7be011a585
```

Per-file SHA-256, upstream published bytes (`sha256sum` format):

```
7a9d5a3b08b3802d77eb237283c26ce6ae346af58b86b1b0d655dada45416915  LICENSE
c92c60d057e456beea6c0cd27bad26bf60f9a8909a852c96b36eceaf802cfc1d  README.md
84745a1bb00d773acf2e5ab5e32dc42825ffe164100ba469375dcabbbd5f9dab  cordis.patch.yml
a40b52b3891485821ad01b00c322006abee8a51a0d4a2ae4ddb8427a0183d99b  lib/index.js
3526be1cd885f99592f1cfb5133f065879411f13bffba2672a74a55a11e66491  lib/startup.js
7e96b1467153b4aed91e65e52ab03ebecf901f52a1f437484da14382e8a4a0fb  package.json
```

Per-file SHA-256, committed adapted bytes (`sha256sum` format):

```
7a9d5a3b08b3802d77eb237283c26ce6ae346af58b86b1b0d655dada45416915  LICENSE
c92c60d057e456beea6c0cd27bad26bf60f9a8909a852c96b36eceaf802cfc1d  README.md
84745a1bb00d773acf2e5ab5e32dc42825ffe164100ba469375dcabbbd5f9dab  cordis.patch.yml
325eccc0d67de1dcea79a3c2d89eebb139b10970d0b7efd6dd1b7e3e6949fe85  lib/index.js
3526be1cd885f99592f1cfb5133f065879411f13bffba2672a74a55a11e66491  lib/startup.js
7e96b1467153b4aed91e65e52ab03ebecf901f52a1f437484da14382e8a4a0fb  package.json
```

Substituting the upstream expression back into the adapted `lib/index.js`
reproduces the upstream `lib/index.js` SHA-256
`a40b52b3891485821ad01b00c322006abee8a51a0d4a2ae4ddb8427a0183d99b`, which is
how the committed-bytes test proves the delta is that one expression.

This note records no plugin component or composite value: the single Rust
function design D6 places beside the DSH planner is their only producer.

## `resume-policy/` — conditional `brokkr-dsh-resume-policy`

Not authored. A thin Cordis extension is permitted only if 10.7 demonstrates a
missing policy or pre-work observation hook that the adapted plugin's documented
`setup` interface cannot supply. Until then this directory does not exist, is
not composed into any profile, and contributes no line to the composite. When
it is needed, its repository home is `extensions/dsh/resume-policy/` with
exactly four committed runtime files (`package.json`, `index.js`,
`cordis.patch.yml`, `LICENSE`), and this section records its authorship, the
dated missing-fact probe, the documented module/hook and core version, its
limited behavior, licence and per-file digests, again with no hand-computed
component or composite.
