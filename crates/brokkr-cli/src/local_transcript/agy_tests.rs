//! Safe discovery and secret-masking tests for `agy-conversation` (#1227).

use std::os::unix::fs::{symlink, PermissionsExt};

use brokkr_view::transcript::{LegacyProvenance, TranscriptKind, Unavailable};
use brokkr_view::{Subject, Transcript};

use super::{mask_secrets, read_local};

const UUID: &str = "5ca7618a-9b3f-4eb5-9863-d4bdae9eb945";
/// `l2b-printed-value.transcript.ndjson`, read at test run time: the evidence
/// lives outside `crates/brokkr-cli`, so no `include_str!` in `src/` reaches it
/// (`packaging::no_crate_includes_a_file_from_outside_itself`).
fn printed_value() -> String {
    std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/evidence/adapters/agy-cli-1-5/l2b-printed-value.transcript.ndjson"),
    )
    .expect("committed l2b-printed-value evidence transcript")
}
const SENTINEL: &str = "l2b-sentinel-value-7f3a9c2e41d8";

fn agy_subject(home: &std::path::Path, locator: &str) -> Subject {
    Subject {
        run: "r1".into(),
        key: "e1".into(),
        reference: Some(Transcript {
            kind: "agy-conversation".into(),
            locator: locator.into(),
            home: home.to_str().unwrap().into(),
        }),
        provenance: LegacyProvenance::Other,
        legacy_id: None,
        working: false,
        tab: 0,
        realm: None,
    }
}

fn planted_printed_home() -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("agy-conversations");
    let conv = home.join(UUID);
    std::fs::create_dir_all(&conv).unwrap();
    std::fs::write(conv.join("transcript.jsonl"), printed_value()).unwrap();
    (dir, home, conv)
}

#[test]
fn agy_discovery_reads_kept_file_and_refuses_symlinks_and_non_uuid() {
    let (dir, home, conv) = planted_printed_home();

    let read = read_local(&agy_subject(&home, UUID), None);
    assert!(read.is_readable(), "{read:?}");
    assert_eq!(read.kind, Some(TranscriptKind::AgyConversation));
    assert_eq!(read.turns.len(), 4);
    assert!(read.turns[2].blocks[0].text.contains(SENTINEL));

    // Symlink at transcript.jsonl -> UnsafePath.
    let link_home = dir.path().join("link-leaf");
    let link_conv = link_home.join(UUID);
    std::fs::create_dir_all(&link_conv).unwrap();
    symlink(
        conv.join("transcript.jsonl"),
        link_conv.join("transcript.jsonl"),
    )
    .unwrap();
    let unsafe_leaf = read_local(&agy_subject(&link_home, UUID), None);
    assert_eq!(unsafe_leaf.unavailable, Some(Unavailable::UnsafePath));

    // Symlink at conversation dir -> UnsafePath.
    let link_dir_home = dir.path().join("link-dir");
    std::fs::create_dir_all(&link_dir_home).unwrap();
    symlink(&conv, link_dir_home.join(UUID)).unwrap();
    let unsafe_dir = read_local(&agy_subject(&link_dir_home, UUID), None);
    assert_eq!(unsafe_dir.unavailable, Some(Unavailable::UnsafePath));

    // Non-UUID locator -> InvalidReference.
    let bad = read_local(&agy_subject(&home, "../escape"), None);
    assert_eq!(bad.unavailable, Some(Unavailable::InvalidReference));

    // Missing conversation -> NotFound.
    let missing = read_local(
        &agy_subject(&home, "00000000-0000-0000-0000-000000000000"),
        None,
    );
    assert_eq!(missing.unavailable, Some(Unavailable::NotFound));
}

#[test]
fn agy_printed_sentinel_is_masked_by_the_one_mask_secrets_path() {
    let (dir, home, _conv) = planted_printed_home();

    let store = dir.path().join("secrets.env");
    std::fs::write(&store, format!("L2B_SENTINEL={SENTINEL}\n")).unwrap();
    std::fs::set_permissions(&store, std::fs::Permissions::from_mode(0o600)).unwrap();

    let masked = mask_secrets(read_local(&agy_subject(&home, UUID), None), &store);
    assert!(masked.is_readable(), "{masked:?}");
    assert_eq!(masked.turns.len(), 4);
    for turn in &masked.turns {
        for block in &turn.blocks {
            assert!(
                !block.text.contains(SENTINEL),
                "sentinel leaked in {}: {}",
                turn.role,
                block.text
            );
        }
    }
    assert!(
        masked
            .notices
            .iter()
            .any(|notice| notice.contains("L2B_SENTINEL")),
        "{:?}",
        masked.notices
    );

    // TUI pane/doors over the masked read contain no sentinel bytes.
    let (keys, selected_door, whole_door) =
        crate::tui::transcript_surfaces_for_test(&masked, Some(2));
    assert_eq!(keys.len(), 4);
    for door in [&selected_door, &whole_door] {
        assert!(!door.contains(SENTINEL), "{door}");
        assert!(door.contains("[secret:L2B_SENTINEL]"), "{door}");
    }
}
