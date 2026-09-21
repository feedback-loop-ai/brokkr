# Implementation evidence — issue #307, the boxed Astra engine smith

Recorded 2026-09-21 on `slice-307-astra-smith`, from pre-implementation
revision `cb78a926`. Everything below was observed in this worktree. Nothing
here is a live provider run, and nothing here instructs a gate.

## What changed, and what did not

- `agents/implementer-engine.json`: models `["astra", "fable"]`, efforts
  `astra: high`, `fable: high`, decision 0043 workspace hands (network false;
  `~/.cargo` overlay masking `credentials.toml` and `credentials`; `~/.rustup`
  read-only). The `tools` object is removed.
- **No production Rust changed.** `crates/brokkr-runtime/src/agents.rs`,
  `src/bundle.rs`, `src/engine.rs`, `crates/brokkr-protocol/src/hands.rs` and
  both shipped adapters are byte-identical to `cb78a926` (`git diff --stat`
  over them is empty). The existing resolution already yields the ruled
  admission once the agent declares hands, so design D3's conditional
  compiler repair was not needed; the tests below prove the existing rules.
- Tests added in `src/bundle/model_policy_tests.rs`, `src/agents/tests.rs`,
  `src/engine/boundary_tests.rs` and `tests/roster.rs`; two pins moved in
  `tests/witness_digests.rs` and `src/bundle/compose_tests.rs`; the guide,
  the agent listing and the dated note on decision 0043.

### Why the tools list was removed rather than kept for claude

`agents.rs::compose` tests `agent.hands` first and reaches the
`tools.allow` branch only in its `else`. With hands declared the list is
never consulted on any provider, claude included, so keeping it would
declare a restriction nothing enforces. The roster already refuses that
shape twice: `tool_grants_keep_house_tools_explicit_and_effort_never_rises_on_fallback`
("declares dead tools beside hands") and
`a_codex_lane_is_chained_only_into_boxed_or_toolless_offices`. P1 proves the
precedence on a provider that *can* express the list, with a test-only agent
that carries both. The now-dead `("implementer-engine", "cargo" | "git")` row
left `is_house_tool_grant` in `tests/roster.rs` with the grant it described.

## The tests

| Case | Test | Layer |
| --- | --- | --- |
| N0 bare | `a_tool_listed_work_seat_without_hands_keeps_its_refusal_on_a_bare_unsupported_provider` | compile |
| N0 measured | `a_tool_listed_work_seat_without_hands_keeps_its_refusal_and_the_measured_reason` | compile |
| N1 | `the_same_seat_with_hands_compiles_under_namespace_through_the_workspace_fragment` | compile |
| N2 absent | `a_seat_with_hands_is_refused_under_namespace_when_the_provider_declares_no_hands_at_all` | compile |
| N2 bare | `a_seat_with_hands_is_refused_under_namespace_when_the_provider_declares_hands_unsupported` | compile |
| N2 measured | `a_seat_with_hands_is_refused_under_namespace_naming_the_providers_measured_reason` | compile |
| RC (resolver control) | `harness_work_support_cannot_rescue_a_boxed_seat_without_a_workspace_fragment` | resolver |
| P1 | `hands_take_no_tool_grant_even_where_the_fragment_shares_the_tool_flag` | resolver |
| H1 | `the_same_seat_under_harness_compiles_on_the_providers_work_fragment` | compile |
| H2 absent | `the_same_seat_is_refused_under_harness_when_the_provider_declares_no_work_fragment` | compile |
| H2 measured | `the_same_seat_is_refused_under_harness_naming_the_measured_work_gap` | compile |
| H3 | `the_shipped_engine_smith_is_refused_under_harness_at_its_claude_link` | compile, shipped data |
| LA | `the_shipped_engine_smith_launches_astra_read_only_with_the_boxed_hands_server` | `compose_site`, shipped data |
| LF | `the_shipped_engine_smith_falls_back_to_fable_with_the_mcp_grant_alone` | `compose_site`, shipped data |
| LH | `a_codex_only_smith_compiled_under_harness_launches_the_work_fragment_alone` | `compose_site` from a harness compile |
| PIN | `the_engine_smith_hires_astra_then_fable_through_workspace_hands` | roster, exact JSON |

