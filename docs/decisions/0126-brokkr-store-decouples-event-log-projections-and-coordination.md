# 0126 — brokkr-store decouples event log, projections, and coordination

Status: accepted — operator ruled 2026-11-25 ("go with recommended on all questions")
Date: 2026-11-25

Built: partial (#1362) — Slice 1 (ports, backend-agnostic Journal facade, and co-located SqlitePorts adapter over Store) built; Slice 2 (local fs-log + trigger-free disposable index.db + isolated queue.db) and Slice 3 (brokkr-store-pg) follow.

## Context

`brokkr-store` currently combines three distinct storage concerns in one SQLite
database file (`.forge/forge.db`):

1. The append-only, hash-chained run event log (`runs` and `events`).
2. Derived read-model projections (`run_summary`, `seat_summary`,
   `event_index`), which are rebuildable from the event log (decision 0089).
3. Mutable operational coordination state (`queue_*`, `provider_cooldowns`,
   dispatch holder leases; decisions 0068 and 0103).

Because every caller talks directly to the concrete SQLite `Store`, domain
invariants (hash-chain continuity, optimistic `expected_head` fencing,
`seat_record` schema validation, canonical NDJSON export/import) are coupled
to SQLite transaction code. Supporting a local split storage layout (an
append-only file log per run with a disposable SQLite index and an isolated
queue database) or a future PostgreSQL service backend would otherwise require
duplicating those domain rules or modifying `brokkr-core` and `brokkr-runtime`.

Under decision 0071 ruling 2, a storage backend seam spans several operations
that must stay consistent across a persistence boundary, so narrow traits at
the storage edge are warranted.

## Rulings

1. **R1 — Three narrow synchronous port traits, no async runtime.**
   `brokkr-store` defines three synchronous storage port traits in
   `brokkr_store::ports`:
   - `EventLog`: durable per-run manifests and append-only event streams
     (`create_run`, `append_envelopes`, `head`, `read_stream`, `manifest`,
     `list_run_ids`).
   - `ProjectionStore`: derived read-model updates, fleet run listings, and
     full rebuild from any `EventLog` (`apply_envelopes`, `list_runs`,
     `rebuild_from`).
   - `CoordinationStore`: queue enqueue/listing and active provider cooldown
     queries (`enqueue`, `list_queue`, `active_cooldowns`).

   All port operations remain synchronous. `brokkr-store` takes no `tokio` or
   other async runtime dependency.

2. **R2 — One backend-agnostic `Journal<E, P, C>` domain facade.**
   `brokkr_store::ports::Journal<E, P, C>` is the single home of domain rules
   across backends: run-id admission, hash-chain continuity, optimistic
   `expected_head` fencing (`StoreError::HeadMoved`), `seat_record` schema
   validation, redaction, and canonical NDJSON export/import. Storage backends
   implement only persistence and optimistic compare-and-append; they never
   duplicate domain rules.

3. **R3 — Cooldowns decouple via idempotent projection catch-up.**
   `append_with_cooldowns` decouples across the `EventLog` /
   `CoordinationStore` boundary by deriving cooldowns idempotently from
   committed events (projection catch-up) rather than requiring two-phase
   commit (2PC) across separate storage engines.

4. **R4 — Local split backend (Slice 2).**
   Slice 2 introduces a local split implementation of the three ports:
   - Per-run append-only `FsEventLog` storing
     `.forge/runs/<run-id>/manifest.json` and `events.ndjson`, using
     `O_APPEND`, `sync_data()`, and Rust 1.89's `std::fs::File::lock()`.
   - Dedicated trigger-free disposable `SqliteProjectionStore` (`index.db`),
     rebuildable from any `EventLog` via `brokkr journal reindex` (replacing
     Slice 1's co-located `SqlitePorts` projection role when the event log
     lives outside SQLite).
   - Isolated `SqliteCoordinationStore` (`queue.db`).

5. **R5 — Future service backend (Slice 3).**
   Slice 3 introduces a PostgreSQL implementation (`brokkr-store-pg`) of the
   same three traits: `runs` and `events` with `PRIMARY KEY (run_id, seq)` and
   `JSONB` payloads, projection tables, and `queue` coordination using
   `SELECT ... FOR UPDATE SKIP LOCKED`, requiring zero changes to
   `brokkr-core` or `brokkr-runtime`.

## Delivery slices

- **Slice 1 (#1362):** Define `EventLog`, `ProjectionStore`,
  `CoordinationStore`, and `Journal<E, P, C>` in `brokkr-store::ports`.
  `Journal<E, P, C>` proves full storage decoupling against in-memory port
  backends, while `SqlitePorts<'a>(&'a Store)` is strictly the co-located
  adapter over the existing SQLite `Store` — zero breaking changes to existing
  callers, zero extra schema tables, and zero new crate dependencies.
- **Slice 2:** Implement the local split backend (`FsEventLog`, dedicated
  trigger-free `SqliteProjectionStore` on `index.db`, and isolated
  `SqliteCoordinationStore` on `queue.db`) and `brokkr journal reindex`.
- **Slice 3:** Implement `brokkr-store-pg` behind the same three traits.
