# Hands argument reference

`brokkr hands` is an internal engine verb (`#[command(hide = true)]`), so the
generated [CLI reference](cli.md) omits it. This page records the argument
contract for `brokkr hands serve` and `brokkr hands exec` as defined in
`crates/brokkr-cli/src/hands.rs` (decision [0043](../decisions/0043-the-hands-are-one-tool.md),
decision [0065](../decisions/0065-the-box-seals-what-the-harness-hands-it.md),
and proposed decision [0125](../decisions/0125-a-dsh-hands-row-carries-its-documents-as-unpadded-base64url.md)).
The synopsis and option table of each subcommand are checked against
`Cli::command()` by `crates/brokkr-cli/src/cli_reference_tests/hands.rs`.

## `brokkr hands serve`

Serve the one `workspace` tool over MCP (newline-delimited JSON-RPC on stdio);
every call runs `bash -lc <command>` inside the box.

```text
Usage: brokkr hands serve [OPTIONS] --workdir <WORKDIR> --artifacts <ARTIFACTS>
```

| Argument | Default | Description |
| --- | --- | --- |
| `--workdir <WORKDIR>` |  | The worktree, bound read-write at its own path |
| `--artifacts <ARTIFACTS>` |  | The engine's artifact root, which the box never writes (U8a) |
| `--reach <REACH>` |  | The reach sealed at admission as JSON, which every box is built from (U8a2c). Pass either `--reach` or `--reach-base64url` |
| `--reach-base64url <REACH_BASE64URL>` |  | The reach sealed at admission as unpadded base64url of its JSON bytes. Pass either `--reach` or `--reach-base64url` |
| `--spec <SPEC>` | `"workspace"` | The box spec as JSON: {"kind":"workspace","network":…,"binds":[…]}. Pass either `--spec` or `--spec-base64url`; omitting both uses `"workspace"` |
| `--spec-base64url <SPEC_BASE64URL>` |  | The box spec as unpadded base64url of its JSON bytes. Pass either `--spec` or `--spec-base64url` |

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
- **Refusal diagnostics.** On `hands serve` and `hands exec`, the refusals made
  about a document given in the plain spelling (`--reach` or `--spec`: not
  JSON, not a valid sealed reach, or not a valid hands spec) preserve main's
  diagnostics byte for byte, and so does the missing-reach diagnostic. The
  usage synopsis inside any other clap error on `hands serve` (for example
  `--reach a --reach b`) is clap's for the present argument set and is not
  promised to match main's. Refusals on `--reach-base64url` and
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

| Argument | Default | Description |
| --- | --- | --- |
| `--workdir <WORKDIR>` |  | The worktree, bound read-write at its own path |
| `--artifacts <ARTIFACTS>` |  | The engine's artifact root, as `serve` takes it |
| `--reach <REACH>` |  | The reach sealed at admission, as `serve` takes it |
| `--bundle-root <BUNDLE_ROOT>` |  | Strategy root, bound read-only at /runtime/bundle |
| `--spec <SPEC>` | `"workspace"` | The box spec as JSON, as `serve` takes it |
| `<COMMAND>...` |  | The command and its arguments, run inside the box |

`brokkr hands exec` takes plain `--reach` and `--spec` only; its plain-document
refusals (not JSON, not a valid sealed reach, or not a valid hands spec) and
its missing-reach diagnostic keep main's diagnostics byte for byte.
