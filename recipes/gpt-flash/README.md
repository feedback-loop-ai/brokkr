# GPT boss / Flash implementer

Extends `triage` with full specification routing and its existing verification,
shipping, retry, and escalation policy.

| Responsibility | Model |
| --- | --- |
| Triage, specification, clarification, design chief, tasks, analysis | GPT Sol |
| Implementation (chore, feature, design, engine) | DeepSeek Flash 4.1 |
| Final review chief (every strategy) | GPT Sol, with GPT Astra as its last fallback |
| Design and review panels | GPT and Flash, preserving vendor diversity |
| Verification and shipping | Inherited deterministic gates |

All strategies use a correctness/security panel before the chief; design and engine
retain their additional specialist reviewers. The chief cannot lower the panel verdict.

Implementation retains each strategy's existing charter. Flash uses the
`flash` adapter alias, which pins `deepseek-flash`: the name DeepSeek's
[pricing page](https://api-docs.deepseek.com/quick_start/pricing/) gives
DeepSeek-V4.1-Flash. The beta id the recipe first pinned
(`deepseek-v4.1-flash-expires-on-0910`) has expired, and the older
`deepseek-v4-flash` is retired; DeepSeek serves both as `deepseek-flash`. No
fallback lane is configured. The scoped `gpt-flash-*` offices are proposed by
decision 0058: each reuses a library charter and pins exactly one model, so the
mandated crew is forced and no fallback is hired. The one exception is the
final review chief, which chains Sol at `high` and then Astra at `max`
(decision 0045's addendum of 2026-09-30). Both are Codex models.

DSH is an untrusted work provider: it cannot judge gates. Its adapter does not
support workspace hands or named tool filtering; the Flash agents therefore
declare neither. Seven of the nine GPT agents declare workspace hands, which
box their commands only under a `namespace` boundary; `gpt-flash-triage` and
`gpt-flash-position-robustness` declare none, so Brokkr adds no box, sandbox
fragment or tool flag to their command. Flash panel positions
are work seats; the chief alone makes the final review decision after the panel.
No Claude model is hired by this recipe.

Select with `--recipe gpt-flash` when starting a run from this library. Compiling
the recipe validates configuration without making a provider request:

```sh
brokkr compile --bundle recipes/gpt-flash
```
