# cspergel/Charter: implementation dossier

Research date: 2026-09-27 (Europe/Sofia). Live metadata retrieved 2026-09-26T22:00:45.610813+00:00. Default branch: `main`. Revision: [`3a46876de80d17d5d94682160835f5695c3487a4`](https://github.com/cspergel/Charter/tree/3a46876de80d17d5d94682160835f5695c3487a4); commit date: 2026-06-18T12:13:40Z. Repository license: [MIT](https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/LICENSE), recorded without a legal interpretation.

[Inspection method and limits](../0019-github-peer-implementation-investigations.md#method-and-limits) apply to this dossier.

## What the project establishes

Charter is an explicitly small proof of concept for mapping architectural decisions to checks, not a multi-agent graph runtime. It is more substantive than the original brief comparison conveyed: it has deterministic assertion execution, approval/trust checks, adversarial mutations, edit hooks and a hash-linked governance record with a committed head anchor. It is especially relevant to Brokkr's principle-to-enforcement mapping and qualification backlog. The pinned package says version 0.5.0, Python 3.10+, a single `charter.py` module, and zero runtime dependencies. Optional LLM use calls a custom command or Anthropic HTTP API through standard-library code. [Scope][C1], [packaging][C2].

The operating flow is design prose → optional LLM annotation → human-reviewed `CHARTER.md` decisions → local approval → deterministic `check` and optional edit hook → tripwire/adversarial `verify` → advisory LLM audit for supervision-only rules → trace/graph/log/digest for inspection. The graph is decision-to-code traceability, not a scheduler. Reusable decision IDs link intent to code citations, explicit watched paths and a declared enforcement rung. [CLI implementation][C3], [self-governing charter][C4].

## Enforcement and meaningful refusal cases

An assertion is a repository-authored shell command expected to exit zero on compliance. A tripwire is a separate must-succeed probe showing that its detection mechanism can recognize a known violation. `check` fails when an assertion or tripwire fails or times out. This directly addresses the “green by omission” problem where a typoed path makes a grep always pass. Other enforcement rungs are weaker: a type/test/lint target is checked for file or token presence, not automatically executed as a test. Dotted symbols may resolve to their final member token. These are liveness checks on evidence references, not proof of semantic coverage. [Enforcer implementation][C5].

`check` refuses unapproved or changed charter content; it does not execute smuggled asserts while the approval stamp is invalid. Supervision-only decisions with neither code citations nor watched paths fail by default, while broad watch scopes, uncited governed files, bare test-file references and trivial always-true tripwires are warnings. That severity distinction matters: the existence of a tripwire field is not mandatory universal proof. Tests explicitly check unapproved content, tampering, vacuous asserts, blind supervision and coverage-blind targets. [Check path][C6], [regression tests][C22].

The edit hook can temporarily apply proposed content and block an edit when an assert fails with the edit but passes without it, avoiding blame for pre-existing failures. The inspected tests distinguish a violating change from a compliant one and require silence when the charter is untrusted/tampered. That is a targeted in-loop refusal mechanism. Its coverage depends on the host calling the hook for the supported tool payload, not on a filesystem-level write barrier. [Edit hook][C8], [hook tests][C23].

## Local trust, arbitrary commands and secrets

The security document is unusually explicit about its trust boundary. A committed charter hash only shows that someone approved those bytes; a malicious committer can rewrite both charter and stamp. Assert execution therefore also requires a per-user trust record outside the repository, keyed by absolute path and bound to an uncommitted `.git` instance nonce. A newly cloned repository cannot inherit trust merely by shipping approval files; replacement at the same path loses the nonce association. Tests cover forged in-repository trust markers and delete/reclone replacement. Non-Git repositories and worktrees fall back to path plus hash, a documented weaker instance identity. CI can opt into execution using `--trust` or `CHARTER_TRUST_ASSERTS=1`. [Threat model][C9], [trust implementation][C10], [forgery tests][C11].

This is intentional local approval, not sandboxing. Assert commands run through a POSIX shell in the repository, with the process's ambient authority/environment. The 30-second subprocess timeout does not show owned process-group teardown; Python's direct process handling alone does not establish descendant settlement. `capture_output` buffers output, and slicing error messages or LLM responses afterward is not a pre-capture memory bound. The custom LLM command uses `shell=True` and a five-minute timeout; direct HTTP uses a three-minute timeout. These surfaces require the operator's reviewed command policy. [Shell and LLM implementation][C12].

Only annotate/audit require LLM content transfer; audit sends governed file contents and annotate sends design input. Security documentation acknowledges secrets can leave and that model prompt-injection resistance is imperfect. Audit has bounded parsing, depth/candidate limits and per-decision file caps; invalid/unavailable backend results degrade to AMBIGUOUS, while backend nonzero exit discards its output. An AMBIGUOUS verdict is advisory and does not itself increment the violation exit count. A CI caller must not interpret exit zero from advisory audit as established compliance for every decision. [Security scope][C9], [audit implementation][C13], [backend/parser tests][C24].

## Adversarial verification and its limits

`verify --adversarial` asks a model for one concrete create/append mutation per assertion, applies it, runs the assertion, restores the targeted original bytes in `finally`, and can request a stronger suggested enforcer when a bypass succeeds. Tests include an in-scope violation that is caught and a violation placed outside the grep's scope that is missed, with restoration checked in both cases. This is a useful negative-control discipline for Brokkr's qualification work: demonstrate the detector would fail when the claimed invariant is broken. The README's reported rust-analyzer bypass count remains a maintainer anecdote, not an independently reproduced result. [Adversarial implementation][C14], [mutation tests][C15], [README claim][C1].

The implementation's “sandboxed” wording should be qualified. `_mutated` writes the real target path temporarily and restores that one path; it is not a separate isolated checkout or OS sandbox. `safe_mutation_target` prevents path escape, charter/state replacement and directory targets, but the asserted shell command itself can have arbitrary additional effects. `finally` cannot restore after uncatchable termination, and the inspected exclusion does not list `.git` as protected from model-proposed mutation. Consequently a successful single attack test proves only that chosen detector caught that mutation, not that the architectural rule is universally unbypassable. The code's stronger “provably enforced” success wording exceeds that experimental scope. [Mutation guard and restoration][C16], [verify result wording][C14].

## Governance history and durability

The ledger records timestamps, decisions/actions, reasons, review markers and a hash of the preceding raw line. It truncates SHA-256 to sixteen hexadecimal characters. A separate committed anchor stores count and final-line hash so simple truncation/prefix replacement is detectable relative to that anchor. Tests exercise tampering, truncation and review marking. This is real tamper-evidence relative to retained Git history, and should be included in the peer comparison. [Ledger append][C17], [verification][C18], [ledger tests][C7].

It is not an immutable append-only store in the strict Brokkr sense: `digest --mark` rewrites prior entries' reviewed flags, recalculates the chain and rewrites the anchor. The append path and separate head update lack a transaction/fsync/locking protocol in the inspected code. A crash or concurrent writer can therefore leave a mismatch; no recovery protocol was established. Verification tolerates foreign/unparseable lines and entries lacking `prev`, rather than enforcing one closed event schema. A committer able to rewrite ledger and anchor can reconstitute a consistent history. The security document's limited threat model is more accurate than interpreting hashes as authenticated governance. [Review rewrite][C19], [ledger verification][C18].

## Maintenance and Brokkr experiments

CI compiles the module, runs pytest, installs the package, invokes help and runs Charter against its own charter on Ubuntu/Windows with Python 3.10/3.13. Publication uses an OIDC PyPI workflow. The tests include concrete trust-forgery and detector-mutation scenarios; this review did not execute them. A small single-file implementation makes tracing policy-to-code easy, while combining parsing, execution, trust, ledger and hooks in that file creates shared maintenance risk as scope grows. [CI][C20], [publish workflow][C21].

For Brokkr #429/#435–439, copy the negative-control method: pair each claimed invariant with a known violating fixture, then ask an independent maintainer to propose another bypass. Distinguish reference liveness, behavioral enforcement and advisory model judgment in the report. Test stale approval, cloned/replaced repository identity and worktree trust fallback. For #403/#433, run a reviewed fake assert that leaves descendants and produces unbounded output. For #431/#434, kill between ledger append and anchor update, then compare explicit recovery/refusal. These bounded experiments extract Charter's strongest lesson—testing that checks can actually fail—without adopting its shell trust model or overstating its evidence durability.

## Pinned evidence ledger

| ID | Source | Evidence role |
|---|---|---|
| C1 | [README.md](https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/README.md#L1) | Proof-of-concept scope and maintainer reports |
| C2 | [pyproject.toml](https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/pyproject.toml#L1) | Runtime, version and zero dependencies |
| C3 | [charter.py](https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/charter.py#L1734) | CLI command surface |
| C4 | [CHARTER.md](https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/CHARTER.md#L1) | Project self-governance decisions |
| C5 | [charter.py](https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/charter.py#L316) | Assert/tripwire and symbol presence checks |
| C6 | [charter.py](https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/charter.py#L810) | Approval and enforcement severity |
| C7 | [tests/test_charter.py](https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/tests/test_charter.py#L259) | Behavioral regression cases |
| C8 | [charter.py](https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/charter.py#L1395) | Proposed-edit refusal hook |
| C9 | [SECURITY.md](https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/SECURITY.md#L1) | Threat model, worktree fallback and data disclosure |
| C10 | [charter.py](https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/charter.py#L434) | External trust store and instance nonce |
| C11 | [tests/test_charter.py](https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/tests/test_charter.py#L792) | Forged approval and replacement tests |
| C12 | [charter.py](https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/charter.py#L164) | Shell/LLM timeouts and capture |
| C13 | [charter.py](https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/charter.py#L1183) | Advisory audit outcomes |
| C14 | [charter.py](https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/charter.py#L1100) | Adversarial verification algorithm |
| C15 | [tests/test_charter.py](https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/tests/test_charter.py#L137) | Caught and bypassing mutation tests |
| C16 | [charter.py](https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/charter.py#L968) | Real target mutation and finally restoration |
| C17 | [charter.py](https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/charter.py#L131) | Truncated hash chain and separate head write |
| C18 | [charter.py](https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/charter.py#L1496) | Tolerant chain verification |
| C19 | [charter.py](https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/charter.py#L1334) | History rewrite on digest mark |
| C20 | [.github/workflows/ci.yml](https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/.github/workflows/ci.yml#L1) | Cross-platform tests and self-check |
| C21 | [.github/workflows/workflow.yml](https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/.github/workflows/workflow.yml#L1) | OIDC publication configuration |
| C22 | [tests/test_charter.py](https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/tests/test_charter.py#L331) | Unapproved content, assertion and supervision tests |
| C23 | [tests/test_charter.py](https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/tests/test_charter.py#L215) | Violating/compliant edit-hook tests |
| C24 | [tests/test_charter.py](https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/tests/test_charter.py#L417) | Unavailable/failing backend tests |

[C1]: https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/README.md#L1
[C2]: https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/pyproject.toml#L1
[C3]: https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/charter.py#L1734
[C4]: https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/CHARTER.md#L1
[C5]: https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/charter.py#L316
[C6]: https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/charter.py#L810
[C7]: https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/tests/test_charter.py#L259
[C8]: https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/charter.py#L1395
[C9]: https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/SECURITY.md#L1
[C10]: https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/charter.py#L434
[C11]: https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/tests/test_charter.py#L792
[C12]: https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/charter.py#L164
[C13]: https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/charter.py#L1183
[C14]: https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/charter.py#L1100
[C15]: https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/tests/test_charter.py#L137
[C16]: https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/charter.py#L968
[C17]: https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/charter.py#L131
[C18]: https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/charter.py#L1496
[C19]: https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/charter.py#L1334
[C20]: https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/.github/workflows/ci.yml#L1
[C21]: https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/.github/workflows/workflow.yml#L1

[C22]: https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/tests/test_charter.py#L331
[C23]: https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/tests/test_charter.py#L215
[C24]: https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/tests/test_charter.py#L417
