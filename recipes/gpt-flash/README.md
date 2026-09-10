# GPT boss / Flash implementer

Extends `triage` with full specification routing and its existing verification,
shipping, retry, and escalation policy.

| Responsibility | Model |
| --- | --- |
| Triage, specification, clarification, design chief, tasks, analysis | GPT Sol |
| Implementation (chore, feature, design, engine) | DeepSeek Flash 4.1 |
| Final review chief (every strategy) | GPT Astra |
| Design and review panels | GPT and Flash, preserving vendor diversity |
| Verification and shipping | Inherited deterministic gates |

All strategies use a correctness/security panel before Astra; design and engine
retain their additional specialist reviewers. Astra cannot lower the panel verdict.

Implementation retains each strategy's existing charter. Flash uses the
`flash-experiment` adapter alias, currently
`deepseek-v4.1-flash-expires-on-0910`. This is an expiring experimental endpoint;
refresh the adapter mapping when a replacement is available. No fallback to
Flash 4.0 is configured. The scoped `gpt-flash-*` offices are proposed by
decision 0058: each reuses a library charter and pins exactly one model, so the
mandated crew is forced and no fallback is hired.

DSH is an untrusted work provider: it cannot judge gates. Its adapter does not
support workspace hands or named tool filtering; the Flash agents therefore
declare neither. GPT agents retain their existing workspace restrictions. Flash panel positions
are work seats; Astra alone makes the final review decision after the panel.
No Claude model is hired by this recipe.

Select with `--recipe gpt-flash` when starting a run from this library. Compiling
the recipe validates configuration without making a provider request:

```sh
brokkr compile --bundle recipes/gpt-flash
```
