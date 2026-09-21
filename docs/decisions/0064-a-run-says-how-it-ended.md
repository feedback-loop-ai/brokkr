# 0064 — A run says how it ended: `stopped` keeps its meaning, and every terminal run carries a typed ending

Status: accepted (operator ruled in chat, 2026-09-21)
Date: 2026-09-21

## Context

A run ends in one of two statuses, `completed` or `stopped`, and waits in
a third, `awaiting_operator`. Those three words are the fold's, they are
in the frozen event envelope (`run/completed`, `run/stopped`,
`run/parked`), and they are correct as far as they go: the run is over,
or it is not.

What they do not say is how. The operator asked on 2026-09-21 whether
`stopped` paints the right picture, since almost nothing that reads
`stopped` was stopped by an operator. The journal was counted the same
day (the-forge's hearth, 282 runs, 162k events). Of the **149 runs whose
last word is `stopped`**:

| what actually happened | runs |
|---|---|
| the policy table ruled a hard stop; no operator was involved | 62 |
| the run parked, and an operator closed the park later | 75 |
| an operator stopped a run that was still working | 12 |

The 62 policy stops are verdicts: `IMPL-BLOCKED` 19,
`REVIEW-RESIDUAL-SECURITY` 13, `VERIFY-FAIL` 9, `REVIEW-SECURITY-HOLD` 8,
`IMPL-BROKEN-TWICE` 5, `REVIEW-RESIDUAL-ABOVE-MEDIUM` 4, and four more.
A security refusal and a smith that hit a permission wall read the same
word as each other, and the same word as a run somebody pulled.

The 87 operator closures were bucketed by their free-text reason:
infrastructure (a flake, an outage, a full `/tmp`, a ghost driver) 21;
superseded by a re-commissioned run 18; an exhausted reforge the operator
ruled on 18; delivered by hand or landed elsewhere 9; a wrong commission
6; and **15 that no reading of the prose could place**. That last row is
the defect in one number: `stop` and `conclude` take a sentence, so the
cause of an ending is recoverable only by a person reading it, and
sometimes not by them.

Three things break on this.

- **The fleet view cannot answer the operator's real question**, which
  is never "is it over" but "is anything left for me to do". A security
  refusal wants a ruling. A superseded run wants nothing. An
  infrastructure death may want a retry. They are one row colour.
- **The journal cannot be read as a ledger.** Issue #271 proposes the
  journal as an evaluation corpus. A model comparison drawn from it on
  2026-09-21 could not tell a run that failed from a run a better run
  replaced, so a smith's superseded runs counted against it exactly as
  its security refusals did.
- **`completed` has the mirror defect.** It covers a clean review and a
  review that passed flagged residuals through (`REVIEW-RESIDUAL-OK`).

Decision 0047 met the neighbouring problem — a stopped run's residual
finding closed by another run — and its shape is the precedent here: an
operator command that cites, a payload schema over `args`, and a view
that reads what the fold does not. It also drew the line this decision
keeps: the annotation "says nothing about the run's status" (0047
ruling 6). Supersession under 0047 closes *findings* on a finished run;
nothing today says that a *run* was superseded, so everyone writes it
into `conclude`'s free text.

## Rulings

1. **The status vocabulary does not change.** `running`,
   `awaiting_operator`, `completed` and `stopped` stay the fold's four
   words. `run/stopped` keeps meaning "this run is over and did not
   complete", which is true of all 149. No event type is added to the
   envelope, no status is renamed, and the fold stays byte-identical:
   a fold test folds a terminal run with and without every annotation
   this decision adds and asserts the states are equal. The
   byte-identity witnesses do not move.

2. **Every terminal run has an `ending`, and it is a view, never a
   status.** The ending is one word from a closed vocabulary, derived
   from the journal by the read side exactly as decision 0047 ruling 3
   derives its marks. The vocabulary:

   | ending | meaning | what it asks of the operator |
   |---|---|---|
   | `shipped` | completed, review clean | nothing |
   | `shipped-flagged` | completed, residuals passed through flagged | read the residuals |
   | `refused` | a review rule ruled a hard stop on the work | a ruling on the finding |
   | `failed` | verification or a twice-broken smith ended it | a new commission, or none |
   | `blocked` | the smith reported it could not proceed | remove the block |
   | `exhausted` | a bounded loop ran out and the ladder's terminal rung ruled | a debt ruling (0022) |
   | `superseded` | another run replaced this one | nothing; follow the citation |
   | `infrastructure` | the host, a provider or the engine failed; the work was not judged | retry when the cause is gone |
   | `delivered-elsewhere` | the work landed by hand or by another road | nothing |
   | `withdrawn` | the commission was wrong or no longer wanted | nothing |
   | `operator-stopped` | an operator stopped a run that was still working, and named no other cause | nothing |
   | `unclassified` | the journal does not say | classify it (ruling 5) |

   The vocabulary is closed and lives in one place in `brokkr-core`. A
   new ending is a new decision.

3. **A policy ending is derived from the ruling that ended the run, not
   from its rule id.** The last `transition/decided` of a policy-stopped
   run names `from` and `result`, and the result vocabulary is already
   closed. The derivation is a total function over that pair and the
   counters the ruling carried: a review ruling that stops is `refused`;
   a `fail` from verify, a `broken` that exhausts, and any `…-FAIL-TWICE`
   is `failed`; `blocked` is `blocked`; a rule whose `when` cites an
   exhausted counter is `exhausted`. `completed` splits on the severity
   of the last review ruling: `flagged` is `shipped-flagged`, otherwise
   `shipped`. Recipes invent rule ids freely; they cannot invent results,
   so no recipe can produce an ending the view cannot name. **No phase
   machine, policy table or schema changes**, and
   `policy/phase-machine.json` does not move.

   **Enforcement binding:** a sweep test derives the ending of every
   stopping and completing rule in every shipped recipe and bundle, and
   fails on a rule that lands in `unclassified`.

4. **An operator who ends a run names the cause, by type, and cites
   where there is something to cite.** `brokkr operator stop` and
   `brokkr conclude` gain `--cause`, taking exactly the operator-owned
   endings: `superseded`, `infrastructure`, `delivered-elsewhere`,
   `withdrawn`, `exhausted`, `operator-stopped`. `--reason` stays, and
   stays a sentence; the cause is what the machine reads.
   - `superseded` requires `--by-run` and takes `--by-realm`, keyed as
     decision 0026 ruling 3 keys every fleet fact. The cited run must
     exist in the journal it names; it need not be finished, because a
     re-commission is usually fired in the same minute.
   - `delivered-elsewhere` takes `--by-ref`, a commit or a pull request,
     recorded as written and not resolved.
   - `exhausted` is the operator closing a park that an `…-EXHAUSTED`
     rule opened. It accepts no risk by itself: a debt ruling is
     decision 0022's and a closed finding is decision 0047's, and
     neither is implied by this word.
   The cause rides in `args` of the `operator/commanded` event that
   already records the stop, under a payload schema
   `operator-ending.v1.schema.json` over `args` and nothing else, as
   0047 ruling 1 did. **A `--cause` is required** on both verbs from the
   release that carries this decision; omitting it is a usage error that
   names the vocabulary. The envelope is unchanged.

5. **The past is classified by name or not at all.** No ending is ever
   inferred from a reason's prose: the census in this decision's context
   did that once, by hand, to measure the problem, and its 15
   unplaceable rows are why it is not a mechanism. A terminal run whose
   operator closure carries no typed cause reads `unclassified`. The
   operator may classify it afterwards with `brokkr operator classify
   --run <run> --cause <ending> [--by-run …] --reason <text>`, which
   appends one `operator/commanded` event with `command: "classify"` to
   the finished run, takes no `operator/accepted`, and is read by the
   view and not by the fold — decision 0047's shape, for the same reason.
   A later `classify` on the same run supersedes an earlier one, and the
   view says so. Policy endings need no classification: ruling 3 derives
   them for every run already in the journal.

6. **Every read surface shows the ending, and colour says what a row is
   waiting on, never whether it was good.** An ending implies its status
   (`shipped` and `shipped-flagged` are `completed`; every other ending is
   `stopped`), so `brokkr runs` prints the ending in the status column
   for a terminal run and the status for a live one. The table gains no
   column and the feature text keeps its width; `--status` and the JSON
   keep the fold's word.

   A red row says "something went wrong", and of the endings that is
   true of exactly one. A refusal is the review gate doing its job, an
   exhausted loop ended where decision 0022 says it should, and a blocked
   smith was never judged. So tone is keyed to the third column of
   ruling 2's table:

   | tone | endings | what it says |
   |---|---|---|
   | green | `shipped`, `shipped-flagged` | done |
   | yellow | `refused`, `exhausted`, and a parked `awaiting_operator` | waiting on a ruling from the operator |
   | cyan | `blocked`, `infrastructure` | waiting on an action; the work was not judged |
   | red | `failed` | judged, and did not pass |
   | dim | `superseded`, `delivered-elsewhere`, `withdrawn`, `operator-stopped`, `unclassified` | settled, or nothing known |
   | bold | `running` | live, as today |

   `shipped-flagged` stays green because it shipped; `unclassified` is
   dim because it is an absence of information, not a warning.

   A one-character gutter carries the same distinction without colour,
   so the table reads under `NO_COLOR`: `!` a ruling is wanted, `>` an
   action is wanted, `~` shipped with residuals to read, `?`
   unclassified, a space otherwise. The header counts the two that
   matter, separately: `282 runs · 2 want a ruling · 1 wants an action`.
   The existing dim detail line under a row names the rule that ruled a
   policy ending, and the citation for `superseded` and
   `delivered-elsewhere`. A closing line tallies the listing by ending,
   which keeps the unclassified past visible until it is named.

   ```
   brokkr · 282 runs · 2 want a ruling · 1 wants an action · ./.forge/forge.db
     dsh-launch-planner-…-ed4ff1bc  running          implement  seq 1113  1h32m  …
   > issue-307-astra-as-…-8cb205dc  blocked          implement  seq 1233  1h32m  …
       IMPL-BLOCKED
   ! dsh-composite-identity-…7331e  refused          review     seq 902     2d   …
       REVIEW-SECURITY-HOLD
   ~ dsh-launch-planner-…-10abd37c  shipped-flagged  done       seq 424   3h04m  …
     dsh-launch-planner-…-3d08ce19  superseded       review     seq 1337  5h10m  …
       by dsh-launch-planner-…-10abd37c
   ```

   `--ending <word>` filters on it; `--needs-operator` lists exactly the
   `!` and `>` rows. No listing hides settled runs by default: a table
   that silently omits rows is a table nobody trusts; `--hide-settled`
   is the explicit form. `brokkr inspect` and `watch` have the room and
   show both words — `status stopped · ending superseded by <run>` —
   with the ruling's reason sentence for a policy ending. The TUI and
   the console paint the same word as a chip from the same six tone
   classes, added to the console's fixed class allowlist, and the
   console groups the fleet as *wants you*, *live*, *settled*. Muninn
   queues by the ending: a `superseded`, `withdrawn` or
   `delivered-elsewhere` run is never offered as work, and an
   `infrastructure` or `blocked` run is offered as a retry or an unblock,
   never as a failure of the seat that was in it. JSON output carries
   `ending`, `ending_cited` and `wants` (`ruling`, `action` or null)
   under a bumped `view_version`.

   **Enforcement binding:** golden tests render a listing holding every
   ending, once with colour and once under `NO_COLOR`, and a test asserts
   that tone and gutter are total functions of the ending, so a new
   ending cannot ship unpainted. Every ending, rule id and citation
   reaches the terminal through `Safe`, as every journal string does.

7. **The ledger reads endings, and says which it excluded.** Any
   comparison of seats, models or recipes drawn from the journal —
   `brokkr compare`, Muninn's advice, issue #271's corpus — counts
   `shipped`, `shipped-flagged`, `refused`, `failed` and `exhausted` as
   outcomes of the work. It excludes `blocked`, `infrastructure`,
   `superseded`, `withdrawn`, `delivered-elsewhere` and
   `operator-stopped` from any success rate, because in none of them was
   the work judged to its end, and it prints the count it excluded. An
   `unclassified` run is excluded and counted as such. A `refused` run is
   read both ways: against the seat whose work was refused, and for the
   gate that raised the finding — a security hold is the judge's result
   as much as the smith's.

## What this decision does not do

It does not rename `stopped`, add a status, or add an event type. It
does not make an ending a ground for any rule in the phase machine: a
policy reads results and counters, never the view. It does not close a
finding (0047) or accept a risk (0022). It does not classify the
existing journal; ruling 5 gives the operator the verb and leaves the
87 to them.

## Consequences

The fleet view answers the question the operator actually asks. Of the
149 stopped runs counted here, 62 gain an exact ending the day this
ships, with no operator action, because the journal already holds the
ruling that ended them. The other 87 read `unclassified` until named,
which is honest: the journal does not say.

`conclude` and `stop` get slightly harder to type and much easier to
read back. The cost is one required flag; the alternative measured here
is a sixth of operator closures that nobody, including their author,
can place a month later.

The journal becomes usable as the corpus issue #271 wants. A smith is
no longer charged for a run that was replaced, and a recipe is no longer
credited for one that shipped its residuals.

Scratch and experimental recipes cost nothing: they cannot invent a
result, so they cannot produce an ending the view lacks a word for.

Amends the read surfaces of decisions 0026 and 0047 in part: both gain
a column. Extends decision 0047's operator-annotation shape to a second
command. Decisions 0022 and 0029 are unchanged; `conclude` stays fenced.

## Evidence

The census is reproducible from the journal alone: for every run whose
last terminal event is `run/stopped`, read its `reason` (`hard stop
ruled by <RULE>` names a policy stop), whether an `operator/commanded`
stop precedes it, and whether a `run/parked` precedes that. The
controller's scripts that produced the counts above, and the model and
cost comparison that first tripped on the ambiguity, are kept on the
operator's host under the-forge's `.forge/tools/`
(`journal-model-ledger.py`, `journal-economics.py`); the counts are
dated 2026-09-21 and will drift as the journal grows.
