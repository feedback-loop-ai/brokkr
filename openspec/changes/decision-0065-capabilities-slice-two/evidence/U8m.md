# U8m — Redact doctor's server words and refuse a storeless boxed writer

Tasks 47.3 and 47.4; requirements MB4 and SI1, with SD2 and SD4 for the
verification duty. Branch `s2/U8m`, cut from main at c99d2472e, recipe
`claude-flash-dsh-adv`, run `0065-slice-two-unit-u8m-see-the--7ef0aac5`.
Everything below was observed in this run's implement seat, on this tree,
on Linux 6.17. What could not be run is listed as pending at the end.

## What changed

C38. `server_words` in `crates/brokkr-cli/src/doctor/capabilities/mcp.rs`
fed both the grant's facts line and its unmeasured host line, and printed a
stdio server's whole argv and a URL as declared. A stdio server is now named
by its program, `argv[0]` as declared, and, where arguments follow it,
`(stdio, N further argument(s) withheld)`; a program alone keeps the old
`'<program>' (stdio)` words. A URL goes through a new private `url_words`:
it keeps the scheme with its `://`, the host and port after the authority's
last `@`, and the path, and drops everything from the first `?` or `#` on.
Where a userinfo, a query or a fragment was present, the line carries the one
exact marker ` (user, query and fragment withheld)`; a URL with none of them
reads as before. The marker is the same whichever of the three were present,
so the words do not tell which part carried something. The parse is by hand,
over `str` alone: the CLI crate has no URL parser (`url` is a workspace
dependency of `brokkr-core` only), and the framing authorised no manifest
edit. Every other word of doctor's lines is unchanged.

C42. `Engine::prepared` in `crates/brokkr-runtime/src/engine/boxed.rs` now
reads the store the boxed hands' sealed reach names first, and refuses with
a new `Unprepared::Storeless` where there is no reach for those hands or
its seal names no store (`"store": null`). The refusal comes before
`make_root`, `led` and `make_store`, so nothing is made and no box is built.
Its text: `dispatch refused: secrets store cannot be hidden from workspace
hands: the box's reach sealed no store for these hands at admission, so no
box of them is built (U8m)`. A sealed store at another place than the kept
one still refuses with `Strayed`, unchanged, and a matching one is made as
before. Doctor's and the probe's own `seal(..., None)` calls are not boxed
writers, never reach `prepared`, and were not touched.

No other production file moved. No public item was added: `url_words`,
`URL_WITHHELD` and `Storeless` are private or `pub(super)` within their
module trees, and `git status --short quality/public-api` printed nothing.

## The proofs

| Test | What it pins |
| --- | --- |
| `doctor::capability_tests::mcp_tests::doctor_names_a_server_without_its_arguments_userinfo_query_or_fragment` | A whole doctor report (`report_capabilities`, no bundle) over three realms: one granting a stdio dialect whose argv is `/nonexistent/docs-mcp --token U8M-MARKER`, one a URL dialect `https://docs.example:8443/mcp?token=U8M-MARKER#U8M-MARKER`, one a URL dialect whose userinfo is `U8M-MARKER:U8M-MARKER`. No line contains `U8M-MARKER` (asserted as an empty list of lines that do). The facts and host lines are exact: `'/nonexistent/docs-mcp' (stdio, 2 further argument(s) withheld)` and `at URL 'https://docs.example:8443/mcp' (user, query and fragment withheld)`. The userinfo dialect is refused at load by the tool-dialect contract's own URL pattern, and its line is pinned exactly: it names the pointer `/connection/url` and the pattern, never the value. |
| `doctor::capabilities::mcp::tests::a_server_is_named_without_its_userinfo_query_fragment_or_arguments` | `server_words` directly, where the contract's refusal cannot stand in front of it: a userinfo with two `@` plus port, path, query and fragment; a userinfo alone over `http`; a fragment alone; an `@` inside the path, kept as the path's with no marker; a stdio program alone; a stdio program with two arguments. Each is an exact string. New test child `crates/brokkr-cli/src/doctor/capabilities/mcp/tests.rs` (60 lines), declared `#[cfg(test)] mod tests;` from `mcp.rs`, because `mcp` is a private module that the owning suite `doctor::capability_tests` cannot name. |
| `engine::boundary_tests::sealed_tests::store_tests::a_boxed_writer_whose_reach_seals_no_store_refuses_before_any_box` | Under `namespace`, a boxed exec seat composed with no reach sealed for its hands, then with a reach whose seal names no store (asserted `Some(None)` through `sealed_store`), each refuses with exactly `Unprepared::Storeless`'s text. Neither the artifact root nor the store exists afterwards. The variant's text is pinned once. The null-store reach is planted by a new fixture, `Engine::sealed_storeless`, in `engine/broker/admission/tests.rs` beside `sealed_for`, because `Reach` is private to the broker's module tree. |
| `engine::broker::admission::tests::reach::a_box_whose_hands_were_sealed_nothing_refuses_its_spawn` | Existing U8a2c test, moved: the spawn of hands with no reach, and of hands whose only reach is another hands', now refuses with `Storeless`, which the engine's preparation returns before `hands_command`'s own `Unnamed::Unsealed`. That variant's text is still pinned in this test, and `hands_command` still runs and refuses there. |

