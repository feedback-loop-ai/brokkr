# Change: DSH operator console, slice one — what Brokkr supports today (#568)

Status: proposed. Fired for delivery by the operator on 2026-10-13. The
defaults named under **Defaults taken** stand unless the operator rules
otherwise.
Change: `2026-10-13-568-operator-console-slice-one`.
UX source: the "DSH Operator Console" canvas, page **Step 1 · feasible today**,
boards S1Realms, S1Run, S1Decisions and S1Analytics (2026-10-13). The canvas's
Exploration page holds the fuller vision and the feasibility notes this slice
was cut from.
Parent story: [#568](https://github.com/feedback-loop-ai/brokkr/issues/568), part of #330.

## Why

#568 asks for an operator journey inside a DSH client: inspect → explain →
choose → execute → observe. A feasibility pass against `main` (`67a1886e`)
found that the read side and four operator commands rest on facts and verbs
Brokkr already has. The step-1 boards draw exactly that. Everything else from
the exploration appears in the UI as "not available yet", with its reason.

## What Changes

Two deliverables. The Brokkr side is Rust in this workspace. The DSH side is
plugin bytes under decision 0009's extension boundary.

### Brokkr side (Rust, `brokkr-view` + `brokkr-cli`)

- **Admissible commands in the view model.**
  - `RunView` gains, for the run as folded, the operator commands Brokkr
    would accept now: retry (naming the phase it re-runs), stop, resume and
    conclude.
  - Each refused command carries the refusal text the verb would give.
  - Each disabled action the UI shows carries its stated reason: dismiss a
    finding, talk to the reviewer, hold a parked run, answer here.
  - `inspect --json` emits it. One derivation, per decision 0013.
- **A cross-run analytics readout.**
  - A pure aggregation in `brokkr-view` over the hearths `realms.json`
    names, for a range and an optional realm.
  - A new `brokkr analytics --json` readout emits it.
  - Metrics are listed in `operator-console-analytics`.
- **Clarify questions as a readable fact.** When a run parks after
  clarify's rounds are exhausted, the view model carries the clarifier
  seat's open questions. It also carries the run's worktree path, so a
  client can open the spec for the operator to answer.
- No journal, schema, fold or policy change. No new operator verb.

### DSH side (`extensions/dsh/brokkr-console/`)

- A host plugin that runs Brokkr readouts and the four existing verbs and
  serves them to its client over plugin-owned HTTP and WebSocket routes.
- A client module:
  - a **Brokkr** entry in DSH's left panel list (`sidebar.panellist`), with
    the waiting-on-you count drawn in its own icon;
  - a panel registered in the root `main` slot, with two views: **Realms**
    and **Analytics**.
- **Realms:**
  - realm cards (what needs you with Brokkr's reason, running, shipped and
    stopped in 24h);
  - the queue with each entry's admission verdict and who held it;
  - an **Ask Brokkr** assistant;
  - Muninn's last review, from `brokkr muninn list`.
- **One realm:**
  - the run list (needs you / running / last 24h, filterable), run header,
    run graph and timeline;
  - the decision card: Brokkr's park reason word for word, the rule, links
    to evidence, a required reason, and only the admitted commands. The
    "not available yet" actions are shown disabled with their reasons;
  - **Ask about this run**, which may suggest one admitted command as a
    card the operator uses or ignores.
- **Other decisions:**
  - clarify questions: open the spec in DSH's Files sidebar, then Retry
    clarify;
  - out of attempts: retry starts over, with no cost estimate;
  - quiet runs: Resume, or Close it (conclude).
- **Outcomes:** done; not sent (the run changed); refused (Brokkr's text);
  checking (lost acknowledgement).
- **Analytics:** the step-1 board, with "not recorded yet" listed instead
  of invented numbers.

## Defaults taken

- **D4 (fencing):** the plugin host checks the run's head before invoking a
  verb, and dedupes by command id. The verbs gain no `--expect-seq` in this
  slice.
- **DSH target:** `0.2.0-rc.2`, the version installed and on npm `latest`.
  The adapter's `0.1.5-rc.1` pin for seats is unrelated and unchanged.
- **Authority:** a single local operator. The plugin binds to loopback and
  acts as the OS user, as the CLI does.
- **Plugin home:** `extensions/dsh/brokkr-console/`, following
  `plugin-cli-session`'s precedent. A proposed decision records that this
  plugin, unlike the precedent, carries its own source and tests there.

## Capabilities

### New Capabilities

- `operator-console-read`
- `operator-console-commands`
- `operator-console-analytics`
- `operator-console-assistant`

### Modified Capabilities

None.

## Out of scope (shown as "not available yet" or absent)

| Element | Reason | Owner |
|---|---|---|
| Answer a clarify question in the console | No answer channel; resume takes no text | decision on operator guidance to seats |
| Talk to the reviewer | Gates are always fresh (0056); resume is live only for codex work seats | same decision |
| Dismiss a finding while parked | Supersede applies only to finished runs (0047) | amend 0047 |
| Hold a parked run | `hold` acts on queue entries only | engine work |
| Start a queued run from the console | The dispatch loop is unbuilt | #430 |
| Seats as capacity, hosts, seat utilisation | Not in Brokkr's model or journal | later ruling |
| Cost caps, €, Codex/dsh cost, tokens | Not recorded or unreliable (#281, #560) | later |
| Second operator or remote DSH host | No authority model yet | #568 ruling |

## Impact

- `crates/brokkr-view`: admissible commands, clarify questions and the
  worktree path in `RunView`; the analytics aggregation. Pure, with tests
  under the coverage gate.
- `crates/brokkr-cli`: an `analytics` readout. `inspect --json` carries the
  new fields.
- `extensions/dsh/brokkr-console/`: plugin bytes with a provenance note.
  Never published.
- `docs/status.md` and a guide entry.
