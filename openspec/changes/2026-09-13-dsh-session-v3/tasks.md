# Tasks

## 1. The admitted names

- [x] 1.1 Amend the living `transcript-reading` requirement "Discovery identifies one owned local file" so DSH discovery names a closed, ordered set of session filenames — `session.v3.jsonl`, then `session.jsonl` — states that a session directory yields at most one candidate (the first admitted name carrying a valid header), and keeps a name outside the set unreadable. [Requirements: transcript-reading — "Discovery identifies one owned local file"]
- [x] 1.2 Make `dsh_walk` in `crates/brokkr-cli/src/ui.rs` try the admitted names in order, stopping at the first decisive outcome for a session directory so exactly one candidate is pushed, and record the same `io_seen` / `unsafe_seen` / `limit_hit` / `invalid_depth_seen` signals it records today. [Requirements: transcript-reading — "Discovery identifies one owned local file"]
- [x] 1.3 Prove it with host-independent tests: a `session.v3.jsonl` session is discovered and read; a directory holding both admits the versioned one and stays unambiguous; a name outside the set is still not read. [Requirements: transcript-reading — "Discovery identifies one owned local file"]
- [x] 1.4 Run the applicable gates on the result: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, the `brokkr-cli` suites and `openspec validate --all --strict`. [Requirements: transcript-reading — "Discovery identifies one owned local file"]
