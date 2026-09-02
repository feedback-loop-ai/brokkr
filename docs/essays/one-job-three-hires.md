# One Job, Three Hires

A phase of the software lifecycle executed by a model is a job, in the ordinary sense of the word. A developer does implementation; a reviewer reviews; a release engineer ships. When people hold those jobs we write a job description for each, put a price tag on it, and never confuse the job with the person currently in it. I see no reason to treat the same phases differently when a model holds them. The charter is the job description, the crew is the hire, the rate card is the salary; and what a delivery costs is set by how the jobs are cut and described far more than by who is hired into one of them. I would put it more strongly: designing those jobs — their descriptions, their prices, and the graph of who hands what to whom and when work goes back — is what engineering management now is. I ran the same commission under three crews to see whether that holds, and the journal says it does — with two confessions from the referee's side that belong in the record beside the numbers.

The commission came out of a wound. When Brokkr adopted its first
foreign repository, the scaffold it wrote handed the seats an empty
tool map; they could not run the project's own build and test
commands, and the run stopped with the honest word `blocked`. So the
commission was: `brokkr init` scaffolds, per detected stack, the tool
grants each seat's charter actually needs — the runner plus `git`,
`ls`, `rg`, `mkdir` for working seats, the read-only subset for judging
seats, an empty map and a README that says so when the stack is
unknown — and proves it by compiling the scaffold under its own
adapters.

That is a job with a description. The implementer seat's charter reads like one: what it may touch, what it must prove, what it must not do, and gates it does not control. Three crews were hired into that one job. They got the text byte for byte from the same base commit, every recipe digest recorded before anyone ran, and the recipe that differed between them differed by one line: the implement seat's driver. Thirty-four lines against a hundred and nine, one of them the experiment. The other hundred and eight — the phase table, the verify and review gates, the reforging ladder, the limits — were the same for everyone, and the judging jobs were held by the same model throughout.

## Why one line was enough

None of that is luck. It is the first law from [The Model Is a
Detail](the-model-is-a-detail.md): determinism belongs in control
flow, stochastic execution belongs in leaf effects. A crew is a leaf
effect. The phase machine is data (decision 0002), the machine never
lets a model repair its own control plane (decision 0001), every crew
speaks one driver protocol to one native runtime (decision 0003), and
an adapter is a data file that maps an abstract model name to a
concrete one and says what the provider cannot express (decision
0016). Recipes extend and override each other as data (decision 0017),
which is why a wager is a thirty-four-line file and not a fork. Swap
the leaf and nothing above it moves: the gates, the limits, the
journal, the digests all stay put, and so the comparison is between
crews and nothing else. Most agent stacks cannot make that swap at
all, because the model is wired into the control flow. Here the model
is a detail, which is the only reason its price is a choice.

## What the table says about the phases

