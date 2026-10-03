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
or oversized payloads SHALL be rejected, not repaired. Use existing
canonical::to_bytes and canonical::sha256_bytes at the bounded edge, with one
masked buffer for staging and forwarding. Parsing/serialization SHALL preserve
JSON numeric values exactly; an unrepresentable number SHALL refuse "MCP server
protocol is invalid", never round into different evidence. This is a strict
edge check, not a second canonicalizer or normalization of embedded documents.
MB4's output-edge check SHALL additionally refuse an unsafe scalar/structural
secret with "MCP response cannot be safely masked" before staging or delivery,
regardless of retention policy. Such a forwarded call is failed without a
response digest; no unsafe child bytes or substituted scalar are retained.
For a response admitted by both checks, the same masked object
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
are not sufficient. Protection SHALL cover every Brokkr-managed writer sharing the operated
artifact root, including zero-grant panel siblings, sequence steps, boxed exec,
fallbacks and later runs with historical artifacts. It SHALL be established
before any such writer starts, not only before the retaining broker. A known
concurrent/already-running unprotected writer or an unprovable writer boundary
SHALL refuse with "capability evidence root cannot be protected from every managed writer".
This does not claim protection from arbitrary operator host processes or
older uncoordinated engines; migration requires quiescing those writers.
Read/write verification belongs above pure core/view.

#### Scenario: The digest names the delivered masked data

- **WHEN** a retained response contains a bound secret and nested structured result data
- **THEN** the artifact is the canonical masked result object, its SHA-256 equals response_sha256 on the terminal checkpoint, and the delivered response has identical masked result data
- **AND** different JSON-RPC ids do not change this digest; changed result bytes do

#### Scenario: Error responses may be retained but absent responses may not

- **WHEN** an admitted child call returns a complete, safely maskable MCP error or result with isError true under effective retention
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

#### Scenario: A sibling with no grant cannot alter retained evidence

- **WHEN** a retaining panel member runs beside a zero-grant work member or boxed exec that tries to replace the artifact root or an existing digest
- **THEN** both writers received protection before launch and the legitimate artifact remains unchanged
- **AND** an already-running unprotected managed writer makes admission refuse with the exact managed-writer cause; later zero-grant runs preserve historical evidence too

#### Scenario: Canonicalization cannot silently change numbers

- **WHEN** a response contains a large integer or precise decimal that the existing serializer would round
- **THEN** the edge refuses "MCP server protocol is invalid" with no delivered substitute or retained digest
- **AND** exact representable values retain their value and use the existing canonical byte/hash functions unless MB4's independent masking check refuses; masking-created duplicate keys also refuse

#### Scenario: Exact numeric representation does not prove secret safety

- **WHEN** an exactly representable numeric secret is returned under retained true without a veto
- **THEN** MB4's exact unsafe-masking cause produces a failed call with no response_sha256, staged body or published artifact
- **AND** text-redaction and unrelated-number controls retain their exact canonical masked bytes; disabling retention or vetoing it never permits the unsafe response to be delivered

### Requirement: CR3 the ledger is durable and belongs to one attempt

Each broker SHALL write only its engine-provisioned per-attempt/per-capability
ledger, exclusively created for one process lifetime. A second lifetime using
that plan SHALL refuse "broker plan has already been opened" before child
spawn; reconnect never resets the sequence. The sealed plan inventory is the
single source for expected ledgers. Existing engine launch facts distinguish
proven never-started from uncertain starts; no separate authoritative launch
state machine is required. Missing/uncertain evidence cannot certify success.

