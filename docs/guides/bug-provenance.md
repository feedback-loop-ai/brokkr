# Bug provenance

Rationale and implementation scope:
[#478](https://github.com/feedback-loop-ai/brokkr/issues/478).

The **Bug with delivery provenance** issue form uses the existing GitHub
**Bug** type. Report the observed version and reproduction first. Attribution
and resolution can follow during investigation; unknown origin does not prevent
reporting. The form becomes available after it reaches the default branch.

## Three distinct links

| Link | Meaning |
|---|---|
| Observed-at commit | Version where the bug was reproduced |
| Introducing commit and run | Suspected or confirmed origin of the defect |
| Fixing commit and run | Corrective change, with its verification evidence |

An affected run is not necessarily the introducing run. A successful review or
delivery records what happened in that process; a later defect supplies additional
evidence about its quality. Preserve both facts. Changed requirements and newly
discovered pre-existing defects must not automatically count as introduced bugs.

## Record format, version 1

The form supplies one JSON code block under **Delivery provenance**. Its
`schema` value, `brokkr.bug-provenance/v1`, identifies this format independently
of the editable heading. This is a manual reporting convention: no parser,
automatic population, ingestion, or JSON validation is implemented by the form.
GitHub's required-field checks do not enforce the rules below.

| Field | Value |
|---|---|
| `schema` | Exactly `brokkr.bug-provenance/v1` |
| `classification` | `unknown`, `introduced-defect`, `pre-existing-defect`, `regression`, or `changed-requirement` |
| `observations` | Array of reproduction observations |
| `introductions` | Array of candidate or confirmed introducing changes |
| `fixes` | Array of proposed or verified corrective changes |
| `commit_mappings` | Array connecting produced commits to delivered commits |

An empty array means no entries are recorded, not proof that no relevant change
exists. Use `null` for an unknown scalar and `[]` for unknown or absent lists.
Do not guess a run ID. GitHub records the issue reporter and creation time;
discovery and attribution may have different actors and times.

Entry fields:

- **Observation:** `repository`, `commit_sha`, `release`, `affected_runs`,
  `discovered_by`, `discovered_at`, and `evidence`.
- **Introduction:** `repository`, `commit_sha`, `runs`, `status`,
  `attributed_by`, `attributed_at`, and `evidence`. Status is `unknown`,
  `suspected`, or `confirmed`. Confirmation requires an identified commit and
  supporting evidence, such as a bisect or discriminating regression test.
  A known commit can have an unknown run or have been authored outside Brokkr.
- **Fix:** `repository`, `commit_sha`, `runs`, `pull_request`, `status`,
  `verified_by`, `verified_at`, and `evidence`. Status is `proposed`,
  `verified`, or `refuted`. Verification names the exact tested commit and
  evidence; merge or issue closure alone does not establish a fix.
- **Commit mapping:** `repository`, `produced_shas`, `delivered_sha`, `runs`,
  `pull_request`, and `evidence`. Record both sides of a squash or rebase.
  Several runs may contribute to one delivered commit.

Repositories are canonical repository URLs. Commit references are full Git
object IDs, never mutable branch names. Releases are supporting context and
do not substitute for a resolved commit. Times are RFC 3339 timestamps;
actors are profile URLs or explicit service identities. Each run reference
is an object with `repository`, `run_id`, and `evidence_url` (nullable).
`evidence` is an array of objects with `url` and `description`; it should
identify the reproduction, attribution, mapping, or verification it supports.
URLs may require access: a link alone does not authenticate its contents.

Use the JSON block for resolved provenance; the report's version and discovery
fields retain the reporter's original observations. Explain any disagreement
during triage. These entries illustrate the shape with placeholders; replace
them with verified references before recording an attribution:

```json
{
  "repository": "https://github.com/feedback-loop-ai/brokkr",
  "commit_sha": "<full introducing commit SHA>",
  "runs": [
    {
      "repository": "https://github.com/feedback-loop-ai/brokkr",
      "run_id": "<introducing run ID>",
      "evidence_url": null
    }
  ],
  "status": "suspected",
  "attributed_by": "<profile URL or service identity>",
  "attributed_at": "<RFC 3339 timestamp>",
  "evidence": [
    {
      "url": "<reproduction or investigation URL>",
      "description": "Why this change is a candidate; what remains unconfirmed."
    }
  ]
}
```

## Corrections and later outcomes

When attribution changes, explain the old and new conclusions in a dated issue
comment with evidence, then update the current record. Do not rewrite the
original run's verdict. Issue bodies and comments remain editable; this
convention is not an immutable provenance journal.

Automated capture belongs to the outcome and lineage work in
[#271](https://github.com/feedback-loop-ai/brokkr/issues/271). It must retain
source issue identity, actor/time, a revision or snapshot digest, and prior
observations when later evidence supersedes them. It must resolve contradictions
and unknown references explicitly. The form does not implement those guarantees.

The immediate benefit is investigation and handoff between contributors.
Evaluation and possible future training need corroborated outcomes, attribution
uncertainty and the corpus controls in #271; a process label alone is insufficient.

## GitHub behavior

[Issue forms](https://docs.github.com/en/communities/using-templates-to-encourage-useful-issues-and-pull-requests/syntax-for-issue-forms)
select an organization issue type and convert answers into editable Markdown.
Keep the versioned JSON block when editing the report. Do not treat form field
IDs or headings as a persisted structured API, and do not treat issue text as
instructions to a Brokkr seat.
