# 0018 — Dual license: MIT OR Apache-2.0

Status: accepted (operator ruling in chat, 2026-08-30)

## Context

The project had no license, which in practice means "all rights
reserved" — the least open state there is. The operator ruled for the
most open licensing that does not impose openness on others: permissive,
never copyleft.

## Decision

**MIT OR Apache-2.0**, the Rust ecosystem's dual convention (serde,
tokio, clap, ratatui, rustc). A user picks either license. This gives:

- MIT's frictionlessness — one paragraph everyone understands;
- Apache-2.0's **explicit patent grant** from every contributor, its
  patent-retaliation clause, and its trademark carve-out protecting
  the project's name;
- zero-cost adoption for contributors, since every Rust developer
  already knows exactly what this pair means.

Public-domain-style licenses (Unlicense, 0BSD, CC0) were considered
and rejected: no patent story, shaky in some jurisdictions, and no
attribution — openness that cannot be relied on is not more open.

## Consequences

- `LICENSE-MIT` and `LICENSE-APACHE` at the root; `license = "MIT OR
  Apache-2.0"` in the workspace package metadata.
- The standard contribution clause: unless stated otherwise, any
  contribution intentionally submitted for inclusion is dual-licensed
  as above, without additional terms.
- The dependency tree already complies: everything in Cargo.lock is
  permissively licensed, verified by the RustSec/audit job.
