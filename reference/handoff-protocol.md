# Forge durable handoff protocol

Every model or deterministic layer produces a durable handoff before the next
layer starts. The handoff is the recovery authority; a provider session is an
optional accelerator.

## Invariants

1. Never advance a phase without a written handoff whose schema, canonicalized
   frontmatter-plus-body content hash, body hash, and recursively validated
   parent graph pass `.scripts/forge-control.py verify-handoff`.
2. Keep attempts append-only. A retry gets a new invocation ID and file. Mark
   an old attempt superseded through a later handoff; do not overwrite it.
3. Store the provider, requested and actual model, requested and actual effort,
   runner, billing mode, token/cache usage, marginal or reported USD cost,
   repository SHAs, and a fingerprint of the provider session in the committed
   frontmatter. Subscription calls record a null marginal price rather than a
   fictional API cost.
4. Store the actual provider session ID only in the ignored local state under
   `.forge/state/<run-id>/sessions.json`. Treat it like a resumability token.
5. Persist only intentional outputs and evidence. Never persist hidden chain of
   thought, DeepSeek `reasoning_content`, or Qwen reasoning items in a committed
   handoff.
6. Give downstream agents the raw handoff assets and referenced evidence, not
   a coordinator-authored conclusion that replaces them.
7. Council provenance is blind during deliberation. Give seats and the chief
   the output of `.scripts/forge-control.py blind-handoff`, which preserves the
   authored body and hash while removing provider, model, runner, billing, and
   session metadata. The full handoff remains available only to the coordinator
   and final ledger until the chief has ruled.

## Phase machine state

`state.json`, `events.jsonl`, and `journal-head.json` belong to the
deterministic phase machine (`.agents/forge/phase-machine.json`), not to a
model. Only `.scripts/forge-control.py` writes them.

- `state.json` — current phase, last result and rule id, shippability, open
  blockers, and the repo HEADs the review panel examined. A cache: re-read it
  with `phase-state`, but never treat it as the authority.
- `events.jsonl` — append-only, hash-chained journal, one record per
  transition: timestamp, from-phase, result, deciding rule id, next phase,
  severity, the computed facts behind the ruling, observed repo HEADs, drift,
  and dirty worktrees. Open hard blockers are reconstructed from it, which is
  why deleting `state.json` cannot discard a security hold. Editing an entry
  breaks the chain and is detected.
- `journal-head.json` — the chain head recorded outside the journal, so
  truncation (which leaves a valid prefix) is detectable too. A convenience
  anchor only: it is an ordinary writable file, so an attacker who rewrites the
  journal can rewrite it to agree.
- `refs/forge/<feature-id>` — **the forensic anchor.** Every phase
  transition, clearance, and attestation appends a commit to this ref recording
  the journal's SHA-256, its entry count, and every feature repo's HEAD at that
  moment. Because a commit object is content-addressed, a consistent rewrite of
  both the journal and `journal-head.json` — which defeats the file anchor —
  still fails against it. Built with git plumbing on a dedicated ref, so the
  index, working tree, and checked-out branch are never touched, and replaying a
  recorded journal reproduces the same SHAs. **Tamper-evidence, not
  tamper-proofing**: the ref carries no signature, so anyone able to run git in
  the workspace can rebuild the chain. It raises the cost of forgery and makes
  honest corruption visible; it does not constrain a hostile agent.

  `FORGE_SIGN_ANCHORS=1` requests signed anchors, waiting up to
  `FORGE_SIGN_TIMEOUT` seconds (default 60, clamped 5–600) for the operator to
  touch their key, then falling back to an unsigned anchor. The budget is a
  person's reaction time, not a process timeout: an unattended run still
  finishes, and a present operator can actually sign.

  **Know where the prompt appears.** An agent-run process has no tty, so a
  curses pinentry renders into whatever pty `gpg-agent` last saw — often a
  different terminal pane. If nothing seems to be asking for a PIN, look at the
  terminal that started `gpg-agent`; for a fully non-interactive key, configure
  a loopback or graphical pinentry. This is why signing appeared not to work at
  all before the prompt was understood.

  A missed touch is a transient miss, not proof the key is gone: the "do not
  re-pay the timeout" probe expires after 15 minutes, so one missed prompt does
  not silently disable signing for the rest of a run. The probe lives in the
  ignored `.forge/state/` (keyed by GNUPGHOME), never beside the journal where
  an agent could pre-plant it.

  A signature alone still proves nothing — `%G?` reports "good" for *any*
  locally-trusted key, and an agent can generate one in seconds — so
  `anchor_signed()` additionally requires the fingerprint to appear in the
  committed `.agents/forge/signing-keys.txt`. That allowlist is deliberately a
  tracked file: git config is writable by the agent and `git config` is not a
  governed command.

  `ship-gate` reports `anchor_mode: signed | partially-signed | unsigned`.
  A partially-signed chain is reported, never blocking — an all-unsigned chain
  is shippable, so treating a partial one as worse wedged runs where the key
  simply went away mid-run.

  Anchoring is best-effort, so a write can fail (a stale `.lock`, a lost CAS, a
  full disk). Those gaps are recorded in `anchor-gaps.jsonl` and the gate
  distinguishes them from a rewritten chain, with
  `forge-control.py phase-reanchor` rebuilding the chain from the journal (the
  superseded chain is kept at `refs/forge-superseded/<id>`). **Repair is
  operator-only** — it rebuilds evidence *from* the journal, so an agent that
  doctored the journal could otherwise mint a matching chain and ship. It
  requires an interactive confirmation or an explicit `--assume-operator`,
  exactly like retiring a blocker. Without that, one
  transient failure permanently blocked a completed feature and blamed
  tampering — whose only rational response is to bypass the gate.

