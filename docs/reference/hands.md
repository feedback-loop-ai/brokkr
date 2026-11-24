# Hands argument reference

`brokkr hands` is an internal engine verb (`#[command(hide = true)]`), so the
generated [CLI reference](cli.md) omits it. This page records the argument
contract for `brokkr hands serve` and `brokkr hands exec` as defined in
`crates/brokkr-cli/src/hands.rs` (decision [0043](../decisions/0043-the-hands-are-one-tool.md),
decision [0065](../decisions/0065-the-box-seals-what-the-harness-hands-it.md),
and proposed decision [0125](../decisions/0125-a-dsh-hands-row-carries-its-documents-as-unpadded-base64url.md)).

## `brokkr hands serve`

Serve the one `workspace` tool over MCP (newline-delimited JSON-RPC on stdio);
every call runs `bash -lc <command>` inside the box.

```text
Usage: brokkr hands serve [OPTIONS] --workdir <WORKDIR> --artifacts <ARTIFACTS> --reach <REACH>
```

| Argument | Default | Required | Description |
| --- | --- | --- | --- |
| `--workdir <WORKDIR>` |  | yes | The worktree, bound read-write at its own path. |
| `--artifacts <ARTIFACTS>` |  | yes | The engine's artifact root, which the box never writes. |
| `--reach <REACH>` |  | one of `--reach` or `--reach-base64url` | The reach sealed at admission as JSON. |
| `--reach-base64url <REACH_BASE64URL>` |  | one of `--reach` or `--reach-base64url` | The reach sealed at admission as RFC 4648 unpadded base64url of its JSON bytes (`allow_hyphen_values = true`). |
| `--spec <SPEC>` | `"workspace"` | no | The box spec as JSON: `{"kind":"workspace","network":…,"binds":[…]}` or `"workspace"`. |
| `--spec-base64url <SPEC_BASE64URL>` |  | no | The box spec as RFC 4648 unpadded base64url of its JSON bytes (`allow_hyphen_values = true`). |

### Spelling, mutual exclusion, and defaults

- **Reach is required in exactly one spelling.** Passing neither `--reach` nor
  `--reach-base64url` fails at clap with exit code 2 before `--spec` is parsed,
  printing `--reach <REACH>` before `--spec <SPEC>` (selected from structured
  parser state). Passing both `--reach` and `--reach-base64url` is refused with
  `DocumentError::BothSpellings`.
- **Spec accepts at most one spelling and defaults to `"workspace"`.** Omitting
  both `--spec` and `--spec-base64url` resolves to `"workspace"` (the default
  box spec), parsed through the same path as an explicit `--spec`. Passing both
  `--spec` and `--spec-base64url` is refused with `DocumentError::BothSpellings`.
- **Leading hyphens in encoded values.** Both `--reach-base64url` and
  `--spec-base64url` set `allow_hyphen_values = true` so base64url strings
  opening with `-` (alphabet index 62) reach the typed decoder instead of being
  misread as flags.
- **One deserialization per document.** After base64url decoding, each document
  goes through the exact same parse as its plain JSON spelling:
  - reach: `serde_json::from_str::<Sealed>` directly (no intermediate
    `serde_json::Value` or map), so duplicate fields refuse identically in both
    spellings and every valid JSON form `Sealed` accepts (including serde's
    positional array form) is admitted identically;
  - spec: `serde_json::from_str::<serde_json::Value>` followed by
    `HandsSpec::checked`, matching the plain spelling on every input.
- **Refusal diagnostics.** Plain `--reach` and `--spec` preserve main's
  diagnostics byte for byte. Refusals on `--reach-base64url` and
  `--spec-base64url` reduce errors at rendering to `Base64UrlError`,
  `serde_json::error::Category`, or payload-free `SpecRefusalKind`, retaining
  neither the raw argument value nor any decoded text in `Display` or `Debug`.
- **Harness carriage.** Only the `dsh` overlay (`crates/brokkr-protocol/src/adapters/dsh_hands.rs`
  and `crates/brokkr-protocol/src/adapters/mcp.rs`) emits `--reach-base64url`
  and `--spec-base64url`, leaving `--workdir` and `--artifacts` plain. Harnesses
  that carry MCP configuration in JSON (`claude`, `codex`, `agy`, `qoder`) emit
  plain `--reach` and `--spec`.

## `brokkr hands exec`

Run one command whole inside the box with stdio passed through. Exits with the
command's own exit code (see [Exit codes](cli.md#exit-codes)).

```text
Usage: brokkr hands exec [OPTIONS] --workdir <WORKDIR> --artifacts <ARTIFACTS> --reach <REACH> <COMMAND>...
```

| Argument | Default | Required | Description |
| --- | --- | --- | --- |
| `--workdir <WORKDIR>` |  | yes | The worktree, bound read-write at its own path. |
| `--artifacts <ARTIFACTS>` |  | yes | The engine's artifact root. |
| `--reach <REACH>` |  | yes | The reach sealed at admission as JSON. |
| `--bundle-root <BUNDLE_ROOT>` |  | no | Strategy root, bound read-only at `/runtime/bundle`. |
| `--spec <SPEC>` | `"workspace"` | no | The box spec as JSON. |
| `<COMMAND>...` |  | yes | The command and its arguments, run inside the box. |

`brokkr hands exec` takes plain `--reach` and `--spec` only and keeps main's
diagnostics byte for byte.
