# 0031 — Does the Agent's Map Match the Territory? Architectural Claims in AI Agent Context Files and Their Fit with the Code

Source: https://arxiv.org/abs/2611.29916
Authors: Bachan Ghimire, Daniela Damian (University of Victoria)
Read: 2026-11-24
Status: proposed
Intake: research sweep, run weekly-research-sweep-registry-d-5feb3fa3, 2026-11-24

## Summary

A coding agent starts each session with no memory of the system's
architecture and takes what its context file says (`CLAUDE.md`,
AGENTS.md, Copilot instructions) as true. The paper asks whether
those files match the code, for the two kinds of architectural claim
that can be checked: path claims, which name a folder or file, and
dependency rules, which say what may import what. From 36,710
engineered GitHub projects it takes 3,602 context files in 3,346
projects. Path claims are extracted by rule and checked against the
file tree at the snapshot and then commit by commit, with READMEs as a
baseline; dependency rules are found with an LLM, reviewed by hand and
each turned into a static import checker (reviewer agreement kappa 0.89
to 0.94).

The maps are good. Of 41,090 path claims about 5 percent point at a
path that cannot be found, but the estimated share that are real errors
is 2.2 percent, against 3.5 percent for READMEs. Of claims valid when
written, 2.6 percent later went stale, at 0.99 per 100 claim-months
against 1.74 for READMEs; 61 percent of stale claims were fixed, most
in the same commit that broke them, and the unfixed ones had a median
age of 59 days. Stale claims are fixed in 73 percent of cases in
AGENTS.md, 59 in CLAUDE.md and 43 in Copilot instruction files.

The rulebooks are less reliable. Of 184 dependency rules from 115
projects, 33 (18 percent) were confirmed broken on the day they were
written, usually by older code, and nearly all stayed broken: the rule
described an intended architecture, not the existing one. In 7,480
commits across 96 projects, agent-assisted commits broke a rule about
1.8 times as often as others (0.29 against 0.16 percent), on 14 breaks
in all and not significant (within-project odds ratio 2.6, interval 0.7
to 10.2).

Limits stated: about half of rule sentences could not be checked;
import resolution differs by language and bundler configuration, so
breaks are lower bounds; agent involvement is read from visible traces; a
median window of 67 days per file; open source only. The figures past
the abstract, and the recommendations, were read from a fetch summary
of the paper's body, not verbatim text.

## Findings

| # | Finding | Classification | Citation |
|---|---|---|---|
| 1 | Check every path a context or architecture document names against the tree, mechanically | implemented | `crates/brokkr-cli/tests/doc_truth.rs` and `crates/brokkr-cli/tests/research_registry.rs`: every repository path a code span names in a living document must be in the tree, a span the rule cannot read is refused rather than skipped, and a registry citation's paths must resolve |
| 2 | Fix a stale claim in the commit that makes it stale: what is not fixed the same day lingers for months | alternative | `crates/brokkr-cli/tests/doc_truth.rs`: the path rule runs in the suite every change must pass, so the commit that moves a file fails until the documents that name it move too; stated Brokkr versions, decisions called proposed and retired terms are checked the same way, other prose claims are not, and registry entries 0016 and 0022 carried stale prose claims for weeks |
| 3 | Check a written architecture rule against the code on the day it is written: 18 percent are already broken on the day they are written, mostly by older code | implemented | decision 0071; `quality/README.md` and `quality/ratchet.sh`: the principles were ruled from a measured sweep, each gated ceiling carries the offenders counted at the time as a baseline, and the ratchet fails a change that adds to them |
| 4 | Say which rules describe a goal and which describe the code as it stands | implemented | decision 0071; `docs/decisions/README.md` and `crates/brokkr-cli/tests/decisions_index.rs`: each decision's header says built, partial or unbuilt with the issue that carries the rest, a test holds the index to the headers, and the house rules name a target form as a target |
| 5 | Enforce the rules that matter in a linter or CI gate rather than in prose, with known exceptions listed | implemented | decision 0071; `crates/brokkr-core/clippy.toml`, `quality/suppressions.txt` and `quality/ceilings.json`: ruling 11 says which principle a deterministic gate holds and which a charter judges, the pure core's import rule is a lint that names every effectful call it refuses, and today's exceptions are a listed baseline that may only shrink |
| 6 | Track whether agent-written commits break written architecture rules more often than others | not-planned | |

## Candidates

This entry mostly confirms. Brokkr's doc-truth test, decision ledger
and quality ratchets are the paper's recommendations already built,
and the paper's two numbers are the argument for having built them: a
written rule is broken on arrival about one time in five, and a stale
claim not fixed in the breaking commit lasts a median of 59 days.

The paper's checkable rules are import rules, and Brokkr's nearest is
decision 0071 ruling 1, the pure core. It is held the way the paper
recommends: `crates/brokkr-core/clippy.toml` and its twin for the view
crate disallow the effectful standard-library surface by name, and
`crates/brokkr-cli/tests/layering/` scans the crates. Nothing to take
up there. What the paper adds is the caution in finding 4 for the
rulings that are still judged and not gated: about half the rule
sentences it read could not be checked at all, and those are the ones
that drift unseen.

Finding 6 is weak evidence (14 breaks, not significant) and every
commit here is agent-written, so the comparison does not transfer. The
transferable form is a count per run of findings that cite a decision
0071 ruling number, which the judges' rating table already asks them to
write.
