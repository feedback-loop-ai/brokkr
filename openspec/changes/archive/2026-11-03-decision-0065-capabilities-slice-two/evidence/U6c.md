# U6c evidence — the bound plan, defined and consumed

Unit U6c closes tasks 28.1 and 28.2 on branch `s2/U6c`, cut from main at
`5b1d6785`. This is the repair visit (run
`0065-slice-two-unit-u6c-see-the--0382c480`). It started from the whole
held attempt of run `0065-slice-two-unit-u6c-see-the--584f79f0` (head
`c905a1f1`), applied uncommitted on main, and answers that council's
findings F1–F6. F7 asked for no code. Everything below was observed in
the repair session on 2026-10-06, against the working tree that became
this unit's commit. Nothing is carried over from the held attempt's
evidence.

The same run's review returned one more finding, R1: the owner's
repository was compared with its directory but never checked as a
sha256, so a self-consistent `not-a-digest` or upper-case repository
reached `ServingIncomplete`. The return visit, also on 2026-10-06, adds
`owner.repo` to the shape check's digest list, so it must be the
canonical lower-case sha256 D6 names, and `Sealed::at` now takes the
repository its tree is sealed under. Rows W1r and W1u below are its two
removals. The other mutation rows are the repair visit's; the gates the
return visit re-ran are named under "Gates on this tree".

## What changed

