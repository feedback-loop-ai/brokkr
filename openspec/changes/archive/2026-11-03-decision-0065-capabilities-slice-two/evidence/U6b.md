# U6b evidence — the broker command as a closed handler

Unit U6b closes tasks 27.1 and 27.2 on branch `s2/U6b`, cut from main at
`79addef6`. Everything below was observed in the implementing session on
2026-10-05, against the working tree that became this unit's commit.

## What changed

`brokkr broker serve --plan <PATH> --plan-digest <HEX>` is a new `Cmd::Broker`
variant whose subcommand enum `BrokerCmd` has the one variant `Serve`. Its
handler is `broker::run` in the new `crates/brokkr-cli/src/broker.rs`. The
locator must be absolute, may name no `..` component and may be at most
1,024 bytes (macOS's `PATH_MAX`, the smaller of the two supported hosts'
under decision 0063). The digest is checked with
`brokkr_core::canonical::is_sha256_hex`, the house's one spelling of a
sha256. The command takes no other option and no trailing argv, so a server
argv, a grant or a secret value has no way in.

No engine plan inventory exists before U6c, so no locator and digest a
caller names can be bound to an attempt. The handler refuses every
well-formed invocation with MB3's `broker plan is not bound to this attempt`
before reading the plan or starting anything. SD3's later
`broker serving protections are incomplete` cause belongs to the bound-plan
path, which U6c builds. U6b adds no variant for it because nothing could yet
construct one (decision 0071 ruling 6). The verb is hidden, like
`fake-driver`, because the harness starts it from engine-written
configuration and no operator invokes it. Hiding it also leaves
`docs/reference/cli.md` byte-identical.

The CLI crate has no `thiserror` dependency, and adding one would touch
`Cargo.toml` and `Cargo.lock` outside the row. `BrokerError` is therefore a
closed enum with a hand-written `Display` and `std::error::Error`, following
`selector::RefusalError`. Each variant's text is pinned once, through the
binary, in `crates/brokkr-cli/tests/capability_broker.rs`.

## Room in `run_with` (controller's CRAP ceiling, 2026-10-04)

#484 is not on this base. `quality/crap-baseline.json` reads `run_with` at
cyclomatic 35, so the target was 34 or under. The unit took the second
route the controller offered: it extracted an existing group of arms into a
helper in `lib.rs`. The four library verbs `recipes`, `agents`, `muninn`
and `secrets` moved, verbatim with their doc comments, into a
`#[command(flatten)]` subcommand enum `LibraryCmd` in `cli_args.rs`. They
are dispatched by one arm, `Cmd::Library(command) => library(workspace, command)`.
The command line still reads exactly as before, because flattening keeps
the verbs top-level and in their old order. The new `Cmd::Broker` arm then
costs one branch.

`cargo crap` (0.5.0, no LCOV, so the score is cyclomatic complexity alone)
was run over `git show 79addef6:crates/brokkr-cli/src/lib.rs` and over the
changed tree:

| Function | Base | This unit |
| --- | --- | --- |
| `lib.rs` `run_with` | 35 | 33 |
| `lib.rs` `library` (new) | — | 5 |
| `broker.rs` `BrokerError::fmt` | — | 6 |
| `broker.rs` `plan_locator` | — | 4 |
| `broker.rs` `plan_digest` | — | 3 |
| `broker.rs` `run` | — | 2 |

`run_with` lands two below its baseline, which leaves #484's ruled `probe`
arm its point. `quality/crap-baseline.json` is not edited: it is measured
from the exact-coverage LCOV, which only CI or a capable host writes.

## Tests and their removal proofs

Every test below lives in the new `crates/brokkr-cli/tests/capability_broker.rs`
and drives the real binary from a canonicalised temporary root. Each
mutation was a compiling edit, run with
`cargo test -p brokkr-cli --test capability_broker`, and then restored. The
restored suite was observed passing 5 of 5.

