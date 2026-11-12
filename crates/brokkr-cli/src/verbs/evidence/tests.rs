//! What `brokkr evidence` prints, and the one argument combination the
//! group refuses when it is driven directly: the retirement's words for an
//! archived root with and without its inventory, and a discarded one.

use std::path::PathBuf;

use brokkr_runtime::engine::evidence::{Cause, EvidenceRoot, Retirement, Superseded};

use super::{fixture, painted, run, run_in, EvidenceCmd, ListArgs, RetireArgs};
use crate::fleet::page::tests::Refusing;

/// A retirement with `cause` and `archive`.
fn retirement(cause: Cause, archive: Option<&str>) -> Retirement {
    Retirement {
        artifacts: PathBuf::from("/evidence/root"),
        cause,
        archive: archive.map(PathBuf::from),
        inventory_sha256: archive.map(|_| "a".repeat(64)),
        operator: "operator".to_string(),
        reason: "the evidence is archived".to_string(),
        retired_at: "2026-11-08T00:00:00Z".to_string(),
        engine: "0.13.1".to_string(),
    }
}

#[test]
fn a_root_reads_its_existence_its_digests_and_its_retirement() {
    let root = |exists, retirement| EvidenceRoot {
        artifacts: PathBuf::from("/evidence/root"),
        exists,
        digests: 2,
        bytes: 7,
        retirement,
        history: Vec::new(),
        problem: None,
    };
    assert_eq!(
        painted(&root(true, None)),
        "/evidence/root  exists, 2 digests, 7 bytes, not retired\n"
    );
    assert_eq!(
        painted(&root(false, Some(retirement(Cause::Discarded, None)))),
        "/evidence/root  absent, 2 digests, 7 bytes, discarded on 2026-11-08T00:00:00Z by operator: \
         the evidence is archived\n"
    );
    let archived = retirement(Cause::Archived, Some("/archive")).words();
    assert_eq!(
        archived,
        format!(
            "archived at /archive (inventory {}) on 2026-11-08T00:00:00Z by operator: the evidence \
             is archived",
            "a".repeat(64)
        )
    );
    assert_eq!(
        retirement(Cause::Archived, None).words(),
        "archived on 2026-11-08T00:00:00Z by operator: the evidence is archived"
    );
}

#[test]
fn retire_driven_without_a_cause_refuses_by_name() {
    let mut out = Vec::new();
    let args = RetireArgs {
        root: PathBuf::from("/evidence/root"),
        archived_to: None,
        discarded: false,
        reason: "why".to_string(),
        json: false,
    };
    let refused = run_in(
        EvidenceCmd::Retire(args),
        std::path::Path::new("/nonexistent-home"),
        "2026-11-08T00:00:00Z",
        "operator",
        &mut out,
    );
    assert_eq!(
        refused.unwrap_err().to_string(),
        "retire needs --archived-to <dir> or --discarded"
    );
}

#[test]
fn an_empty_host_lists_nothing_and_json_prints_an_empty_array() {
    let home = tempfile::tempdir().unwrap();
    let mut out = Vec::new();
    run_in(
        EvidenceCmd::List(ListArgs { json: false }),
        home.path(),
        "2026-11-08T00:00:00Z",
        "operator",
        &mut out,
    )
    .unwrap();
    assert_eq!(String::from_utf8(out).unwrap(), "");
    let mut out = Vec::new();
    run_in(
        EvidenceCmd::List(ListArgs { json: true }),
        home.path(),
        "2026-11-08T00:00:00Z",
        "operator",
        &mut out,
    )
    .unwrap();
    assert_eq!(String::from_utf8(out).unwrap(), "[]\n");
}

/// S3: every untrusted field of a root goes through the surface's
/// escaping: the current retirement, each history row and the problem, so
/// no forged line and no raw escape reaches the terminal.
#[test]
fn a_roots_words_cannot_forge_a_line_or_carry_an_escape() {
    let hostile = "given up\nok capabilities evidence /forged: all good\n\u{1b}[2J";
    let mut retired = retirement(Cause::Discarded, None);
    retired.reason = hostile.to_string();
    let root = EvidenceRoot {
        artifacts: PathBuf::from(
            "/evidence/\nok capabilities evidence /path-forged: all good\n\u{1b}[2J",
        ),
        exists: false,
        digests: 0,
        bytes: 0,
        retirement: Some(retired.clone()),
        history: vec![Superseded::of(&retired, "run-a", "2026-11-09T00:00:00Z")],
        problem: Some(hostile.to_string()),
    };
    let out = painted(&root);
    assert!(!out.contains('\u{1b}'), "no raw escape: {out:?}");
    assert!(
        !out.lines().any(|line| line.starts_with("ok capabilities")),
        "no forged line starts: {out:?}"
    );
    assert!(out.contains("capabilities evidence /forged"), "{out:?}");
    assert!(
        !out.lines().any(|line| line.starts_with("ok capabilities")),
        "the root's path cannot forge a line: {out:?}"
    );
    assert!(
        out.contains("/path-forged"),
        "the path still prints: {out:?}"
    );
}

/// I1: a host that names no HOME holds no evidence root, and the verb says
/// so before it reads one.
#[test]
fn a_host_that_names_no_home_refuses_the_verb() {
    let mut env = crate::tests::env_guard::EnvGuard::lock();
    env.remove("HOME");
    let refused = run(EvidenceCmd::List(ListArgs { json: false })).unwrap_err();
    assert_eq!(
        refused.to_string(),
        "this host names no HOME, so it holds no evidence root"
    );
}

/// `retire`'s text row ends the verb's output and answers completed: the
/// root is gone, and the retirement is sealed beside its historical record.
#[test]
fn a_discarded_retirement_prints_its_row_and_completes() {
    let host = fixture::Host::new();
    let artifacts = host.artifacts();
    std::fs::create_dir_all(&artifacts).unwrap();
    host.record(None);
    let (at, key) = host.at();
    let args = RetireArgs {
        root: artifacts.clone(),
        archived_to: None,
        discarded: true,
        reason: "the evidence is given up".to_string(),
        json: false,
    };
    let mut out = Vec::new();
    run_in(
        EvidenceCmd::Retire(args),
        &host.home,
        "2026-11-08T00:00:00Z",
        "operator",
        &mut out,
    )
    .unwrap();
    assert_eq!(
        String::from_utf8(out).unwrap(),
        format!(
            "retired {}: discarded on 2026-11-08T00:00:00Z by operator: the evidence is given up\n",
            artifacts.display()
        )
    );
    assert!(!artifacts.exists(), "the discard removed the root");
    assert!(
        at.join(format!("retired-{key}.json")).exists(),
        "the retirement is sealed beside the historical record"
    );
}

/// A row the console cannot write refuses after the retirement committed,
/// never as a retirement that failed.
#[test]
fn a_retired_row_that_cannot_be_written_refuses() {
    let host = fixture::Host::new();
    let artifacts = host.artifacts();
    std::fs::create_dir_all(&artifacts).unwrap();
    host.record(None);
    let args = RetireArgs {
        root: artifacts,
        archived_to: None,
        discarded: true,
        reason: "the evidence is given up".to_string(),
        json: false,
    };
    let refused = run_in(
        EvidenceCmd::Retire(args),
        &host.home,
        "2026-11-08T00:00:00Z",
        "operator",
        &mut Refusing::default(),
    )
    .unwrap_err();
    assert_eq!(refused.to_string(), "broken pipe");
}
