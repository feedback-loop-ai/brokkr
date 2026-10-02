## Purpose

Retain citable masked MCP responses by content digest and fold every broker
call through the engine without losing or inventing evidence.

## ADDED Requirements

### Requirement: CR1 the dialect opts in and the realm may only veto

Effective retention SHALL be true exactly when the MCP dialect declares
retained true and the realm grant does not carry retain false. Omitted
retained is false. The realm SHALL not force retention, and a recipe, seat,
model or child server SHALL not alter it. Provider-native retention remains
unsupported because this slice does not capture native response bodies.

The declaration, veto and effective result SHALL be pinned in run-manifest
v12. Changing either declaration SHALL change identity even when a veto leaves
the effective result unchanged. A call with effective retention false SHALL
write metadata only; no response body SHALL enter the ledger, artifact store
or journal. Necessary in-memory bounded forwarding is not retention.

#### Scenario: The four retention outcomes are explicit

- **WHEN** dialect retained is absent/false with no veto, absent/false with retain false, true with no veto, or true with retain false
- **THEN** effective retention is respectively false, false, true and false
- **AND** the response digest is present only for a terminal response in the third case, with all declarations still pinned

#### Scenario: A realm cannot upgrade the dialect

- **WHEN** a grant in the next realms version after v7 writes retain true, null, an object or a string
- **THEN** loading refuses "realm '<realm>' capability '<capability>': retain must be false when present; the realm may veto retention, never require it"
- **AND** a model asking to keep a response under retain false creates no artifact or digest

### Requirement: CR2 retained bytes are the masked response prepared for delivery

For each complete retained child response, the broker SHALL decode the
MCP response at the edge, mask bound secret values and common encodings in
all textual data, and serialize one canonical UTF-8 JSON object containing
exactly one of result or error. JSON-RPC version/id and transport framing
SHALL be excluded; result/error data, arrays, structured content and isError
SHALL be preserved. Object keys SHALL be deterministically ordered, array
order preserved, with no trailing newline. Duplicate JSON keys and non-JSON
or oversized payloads SHALL be rejected, not repaired. The same masked object
SHALL be delivered to the harness with the original correlation envelope.
Retention SHALL cite these prepared data bytes, not raw secret-bearing
wire bytes or a later interpretation. On a completed delivery they SHALL be
identical to what was forwarded. A durable terminal record proves the child's
outcome and the prepared response, not harness receipt or model consumption;
there is no atomic commit across the ledger and harness stdin.

The broker SHALL durably stage those bytes outside the seat's writable reach.
The engine SHALL verify the bound ledger reference, digest and bytes, then
publish the artifact atomically at .forge/artifacts/sha256/<hex>, where hex
is the lowercase SHA-256 of exactly those bytes. Existing identical content
SHALL be reused only after digest verification; conflicting existing bytes,
symlinks, nonregular files and traversal SHALL refuse. Durable publication
SHALL precede the journal checkpoint that references it. The engine SHALL
not overwrite an existing digest path with different content.

The seat SHALL not be able to forge the ledger/plan, substitute staging
bytes, or mutate published evidence through workspace hands, overlapping
binds, aliases or native tools. Design SHALL name the protected host storage
and namespace mounts; permissions on a file writable through the worktree
are not sufficient. Read/write verification belongs above pure core/view.

#### Scenario: The digest names the delivered masked data

- **WHEN** a retained response contains a bound secret and nested structured result data
- **THEN** the artifact is the canonical masked result object, its SHA-256 equals response_sha256 on the terminal checkpoint, and the delivered response has identical masked result data
- **AND** different JSON-RPC ids do not change this digest; changed result bytes do

#### Scenario: Error responses may be retained but absent responses may not

- **WHEN** an admitted child call returns a complete MCP error or result with isError true under effective retention
- **THEN** it has a failed terminal outcome and a digest of its complete masked response object
- **AND** broker-local refusal, timeout, truncated response or interruption has no response digest and cannot cite fabricated child bytes

#### Scenario: A crash before delivery is not evidence of model consumption