## Removals, each compiled and restored

| Removal | Failed, with the assertion's own words | Passed |
| --- | --- | --- |
| R1: the stdio arm prints `argv.join(" ")` again for any argument count | Both doctor tests. The direct test at `mcp/tests.rs:56`: left `'/opt/docs --key U8M-MARKER' (stdio)`, right `'/opt/docs' (stdio, 2 further argument(s) withheld)`. The report test at `mcp_tests.rs:407`: the planted-lines list held the argv realm's facts and host lines, both carrying `'/nonexistent/docs-mcp --token U8M-MARKER' (stdio)`, against `[]`. | the other 112 doctor tests |
| R2: the URL arm prints `at URL '{url}'` again | Both doctor tests. The direct test at `mcp/tests.rs:51`: left `at URL 'https://U8M-MARKER:U8M-MARKER@a@docs.example:8443/mcp?U8M-MARKER#U8M-MARKER'`. The report test at `mcp_tests.rs:407`: the planted-lines list held the query realm's lines with `?token=U8M-MARKER#U8M-MARKER`, against `[]`. | the other 112 |
| R3: `host = authority`, userinfo kept | The direct test at `mcp/tests.rs:51`: left `at URL 'https://U8M-MARKER:U8M-MARKER@a@docs.example:8443/mcp' (user, query and fragment withheld)`. The report test passed, as the contract refuses such a dialect before doctor names it, which is why the direct test exists. | the other 113 |
| E1: `prepared` restored to main's shape (`make_root`, `led`, then `if let Some(place) = sealed.filter(..)`) | `a_boxed_writer_whose_reach_seals_no_store_refuses_before_any_box` at `store_tests.rs:305` ("no reach was sealed"): left `dispatch refused: MCP server box filesystem identity is not protected: the box's reach was not sealed at admission, or is not UTF-8, so no box of these hands is built from it (U8a2c)`, right the `Storeless` text. | the other 6 store tests |
| E2: only an absent reach refuses (`self.reach.sealed(spec).ok_or(Storeless)?`), a null store passes the old filter | The same test at `store_tests.rs:309` ("its store null"): left `None`, right the `Storeless` text. | the other 6 |
| E3: `make_root` moved ahead of the refusal | The same test at `store_tests.rs:35` (`unmade`, the artifact root): left `None`, right `Some(NotFound)`. | the other 6 |

After restoring each, `cargo test -p brokkr-cli --lib doctor::` passed
(114 passed, 0 failed) and `cargo test -p brokkr-runtime --lib store_tests`
passed (7 passed, 0 failed).

## Gates run here

`cargo fmt --all -- --check` printed nothing, and `cargo clippy --workspace
--all-targets --all-features --locked -- -D warnings` finished with no
warning.

Suites, with `--locked`, on the final tree: brokkr-runtime lib 1429 passed,
`capability_launch` 76, `it` 143, `operated_repo` 1, `queued_launch` 8;
brokkr-cli lib 1089 passed with 1 ignored, `it` 818 passed with 3 ignored,
`driver_conformance` 27, `transcript_surfaces` 14. The runtime's `it`
includes `witness_digests`, which passed, so no witness or compose pin
moved.

The self recipe compiles (`compile --bundle recipes/self`, with a policy
sweep of 48 valuations over 11 groups and none unruled); the strict openspec
validation counts all 36 items passing. Typos over the touched directories
and this change's own directory, both whitespace checks of the diff, staged
and not, and the files and clones ratchets ("file size holds", "duplication
holds") all came back clean.

Line counts, measured with `wc -l` and set in `quality/file-lines.txt`:
`doctor/capabilities/mcp.rs` 294 → 326, `engine/boxed.rs` 405 → 418, the new
`doctor/capabilities/mcp/tests.rs` 60, `doctor/capability_tests/mcp_tests.rs`
803 → 854, `sealed_tests/store_tests.rs` 343 → 371,
`broker/admission/tests.rs` 1837 → 1908 (main's own tree already stood at
1891 against the 1837 row) and `admission/tests/reach.rs` 231 → 233. Every
file stays under its ceiling. No function nears 100 lines, no suppression was
added, and no test moved between files.

## Pending

- The other workspace crates' suites (brokkr-core, brokkr-view, brokkr-store, brokkr-bridge, brokkr-protocol, brokkr-seatbelt-probe), which this unit does not touch, and brokkr-cli's `heap_*` targets, which a combined run did not finish inside the seat's 10-minute limit.
- Exact coverage (`scripts/coverage-exact.sh`).
- macOS. Nothing here is Linux-only.
- Remote CI.
