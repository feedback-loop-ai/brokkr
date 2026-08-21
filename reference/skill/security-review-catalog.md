# Forge Security Review Catalog

Control catalog for the `vf-security-reviewer` agent. Every forge feature diff
is swept family by family; each family maps SOC 2 Trust Services Criteria and
ISO/IEC 27001:2022 Annex A controls to **concrete, code-level checks for the
Alkemio stack** (NestJS/GraphQL + TypeORM, React, Go services, Python/FastAPI,
RabbitMQ, Ory Kratos/Oathkeeper/Hydra, Kustomize/Helm on Scaleway).

Rules of use:
- Judge applicability by what the **diff touches**. Families not touched get a
  one-line `not-touched` note in `families_checked` — never silent omission.
- Every finding maps to ≥1 SOC 2 criterion and ≥1 ISO control from this file.
- The reviewer rates and remediates; it never accepts risk. Residual risk
  acceptance is a human decision surfaced via the evidence ledger and, at
  release time, the Release NN risk profile.

---

## 1. Access control & authorization

**SOC 2**: CC6.1, CC6.3 · **ISO 27001**: A.5.15, A.8.2, A.8.3

- Every new/changed GraphQL resolver, field resolver, REST route, MCP tool,
  and subscription enforces Alkemio's authorization framework (privilege check
  against the entity's authorization policy). A resolver without an explicit
  authorization guard is a finding — `critical` if it exposes write or
  cross-tenant read.
- New entities: authorization policy created AND cascaded from the parent
  (Alkemio's authorization-policy reset/inheritance flow). Orphan entities
  with default-open policies are `critical`.
- Privilege *names* used match the action's sensitivity (e.g. READ vs
  UPDATE vs GRANT); no privilege downgrades to "make it work".
- Admin-only surfaces live under the platform-admin roots, not general queries.
- Multi-tenancy: queries scoped by space/account membership — look for
  `find*` calls that filter only by ID without an authorization check
  upstream (IDOR pattern).
- Go/Python services: RabbitMQ consumers and HTTP handlers verify the caller
  (service auth, API key, or queue-topology trust documented in the plan);
  new internal endpoints are not reachable via the public ingress unless
  intended (check Oathkeeper rules / ingress manifests).

## 2. Authentication & session/credential handling

**SOC 2**: CC6.1, CC6.6 · **ISO 27001**: A.5.16, A.5.17, A.8.5

- Kratos/Oathkeeper/Hydra flows not weakened: no new endpoints bypassing the
  Oathkeeper JWT boundary; no session-token acceptance where a minted JWT is
  required; audience/issuer checks intact.
- No authentication toggles, mock identities, or `DEV_` bypasses reachable in
  production configuration.
- API keys/service credentials: prefixed/namespaced per convention, loaded
  from env/secret — never generated-and-logged, never default-valued in code.
- Password/recovery/verification flows: no user enumeration via differing
  errors; courier/email contents don't leak tokens to logs.

## 3. Input validation & injection

**SOC 2**: CC6.1, PI1.2 · **ISO 27001**: A.8.28

- DTO validation (`class-validator` / FastAPI pydantic / Go validators) on all
  new mutation inputs; length limits via the repo's constants (UUID_LENGTH etc.).
- SQL: TypeORM query-builder/raw-query usage with interpolated strings is a
  finding; parameters only. Same for any `psql`/driver calls in Go/Python.
- Command execution (`exec`, `spawn`, `subprocess`): no request-derived input
  into shell strings; argument arrays only, allowlisted binaries.
- Path traversal: file/document IDs mapped through storage abstractions, not
  user-supplied paths; uploads sniffed/validated (file-service conventions).
- SSRF: any new outbound fetch to a user-influenced URL must allowlist
  scheme/host or resolve through a vetted adapter.
- React: no `dangerouslySetInnerHTML` on user content without sanitization;
  markdown renderers configured to strip raw HTML/scripts; URL props validated
  (`javascript:` scheme).
- GraphQL: new list fields paginated per `docs/Pagination.md`; no unbounded
  `first`/`limit`; input complexity bounded (nested create graphs).

## 4. Secrets & configuration

**SOC 2**: CC6.1, CC6.4 · **ISO 27001**: A.8.9, A.8.12 (leak vector), A.5.33

- Grep-sweep the diff: private keys, JWTs, API tokens, passwords, connection
  strings, `Bearer` literals — in code, tests, fixtures, compose files,
  manifests, or docs. Any real credential is `critical` (and flag for rotation
  — committed history counts as exposed).
- New config: secret material comes from k8s Secrets / env — never ConfigMaps,
  never Kustomize literals in infra-ops; `.env.example` gets placeholders only.
- New env vars the code requires are declared in the relevant infra-ops
  overlays / dev-orchestration manifests (missing = availability finding;
  hardcoded fallback secret = `critical`).