- **WHEN** the broker persists a complete masked response and terminal record, then dies before forwarding it
- **THEN** recovery may retain that response and record the child outcome, but does not claim the harness or model received it
- **AND** the external call is not replayed to fill that delivery gap

#### Scenario: Existing corrupt or redirected content refuses

- **WHEN** publication encounters different bytes at the expected digest path, a symlink, a nonregular target or a staging digest mismatch
- **THEN** the engine refuses with respectively "retained artifact digest mismatch", "retained artifact path is not a regular contained file" or "broker response digest mismatch"
- **AND** it appends no success checkpoint referencing unverified bytes and does not replace the suspicious path

#### Scenario: Seat writes cannot forge evidence

- **WHEN** a boxed seat tries to rewrite or replace a plan, ledger, staged response or published digest via an ancestor rename, alias or overlapping bind
- **THEN** the protected authority/evidence bytes remain unchanged or use refuses with "broker evidence is not bound to this attempt"
- **AND** a fake-server end-to-end test observes both the legitimate artifact and the rejected tampering path

### Requirement: CR3 the ledger is durable and belongs to one attempt

Each broker SHALL write only its engine-provisioned per-attempt/per-capability
ledger. Records SHALL be at most 4 KiB, the ledger at most 64 MiB and each
broker attempt at most 4,096 accepted calls, reserving terminal capacity
before accepting a call. At the limit the session SHALL end before another
call is accepted with "broker ledger exceeds the attempt limit"; no accepted
call or ledger history is silently discarded. The broker SHALL stop reading
new request frames before exhausting the capacity reserved for every valid
request already decoded, including concurrency and ungranted-tool refusals.
Unread frames after closure are not accepted calls; every decoded valid call
still obeys CC2. The engine SHALL create and retain its expected ledger inventory
before launch, so a missing file is distinguishable from no calls or a broker
never started. A begun call SHALL be flushed durably before child forwarding;
its masked terminal metadata and retained response staging, if any, SHALL
be durable before returning the response. Failure to record SHALL prevent
forwarding or delivery, with "broker call could not be recorded".

Ledger records SHALL be strictly decoded into closed typed variants carrying
owner, monotonic sequence, call identity, tool, state and optional response
reference. They SHALL contain no call arguments or raw response body.
The engine SHALL validate every record against the sealed site/attempt,
dialect and grant before journal append. A hash without inaccessible ownership
does not authenticate a ledger. A broker cannot choose another run, attempt,
holding or output path.

#### Scenario: No-call and missing evidence differ

- **WHEN** an expected broker starts and makes no calls
- **THEN** its valid opened/closed ledger proves zero calls
- **AND** an absent expected ledger produces "broker ledger is missing for this attempt", while a broker proven never started is recorded as a failed start with no claim of a successful empty session

#### Scenario: Ledger bounds preserve accepted-call history

- **WHEN** the next call would exceed the ledger record/byte/call bound or leave no terminal capacity
- **THEN** the broker ends with "broker ledger exceeds the attempt limit" before accepting/forwarding it, preserving all accepted-call records
- **AND** exactly bounded positive controls remain recordable; recovery never treats this ending as successful completion

#### Scenario: Failure to record blocks external work

- **WHEN** writing or synchronizing the begun record fails
- **THEN** the broker returns "broker call could not be recorded" and the fake child's call log remains empty
- **AND** a terminal persistence failure withholds the response and leaves the durable begun record for interrupted recovery

#### Scenario: A forged owner or tool cannot become granted evidence

- **WHEN** a ledger names another attempt/dialect, repeats a conflicting call id, or marks an ungranted tool succeeded
- **THEN** folding refuses respectively "broker ledger owner does not match this attempt", "broker ledger contains a conflicting duplicate" or "broker ledger claims an ungranted tool"
- **AND** the engine parks the attempt through its existing failure path with no successful result admission

### Requirement: CR4 folding is idempotent and failures remain visible

The engine SHALL fold all available complete ledger records before admitting
a seat's terminal result, on normal completion, cancellation and recovery.
Fold identity SHALL be the bound attempt/broker/sequence and stage. Already
journaled byte-identical records SHALL not append again after recovery.
Conflicting duplicates SHALL refuse. The engine SHALL never replay a child
call to repair missing evidence.

