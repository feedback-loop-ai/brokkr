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

4. **Encoded refusals redact decoded input at rendering while plain diagnostics
   stay unchanged.** Refusals on `--reach-base64url` and `--spec-base64url`
   reduce errors to `Base64UrlError`, `serde_json::error::Category`, or
   payload-free `SpecRefusalKind`, retaining neither raw nor decoded text in
   `Display` or `Debug`. Plain `--reach` and `--spec` on `hands serve` and
   `hands exec` preserve main's diagnostics byte for byte. Enforcement
   binding: redaction and plain-diagnostic tests in
   `crates/brokkr-cli/src/hands/tests.rs` and `crates/brokkr-cli/tests/hands.rs`.

## Consequences

- Boxed `dsh` seats produce composed trees that read through the unchanged line
  rule and pass both judges; the controller observed an unkeyed launch on round
  2's head pass both judges and start `dsh` up to the missing credential
  check (`docs/evidence/adapters/dsh-hands-0-2-2.md`), while serving a keyed
  turn remains pending.
- The argument contract for `brokkr hands serve` and `brokkr hands exec` is
  documented in `docs/reference/hands.md` and linked from
  `docs/reference/cli.md`.
- Any `--workdir` or `--artifacts` path that `dsh` folds onto a line refused by
  the line rule remains refused, fail closed.