`crates/brokkr-protocol/src/broker.rs` is new, and `lib.rs` registers it as
`pub mod broker`. It defines the shared closed types: `Inventory` and `Pin`
(the attempt's sealed list of plans, by locator and digest), `Plan` with
`Owner`, `Dialect`, `Connection`, `Restrictions`, `Clearance` and the box
intent `BoxIntent`, and `BoxIntent`'s parts `Reach`, `Tree`, `Sources`,
`Writers`, `Privilege`, `Network`, `Bootstrap` and `Excluded`. It also
defines the typed failure `Refusal`, a `thiserror` enum that carries MB3's
and MB4's exact causes and SD3's incomplete-serving cause.

Every record is `deny_unknown_fields` and is read only as a JSON object
(F3). serde's derived struct visitor also accepts a sequence and reads its
fields by position, so `deny_unknown_fields` alone let `[...]` through.
Each record now derives with `#[serde(remote = "Self")]`, and one
`objects!` macro gives each its `Deserialize`. That impl feeds the derived
visitor through a private `Object` deserializer, which offers it a map and
nothing else. Every record position therefore refuses an array before any
field is read: the inventory, each pin, the plan, and each nested record.
`Restrictions` is an empty struct, so exactly `{}` parses. `Tree::System
{}` stays a struct variant, so a field beside its tag is refused.
`Network` is `isolated`/`shared`, the projection of egress, so the egress
words do not parse there. `native_controls/mcp.rs` and its `Transport`
export are untouched.

`Owner`'s effect, attempt and instance are strings, in the engine's own
spelling (F2). The engine mints UUID strings for its effect and attempt ids
(runtime `engine.rs`, `Uuid::new_v4().to_string()`), and its site instance
reference is a `String` (`engine/resume.rs`). The u64 fields and the
decimal attempt directory are gone.

`crates/brokkr-cli/src/broker.rs` consumes these types immediately.
`BrokerError` keeps the four parse-time locator and digest causes; the
unbound text lives once, in `Refusal::Unbound`. `run` binds, admits, and
then refuses with `broker serving protections are incomplete`. No secret is
resolved and nothing is spawned on any path.

Binding (`bound`) works as follows:

1. **The trusted root (F1).** HOME comes from
   `brokkr_protocol::hands::home_dir` over this process's own environment,
   which the broker inherits from the engine. The locator never supplies
   it. HOME must be absolute, and it is canonicalised. The protected root
   `HOME/.local/state/brokkr/capabilities` is then opened from `/` without
   following a symlink below HOME, and its device and inode are kept.
2. The locator must read `<root>/<repo>/<run>/<attempt>/<plan>`. Its
   directories are opened from `/` one component at a time with
   `O_NOFOLLOW|O_DIRECTORY`. The component in the root's position must be
   the very directory from step 1, compared by device and inode and never
   by spelling. A self-consistent tree anywhere else is unbound.
3. Every ancestor of that root must be owned by this user or by root, and
   writable by no one else unless it is a root-owned sticky directory. The
   root and everything below it must be this user's alone
   (`mode & 0o077 == 0`).
4. `inventory.json` and the plan are each read through one no-follow
   handle. Each must be a regular file of one link that only this user can
   access, and at most 1 MiB (MB3's request bound).
5. The inventory must pin exactly this locator, once, at exactly this
   digest. The plan's bytes must hash to that digest and parse closed.
6. The plan's owner must name the repository, run and attempt directories
   that hold it. The attempt must also be one portable path component
   (F2): ASCII letters, digits, `.`, `_` or `-`. A locator's components are
   never empty, `.` or `..`, so the equality excludes those.

Admission (`admit`) then checks in MB3's order:

1. **Shape**, refused as unbound. This covers:
   - sha256 digests, the owner's repository among them (R1, D6), and
     non-empty names (capability, dialect name and
     version, effect, site, instance);
   - the server named exactly `cap-<capability>` (F4, D5/0077);
   - tool and fixed-environment sets, with every tool non-empty (F4), and
     a binding-name set, possibly empty;
   - an argv with every member non-empty (F4);
   - the clearance receipt's dialect digest equal to the dialect's;
   - the bootstrap being this binary (`current_exe`);
   - the attempt directory among the control roots;
   - every sealed path absolute and normal.
2. **Binding names**: decision 0012's `validate_name`, then a
   fixed-environment collision (MB3's startup-input cause). The plan wrote
   the name, so a control character in the 0012 cause is escaped and the
   diagnostic stays on one line (F6). The cause and its text are otherwise
   unchanged, and the shared grammar is untouched.
3. **The program tree** (`MCP server box program tree cannot be resolved`):
   - A launch path with a `/` that is not absolute refuses.
   - An executable directly in `/usr/bin`, `/usr/sbin`, `/usr/local/bin`,
     `/usr/local/sbin`, `/bin` or `/sbin` is a system entry.
   - Otherwise it is a package rooted at its parent, or at the parent's
     parent when the parent is a `bin` or `sbin`.
   - A package root that is the trusted HOME or an ancestor of it refuses
     (F1: the trusted HOME, never the locator's). `/` is an ancestor of
     every absolute HOME. The held attempt's separate `== "/"` term could
     never refuse alone, so this visit folded it into the HOME check.
4. **Reach**: the executable in a writable root refuses with the launch
   cause first. The program tree or the bootstrap overlapping any reach root
   in either direction refuses with the overlap cause.
5. **Identity facts** (`MCP server box filesystem identity is not
   protected`):
   - The writer list must not be empty, and no writer may hold the overflow
     uid 65534 or `u32::MAX`.
   - Sources must stay within 1,000,000 entries and 65,536 mount records.
   - No control root may overlap a reach root or a box source.
6. **The store**: inside reach refuses with MB4's hands-reach cause. As a
   box source refuses with MB4's in-box cause.

Assumptions this unit states, because no document fixes them:

- The inventory's file name is `inventory.json`, beside the attempt's plans.
- The plan's JSON field names are this unit's. The design fixes the plan's
  content, not its spelling.
- The inventory pins plans only. Pinning ledgers is U6d's and U7c's, and a
  ledger field without a consumer would break ruling 6.
- "Safe path component" for the attempt is read as POSIX's portable
  filename character set. The engine's UUID attempt ids are within it.

U7c writes both files. These checks are on the sealed plan itself. Later
units own the rest: argv executable resolution and source verification
(U6c4, U6c5), source and bootstrap reobservation, network comparison, and
identity-based overlap checks. Nothing here claims them.

## Tests and their mutations

All tests are in the owning suite
`crates/brokkr-cli/tests/capability_broker.rs`, which `tests/it.rs` already
registers. No test moved and no module was added. The suite has 20 tests,
two of them new in this visit: `only_the_engines_home_roots_a_plan` and
`every_record_is_an_object_never_a_positional_array`. They drive the real
binary. Fixtures are built under a canonicalised `tempfile` root by the
shared `Sealed` builder. Its `serve_as` gives the broker child its HOME
through the child's own environment (`Command::env`, or `env_remove` for
none). It never sets the test process's environment. Owner fixtures are
engine-shaped: UUID effect and attempt ids, a string instance, and the
attempt directory named by the attempt UUID. The shape loop now labels each
case `pointer = value`, so a failure names the exact case.

The two counters:

- **Lookups.** The store is a 0600 FIFO, and a test thread holds its write
  end. That thread unblocks, and timestamps, only when a reader opens the
  FIFO. The test's own release opens it only after the broker has exited.
- **Starts.** The planted server is a script that would `touch` a marker.

Each mutation was applied alone with `Edit`. The whole focused suite then
ran with `cargo test -p brokkr-cli --test it capability_broker::` (R3 alone
ran only its test). The tree was restored with `git checkout --` from the
staged tree, in the same command. Every row's first failing assertion is
quoted as observed, `left` being the mutant's answer. The restored suite
passed 20 of 20, including after the last restore.

| # | Behaviour | Compiling removal | Failing test: assertion, as observed |
| --- | --- | --- | --- |
| F1a | locator root is the trusted root | `same(stat, root)` made always true | `only_the_engines_home_roots_a_plan`: lookalike under `HOME=host` left `ServingIncomplete`, right `Unbound`; also `a_plan_answers_for_the_attempt_it_lies_in_and_its_size`, stray `.local/state/brokkr/other` pin |
| F1b | trusted HOME is canonical | `canonicalize` dropped | same test: `HOME` reached through a symlink, left `Unbound`, right `ServingIncomplete` |
| F1c | trusted HOME is absolute | `is_absolute` made true | same test: `HOME=home` relative to the cwd, left `ServingIncomplete`, right `Unbound` |
| F1d | widening uses the trusted HOME | `home.starts_with(derived)` made false | `the_program_tree_is_mb3s_layout_of_the_executable`: `/docs-mcp` with package `/`, left `BindOverlapsReach`, right `ProgramTree`. Before `/` was folded in (see above), the same removal failed on `home/d` with package HOME: left `Identity` |
| F2a | owner names its directories | equality made always true | `a_plan_answers_…`: `/owner/repo`, left `ServingIncomplete` |
| F2b | engine-shaped attempt id binds | only the attempt's equality removed | same test: `/owner/attempt` set to another UUID, left `ServingIncomplete` |
| F2c | attempt is a portable component | portable test made true | same test: `Sealed::at("attempt 1")`, left `ServingIncomplete`, right `Unbound` |
| F3 ×15 | each record is an object | that one type's impl replaced by its derived `deserialize` without `Object` | `every_record_is_an_object_never_a_positional_array`, each at its own case, left `ServingIncomplete`: `""` (Plan), `/owner`, `/dialect`, `/connection`, `/restrictions` (`[]`), `/clearance`, `/box`, `/box/reach`, `/box/tree`, `/box/sources`, `/box/writers`, `/box/bootstrap`, `/box/excluded`, `inventory`, `pin` |
| F4a | server is `cap-<capability>` | equality made always true | `a_sealed_plan_of_the_wrong_shape_is_unbound`: `/server = ""` |
| F4b | same, beyond non-empty | equality replaced by `!server.is_empty()` | same test: `/server = "cap-unrelated"` |
| F4c | tool members non-empty | term made true | same test: `/tools = [""]` |
| F4d | argv members non-empty | term made true | same test: `/connection/argv = [""]` |
| F4e | trailing argv member | check limited to `argv[0]` | same test: `/connection/argv = ["docs-mcp",""]` |
| F6 | one-line name cause | `map_err(Refusal::Name)` without escaping | `binding_names_are_checked_…`: stderr `…'X\nerror: forged'…` (two lines), right the escaped one-line text |
| W1 | digests are sha256 | term made true | wrong-shape: `/clearance/policy = "AAAA…"` (`/dialect/digest` upper case is also refused by the receipt equality) |
| W1r | repository is a sha256 (R1) | `&owner.repo` dropped from the digest list | `a_plan_answers_…`: repository and directory both `not-a-digest`, left `ServingIncomplete`, right `Unbound` |
| W1u | repository canonical case (R1) | `&owner.repo.to_lowercase()` in its place | same test: repository and directory both `DIGEST` upper case, left `ServingIncomplete`, right `Unbound` |
| W2 | names non-empty | whole term made true | wrong-shape: `/owner/effect = ""` |
| W2a–f | each name entry | that one entry removed from `names` | wrong-shape, in turn: `/owner/effect = ""`, `/owner/site = ""`, `/owner/instance = ""`, `/dialect/name = ""`, `/dialect/version = ""`, and (capability) the `capability ""`/`server "cap-"` pair |
| W3 | tools a non-empty set | term made true | wrong-shape: `/tools = []` |
| W3b | `set` uniqueness | `==` made `<=` | wrong-shape: `/tools = ["read","read"]` |
| W4 | binding names a set | term made true | wrong-shape: `/secrets = ["DOCS_TOKEN","DOCS_TOKEN"]` |
| W4b | empty bindings allowed | `secrets.is_empty()` made false | wrong-shape: `/secrets = []` left `Unbound`, right `ServingIncomplete` |
| W5 | environment a non-empty set | term made true | wrong-shape: `/box/environment = []` |
| W5b | environment unique | term replaced by `!is_empty()` | wrong-shape: `/box/environment = ["PATH","PATH"]` |
| W6 | environment names valid | `valid_name` made true | wrong-shape: `/box/environment = ["PATH","lower"]` |
| W7 | argv non-empty | term made true | wrong-shape: `/connection/argv = []`, left exit 101 (`argv[0]` out of bounds) |
| W8 | clearance receipt matches | equality made true | wrong-shape: `/clearance/dialect = "cccc…"` |
| W9 | bootstrap is this binary | term made true | wrong-shape: `/box/bootstrap/path = …/other-brokkr` |
| W10 | attempt among control roots | term made true | wrong-shape: `/box/excluded/control = [<run dir>]` |
| W11 | sealed paths normal | term made true | wrong-shape: `/box/reach/readable = [".../cache/../opt"]` |
| W11b | `normal` requires absolute | `is_absolute` made true | wrong-shape: `/box/excluded/store = "store/secrets.env"` |
| I1 | writers non-empty | `!uids.is_empty()` made true | `unprovable_writers_…`: `uids []`, left `ServingIncomplete`, right `Identity` |
| I2 | overflow uid unmapped | 65534 dropped from `UNMAPPED_UIDS` | same test: `uids [1000, 65534]` |
| I3 | `u32::MAX` unmapped | `u32::MAX` dropped | same test: `uids [4294967295]` |
| I4 | source-entry bound | bound raised by one | same test: `entries 1_000_001`, left `ServingIncomplete`, right `Identity` |
| I5 | mount-record bound | bound raised by one | same test: `mounts 65_537`, left `ServingIncomplete`, right `Identity` |
| I6 | control vs reach roots | reach roots dropped (`take(0)`) from the exposed set | same test: writable `[HOME]`, left `ServingIncomplete` |
| I7 | control vs box binds | binds dropped (`take(0)`) from the exposed set | same test: control `[attempt, opt/docs/state]`, left `ServingIncomplete` |
| S1 | store not in reach | `!reachable` made true | `the_store_is_neither_in_reach_nor_in_the_box`: `work/.forge/secrets.env`, left `ServingIncomplete` |
| S2 | StoreInBox predicate | `!mounted` made true | same test: `opt/docs/secrets.env`, left `ServingIncomplete`, right `StoreInBox` |
| S3 | StoreInBox covers the bootstrap | bootstrap dropped from the store's binds (`take(1)`) | same test: store = bootstrap, left `ServingIncomplete` |
| B1 | refused even when admitted | `run` returns `Ok(SUCCESS)` | `a_sealed_plan_is_admitted_…`: left `(Some(0), "")` (eight other tests also failed) |
| B2 | zero lookups | `resolve_bindings(store, secrets)` added | same test: `store.lookups(ended) == 0`, left `1` |
| B3 | zero starts | `Command::new(executable).status()` added | same test: `!sealed.path("started").exists()` failed |
| B4 | pinned exactly once at this digest | pin check made true | `only_the_protected_inventory_binds_a_plan`: inventory pinning another plan, left `ServingIncomplete` |
| B4b | not twice | check made `pins.contains(&digest)` | same test: this plan pinned twice |
| B4c | at this digest | check made `pins.len() == 1` | same test: pinned at another digest |
| B5a | root and below owner-only | private arm made true | `a_plan_under_an_unprotected_root_or_file_is_unbound`: capabilities root at 0750 |
| B5b | ancestors unwritable | ancestor arm made true | same test: HOME at 0777 |
| B6a | files owner-only | mode term zeroed | same test: plan file at 0640 |
| B6b | files of one link | `st_nlink.min(1)` | same test: hard-linked alias |
| B7 | 1 MiB bound | bound raised by one | `a_plan_answers_…`: a plan of 1 MiB + 1, left `ServingIncomplete` |
| B8 | bytes hash to the digest | digest check made true | `a_field_changed_after_sealing_acquires_no_authority`: `/box/executable` altered, left `ProgramTree`, right `Unbound` |
| B9 | plan closed | `deny_unknown_fields` removed from `Plan` | `a_plan_parses_closed_and_defaults_nothing`: unknown top-level field |
| N1 | 0012 names checked | `validate_name` errors discarded | `binding_names_…`: `named("lower")`, left `ServingIncomplete` |
| N2 | fixed keys collide | collision check made true | same test: `HOME`, left `ServingIncomplete`, right `StartupInputs` |
| T1 | package root derived | `root == derived` made true | `the_program_tree_…`: `opt/docs/bin/docs-mcp` with root `opt/docs/bin` |
| T2 | no relative launch path | `searched` made true | same test: argv `bin/docs-mcp` |
| R1 | launch not in reach | check made true | `the_box_neither_…`: `work/tool/server`, left `BindOverlapsReach`, right `LaunchInReach` |
| R2 | binds clear of reach | check made true | same test: `cache/tool/server`, left `ServingIncomplete` |

The F1 lookalike control is the strongest form the finding names. The very
sealed tree, owner-only, pinned at its true digest, is served by an engine
whose HOME is another directory with its own protected root. Only the
identity comparison refuses it.

The five U6b tests pass unchanged. That includes the manual invocation of a
real file with its true digest, which still reads unbound, and the compile
fence on an unused MCP grant with an empty office list. Native OFF and the
realm-wide fence for wants and unused grants are pinned in the runtime
suites, which this unit does not touch. They passed on this tree, as
recorded below.

## Gates on this tree

| Check | Observed |
| --- | --- |
| focused suite, default TMPDIR (`/tmp`) | 20 passed. The fixture roots were `/tmp/.tmp*`, which the ancestor guard admitted |
| focused suite, TMPDIR in a group-writable chain (`cargo test --config 'env.TMPDIR=…/.forge/u6c/tmp'`; the worktree and `.forge` are 0775, owned by this user) | every sealed fixture refused as `Unbound` (8 passed, 12 failed). This is the box-`/tmp` case: the guard correctly refuses a group-writable ancestor |
| `cargo test -p brokkr-cli` | lib 627, `it` 476 (2 ignored), driver_conformance 27, heap 1+1+1, transcript_surfaces 13 — all passed |
| `cargo test -p brokkr-protocol` | lib 642 (2 ignored), hands_exits 6, secret_drop 1, doc 1 — all passed |
| core, store, view, bridge | all passed |
| runtime `--lib` 831; `--tests`: capability_launch 72, `it` 120 (witness pins included, unchanged), operated_repo 1, queued_launch 3 | all passed |
| `cargo +1.88 check --workspace --all-targets --all-features` | finished, no error |
| `compile --bundle bundles/self` | exit 0 |
| `openspec validate --all --strict` | 20 passed, 0 failed |
| `typos --hidden`, `git diff --check` | clean |
| `cargo +nightly-2026-09-05 llvm-cov -p brokkr-cli -p brokkr-protocol --branch --test it -- capability_broker::` | CLI `broker.rs` 299/299 lines, 6/6 branches, every function hit; protocol `broker.rs` 6/6 instrumented lines |

After R1's change the return visit re-ran, on the final tree: the focused
suite (20 passed, default TMPDIR), `cargo test -p brokkr-cli` (lib 627,
`it` 476, driver_conformance 27, heap 1+1+1, transcript_surfaces 13, all
passed), formatting, workspace clippy, the Rust 1.88 check,
`quality/ratchet.sh files` ("file size holds") and `clones`
("duplication holds"), typos, `git diff --check`, openspec (20 passed)
and `compile --bundle bundles/self` (exit 0). R1 touched only CLI
`broker.rs` and its owning suite, so the other crates were not re-run.

The workspace was run crate by crate under the ten-minute foreground limit,
not as one `cargo test --workspace` invocation. Formatting and workspace
clippy (`--all-targets --all-features --locked -D warnings`) were clean on
this tree.

## Measured records

- `quality/file-lines.txt` records CLI `broker.rs` at 492 lines (main: 91),
  the new protocol `broker.rs` at 283, protocol `lib.rs` at 219 (main:
  218), and the owning suite at 1076 (main: 262). Each was measured with
  `wc -l` on this tree. All are under their 800 and 2,000 ceilings, and no
  other row moved.
- `quality/public-api/brokkr-protocol.txt` was regenerated with
  cargo-public-api 0.52.0 on `nightly-2026-09-05`, using the snapshot's
  `-sss` invocation and its header line. Against main it adds 112 lines,
  all under `brokkr_protocol::broker`. 30 of them are the 15 hand-written,
  map-only `Deserialize` impls, which `-sss` lists where it hides derived
  ones. The operator rules on the raise, which the controller expected.
- No suppression, function over 100 lines, dependency, witness input or
  budget input moved. `too-many-lines.txt`, `suppressions.txt`, the budget
  files and the witness pins are unchanged.

## Pending

- **`quality/ratchet.sh` `api` and `baselines origin/main`.** The return
  visit ran `files` and `clones` (above). The review's chief reported
  `api` passing and `baselines origin/main` refusing the protocol API
  raise (730 to 842 items) plus two inherited rows outside this diff
  (`adapters.rs`, `driver_conformance.rs`); the raise waits on the
  operator's ruling.
- **The owner-only TMPDIR run** on the final tree. The review's chief
  observed 20 of 20 under an owner-only TMPDIR before R1; this seat could
  not create a directory outside the worktree to repeat it after R1.
- **Exact coverage.** `bash scripts/coverage-exact.sh` was not run; the
  scoped diagnostic above is not that gate.
- **macOS.** The tests were not run on macOS. There is no Linux-only API;
  the device and inode comparison and the mode and link-count comparisons
  infer their types per platform.
- **Remote CI** on the final head.

Follow-ups for later rows:

- U7c writes `inventory.json` and plans in the shape above, owner ids
  included.
- U6c4 and U6c5 resolve the argv executable and reobserve the sealed
  sources.
- U6c2 moves `bound` and `admit` into `session.rs`.
