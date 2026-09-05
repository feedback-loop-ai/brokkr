# 0034 — The seat record is a contract

Status: accepted (operator ruled in chat, 2026-09-02)
Date: 2026-09-02

## Context

The built-in drivers already journaled a useful but uneven account of each
seat. Claude recorded turns, tools, targets, cost, and its session id but
dropped the usage present in every assistant stream message. Codex recorded
input, cache-read, and output tokens but no price. DSH recorded the usage its
harness supplied. Exec had no model usage. Decisions 0031 and 0032 then added
the served model and a common transcript locator, but the complete record was
still convention rather than a contract checked before it became evidence.

The operator reviewed those journals in chat on 2026-09-02 and ruled: “that's
a contract, basically, and a good contract at that.” This decision records
that ruling and makes the existing useful shape enforceable without rewriting
historical journals.

## Rulings

1. **A seat record has one frozen, closed vocabulary.**
   `contracts/seat-record.v1.schema.json` defines per-turn checkpoints,
   finishing checkpoints, and successful results. The common fields are turn,
   served model, input and output tokens, cache reads, cache writes, harness
   cost, session id, transcript locator, and the tool and file target of a tool
   turn. Driver lifecycle fields needed to distinguish those records remain in
   the same closed schema.

   `input_tokens` includes cache reads. `cache_read_tokens` is the separately
   visible subset and is never added again. A harness cache creation count is
   `cache_write_tokens`. Numeric measurements are strictly positive when
   present and absent when unreported; zero and string sentinels are not
   measurements. The two model sentinels remain exactly `not reported` and
   `not applicable` under decision 0031.

   **Enforcement binding:** the frozen JSON Schema is embedded by
   `brokkr-store`. Export validates every checkpoint and successful result,
   and `verify-run` applies the same validator after checking the hash chain.
   The frozen-contract test pins the published and embedded bytes.

2. **The record contains no prose.** A seat record is accounting and
   provenance, not a transcript. It admits no prompt, response, reasoning,
   command, arguments, tool output, diff, or rationale. Tool is a bounded
   identifier and target is a bounded file path only. The phase machine's
   separately governed `result`, `inputs`, and `notes` report remains inside a
   successful result; those fields do not widen the adjacent seat-record
   vocabulary.

   **Enforcement binding:** the schema is closed with
   `additionalProperties: false`, constrains identifiers and sizes, and the
   validator's diagnostic reports a sequence and schema path without echoing
   the rejected private value.

3. **Drivers report what their harnesses report, and no invented zero.**
   Claude normalizes its raw input plus cache-read count to inclusive input,
   maps `cache_creation_input_tokens` to `cache_write_tokens`, and records all
   four counts per assistant message and at completion. LaneTally inherits the
   Claude fold. Codex and DSH retain their inclusive-input normalization and
   report cache reads when supplied. Exec reports model `not applicable` and
   no usage.

   **Enforcement binding:** the built-in folds omit zero or absent
   measurements. Driver conformance validates every emitted checkpoint and
   successful result against the seat-record schema and asserts each harness's
   actual field set.

4. **One accounting rule serves every readout.** `brokkr costs`, the seat
   surfaces, and `brokkr inspect` show the reported token fields, including
   cache writes. Session totals prefer per-turn measurements and use the
   finishing record only as a fallback, so a final repetition is not counted
   twice. Displayed total tokens are exactly input plus output.

   **Enforcement binding:** `brokkr-view` version 6 derives one structured
   usage record and rendered cell for terminal, TUI, and web consumers;
   `brokkr costs` uses the same field meanings and has per-driver aggregation
   tests.

5. **Old journals remain readable.** A pre-0031 or pre-0032 journal has no
   record-version marker that can distinguish it from a new row. The schema
   therefore permits model and transcript to be absent, while current
   built-in conformance requires them. Missing historical evidence stays
   visibly missing; it is never backfilled from configuration.