| | Claude | DeepSeek | Codex |
|---|---|---|---|
| Model served | claude-fable-5-1 — the account default; `fast` pins none | deepseek-v4-flash, official API | gpt-5.6-sol, codex config default |
| Recipe · run | `fast` · 4dd3b998 | `wager-harness-dsh` · 11189cf7 | `wager-harness` · 919e2e14 |
| Cycle time, start to end | 33.2 min, stopped at review | 30.2 min, shipped | 36.8 min, shipped |
| Implement seat time | 20.2 min | 22.1 min | 27.1 min |
| Implement tokens: input miss | 7,424 | 199,460 | 260,574 |
| cache write | 254,202 | n/a, priced as miss | not reported |
| cache read | 9,970,604 | 38,227,328 | 10,482,688 |
| output | 88,600 | 140,408 (68,556 reasoning) | 41,607 |
| Cache miss share | 2.6% | 0.5% | 2.4% |
| Implement, factual charge | subscription, $0 marginal | **$0.81**, metered | subscription, $0 marginal |
| Implement, counterfactual at list | $12.08 | $0.81 | $6.07 |
| Same tokens, DeepSeek off-peak | — | $0.41 | — |
| Gates (all Fable 5.1), at list | $8.70 | $6.55 | $7.42 |
| Whole run at list | $20.78 | $7.36 | $13.49 |
| Verify | pass ×2 | pass, 852 tests | pass |
| Review | clean, then **high** — hard stop | **medium**, carried | **medium**, carried |
| README at `init .` | overwrote the project's | overwrote the project's | refused to write |
| Grant derivation | first word of each command | closed tool table; unknown runner refused | one runner per row |
| Tests added | 4 | 14 | 2 |
| Diff | +745 / −127, 11 files | +1525 / −254, 16 files | +701 / −206, 16 files |
| Artifact, pinned | [`wager/2026-09-02/claude`](https://github.com/feedback-loop-ai/brokkr/tree/wager/2026-09-02/claude) at 960f3f2 | [`wager/2026-09-02/deepseek`](https://github.com/feedback-loop-ai/brokkr/tree/wager/2026-09-02/deepseek) at 2ab21bb; **on main via #137** | [`wager/2026-09-02/codex`](https://github.com/feedback-loop-ai/brokkr/tree/wager/2026-09-02/codex) at 045e0d6 |

Every branch starts from the same base, [`wager/2026-09-02/base`](https://github.com/feedback-loop-ai/brokkr/tree/wager/2026-09-02/base) at a78ecc6, and the rigged first DeepSeek round is kept beside them as [`wager/2026-09-02/deepseek-round-1`](https://github.com/feedback-loop-ai/brokkr/tree/wager/2026-09-02/deepseek-round-1) at 6ac24b0, committed at the bench exactly as the seat left it. The DeepSeek branch carries the shipped change rebased onto main; the run's journal names the pre-rebase commit, b347be8, which the squash merge retired.

List prices used, USD per million tokens:

| Model | Input | Cache write | Cache read | Output | Source |
|---|---|---|---|---|---|
| Claude Fable 5.1 | 10 | 20 (1h) | 0.25 | 50 | Anthropic's pricing page; Claude Code prices at one-hour writes, list basis |
| DeepSeek V4 Flash, peak | 0.44 | priced as miss | 0.014 | 1.32 | DeepSeek's pricing page; both rounds ran inside the 06:00–10:00 UTC peak |
| GPT-5.6 Sol | 4 | n/a | 0.40 | 20 | OpenAI's pricing page, standard tier; promotional through 21 November 2026 |

Read the two cost rows against each other. The gates cost between
$6.55 and $8.70 for every crew — a near-constant, because the judging
phases ran on the same model with the same charters whatever the crew
did. The implement seat is where the crew choice bit: $12.08, $6.07,
$0.81. Same phase, same bound, fifteen to one at list; both DeepSeek
rounds happened to run inside the provider's weekday peak window, when
every rate is doubled, so off-peak the same tokens cost $0.41 and it
becomes thirty to one. Two honesties ride with that: the Claude crew
was not on Opus, because our standard recipe pins no model and the
account's default served; and the ratio is a counterfactual, since the
Claude and Codex seats ran on subscriptions whose marginal price is
zero and whose flat fee is a separate ledger. The list column is the
one that compares like with like.

Read the token rows too, because the DeepSeek seat's price is not only
its rate card. It consumed the most tokens of the three — 38.6 million
against about 10.3 and 10.8 million — and paid the least, because it
missed its cache five times less often: half a percent of its input
was uncached, against two and a half percent for the other two. The
harness keeps its context stable enough that almost every request is a
re-read, and DeepSeek prices a re-read at about three percent of a
miss. Aggressive pricing multiplied by disciplined caching is what the
$0.81 is made of; more tokens, fewer misses, a lower bill.

So the shape of a cost-efficient run is visible in a few lines. The working job is the elastic one: describe it tightly enough and a hire fifteen to thirty times cheaper does the work at least as well. The judging jobs are the fixed cost, and they should be, because they are where a wrong answer costs more than any seat. What you pay for a delivery is mostly a function of how the jobs are cut and described, not of whose logo is on the working seat.

Put it in hiring terms. Paying Fable or Sol to do the same job as DeepSeek, or a slightly worse one, at fifteen to thirty times the rate is hiring a San Francisco engineer with an arguably slightly better skill set over a top developer in China for the same job, at thirty times the price. Every engineering organisation has faced that decision, and the ones that make it well make it by reading the job description and the price tag first, not the passport.

Which is why I think the management job has moved without changing its nature. Running an engineering organisation always meant writing the job descriptions, pricing the roles, drawing the chart of who hands work to whom, and deciding when work goes back. Running one that works through crews means exactly that, one level of abstraction up: the job descriptions are charters that compile, the chart is a phase table, the price is a rate card, and the minutes are a journal. The hire in each seat is a leaf. The seat is the job. The crew is the hire. The graph is the work.

## The gates did the quality

All three crews shipped the feature. All three compiled their scaffold
under its own adapters, asserted the argv, handled the unknown stack
honestly. And all three wrote the scaffold's README where `brokkr init
.` runs, over a project's own README. The crews differed in taste —
one derived grants by splitting the first word off each command, one
built a closed table that refuses to invent a runner, one kept a single
runner per row — and in how much proof they attached: four tests,
fourteen, two. The defect they shared was caught by the phase that
exists to catch it, for every crew, at no extra cost to the commission.

That phase also explains the one thing in the table that looks like a
crew difference and is not. The Claude crew's run took two review
rounds and the others took one, not because the Claude crew chose a
longer road but because its first reviewer fixed a wording issue
itself and returned `clean`. The phase table treats a judge that has
picked up the hammer as no longer a judge of that tree, sends the run
back through verify, and seats a fresh reviewer. That second reviewer
read the README defect as high and stopped the run. The rule cost about
five dollars and bought a second opinion. That is what a bounded
judging phase is for.

## Two confessions from the referee's side

The first is rigging, again. Round one of the DeepSeek crew implemented the whole feature, reported the workspace green, and then answered `blocked` — on `git commit`. The fire had been lit in a git worktree, the way every fire on this bench is lit; a worktree keeps its metadata under the shared repository's `.git`, and the DeepSeek harness confines writes to the seat's own directory. The Claude crew's sandbox never noticed. The DeepSeek crew would not report complete with uncommitted work, which is the charter's correct answer. Same remedy as last time: the blocked run
stays in the journal, labelled, its artifact committed at the bench so
it can be judged; the rematch ran in a standalone clone, and only the
rematch counted. Last time the rigging was a sandbox default of mine that I had not questioned. This time it was a default of the bench that I had not questioned either. Either way the referee's side is two for two on rigging round one against the newcomer, and two for two on the record surviving to say so.

The second is the sentence. The judge was the same model for every
crew, by design, so the comparison measures the crews and not the
referee — and the same reviewer seat, in separate sessions, reading the
same defect in the same file, wrote "high" once and "medium" once. The
third sentence, on the Codex crew's refusal to overwrite, is at least
defensible, since refusing loses no data. But the spread between the
first two is not explained by anything in the diffs. The referee's
severity calibration is a residual of its own, and it is now in the
record next to the rigging.

## How I know the numbers

The Claude seats journal their dollars but not their tokens, and the
DeepSeek seat journals nothing at all; the provider's API offers a
balance and no history. So every seat was priced from the transcript the journal's session id names, at the published rates, and I checked the reconciliation myself. The Claude sums reproduce Claude Code's
own dollar figures to the cent, seat by seat, which is how I know the
rate table and the deduplication are right. The DeepSeek sums — 191
requests in the rematch, 241 in the rigged round, two one-word smoke
tests — come to $2.08, which is the figure the console shows. I trust a
number I can reconstruct from two independent sources more than I trust
either alone.

## What the tier question became

Decision 0021 says a driver's tier is earned by evidence and a
newcomer's first outings are wagers. People read that as a promotion
ladder. The wager measured a working seat; gates stayed on Claude for
all three crews, by construction. So the evidence says one thing: for
a commission of this size, the DeepSeek lane holds a bounded work seat
at least as well as the Claude crew did, at a fraction of the list
price. It says nothing about judging, and no ruling followed, because
none was needed — the lane was already lawful on work seats and the
overnight recipe already sits on it. The tier question got boring,
which is the point of tiers as data instead of opinions.

What the machine owed afterward was plumbing, not a promotion: a
DeepSeek seat that speaks every turn to the journal, so the next
wager's cost is in the record instead of reconstructed after. That fire
lit the same afternoon and parked on a real question about whether a
seat's transcript belongs to the operator or to the void. That one is
mine to answer, in the journal, where the machine put it.

*Three crews, one commission, one referee. The working job was described tightly enough that the hire became a price; the judging jobs cost the same whoever worked, and caught the same defect for everyone. The DeepSeek artifact is on main. All three, and the rigged round, are on branches under `wager/2026-09-02/`, labelled, for anyone who wants to disagree with me by reading rather than by guessing.*