- TLS verification never disabled (`rejectUnauthorized: false`,
  `InsecureSkipVerify`, `verify=False`); permissive CORS (`*` with
  credentials) is a finding.

## 5. Cryptography

**SOC 2**: CC6.1, CC6.7 · **ISO 27001**: A.8.24

- No homegrown crypto/signing/random-token schemes — platform libs only;
  tokens from CSPRNG (`crypto.randomBytes`/`secrets`), never `Math.random()`.
- Hashing for credentials/lookup uses fit-for-purpose algorithms (no MD5/SHA1
  for anything security-relevant); constant-time comparison for secrets.
- Data in transit between services stays on TLS or in-cluster channels the
  plan documents; no new plaintext external hops.

## 6. Logging, monitoring & audit trail

**SOC 2**: CC7.2, CC7.3, CC4.1 · **ISO 27001**: A.8.15, A.8.16

- Repo convention enforced: **no dynamic data (IDs, emails, tokens) in
  exception messages** — structured `details` only; Winston signatures correct.
- No PII/secrets/session tokens in log lines, APM spans, or error payloads
  returned to clients (stack traces stripped in prod paths).
- Security-relevant actions (role grants, authz policy changes, credential
  ops, destructive admin mutations) emit auditable log/activity entries —
  removal or bypass of existing audit logging is a finding even if tests pass.
- New failure paths log enough context to investigate (who/what/when) without
  violating the PII rule.

## 7. Privacy & data protection

**SOC 2**: P-series (P1–P6), C1.1 · **ISO 27001**: A.5.12, A.5.34, A.8.10, A.8.11

- New personal-data fields: classified, minimized (needed for the feature?),
  and covered by deletion flows (user/account deletion cascades reach them).
- PII not sent to third parties (incl. LLM/AI services) without an existing
  documented basis; VC/assistant features: check what leaves the platform
  boundary in prompts and tool payloads.
- Exports/messages/notifications don't leak data across space/tenant
  boundaries (check recipient resolution against membership).

## 8. Supply chain & dependencies

**SOC 2**: CC9.2, CC6.8 · **ISO 27001**: A.5.19, A.5.21, A.8.30

- Lockfile diffs: new packages are reputable, pinned, and license-compatible
  (EUPL-1.2 platform); watch for typosquats and postinstall scripts.
- No `pkg.pr.new`/git/preview dependencies added beyond the documented TypeORM
  fork exception without a plan-level justification.
- Dockerfiles/base images: no `:latest` for production images; new images in
  manifests come from the org's registries.

## 9. Change management, migrations & rollback

**SOC 2**: CC8.1 · **ISO 27001**: A.8.32

- DB migrations: idempotent, inline rollback notes, validated via the repo's
  migration harness; destructive migrations (drop/rename) staged per the
  expand-contract pattern and called out in rollout ordering.
- GraphQL schema: `schema:diff` clean of **unapproved BREAKING** changes;
  deprecations follow `REMOVE_AFTER=YYYY-MM-DD | reason`.
- Feature flags: risky surfaces land default-off with the flag flip staged in
  infra-ops per the plan's rollout ordering.
- The evidence ledger (forge-run.md) must be able to answer "what changed,
  who reviewed, what were the gates" — flag anything that undermines that.

## 10. Availability & resilience

**SOC 2**: A1.1, A1.2 · **ISO 27001**: A.8.6, A.8.14

- Unbounded work per request: missing pagination, N+1 resolver fan-out without
  dataloaders, unindexed hot queries on large tables.
- RabbitMQ consumers: acking semantics correct; long-running handlers vs
  `consumer_timeout` (a known production failure mode — heavy messages
  redelivering forever); poison-message handling / retry caps.
- k8s manifests: resource requests/limits present for new workloads; probes
  defined; single-replica statefulness acknowledged in the plan.
- Timeouts + error handling on all new outbound calls (HTTP, Matrix, LLM
  engines); no infinite retry loops.

## 11. Environment separation

**SOC 2**: CC6.1, CC8.1 · **ISO 27001**: A.8.31

- No production endpoints, credentials, or data references in dev/test config
  or seeds; no acceptance↔production cross-wiring in overlays.
- Test fixtures don't carry production-derived personal data.

---

## Severity ladder

| Severity | Bar |
|---|---|
| `critical` | Exploitable now: auth bypass, cross-tenant access, secret exposure, RCE/injection, data loss |
| `high` | Exploitable with realistic preconditions, or a control removed (audit logging, validation, TLS) |
| `medium` | Defense-in-depth gap, risky-but-guarded pattern, compliance drift needing tracked remediation |
| `low` | Hardening opportunity, convention drift with no realistic path |
| `info` | Observation for the ledger; no action required |

**Verdict**: `fail` = any critical/high open · `conditional` = only medium open
(ship allowed, remediation tracked in the story) · `pass` = low/info at most.