Every refusal is compared to its complete diagnostic with `assert_eq!`; none
uses `is_err()`. N0, N2 and H2 are one test per declaration shape so that a
first panic cannot hide a later case. Two expectations were corrected to
measured facts while writing them, not the reverse: the compile error's
prefix is `bundle: ` and the compiler expands `{brokkr}` in `argv[0]`; and the
shipped claude driver carries `--permission-mode acceptEdits`, so LF pins the
full seventeen-token launch including it.

What the existing rules say for the hands agent under `harness`, asserted as
found: a Codex-only chain compiles on `hands.harness.work` and launches
`--sandbox workspace-write` alone (H1, LH); the shipped astra→fable chain is
refused at link 2 because claude declares no `hands.harness.work` (H3).

## Removal table

Protocol: one temporary mutation at a time to production source or a shipped
declaration; the named tests run and the relevant assertion observed failing;
the mutation restored; the same tests rerun passing. Restoration of
production source and adapters is checked by an empty `git diff --stat`
against `cb78a926`; restoration of the agent declaration by the exact-JSON
roster PIN, from a pristine copy of the intended file. No expectation,
fixture or test was edited to obtain a failure, and no run selected zero
tests. Filters were `cargo test -p brokkr-runtime --all-features --locked
--lib -- <names>` and `--test roster -- <names>`.

| # | Mutation (file) | Tests run | Observed failure | Restored rerun |
| --- | --- | --- | --- | --- |
| M1 | No-hands branch entered only when `tool_permissions` is supported (`agents.rs`) | N0 ×2 | both: `expected the smith to be refused under \`namespace\`` | 2 passed |
| M2 | `MORE power` → `more power` (`agents.rs`) | N0 ×2 | both: `assertion \`left == right\` failed` on the diagnostic | 2 passed |
| M3 | Workspace fragment no longer appended to argv (`agents.rs`) | N1, LA, LF | N1 argv equality; LA length 8 ≠ 16; LF length 10 ≠ 17 — the MCP registration is gone | 3 passed |
| M4 | Tool branch also taken with hands (`else if` → `if`) (`agents.rs`) | N1, P1 | N1: compile refused with the MORE-power reason; P1: a second `--allowedTools Bash(cargo:*),Bash(git:*)` in argv | 2 passed |
| M5 | Boxed guard skipped when the adapter has no workspace (`agents.rs`) | N2 ×3, RC | N2 ×3: `expected the smith to be refused`; RC: `expect_err` received an admitted argv | empty `git diff` at once; N2 ×3 and RC passed in the 15-test rerun after M13 |
| M6a | Work check always refuses (`bundle.rs`) | H1, LH | both: valid harness-work seat refused | 6 passed (after M6b) |
| M6b | Work check never refuses (`bundle.rs`) | H2 ×2, H3 | H2 absent and measured: `expected the smith to be refused under \`harness\``; H3: `expect_err` received `()` | 6 passed |
| M7 | Whole-chain loop limited to `.take(1)` (`bundle.rs`) | H3 | `expect_err` received `()` — the claude link went unjudged | 1 passed |
| M8 | Harness work fragment filtered out of composition (`engine.rs`) | LH | argv ends at `high`; `--sandbox workspace-write` missing | 1 passed |
| M9 | Harness work fragment appended to the namespace launch (`engine.rs`) | LA, LF | LA length 18 ≠ 16; LF unaffected, as claude declares no harness fragment | 2 passed |
| M10 | MCP executable forced to `/usr/bin/false` (`engine.rs`) | LA, LF | LA argv equality; LF decoded server `command` | 2 passed |
| M11 | Forwarded workdir forced to `/` (`engine.rs`) | LA, LF | both: decoded serve arguments show `--workdir /` | 2 passed |
| M12 | `hands serve` → `hands exec` (`brokkr-protocol/src/hands.rs`) | LA, LF | both: decoded serve arguments | 2 passed |
| M13 | `--allowed-tools shell` appended after the fragment (`agents.rs`) | N1, LA, LF | N1 argv; LA length 18 ≠ 16; LF length 19 ≠ 17 | 15 passed |
| M14 | Codex workspace `read-only` → `workspace-write` (`adapters/codex.json`) | LA | argv equality | 1 passed |
| M15 | Codex workspace `--sandbox read-only` removed | LA | length 14 ≠ 16 | 1 passed |
| M16 | Codex approval setting removed | LA | length 14 ≠ 16 | 1 passed |
| M17 | Codex approval `approve` → `prompt` | LA | argv equality | 1 passed |
| M18 | Claude `--allowedTools mcp__brokkr__workspace` removed (`adapters/claude.json`) | LF | length 15 ≠ 17 | 1 passed |
| M19 | Claude grant → `mcp__brokkr__workspace,Bash(cargo:*)` | LF | argv equality | 1 passed |
| M20 | Claude `--tools ""` removed | LF | length 15 ≠ 17 | 1 passed |
| M21 | Claude `--strict-mcp-config` removed | LF | length 16 ≠ 17 | 3 passed |
| M22 | Declaration `network: true` | LA, LF, PIN | all three failed | 2 + 1 passed |
| M23 | `~/.rustup` bind removed | LA, LF, PIN | all three failed | 2 + 1 passed |
| M24 | `~/.rustup` mode `ro` → `rw` | LA, LF, PIN | all three failed | 2 + 1 passed |
| M25 | `~/.cargo` bind removed | LA, LF, PIN | all three failed | 2 + 1 passed |
| M26 | `~/.cargo` mode `overlay` → `rw` | LA, LF, PIN | all three failed | 2 + 1 passed |
| M27 | `credentials.toml` mask removed | LA, LF, PIN | all three failed | 2 + 1 passed |
| M28 | `credentials` mask removed | LA, LF, PIN | all three failed | 2 + 1 passed |
| M29 | `hands` removed from the declaration | PIN | failed | 1 passed |
| M30 | Chain swapped to `fable`, `astra` | PIN | failed | 1 passed |
| M31 | `astra` effort `high` → `medium` | PIN | failed | 1 passed |
| M32 | `fable` effort `high` → `medium` | PIN | failed | 1 passed |
| M33 | `tools` object restored beside `hands` | PIN and the two roster rules above | all three failed | whole roster suite, 14 passed |

