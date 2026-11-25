//! #1277: a boxed dsh seat's own `brokkr-hands` row carries `--reach` and
//! `--spec` as unpadded base64url so dsh's `--dump-config` never folds a
//! `{`-led JSON scalar onto a line the driver's line rule refuses.
//!
//! These tests read the controller's captures from the workspace's
//! `docs/evidence/adapters/dsh-hands-1277/` directory at run time (not
//! `include_bytes!`/`include_str!`), matching the adjacent runtime fixture
//! convention in `tests.rs` and `composed_carriers.rs` so the packaged crate
//! compiles without files above its manifest directory.
//!
//! - B5: bind the controller's 2026-11-24 Linux/dsh 0.2.2-rc.1 evidence at
//!   run time (`dump-folded-json.txt` refuses `Unreadable`;
//!   `dump-base64url.txt` reads 100 rows, holds `brokkr-hands` enabled on
//!   `@deepseek-ai/dsh-mcp-client`, and passes both `judge` and `judge_mode`).
//! - B6: bind the overlay writer (non-ASCII path, bytes encoding to `-` and
//!   `_`, exact round-trip decode, every line of the emitted row and its
//!   folded-block-scalar dump passing `rows::carriers::check` via
//!   `composition::closure`).

use std::path::{Path, PathBuf};

use serde_json::json;

use super::super::composition::{closure, judge, judge_mode, rows, Composition};
use super::super::mcp;
use super::super::{hands_rows, Mode, Release, HANDS_ID, MCP_CLIENT};
use crate::hands::{decode_base64url, Bind, BindMode, GitFacts, HandsSpec, Sealed, SEALED_REACH};
use crate::native_controls::{SealedBoundary, SealedDialect, ARTIFACT_ROOT, SERVING_INPUTS};

/// Controller capture (2026-11-24, Linux, dsh 0.2.2-rc.1): real seat overlay
/// with long JSON `--reach` and `--spec`, folded by js-yaml onto lines 455
/// and 458 starting with `{`.
///
/// Read at run time by explicit `.txt` file path (not `include_*!` and not by
/// joining onto the directory literal, which also holds `README.md`): the
/// measured captures have one home in `docs/evidence/`, a crate must not
/// include a file above itself, and the test reads no `.md` page.
fn dump_folded_json() -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/evidence/adapters/dsh-hands-1277/dump-folded-json.txt"),
    )
    .expect("the controller's #1277 folded-JSON capture is in the workspace")
}

/// Controller capture (2026-11-24, Linux, dsh 0.2.2-rc.1): the same seat
/// overlay with `--reach` and `--spec` values base64url-encoded without
/// padding.
fn dump_base64url() -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/evidence/adapters/dsh-hands-1277/dump-base64url.txt"),
    )
    .expect("the controller's #1277 base64url capture is in the workspace")
}

/// Extract the single-quoted `args:` entries from the emitted `brokkr-hands`
/// row.
fn emitted_args(overlay: &str) -> Vec<String> {
    let after = overlay
        .split_once("        args:\n")
        .expect("overlay writes args block")
        .1;
    after
        .lines()
        .map(|line| {
            let inner = line
                .strip_prefix("          - '")
                .and_then(|rest| rest.strip_suffix('\''))
                .unwrap_or_else(|| panic!("expected single-quoted arg line, got {line:?}"));
            inner.replace("''", "'")
        })
        .collect()
}

/// Format `args` the way dsh's `--dump-config` (js-yaml) prints the
/// `brokkr-hands` row's `args:` block, folding any scalar longer than 60
/// bytes onto a `>-` block-scalar content line.
fn dump_args_block(args: &[String]) -> String {
    let mut block = String::from("    args:\n");
    for arg in args {
        if arg.len() > 60 {
            block.push_str("      - >-\n        ");
            block.push_str(arg);
            block.push('\n');
        } else if arg.starts_with("--") {
            block.push_str(&format!("      - '{arg}'\n"));
        } else {
            block.push_str(&format!("      - {arg}\n"));
        }
    }
    block
}

