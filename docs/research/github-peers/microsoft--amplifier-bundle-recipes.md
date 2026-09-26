# microsoft/amplifier-bundle-recipes: implementation dossier

Research date: 2026-09-27 (Europe/Sofia). Live metadata retrieved 2026-09-26T22:00:45.928939+00:00. Default branch: `main`. Revision: [`f8ec2ec8190ec55309cae784a98a1f7bfcdd1d63`](https://github.com/microsoft/amplifier-bundle-recipes/tree/f8ec2ec8190ec55309cae784a98a1f7bfcdd1d63); commit date: 2026-09-21T10:24:47Z. Repository license: [MIT](https://github.com/microsoft/amplifier-bundle-recipes/blob/f8ec2ec8190ec55309cae784a98a1f7bfcdd1d63/LICENSE), recorded without a legal interpretation.

[Inspection method and limits](../0019-github-peer-implementation-investigations.md#method-and-limits) apply to this dossier.

## What the project establishes

Recipes is a hybrid Amplifier bundle and Python runner library/CLI. Its strongest overlap with Brokkr is reusable agents drawn from declared dependency closure, executable recipe data, lock/provenance utilities, approval pauses and resume. The deeper investigation substantially qualifies the earlier survey: several available trust mechanisms are not visibly connected to the standard execution path, and library callers carry responsibilities that the CLI handles separately. [Package surface][R1], [execution API][R2].

The flow is: parse manifest/recipe → precheck all declared sources → resolve bundles and construct a closed agent catalog → construct an execution plan → compose a narrowed Foundation session → execute ordered steps → persist progress/approval state. The CLI adds lock verification, run-manifest recording and resume provenance checks around that flow. There remain two engines: standalone `StepEngine` and legacy in-session `RecipeExecutor`; the parity document says v2 in-session calls still use the legacy engine with the closed catalog. The package-level description of one execution home must not erase that actual split. [Planner][R3], [parity matrix][R4].

## Recipe and refusal semantics

Recipes support agent, bash and subrecipe steps; conditions, foreach and parallel iterations, while/convergence loops, staged approvals, retries and timeout templates. Declaration order is execution order; `depends_on` is advisory rather than a DAG scheduler. Undefined substitutions fail by name. Unknown step/stage keys and unsupported nested step bodies have explicit refusal paths. Agent resolution refuses canonical-name collisions and ambiguous bare names, while deduplicating the identical agent reached through a shared include. It refuses the ambient `self` pseudo-agent even when hidden through an alias. These are valuable controls against silently borrowing host context or ignoring declared work. [Planner][R3], [self refusal tests][R5], [unknown-key tests][R6].

An important positive test uses a backend that raises `RuntimeError("No providers available")`: the first step is not marked complete, a later failure retains only genuinely completed preceding steps, and the run reports FAILED with the error at top level. This is stronger evidence than a happy-path completion assertion. Configurable `on_error: continue` can absorb a failed result, while `skip_remaining` can end successfully early; those are intentional recipe semantics, not protected mandatory review. Approval gates pause when no decision is available. [Failure tests][R7], [engine][R8].

## Trust: implemented APIs versus connected enforcement

`TrustPolicy.check_source` checks schemes, remote host allowlists, allowed local roots and immutable ref syntax. The planner calls it for **every** declared dependency before handing any dependency to the resolver, so a refusal of dependency two precedes fetching dependency one. Empty allowlists mean no access, unlike `None`, and CI posture demands full 40/64-character revisions. These are concrete preflight controls with targeted tests for path traversal, shared-prefix roots, credential-bearing transports and floating refs. [Trust policy][R9], [trust tests][R10].

The same module exposes `check_resolved`, `check_dependency_install`, and three-way capability intersection. The inspected planner and standard session-factory paths do not invoke the former two methods. This is a call-path observation; callers outside those paths were not ruled out. The planner records the capability intersection, but the inspected engine/execution path does not consume `plan.policy.capabilities` to authorize bash or agent tools. Consequently the existence and unit tests of these methods do **not** establish a complete installed-dependency or runtime capability boundary. [Trust API][R9], [planner hook][R3], [execution][R2].

This is reinforced by the actual session factory: `FoundationSessionFactory` defaults `install_deps=True` and calls `bundle.prepare(install_deps=self._install_deps)` without consulting the trust policy. CI policy's `allow_dependency_install=False` is therefore not demonstrated as an enforced standard-factory constraint. A custom host can supply stronger controls, but the comparison should not credit those hypothetical controls to the default path. The root dependency itself tracks Foundation `@main`, and CI module/conformance jobs explicitly install Core/Foundation from main; run-lock semantics do not make the toolchain build reproducible. [Factory][R11], [package][R1], [CI][R12].

## Locking and provenance limits

Lock utilities implement locked, update-lock and unlocked modes, strict sidecar parsing, and dependency identity comparison. The CLI verifies the lock against a plan, records provenance, and then calls library `run`, which replans internally rather than executing that exact already-verified plan. The inspected code does not compare the second plan to the first inside ordinary library run. That is a concrete candidate for a resolution drift experiment, not proof that every real resolver can be induced to drift. Library consumers should not assume setting a `lock_mode` field invokes the CLI's lock gate. [Lock implementation][R13], [CLI run sequence][R14], [library run][R2].

Resume provenance compares the recipe digest and added, removed or changed dependency identities. Dependency identity prefers a resolved revision over a content digest when both exist. Provider source/model are recorded but explicitly not compared on resume; recorded runner/Foundation versions and policy are not checked by this comparison function either. Library `resume` explicitly delegates provenance checking to its caller; the CLI invokes it. Thus “pinned run identity” is accurate only with the precise entrypoint and compared fields stated. No general cryptographic authentication of the author or model output follows from the manifest. [Provenance comparison][R15], [library contract][R2].

## Recovery, process lifetime and uncertain effects

Foreach checkpoints store completed item indices/results and preserve input order. The tests append to a file to distinguish genuinely skipped work from re-execution that happens to return the same output. They cover partial parallel completion, continued failures occupying their slot, and subrecipes not overwriting parent state. This is an excellent experiment pattern for Brokkr #434. It still establishes skipping acknowledged completed iterations, not exactly-once external effects. A crash after a side effect but before its checkpoint leaves the usual uncertainty window. [Foreach tests][R16].

Persistence is weaker than Foundation's separate shared-session store. `RunStateStore.save` directly overwrites JSON and catches `OSError`; `load` treats malformed/unreadable state as no state. `StepEngine._write_checkpoint` also catches checkpoint exceptions and continues, emitting a failure event. The tests explicitly preserve successful execution when a checkpoint hook fails. Therefore successful run completion is not a durable-checkpoint acknowledgement, and operators need to see the distinction. There is no established append-only hash chain or pure fold that reconstructs the engine from canonical events. [State store][R17], [checkpoint path][R8], [tests][R16].

The bash implementation inherits `os.environ`, substitutes context into a shell command, allows an existing configured working directory, and buffers stdout/stderr through `communicate()`. Its timeout kills the immediate process and awaits it; it does not create a process group or provide the Foundation subprocess runner's cancellation cleanup. No `CancelledError` cleanup appears in this bash path. An agent step uses `asyncio.wait_for`, retry count and backoff, but cancellation settlement depends on the supplied backend. These are distinct effect surfaces and should be tested separately. [Bash execution][R18], [agent retry path][R8].

## Maintainer ergonomics and Brokkr experiments

The parity document, hermetic conformance kit and legacy golden baselines make semantic drift reviewable. CI gates both engines, Python 3.11–3.13, bundle parsing and pinned Ruff rules. The parity report describes mutation checks that caught JSON extraction and loop-context ordering changes; this is a maintainer claim backed by inspectable harnesses, not an independent benchmark result. [Parity][R4], [CI][R12].

For Brokkr #429/#432, require evidence that every policy field reaches the effect boundary. Use a spy resolver/factory to test CI policy with attempted dependency installation and an empty capability intersection followed by bash. For #431/#434, inject disk-write failure and a crash after a non-idempotent fake effect; record whether success, recoverability and durable acknowledgement diverge. For lock integrity, make a controlled resolver return different identities on its two calls and test the ordinary CLI path. For #403/#433, cancel a bash step whose child owns a grandchild and floods output. These experiments target specific mechanisms without invoking live providers or asserting unmeasured superiority for either project.

## Pinned evidence ledger

| ID | Source | Evidence role |
|---|---|---|
| R1 | [pyproject.toml](https://github.com/microsoft/amplifier-bundle-recipes/blob/f8ec2ec8190ec55309cae784a98a1f7bfcdd1d63/pyproject.toml#L1) | Hybrid packaging and floating toolchain dependency |
| R2 | [src/amplifier_recipe_runner/execution.py](https://github.com/microsoft/amplifier-bundle-recipes/blob/f8ec2ec8190ec55309cae784a98a1f7bfcdd1d63/src/amplifier_recipe_runner/execution.py#L1295) | Library run/resume responsibilities |
| R3 | [src/amplifier_recipe_runner/planner.py](https://github.com/microsoft/amplifier-bundle-recipes/blob/f8ec2ec8190ec55309cae784a98a1f7bfcdd1d63/src/amplifier_recipe_runner/planner.py#L253) | Closed catalog and source precheck wiring |
| R4 | [docs/EXECUTOR_PARITY.md](https://github.com/microsoft/amplifier-bundle-recipes/blob/f8ec2ec8190ec55309cae784a98a1f7bfcdd1d63/docs/EXECUTOR_PARITY.md#L1) | Two engines, semantics and conformance |
| R5 | [src/amplifier_recipe_runner/tests/test_self_agent_refusal.py](https://github.com/microsoft/amplifier-bundle-recipes/blob/f8ec2ec8190ec55309cae784a98a1f7bfcdd1d63/src/amplifier_recipe_runner/tests/test_self_agent_refusal.py#L59) | Self/alias refusal |
| R6 | [src/amplifier_recipe_runner/tests/test_unknown_step_and_stage_keys.py](https://github.com/microsoft/amplifier-bundle-recipes/blob/f8ec2ec8190ec55309cae784a98a1f7bfcdd1d63/src/amplifier_recipe_runner/tests/test_unknown_step_and_stage_keys.py#L113) | Unknown-key refusal |
| R7 | [src/amplifier_recipe_runner/tests/test_errored_step_fails_the_run.py](https://github.com/microsoft/amplifier-bundle-recipes/blob/f8ec2ec8190ec55309cae784a98a1f7bfcdd1d63/src/amplifier_recipe_runner/tests/test_errored_step_fails_the_run.py#L92) | Real-error completion accounting |
| R8 | [src/amplifier_recipe_runner/engine.py](https://github.com/microsoft/amplifier-bundle-recipes/blob/f8ec2ec8190ec55309cae784a98a1f7bfcdd1d63/src/amplifier_recipe_runner/engine.py#L1264) | Approvals, retry, checkpoint handling |
| R9 | [src/amplifier_recipe_runner/trust.py](https://github.com/microsoft/amplifier-bundle-recipes/blob/f8ec2ec8190ec55309cae784a98a1f7bfcdd1d63/src/amplifier_recipe_runner/trust.py#L376) | Trust API and postures |
| R10 | [src/amplifier_recipe_runner/tests/test_trust.py](https://github.com/microsoft/amplifier-bundle-recipes/blob/f8ec2ec8190ec55309cae784a98a1f7bfcdd1d63/src/amplifier_recipe_runner/tests/test_trust.py#L139) | Trust method unit tests |
| R11 | [src/amplifier_recipe_runner/execution.py](https://github.com/microsoft/amplifier-bundle-recipes/blob/f8ec2ec8190ec55309cae784a98a1f7bfcdd1d63/src/amplifier_recipe_runner/execution.py#L1011) | Default install setting and activation |
| R12 | [.github/workflows/ci.yml](https://github.com/microsoft/amplifier-bundle-recipes/blob/f8ec2ec8190ec55309cae784a98a1f7bfcdd1d63/.github/workflows/ci.yml#L1) | CI gates and dependency acquisition |
| R13 | [src/amplifier_recipe_runner/lockfile.py](https://github.com/microsoft/amplifier-bundle-recipes/blob/f8ec2ec8190ec55309cae784a98a1f7bfcdd1d63/src/amplifier_recipe_runner/lockfile.py#L446) | Lock validation modes |
| R14 | [src/amplifier_recipe_runner/cli.py](https://github.com/microsoft/amplifier-bundle-recipes/blob/f8ec2ec8190ec55309cae784a98a1f7bfcdd1d63/src/amplifier_recipe_runner/cli.py#L1128) | Verify first plan, run replans |
| R15 | [src/amplifier_recipe_runner/provenance.py](https://github.com/microsoft/amplifier-bundle-recipes/blob/f8ec2ec8190ec55309cae784a98a1f7bfcdd1d63/src/amplifier_recipe_runner/provenance.py#L253) | Fields actually compared on resume |
| R16 | [src/amplifier_recipe_runner/tests/test_foreach_checkpointing.py](https://github.com/microsoft/amplifier-bundle-recipes/blob/f8ec2ec8190ec55309cae784a98a1f7bfcdd1d63/src/amplifier_recipe_runner/tests/test_foreach_checkpointing.py#L1) | Side-effect-sensitive replay tests |
| R17 | [src/amplifier_recipe_runner/execution.py](https://github.com/microsoft/amplifier-bundle-recipes/blob/f8ec2ec8190ec55309cae784a98a1f7bfcdd1d63/src/amplifier_recipe_runner/execution.py#L1796) | Best-effort overwritten state |
| R18 | [src/amplifier_recipe_runner/engine.py](https://github.com/microsoft/amplifier-bundle-recipes/blob/f8ec2ec8190ec55309cae784a98a1f7bfcdd1d63/src/amplifier_recipe_runner/engine.py#L1668) | Bash environment and termination |

[R1]: https://github.com/microsoft/amplifier-bundle-recipes/blob/f8ec2ec8190ec55309cae784a98a1f7bfcdd1d63/pyproject.toml#L1
[R2]: https://github.com/microsoft/amplifier-bundle-recipes/blob/f8ec2ec8190ec55309cae784a98a1f7bfcdd1d63/src/amplifier_recipe_runner/execution.py#L1295
[R3]: https://github.com/microsoft/amplifier-bundle-recipes/blob/f8ec2ec8190ec55309cae784a98a1f7bfcdd1d63/src/amplifier_recipe_runner/planner.py#L253
[R4]: https://github.com/microsoft/amplifier-bundle-recipes/blob/f8ec2ec8190ec55309cae784a98a1f7bfcdd1d63/docs/EXECUTOR_PARITY.md#L1
[R5]: https://github.com/microsoft/amplifier-bundle-recipes/blob/f8ec2ec8190ec55309cae784a98a1f7bfcdd1d63/src/amplifier_recipe_runner/tests/test_self_agent_refusal.py#L59
[R6]: https://github.com/microsoft/amplifier-bundle-recipes/blob/f8ec2ec8190ec55309cae784a98a1f7bfcdd1d63/src/amplifier_recipe_runner/tests/test_unknown_step_and_stage_keys.py#L113
[R7]: https://github.com/microsoft/amplifier-bundle-recipes/blob/f8ec2ec8190ec55309cae784a98a1f7bfcdd1d63/src/amplifier_recipe_runner/tests/test_errored_step_fails_the_run.py#L92
[R8]: https://github.com/microsoft/amplifier-bundle-recipes/blob/f8ec2ec8190ec55309cae784a98a1f7bfcdd1d63/src/amplifier_recipe_runner/engine.py#L1264
[R9]: https://github.com/microsoft/amplifier-bundle-recipes/blob/f8ec2ec8190ec55309cae784a98a1f7bfcdd1d63/src/amplifier_recipe_runner/trust.py#L376
[R10]: https://github.com/microsoft/amplifier-bundle-recipes/blob/f8ec2ec8190ec55309cae784a98a1f7bfcdd1d63/src/amplifier_recipe_runner/tests/test_trust.py#L139
[R11]: https://github.com/microsoft/amplifier-bundle-recipes/blob/f8ec2ec8190ec55309cae784a98a1f7bfcdd1d63/src/amplifier_recipe_runner/execution.py#L1011
[R12]: https://github.com/microsoft/amplifier-bundle-recipes/blob/f8ec2ec8190ec55309cae784a98a1f7bfcdd1d63/.github/workflows/ci.yml#L1
[R13]: https://github.com/microsoft/amplifier-bundle-recipes/blob/f8ec2ec8190ec55309cae784a98a1f7bfcdd1d63/src/amplifier_recipe_runner/lockfile.py#L446
[R14]: https://github.com/microsoft/amplifier-bundle-recipes/blob/f8ec2ec8190ec55309cae784a98a1f7bfcdd1d63/src/amplifier_recipe_runner/cli.py#L1128
[R15]: https://github.com/microsoft/amplifier-bundle-recipes/blob/f8ec2ec8190ec55309cae784a98a1f7bfcdd1d63/src/amplifier_recipe_runner/provenance.py#L253
[R16]: https://github.com/microsoft/amplifier-bundle-recipes/blob/f8ec2ec8190ec55309cae784a98a1f7bfcdd1d63/src/amplifier_recipe_runner/tests/test_foreach_checkpointing.py#L1
[R17]: https://github.com/microsoft/amplifier-bundle-recipes/blob/f8ec2ec8190ec55309cae784a98a1f7bfcdd1d63/src/amplifier_recipe_runner/execution.py#L1796
[R18]: https://github.com/microsoft/amplifier-bundle-recipes/blob/f8ec2ec8190ec55309cae784a98a1f7bfcdd1d63/src/amplifier_recipe_runner/engine.py#L1668
