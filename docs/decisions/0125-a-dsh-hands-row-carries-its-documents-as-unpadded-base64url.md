# 0125 — A dsh hands row carries its reach and spec as unpadded base64url, and `brokkr hands serve` parses both spellings through one path

Status: proposed
Date: 2026-11-24

Built: built

## Context

Decision 0043 serves a boxed seat's workspace through `brokkr hands serve`,
decision 0065 seals the box's reach into `--reach`, and decision 0098 judges
every boxed `dsh` seat's composed tree before launch through the bounded line
reader (`crates/brokkr-protocol/src/adapters/dsh_hands/composition/rows.rs` and
`crates/brokkr-protocol/src/adapters/dsh_hands/composition/rows/carriers.rs`).
On 2026-11-24 the controller captured a real boxed `dsh` 0.2.2-rc.1 seat dump
(`docs/evidence/adapters/dsh-hands-1277/`): `dsh --dump-config` (js-yaml)
folds long JSON `--reach` and `--spec` values onto `>-` block-scalar content
lines starting with `{`, which the line-property closure refuses as
`Composition::Unreadable`.

Three alternatives were weighed:
1. Weakening the composed-tree line rule to allow `{`-led folded lines, which
   reopens the YAML flow-collection attack surface the line rule closes.
2. Writing the reach and spec to sidecar files and passing file paths in the
   overlay, which adds temporary-file lifecycle and path-handling surface to
   `hands serve`.
3. Encoding the exact JSON bytes of `--reach` and `--spec` as RFC 4648
   unpadded base64url in the `dsh` overlay only, while teaching
   `brokkr hands serve` to accept both the plain JSON spelling and the unpadded
   base64url spelling through one shared parse per document.

The controller chose the third on #1277, keeping the line rule, measured
release set, judges list, trust tier, adapter files, route overlay, and all
JSON-configured harnesses unchanged.

## Rulings

1. **The `dsh` overlay encodes `--reach` and `--spec` as RFC 4648 unpadded
   base64url.** For a boxed `dsh` seat only, the emitted `brokkr-hands` row
   carries `--reach-base64url` and `--spec-base64url` holding the exact JSON
   bytes of the sealed reach and hands spec encoded without `=` padding by the
   single codec in `crates/brokkr-protocol/src/base64.rs`. `--workdir` and
   `--artifacts` stay plain paths and fail closed if `dsh` folds either onto a
   line the line rule refuses. Harnesses whose MCP configuration is JSON
   (`claude`, `codex`, `agy`, `qoder`) keep plain `--reach` and `--spec`, and
   `composition/rows.rs` and `composition/rows/carriers.rs` do not change.
   Enforcement binding: `crates/brokkr-protocol/src/adapters/dsh_hands/tests/encoded_row.rs`
   and `crates/brokkr-protocol/src/base64/tests.rs`.

2. **`brokkr hands serve` accepts either spelling per document, enforces mutual
   exclusion, and keeps main's default and usage order.** For reach,
   `brokkr hands serve` requires exactly one of `--reach` or
   `--reach-base64url`: omitting both fails at clap with exit code 2 before
   `--spec` is parsed and renders main's usage order (`--reach <REACH>`
   before `--spec <SPEC>`), selected from structured clap parser state rather
   than rendered error text; passing both is refused with
   `DocumentError::BothSpellings`. For spec, passing both `--spec` and
   `--spec-base64url` is refused with `DocumentError::BothSpellings`, and
   omitting both uses `"workspace"`. Both encoded arguments set
   `allow_hyphen_values = true` so base64url values starting with `-` reach the
   typed decoder. `brokkr hands exec` keeps plain `--reach` and `--spec`
   unchanged. Enforcement binding: `crates/brokkr-cli/src/hands/tests.rs` and
   `crates/brokkr-cli/tests/hands.rs`.

3. **Both spellings of each document share one deserialization path and agree
   on every input.** After base64url decoding, reach text goes directly into
   `serde_json::from_str::<Sealed>` with no intermediate `serde_json::Value` or
   map, so duplicate fields refuse identically in both spellings and every
   document form `Sealed` accepts matches plain `--reach`. Spec text goes
   through `serde_json::from_str::<serde_json::Value>` and
   `HandsSpec::checked` in both spellings, matching main. Enforcement binding:
   `plain_and_encoded_spellings_agree_on_every_document` in
   `crates/brokkr-cli/src/hands/tests.rs`.

4. **Encoded refusals redact decoded input at rendering while plain-document
   refusals and missing-reach stay unchanged.** Refusals on `--reach-base64url`
   and `--spec-base64url` reduce errors to `Base64UrlError`,
   `serde_json::error::Category`, or payload-free `SpecRefusalKind`, retaining
   neither raw nor decoded text in `Display` or `Debug`. On `hands serve` and
   `hands exec`, the refusals made about a document given in the plain
   spelling (not JSON, not a valid sealed reach, or not a valid hands spec)
   preserve main's diagnostics byte for byte, and so does the missing-reach
   diagnostic; the usage synopsis inside any other clap error on `hands serve`
   (for example `--reach a --reach b`) is clap's for the present argument set
   and is not promised to match main's. Enforcement binding: redaction,
   plain-diagnostic, and duplicate-reach clap tests in
   `crates/brokkr-cli/src/hands/tests.rs` and `crates/brokkr-cli/tests/hands.rs`.

## Consequences

- Boxed `dsh` seats produce composed trees that read through the unchanged line
  rule and pass both judges. On 2026-11-24 (Linux, `dsh` 0.2.2-rc.1, recipe
  `ig-chief-dsh-qwen4-max-boxed-k4`), on round 4's change merged onto `main` at
  `4d51c363`, an unkeyed launch passed the reader and both judges and stopped
  at the missing credential (`eval-note-not-part-of-the-story--c8b73b47`), and
  with a provider key a boxed Qwen4-Max chief seat served its whole turn
  (`eval-note-not-part-of-the-story--be048762`: completed, boundary `namespace`,
  24 seat turns in 5m22s, 769,587 input tokens of which 713,216 cache reads,
  19,344 output; an earlier keyed run on round 2's head completed 21 turns in
  `eval-note-not-part-of-the-story--1ec99967`). On round 6's head (`fcddea103`),
  an engine built from it stopped at the same missing-credential point without
  a key (`eval-note-not-part-of-the-story--f2b8935d`) and composed the equal
  seat overlay (the same four flags in the same order, and the same encoded
  reach of 689 JSON bytes and spec of 168 JSON bytes). No keyed turn was run on
  a head after round 4's (`docs/evidence/adapters/dsh-hands-0-2-2.md`).
- The argument contract for `brokkr hands serve` and `brokkr hands exec` is
  documented in `docs/reference/hands.md`, linked from `docs/reference/cli.md`,
  and its synopses and option tables are held to `Cli::command()` by
  `crates/brokkr-cli/src/cli_reference_tests/hands_tests.rs`.
- Any `--workdir` or `--artifacts` path that `dsh` folds onto a line refused by
  the line rule remains refused, fail closed.