/// A long sealed reach whose path holds a non-ASCII character and bytes
/// whose base64url encoding contains both `-` and `_`, paired with a spec
/// carrying binds and masks so both documents fold in dsh's dump.
fn long_non_ascii_reach_and_spec() -> (Sealed, HandsSpec) {
    let spec = HandsSpec {
        network: false,
        binds: vec![
            Bind {
                path: "~/.cargo".to_string(),
                mode: BindMode::Overlay,
                mask: vec!["credentials.toml".to_string(), "credentials".to_string()],
            },
            Bind {
                path: "~/.rustup".to_string(),
                mode: BindMode::Ro,
                mask: Vec::new(),
            },
        ],
    };
    let work = PathBuf::from(
        "/home/valentin/source/feedback-loop-ai/caf\u{00e9}-\u{00be}\u{00be}\u{00be}-\u{00bf}\u{00bf}\u{00bf}-worktree-long-enough-to-fold",
    );
    let home = PathBuf::from("/home/valentin");
    let facts = GitFacts {
        git_dir: Some(PathBuf::from(
            "/home/valentin/source/feedback-loop-ai/brokkr/.git/worktrees/caf\u{00e9}-wt",
        )),
        common_dir: Some(PathBuf::from(
            "/home/valentin/source/feedback-loop-ai/brokkr/.git",
        )),
        identity: Vec::new(),
    };
    let store = work.join(".forge/secrets.env");
    let sealed = crate::hands::seal(&spec, &home, &work, &facts, Some((&store, &store)));
    (sealed, spec)
}

struct EmittedHands {
    reach: Sealed,
    spec: HandsSpec,
    workdir: &'static str,
    artifacts: &'static str,
    command: String,
    args: Vec<String>,
}

/// Shared fixture for both writer tests: a boxed dsh seat with long non-ASCII
/// reach and spec, passed through `mcp::dsh_hands`.
fn emitted_hands_fixture() -> EmittedHands {
    let (reach, spec) = long_non_ascii_reach_and_spec();
    let workdir = "/home/valentin/source/feedback-loop-ai/caf\u{00e9}-worktree-long-enough-to-fold";
    let artifacts = "/home/valentin/source/feedback-loop-ai/caf\u{00e9}-worktree-long-enough-to-fold/.forge/artifacts";
    let dialect = SealedDialect {
        stands: Some(SealedBoundary::Namespace),
        ..Default::default()
    };
    let serving = super::dispatched(dialect, Vec::new(), Some(spec.clone()));
    let mut input = json!({
        "workdir": workdir,
        ARTIFACT_ROOT: artifacts,
        SERVING_INPUTS: serving,
    });
    input[SEALED_REACH] = serde_json::to_value(&reach).unwrap();

    let (command, args) = mcp::dsh_hands(&input, workdir)
        .expect("dsh_hands binds")
        .expect("boxed hands present");
    EmittedHands {
        reach,
        spec,
        workdir,
        artifacts,
        command,
        args,
    }
}

/// B5: bind the controller's real dsh 0.2.2-rc.1 captures from
/// `docs/evidence/adapters/dsh-hands-1277/` at run time.
#[test]
fn the_captured_dsh_dumps_prove_folded_json_refuses_and_base64url_passes_both_judges() {
    let folded_json = dump_folded_json();
    let base64url = dump_base64url();
    assert_eq!(rows(&folded_json), Err(Composition::Unreadable));

    let parsed = rows(base64url.as_bytes()).expect("base64url dump must read");
    assert_eq!(parsed.len(), 100);
    let hands: Vec<_> = parsed.iter().filter(|row| row.id == HANDS_ID).collect();
    assert_eq!(hands.len(), 1);
    assert_eq!(hands[0].name, MCP_CLIENT);
    assert!(!hands[0].disabled);

    assert_eq!(judge(Release::V0_2_2, &parsed), Ok(()));
    assert_eq!(judge_mode(&parsed, Mode::WorkspaceWrite), Ok(()));
}

