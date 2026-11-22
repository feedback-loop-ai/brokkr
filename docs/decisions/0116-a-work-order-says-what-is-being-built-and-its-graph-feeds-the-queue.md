# 0116 — A work order says what is being built, and its graph feeds the queue

Status: proposed — the operator's direction in chat, 2026-11-22: "we should have a workorder abstraction - like future [commissions], they can have a reference to external tools, e.g. github, gitlab, azure devops, linear, zenhub, jira etc, but essentially it is a DAG of future [commissions] distilled so we know what we are building"
Date: 2026-11-22

Built: unbuilt (#1272)

## Context

Brokkr keeps two graphs today, and neither is the work.

- **Inside a run,** decision 0067 makes the plan the schedule: a task graph that implement runs in bounded visits. It is accepted and unbuilt (#463).
- **Between runs,** decision 0068's queue holds runs waiting to start. Its entries carry priorities and waits, and admission starts each admitted entry while room lasts. It is built.
- **Nothing holds the work itself:** what is being built, what it needs first, what the operator ruled about it, and when it is done. Those facts live in issue comments, in commissions written by hand, and in the controller's head.

The agents-server lane on 2026-11-21 and 2026-11-22 shows what that costs.

- **#1191 was one piece of work and four runs:** story 1191a, fix rounds 1191a2 and 1191a3, and landing 1191aL. The journal holds four unrelated runs. The two rulings that shaped them are comments on #1191. "Done" was PR #1254 merging, which no run records.
- **#1192 has one set of rulings, the public-API caps and the merge wait, that spans two stories** (1192a and 1192b) on one branch. Each story's commission has to copy them by hand.
- **Commissions written ahead go stale.** Story 353ac's commission was written against one base, and the CLI nesting (decision 0113) changed a text it named before the story fired. Story 1192a's commission was written stacked on an open pull request that merged an hour later. A commission mixes two kinds of fact. One kind is stable: the goal, the rulings, the acceptance, what not to touch and the open asks. The other belongs to a base: the commit, the paths and lines, the contention, the known flakes.
- **Dependencies exist before anything is queued.** #1196 asks whether its graph spans stories not yet queued, and #1264's drawing has nowhere to show them.

The operator also named the aim:

> "if we get that right, we decouple the workhorses of development (LLM clients) and the workhorses of project [management] (the jiras of this world). IT all becomes abstract and interchangeable in the brokkr engine."

Brokkr already abstracts the first side. A harness is an adapter and a driver, and #347 wants a new harness to cost one module plus its adapter data. This decision gives the second side the same shape. A tracker is a citation and an adapter, and the work order sits between the two sides, journaled.

## Rulings (proposed)

1. **A work order is one typed document: what is being built.** It holds:
   - **Brokkr's own id,** unique in its realm, and a title. A tracker's item is never the id: it is cited, with its link (ruling 7);
   - the **intent**, the distilled statement of what is being built and why;
   - the **acceptance**, as statements a review can check;
   - the **rulings** the operator made about it, each with its words, its date and a citation;
   - the **asks** still open, each closed by a ruling;
   - the **constraints**, such as what must not be touched and the public-API caps per crate;
   - its **references** to external items (ruling 7);
   - an optional **parent** (ruling 6), and its **edges** (ruling 3);
   - its **state** (ruling 5).

   One contract, `contracts/work-order.v1.schema.json`, defines it. The document is the same whether it is read from a file or a row.

2. **Storage is not the definition.** A work order may be authored as a file in the realm, reviewed like any other change, or held as a row in the journal. Both are read through one parser against the one schema. Every change to a work order is journaled as an event: proposed, ruled, edited, queued, closed. What was decided therefore stays replayable, whatever holds the document. A later Postgres store keeps the document as a native `jsonb` object under the same schema. The operator, 2026-11-22: "yes, could be files, or could be entries in the db, going forward (and assuming we will move to pg with jsonb as native objects eventually)".

   **A work order authored as a file is Markdown:** a typed header validated against the schema, with the intent and acceptance as its body. It lives in the realm's work-order directory, which is named after the verbs' root (ask 1). The operator, 2026-11-23: "Markdown".

3. **Edges belong to work orders, and the queue's waits are derived from them.**
   - A work order waits on another with a condition: `completed`, `ended`, a named ending of decision 0064 (#1192), or `merged`, recorded by the journaled operator command #1192 builds.
   - The edges form a directed acyclic graph, and a write that would close a cycle is refused.
   - When a work order is queued, its entry's waits are derived from its edges. When an edge changes, the waits of a waiting entry change with it, as #1196 lets edges change.
   - The operator may still add a wait to an entry by hand, journaled.
   - Admission reads only the waits stored on entries, as decision 0068 ruling 3 requires. It never reads a work order.

4. **A commission is rendered from a work order when it is queued, never stored ahead.**
   - The rendering joins the work order's stable parts with the base's facts at that moment: the commit, the paths and lines, the contention and the known flakes.
   - A seat may do the rendering. Scoping a story is that seat's job.
   - The rendered commission is journaled with its queue entry, together with the work order's id and digest, so every run says which version of which work order it served.

5. **A work order outlives its runs.**
   - Every run that serves it cites it: the story, its fix rounds and its landings.
   - Its state moves proposed → ruled (no open asks) → queued → in progress → shipped → merged. It may also end as withdrawn, superseded or delivered-elsewhere, in decision 0064's words.
   - **A work order with an open ask cannot be queued.**
   - **Done means merged, not shipped:** a shipped run whose pull request has not merged leaves its work order in progress.

6. **Work orders nest, for grouping only.**
   - A parent groups its children: an epic, its work orders, their slices.
   - A parent's state rolls up from its children: it is merged when every child is merged or has ended without needing to be.
   - Edges may join any two work orders, across parents. The tree groups, and the graph orders.

7. **A tracker is cited, never read at admission.**
   - A reference names a tracker and an item in it: a GitHub issue or pull request, a GitLab issue or merge request, an Azure DevOps work item, a Linear issue, a Jira issue, a ZenHub item.
   - **Each reference carries the item's link,** so a reader can follow it without an adapter. The work order keeps Brokkr's own id (ruling 1). The operator, 2026-11-23: "Brokkr's own id with tracker's items cited with a link".
   - The trackers' vocabulary is closed and grows by adapter data, the way harnesses do (#347), never by an edit to the engine.
   - Admission, the dispatch pass and the fold never read a tracker.
   - Moving data in or out is a tool. An **import** reads a tracker and writes journaled work-order commands. An **export** posts a work order's state back.
   - The import comes first, and the export later. The operator, 2026-11-22: "yes".
   - Decision 0064's `delivered-elsewhere` citation is the same idea at a run's end.

8. **#1196's analysis produces work orders, and #1264 draws them.**
   - The analysis seat reads issues, commissions and task breakdowns, and proposes work orders and their edges. Its output is journaled as one command, and is trusted, as #1196 says.
   - #1264's view draws work orders. A queued one is drawn solid, with its entry's state. A future one is drawn dotted, in the column it would start in.
   - The operator, 2026-11-22: "yes to both".

## Slices

1. **The document and its store:** the contract, the parser for files and for rows, the journal's events, and verbs to add, show, list and edit a work order.
2. **The graph:** edges with their conditions, cycle refusal, queuing a work order with its waits derived, and the rendered commission journaled with the entry.
3. **The analysis and the drawing:** #1196 produces work orders, and #1264 draws them.
4. **The trackers:** an import adapter, GitHub's first, then the export.

Each slice is its own unit, with its own API cap ruled before it is commissioned.

## Consequences

- **The queue's admission does not change.** It gains a source for its waits, and still reads only what is stored on its entries.
- **Commissions stop going stale in a feed,** because they are written when their base is known.
- **Rulings become data.** A fix round inherits its story's caps and answered asks from the work order, where today the controller copies them into each commission by hand.
- **Neither side is privileged.** The work order is the same whichever tracker it was cited from and whichever harness serves its runs.
- **The cost:** a new contract, a new store table with a storage version, a new family of verbs, and an import adapter per tracker.

## Asks (operator)

1. **The verbs' root.** Decision 0113 holds the CLI at twelve roots. Where do work orders' verbs live, and what is the work-order directory called?

The id and the file form were asked here and ruled on 2026-11-23. They are now part of rulings 1, 2 and 7.