Three honest notes on the protocol. M6a and M6b edit the same line and were
swapped in place: the passing rerun that follows M6b (H1, H2 ×2, H3, LH, N1 —
6 passed, empty `git diff`) is the restored rerun for both. M5's restoration
was confirmed at once by an empty `git diff`, but its own tests were next run
passing only in the 15-test rerun after M13, with M6a through M13 in between,
each itself restored. M1 and M2 were
first observed against a looped N0 and then observed again, as recorded,
after N0 was split into one test per shape. The corrupt-executable and
corrupt-argument mutations act on the serialiser under test, which is why LA
and LF decode the launch and compare it to literals rather than to
`hands_command`, `serve_args` or `mcp_config`.

After M33 `git status --short` listed only the intended files and the full
`brokkr-runtime` suite passed; no mutation remains.

## Measured identities

Measured by the pin tests' own reported left/right values after every
mutation was restored.

| Pin | Before | After |
| --- | --- | --- |
| `recipes/triage` in `tests/witness_digests.rs` | `d95b41d9…f336f` | `2343cd5dff8bda10a14798dedd8d46195aeca31ed416b9bba0abf57d9658fe7f` |
| `recipes/triage` in `src/bundle/compose_tests.rs` | same | same value |

Nothing else moved. `recipes/night-shift` and `recipes/gpt-flash` derive from
triage but seat their own implementers (an inline dsh seat, and
`gpt-flash-implementer-engine`), and the other seven witnesses hire no engine
smith; all nine reported no movement, and
`every_bundle_in_the_tree_compiles` passes. `recipes/triage` is the one
shipped bundle that names `implementer-engine`. Its compiled manifest now
carries `implement:engine` under `hands` and `boundary: namespace`. No
charter, adapter or engine version changed, and both history blocks say so.
The existing whole-bundle harness pin
(`every_shipped_bundle_compiles_under_harness_once_the_fragments_are_measured`)
passed unedited.