/// B6 and B11: the dsh overlay writer emits `--reach-base64url` and
/// `--spec-base64url` with the exact JSON bytes encoded without padding,
/// keeps `--workdir` and `--artifacts` plain, passes `rows::carriers::check`
/// on every emitted and folded line, and reads cleanly when substituted into
/// the captured 0.2.2-rc.1 composed dump.
#[test]
fn the_dsh_overlay_writer_encodes_reach_and_spec_and_passes_the_line_rule() {
    let fixture = emitted_hands_fixture();
    let overlay = hands_rows(
        Release::V0_2_2,
        Mode::WorkspaceWrite,
        &fixture.command,
        &fixture.args,
    )
    .unwrap();
    let written = emitted_args(&overlay);
    assert_eq!(written, fixture.args);
    assert_eq!(
        &written[..6],
        [
            "hands",
            "serve",
            "--workdir",
            fixture.workdir,
            "--artifacts",
            fixture.artifacts
        ]
    );
    assert_eq!(written[6], "--reach-base64url");
    assert_eq!(written[8], "--spec-base64url");
    assert_eq!(written.len(), 10);

    let (reach_b64, spec_b64) = (&written[7], &written[9]);
    assert!(reach_b64.len() > 80, "reach must be long enough to fold");
    assert!(spec_b64.len() > 80, "spec must be long enough to fold");
    assert!(reach_b64.contains('-'), "reach encoding must exercise -");
    assert!(reach_b64.contains('_'), "reach encoding must exercise _");
    assert!(!reach_b64.contains('='));
    assert!(!spec_b64.contains('='));

    let expected_reach_json = serde_json::to_string(&fixture.reach).unwrap();
    let expected_spec_json = fixture.spec.to_value().to_string();
    assert!(
        !expected_reach_json.is_ascii(),
        "reach must hold a non-ASCII path"
    );

    let decoded_reach_json = decode_base64url(reach_b64).expect("decode reach");
    let decoded_spec_json = decode_base64url(spec_b64).expect("decode spec");
    assert_eq!(decoded_reach_json, expected_reach_json);
    assert_eq!(decoded_spec_json, expected_spec_json);

    let decoded_reach: Sealed = serde_json::from_str(&decoded_reach_json).unwrap();
    let decoded_spec =
        HandsSpec::parse(&serde_json::from_str(&decoded_spec_json).unwrap()).unwrap();
    assert_eq!(decoded_reach, fixture.reach);
    assert_eq!(decoded_spec, fixture.spec);

    // Every line of the emitted `brokkr-hands` row passes the line rule on its own.
    let hands_row = overlay
        .split_once("- insert:\n")
        .expect("insert row present")
        .1;
    for line in hands_row.lines() {
        assert_eq!(closure(line), Ok(()), "line refused: {line:?}");
    }

    // Folded as dsh's `--dump-config` prints long scalars (`>-` header +
    // indented content line), every line passes `rows::carriers::check`.
    let folded_block = dump_args_block(&written);
    for line in folded_block.lines() {
        assert_eq!(closure(line), Ok(()), "folded line refused: {line:?}");
    }

    // Direct check against `serve_args` (JSON spelling) to prove the exact
    // same two JSON documents travel in both spellings.
    let plain = crate::hands::serve_args(
        Path::new(fixture.workdir),
        Path::new(fixture.artifacts),
        &fixture.reach,
        &fixture.spec,
    );
    assert_eq!(plain[6], "--reach");
    assert_eq!(plain[7], decoded_reach_json);
    assert_eq!(plain[8], "--spec");
    assert_eq!(plain[9], decoded_spec_json);
}

/// B5 + B11: substituting the args the overlay writer currently produces
/// (folded onto `>-` block-scalar lines as dsh's `--dump-config` writes
/// long scalars) into the captured 0.2.2-rc.1 dump still reads 100 rows and
/// passes both judges; emitting JSON again fails `rows(...)` with `Unreadable`.
#[test]
fn a_regenerated_composed_dump_with_the_writers_args_passes_both_judges() {
    let fixture = emitted_hands_fixture();
    let folded_block = dump_args_block(&fixture.args);
    let base64url = dump_base64url();
    let head = base64url
        .split_once("    args:\n")
        .expect("captured dump carries args block")
        .0;
    let regenerated = format!("{head}{folded_block}");
    let parsed = rows(regenerated.as_bytes()).expect("regenerated dump must read");
    assert_eq!(parsed.len(), 100);
    assert_eq!(judge(Release::V0_2_2, &parsed), Ok(()));
    assert_eq!(judge_mode(&parsed, Mode::WorkspaceWrite), Ok(()));
}