| Test | Exact assertion | Removal mutation | Observed failure |
| --- | --- | --- | --- |
| `a_manual_invocation_with_a_real_file_and_its_digest_is_unbound` | Exit 1; stderr is exactly `error: broker plan is not bound to this attempt`; stdout is empty; the plan's server marker is never written. The plan's real sha256 is passed, so a caller's path and digest confer nothing. | `broker::run` returns `Exit::Completed` instead of the refusal (M1) | `left: Some(0)`, `right: Some(1)` |
| `the_plan_locator_and_digest_are_bounded_where_they_are_parsed` | Exit 2 with clap's exact `invalid value` line for a relative, a `..` and a 1,025-byte locator, and for an uppercase, a 63-character and a `sha256:`-prefixed digest. A 1,024-byte locator parses and meets the unbound refusal. | M1 breaks the positive control. M2 doubles the length bound, M3 drops the absolute check, M4 drops the parent check, and M5 makes the digest parser accept anything. | M1 at line 144, M2 at 125, M3 at 113 and M4 at 117. M5 fails on the uppercase digest at line 134. |
| `no_server_argv_grant_or_secret_value_is_an_option` | `--server`, `--argv`, `--grant`, `--tools`, `--secret` and `--env` each meet exit 2 with `error: unexpected argument '<option>' found`. A trailing `-- /usr/bin/docs-mcp` is refused the same way, and leaving out `--plan-digest` names the missing argument. | M6 adds a `#[arg(long)] grant: Option<String>` field to `BrokerServeArgs`. | `left: (Some(1), "error: broker plan is not bound to this attempt")`, `right: (Some(2), "error: unexpected argument '--grant' found")` |
| `the_grouped_library_verbs_still_dispatch` | `secrets list` over a 0600 store prints exactly `DOCS_TOKEN\n` and exits 0, through the new `library` arm. | M7 makes the `LibraryCmd::Secrets` arm return `Exit::Completed` without dispatching. | `left: (Some(0), "")`, `right: (Some(0), "DOCS_TOKEN\n")` |
| `the_compile_fence_still_refuses_an_unused_mcp_grant` | `brokkr init .`'s workspace, its map lifted to v6, grants `library-docs` through an MCP dialect with `offices: []`. `compile --bundle .` then exits 1, and stderr is exactly `error: bundle: realm 'starter' grants capability 'library-docs' through dialect 'docs-mcp' of kind 'mcp', whose broker support is not implemented until decision 0065 slice two`. | M8, applied temporarily in `brokkr-runtime/src/capabilities/binding.rs`, makes `native`'s `Binding::Mcp` arm return `Ok`. | `left: Some(0)`, `right: Some(1)`: without the fence the same workspace compiles, so the fence is the only refusal. |

The unchanged visible command line is held by the existing
`cli_reference_tests::the_cli_reference_is_rendered_from_clap`. It renders
`docs/reference/cli.md` from clap's tree and passed unchanged after the
flatten. M9 set the broker verb's `hide = true` to `hide = false`, and that
test then failed at `cli_reference_tests.rs:335`. After restoration all 17
`cli_reference` tests passed again. The runtime regression
`an_mcp_grant_refuses_until_slice_two_even_unused_and_a_hands_grant_is_reserved`
is unchanged and passed in `cargo test -p brokkr-runtime --lib`, which ran
768 tests with none failing.

## Gates on this tree

Formatting, workspace clippy with `-D warnings`, and `cargo +1.88 check`
over all targets and features finished without a diagnostic. The suites
were run crate by crate under `timeout 590`, with no failure:

- brokkr-cli: 627 unit tests and every integration binary, including the new
  suite.
- brokkr-runtime: 768 unit tests and every integration binary, among them
  `witness_digests` and `budgets`.
- core, protocol, store, view and bridge.
- brokkr-seatbelt-probe: 99 tests.

`compile --bundle bundles/self` and `compile --bundle bundles/verify` both
printed their manifests and policy sweep reports. Strict OpenSpec validation
reported 20 of 20 items passing. `typos --hidden`, `git diff --check`,
`quality/ratchet.sh files` and `quality/ratchet.sh clones` were all clean.
The clones ratchet had first refused one six-line test clone, which was an
inline policy fixture repeating `capability_verbs.rs`. The fence test now
builds its workspace with `brokkr init` instead.

`quality/file-lines.txt` records the measured counts of the touched files.
`lib.rs` falls from 1,939 to 1,929, and `cli_args.rs` stands at 699. The new
`broker.rs` has 91 lines and `capability_broker.rs` has 262. No
function-length or suppression listing moved, and no suppression was added.
No witness or compose input changed: the CLI's argument tree and dispatch
are not hashed into either. The budget inputs did not move either (prompt
bytes, `Cargo.lock` and the transcript readers), and their tests passed.

## Pending

`bash scripts/coverage-exact.sh`, and the CRAP ratchet that reads its LCOV,
were not run here because the seat cannot nest the namespaces the boundary
tests need. They wait for CI or a capable host. Remote CI on both supported
operating systems is also pending.