Clearance records (`kind: "clearance"`) retire a hard blocker only when
`attested: true`. An agent may write a proposal with `phase-clear`; only an
operator's `phase-attest` retires it. Signing records (`kind: "signing"`) note
a batched signature-only rewrite and carry no phase transition.

## Layout

Use this structure beneath `specs/<feature-id>/forge/`:

```text
run.json
events.jsonl
architecture/positions/<seat>-<attempt>.md
architecture/clashes/<seat>-<attempt>.md
architecture/chief/<attempt>.md
implementation/<repo>/<attempt>.md
contracts/<contract>/<attempt>.md
gates/<repo>/<attempt>.md
verification/<track-or-story>/<attempt>.md
public-evidence/manifest.json
public-evidence/<explicitly-disclosable-prompts-schemas-and-images>
review/<repo>/<dimension>/<attempt>.md
findings/<finding>/<lens>-<attempt>.md
regression/<attempt>.md
ship/<repo>/<attempt>.md
```

`forge-run.md` remains the concise human ledger and links to these assets. It
does not duplicate their full content.

At checkpoints, run `.scripts/forge-control.py summarize-handoffs
specs/<feature-id>/forge` to aggregate per-provider input/output/cache tokens,
known USD cost, and explicitly unpriced subscription/API invocations. Never
turn a null marginal subscription cost into a made-up API equivalent.

## Session boundaries

- Resume a council seat from position to clash.
- Start the chief in a fresh session from the written positions and clashes.
- Resume an implementer for a targeted repair only while its worktree and
  parent handoff hashes still match.
- Gate auditors, reviewers, security reviewers, finding verifiers, and live
  acceptance walkers always start fresh independent sessions.
- On resume, prefer the recorded session only when the provider, model,
  prompt-schema version, repository SHAs, and parent hashes still match.
  Otherwise start clean from the handoff.

## Cache-aware prompt order

Render provider prompts in this order:

1. stable tool definitions and output schema;
2. stable Forge phase charter;
3. stable repository instructions;
4. stable spec, task, and accepted parent handoffs;
5. current diff, failure, attempt, timestamp, and other volatile data.

Never interpolate timestamps, session IDs, invocation IDs, or changing Git
status into the stable prefix. Keep provider-specific cache controls in the
runner adapter and record cache reads/writes in the handoff usage object.

For Qwen text calls, a matching Model Studio response ID may resume the council
seat while the provider/model/schema/parent/repository invariants still hold.
Qwen multimodal calls do not resume by response ID: order stable instructions
before images to maximize context-cache reuse, then resume the trusted verifier
from the written visual handoff.

## DeepSeek boundary

Before serializing a DeepSeek prompt, require a successful eligibility check.
The gate explicitly denies chief, review, security review, live verification,
finding verification, stack operation, and infrastructure provisioning or
operations tasks. The two infrastructure repositories remain hard-denied even
if their visibility metadata changes. A denied task must fail before its prompt
or handoffs are read into the DeepSeek invocation adapter.

Build a sanitized bundle containing only the approved public-repository inputs
and public-context task. The DeepSeek process receives only that bundle and the
approved, dedicated, clean public worktree paths. Git inspection is fail-closed:
tracked modifications, untracked files, ignored files, or inspection errors
deny dispatch, and local Git metadata is hidden inside the sandbox. It never
receives the workspace root, private repositories, credentials, private specs,
private issue text, or mixed public/private contract payloads.

Every provider process runs in its own process group with a configurable
deadline. Timeout terminates the complete process tree and persists a failed
result/metadata record before the invocation returns an error.

## Qwen / Model Studio boundary

Before reading a Qwen prompt or image file, require a successful public-context
eligibility check for an explicit set of public, non-infrastructure repositories.
Only `council-seat` and `visual-verifier` are eligible. Qwen is read-only and
receives no repository mount, shell, browser, credentials, raw traces, or agent
tools. The trusted coordinator must serialize a bounded public excerpt bundle
for architecture work.

Every prompt and optional output schema must be staged beneath the feature's
`public-evidence/` directory and attested in `manifest.json` before invocation.
The manifest uses `forge.public-evidence/v1` with a `files` array of exact
`path`, `sha256`, and `kind` (`prompt` or `schema`) records. The adapter checks
the path, bounded size, UTF-8/JSON encoding, and exact bytes before serializing
them; a caller-provided `--context-public` flag alone is never sufficient.

Visual evidence must be copied intentionally beneath
`specs/<feature-id>/forge/public-evidence/` and supplied with explicit `--image`
arguments. The adapter rejects symlinks, paths outside that directory,
unsupported types, images over 7 MiB, more than 32 images, and aggregate input
over 32 MiB. It encodes approved files locally, records their paths and hashes,
and sends them only to an allowlisted HTTPS Model Studio endpoint. Redirects
and ambient proxies are disabled. The Qwen result is advisory: a trusted
Playwright verifier consumes its model-blind handoff and owns the final verdict.
