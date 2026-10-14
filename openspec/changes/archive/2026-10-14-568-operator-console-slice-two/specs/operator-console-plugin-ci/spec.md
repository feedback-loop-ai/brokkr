## ADDED Requirements

### Requirement: CI runs the plugin's tests

CI SHALL run `npm test` in `extensions/dsh/brokkr-console/` with the repository's pinned Node, on Linux and macOS, on every pull request and push to main.

#### Scenario: A failing plugin test

- **WHEN** a pull request breaks a plugin test
- **THEN** the `plugin tests` job fails on that pull request
