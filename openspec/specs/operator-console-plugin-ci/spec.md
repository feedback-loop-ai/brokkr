# operator-console-plugin-ci Specification

## Purpose
Hold the DSH operator console plugin's own tests as a CI job on both
supported hosts, so a change to the plugin is proven the way the Rust
workspace is.

## Requirements

### Requirement: CI runs the plugin's tests

CI SHALL run `npm test` in `extensions/dsh/brokkr-console/` with the repository's pinned Node, on Linux and macOS, on every pull request and push to main.

#### Scenario: A failing plugin test

- **WHEN** a pull request breaks a plugin test
- **THEN** the `plugin tests` job fails on that pull request

## Provenance

- `2026-10-14-568-operator-console-slice-two` — folded 2026-10-14
