# Admit DSH session format version four, plain and compressed (#678)

## Why

The installed DSH core, 0.2.0-rc.2, writes each seat's session as
`session.v4.jsonl`, or `session.v4.jsonl.zstd` under its persistence
backend's default encoding, with a version-four header. Brokkr admits
only the version-zero and version-three names and headers, so every DSH
seat's transcript is unavailable in `brokkr transcript`, the TUI pane, the
browser and the DSH console that reads `brokkr transcript --json`. The
driver's tail asks for the same two names, so a working seat folds no
turn and its held transcript reference reaches the journal only when the
seat finishes.

## What changes

- Discovery admits `session.v4.jsonl` and `session.v4.jsonl.zstd`, in the
  closed order `session.v4.jsonl`, `session.v4.jsonl.zstd`,
  `session.v3.jsonl`, `session.jsonl` (design D6).
- A compressed candidate's header and body are decompressed by
  `brokkr-protocol`'s one zstd decoder, moved to a module of its own,
  within the plain source's bounds; a frame read whole is kept only on a
  matching content checksum (D6).
- The projector admits version four: version three's vocabulary and
  mapping, plus `developer/message` quiet, the tool-role `tool/result`
  message, and the retired wrapper counted (D3 to D5).
- The driver's tail asks for `session.v4.jsonl` first, so a working seat's
  first turn crosses decision 0053's boundary and flushes its transcript
  row while it works (D1, D7).
- Decision 0086, `Status: proposed`, amends 0055 ruling 2 and 0061 and
  supersedes 0062 ruling 6's set sentence; `docs/status.md` and
  `docs/guides/read-surfaces.md` name the version-four file.

## Not changed

No `Unavailable` reason, `brokkr.transcript/v1` field, contract, policy
table, reference schema or frozen fixture. No change to
`extensions/dsh/`, to the driver's pins or overlay, or to the warm
admission's census (decision 0056). Versions one and two stay refused,
and `session.v3.jsonl.zstd` and `session.jsonl.zstd` stay unread.

## Impact

`crates/brokkr-view/src/transcript.rs` and the new
`transcript/dsh_versions.rs`; `crates/brokkr-cli/src/local_transcript/`;
`crates/brokkr-protocol/src/zstd.rs`, `probe/observe.rs` and
`adapters.rs`; tests beside each, and synthetic fixtures under
`crates/brokkr-cli/tests/fixtures/dsh-v4/`. `brokkr-protocol`'s public API
grows by the `zstd` module (`unpack`, `Unpacked`, `Refused`).
