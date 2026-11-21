# 0111a: the forge's evidence

Decision 0111's first slice (#1093): real payloads from this repository's own CI, captured before any contract is written. The operator ruled "measure first" on 2026-11-21, after 0111's eleventh hold.

## How it was taken

- **When, by whom:** 2026-11-21, 16:50Z to 18:15Z, by the controller on the operator's behalf, by hand and outside any run.
- **How:** every call is `gh api --method GET`. 0111a asks for a read-only token, and the token used was not one, so the capture kept to GET calls.
- **What was read for each head:**
  - its check runs under `filter=latest` and `filter=all`;
  - its commit statuses, and the combined status;
  - the workflow runs on it;
  - for each run, every attempt, each attempt's jobs, and the run's jobs under `filter=latest` and `filter=all`;
  - the pull requests the commit belongs to, and the open ones on its branch;
  - main's rules, and its branch protection's required checks.
- **Pages:** every list is read with `per_page=100` to its last page, the page that holds fewer than 100 items.
- **What was left out:** no job log is captured.
- **Two forge writes** made the cases history lacked, both on #1237's head 4f343ba26:
  - one `workflow_dispatch` of ci (run 44926784147);
  - one full re-run of its pull-request run 44924886779, as attempt 2.
- **Redaction:** each `email` value, and every other e-mail address in a string, is replaced by `redacted@example.invalid`. That covers commit messages' sign-offs, but not `git@github.com` or `noreply@anthropic.com`. Nothing else changed. Files are compact JSON.
- **`index.json`** in each case names every file's API path, the byte size of the forge's raw response, and when it was taken.

## The cases

| Directory | Head | What it holds |
|---|---|---|
| `cancelled-twin/` | 669303526 (#1222, closed) | two `pull_request` runs of ci one second apart. The older, 3560, was cancelled by the newer's concurrency group, and 3561 is green. |
| `failed-jobs-rerun-with-cancelled-twin/` | f26bccaea (#1216, closed) | ci 3515 cancelled, and ci 3516 failed, then had its failed jobs re-run as attempt 2. |
| `two-runs-both-rerun/` | d66801d2d (#1229, closed) | ci 3542 and 3543, both cancelled, then each re-run as attempt 2. The older run's attempt started first. |
| `dispatch-beside-pr-in-progress/` | 4f343ba26 (#1237, open) | the pull-request run 3568 and the dispatch run 3569, both in progress. |
| `dispatch-in-progress-beside-pr-green/` | 4f343ba26 | 3568 green, and 3569 still in progress. |
| `dispatch-and-pr-both-green/` | 4f343ba26 | both green. |
| `older-run-full-rerun-in-progress/` | 4f343ba26 | 3568's full re-run (attempt 2) in progress, newer by its start than 3569. |
| `older-run-full-rerun-complete/` | 4f343ba26 | the same, complete and green. |

## What it settles

Each line names the payloads that show it.

- **A check run names no attempt.** Its fields are `id`, `name`, `status`, `conclusion`, `head_sha`, `details_url`, `check_suite.id`, `app`, `pull_requests`, the times and `output`.
  - Its `id` is its job's `id`, and `details_url` names `/actions/runs/<run>/job/<job>`.
  - Only the job carries `run_id` and `run_attempt`.
  - So a check's attempt is read through its job (every case).
- **`filter` on check runs defaults to `latest`.** On f26bccaea, `total_count` is 70 with no filter, 70 under `latest` and 103 under `all`.
  - `latest` drops an earlier attempt's check runs within the same run (check suite).
  - It keeps every other run's, so a cancelled twin's checks stay visible under `latest` (`cancelled-twin`: 74 and 74).
- **A failed-jobs-only re-run lists every job in the new attempt.** f26bccaea's attempt 2 lists all 33 jobs:
  - each with `run_attempt: 2` and a new `id`;
  - none of attempt 1's ids recur;
  - 27 of them were not re-run, and keep attempt 1's `started_at`, `completed_at` and `conclusion`. They are told apart only by a `started_at` before the attempt's `run_started_at`.
- **A full re-run re-creates every job.** 4f343ba26's attempt 2 has 33 jobs, all started after the attempt began, with no id shared with attempt 1.
- **How a run's times move:**
  - `created_at` is the run's first creation, and stays put across attempts;
  - `run_started_at` is the latest attempt's start;
  - `run_number` orders runs by creation only.
  - An older run's re-run is newest by `run_started_at`. 3568's attempt 2 started at 17:32:56Z, after 3569's 16:50:47Z.
  - Each run's `previous_attempt_url` names its earlier attempt.
- **What a run's `pull_requests` and a check run's `pull_requests` hold:**
  - an open pull request's number on its head's runs and check runs, the `workflow_dispatch` run included (#1237);
  - empty on the heads whose pull request is closed (#1216, #1222, #1229), though `commits/<sha>/pulls` still names the closed one.
- **A dispatch run shares the required names.** It writes check runs under the same names as the pull-request run, each beside the other under `latest`.
  - Its `delivered by brokkr` concludes `skipped`, since that job runs only for a pull request.
  - While it ran, a required name could hold a green pull-request check beside a dispatch check still `in_progress`, or none yet (`dispatch-in-progress-beside-pr-green`).
- **No commit statuses:** every head holds none. The checks are check runs only.
- **No tested-commit field:** a run names only `head_sha`, the pull request's head, in both checkout modes. No field names the merge revision a `merge`-checkout job tested.
- **No job key:** a job carries its display `name` (`test (ubuntu-latest) on linux/1`), but neither its workflow key nor a matrix key.
- **The app:** every check run is `github-actions`, id 15368.
- **The required set** is branch protection's 12 checks, each with `app_id` 15368, and `strict: false`. The main ruleset holds only the merge queue (`rules.main.json`, `protection.required_status_checks.json`).
- **Pages:** three heads' check runs under `filter=all` took two pages (103, 103 and 136 runs), and the last page held fewer than 100. `total_count` on the first page gives the whole count. The largest page any list returned is 309,267 bytes, under 0111's 8 MiB response bound.
- **The status and conclusion words seen:**
  - `completed` with `success`, `failure`, `cancelled` or `skipped`;
  - `in_progress` with no conclusion.
  - `queued` and `waiting` were not caught.

## Still unmeasured

- whether `gh api` streams a response body or holds it whole;
- a fork's pull request;
- the `queued` and `waiting` words;
- a list whose last page is exactly 100 items (the `per_page` rule's edge).