Records SHALL be strictly decoded into closed Opened, Started, Terminal and
Closed variants, with owner and contiguous record sequence. Exactly one Opened
comes first; every accepted call has the next call sequence and one Started,
then at most one Terminal for that same call/tool. Refused calls also have a
Started record but no forwarding. Terminal without Started, duplicate Opened,
changed ownership or any record after Closed SHALL refuse. Closed appears once,
last, after all accepted calls have terminal records. A missing Closed after
process settlement SHALL refuse "broker ledger lifecycle is invalid", as do
illegal state transitions; owner mismatch retains its more specific cause.
Verified begun calls without terminal become interrupted under CR4.
Closed SHALL carry a closed typed disposition, Clean or Failed with a bounded
broker cause. A fatal initialization/version/protocol, response-limit, timeout,
unsafe-correlation or ledger-limit ending SHALL latch Failed, including when
there were no accepted calls. Keep the first observed fatal cause; later EOF
or orderly cleanup cannot overwrite it. Causes use the owning MB3/MB4/CR3
fixed text, never raw child output. Ordinary child tool errors and durable
local tool/active-call/budget denials alone do not latch session failure.
Reserve closure capacity before acceptance and fsync its disposition before
normal exit. If closure cannot be persisted, preserve the verified prefix
and the existing missing/partial/lifecycle refusal; do not invent Clean or a
precise persistence cause absent from recovery evidence.

Records SHALL be at most 4 KiB, each ledger at most 64 MiB and each broker
attempt at most 4,096 accepted calls. Reserve terminal and closure capacity
before accepting/forwarding. A valid bounded request becomes accepted only
when its Started is durable; tool, active-call and retention-budget denials
follow that point and receive refused Terminal records. Invalid request
identities are rejected at the MB3/SC4 edge without echoing them into evidence.
Stop reading new frames before those bounds are exhausted, preserving space
for every accepted call including local refusals. At the limit end with
"broker ledger exceeds the attempt limit";
never silently discard accepted calls. Unread frames are not accepted calls.
A begun record SHALL be fsynced before forwarding. Terminal metadata and any
retained staging SHALL be durable before response delivery. Failure SHALL
prevent forwarding/delivery with "broker call could not be recorded".
No arguments, raw response body, command or request URL enters ledger metadata.

Retained responses SHALL also have a fixed aggregate budget: 256 MiB of
cumulative canonical response bytes per attempt across all brokers, including
reserved in-flight responses. Allocate disjoint shares in the sealed inventory
before launch; include all permitted fallback broker slots, with no new quota
on retry/reconnect. Each retaining slot needs at least 8 MiB, reserved before
forwarding one call; after durable terminal staging charge actual bytes and
release only the unused reservation. Completed-byte charges remain until
attempt settlement, even for identical digests. Staging plus engine publication
copies SHALL never exceed 512 MiB of newly written response data per attempt.
Insufficient allocation, share exhaustion or unavailable reserved capacity
SHALL refuse "capability evidence budget is exhausted" before external work.
No automatic eviction, garbage collection or metadata-only downgrade is allowed.
Nonretaining responses remain bounded in-memory forwarding only.

#### Scenario: No-call and missing evidence differ

- **WHEN** an expected broker starts and makes no calls
- **THEN** exactly Opened then Closed proves zero accepted calls, and only a Clean disposition proves the broker reported a healthy session
- **AND** Failed closure with zero calls retains the fatal cause and cannot certify success
- **AND** an absent expected ledger produces "broker ledger is missing for this attempt"; a proven never-started broker records failed start, not successful empty use

#### Scenario: Lifecycle order and one lifetime are enforced

- **WHEN** a settled ledger lacks Closed, repeats Opened, records Terminal before Started or appends after Closed
- **THEN** folding refuses "broker ledger lifecycle is invalid", preserving only verified prior calls
- **AND** reopening the same plan refuses "broker plan has already been opened" without child spawn; sequence ownership cannot restart

#### Scenario: Ledger bounds preserve accepted-call history

- **WHEN** the next call would exceed the record/byte/call bound or leave no terminal/closure capacity
- **THEN** the broker ends with "broker ledger exceeds the attempt limit" before accepting/forwarding, preserving accepted-call records
- **AND** exact-limit controls remain recordable; this ending never becomes successful completion

#### Scenario: Aggregate retention exhaustion cannot lose a delivered body