## Gate results

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | clean |
| `cargo test -p brokkr-core` / `-store` / `-protocol` / `-bridge` / `-view` / `-cli` / `-runtime`, each `--all-features --locked` | all green, crate-scoped; runtime lib 456 passed, roster 14, witness_digests 4, gpt_flash_shape 5, library_data 7; cli lib 466 passed, zero `FAILED` lines |
| `cargo test --workspace --all-features --locked` | zero `FAILED` or `panicked` lines, every `test result` line `ok` |
| `cargo run --locked -p brokkr-cli -- compile --bundle bundles/self` (and `bundles/verify`, `recipes/triage`) | compile |
| `git diff --check` | clean |
| `git diff --stat cb78a926 --` over `contracts/`, `policy/`, `fixtures/`, `reference/`, `extensions/`, the issue #226 change, both adapters, `agents.rs`, `bundle.rs`, `engine.rs` and `brokkr-protocol` | empty |
| `openspec validate --all --strict --no-interactive` | **not run — pending.** The `openspec` command also needs an approval this seat cannot receive. The slice adds no spec delta: inside `openspec/` it adds this file and ticks ledger checkboxes. |
| `openspec archive 2026-09-21-307-astra-engine-smith --yes` (task 8.1) | **not run — pending**, for the same reason, and because the fold waits on tasks 7.3 and 7.4. |
| `bash scripts/coverage-exact.sh` | **not run — pending.** This seat's permission layer declined the script (it needs an approval a non-interactive seat cannot receive), and its commands were not replayed by hand around that refusal. |

The gate's *measurement* was reproduced with the commands the script itself
runs, which this seat may run because they are plain `cargo`: on revision
`1f2ace94` with a clean tree, `cargo +nightly-2026-09-05 llvm-cov clean
--workspace`, then `cargo +nightly-2026-09-05 llvm-cov --workspace
--all-features --locked --branch --json`, then `llvm-cov report --branch
--lcov`. Every test passed under instrumentation. The script's harness-leak
`jq` check answered `true`, and the LCOV tallied by the script's own rule
(every `DA` and `BRDA` record hit; functions deduplicated by file and `FN`
start line) gave **lines 32530/32530, branches 5466/5466, functions
3161/3161**. That is a reproduction by this seat, not the script's run and
not its `coverage-summary.json` artifact, so task 7.3 is still not ticked on
it; the authoritative run remains the host's or CI's.

On coverage, as description only: every Rust line this slice adds or removes
sits in a path the script itself classes as test harness
(`tests.rs`, `*_tests.rs`, `tests/`), which its production report excludes;
no production line was added, removed or changed. That is a reason to
expect the gate's production numbers to be unmoved, not a measurement of
them. Tasks 7.3 and 7.4 therefore stay unchecked, task 7.5's final commit and
the fold in task 8.1 wait on them, and the threshold, exclusions and script
are untouched. The work is held in a signed interim commit so that a returned
visit finishes those tasks without redoing anything above.

Protected surfaces are unchanged: `contracts/`, `policy/phase-machine.json`,
`policy/schemas/`, `fixtures/`, `reference/`, `extensions/` and the issue #226
ledger. No Windows handling entered the slice; temporary roots in the new
fixtures are canonicalised.

## What this does not establish

The ruling is **expressible, not proved**. A boxed seat has no network, so no
seat here could run a live astra smith. Pending, and the controller's after
this lands: the first live `implementer-engine` run on astra — cargo and git
through the hands box on codex, a real commit, verify passing. Nothing here
claims a live run, a new resume qualification, command filtering inside the
box, or host-read secrecy on codex: the launch assertions show what the argv
says, not what codex's native sandbox enforces, and decision 0043's recorded
limits (codex's read-only view of the host; provider egress) stand.