6. **The journal refuses a nonconforming record at append** (added
   2026-09-03, operator ruled). Ruling 1 bound the contract to export and
   verification, and the first run to test it showed why that is too late:
   a work seat on the Spark lane wrote a result carrying a `commits` field
   the contract does not admit, the append took it, and the run concluded
   normally — then `export` refused the run at that row, and so did
   `anchor`, and the journal being append-only, nothing could ever put it
   right. A check that fires only when evidence is wanted cannot refuse the
   write; it can only discover, later, that the run will never be evidence.

   The fence therefore sits where the row is sealed. A checkpoint or a
   successful result that violates the contract is refused at the seq it
   would have taken, and nothing is written. The attempt that produced it
   does not vanish and is not repaired: the engine journals the refusal as
   that attempt's failure, in the same words the store used, so the run
   ends the way a seat that met no other contract ends — a determinate
   failure, retried and then parked under decision 0006. The raw evidence
   stays where the seat left it, in its result file and its transcript
   (decision 0032); the journal records that it was refused and the schema
   path, never the value.

   Export and verification keep their sweeps. They are the only defence a
   journal written before this fence has, and a foreign export carries no
   promise about the engine that wrote it.

   **Enforcement binding:** `brokkr-store` validates every checkpoint and
   successful result inside the append transaction, before the envelope is
   sealed, through the same embedded schema and the same
   `validate_seat_record` that export and `verify-run` use; a refusal rolls
   the transaction back. `brokkr-runtime` turns a refused result into
   `effect/failed` and a refused live checkpoint into the attempt's failure
   once its driver exits, for single seats, panel members and sequence
   steps alike. Tests cover the refusal writing nothing, and each engine
   site's failed ending.

7. **The dialect step's `state` is admitted to the typed report, as v3**
   (added 2026-09-05, operator ruled). Ruling 6's fence, ported onto the
   engine that had meanwhile gained decision 0042's dialect steps, refused
   the first record it met: the exec driver writes `state` — the output of
   the dialect's own state command — onto a dialect validator's successful
   result, and the contract's `result` definition is `additionalProperties:
   false` and never admitted the name. Every run reaching a dialect step
   was therefore writing a journal that export, `anchor` and offline verify
   would refuse, permanently, exactly the trap ruling 6 exists to close.
   `recipes/triage` has five such steps; no journal in `docs/evidence` had
   yet been written through one.

   The field is admitted rather than removed, because it is not an
   accounting field: it belongs to the separately governed typed report —
   the family of `result`, `inputs` and `notes` — and the same command's
   stdout and stderr already ride the record as `notes`, which this contract
   admits and constrains to nothing. `state` is added on the same terms:
   the contract admits the name and governs none of the content. The
   accounting vocabulary is untouched and remains what ruling 1 froze, an
   accounting record and never a transcript.

   **Enforcement binding:** `contracts/seat-record.v3.schema.json`, a new
   file beside v2 whose bytes do not move and stay pinned, adding `state` to
   the successful result alone. Its engine boundary is v2's own 0.8.0 line,
   because `engine` is the crate version and carries no position within a
   line: v2 and v3 both landed after the 0.8.0 tag and cannot be told apart
   by it. Within a line the newest contract wins, which refuses nothing a v2
   record could have carried, since each version adds optional properties
   and takes none away. Naming the unreleased line instead would judge every
   record this engine writes under v2 and refuse the `state` it is already
   writing.


## Consequences

A malformed seat checkpoint or result never enters the journal: the append
refuses it and the attempt fails. A canonical export still cannot carry one
out of a journal written before the fence, and verification still refuses
one before folding it. Claude usage that was already available finally
appears beside Codex and DSH usage. Operators see cache creation without
confusing it with input or adding cache reads twice.
The driver wire protocol and event envelope remain v1: their existing object
extension points carry the new, independently versioned seat-record contract.

Decision 0031 still governs the meaning and sentinels of `model`. Decision
0032 still governs transcript ownership, retention, kinds, and locators; this
decision incorporates that shape rather than superseding it.