- **WHEN** several brokers would exceed their preallocated shares or the attempt's 256 MiB cumulative retained budget
- **THEN** the next call refuses "capability evidence budget is exhausted" before forwarding, with no silent metadata-only success
- **AND** simultaneous brokers and fallback slots cannot spend the same reservation; exact-bound controls pass, newly written staging/publication stays within 512 MiB, and prior retained evidence remains intact

#### Scenario: Failure to record blocks external work

- **WHEN** writing or synchronizing Started fails
- **THEN** the broker returns "broker call could not be recorded" and the fake child's call log is empty
- **AND** it admits no successful attempt or fabricated call checkpoint; settlement recovers only the verified ledger prefix, including a complete Started whose acknowledgement was lost
- **AND** terminal persistence failure withholds delivery and leaves the durable Started for interrupted recovery

#### Scenario: A forged owner or tool cannot become granted evidence

- **WHEN** a ledger names another attempt/dialect, repeats conflicting call evidence, or marks an ungranted tool succeeded
- **THEN** folding refuses respectively "broker ledger owner does not match this attempt", "broker ledger contains a conflicting duplicate" or "broker ledger claims an ungranted tool"
- **AND** no successful result is admitted; hash equality cannot authenticate writable evidence

#### Scenario: Session failure survives orderly closure

- **WHEN** the child's pinned version mismatches before any call, fatal protocol handling fails after a terminal call, or the ledger reaches exactly 4,096 accepted calls
- **THEN** Closed durably carries Failed and respectively the pinned-version/protocol/ledger-limit cause, with exact accepted-call counts 0, 1 and 4,096
- **AND** later EOF cannot replace the failure with Clean; failed closure persistence leaves only verified-prefix recovery with its exact evidence refusal
- **AND** a healthy unused session and a session with only recoverable tool errors or ungranted-tool denials may close Clean without relabeling their individual call outcomes

### Requirement: CR4 folding is idempotent and failures remain visible

After the owned process tree settles, the engine SHALL fold each inventoried
ledger through the existing checkpoint settlement barrier before admitting any
terminal seat result: success, failure, cancellation, timeout or recovery.
It SHALL append exactly one settled checkpoint per accepted call, using known
Terminal or interrupted when only a verified Started exists. No live ledger
scan or public started checkpoint is required. Settlement SHALL also inspect
each validated Closed disposition before admitting the harness result. A known
Failed session overrides harness success through existing failed/indeterminate
attempt handling, retaining its exact typed cause and verified call outcomes.
A zero-call Failed session creates no fictitious tool checkpoint. Clean closure
is necessary but not sufficient: all other evidence checks must still pass.
Live telemetry is diagnostic;
a partial write while its broker is running SHALL NOT be judged corruption.

Fold identity is call_id, with full typed payload comparison. The committed
journal is the deduplication authority; no second authoritative cursor is kept.
A queued/offered checkpoint is not committed. Folding SHALL wait for actual
append confirmation, keeping unread records on durable disk and bounded working
memory instead of using lossy progress coalescing or the bounded held-event
queue. A retry under the existing append patiences SHALL neither duplicate a
pending row nor discard it. Exhausted patiences SHALL leave evidence recoverable
and use the existing failed/indeterminate settlement; no clean terminal result
is admitted. Recovery SHALL never replay the external call.

A malformed record, gap, unexpected/missing ledger, invalid lifecycle or
conflicting call SHALL prevent successful completion with its exact typed
cause. Unexpected ledgers refuse "broker ledger is not expected for this
attempt"; conflicts use CR3's conflicting-duplicate cause. After settlement,
a partial final record leaves verified prior calls recoverable and unresolved
calls interrupted, refusing "broker ledger ends in a partial record". Malformed
middle records refuse "broker ledger record is malformed"; gaps refuse "broker
ledger sequence is not contiguous". Do not invent a missing call or infer
success from an orphan artifact. Validate framing/owner/order before projecting
verified calls; stop at the first invalid transition and preserve its cause.

#### Scenario: A successful harness cannot hide a fatal broker session