A malformed record, sequence gap, unexpected ledger, missing expected ledger
or conflicting terminal SHALL prevent successful attempt completion with
its exact typed cause; an unexpected ledger has cause "broker ledger is not
expected for this attempt", and conflicting terminals use the conflicting-duplicate cause. A partial final record SHALL leave complete prior
records recoverable and mark unresolved calls interrupted; it SHALL park
with "broker ledger ends in a partial record", not accept a clean zero-use
attempt. Truncation in an earlier record SHALL refuse "broker ledger record
is malformed". A begun call lacking a terminal after process settlement
SHALL record interrupted with no digest. Crash recovery SHALL inspect existing
journal call ids before append and never infer response success from an
orphan artifact.

#### Scenario: Crash after append does not append twice

- **WHEN** the engine crashes after committing a terminal checkpoint but before recording its local fold cursor
- **THEN** recovery recognizes that bound call/stage in the journal and appends no duplicate
- **AND** a repeated record with different outcome/digest refuses with the conflicting-duplicate cause

#### Scenario: Partial or gapped ledgers cannot certify success

- **WHEN** folding encounters a partial tail, malformed middle record or a sequence gap
- **THEN** the attempt cannot complete successfully and reports respectively "broker ledger ends in a partial record", "broker ledger record is malformed" or "broker ledger sequence is not contiguous"
- **AND** complete verified prior records remain attributable; no missing call is invented or silently skipped

#### Scenario: An unexpected broker cannot add evidence

- **WHEN** a complete ledger exists beside the inventoried ledgers but its broker was not provisioned for this attempt
- **THEN** folding refuses "broker ledger is not expected for this attempt" and admits no successful seat result
- **AND** ignoring the extra ledger is not a valid recovery policy

#### Scenario: A child may have acted without returning

- **WHEN** a begun record exists but the child or broker dies before a durable terminal record
- **THEN** the engine records the call interrupted, without a response digest or an automatic retry of the external action
- **AND** a later attempt has a different identity; it cannot supply the missing terminal for this call

### Requirement: CR5 inspect opens only verified journal-referenced artifacts

Inspect SHALL offer retained evidence from a selected checkpoint/call, resolve
only its recorded digest under the operated repository's artifact root, and
verify the bytes before rendering or returning them. A caller-supplied path
SHALL not substitute for the digest. One pure derivation in brokkr-view SHALL
describe recorded attribution/retention across read surfaces; CLI/runtime
I/O SHALL perform artifact reads. No view function SHALL admit a grant.

#### Scenario: A reader opens exactly the cited response

- **WHEN** inspect opens a terminal checkpoint with a valid response_sha256 and matching content
- **THEN** it returns the stored masked bytes and their capability/dialect/tool provenance without contacting the server

#### Scenario: A call selector belongs to the selected run

- **WHEN** inspect --capability-call names an identifier not present in the selected run
- **THEN** it refuses "capability call is not recorded in this run" without reading an arbitrary path or another run's evidence

#### Scenario: Missing and corrupt evidence stay distinct

- **WHEN** the checkpoint has no digest, the file is absent, or its bytes fail SHA-256
- **THEN** inspect says respectively "no retained response is recorded for this call", "retained response artifact is missing" or "retained response artifact digest mismatch"
- **AND** it does not refetch, backfill the checkpoint or reinterpret historical absence as a retention veto

## Decisions

R4 assigns journal writes to the engine and per-attempt ledger writes to the
broker. Putting a broker SQLite writer beside the engine is rejected.
Putting a supposedly private ledger under an ordinary writable .forge path
is also rejected: it would allow a seat to manufacture evidence.

Only masked response data is citable; successful receipt is not inferred from persistence. Exact raw wire retention would violate
0012 when a server echoes its secret; hashing after masking states the bytes
honestly. Native retention is deferred, since the checked adapters do not
observe native response bodies. Veto false is not an instruction to delete
previous runs' historical artifacts; this slice introduces no garbage collector.
