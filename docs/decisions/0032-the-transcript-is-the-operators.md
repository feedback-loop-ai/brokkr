# 0032 — The transcript is the operator's: one law for every driver

Status: accepted (ruled 2026-09-02)
Date: 2026-09-02

## Context

Every built-in harness already left some account of a seat behind, but
the journal named each one differently. Claude kept JSONL below
`~/.claude/projects` and reported a session id. Codex kept its thread
below its own home and reported a thread id under the same old field.
DSH staged a retained root below `$DSH_HOME/sessions/brokkr` and put a
clamped absolute path in `harness-started.target`. Exec said nothing.
An operator therefore had to know which driver ran before asking the
journal where that seat's transcript was.

Run `per-turn-checkpoints-for-the-dsh-d7ba1e44` made the ownership issue
concrete. Its parked residual found that the DSH root was a temporary
directory dropped with the seat. On the retry the operator ruled
“retain”: the root moved under the operator's DSH home and the driver
stopped deleting it. That ruling established ownership for one driver,
but left the protocol shape and the other drivers uneven.

The operator generalized it in chat on 2026-09-02: “the same base
implementation for claude, codex and dsh (and whatever else) — we
always need the transcript.”

## Rulings

1. **Every driver reports one transcript shape.** Each invocation emits
   a checkpoint whose step is `transcript` and repeats the object in its
   finishing `session_meta`:

   ```json
   {"kind":"codex-thread",
    "locator":"019c…",
    "home":"/home/operator/.codex"}
   ```

   The kind vocabulary is closed to `claude-session`, `codex-thread`,
   `dsh-session`, and `none`. A harness without a transcript still
   reports `none` with empty locator and home. A harness which has not
   announced its id reports its real kind with an empty locator; absence
   is explicit and no location is invented.

   **Enforcement binding:** `brokkr-protocol::transcript` owns the shape,
   closed kinds, checkpoint emission, `session_meta` insertion, and the
   80-character locator clamp. Every built-in adapter arm calls it and
   supplies only the harness locator. Driver conformance covers every
   built-in in both reporting and silent-harness cases.

2. **Harness homes are facts, and paths are relative to them.** Claude's
   home is `~/.claude/projects`; Codex uses non-empty `$CODEX_HOME` or
   `~/.codex`; DSH uses non-empty `$DSH_HOME` or `~/.dsh`. Claude and
   Codex locators are the harness's session and thread ids. DSH keeps one
   root below `<dsh-home>/sessions/brokkr/<seat>/` and records its
   forward-slashed path relative to the separately recorded home. Exec
   reports `none`.

   **Enforcement binding:** home resolution and relative-path checking
   live beside the common shape. Conformance injects test-owned `HOME`,
   `CODEX_HOME`, and `DSH_HOME`; no shim test can name or write below the
   operator's real harness homes.

3. **The transcript is the operator's and is always retained.** A
   built-in driver never removes a harness transcript or a directory it
   stages for one. The journal carries only a path or id, never prompt
   text, reasoning, commands, tool arguments, tool results, or other
   transcript prose. Paths in, prose out.

   **Enforcement binding:** the DSH root releases its temporary handle
   with `keep`, just as in the accepted retry of the cited run. Claude
   and Codex remain in their harness-owned homes. The shared module never
   opens transcript content and has no removal operation. The bridge
   redacts the locator before an exported checkpoint leaves the journal.

4. **Every local read surface uses the common label.** `brokkr inspect`
   seat rows and the TUI participant detail render `transcript` for all
   drivers. Claude's valid locator additionally retains the local
   `claude --resume <id>` convenience; it is a harness-specific action
   below the common transcript fact, not the fact's schema.

   **Enforcement binding:** view version 5 derives one transcript cell
   from the journal. Both terminal surfaces render that cell. The TUI
   constructs a resume command only for `claude-session`; other kinds
   never borrow Claude behavior.

5. **Costs and usage do not change.** Locating and retaining a
   transcript does not reinterpret turns, tokens, cache reads, model
   evidence, or price.

## Protocol version

No frozen contract changes. `contracts/driver-protocol.v1.schema.json`
already defines checkpoint `data` as a driver-owned object without a
closed property set, and `session_meta` is the established additive
checkpoint convention rather than a control-plane message. The nested
optional `transcript` member is therefore v1-compatible: an older engine
or driver ignores a field it does not read, while a new reader can still
fold old flat `session_id` journals for resume compatibility. A v2 file
would falsely claim an incompatible wire change, so no new contract
version is added and the frozen v1 file is untouched.

## Consequences

The journal alone now tells the operator whether a transcript exists,
which harness owns it, and how to address it under that harness's home.
Retry uses the same locator irrespective of driver. New drivers join one
base law instead of inventing a checkpoint. Existing journals remain
readable; their missing common reference stays visibly absent, with the
old Claude/session-id fold retained only as compatibility.

Decision 0030's same-seat ownership and sandbox rules are unchanged.
Where that decision describes Codex's handle as a flat `session_id`,
this decision supersedes only that storage shape: resume now reads the
Codex `transcript.locator` and keeps the old field as a legacy fallback.