- **WHEN** a deterministic harness ignores broker failure and reports success after a pinned-version mismatch before any call, a fatal protocol failure after one completed call, or exhaustion at exactly 4,096 accepted calls
- **THEN** the real engine settlement barrier rejects that success with respectively "MCP server version does not match the dialect's pinned version", "MCP server protocol is invalid" or "broker ledger exceeds the attempt limit", retaining exactly 0, 1 or 4,096 call checkpoints
- **AND** it invents no extra call; recovery makes the same judgment from the durable disposition without replay, while healthy zero-call and recoverable-tool-error controls can complete
- **AND** independently removing the engine disposition check makes these exact attempt-outcome assertions fail; a cleanup-only mutation proves a different property

#### Scenario: Crash after append does not append twice

- **WHEN** the engine crashes after a checkpoint commit but before append acknowledgement
- **THEN** recovery matches the exact call_id/payload in the journal and appends no duplicate
- **AND** different outcome/digest for the same call_id refuses the conflicting-duplicate cause

#### Scenario: Contention cannot turn a queued row into lost evidence

- **WHEN** a journal lock spans repeated settlement attempts for the same call
- **THEN** exactly one row is committed after release, with evidence kept on disk until confirmation
- **AND** contention outlasting the existing patiences preserves recovery inputs and blocks clean terminal admission; no progress queue drop is treated as success

#### Scenario: Live partial writes differ from settled corruption

- **WHEN** a live broker pauses halfway through appending a record
- **THEN** no live fold declares it corrupt; a completed valid ledger folds after process settlement
- **AND** a partial tail remaining after settlement refuses "broker ledger ends in a partial record", a malformed middle record refuses "broker ledger record is malformed", and a sequence gap refuses "broker ledger sequence is not contiguous"

#### Scenario: An unexpected broker cannot add evidence

- **WHEN** a ledger exists beside the expected ledgers but its broker is absent from the sealed inventory
- **THEN** folding refuses "broker ledger is not expected for this attempt" and admits no successful result
- **AND** ignoring the extra ledger is not a recovery policy

#### Scenario: A child may have acted without returning

- **WHEN** Started exists but the child or broker dies before durable Terminal
- **THEN** settlement records exactly one interrupted checkpoint without digest or automatic retry
- **AND** missing closure prevents a successful attempt; a later attempt cannot supply this call's missing terminal

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

Council robustness C/D/F and simplicity A/C are combined here: protect every
managed writer, retain the private lifecycle, and publish one settled row only
on confirmed append. The earlier producer-only and offered-row assumptions
were upstream safety gaps (0071 rulings 3, 5, 8, 9). Fixed disjoint quotas avoid
a new evidence service. Reuse existing canonical bytes with an exact-number
edge guard; a second serializer is rejected under ruling 5. Delayed authoritative
MCP display and serialized managed runs sharing an artifact root are accepted
costs; losing evidence or silently changing a returned value is not.

Specify return, 2026-10-03: adopt CR2–CR4's returned repairs. Writable worktree
mounts and Checkpoints::offer's unconfirmed queue support the council's
all-writer protection and confirmed settlement requirements; a hash or queued
row cannot supply either proof (0071 rulings 3, 8, 9). Retain fixed attempt
quotas, exact-number validation and reuse of canonical bytes (rulings 3, 5).
CR3 now states the acceptance point shared with CC2; neither ledger exhaustion
nor a failed Started write creates a fictitious recorded call. Runtime security
proofs remain tasks, not findings closed by prose.

Council return R-G/R-H, 2026-10-03: adopt typed Clean/Failed private closure
and engine settlement of that outcome. Presence of Closed, a complete call
set and harness success cannot distinguish a healthy zero-call session from
failed initialization; an end marker is not a session result. Reject using
missing closure to encode a known fault when its safe cause can be synced.
Reuse existing failure transitions and confirmed append; no public lifecycle,
new journal writer or policy-table input is required (0071 rulings 2, 3, 8, 9).
CR2 also adopts MB4's scalar/structural masking refusal. The legacy numeric
residual conflicts with the stronger retention promise; exact numeric identity
alone cannot discharge secrecy. These owning repairs and scenarios go upstream
for review, with all dependent design/proof tasks retained.
