# 0077 — The capability broker is Brokkr's own and the harness's child

Status: accepted (operator ruled in chat, 2026-10-03)
Date: 2026-10-03

Built: unbuilt (#467) — strict MCP prerequisite; complete slice-two work is in the linked change tasks
Amends: 0065

## Context

Decision 0065 ruling 6 says an MCP dialect's server is "launched by the engine
beside the hands server". The operator's R1 of 2026-10-03 requires an explicit
amendment: the intended authority stands, but that launch ownership does not
describe the implementation we must extend.

The [operator ruling dated 2026-10-05](../../openspec/changes/decision-0065-capabilities-slice-two/operator-ruling-2026-10-03.md#addendum-2026-10-05-each-mcp-server-runs-in-its-own-box) supplies the amendment: each MCP server runs in its own box.

In the checked source, crates/brokkr-runtime/src/engine.rs:4434–4440 documents the
harness-spawned hands child. crates/brokkr-protocol/src/hands.rs:1119–1142
builds one MCP server entry invoking this executable's hands serve.
crates/brokkr-protocol/src/native_controls.rs:2433–2456 admits exactly the
transport's hands. Adding an independently launched engine child would need
a different transport and supervision arrangement. An engine-authored second
configuration entry alone would still fail the existing final check.

The [operator record](../../openspec/changes/decision-0065-capabilities-slice-two/operator-ruling-2026-10-03.md)
governs the proposed slice; only the operator accepts this decision. Exact
admission, refusal and evidence obligations live in the linked change.
[Reservation and review provenance](../../openspec/changes/decision-0065-capabilities-slice-two/evidence.md#reservation-evidence)
remain in its durable evidence record; concurrent number claims must be
rechecked before implementation.

The unbuilt marker links open [#467](https://github.com/feedback-loop-ai/brokkr/issues/467),
the strict MCP prerequisite. The complete work is in
[the slice-two tasks](../../openspec/changes/decision-0065-capabilities-slice-two/tasks.md).
No runtime protection or decision acceptance is claimed by these documents.

## Rulings

1. **The engine authorizes; the harness launches Brokkr's broker.**
   For every held MCP capability, the engine writes one server entry named
   cap-<capability> in the same MCP configuration as brokkr workspace hands.
   It invokes the current Brokkr executable as brokkr broker serve.
   The harness starts that broker as its child; the broker starts the real
   stdio server whose argv and version are the pinned operator dialect's.
   The broker and child stay in the attempt's process group, covered by the
   attempt cleanup machinery. Brokkr supplies mediation, not a capability
   server or a catalogue of knowledge tools.

   This amends 0065 ruling 6's "launched by the engine" to "launched by the
   harness from the engine's sealed MCP configuration, through Brokkr's own
   broker". Its realm-only authority and outside-the-box placement stand.

   **Enforcement binding:** selected site/candidate MCP plan and engine-owned
   configuration builder; broker Cmd variant and handler; final command/config
   parse-back against independent intent; fake-server process-tree/cleanup
   tests at the production launch boundary. Authored matching bytes retain
   authored provenance and refuse.

2. **The first broker channel is namespace only.**
   MCP capability holdings require actual workspace hands under namespace,
   network false and a harness measured to exclude ambient MCP configuration.
   No harness, open, seatbelt, container, handless or exec site gains a broker
   channel from this proposal. Wider scope requires a later decision.
   A harness's own MCP configuration is never inherited, including by seats
   with no capability requests. Codex isolation is selected by U0 measurement,
   not by an assumed configuration precedence rule.

   **Enforcement binding:** compile and final-launch eligibility checks with
   exact site/boundary/strictness causes; U0 adapter evidence, SI1–SI2 and MB2
   in the change. Namespace availability remains a real host check. Linux and
   macOS remain the supported hosts; macOS does not gain a namespace launcher.

3. **The broker holds the tool filter and secret boundary.**
   The real server receives only its declared secret bindings, resolved by
   the broker from the operator's store at spawn and injected into that
   child's environment. Values never enter argv, the harness environment,
   a manifest or a seat's box. The decision 0012 single plaintext injector
   is reused or factored without a second disclosure call site, including
   calls inside the secret module in the machine proof.
   The broker lists only the realm-admitted tools and refuses an ungranted
   call before forwarding it. Its output is masked before delivery or
   persistence. A known secret in a scalar or structural field that cannot
   be redacted without changing the admitted shape refuses with
   "MCP response cannot be safely masked"; no unsafe response is delivered
   or retained. A forwarded call fails without a digest; unsafe correlation
   ends the session without emitting an unsafe frame. The legacy numeric
   residual is not inherited by this broker boundary.
   Neither tool arguments nor returned content can change
   connection, grants, restrictions, secrets or retention. Secret-bearing
   holdings additionally require measured exclusion of store and child-process
   reads from both hands and model-native tools; read-only write containment
   does not establish that. Before secrets are resolved, the executable,
   startup inputs and working directory must be protected from seat writes.
   The broker's fixed startup environment survives binding injection: names
   colliding with its builder-owned keys, including HOME/TMPDIR, refuse before
   store lookup or spawn with MB3's existing startup-input cause. This leaves
   the shared name grammar and one injector intact. An unprovable startup
   arrangement refuses; no whole-harness box or installer supplies a proof.

   **Enforcement binding:** typed identity-bound broker plan, tool-list/call
   checks, shared secret resolver/injector/masker, no-forward and leak-scan
   tests against a fake server, including common encodings and chunk splits.
   Decision 0036's binding clearance and decision 0043's boxed-seat binding
   refusal stay intact. Nonempty restrictions remain deferred by slice-one D11.

4. **The engine alone writes capability evidence to the journal.**
   Every accepted broker call is recorded in a protected per-attempt ledger
   before external forwarding or a local tool/budget denial, with a terminal
   outcome when known. Durable Started defines acceptance under the change's
   bounded request/ledger contract; unread or invalid requests gain no invented
   checkpoint, and persistence failure blocks successful completion. The engine
   folds and validates it against the pinned holding and existing journal state.
   After process settlement, one public checkpoint records each accepted call's
   settled outcome. Private Started/Terminal records stay private; actual
   journal commit, not a queued offer, confirms evidence. An uncertain action
   is never replayed. A fixed attempt-wide retention budget is reserved before
   forwarding, with no silent downgrade on exhaustion.
   Private Closed carries Clean or a latched typed failure cause. The engine
   checks it even when the harness reports success, including zero accepted
   calls and ledger exhaustion. A fatal session cannot become a clean attempt;
   ordinary tool errors remain call outcomes. If closure cannot persist,
   existing incomplete-evidence refusals apply without invented success.
   A broker never opens the journal for writing. Missing, malformed, partial
   or conflicting evidence cannot produce a successful complete attempt.
   Retained responses are masked data prepared for delivery, stored content-addressed
   at .forge/artifacts/sha256/<hex>, with the digest in the checkpoint.
   A persisted response proves no model receipt across a crash.
   The dialect opts in through retained; the realm may veto through the new
   reserved grant key. Every Brokkr-managed writer sharing the artifact root,
   including a zero-grant sibling and later runs, is subject to evidence
   protection before dispatch. Unsafe concurrent writers refuse; historical
   artifacts remain protected. This is not a guarantee against arbitrary
   operator host writes or older engines that have not been quiesced.

   **Enforcement binding:** seat-record v6 and run-manifest v12; additive
   realm contract and existing tool-dialect v1; broker durable ledger, engine idempotent fold and
   artifact publication; inspect digest verification; crash/tampering/veto
   tests. Definitions of retained bytes and refusal text live in the change's
   retention and contract deltas.

5. **Enablement follows the operator's unit process.**
   The completed specification and implementation plan require operator ruling.
   Subsequent narrow units are signed PRs from main
   through the merge queue. No unit before U9 permits compilation of an MCP
   grant. Independent native gate rules, universal strict MCP, panic repairs
   and native attribution may become active before the broker is enabled.
   Generated declarations migrate before mandatory strict admission; native
   driver emission waits for the v6 fence and every engine attribution consumer.
   Intermediate merges preserve valid legacy checkpoints.

   **Enforcement binding:** the unchanged realm-wide MCP refusal and its
   removal-control test until U9; the design's ordered one-PR units with at
   most three production files each; tasks and final validation evidence.
   Operator acceptance is recorded by the operator, never by this author.

## Alternatives weighed

- **An engine-spawned independent server.** Rejected for this slice: it does
  not use the harness's existing stdio child channel and would need new
  supervision outside the attempt's inherited process group.
- **Expose the dialect server directly.** Rejected: tool filtering, refusal
  recording, secret masking and the private ledger need Brokkr's mediator.
- **Put secret values into MCP configuration or the harness environment.**
  Rejected: the configuration can enter argv and the harness would receive
  credentials meant solely for the server.
- **Trust a successful Codex hands call as isolation evidence.** Rejected:
  adapters/codex.json:94 explicitly says ambient exclusion was not established.
- **Let the broker append directly to SQLite.** Rejected: the engine is the
  journal's single writer; per-attempt ledgers preserve that ownership.
- **Enable the parser before its serving protections.** Rejected: the
  resolver and doctor have native-only binding assumptions and the final
  check has a one-server invariant. These are prerequisites, not exceptions.

## Consequences

The launch tree becomes engine, driver, harness, broker, dialect server,
with hands remaining the harness's other engine-configured child. The model's
workspace calls stay inside their network-sealed namespace. The real MCP
server has the operator-declared reach outside it; tool filtering is not a
claim of a generic child-server sandbox or a new host-allowlist transport.

This proposal changes the launch mechanism described by 0065 while preserving
its grant, DATA and evidence intent. It does not enable comparison work,
whole-harness boxes, URL clients, nonempty restriction transport or native
response retention. The existing attempt-cleanup residuals remain residuals.

Implementation is Rust under crates/. Closed state is typed and matched
exhaustively; effects live above core/view, shared readouts derive once in
brokkr-view, errors are typed, and new tests bind under decision 0071 rulings
1–3 and 7–10. File/function ceilings and the unchanged exact-coverage gate
still govern the implementation. This document proves no runtime behavior.
